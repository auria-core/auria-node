// File: handlers.rs - This file is part of AURIA
// Copyright (c) 2026 AURIA Developers and Contributors
// Description:
//     Request handlers for AURIA Runtime Core.
//
use crate::state::NodeState;

use auria_backend_cpu::CpuBackendImpl;
use auria_cluster::ClusterCoordinator;
use auria_core::{
    AuriaError, AuriaResult, ExecutionOutput, ExecutionState, ExpertId,
    RequestId, RoutingDecision, Tensor, TensorDType, Tier,
};
use auria_execution::{ExecutionBackend, ShardStorage};
use auria_license::LicenseManager;
use auria_network::{InferenceRequest, InferenceResponse, RequestHandler, UsageInfo};
use auria_observability::MetricsCollector;
use auria_router::{DeterministicRouter, Router};
use auria_settlement::SettlementClient;
use auria_storage::MultiTierStorage;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct InferenceRequestHandler {
    state: Arc<RwLock<NodeState>>,
    router: Arc<DeterministicRouter>,
    storage: Arc<MultiTierStorage>,
    settlement: Arc<SettlementClient>,
    license: Arc<LicenseManager>,
    cluster: Arc<RwLock<Option<ClusterCoordinator>>>,
    metrics: Arc<MetricsCollector>,
    backend: CpuBackendImpl,
}

impl InferenceRequestHandler {
    pub fn new(
        state: Arc<RwLock<NodeState>>,
        router: Arc<DeterministicRouter>,
        storage: Arc<MultiTierStorage>,
        settlement: Arc<SettlementClient>,
        license: Arc<LicenseManager>,
        cluster: Arc<RwLock<Option<ClusterCoordinator>>>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            state,
            router,
            storage,
            settlement,
            license,
            cluster,
            metrics,
            backend: CpuBackendImpl::new(),
        }
    }

    async fn execute_inference(
        &self,
        tier: Tier,
        prompt: &str,
        max_tokens: u32,
    ) -> AuriaResult<ExecutionOutput> {
        let routing = self.router.route(tier, 0);
        
        info!("Executing inference with {} experts", routing.expert_ids.len());

        let input = self.tokenize(prompt);
        
        let mut exec_state = ExecutionState {
            position: 0,
            kv_cache: Vec::new(),
        };

        let mut all_tokens = Vec::new();
        
        for step in 0..max_tokens {
            let routing = self.router.route(tier, step as u64);
            
            let output = self.backend.execute_step(
                input.clone(),
                Vec::new(),
                exec_state.clone(),
            ).await?;

            all_tokens.extend(output.tokens.clone());
            exec_state.position += 1;
        }

        Ok(ExecutionOutput {
            tokens: all_tokens,
            usage: auria_core::UsageStats {
                tokens_generated: max_tokens,
            },
        })
    }

    fn tokenize(&self, text: &str) -> Tensor {
        let chars: Vec<u8> = text.bytes().collect();
        let data: Vec<u8> = chars
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    let low = chunk[0] & 0xFF;
                    let high = (chunk[1] & 0xFF) << 8;
                    low | high
                } else {
                    chunk[0]
                }
            })
            .collect();

        let len = data.len();
        Tensor {
            data,
            shape: vec![1, len as u32],
            dtype: TensorDType::INT8,
        }
    }

    fn detokenize(&self, tensor: &Tensor) -> Vec<String> {
        let chars: Vec<char> = tensor
            .data
            .iter()
            .flat_map(|&b| {
                let c1 = (b & 0x7F) as char;
                if c1.is_ascii_graphic() || c1 == ' ' {
                    Some(c1)
                } else {
                    None
                }
            })
            .collect();

        let words: Vec<String> = String::from_iter(chars)
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if words.is_empty() {
            vec!["[generated]".to_string()]
        } else {
            words
        }
    }

    async fn verify_licenses(&self, _expert_ids: &[ExpertId]) -> AuriaResult<bool> {
        Ok(true)
    }

    async fn process_payment(&self, _expert_ids: &[ExpertId], _tokens: u32) -> AuriaResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl RequestHandler for InferenceRequestHandler {
    async fn handle_request(&self, request: InferenceRequest) -> AuriaResult<InferenceResponse> {
        let request_id = RequestId(uuid::Uuid::new_v4().into_bytes());
        
        {
            let mut state = self.state.write().await;
            state.start_request();
        }

        info!("Handling inference request {} (tier: {:?}, max_tokens: {})",
            hex::encode(request_id.0), request.tier, request.max_tokens);

        let start_time = std::time::Instant::now();

        let result = async {
            self.verify_licenses(&[]).await?;
            
            let output = self.execute_inference(
                request.tier,
                &request.prompt,
                request.max_tokens,
            ).await?;

            self.process_payment(&[], request.max_tokens).await?;

            Ok::<_, AuriaError>(output)
        }.await;

        let elapsed = start_time.elapsed();

        let response = match result {
            Ok(output) => {
                let mut state = self.state.write().await;
                state.increment_requests();
                state.increment_tokens(output.usage.tokens_generated as u64);
                
                self.metrics.record_inference(elapsed, true).await;

                InferenceResponse {
                    request_id,
                    tokens: output.tokens,
                    usage: UsageInfo {
                        prompt_tokens: request.prompt.split_whitespace().count() as u32,
                        completion_tokens: output.usage.tokens_generated,
                        total_tokens: request.prompt.split_whitespace().count() as u32 + output.usage.tokens_generated,
                    },
                }
            }
            Err(e) => {
                error!("Inference failed: {}", e);
                let mut state = self.state.write().await;
                state.increment_failures();
                
                self.metrics.record_inference(elapsed, false).await;

                return Err(e);
            }
        };

        {
            let mut state = self.state.write().await;
            state.end_request();
        }

        info!("Request {} completed in {:?}", hex::encode(request_id.0), elapsed);

        Ok(response)
    }

    fn supported_tiers(&self) -> &[Tier] {
        &[Tier::Nano, Tier::Standard, Tier::Pro, Tier::Max]
    }
}
