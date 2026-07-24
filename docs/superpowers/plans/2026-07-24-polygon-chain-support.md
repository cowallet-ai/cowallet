# Polygon 链支持 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `chain-evm` crate 补齐 Polygon (chain_id 137) 主网配置,使 6 条 EVM 主网链名副其实,并更正 PROJECT_STATUS.md。

**Architecture:** 根因单点在 `crates/chain-evm/src/chains.rs`——移动端与后端路由层早已支持 Polygon 137,唯独底层配置 crate 漏了。新增 `polygon_mainnet()` 构造函数,纳入 `all_mainnet()` 与 `by_chain_id`,反转现有"断言未实现"的测试,并对文档做最小化措辞更正。`gas.rs` 无需改动(经 `by_chain_id` 取 gas_model,自动生效)。

**Tech Stack:** Rust, alloy-primitives, serde, cargo test/clippy。

## Global Constraints

- Polygon 配置须与移动端 `mobile/lib/config/api_config.dart` 保持一致:`chain_id 137`、`name "polygon"`、`symbol "POL"`、`is_l2 false`。
- `gas_model` 用 `GasModel::Eip1559`(Polygon 自 London 分叉起支持 EIP-1559)。
- EntryPoint v0.6 地址统一为 `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789`,与所有现有链一致。
- RPC 默认 `https://polygon-rpc.com`,可被 env `POLYGON_MAINNET_RPC_URL` 覆盖;bundler 用 `BUNDLER_URL_POLYGON`。
- 不动移动端、不动 `routes/chains.rs` / `routes/balance.rs`(均已含 137)。不加测试网 Amoy 80002。
- 无需向后兼容(pre-production 项目)。

---

### Task 1: 反转测试为"Polygon 已实现"(红灯)

先改测试表达"Polygon 应被支持"的预期,运行确认失败,再实现。此任务与 Task 2 同属一个 TDD 循环、一次提交,故合并为单次 commit 的两半。本任务只改测试并确认红灯。

**Files:**
- Modify: `crates/chain-evm/src/chains.rs`(测试模块,约 333-422 行)

**Interfaces:**
- Consumes: 现有 `ChainConfig::by_chain_id(u64) -> Option<Self>`、`ChainConfig::all_mainnet() -> Vec<Self>`、`GasModel::Eip1559`。
- Produces: 对尚不存在的 `ChainConfig::polygon_mainnet() -> Self` 的调用(Task 2 实现)。

- [ ] **Step 1: 替换 `test_polygon_via_chain_id`(约 333-338 行)**

将原本断言 `is_none()` 的测试整体替换为断言已实现:

```rust
    #[test]
    fn test_polygon_via_chain_id() {
        let chain = ChainConfig::by_chain_id(137).expect("Polygon should be supported");
        assert_eq!(chain.chain_id, 137);
        assert_eq!(chain.name, "polygon");
        assert_eq!(chain.native_currency.symbol, "POL");
        assert_eq!(chain.gas_model, GasModel::Eip1559);
        assert!(!chain.is_testnet);
        assert!(!chain.is_l2);
    }
```

- [ ] **Step 2: 新增 `test_polygon_mainnet_config`(紧接上一测试之后)**

```rust
    #[test]
    fn test_polygon_mainnet_config() {
        let chain = ChainConfig::polygon_mainnet();
        assert_eq!(chain.chain_id, 137);
        assert_eq!(chain.name, "polygon");
        assert_eq!(chain.display_name, "Polygon");
        assert_eq!(chain.native_currency.symbol, "POL");
        assert_eq!(chain.native_currency.decimals, 18);
        assert_eq!(chain.gas_model, GasModel::Eip1559);
        assert!(!chain.is_testnet);
        assert!(!chain.is_l2);
        assert!(chain.erc4337_entrypoint.is_some());
    }
```

- [ ] **Step 3: 从 `test_by_chain_id_unsupported` 移除 137(约 353 行)**

```rust
        let unsupported_chains = vec![43114, 250, 100];
```

- [ ] **Step 4: 向 `test_by_chain_id_all_supported` 加入 137(约 342 行)**

```rust
        let supported_chains = vec![1, 8453, 42161, 10, 56, 137, 84532, 11155111];
```

- [ ] **Step 5: 更新 `test_all_mainnet_chains` 数量断言(约 368 行)**

```rust
        let mainnets = ChainConfig::all_mainnet();
        assert_eq!(mainnets.len(), 6);
```

- [ ] **Step 6: 向 `test_entrypoint_addresses_consistent` 的列表加入 polygon(约 406-412 行)**

将该 `vec!` 替换为:

```rust
        let chains = vec![
            ChainConfig::ethereum_mainnet(),
            ChainConfig::base_mainnet(),
            ChainConfig::arbitrum_one(),
            ChainConfig::optimism_mainnet(),
            ChainConfig::bnb_chain(),
            ChainConfig::polygon_mainnet(),
        ];
```

- [ ] **Step 7: 运行测试确认失败(红灯)**

