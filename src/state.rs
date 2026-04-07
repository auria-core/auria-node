// File: state.rs - This file is part of AURIA
// Copyright (c) 2026 AURIA Developers and Contributors
// Description:
//     Node runtime state for AURIA Runtime Core.
//
use auria_core::{PublicKey, Tier};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct NodeState {
    pub tier: Tier,
    pub public_key: PublicKey,
    pub started_at: std::time::SystemTime,
    pub last_heartbeat: std::time::SystemTime,
    pub requests_processed: u64,
    pub requests_failed: u64,
    pub tokens_generated: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub active_requests: usize,
}

impl NodeState {
    pub fn new(tier: Tier, public_key: PublicKey) -> Self {
        Self {
            tier,
            public_key,
            started_at: std::time::SystemTime::now(),
            last_heartbeat: std::time::SystemTime::now(),
            requests_processed: 0,
            requests_failed: 0,
            tokens_generated: 0,
            cache_hits: 0,
            cache_misses: 0,
            active_requests: 0,
        }
    }

    pub fn uptime(&self) -> u64 {
        self.started_at.elapsed().map(|d| d.as_secs()).unwrap_or(0)
    }

    pub fn increment_requests(&mut self) {
        self.requests_processed += 1;
    }

    pub fn increment_failures(&mut self) {
        self.requests_failed += 1;
    }

    pub fn increment_tokens(&mut self, count: u64) {
        self.tokens_generated += count;
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    pub fn start_request(&mut self) {
        self.active_requests += 1;
    }

    pub fn end_request(&mut self) {
        self.active_requests = self.active_requests.saturating_sub(1);
    }

    pub fn is_healthy(&self) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed.as_secs() < 120
        } else {
            false
        }
    }
}
