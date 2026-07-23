use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::middleware::audit::AuditLogger;
use crate::middleware::metrics::MetricsStore;
use crate::middleware::rate_limit::AnyRateLimiter;
use crate::retry::{CircuitBreaker, CircuitBreakerConfig};
use crate::routes::price::PriceCache;
use crate::routes::yield_::YieldCache;
use crate::services::ai_provider::{AiProvider, ProviderKind};
use crate::services::email::EmailService;
use crate::services::mpc_participant::MpcParticipant;
use crate::services::presign_manager::PresignManager;
use crate::services::rpc_provider::RpcProvider;
use crate::services::tx_tracker::TxTracker;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub rpc_url: String,
    pub rpc_urls: HashMap<u64, String>,
    pub rpc: RpcProvider,
    pub price_cache: PriceCache,
    pub yield_cache: YieldCache,
    pub http: reqwest::Client,
    pub ai_bedrock: Option<Arc<dyn AiProvider>>,
    pub ai_deepseek: Option<Arc<dyn AiProvider>>,
    /// Operator's preferred AI provider, resolved from `AI_PROVIDER` at startup.
    pub ai_provider_pref: ProviderKind,
    pub nats: Option<async_nats::Client>,
    pub rate_limiter: AnyRateLimiter,
    pub rpc_circuit_breaker: CircuitBreaker,
    pub defi_circuit_breaker: CircuitBreaker,
    pub metrics: MetricsStore,
    pub audit_logger: AuditLogger,
    pub mpc_participant: Option<Arc<MpcParticipant>>,
    pub presign_manager: Option<Arc<PresignManager>>,
    pub okx_credentials: Option<crate::services::okx::OkxCredentials>,
    pub bridgers_source_flag: String,
    pub bundler_url: Option<String>,
    pub paymaster_url: Option<String>,
    pub tx_tracker: Option<Arc<TxTracker>>,
    pub email: Option<EmailService>,
}

