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

use std::time::Instant;

use lazy_static::lazy_static;
use prometheus_exporter::prometheus::{register_counter, register_histogram};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericCounter};
use prometheus_exporter::prometheus::Histogram;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

lazy_static! {
    static ref SEND_SUCCESS_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_tcp_server_send_success_total", "tcp server send success total").expect("can not create counter tcp_server_send_success_total");
    static ref SEND_FAIL_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_tcp_server_send_fail_total", "tcp server send fail total").expect("can not create counter tcp_server_send_fail_total");
    static ref SEND_SUCCESS_LATENCY: Histogram = register_histogram!("perf_network_tcp_server_send_success_latency_ms", "tcp server send success latency").expect("can not create histogram tcp_server_send_success_latency_ms");
}

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:5678";

    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop.
    let listener = TcpListener::bind(&addr).await?;
    log::info!("Listening on: {}", addr);

    loop {
        // Asynchronously wait for an inbound socket.
        let (mut socket, _) = listener.accept().await?;

        // And this is where much of the magic of this server happens. We
        // crucially want all clients to make progress concurrently, rather than
        // blocking one on completion of another. To achieve this we use the
        // `tokio::spawn` function to execute the work in the background.
        //
        // Essentially here we're executing a new task to run concurrently,
        // which will allow all of our clients to be processed concurrently.

        tokio::spawn(async move {
            let mut buf = vec![0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    return;
                }
                log::debug!("Echoed {} bytes {}", n, socket.peer_addr().unwrap().to_string());

                let start = Instant::now();
                let result = socket.write_all(&buf[0..n]).await;
                match result {
                    Ok(_) => {
                        SEND_SUCCESS_COUNT.inc();
                        SEND_SUCCESS_LATENCY.observe(start.elapsed().as_millis() as f64);
                    }
                    Err(_) => {
                        SEND_FAIL_COUNT.inc();
                        log::error!("failed to write data to socket {}", socket.peer_addr().unwrap().to_string())
                    }
                }
            }
        });
    }
}