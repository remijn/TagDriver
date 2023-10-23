use debug_print::debug_println;
use serialport::SerialPort;
use std::io::{BufReader, Error, ErrorKind, Read, Write};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const DELIMITER: &str = "\r\n"; // Hardcoded to "\r\n"
const CHUNK_DELAY: u64 = 30; // Delay in milliseconds between sending chunks (adjust as needed).
const CHUNK_SIZE: usize = 1000;

#[allow(dead_code)]
const BUSY_DELAY: u64 = 50;

#[allow(dead_code)]
const BUFFER_SIZE: u32 = 250 * 128 * 2 / 8; // 2x for red channel

// Define an Interface struct to manage the serial port.
pub struct Interface {
    reader: BufReader<Box<dyn SerialPort>>,
}

impl Interface {
    pub fn new(port: Box<dyn SerialPort>) -> Result<Self, serialport::Error> {

        


        let reader = BufReader::new(port);

        Ok(Interface { reader })
    }

    pub fn dump_rx(&mut self) {
        self.reader
            .get_mut()
            .clear(serialport::ClearBuffer::Input)
            .expect("Error clearing buffer");
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<String, std::io::Error> {
        self.dump_rx();
        self.reader.get_mut().write_all(message)?;
        self.reader.get_mut().flush()?;

        let mut response: String = String::new();

        let mut buffer = [0; 1024]; // Adjust the buffer size as needed.

        while let Ok(bytes_read) = self.reader.read(&mut buffer) {
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
            Ok(response.trim().to_string())
        } else if response.contains("BUSY") {
            Err(Error::new(
                std::io::ErrorKind::WouldBlock,
                response.trim().to_string(),
            ))
        } else {
            Err(Error::new(
                std::io::ErrorKind::InvalidData,
                response.trim().to_string(),
            ))
        }
    }

    // async fn do_send_cmd(&mut self, cmd: &String) -> Result<String, std::io::Error> {
    //     let start = Instant::now();

    //     let resp: Result<String, Error> = self.send_message(cmd.as_bytes()).await;
    //     match resp {
    //         Ok(response) => {
    //             debug_println!(
    //                 "\r-> {:<30} <- {:<8} {}ms",
    //                 cmd.trim(),
    //                 response.trim(),
    //                 start.elapsed().as_millis()
    //             );
    //             return Ok(response);
    //         }
    //         Err(error)
    //             if error.kind() == ErrorKind::WouldBlock && start.elapsed().as_secs() < 20 =>
    //         {
    //             // Recursivly handle busy
    //             sleep(Duration::from_millis(BUSY_DELAY)).await; // sleep before re-calling
    //                                                             // let resp = self.send_cmd_rec(cmd, start, d + 1).await;
    //                                                             // return resp;
    //         }
    //         Err(error) => {
    //             debug_println!(
    //                 "\r-> {:<30} <- {:<9} {}ms",
    //                 cmd.trim(),
    //                 error,
    //                 // d,
    //                 start.elapsed().as_millis()
    //             );
    //             return Err(error);
    //         }
    //     }

    //     return Ok(cmd.clone());
    // }

    // async fn send_cmd_rec(
    //     &mut self,
    //     cmd: &String,
    //     start: Instant,
    //     d: u32,
    // ) -> BoxFuture<'static, Result<String, std::io::Error>> {
    //     if d == 0 {
    //         debug_print!(
    //             "-> {:<30} <- {:>10}ms",
    //             cmd.trim(),
    //             start.elapsed().as_millis()
    //         );
    //     }

    //     let resp: Result<String, Error> = self.send_message(cmd.as_bytes());
    //     match resp {
    //         Ok(response) => {
    //             debug_println!(
    //                 "\r-> {:<30} <- {:<8} {}ms",
    //                 cmd.trim(),
    //                 response.trim(),
    //                 start.elapsed().as_millis()
    //             );
    //             return Ok(response);
    //         }
    //         Err(error)
    //             if error.kind() == ErrorKind::WouldBlock && start.elapsed().as_secs() < 20 =>
    //         {
    //             // Recursivly handle busy
    //             sleep(Duration::from_millis(BUSY_DELAY)).await; // sleep before re-calling
    //             let resp = self.send_cmd_rec(cmd, start, d + 1).await;
    //             return resp;
    //         }
    //         Err(error) => {
    //             debug_println!(
    //                 "\r-> {:<30} <- {:<6} {} {}ms",
    //                 cmd.trim(),
    //                 error,
    //                 d,
    //                 start.elapsed().as_millis()
    //             );
    //             return Err(error);
    //         }
    //     }
    // }

    pub async fn send_cmd(&mut self, cmd: &String) -> Result<String, std::io::Error> {
        let start = Instant::now();
        debug_println!(
            "-> {:<30} <- {:>10}ms",
            cmd.trim(),
            start.elapsed().as_millis()
        );
        loop {
            let resp: Result<String, Error> = self.send_message(cmd.as_bytes()).await;
            match resp {
                Ok(data) => {
                    return Ok(data);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock && start.elapsed().as_secs() < 20 => {
                    // Busy
                    sleep(Duration::from_millis(50)).await;
                    continue; //Retry
                }
                Err(e) => {
                    debug_println!(
                        "\r-> {:<30} <- {:<8} {}ms",
                        cmd.trim(),
                        e.to_string(),
                        start.elapsed().as_millis()
                    );
                    return Err(e);
                }
            }
        }
    }

    pub async fn send_data_in_chunks(&mut self, data: &[u8]) -> Result<String, std::io::Error> {
        let print_str = format!(
            "-> TX {} Bytes in {} chunks",
            data.len(),
            (data.len() as f32 / CHUNK_SIZE as f32).ceil() as u32
        );

        debug_println!("{}", print_str);

        let start = Instant::now();

        self.reader.get_mut().flush()?;

        let mut remaining_data: &[u8] = data;

        while !remaining_data.is_empty() {
            let chunk_size = std::cmp::min(CHUNK_SIZE, remaining_data.len());
            let (chunk, rest) = remaining_data.split_at(chunk_size);

            // println!("rd:{} cs:{}", remaining_data.len(), chunk_size);

            self.reader.get_mut().write_all(chunk)?;
            self.reader.get_mut().write_all(DELIMITER.as_bytes())?;
            self.reader.get_mut().flush()?;

            remaining_data = rest;

            sleep(Duration::from_millis(CHUNK_DELAY)).await; // Add a delay between sending chunks.
        }

        let mut response: String = String::new();

        let mut buffer = [0; 1024]; // Adjust the buffer size as needed.

        while let Ok(bytes_read) = self.reader.read(&mut buffer) {
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

        debug_println!(
            "\r{:<33} <- {:<5} {:>4}ms",
            print_str.trim(),
            response.trim(),
            start.elapsed().as_millis()
        );

        if response.contains("OK") {
            Ok(response.trim().to_string())
        } else if response.contains("BUSY") {
            Err(Error::new(
                std::io::ErrorKind::WouldBlock,
                response.trim().to_string(),
            ))
        } else {
            println!("!!! Error {} !!!", response);
            // sleep(Duration::from_millis(10));
            Err(Error::new(
                std::io::ErrorKind::InvalidData,
                response.trim().to_string(),
            ))
        }
    }

    #[allow(dead_code)]
    pub async fn reset(&mut self) -> Result<(), std::io::Error> {
        // Bring DTS and RTS high for 1 second.
        self.reader.get_mut().write_data_terminal_ready(true)?;
        self.reader.get_mut().write_request_to_send(true)?;
        sleep(Duration::from_millis(100)).await; //Sleep to reset
        self.reader.get_mut().write_data_terminal_ready(false)?;
        self.reader.get_mut().write_request_to_send(false)?;
        sleep(Duration::from_millis(100)).await; //Wait for start
        Ok(())
    }

    pub async fn wait_ready(&mut self) {
        let cmd: String = "AT+READY=\r\n".to_string();
        self.send_cmd(&cmd).await.expect("Timed out!!!");
    }

    #[allow(dead_code)]
    pub async fn set_led(&mut self, value: u8) -> Result<(), Error> {
        self.dump_rx();
        let cmd = format!("AT+LED={}\r\n", value);
        let resp = self.send_cmd(&cmd).await;
        if resp.is_err() {
            return Err(resp.expect_err(""));
        }
        return Ok(());
    }

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
        let crc: u8 = data.into_iter().fold(0, |acc, &x| acc.wrapping_add(x));
        // self.dump_rx();

        let cmd = format!(
            "AT+IMG={} {} {} {} {} {}\r\n",
            with_red as u8, x, y, width, height, crc
        );
        if let Err(_error) = self.send_cmd(&cmd).await {
            println!("Error starting image transfer");
        }
        // sleep(Duration::from_millis(CHUNK_DELAY)); // wait to start transfer

        // self.dump_rx();

        if let Err(_error) = self.send_data_in_chunks(data).await {
            println!("Error sending data");
        }

        // self.wait_ready();

        let cmd = format!("AT+SHOW={} {}\r\n", full_refresh as u8, border as u8);
        if let Err(_error) = self.send_cmd(&cmd).await {
            println!("Error showing image");
        }
        sleep(Duration::from_millis(400)).await; //Wait for start
    }
}
