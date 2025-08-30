use crate::{AppState, handlers::Hub, shared_types::WsMessage};
use axum::{
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use redis::AsyncCommands;
use std::collections::HashMap;
use tokio::sync::{Mutex, broadcast};
use tracing::{info, warn};

/// The shared state for our WebSocket rooms.
/// We use a Mutex to safely access the HashMap of rooms from multiple threads.
/// Each room has a broadcast channel to send messages to all connected clients.
#[derive(Debug, Default)]
pub struct AppWsState {
    rooms: Mutex<HashMap<String, broadcast::Sender<String>>>,
}

/// The entry point for WebSocket connections.
/// This function handles the initial upgrade from HTTP to WebSocket.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(hub_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, hub_id))
}

/// The main logic for a single WebSocket connection.
async fn handle_socket(socket: WebSocket, state: AppState, hub_id: String) {
    info!("New WebSocket connection for hub: {}", hub_id);

    // Get a sender for the room's broadcast channel, creating it if it doesn't exist.

    let tx = {
        let mut rooms = state.ws_state.rooms.lock().await;
        rooms
            .entry(hub_id.clone())
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    };

    // Subscribe to the broadcast channel to receive messages.
    let mut rx = tx.subscribe();

    // Split the WebSocket into a sender and receiver.
    let (mut sender, mut receiver) = socket.split();

    // Task to forward messages from the broadcast channel to the client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Convert the String from the broadcast channel into the type expected by Message::Text.
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Task to handle incoming messages from the client.
    let recv_task_hub_id = hub_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if tx.send(text.to_string()).is_err() {
                // No active subscribers, but that's okay.
            }

            if let Ok(WsMessage::PathCompleted(path)) = serde_json::from_str(&text) {
                let mut conn = match state.redis.get().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        warn!("Failed to get Redis connection: {}", e);
                        continue;
                    }
                };

                let key = format!("hub:{}", recv_task_hub_id);

                // Fetch the current hub data
                if let Ok(Some(hub_json)) = conn.get::<_, Option<String>>(&key).await {
                    if let Ok(mut hub) = serde_json::from_str::<Hub>(&hub_json) {
                        // Add the new path and save it back
                        hub.whiteboard.push(path);
                        let updated_json = serde_json::to_string(&hub).unwrap();
                        let ttl: isize = conn.ttl(&key).await.unwrap_or(-1);

                        if ttl > 0 {
                            let _: () = conn.set_ex(&key, updated_json, ttl as u64).await.unwrap();
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to finish. If one does, the other should be aborted.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("WebSocket connection for hub {} closed", hub_id);
}