Run: `cargo test -p chain-evm`
Expected: 编译失败 —— `no function or associated item named 'polygon_mainnet' found`(Task 2 将补上)。

---

### Task 2: 实现 `polygon_mainnet()` 并接入(绿灯 + 提交)

**Files:**
- Modify: `crates/chain-evm/src/chains.rs`(impl 块,约 153-268 行)

**Interfaces:**
- Consumes: `std::env`、`NativeCurrency`、`GasModel::Eip1559`、`ChainConfig` 结构体。
- Produces: `ChainConfig::polygon_mainnet() -> Self`;`all_mainnet()` 返回值含 Polygon;`by_chain_id(137)` 返回 `Some`。

- [ ] **Step 1: 新增 `polygon_mainnet()`(紧接 `bnb_chain()` 之后,约 180 行后)**

```rust
    pub fn polygon_mainnet() -> Self {
        let default_rpc = "https://polygon-rpc.com".to_string();
        let rpc_url = env::var("POLYGON_MAINNET_RPC_URL").unwrap_or(default_rpc);
        let bundler_url = env::var("BUNDLER_URL_POLYGON").ok();

        Self {
            chain_id: 137,
            name: "polygon".into(),
            display_name: "Polygon".into(),
            rpc_urls: vec![rpc_url],
            block_explorer: "https://polygonscan.com".into(),
            native_currency: NativeCurrency {
                name: "POL".into(),
                symbol: "POL".into(),
                decimals: 18,
            },
            gas_model: GasModel::Eip1559,
            erc4337_entrypoint: Some(
                "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                    .parse()
                    .expect("valid EntryPoint v0.6 address"),
            ),
            bundler_url,
            paymaster_url: None,
            is_testnet: false,
            is_l2: false,
        }
    }
```

- [ ] **Step 2: 将 Polygon 加入 `all_mainnet()`(约 241-249 行)**

将该 `vec!` 替换为:

```rust
    pub fn all_mainnet() -> Vec<Self> {
        vec![
            Self::ethereum_mainnet(),
            Self::base_mainnet(),
            Self::arbitrum_one(),
            Self::optimism_mainnet(),
            Self::bnb_chain(),
            Self::polygon_mainnet(),
        ]
    }
```

- [ ] **Step 3: 将 137 加入 `by_chain_id`(约 257-268 行)**

在 `56 => Some(Self::bnb_chain()),` 之后加入一行:

```rust
            137 => Some(Self::polygon_mainnet()),
```

- [ ] **Step 4: 运行测试确认全部通过(绿灯)**

Run: `cargo test -p chain-evm`
Expected: PASS —— 含 `test_polygon_via_chain_id`、`test_polygon_mainnet_config`、`test_all_mainnet_chains`、`test_entrypoint_addresses_consistent` 等全绿。

- [ ] **Step 5: 运行 clippy 确认无告警**

Run: `cargo clippy -p chain-evm -- -D warnings`
Expected: 无告警,退出码 0。

- [ ] **Step 6: 提交**

```bash
git add crates/chain-evm/src/chains.rs
git commit -m "feat(chain-evm): add Polygon mainnet config (COW-17)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: 更正 PROJECT_STATUS.md

**Files:**
- Modify: `PROJECT_STATUS.md`(第 72 行)

**Interfaces:**
- Consumes: 无(纯文档)。
- Produces: 无。

- [ ] **Step 1: 确认第 16 行无需改动**

第 16 行"六条 EVM 链(ETH/Base/Arbitrum/Optimism/BSC/Polygon)"—— 补齐后已准确,保持不动。

- [ ] **Step 2: 更正第 72 行,标注 Polygon 未真机验证(方案 a,行内追加)**

将:

```markdown
| 多链 EVM 支持 | ✅ 已实现 | 🧪 已验证 | ETH, Base, Arbitrum, Optimism, BSC, Polygon |
```

替换为:

```markdown
| 多链 EVM 支持 | ✅ 已实现 | 🧪 已验证 | ETH, Base, Arbitrum, Optimism, BSC 已验证;Polygon 配置已补齐,未真机验证 |
```

- [ ] **Step 3: 提交**

```bash
git add PROJECT_STATUS.md
git commit -m "docs: correct chain count and Polygon verification status (COW-17)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**1. Spec coverage:**
- §1 `polygon_mainnet()` + `all_mainnet()` + `by_chain_id` → Task 2 ✓
- §2 测试反转 + 新增 → Task 1 ✓
- §3 文档更正(方案 a)→ Task 3 ✓
- 验证(`cargo test` / `clippy`)→ Task 2 Step 4-5 ✓
- 移动端已存在(无需任务)→ spec §背景已核实 ✓

**2. Placeholder scan:** 无 TBD/TODO;每个代码步骤均含完整代码。✓

**3. Type consistency:** `polygon_mainnet()` 命名在 Task 1(调用)与 Task 2(定义)一致;字段名 `chain_id`/`native_currency`/`gas_model`/`is_l2` 与现有 `ChainConfig` 结构体一致。✓
