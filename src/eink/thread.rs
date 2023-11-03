use std::time::Duration;

use tokio::{
    sync::mpsc::{error::TryRecvError, Receiver, Sender},
    time::sleep,
};
use tokio_serial::SerialStream;

use super::{uart_interface::EInkUartInterface, EInkCommand, EInkResponse};

pub(crate) async fn run_thread(
    port: Box<SerialStream>,
    tx: Sender<EInkResponse>,
    mut rx: Receiver<EInkCommand>,
) -> Result<bool, EInkResponse> {
    println!("() Starting AT Thread");

    let mut interface = EInkUartInterface::new(port).expect("Could not create interface");

    interface.reset().await.expect("Error resetting interface");

    loop {
        let mut resp: Result<EInkCommand, TryRecvError> = rx.try_recv();

        let mut frames_dropped: u32 = 0;

        loop {
            let _resp = rx.try_recv();
            if _resp.is_err() {
                break;
            }
            frames_dropped += 1;
            resp = _resp;
        }

        if frames_dropped > 0 {
            println!("Dropped {} frames", frames_dropped)
        }

        match resp {
            Ok(EInkCommand::SHOW {
                buffer,
                x,
                y,
                width,
                height,
                with_red,
                black_border,
                full_refresh,
            }) => {
                // debug_println!("AT Got SHOW");
                tx.send(EInkResponse::BUSY).await.expect("Error Sending");

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

                // sleep(Duration::from_millis(200)).await;
                // interface.wait_ready().await?;
                tx.send(EInkResponse::READY).await.expect("Error Sending");
            }
            Ok(EInkCommand::LED { color }) => {
                interface.set_led(color).await?;
            }
            Err(_e) => sleep(Duration::from_millis(10)).await,
        }
    }
}
