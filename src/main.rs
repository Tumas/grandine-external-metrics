use core::time::Duration;

use anyhow::Result;
use axum::{routing::get, Router};
use clap::Parser;
use prometheus::{Gauge, TextEncoder};
use sysinfo::{Pid, System};

#[derive(Parser)]
pub struct Args {
    #[arg(short, long)]
    pid: Pid,
    #[arg(long, default_value_t = 5054)]
    port: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args { pid, port } = Args::parse();

    let total_cpu_percentage = Gauge::new(
        "GRANDINE_TOTAL_CPU_PERCENTAGE",
        "Grandine CPU load usage measured in percentage",
    )?;

    let default_registry = prometheus::default_registry();
    default_registry.register(Box::new(total_cpu_percentage.clone()))?;

    tokio::task::spawn(async move {
        let mut system = System::new_all();
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        system.refresh_all();

        loop {
            system.refresh_all();

            let grandine = system
                .process(pid)
                .expect("the current process should always be available");

            total_cpu_percentage.set(grandine.cpu_usage() as f64);

            interval.tick().await;
        }
    });

    let router = Router::new().route("/metrics", get(prometheus_metrics));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    axum::serve(listener, router).await.unwrap();

    Ok(())
}

async fn prometheus_metrics() -> String {
    let mut buffer = String::new();

    TextEncoder::new()
        .encode_utf8(prometheus::gather().as_slice(), &mut buffer)
        .expect("unable to gather metrics");

    buffer
}
