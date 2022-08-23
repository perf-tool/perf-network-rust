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

use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use lazy_static::lazy_static;
use prometheus_exporter::prometheus::{register_counter, register_histogram};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericCounter};
use prometheus_exporter::prometheus::Histogram;
use tokio::net::UdpSocket;

lazy_static! {
    static ref RECV_BYTES_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_udp_server_recv_bytes_total", "udp server recv bytes total").expect("can not create counter perf_network_udp_server_recv_bytes_total");
    static ref SEND_BYTES_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_udp_server_send_bytes_total", "udp server send bytes total").expect("can not create counter perf_network_udp_server_send_bytes_total");
    static ref SEND_SUCCESS_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_udp_server_send_success_total", "udp server send success total").expect("can not create counter udp_server_send_success_total");
    static ref SEND_FAIL_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_udp_server_send_fail_total", "udp server send fail total").expect("can not create counter udp_server_send_fail_total");
    static ref SEND_SUCCESS_LATENCY: Histogram = register_histogram!("perf_network_udp_server_send_success_latency_ms", "udp server send success latency").expect("can not create histogram udp_server_send_success_latency_ms");
}

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
    to_send: Option<(usize, SocketAddr)>
}

impl Server {
    async fn run(self) -> Result<(), io::Error> {
        let Server {
            socket,
            mut buf,
            mut to_send,
        } = self;
        loop {
            // First we check to see if there's a message we need to echo back.
            // If so then we try to send it back to the original source, waiting
            // until it's writable and we're able to do so.
            if let Some((size, peer)) = to_send {
                RECV_BYTES_COUNT.inc_by(size as f64);
                let start = Instant::now();
                let result = socket.send_to(&buf[..size], &peer).await;

                match result {
                    Ok(send_size) => {
                        SEND_BYTES_COUNT.inc_by(send_size as f64);
                        SEND_SUCCESS_COUNT.inc();
                        SEND_SUCCESS_LATENCY.observe(start.elapsed().as_millis() as f64);
                        log::debug!("Echoed {}/{} bytes to {}", send_size, size, peer);
                    }
                    Err(_) => {
                        SEND_FAIL_COUNT.inc();
                        log::error!("failed to write data to {}", peer);
                    }
                }

            }

            // If we're here then `to_send` is `None`, so we take a look for the
            // next message we're going to echo back.
            to_send = Some(socket.recv_from(&mut buf).await?);
        }
    }
}

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:5678").await?;
    let server = Server {
        socket,
        buf: vec![0; 1024],
        to_send: None,
    };

    // This starts the server task.
    server.run().await?;

    Ok(())
}
