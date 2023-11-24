use std::{error::Error, time::Duration};

use serialport::{DataBits, Parity, SerialPort, StopBits};
use tokio::{
    sync::mpsc::{self, error::TryRecvError, Receiver, Sender},
    time::sleep,
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::log;

use super::{uart_interface::EInkUartInterface, EInkCommand, EInkInterface, EInkResponse};

pub fn start_eink_thread(
    port_str: &'static str,
    baud: u32,
    width: u32,
    height: u32,
    flip: bool,
) -> Result<EInkInterface, Box<dyn Error>> {
    // Create the serial port
    let mut port = tokio_serial::new(port_str, baud)
        .timeout(Duration::from_millis(1000))
        .open_native_async()
        .expect(format!("Failed to connect to device {}", port_str).as_str());

    port.set_exclusive(true)?;

    const CONFIG_ERROR: &str = "error configuring port";
    port.set_data_bits(DataBits::Eight).expect(CONFIG_ERROR);
    port.set_stop_bits(StopBits::One).expect(CONFIG_ERROR);
    port.set_parity(Parity::None).expect(CONFIG_ERROR);

    // let port_clone = port.try_clone().expect("Could not clone port");

    // Create a channel for communication between threads
    let (thread_tx, rx) = mpsc::channel::<EInkResponse>(512);
    let (tx, thread_rx) = mpsc::channel::<EInkCommand>(1024);

    // Spawn the serial thread
    tokio::spawn(async move {
        run_thread(Box::new(port), thread_tx, thread_rx)
            .await
            .expect("Could not spawn thread");
    });

    let mut h = height;

    if h % 8 != 0 {
        h += 8 - height % 8;
    }

    return Ok(EInkInterface {
        rx,
        tx,
        state: EInkResponse::OK,
        width,
        height,
        buffer_height: h,
        _port: port_str,
        flip,
        black_border: false,
    });
}

pub(crate) async fn run_thread(
    port: Box<SerialStream>,
    tx: Sender<EInkResponse>,
    mut rx: Receiver<EInkCommand>,
) -> Result<bool, EInkResponse> {
    println!("{} Starting AT Thread", log::THREAD);

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
            println!("{} Dropped {} frames", log::ERROR, frames_dropped)
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
