use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::Html,
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::midi::MidiOut;
use crate::sim::Pedalboard;

/// Shared application state for the web server.
#[derive(Clone)]
pub struct AppState {
    pub pedalboard: Arc<Mutex<Option<Pedalboard>>>,
    pub midi: Arc<Mutex<MidiOut>>,
    pub notify: broadcast::Sender<()>,
}

/// JSON action from the web client.
#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum ClientAction {
    #[serde(rename = "button_press")]
    ButtonPress { index: usize },
    #[serde(rename = "button_down")]
    ButtonDown { index: usize },
    #[serde(rename = "button_up")]
    ButtonUp { index: usize },
    #[serde(rename = "encoder_turn")]
    EncoderTurn { index: usize, clockwise: bool },
    #[serde(rename = "preset_select")]
    PresetSelect { index: usize },
}

const WEB_UI_HTML: &str = include_str!("web_ui.html");

/// Build the axum router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn index_handler() -> Html<&'static str> {
    Html(WEB_UI_HTML)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    // Send initial state
    if let Some(json) = get_state_json(&state) {
        if socket.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    let mut rx = state.notify.subscribe();

    loop {
        tokio::select! {
            // State changed — broadcast to this client
            result = rx.recv() => {
                match result {
                    Ok(()) => {
                        if let Some(json) = get_state_json(&state) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Missed some updates, send current state
                        if let Some(json) = get_state_json(&state) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Incoming message from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(action) = serde_json::from_str::<ClientAction>(&text) {
                            handle_action(&state, action);
                            // Notify all clients (including this one)
                            let _ = state.notify.send(());
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

fn handle_action(state: &AppState, action: ClientAction) {
    let mut pb = state.pedalboard.lock().unwrap();
    let mut midi = state.midi.lock().unwrap();

    match action {
        ClientAction::ButtonPress { index } => {
            // Legacy: instant press+release (for simple clicks without long-press)
            if index < 6 {
                if let Some(ref mut pb) = *pb {
                    pb.press_button(index, &mut midi);
                    pb.release_button(index, &mut midi);
                } else {
                    midi.cc(0, 20 + index as u8, 127);
                }
            }
        }
        ClientAction::ButtonDown { index } => {
            if index < 6 {
                if let Some(ref mut pb) = *pb {
                    pb.press_button(index, &mut midi);
                }
            }
        }
        ClientAction::ButtonUp { index } => {
            if index < 6 {
                if let Some(ref mut pb) = *pb {
                    pb.release_button(index, &mut midi);
                }
            }
        }
        ClientAction::EncoderTurn { index, clockwise } => {
            if index < 2 {
                if let Some(ref mut pb) = *pb {
                    pb.turn_encoder(index, clockwise, &mut midi);
                }
            }
        }
        ClientAction::PresetSelect { index } => {
            if let Some(ref mut pb) = *pb {
                pb.switch_preset(index, &mut midi);
            } else {
                midi.program_change(0, index as u8);
            }
        }
    }
}

fn get_state_json(state: &AppState) -> Option<String> {
    let pb = state.pedalboard.lock().unwrap();
    match &*pb {
        Some(pb) => serde_json::to_string(&pb.snapshot()).ok(),
        None => {
            // No config loaded — send a minimal state
            let empty = crate::sim::SimState {
                active_preset: 0,
                preset_name: "(no config)".to_string(),
                num_presets: 0,
                buttons: (0..6)
                    .map(|i| crate::sim::ButtonState {
                        index: i,
                        label: format!("CC {}", 20 + i),
                        active: false,
                        color: "#333333".to_string(),
                    })
                    .collect(),
                encoders: (0..2)
                    .map(|i| crate::sim::EncoderState {
                        index: i,
                        label: format!("Enc {}", i),
                        value: 0,
                    })
                    .collect(),
            };
            serde_json::to_string(&empty).ok()
        }
    }
}
