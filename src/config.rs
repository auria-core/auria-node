// File: config.rs - This file is part of AURIA
// Copyright (c) 2026 AURIA Developers and Contributors
// Description:
//     Node configuration for AURIA Runtime Core.
//
use auria_core::{PublicKey, Tier};
use auria_settlement::SettlementConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub tier: Tier,
    pub public_key: PublicKey,
    pub data_dir: PathBuf,
    pub http_port: u16,
    pub grpc_port: u16,
    pub metrics_port: u16,
    pub expert_count: u32,
    pub vram_cache_entries: usize,
    pub ram_cache_entries: usize,
    pub cluster_mode: bool,
    pub cluster_id: String,
    pub gpu_enabled: bool,
    pub enable_tracing: bool,
    pub plugin_dirs: Vec<PathBuf>,
    pub settlement: SettlementConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            tier: Tier::Standard,
            public_key: PublicKey([0u8; 32]),
            data_dir: PathBuf::from("./data"),
            http_port: 8080,
            grpc_port: 50051,
            metrics_port: 9090,
            expert_count: 1024,
            vram_cache_entries: 16,
            ram_cache_entries: 256,
            cluster_mode: false,
            cluster_id: String::new(),
            gpu_enabled: false,
            enable_tracing: false,
            plugin_dirs: Vec::new(),
            settlement: SettlementConfig::default(),
        }
    }
}

impl NodeConfig {
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NodeConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
