use std::process::exit;

use anyhow::Result;
use clap::command;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[tokio::main]
async fn main() -> Result<()> {
    let path = "/tmp/pomobar.sock";

    let cmd = command!()
        .subcommand(command!("status").about("Get currently pomodoro status."))
        .subcommand(command!("toggle").about("Start/Pause pomodoro."))
        .subcommand(command!("reset").about("Reset pomodoro."));

    let mut socket = UnixStream::connect(path).await?;

    let matches = cmd.clone().get_matches();

    match matches.subcommand_name() {
        Some(command) => {
            socket.write_all(command.as_bytes()).await?;
        }
        None => {
            cmd.clone().print_help().unwrap();
            exit(127);
        }
    };

    let mut buf = vec![0; 1024];
    let content_length = socket.read(&mut buf).await.unwrap();

    if content_length > 0 {
        let content = String::from_utf8(buf[..content_length].to_vec()).unwrap();
        println!("{content}");
    }

    Ok(())
}
