// File: main.rs - This file is part of AURIA
// Copyright (c) 2026 AURIA Developers and Contributors
// Description:
//     Full node binary for AURIA Runtime Core.
//     Entry point for running a complete Auria node that combines all
//     subsystems including execution, routing, storage, licensing, and networking.
//
use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use tracing::info;

use auria_network::http::HttpServer;
use auria_network::InferenceService;

#[derive(Parser, Debug)]
#[command(name = "auria", version, about = "Auria Node — Decentralized LLM Runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Start {
        #[arg(long, default_value = "8080")]
        http_port: u16,
        #[arg(long, default_value = "nano")]
        tier: String,
    },
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.cmd {
        Command::Status => status().await?,
        Command::Start { http_port, tier } => start(http_port, tier).await?,
    }

    Ok(())
}

async fn status() -> anyhow::Result<()> {
    info!("Auria Node Status");
    info!("Version: 0.1.0");
    info!("Checking connectivity to network services...");
    Ok(())
}

async fn start(http_port: u16, tier: String) -> anyhow::Result<()> {
    info!("Starting Auria Node");
    info!("HTTP Port: {}", http_port);
    info!("Default Tier: {}", tier);

    let server = HttpServer::new(http_port);
    let state = server.state().clone();
    
    let inference_service = InferenceService::new();
    state.register_inference_handler(Box::new(inference_service)).await;
    
    let bind_addr: SocketAddr = format!("0.0.0.0:{}", http_port).parse()?;
    
    info!("Starting HTTP server on {}", bind_addr);
    if let Err(e) = server.start(bind_addr).await {
        tracing::error!("Failed to start HTTP server: {}", e);
        return Err(anyhow::anyhow!("Server error: {}", e));
    }
    
    info!("Auria Node is running. Press Ctrl+C to stop.");
    
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}
