use std::{path::Path, time::Duration};

use anyhow::Result;
use chrono::TimeDelta;
use pomobar_rs::models::{send_notification, Pomobar, PomobarDispatcher};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
    sync::mpsc,
    time::sleep,
};

#[macro_use]
extern crate tracing;

/// Events that the server can process.
enum ServerEvent {
    Toggle,
    Reset,
    Status(tokio::sync::oneshot::Sender<String>),
    Tick,
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = "/tmp/pomobar.sock";

    if Path::new(path).exists() {
        std::fs::remove_file(path)?;
        debug!("Removed existing socket file.");
    }

    let listener = UnixListener::bind(path)?;
    debug!("Server listening on {}", path);

    let (tx, mut rx) = mpsc::channel::<ServerEvent>(32);

    // --- Timer Task ---
    let timer_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            timer_tx.send(ServerEvent::Tick).await.unwrap();
            sleep(Duration::from_secs(1)).await;
        }
    });

    // --- Socket Listener Task ---
    let socket_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 1024];
            let n = socket.read(&mut buf).await.unwrap();

            if n > 0 {
                let command = String::from_utf8(buf[..n].to_vec()).unwrap();
                match command.as_str() {
                    "toggle" => socket_tx.send(ServerEvent::Toggle).await.unwrap(),
                    "reset" => socket_tx.send(ServerEvent::Reset).await.unwrap(),
                    _ => {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        socket_tx.send(ServerEvent::Status(resp_tx)).await.unwrap();
                        let response = resp_rx.await.unwrap();
                        socket.write_all(response.as_bytes()).await.unwrap();
                    }
                }
            }
        }
    });

    // --- Main Event Loop ---
    let mut pomodoro = PomobarDispatcher::Idle(Pomobar::new());

    loop {
        let event = rx.recv().await.unwrap();

        match event {
            ServerEvent::Toggle => {
                pomodoro = match pomodoro {
                    PomobarDispatcher::Idle(p) => PomobarDispatcher::Work(p.start()),
                    PomobarDispatcher::Work(p) => PomobarDispatcher::Paused(p.pause()),
                    PomobarDispatcher::Paused(p) => PomobarDispatcher::Work(p.resume()),
                    // Breaks cannot be toggled, they must finish.
                    PomobarDispatcher::ShortBreak(_) => pomodoro,
                    PomobarDispatcher::LongBreak(_) => pomodoro,
                };
                debug!("Toggled state to: {}", pomodoro.get_state_name());
            }
            ServerEvent::Reset => {
                send_notification("Reset timer.");
                pomodoro = PomobarDispatcher::Idle(Pomobar::new());
                debug!("State reset to Idle.");
            }
            ServerEvent::Status(resp_tx) => {
                let json_content = serde_json::to_string(&pomodoro).unwrap();
                resp_tx.send(json_content).unwrap();
            }
            ServerEvent::Tick => {
                if pomodoro.get_remaining_time().eq(&TimeDelta::seconds(0)) {
                    pomodoro = match pomodoro {
                        PomobarDispatcher::Work(p) => p.finish(),
                        PomobarDispatcher::ShortBreak(p) => PomobarDispatcher::Work(p.finish()),
                        PomobarDispatcher::LongBreak(p) => PomobarDispatcher::Work(p.finish()),
                        _ => pomodoro, // No timed action for Idle or Paused
                    };
                    debug!(
                        "Timer finished, transitioned to: {}",
                        pomodoro.get_state_name()
                    );
                }
            }
        }
    }
}
