use std::time::Duration;

use debug_print::debug_println;
use serialport::SerialPort;
use tokio::{
    sync::mpsc::{error::SendError, Receiver, Sender},
    time::sleep,
};

use super::interface::Interface;

#[allow(dead_code)]
pub enum EInkResponse {
    READY,
    BUSY,
    ERROR,
    DISCONNECTED,
}

pub enum EInkCommand {
    SHOW {
        buffer: Vec<u8>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        with_red: bool,
        black_border: bool,
        full_refresh: bool,
    },
}

impl EInkCommand {
    #[allow(dead_code)]
    pub(crate) fn full(buffer: Vec<u8>) -> Self {
        EInkCommand::SHOW {
            buffer,
            x: 0,
            y: 0,
            width: 250,
            height: 128,
            with_red: true,
            black_border: false,
            full_refresh: true,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn fast(buffer: Vec<u8>) -> Self {
        EInkCommand::SHOW {
            buffer,
            x: 0,
            y: 0,
            width: 250,
            height: 128,
            with_red: false,
            black_border: false,
            full_refresh: false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn partial(buffer: Vec<u8>, x: u32, y: u32, width: u32, height: u32) -> Self {
        EInkCommand::SHOW {
            buffer,
            x,
            y,
            width,
            height,
            with_red: false,
            black_border: false,
            full_refresh: false,
        }
    }
}

impl std::fmt::Display for EInkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                &EInkResponse::READY => "Ready",
                &EInkResponse::BUSY => "Busy",
                &EInkResponse::ERROR => "Error",
                &EInkResponse::DISCONNECTED => "Disconnected",
            }
        )
    }
}

pub(crate) async fn run_thread(
    port: Box<dyn SerialPort>,
    tx: Sender<EInkResponse>,
    mut rx: Receiver<EInkCommand>,
) -> Result<bool, SendError<EInkResponse>> {
    println!("() Starting AT Thread");

    let mut interface = Interface::new(port).expect("Could not create interface");

    interface.reset().await.expect("Error resetting interface");

    loop {
        debug_println!("Waiting for data");
        let resp: Option<EInkCommand> = rx.recv().await;
        debug_println!("...");

        match resp {
            Some(EInkCommand::SHOW {
                buffer,
                x,
                y,
                width,
                height,
                with_red,
                black_border,
                full_refresh,
            }) => {
                debug_println!("Got SHOW");
                tx.send(EInkResponse::BUSY).await?;

                interface
                    .send_image(
                        &buffer,
                        x,
                        y,
                        width,
                        height,
                        with_red,
                        full_refresh,
                        black_border,
                    )
                    .await;

                sleep(Duration::from_millis(1000)).await;
                interface.wait_ready().await;
                debug_println!("Send READY");
                tx.send(EInkResponse::READY).await?;
            }
            None => continue,
        }
    }
}
