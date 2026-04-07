// File: lib.rs - This file is part of AURIA
// Tests for auria-node
//
#[cfg(test)]
mod tests {
    use super::*;
    use auria_core::{Tier, PublicKey, RequestId, ShardId, ExpertId, Tensor, TensorDType};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_node_state_creation() {
        let state = NodeState::new(Tier::Standard, PublicKey([1u8; 32]));
        
        assert_eq!(state.tier, Tier::Standard);
        assert_eq!(state.requests_processed, 0);
        assert_eq!(state.requests_failed, 0);
        assert!(state.uptime() >= 0);
    }

    #[tokio::test]
    async fn test_node_state_increment_requests() {
        let mut state = NodeState::new(Tier::Pro, PublicKey([2u8; 32]));
        
        state.increment_requests();
        state.increment_requests();
        
        assert_eq!(state.requests_processed, 2);
    }

    #[tokio::test]
    async fn test_node_state_increment_failures() {
        let mut state = NodeState::new(Tier::Nano, PublicKey([3u8; 32]));
        
        state.increment_failures();
        
        assert_eq!(state.requests_failed, 1);
    }

    #[tokio::test]
    async fn test_node_state_increment_tokens() {
        let mut state = NodeState::new(Tier::Max, PublicKey([4u8; 32]));
        
        state.increment_tokens(100);
        state.increment_tokens(50);
        
        assert_eq!(state.tokens_generated, 150);
    }

    #[tokio::test]
    async fn test_node_state_cache_tracking() {
        let mut state = NodeState::new(Tier::Standard, PublicKey([5u8; 32]));
        
        state.record_cache_hit();
        state.record_cache_hit();
        state.record_cache_miss();
        
        assert_eq!(state.cache_hits, 2);
        assert_eq!(state.cache_misses, 1);
        assert!((state.cache_hit_rate() - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_node_state_active_requests() {
        let mut state = NodeState::new(Tier::Standard, PublicKey([6u8; 32]));
        
        state.start_request();
        state.start_request();
        
        assert_eq!(state.active_requests, 2);
        
        state.end_request();
        
        assert_eq!(state.active_requests, 1);
    }

    #[tokio::test]
    async fn test_node_state_health() {
        let state = NodeState::new(Tier::Standard, PublicKey([7u8; 32]));
        
        assert!(state.is_healthy());
    }

    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        
        assert_eq!(config.tier, Tier::Standard);
        assert_eq!(config.http_port, 8080);
        assert_eq!(config.grpc_port, 50051);
        assert_eq!(config.metrics_port, 9090);
        assert_eq!(config.expert_count, 1024);
    }

    #[test]
    fn test_node_config_serialization() {
        let config = NodeConfig {
            tier: Tier::Pro,
            public_key: PublicKey([8u8; 32]),
            data_dir: std::path::PathBuf::from("/data"),
            http_port: 9000,
            grpc_port: 50052,
            metrics_port: 9091,
            expert_count: 2048,
            vram_cache_entries: 32,
            ram_cache_entries: 512,
            cluster_mode: true,
            cluster_id: "test-cluster".to_string(),
            gpu_enabled: true,
            enable_tracing: true,
            plugin_dirs: vec![],
            settlement: SettlementConfig::default(),
        };
        
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: NodeConfig = toml::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.tier, Tier::Pro);
        assert_eq!(deserialized.http_port, 9000);
        assert!(deserialized.cluster_mode);
    }

    #[tokio::test]
    async fn test_node_status() {
        let state = NodeState::new(Tier::Max, PublicKey([9u8; 32]));
        let state = Arc::new(RwLock::new(state));
        
        {
            let mut s = state.write().await;
            s.increment_requests();
            s.increment_tokens(100);
        }
        
        let status = NodeStatus {
            running: true,
            tier: Tier::Max,
            uptime_seconds: 100,
            requests_processed: 1,
            requests_failed: 0,
            cluster_enabled: true,
        };
        
        assert!(status.running);
        assert_eq!(status.requests_processed, 1);
    }

    #[test]
    fn test_node_builder_default() {
        let builder = NodeBuilder::new();
        let config = builder.config;
        
        assert_eq!(config.tier, Tier::Standard);
    }

    #[test]
    fn test_node_builder_with_tier() {
        let builder = NodeBuilder::new()
            .with_tier(Tier::Pro)
            .with_http_port(8081);
        
        assert_eq!(builder.config.tier, Tier::Pro);
        assert_eq!(builder.config.http_port, 8081);
    }

    #[test]
    fn test_node_builder_with_cluster() {
        let builder = NodeBuilder::new()
            .with_cluster_mode(true, "prod-cluster".to_string());
        
        assert!(builder.config.cluster_mode);
        assert_eq!(builder.config.cluster_id, "prod-cluster");
    }

    #[test]
    fn test_node_builder_with_gpu() {
        let builder = NodeBuilder::new()
            .with_gpu(true);
        
        assert!(builder.config.gpu_enabled);
    }

    #[test]
    fn test_node_builder_data_dir() {
        let builder = NodeBuilder::new()
            .with_data_dir(std::path::PathBuf::from("/custom/path"));
        
        assert_eq!(builder.config.data_dir, std::path::PathBuf::from("/custom/path"));
    }
}
