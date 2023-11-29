// use debug_print::debug_println;
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use tokio::time::sleep;
// use tokio_serial::SerialPortBuilderExt;
use tokio_serial::SerialStream;

use crate::log;

use super::EInkResponse;

const DELIMITER: &str = "\r\n"; // Hardcoded to "\r\n"
const CHUNK_DELAY: u64 = 30; // Delay in milliseconds between sending chunks (adjust as needed).
const CHUNK_SIZE: usize = 1000;

// Define an Interface struct to manage the serial port.
pub struct EInkUartInterface {
    // reader: BufReader<Box<dyn SerialPort>>,
    port: Box<SerialStream>,
}

impl EInkUartInterface {
    pub fn new(port: Box<SerialStream>) -> Result<Self, serialport::Error> {
        // let reader = BufReader::new(port);

        Ok(EInkUartInterface { port })
    }

    pub fn dump_rx(&mut self) {
        self.port
            .clear(serialport::ClearBuffer::Input)
            .expect("Error clearing buffer");
        // self.reader
        //     .get_mut()
        //     .clear(serialport::ClearBuffer::Input)
        //     .expect("Error clearing buffer");
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<EInkResponse, EInkResponse> {
        self.dump_rx();
        self.port.write_all(message)?;
        self.port.flush()?;
        sleep(Duration::from_millis(20)).await; //give it a moment

        let mut response: String = String::new();

        let mut buffer = [0; 1024]; // Adjust the buffer size as needed.
        self.port.readable().await?;

        while let Ok(bytes_read) = self.port.try_read(&mut buffer) {
            if bytes_read == 0 {
                // No more data to read.
                break;
            }

            response.push_str(std::str::from_utf8(&buffer[..bytes_read]).unwrap());

            if response.ends_with(DELIMITER) {
                // If the response ends with the delimiter, it's complete.
                break;
            }
        }

        if response.contains("OK") {
            Ok(EInkResponse::OK)
        } else if response.contains("BUSY") {
            Err(EInkResponse::Busy)
        } else {
            Err(EInkResponse::Error)
        }
    }

    pub async fn send_cmd(&mut self, cmd: &String) -> Result<EInkResponse, EInkResponse> {
        let start = Instant::now();
        println!("{} {}", log::SEND, cmd.trim());
        loop {
            let resp: Result<EInkResponse, EInkResponse> = self.send_message(cmd.as_bytes()).await;
            match resp {
                Ok(data) => {
                    return Ok(data);
                }
                Err(EInkResponse::Busy) if start.elapsed().as_secs() < 20 => {
                    // Busy
                    sleep(Duration::from_millis(100)).await;
                    continue; //Retry
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    pub async fn send_data_in_chunks(&mut self, data: &[u8]) -> Result<String, EInkResponse> {
        // let print_str = format!(
        //     "-> TX {} Bytes in {} chunks",
        //     data.len(),
        //     (data.len() as f32 / CHUNK_SIZE as f32).ceil() as u32
        // );

        // debug_println!("{}", print_str);

        // let start = Instant::now();

        self.dump_rx();
        self.port.writable().await?;

        let mut remaining_data: &[u8] = data;

        while !remaining_data.is_empty() {
            let chunk_size = std::cmp::min(CHUNK_SIZE, remaining_data.len());
            let (chunk, rest) = remaining_data.split_at(chunk_size);

            // println!("rd:{} cs:{}", remaining_data.len(), chunk_size);

            self.port.write_all(chunk)?;
            self.port.write_all(DELIMITER.as_bytes())?;
            self.port.flush()?;

            remaining_data = rest;

            sleep(Duration::from_millis(CHUNK_DELAY)).await; // Add a delay between sending chunks.
        }

        let mut response: String = String::new();

        let mut buffer = [0; 1024]; // Adjust the buffer size as needed.

        self.port.readable().await?;

        while let Ok(bytes_read) = self.port.read(&mut buffer) {
            if bytes_read == 0 {
                // No more data to read.
                break;
            }

            response.push_str(std::str::from_utf8(&buffer[..bytes_read]).unwrap());

            if response.ends_with(DELIMITER) {
                // If the response ends with the delimiter, it's complete.
                break;
            }
        }

        // debug_println!(
        //     "\r{:<33} <- {:<5} {:>4}ms",
        //     print_str.trim(),
        //     response.trim(),
        //     start.elapsed().as_millis()
        // );

        if response.contains("OK") {
            Ok(response.trim().to_string())
        } else if response.contains("BUSY") {
            Err(EInkResponse::Busy)
        } else {
            println!("{} Error: {}", log::ERROR, response);
            // sleep(Duration::from_millis(10));
            Err(EInkResponse::Error)
        }
    }

    #[allow(dead_code)]
    pub async fn reset(&mut self) -> Result<(), EInkResponse> {
        // Bring RTS high for 100ms.

        // self.port.write_data_terminal_ready(true)?;
        self.port.write_request_to_send(true)?;

        sleep(Duration::from_millis(100)).await; //Sleep to reset

        // self.port.write_data_terminal_ready(false)?;

        self.port.write_request_to_send(false)?;
        sleep(Duration::from_millis(100)).await; //Wait for start
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn wait_ready(&mut self) -> Result<EInkResponse, EInkResponse> {
        let cmd: String = "AT+READY=\r\n".to_string();
        self.send_cmd(&cmd).await
    }

    #[allow(dead_code)]
    pub async fn set_led(&mut self, value: u8) -> Result<(), EInkResponse> {
        self.dump_rx();
        let cmd = format!("AT+LED={}\r\n", value);
        let resp = self.send_cmd(&cmd).await;
        if resp.is_err() {
            return Err(EInkResponse::Error);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] //reflects the uart data structure
    pub async fn send_image(
        &mut self,
        data: &[u8], // Full buffer
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        with_red: bool,
        full_refresh: bool,
        border: bool,
    ) {
        // Prepare AT+IMG Command
        let crc: u8 = data.iter().fold(0, |acc, &x| acc.wrapping_add(x));
        // self.dump_rx();

        let cmd = format!(
            "AT+IMG={} {} {} {} {} {}\r\n",
            with_red as u8, x, y, width, height, crc
        );
        if let Err(error) = self.send_cmd(&cmd).await {
            println!("{} Error starting image transfer, {}", log::ERROR, error);
        }
        // sleep(Duration::from_millis(CHUNK_DELAY)); // wait to start transfer

        // self.dump_rx();

        if let Err(error) = self.send_data_in_chunks(data).await {
            println!("{} Error sending data, {}", log::ERROR, error);
        }

        // self.wait_ready();

        let cmd = format!("AT+SHOW={} {}\r\n", full_refresh as u8, border as u8);
        if let Err(error) = self.send_cmd(&cmd).await {
            println!("{} Error showing image, {}", log::ERROR, error);
        }
        sleep(Duration::from_millis(250)).await; //Wait for start
    }
}
