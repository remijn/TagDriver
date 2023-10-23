use std::time::{Duration, Instant};

mod display;
mod eink;
mod zbus_interface;

use debug_print::debug_println;

use display::generate_image;
use eink::thread::{EInkCommand, EInkResponse};

use embedded_graphics::prelude::{Point, Size};
use serialport::{DataBits, Parity, StopBits};
use tokio::sync::mpsc::{self, Receiver, Sender};

use eink::thread;
use tokio_stream::StreamExt;
use zbus::Connection;

// const IMG: &str = "AT+IMG={bytes} {x} {y} {width} {height} {checksum}\r\n";
// const SHOW: &str = "AT+SHOW=1\r\n";

const WIDTH: u32 = 250;
const REAL_HEIGHT: u32 = 122;
const HEIGHT: u32 = 128;

async fn main_thread(tx: Sender<EInkCommand>, mut rx: Receiver<EInkResponse>) {
    let mut state: EInkResponse = EInkResponse::READY;
    let mut display = display::bwr_display::BWRDisplay::new();

    generate_image(&mut display, 0).expect("?");
    let mut br_buffers = display.get_fixed_buffer();
    let both = [br_buffers.0, br_buffers.1].concat();
    let mut update = EInkCommand::full(both);
    tx.send(update).await.expect("Error sending to main thread");

    let start = Instant::now();

    #[allow(unused_variables)]
    let interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        // Proccess from the serial thread without blocking
        if let Ok(message) = rx.try_recv() {
            debug_println!("main got state {}", message);
            // Handle the received message.
            state = message;
        }

        match state {
            EInkResponse::READY => {
                state = EInkResponse::BUSY; //Asume the state here

                let i = (start.elapsed().as_millis() as f32 / 1000.0 % 5.0 * 20.0) as u32;

                generate_image(&mut display, i).expect("?");

                let p = Point::new(100, 0);
                let s = Size::new(50, 128);

                br_buffers = display.get_fixed_buffer();
                let partial = display.partial_buffer(br_buffers.0.as_slice(), p, s);

                // both = [br_buffers.0, br_buffers.1].concat();
                update = EInkCommand::partial(partial, p.x as u32, p.y as u32, s.width, s.height);
                tx.send(update).await.expect("Error sending to main thread");
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --------- UART SETUP ---------
    // Create the serial port
    let mut port = tokio_serial::new(eink::PORT, eink::BAUD_RATE)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect(format!("Failed to connect to device {}", eink::PORT).as_str());

    const CONFIG_ERROR: &str = "error configuring port";
    port.set_data_bits(DataBits::Eight).expect(CONFIG_ERROR);
    port.set_stop_bits(StopBits::One).expect(CONFIG_ERROR);
    port.set_parity(Parity::None).expect(CONFIG_ERROR);

    let port_clone = port.try_clone().expect("Could not clone port");

    // Create a channel for communication between threads
    let (thread_tx, rx) = mpsc::channel::<EInkResponse>(512);
    let (tx, thread_rx) = mpsc::channel::<EInkCommand>(1024);

    // Spawn the serial thread
    tokio::spawn(async move {
        thread::run_thread(port_clone, thread_tx, thread_rx)
            .await
            .expect("Could not spawn thread");
    });

    // --------- DBUS SETUP ---------

    let conn: Connection = Connection::session().await?;

    let player = zbus_interface::playerctld::PlayerProxy::new(&conn).await?;

    let mut position = player.receive_position_changed().await;

    tokio::spawn(async move {
        while let Some(v) = position.next().await {
            println!("Position changed: {:?}", v.get().await);
        }
    });

    let mut volume = player.receive_volume_changed().await;

    tokio::spawn(async move {
        while let Some(v) = volume.next().await {
            println!("Volume changed: {:?}", v.get().await);
        }
    });

    let mut status = player.receive_playback_status_changed().await;

    tokio::spawn(async move {
        while let Some(v) = status.next().await {
            println!("Status changed: {:?}", v.get().await);
        }
    });

    // Run the main thread
    main_thread(tx, rx).await;

    Ok(())
}
