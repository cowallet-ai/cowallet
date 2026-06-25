//! Transfer policy evaluation against user limits.

use super::ToolContext;

impl ToolContext {
    /// Evaluate the transfer against the user's policy limits.
    /// Returns a PolicyResult indicating allow/deny/warn.
    pub(super) async fn evaluate_transfer_policy(
        &self,
        from: &str,
        to: &str,
        token: &str,
        chain_id: u64,
        value: alloy_primitives::U256,
        decimals: u8,
    ) -> policy_engine::PolicyResult {
        // Get token price for USD estimation
        let symbol = if token.is_empty() { "ETH" } else { token };
        let price_usd = self.app_state.price_cache
            .get_usd_price(&self.app_state.http, &symbol.to_uppercase())
            .await
            .unwrap_or(0.0);

        // Calculate value in USD
        let divisor = 10f64.powi(decimals as i32);
        let value_f64 = value.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
        let value_usd = value_f64 * price_usd;

        // Load user limits from DB (fallback to defaults)
        let limits = self.load_user_limits().await;

        // Calculate daily total USD from recent transactions
        let daily_total_usd = self.compute_daily_total_usd(chain_id).await;

        // Check if recipient is new
        let is_new_recipient = self.check_new_recipient(to).await;

        let ctx = policy_engine::TxContext {
            from: from.to_string(),
            to: to.to_string(),
            value_usd,
            token: symbol.to_string(),
            chain_id,
            is_new_recipient,
            daily_total_usd,
        };

        policy_engine::limits::evaluate(&ctx, &limits)
    }

    /// Load per-user policy limits from the database.
    async fn load_user_limits(&self) -> policy_engine::UserLimits {
        let db = match self.app_state.require_db() {
            Ok(db) => db,
            Err(_) => return policy_engine::UserLimits::default(),
        };
        let user_id = match &self.user_id {
            Some(uid) => match uuid::Uuid::parse_str(uid) {
                Ok(id) => id,
                Err(_) => return policy_engine::UserLimits::default(),
            },
            None => return policy_engine::UserLimits::default(),
        };

        let row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT single_limit_usd, daily_limit_usd FROM user_policies WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

        match row {
            Some((single, daily)) => policy_engine::UserLimits {
                single_limit_usd: single,
                daily_limit_usd: daily,
            },
            None => policy_engine::UserLimits::default(),
        }
    }

    /// Compute cumulative USD value of transfers in the last 24 hours.
    async fn compute_daily_total_usd(&self, _chain_id: u64) -> f64 {
        let db = match self.app_state.require_db() {
            Ok(db) => db,
            Err(_) => return 0.0,
        };
        let user_id = match &self.user_id {
            Some(uid) => match uuid::Uuid::parse_str(uid) {
                Ok(id) => id,
                Err(_) => return 0.0,
            },
            None => return 0.0,
        };

        // Sum all transaction values in the last 24h for this user
        // Value is stored as text (wei), so we query and convert
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT value, token FROM transactions
             WHERE user_id = $1 AND created_at > NOW() - INTERVAL '24 hours'
             AND status != 'failed'",
        )
        .bind(user_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        let mut total_usd = 0.0;
        for (value_str, token) in &rows {
            let symbol = token.as_deref().unwrap_or("ETH").to_uppercase();
            let decimals: u8 = match symbol.as_str() {
                "USDC" | "USDT" => 6,
                _ => 18,
            };
            let price = self.app_state.price_cache
                .get_usd_price(&self.app_state.http, &symbol)
                .await
                .unwrap_or(0.0);
            let divisor = 10f64.powi(decimals as i32);
            let value_f64 = value_str.parse::<f64>().unwrap_or(0.0) / divisor;
            total_usd += value_f64 * price;
        }

        total_usd
    }

    /// Check if we have previously sent to this address.
    async fn check_new_recipient(&self, to_address: &str) -> bool {
        let db = match self.app_state.require_db() {
            Ok(db) => db,
            Err(_) => return false,
        };
        let user_id = match &self.user_id {
            Some(uid) => match uuid::Uuid::parse_str(uid) {
                Ok(id) => id,
                Err(_) => return false,
            },
            None => return false,
        };

        // to_addr is stored as BYTEA — decode the hex address for comparison
        let addr_bytes = match hex::decode(to_address.strip_prefix("0x").unwrap_or(to_address)) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let count: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND to_addr = $2 AND status != 'failed'",
        )
        .bind(user_id)
        .bind(&addr_bytes)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

        match count {
            Some((c,)) => c == 0,
            None => false,
        }
    }
}
