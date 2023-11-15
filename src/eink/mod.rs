use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

pub mod thread;
pub mod uart_interface;

pub struct EInkInterface {
    pub width: u32,
    pub height: u32,
    pub rx: Receiver<EInkResponse>,
    pub tx: Sender<EInkCommand>,
    pub state: EInkResponse,
    pub _port: &'static str,
    pub flip: bool,
}

#[derive(Debug, Clone)]
pub enum EInkResponse {
    OK,
    READY,
    BUSY,
    ERROR,
    DISCONNECTED,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    LED {
        color: u8,
    },
}

impl EInkInterface {
    #[allow(dead_code)]
    pub(crate) async fn full(&mut self, buffer: Vec<u8>) -> Result<(), SendError<EInkCommand>> {
        self.send_command(EInkCommand::SHOW {
            buffer,
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
            with_red: false,
            black_border: false,
            full_refresh: true,
        })
        .await
    }

    #[allow(dead_code)]
    pub(crate) async fn fast(&mut self, buffer: Vec<u8>) -> Result<(), SendError<EInkCommand>> {
        println!("Fast display on screen {}", self._port);
        self.send_command(EInkCommand::SHOW {
            buffer,
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
            with_red: false,
            black_border: false,
            full_refresh: false,
        })
        .await
    }

    #[allow(dead_code)]
    pub(crate) async fn partial(
        &mut self,
        buffer: Vec<u8>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SendError<EInkCommand>> {
        self.send_command(EInkCommand::SHOW {
            buffer,
            x,
            y,
            width,
            height,
            with_red: false,
            black_border: false,
            full_refresh: false,
        })
        .await
    }

    pub(crate) async fn send_command(
        &mut self,
        command: EInkCommand,
    ) -> Result<(), SendError<EInkCommand>> {
        self.tx.send(command).await
    }
}

impl std::fmt::Display for EInkResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                EInkResponse::OK => "Ok",
                EInkResponse::READY => "Ready",
                EInkResponse::BUSY => "Busy",
                EInkResponse::ERROR => "Error",
                EInkResponse::DISCONNECTED => "Disconnected",
            }
        )
    }
}

impl From<serialport::Error> for EInkResponse {
    fn from(err: serialport::Error) -> EInkResponse {
        match err.kind {
            serialport::ErrorKind::NoDevice => return EInkResponse::DISCONNECTED,
            _ => return EInkResponse::ERROR,
        }
    }
}
impl From<std::io::Error> for EInkResponse {
    fn from(err: std::io::Error) -> EInkResponse {
        match err.kind() {
            std::io::ErrorKind::WouldBlock => return EInkResponse::BUSY,
            _ => return EInkResponse::ERROR,
        }
    }
}
