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
use std::time::Instant;

use futures::Stream;
use hyper::{Body, body, Method, Request, Response, StatusCode};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use lazy_static::lazy_static;
use prometheus_exporter::prometheus::{register_counter, register_histogram};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericCounter};
use prometheus_exporter::prometheus::Histogram;
use tokio::net::TcpListener;

lazy_static! {
    static ref RECV_BYTES_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_http_server_recv_bytes_total", "http server recv bytes total").expect("can not create counter perf_network_http_server_recv_bytes_total");
    static ref SEND_BYTES_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_http_server_send_bytes_total", "http server send bytes total").expect("can not create counter perf_network_http_server_send_bytes_total");
    static ref SEND_SUCCESS_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_http_server_send_success_total", "http server send success total").expect("can not create counter http_server_send_success_total");
    static ref SEND_FAIL_COUNT: GenericCounter<AtomicF64> = register_counter!("perf_network_http_server_send_fail_total", "http server send fail total").expect("can not create counter http_server_send_fail_total");
    static ref SEND_SUCCESS_LATENCY: Histogram = register_histogram!("perf_network_http_server_send_success_latency_ms", "http server send success latency").expect("can not create histogram http_server_send_success_latency_ms");
}

async fn serve(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/perf") => {
            let start = Instant::now();
            let max = req.body().size_hint().1.unwrap_or(0);
            if max > 64 * 1024 {
                SEND_FAIL_COUNT.inc();
                let mut resp = Response::new(Body::from("Body too big"));
                *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                return Ok(resp);
            }

            let whole_body = body::to_bytes(req.into_body()).await?;

            let reversed_body = whole_body.iter().rev().cloned().collect::<Vec<u8>>();
            RECV_BYTES_COUNT.inc_by(reversed_body.len() as f64);
            SEND_BYTES_COUNT.inc_by(reversed_body.len() as f64);
            SEND_SUCCESS_COUNT.inc();
            SEND_SUCCESS_LATENCY.observe(start.elapsed().as_millis() as f64);
            Ok(Response::new(Body::from(reversed_body)))
        }
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 5678));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(e) = Http::new().serve_connection(stream, service_fn(serve)).await {
                log::error!("server error: {}", e);
            }
        });
    }
}
