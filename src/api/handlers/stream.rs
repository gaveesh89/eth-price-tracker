//! WebSocket streaming endpoint.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use tracing::{info, instrument, warn};

use crate::api::models::{PriceStreamMessage, ReservesInfo};
use crate::app_state::AppState;
use std::sync::atomic::Ordering;

#[utoipa::path(
    get,
    path = "/api/v1/stream/{pool}",
    params(
        ("pool" = String, Path, description = "Pool name")
    ),
    responses(
        (status = 101, description = "WebSocket upgrade")
    ),
    tag = "Streaming"
)]
/// WebSocket endpoint for price updates.
#[instrument(skip(state, ws), fields(pool = %pool_name))]
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(pool_name): Path<String>,
    State(state): State<AppState>,
) -> Response {
    info!(pool = %pool_name, "WebSocket connection requested");

    ws.on_upgrade(move |socket| handle_socket(socket, pool_name, state))
}

async fn handle_socket(mut socket: WebSocket, pool_name: String, state: AppState) {
    let pool_name_normalized = pool_name.replace('-', "/");

    state.ws_connected.store(true, Ordering::Relaxed);
    info!(pool = %pool_name_normalized, "WebSocket connection established");

    let connect_msg = PriceStreamMessage {
        event_type: "connected".to_string(),
        pool: pool_name_normalized.clone(),
        price: 0.0,
        block_number: 0,
        timestamp: chrono::Utc::now(),
        reserves: ReservesInfo { weth: 0.0, usdt: 0.0 },
    };

    if let Ok(json) = serde_json::to_string(&connect_msg) {
        let _ = socket.send(Message::Text(json)).await;
    }

    let mut rx = state.price_broadcast.subscribe();

    loop {
        tokio::select! {
            Ok(price_update) = rx.recv() => {
                if price_update.pool != pool_name_normalized {
                    continue;
                }

                if let Ok(json) = serde_json::to_string(&price_update) {
                    if socket.send(Message::Text(json)).await.is_err() {
                        warn!(pool = %pool_name_normalized, "Failed to send message, closing connection");
                        break;
                    }
                }
            }

            Some(Ok(msg)) = socket.recv() => {
                match msg {
                    Message::Close(_) => {
                        info!(pool = %pool_name_normalized, "Client closed connection");
                        break;
                    }
                    Message::Ping(data) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }

            else => break,
        }
    }

    info!(pool = %pool_name_normalized, "WebSocket connection closed");
    state.ws_connected.store(false, Ordering::Relaxed);
}
