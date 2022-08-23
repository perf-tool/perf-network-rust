use log::LevelFilter;

use crate::{http, tcp, udp};
use crate::module::{CommType, ProtocolType};

pub struct Config {
    pub comm_type: CommType,
    pub protocol_type: ProtocolType,
}

pub async fn start(log_level: LevelFilter, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .apply().unwrap();
    match config.protocol_type {
        ProtocolType::UDP => {
            udp::start(config.comm_type).await
        }
        ProtocolType::TCP => {
            tcp::start(config.comm_type).await
        }
        ProtocolType::HTTP => {
            http::start(config.comm_type).await
        }
    }
}