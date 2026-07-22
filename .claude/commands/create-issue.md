# 创建 Linear Issue

根据用户描述,在 CoWallet team 下创建符合规范的 Linear issue。

## 输入

用户需求描述: $ARGUMENTS

## 执行流程

1. **理解需求 + 读代码** — 阅读相关源码确认路径/字段名真实存在,再建 issue
2. **调用** `mcp__linear-server__save_issue` 创建 issue(team: CoWallet,project: CoWallet v1.0.1)
3. **确认产出** — 打印 issue 链接

## Issue 格式

### 标题
- 中文,动词开头,陈述交付物不含活动
- ≤50 字,不含 issue 编号
- ✅「补齐 MPC presign 并发互斥测试」 ❌「研究 presign 问题」

### 正文(必填四节)

```markdown
## 目标
一句话:做完后世界有什么不同。

## 背景
- 现状(引用真实代码路径,如 `backend/api-server/src/services/presign_manager.rs:42`)
- 为什么需要做

## 范围
- 要新增/修改的文件路径(精确到目录层级)
- 涉及的环境变量/配置
- 测试要求(见下方测试分级)

## 完成标准
- 可观测的验收条件,每条能被一条命令验证
- 例:cargo test -p api-server --features integration-tests 中 presign 测试全绿
```

## 代码路径速查

| 层 | 路径 | 职责 |
|---|---|---|
| MPC 协议 | `crates/mpc-core/` | DKG/presign/sign/reshare,Noise_XX |
| 链 | `crates/chain-evm/` | EVM 签名,交易构建 |
| 策略 | `crates/policy-engine/` | 风控规则,审批流 |
| 密钥存储 | `crates/storage-crypto/` | 加密存储,平台 keychain |
| API 路由 | `backend/api-server/src/routes/` | HTTP 路由(auth/mpc/tx/wallets/ai) |
| AI 执行 | `backend/api-server/src/services/ai_executor/` | AI 工具调用,安全检测 |
| 移动端 | `mobile/lib/` | Flutter App (Dart + Rust FFI) |

## MPC 接缝失败语义(触及签名/策略时注明)

| 路径 | 失败语义 |
|---|---|
| 只读查询(余额/历史/价格) | **fail-open** — 服务降级,不影响资产安全 |
| 签名发起 / 策略评估 | **fail-closed** — 拒绝执行,绝不静默放行 |
| presign 后台生成 | fail-open(池空时 sign 会等待或失败,不泄露) |

## 测试分级

| 类型 | 路径 | 命令 |
|---|---|---|
| Rust 单测 | `#[test]` 同文件 | `cargo test -p <crate>` |
| Rust 集成测试 | `#[sqlx::test]` | `cargo test --features integration-tests` + DATABASE_URL |
| Flutter 单测 | `mobile/test/` | `flutter test` |
| Flutter 分析 | — | `flutter analyze` |

## 优先级

| 值 | 含义 |
|---|---|
| 1 Urgent | 阻塞其他工作或有安全风险 |
| 2 High | 里程碑关键路径 |
| 3 Medium | 计划内,非阻塞 |
| 4 Low | nice-to-have |

## 禁止

- 没有「完成标准」
- 引用不存在的文件路径或字段名
- 改 mpc-core/chain-evm/policy-engine 却不写测试要求
- 超大 issue 跨多个独立交付物(必须拆)
