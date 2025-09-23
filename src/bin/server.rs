use std::{
    path::Path,
    sync::{LazyLock, Mutex},
};

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};

use pomobar_rs::Pomobar;

#[macro_use]
extern crate tracing;

static _INIT_STATE: LazyLock<Mutex<Pomobar>> = LazyLock::new(|| Mutex::new(Pomobar::default()));

#[tokio::main]
async fn main() -> Result<()> {
    let path = "/tmp/pomobar.sock";

    if Path::new(path).exists() {
        std::fs::remove_file(path)?;
        debug!("The pomobar socket file already exists. Remove it.");
    }

    let listener = UnixListener::bind(path)?;
    debug!("socket service on {}", path);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let mut pomobar = _INIT_STATE.lock().unwrap().clone();
        let mut buf = vec![0; 1024];

        tokio::spawn(async move {
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
            } else {
                let command = String::from_utf8(buf[..n].to_vec()).unwrap();

                let pomobar = match command.as_str() {
                    "toggle" => pomobar.toggle(),
                    "reset" => pomobar.reset(),
                    _ => pomobar.status(),
                };

                *_INIT_STATE.lock().unwrap() = pomobar.clone();

                socket
                    .write_all(pomobar.to_string().as_bytes())
                    .await
                    .unwrap();
            }
        });
    }
}
