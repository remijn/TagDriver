pub mod interface;
pub mod thread;

pub const BAUD_RATE: u32 = 921600; //115200;
pub const PORT: &str = "/dev/ttyUSB0";

#[derive(Debug, Clone)]
pub enum EInkResponse {
    OK,
    READY,
    BUSY,
    ERROR,
    DISCONNECTED,
}

#[derive(Debug, Clone)]
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
