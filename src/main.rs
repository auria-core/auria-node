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
use auria_network::{P2PNode, p2p::{P2PNetwork, P2PConfig}};
use auria_settlement::{OnChainSettlement, OnChainSettlementConfig};
use auria_network::http::{ClusterCoordinator, ClusterConfig};

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
        #[arg(long)]
        p2p_port: Option<u16>,
        #[arg(long, value_delimiter = ',')]
        bootstrap_nodes: Vec<String>,
        #[arg(long)]
        settlement_rpc_url: Option<String>,
        #[arg(long)]
        settlement_contract: Option<String>,
        #[arg(long)]
        settlement_mnemonic: Option<String>,
        #[arg(long, default_value = "1")]
        chain_id: u64,
        #[arg(long)]
        cluster_id: Option<String>,
        #[arg(long, value_delimiter = ',')]
        cluster_peers: Vec<String>,
        #[arg(long)]
        model_path: Option<String>,
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
        Command::Start { 
            http_port, 
            tier, 
            p2p_port, 
            bootstrap_nodes,
            settlement_rpc_url,
            settlement_contract,
            settlement_mnemonic,
            chain_id,
            cluster_id,
            cluster_peers,
            model_path,
        } => start(
            http_port, 
            tier, 
            p2p_port, 
            bootstrap_nodes,
            settlement_rpc_url,
            settlement_contract,
            settlement_mnemonic,
            chain_id,
            cluster_id,
            cluster_peers,
            model_path,
        ).await?,
    }

    Ok(())
}

async fn status() -> anyhow::Result<()> {
    info!("Auria Node Status");
    info!("Version: 0.1.0");
    info!("Checking connectivity to network services...");
    Ok(())
}

async fn start(
    http_port: u16, 
    tier: String, 
    p2p_port: Option<u16>, 
    bootstrap_nodes: Vec<String>,
    settlement_rpc_url: Option<String>,
    settlement_contract: Option<String>,
    settlement_mnemonic: Option<String>,
    chain_id: u64,
    cluster_id: Option<String>,
    cluster_peers: Vec<String>,
    model_path: Option<String>,
) -> anyhow::Result<()> {
    info!("Starting Auria Node");
    info!("HTTP Port: {}", http_port);
    info!("Default Tier: {}", tier);

    let node_id = uuid::Uuid::new_v4().to_string();
    let p2p_address = format!("0.0.0.0:{}", p2p_port.unwrap_or(9000));
    
    info!("P2P Node ID: {}", node_id);
    info!("P2P Address: {}", p2p_address);
    if !bootstrap_nodes.is_empty() {
        info!("Bootstrap nodes: {:?}", bootstrap_nodes);
    }

    let server = HttpServer::new(http_port);
    let state = server.state().clone();
    
    let inference_service = if let Some(ref model) = model_path {
        info!("Initializing inference service with model: {}", model);
        let service = InferenceService::new();
        match service.load_model(model).await {
            Ok(()) => {
                info!("Model loaded successfully");
                service
            }
            Err(e) => {
                tracing::warn!("Failed to load model: {}, using simulated inference", e);
                InferenceService::new()
            }
        }
    } else {
        info!("No model specified, using simulated inference");
        InferenceService::new()
    };
    state.register_inference_handler(Box::new(inference_service)).await;
    
    let p2p_config = P2PConfig {
        listen_address: "0.0.0.0".to_string(),
        listen_port: p2p_port.unwrap_or(9000),
        bootstrap_nodes: bootstrap_nodes.clone(),
        max_peers: 50,
        enable_discovery: true,
    };
    
    let p2p_network = P2PNetwork::new(p2p_config.clone());
    
    let p2p_node = P2PNode::with_network(
        node_id.clone(),
        p2p_address.clone(),
        p2p_network.clone(),
    );
    
    if let Err(e) = p2p_network.start_server().await {
        tracing::warn!("Failed to start P2P server: {}", e);
    } else {
        info!("P2P server started on {}", p2p_address);
    }
    
    for bootstrap in &bootstrap_nodes {
        let addr = if bootstrap.contains(':') {
            bootstrap.clone()
        } else {
            format!("{}:9000", bootstrap)
        };
        if let Err(e) = p2p_node.connect_p2p(addr.clone()).await {
            tracing::warn!("Failed to connect to bootstrap node {}: {}", addr, e);
        } else {
            info!("Connected to bootstrap node: {}", addr);
        }
    }
    state.set_p2p_node(p2p_node).await;

    if let Some(rpc_url) = settlement_rpc_url {
        let contract_address = settlement_contract.unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());
        
        info!("Initializing on-chain settlement");
        info!("  RPC URL: {}", rpc_url);
        info!("  Contract: {}", contract_address);
        info!("  Chain ID: {}", chain_id);
        
        let config = OnChainSettlementConfig {
            rpc_url,
            settlement_contract_address: contract_address,
            wallet_mnemonic: settlement_mnemonic,
            chain_id,
            settlement_interval_seconds: 3600,
            min_receipts_for_settlement: 10,
            auto_settle: false,
            settle_on_threshold: true,
            threshold_receipts: 100,
        };
        
        match OnChainSettlement::new(config).await {
            Ok(settlement) => {
                match settlement.connect().await {
                    Ok(true) => info!("Connected to Ethereum blockchain"),
                    Ok(false) => info!("Warning: Could not connect to Ethereum (settlement will be disabled)"),
                    Err(e) => tracing::warn!("Failed to connect to Ethereum: {}", e),
                }
                state.set_settlement(settlement).await;
            }
            Err(e) => {
                tracing::warn!("Failed to initialize settlement: {}", e);
            }
        }
    } else {
        info!("Settlement disabled (no RPC URL provided)");
    }

    if let Some(c_id) = cluster_id {
        info!("Initializing cluster coordinator");
        info!("  Cluster ID: {}", c_id);
        
        let cluster_config = ClusterConfig {
            cluster_id: c_id.clone(),
            heartbeat_interval_ms: 1000,
            election_timeout_ms: 5000,
            max_workers: 100,
            task_timeout_seconds: 300,
            failure_detection_threshold: 3,
        };
        
        let mut cluster = ClusterCoordinator::with_config(cluster_config);
        
        if !cluster_peers.is_empty() {
            info!("  Initializing Raft with peers: {:?}", cluster_peers);
            if let Err(e) = cluster.init_raft(cluster_peers.clone()).await {
                tracing::warn!("Failed to initialize Raft: {}", e);
            }
        } else {
            info!("  Raft disabled (no cluster peers provided)");
        }
        
        state.set_cluster(cluster).await;
    } else {
        info!("Cluster disabled (no cluster ID provided)");
    }
    
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
