use std::time::Duration;

use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::sleep,
};
use tokio_serial::SerialStream;

use super::{interface::EInkInterface, EInkCommand, EInkResponse};

pub(crate) async fn run_thread(
    port: Box<SerialStream>,
    tx: Sender<EInkResponse>,
    mut rx: Receiver<EInkCommand>,
) -> Result<bool, EInkResponse> {
    println!("() Starting AT Thread");

    let mut interface = EInkInterface::new(port).expect("Could not create interface");

    interface.reset().await.expect("Error resetting interface");

    loop {
        let resp: Option<EInkCommand> = rx.recv().await;

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

                sleep(Duration::from_millis(1000)).await;
                interface.wait_ready().await?;
                tx.send(EInkResponse::READY).await.expect("Error Sending");
            }
            None => continue,
        }
    }
}
