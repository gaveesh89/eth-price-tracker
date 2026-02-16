//! Shared application state for API server and streaming.

use std::sync::{atomic::AtomicBool, Arc};
use std::time::SystemTime;
use tokio::sync::broadcast;

use crate::api::models::PriceStreamMessage;
use crate::db::repository::Repository;

/// Shared application state for API handlers.
#[derive(Clone)]
pub struct AppState {
    /// Repository for database access.
    pub repository: Arc<Repository>,
    /// WebSocket connection status flag.
    pub ws_connected: Arc<AtomicBool>,
    /// Application start time for uptime tracking.
    pub start_time: SystemTime,
    /// Broadcast channel for price updates.
    pub price_broadcast: broadcast::Sender<PriceStreamMessage>,
}

impl AppState {
    /// Create a new AppState instance.
    pub fn new(repository: Repository) -> Self {
        let (tx, _) = broadcast::channel(100);

        Self {
            repository: Arc::new(repository),
            ws_connected: Arc::new(AtomicBool::new(false)),
            start_time: SystemTime::now(),
            price_broadcast: tx,
        }
    }

    /// Broadcast a price update to all subscribers.
    pub fn broadcast_price_update(&self, update: PriceStreamMessage) {
        let _ = self.price_broadcast.send(update);
    }
}
