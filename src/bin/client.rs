use chrono::TimeDelta;
use serde::Serialize;

use anyhow::Result;
use clap::command;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use pomobar_rs::PomobarDispatcher;

#[derive(Debug, Clone, Serialize)]
struct Waybar {
    text: String,
    alt: String,
    class: String,
    tooltip: String,
}

impl From<PomobarDispatcher> for Waybar {
    fn from(value: PomobarDispatcher) -> Self {
        let remaining_time = value.get_remaining_time();
        let mins = remaining_time.num_minutes();
        let secs = remaining_time
            .checked_sub(&TimeDelta::minutes(mins))
            .unwrap()
            .num_seconds();

        let state = value.get_state_name();
        let cycles = value.get_cycles();

        Self {
            text: format!("{:02}:{:02}", mins, secs),
            alt: state.to_string(),
            class: state.to_string(),
            tooltip: format!("Complete {} pomodoros.", cycles),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = "/tmp/pomobar.sock";

    let cmd = command!()
        .subcommand(command!("status").about("Get currently pomodoro status."))
        .subcommand(command!("toggle").about("Start/Pause pomodoro."))
        .subcommand(command!("reset").about("Reset pomodoro"));

    let mut socket = UnixStream::connect(path).await?;

    let matches = cmd.get_matches();
    let command = matches.subcommand_name().unwrap();
    socket.write_all(command.as_bytes()).await?;

    let mut buf = vec![0; 1024];
    let content_length = socket.read(&mut buf).await.unwrap();

    if content_length > 0 {
        let json_content = String::from_utf8(buf[..content_length].to_vec()).unwrap();
        let dispatcher: PomobarDispatcher = serde_json::from_str(&json_content).unwrap();
        let waybar = Waybar::from(dispatcher);
        let waybar_json = serde_json::to_string(&waybar).unwrap();

        println!("{waybar_json}");
    }

    Ok(())
}
