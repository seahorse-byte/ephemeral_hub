use serde::{Deserialize, Serialize};

// Data structure for a single drawing path.
// This will be stored in Redis and sent over WebSockets.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PathData {
    pub id: String,
    pub points: Vec<(f64, f64)>,
    pub color: String,
    pub stroke_width: f64,
}
// Message format for WebSocket communication.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum WsMessage {
    PathCompleted(PathData),
}
