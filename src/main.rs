use core::fmt;
use std::{
    thread,
    time::{self, Duration, Instant, SystemTime},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router, Server,
};
use gilrs::{Event, EventType, Gilrs};
use tokio::sync::broadcast;

const COMMAND_DELAY_MS: u64 = 18;

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<ControllerState>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/jim", get(realtime_cpus_get))
        .with_state(app_state.clone());

    // Handle gamepad input in background
    tokio::task::spawn_blocking(move || {
        let mut gilrs = Gilrs::new().unwrap();
        let mut control_state = ControllerState { left: 0, right: 0, mower: 0 };
        let mut last_sent = Instant::now();
        // Iterate over all connected gamepads
        for (_id, gamepad) in gilrs.gamepads() {
            println!("{} is {:?}", gamepad.name(), gamepad.power_info());
        }

        loop {
            // Examine new events
            while let Some(Event { id, event, time: _ }) = gilrs.next_event() {
                // Make control string
                //println!("{:?} New event from {}: {:?}", SystemTime::now(), id, event);
                let previous = control_state.clone();
                let command = create_control_state_from_event(event, previous)
                    .expect("Error creating control string from event.");
                println!(
                    "{:?} New event from {}: {:?}",
                    SystemTime::now(),
                    id,
                    command
                );
                control_state = command.clone();
                // Send it to tx if appropriate amount of time has passed
                let ellapsed_millis = last_sent.elapsed().as_millis();
                tx.send(command).unwrap_or_default();
                last_sent = Instant::now();
            }
            // TODO: Make sure the requests are throttled but we still get an ending state (ie, after a bunch of commands we let go of the triggers, the last command sent should have L00R00M??
        }
    });

    let server = Server::bind(&"0.0.0.0:7032".parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    let res = server.await;
    match res {
        Ok(_) => (),
        Err(_) => (),
    }
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<ControllerState>,
}

#[derive(Clone, Copy, Debug)]
struct ControllerState {
    left: i8,
    right: i8,
    mower: i8,
}

impl fmt::Display for ControllerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{:02}R{:02}M{:02}", self.left, self.right, self.mower)
    }
}

#[axum::debug_handler]
async fn realtime_cpus_get(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    let mut rx = app_state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        ws.send(Message::Text(msg.to_string())).await.unwrap();
    }
}

fn create_control_state_from_event(
    event: EventType,
    prev: ControllerState,
) -> Result<ControllerState, ()> {
    match event {
        EventType::AxisChanged(_, value, code) => {
            let value = linear_conversion(value);
            match code.into_u32() {
                196617 => {
                    return Ok(ControllerState {
                        left: prev.left,
                        right: value,
                        mower: prev.mower,
                    })
                }
                196618 => {
                    return Ok(ControllerState {
                        left: value,
                        right: prev.right,
                        mower: prev.mower,
                    })
                }
                _ => return Ok(prev),
            }
        }
        EventType::ButtonReleased(button, _) => match button {
            gilrs::Button::North => {
                return Ok(ControllerState {
                    left: prev.left,
                    right: prev.right,
                    mower: if prev.mower == 99 { 0 } else { 99 },
                })
            }
            _ => (),
        },
        _ => (),
    }
    return Ok(prev);
}

fn linear_conversion(original: f32) -> i8 {
    const OLD_MIN: f32 = -1.0;
    const OLD_MAX: f32 = 1.0;
    const NEW_MIN: f32 = 0.0;
    const NEW_MAX: f32 = 99.0;
    (((original - OLD_MIN) / (OLD_MAX - OLD_MIN)) * (NEW_MAX - NEW_MIN) + NEW_MIN).round() as i8
}
