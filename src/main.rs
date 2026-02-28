// File: main.rs - This file is part of AURIA
// Copyright (c) 2026 AURIA Developers and Contributors
// Description:
//     Full node binary for AURIA Runtime Core.
//     Entry point for running a complete Auria node that combines all
//     subsystems including execution, routing, storage, licensing, and networking.
//
use clap::{Parser, Subcommand};
use tracing::info;

use auria_core::{
    ExecutionState, RequestId, ShardId, Tier, UsageReceipt,
};
use auria_router::{DeterministicRouter, Router};
use auria_execution::ExecutionEngine;
use auria_storage::Storage;
use auria_license::LicenseManager;
use auria_settlement::SettlementClient;

#[derive(Parser, Debug)]
#[command(name = "auria", version, about = "Auria Node — Decentralized LLM Runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Start {
        #[arg(long, default_value = "nano")]
        tier: String,
    },
    Status,
}

fn parse_tier(s: &str) -> Tier {
    match s.to_ascii_lowercase().as_str() {
        "nano" => Tier::Nano,
        "standard" => Tier::Standard,
        "pro" => Tier::Pro,
        "max" => Tier::Max,
        _ => Tier::Nano,
    }
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
        Command::Start { tier } => start(parse_tier(&tier)).await?,
    }

    Ok(())
}

async fn status() -> anyhow::Result<()> {
    info!("Auria Node Status");
    info!("Version: {}.{}.{}.{}.{}", 1, 0, 0, 0, 0);
    Ok(())
}

async fn start(preferred: Tier) -> anyhow::Result<()> {
    info!("Starting Auria Node with tier: {:?}", preferred);

    let router = DeterministicRouter;
    let storage = Storage::new(1000);
    let license_manager = LicenseManager::new();
    let settlement_client = SettlementClient::new();

    info!(
        "Auria Node started successfully (tier={:?})",
        preferred
    );

    let mut state = ExecutionState {
        position: 0,
        kv_cache: Vec::new(),
    };
    let request_id = RequestId([0u8; 16]);

    for token_index in 0..5u64 {
        let decision = router.route(preferred, token_index);
        info!("Token {} routed to {:?}", token_index, decision.expert_ids);
        state.position += 1;
    }

    info!("Demo execution completed");
    Ok(())
}
