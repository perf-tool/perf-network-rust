// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::net::SocketAddr;

use log::LevelFilter;

use crate::{http, tcp, udp};
use crate::module::{CommType, ProtocolType};

pub struct Config {
    pub comm_type: CommType,
    pub protocol_type: ProtocolType,
    pub prometheus_metrics_disable: bool,
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
    if !config.prometheus_metrics_disable {
        let socket_addr = SocketAddr::from(([0, 0, 0, 0], 20008));
        prometheus_exporter::start(socket_addr)?;
    }
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
