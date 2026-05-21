use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// Per-chain RPC provider with automatic fallback on failure.
#[derive(Clone)]
pub struct RpcProvider {
    chains: Arc<HashMap<u64, ChainRpcs>>,
    default_urls: Arc<Vec<String>>,
    http: reqwest::Client,
}

struct ChainRpcs {
    urls: Vec<String>,
    current: AtomicUsize,
    failures: RwLock<Vec<u32>>,
}

impl Clone for ChainRpcs {
    fn clone(&self) -> Self {
        Self {
            urls: self.urls.clone(),
            current: AtomicUsize::new(self.current.load(Ordering::Relaxed)),
            failures: RwLock::new(Vec::new()),
        }
    }
}

impl RpcProvider {
    pub fn new(http: reqwest::Client, chain_urls: HashMap<u64, Vec<String>>, default_urls: Vec<String>) -> Self {
        let chains: HashMap<u64, ChainRpcs> = chain_urls
            .into_iter()
            .map(|(id, urls)| {
                let len = urls.len();
                (id, ChainRpcs {
                    urls,
                    current: AtomicUsize::new(0),
                    failures: RwLock::new(vec![0; len]),
                })
            })
            .collect();

        Self {
            chains: Arc::new(chains),
            default_urls: Arc::new(default_urls),
            http,
        }
    }

    /// Get the preferred (lowest-failure) RPC URL for a chain.
    pub fn rpc_for_chain(&self, chain_id: u64) -> &str {
        if let Some(chain) = self.chains.get(&chain_id) {
            let idx = chain.current.load(Ordering::Relaxed) % chain.urls.len();
            &chain.urls[idx]
        } else if !self.default_urls.is_empty() {
            &self.default_urls[0]
        } else {
            "https://1rpc.io/eth"
        }
    }

    /// Get all configured RPC URLs for a chain (for iteration / manual fallback).
    pub fn urls_for_chain(&self, chain_id: u64) -> &[String] {
        if let Some(chain) = self.chains.get(&chain_id) {
            &chain.urls
        } else {
            &self.default_urls
        }
    }

    /// Send a JSON-RPC call with automatic fallback across configured RPCs.
    pub async fn rpc_call(
        &self,
        chain_id: u64,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let urls = self.urls_for_chain(chain_id);
        if urls.is_empty() {
            return Err("no RPC URLs configured for chain".into());
        }

        let chain = self.chains.get(&chain_id);
        let start_idx = chain
            .map(|c| c.current.load(Ordering::Relaxed) % c.urls.len())
            .unwrap_or(0);

        let mut last_error = String::new();

        for i in 0..urls.len() {
            let idx = (start_idx + i) % urls.len();
            let url = &urls[idx];

            match self.http.post(url).json(body).send().await {
                Ok(resp) => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // Check for RPC-level error (rate limit, server error)
                            if let Some(err) = json.get("error") {
                                let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                                // -32005 = rate limited, -32603 = internal error — try next
                                if code == -32005 || code == -32603 {
                                    self.mark_failure(chain_id, idx).await;
                                    last_error = format!(
                                        "RPC {} error: {}",
                                        url,
                                        err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
                                    );
                                    continue;
                                }
                            }
                            // Success or application-level error (e.g. insufficient funds) — return as-is
                            self.mark_success(chain_id, idx).await;
                            return Ok(json);
                        }
                        Err(e) => {
                            self.mark_failure(chain_id, idx).await;
                            last_error = format!("RPC {} response parse error: {}", url, e);
                        }
                    }
                }
                Err(e) => {
                    self.mark_failure(chain_id, idx).await;
                    last_error = format!("RPC {} request failed: {}", url, e);
                    tracing::warn!("[rpc_provider] {} — trying next", last_error);
                }
            }
        }

        Err(format!("all RPCs failed for chain {}: {}", chain_id, last_error))
    }

    async fn mark_failure(&self, chain_id: u64, idx: usize) {
        if let Some(chain) = self.chains.get(&chain_id) {
            let mut failures = chain.failures.write().await;
            if idx < failures.len() {
                failures[idx] = failures[idx].saturating_add(1);
            }
            // Rotate to the next URL with fewest failures
            if let Some((best_idx, _)) = failures.iter().enumerate().min_by_key(|(_, &f)| f) {
                chain.current.store(best_idx, Ordering::Relaxed);
            }
        }
    }

    async fn mark_success(&self, chain_id: u64, idx: usize) {
        if let Some(chain) = self.chains.get(&chain_id) {
            let mut failures = chain.failures.write().await;
            if idx < failures.len() {
                // Decay failure count on success
                failures[idx] = failures[idx].saturating_sub(1);
            }
            chain.current.store(idx, Ordering::Relaxed);
        }
    }
}
