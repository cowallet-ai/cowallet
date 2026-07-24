# 补齐 Polygon 链支持并更正文档

- **Issue**: [COW-17](https://linear.app/clawmint/issue/COW-17)
- **日期**: 2026-07-24
- **状态**: 设计已批准

## 背景与根因

`PROJECT_STATUS.md` 号称支持 6 条 EVM 主网链(含 Polygon),但底层配置 crate `crates/chain-evm/src/chains.rs` 只实现了 5 条(ETH / Base / Arbitrum / Optimism / BSC),`by_chain_id(137)` 返回 `None`,且测试主动断言 Polygon 未实现。

关键点:Polygon 支持在**其他层已经存在**,唯独 `chain-evm` 底层配置漏了:

- 移动端 `mobile/lib/config/api_config.dart`、`chat_view.dart` 等:Polygon 137 / POL 已完整配置。
- 后端 `backend/api-server/src/routes/chains.rs`:`/api/v1/chains` 已返回 Polygon 137。
- 后端 `backend/api-server/src/routes/balance.rs`:`get_all_balances` 已查询 `vec![1, 8453, 42161, 10, 56, 137]`。

因此正确方向是**补齐 `chain-evm` 让 6 链成真**,而非把文档降级为 5 链。文档只是暂时与实现不符,补齐后"6 链"即准确。

## 目标

1. `by_chain_id(137)` 返回有效 Polygon 配置;`all_mainnet()` 含 Polygon。
2. `chain-evm` 测试从"断言未实现"改为"断言已实现并通过";`cargo test -p chain-evm` 绿灯。
3. `PROJECT_STATUS.md` 链数量与实现一致,且不虚报 Polygon 的验证状态。

## 改动详情

### 1. `crates/chain-evm/src/chains.rs`

新增 `polygon_mainnet()` 构造函数,与现有链同构:

| 字段 | 值 |
|------|-----|
| `chain_id` | `137` |
| `name` | `"polygon"` |
| `display_name` | `"Polygon"` |
| `native_currency` | name `"POL"` / symbol `"POL"` / decimals `18` |
| `gas_model` | `GasModel::Eip1559`(Polygon 自 London 分叉起支持 EIP-1559) |
| `block_explorer` | `"https://polygonscan.com"` |
| `rpc_urls` | 默认 `https://polygon-rpc.com`,可被 `POLYGON_MAINNET_RPC_URL` 覆盖 |
| `bundler_url` | `env::var("BUNDLER_URL_POLYGON").ok()` |
| `erc4337_entrypoint` | EntryPoint v0.6 `0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789`(与其他链一致) |
| `is_testnet` | `false` |
| `is_l2` | `false`(与移动端 `api_config.dart` 一致) |

同时:

- `all_mainnet()`:末尾加入 `Self::polygon_mainnet()`。
- `by_chain_id`:新增 `137 => Some(Self::polygon_mainnet())`。

`gas.rs` 无需改动 —— 其 `estimate_gas_for_chain` 通过 `by_chain_id` 取 `gas_model`,补齐后自动生效(`GasModel::Eip1559` 分支已实现)。

### 2. 测试改动(TDD:先反转红灯断言)

- `test_polygon_via_chain_id`(334 行):从断言 `is_none()` 改为断言 `is_some()`,并校验 chain_id=137、symbol="POL"、gas_model=Eip1559、is_l2=false。
- `test_by_chain_id_unsupported`(353 行):从 `vec![137, 43114, 250, 100]` 移除 `137`。
- `test_by_chain_id_all_supported`(342 行):把 `137` 加入 supported 列表。
- `test_all_mainnet_chains`(368 行):`len()` 断言从 `5` 改为 `6`。
- 新增 `test_polygon_mainnet_config`(与其他 `test_*_config` 同风格)。
- `test_entrypoint_addresses_consistent`(406 行):把 `ChainConfig::polygon_mainnet()` 加入被校验列表。

### 3. `PROJECT_STATUS.md`

- 第 16 行:保留"六条 EVM 链(ETH/Base/Arbitrum/Optimism/BSC/Polygon)"—— 补齐后已准确,不改。
- 第 72 行(方案 a,最小改动):保留 6 链列表,在该行文字内追加说明,标注 Polygon 为"配置已补齐,未真机验证",避免对新补的 Polygon 配置虚报 `🧪 已验证`。

## 不做的事(YAGNI)

- 不动移动端(已完整支持 Polygon)。
- 不动 `routes/chains.rs` / `routes/balance.rs`(已含 137)。
- 不加 Polygon 测试网(Amoy 80002):移动端虽有零星引用,但 issue 未要求,不扩范围。

## 验证

- `cargo test -p chain-evm` 全绿。
- `cargo clippy -p chain-evm -- -D warnings` 无告警。

## 验收标准映射(COW-17)

| 验收项 | 覆盖 |
|--------|------|
| `chains.rs` 新增 `polygon_mainnet()`,纳入 `all_mainnet()` 与 `by_chain_id` | §1 |
| `test_polygon_via_chain_id` 改为断言已实现并通过;`cargo test -p chain-evm` 绿灯 | §2、验证 |
| 移动端选 Polygon → 余额/发送流程可用(`_nativeSymbol(137)` 已有 POL) | 已存在,§背景已核实 |
| PROJECT_STATUS.md 第 16、72 行"6 链"表述更正为准确数值 | §3 |