impl AppState {
    pub async fn new(
        database_url: &str,
        rpc_url: String,
        rpc_urls: HashMap<u64, String>,
        chain_rpcs: HashMap<u64, Vec<String>>,
    ) -> Result<Self, sqlx::Error> {
        // Configure production-grade connection pool
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(
                std::env::var("DB_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(20),
            )
            .min_connections(
                std::env::var("DB_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5),
            )
            .acquire_timeout(std::time::Duration::from_secs(
                std::env::var("DB_ACQUIRE_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10),
            ))
            .idle_timeout(std::time::Duration::from_secs(
                std::env::var("DB_IDLE_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(600),
            ))
            .max_lifetime(std::time::Duration::from_secs(
                std::env::var("DB_MAX_LIFETIME")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1800),
            ));

        let db = pool_options.connect(database_url).await?;
        sqlx::migrate!("../migrations").run(&db).await?;

        // Initialize NATS client if URL is available
        let nats = match std::env::var("NATS_URL") {
            Ok(url) => match async_nats::connect(&url).await {
                Ok(client) => {
                    tracing::info!("Connected to NATS at {}", url);
                    Some(client)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to connect to NATS at {}: {} — WS will fall back to DB polling",
                        url,
                        e
                    );
                    None
                }
            },
            Err(_) => {
                tracing::info!("NATS_URL not set — MPC WebSocket will use DB polling fallback");
                None
            }
        };

        // Initialize AI providers. Bedrock is the default engine; DeepSeek is the
        // fallback (see `select_ai_provider`). Both are optional — a provider that
        // fails to configure simply stays None.
        let ai_bedrock: Option<Arc<dyn AiProvider>> =
            match crate::services::bedrock_provider::BedrockProvider::from_env().await {
                Ok(provider) => Some(Arc::new(provider)),
                Err(e) => {
                    tracing::warn!("Bedrock AI provider not configured: {}", e);
                    None
                }
            };
        let ai_deepseek: Option<Arc<dyn AiProvider>> =
            match crate::services::claude::AiClient::from_env() {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    tracing::warn!("DeepSeek AI provider not configured: {}", e);
                    None
                }
            };
        let ai_provider_pref = ProviderKind::from_env();
        tracing::info!("Preferred AI provider (AI_PROVIDER): {:?}", ai_provider_pref);

        // Decode + validate ENCRYPTION_KEY. This is the root key for every
        // server shard and presignature, so reject weak/low-entropy keys here
        // (this runs before main.rs's own check).
        let encryption_key =
            hex::decode(std::env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY must be set"))
                .expect("ENCRYPTION_KEY must be valid hex");
        crate::services::crypto::validate_encryption_key(&encryption_key)
            .expect("ENCRYPTION_KEY rejected");
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&encryption_key);
        let encryption = crate::services::crypto::EncryptionService::new(&key_array, "server-mpc");

        // Initialize PresignManager with encryption service
        let presign_manager = Arc::new(PresignManager::new(db.clone(), encryption.clone()));
        let min_presignatures: u32 = std::env::var("PRESIGN_MIN_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        presign_manager.spawn_background_task(min_presignatures);

        // Initialize MPC participant with encryption service and presign manager
        let mut participant = MpcParticipant::new(db.clone(), encryption);
        participant.set_presign_manager(Arc::clone(&presign_manager));
        let participant = Arc::new(participant);
        participant.spawn_cleanup();

        // Balance & tx-history queries use the OKX Web3 Wallet API (Onchain OS).
        // All four credential vars are required; when unset the field stays None
        // and balance/tx-history endpoints return 503 (handled at the route layer).
        let okx_credentials = crate::services::okx::OkxCredentials::from_env();
        if okx_credentials.is_some() {
            tracing::info!("OKX Wallet API configured for balance/tx-history queries");
        } else {
            tracing::warn!("OKX_* credentials not fully set — balance and tx-history endpoints will return 503");
        }

        let bridgers_source_flag =
            std::env::var("BRIDGERS_SOURCE_FLAG").unwrap_or_else(|_| "cowallet".to_string());
        tracing::info!("Bridgers source_flag: {}", bridgers_source_flag);

        let bundler_url = std::env::var("BUNDLER_URL").ok().filter(|s| !s.is_empty());
        if let Some(ref url) = bundler_url {
            tracing::info!("Bundler configured at {}", url);
        } else {
            tracing::info!("BUNDLER_URL not set — ERC-4337 account abstraction disabled");
        }

        let paymaster_url = std::env::var("PAYMASTER_URL")
            .ok()
            .filter(|s| !s.is_empty());
        if let Some(ref url) = paymaster_url {
            tracing::info!("Paymaster configured at {}", url);
        }

        // Initialize transaction confirmation tracker
        let http_client = Self::create_http_client();

        // Build RPC provider with multi-URL fallback
        let rpc = RpcProvider::new(http_client.clone(), chain_rpcs, vec![rpc_url.clone()]);

        let tx_tracker = Arc::new(TxTracker::new(
            db.clone(),
            http_client.clone(),
            rpc_urls.clone(),
            rpc_url.clone(),
        ));
        tx_tracker.spawn_background_task();
        tracing::info!("Transaction confirmation tracker started");

        Ok(Self {
            db: Some(db.clone()),
            rpc_url,
            rpc_urls,
            rpc,
            price_cache: PriceCache::new(),
            yield_cache: YieldCache::new(),
            http: http_client,
            ai_bedrock,
            ai_deepseek,
            ai_provider_pref,
            nats,
            rate_limiter: AnyRateLimiter::from_env()
                .unwrap_or_else(|_| AnyRateLimiter::in_memory()),
            rpc_circuit_breaker: CircuitBreaker::new(CircuitBreakerConfig::default()),
            defi_circuit_breaker: CircuitBreaker::new(CircuitBreakerConfig::default()),
            metrics: MetricsStore::new(),
            audit_logger: AuditLogger::new(Some(db)),
            mpc_participant: Some(participant),
            presign_manager: Some(presign_manager),
            okx_credentials,
            bridgers_source_flag,
            bundler_url,
            paymaster_url,
            tx_tracker: Some(tx_tracker),
            email: EmailService::from_env().await,
        })
    }

    /// Get the preferred RPC URL for a specific chain (first healthy one).
    pub fn rpc_for_chain(&self, chain_id: u64) -> &str {
        self.rpc.rpc_for_chain(chain_id)
    }

    /// Send a JSON-RPC call with automatic multi-RPC fallback.
    pub async fn rpc_call(
        &self,
        chain_id: u64,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.rpc.rpc_call(chain_id, body).await
    }

    /// Create a production-grade HTTP client with reasonable defaults
    fn create_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_default()
    }

    /// Check if database connection is available - returns production error
    pub fn require_db(&self) -> crate::errors::Result<&PgPool> {
        self.db
            .as_ref()
            .ok_or_else(|| crate::errors::ApiError::service_unavailable("Database unavailable"))
    }

    /// AI providers to try, in preference order (preferred first, fallback
    /// second), each paired with its [`ProviderKind`]. Honors the operator's
    /// `AI_PROVIDER` preference. Unconfigured providers are omitted, so an empty
    /// list means no provider is available.
    ///
    /// Route handlers iterate this list and fail over to the next provider when
    /// one errors at request time — AI chat is a read-only/assistive path, so
    /// selection is fail-open across both configuration and runtime failures.
    pub fn ai_providers_ordered(&self) -> Vec<(ProviderKind, Arc<dyn AiProvider>)> {
        order_ai_providers(
            self.ai_provider_pref,
            self.ai_bedrock.as_ref(),
            self.ai_deepseek.as_ref(),
        )
    }

    /// Select the single AI provider to serve a request: the highest-preference
    /// one that is configured, or None when neither is. Fail-open at the
    /// configuration level (see [`AppState::ai_providers_ordered`] for the full
    /// ordered list used for runtime failover).
    pub fn select_ai_provider(&self) -> Option<Arc<dyn AiProvider>> {
        self.ai_providers_ordered()
            .into_iter()
            .next()
            .map(|(_, provider)| provider)
    }
}

/// Pure provider-ordering logic, factored out of [`AppState`] so it can be
/// unit-tested without a database. Returns the configured providers in
/// preference order (preferred first, fallback second); omits unconfigured
/// ones, so the result is empty when neither is available.
fn order_ai_providers(
    pref: ProviderKind,
    bedrock: Option<&Arc<dyn AiProvider>>,
    deepseek: Option<&Arc<dyn AiProvider>>,
) -> Vec<(ProviderKind, Arc<dyn AiProvider>)> {
    let order = match pref {
        ProviderKind::Bedrock => [ProviderKind::Bedrock, ProviderKind::DeepSeek],
        ProviderKind::DeepSeek => [ProviderKind::DeepSeek, ProviderKind::Bedrock],
    };
    order
        .into_iter()
        .filter_map(|kind| {
            let provider = match kind {
                ProviderKind::Bedrock => bedrock,
                ProviderKind::DeepSeek => deepseek,
            };
            provider.map(|p| (kind, Arc::clone(p)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai_provider::{
        AiResult, BoxStream, ChatMessage, ChatResponse, StreamEvent, ToolDef,
    };

    /// Minimal stub provider; selection never calls its methods.
    struct StubProvider;

    #[async_trait::async_trait]
    impl AiProvider for StubProvider {
        async fn chat(
            &self,
            _messages: &[ChatMessage],
            _tools: &[ToolDef],
            _temperature: Option<f32>,
        ) -> AiResult<ChatResponse> {
            unreachable!("selection tests never invoke chat")
        }

        async fn stream_chat(
            &self,
            _messages: &[ChatMessage],
            _tools: &[ToolDef],
            _temperature: Option<f32>,
        ) -> AiResult<BoxStream<StreamEvent>> {
            unreachable!("selection tests never invoke stream_chat")
        }
    }

    fn stub() -> Arc<dyn AiProvider> {
        Arc::new(StubProvider)
    }

    fn select(
        pref: ProviderKind,
        bedrock: Option<&Arc<dyn AiProvider>>,
        deepseek: Option<&Arc<dyn AiProvider>>,
    ) -> Option<Arc<dyn AiProvider>> {
        order_ai_providers(pref, bedrock, deepseek)
            .into_iter()
            .next()
            .map(|(_, p)| p)
    }

    // (c) Default preference (Bedrock) picks Bedrock when both are configured.
    #[test]
    fn default_pref_selects_bedrock() {
        let bedrock = stub();
        let deepseek = stub();
        let got = select(ProviderKind::default(), Some(&bedrock), Some(&deepseek))
            .expect("a provider is available");
        assert!(Arc::ptr_eq(&got, &bedrock));
    }

    // (a) AI_PROVIDER=deepseek with both configured picks DeepSeek.
    #[test]
    fn deepseek_pref_selects_deepseek_when_both_configured() {
        let bedrock = stub();
        let deepseek = stub();
        let got = select(ProviderKind::DeepSeek, Some(&bedrock), Some(&deepseek))
            .expect("a provider is available");
        assert!(Arc::ptr_eq(&got, &deepseek));
    }

    // (b) Preferred provider unconfigured → fail-open to the other.
    #[test]
    fn fails_open_to_other_provider() {
        let deepseek = stub();
        // Prefer Bedrock, but only DeepSeek is configured.
        let got = select(ProviderKind::Bedrock, None, Some(&deepseek))
            .expect("falls back to deepseek");
        assert!(Arc::ptr_eq(&got, &deepseek));

        let bedrock = stub();
        // Prefer DeepSeek, but only Bedrock is configured.
        let got = select(ProviderKind::DeepSeek, Some(&bedrock), None)
            .expect("falls back to bedrock");
        assert!(Arc::ptr_eq(&got, &bedrock));
    }

    #[test]
    fn returns_none_when_neither_configured() {
        assert!(select(ProviderKind::Bedrock, None, None).is_none());
        assert!(select(ProviderKind::DeepSeek, None, None).is_none());
    }

    // Runtime failover: when both are configured the ordered list is
    // [preferred, fallback], so a handler can try index 0 then index 1.
    #[test]
    fn ordered_list_puts_preferred_first_then_fallback() {
        let bedrock = stub();
        let deepseek = stub();

        let order = order_ai_providers(ProviderKind::Bedrock, Some(&bedrock), Some(&deepseek));
        assert_eq!(order.len(), 2);
        assert_eq!(order[0].0, ProviderKind::Bedrock);
        assert!(Arc::ptr_eq(&order[0].1, &bedrock));
        assert_eq!(order[1].0, ProviderKind::DeepSeek);
        assert!(Arc::ptr_eq(&order[1].1, &deepseek));

        let order = order_ai_providers(ProviderKind::DeepSeek, Some(&bedrock), Some(&deepseek));
        assert_eq!(order[0].0, ProviderKind::DeepSeek);
        assert_eq!(order[1].0, ProviderKind::Bedrock);
    }

    #[test]
    fn ordered_list_is_empty_when_neither_configured() {
        assert!(order_ai_providers(ProviderKind::Bedrock, None, None).is_empty());
    }
}
