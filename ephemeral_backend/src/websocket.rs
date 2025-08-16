use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures_util::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use std::{collections::HashMap, sync::Arc};
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
    State(state): State<Arc<AppWsState>>,
    Path(space_id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, space_id))
}

/// The main logic for a single WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<AppWsState>, space_id: String) {
    info!("New WebSocket connection for space: {}", space_id);

    // Get a sender for the room's broadcast channel, creating it if it doesn't exist.
    let tx = {
        let mut rooms = state.rooms.lock().await;
        rooms
            .entry(space_id.clone())
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
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task to handle incoming messages from the client.
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Broadcast the received message to all clients in the room.
            if tx.send(text).is_err() {
                // This happens if there are no active subscribers.
            }
        }
    });

    // Wait for either task to finish. If one does, the other should be aborted.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("WebSocket connection for space {} closed", space_id);
}
