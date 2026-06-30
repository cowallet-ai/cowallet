# CoWallet 功能清单

> 更新日期: 2026-05-28 | 项目阶段: Alpha
>
> 状态说明: ✅ 已实现 | 🔧 部分实现 | ❌ 未实现 | 🧪 已验证（真机/生产环境实测通过）

## 项目进度说明（大白话版）

**项目是什么：** 一个用 AI 对话操作的 MPC 加密钱包，支持多条 EVM 链，用阈值签名保护资产安全。

**目前到哪了：** Alpha 阶段，完成度大约 80-85%。核心功能都写完了，关键链路真机验证过，但安全和测试方面还有硬伤。

### 能用的（真机验证过）

- 密钥生成、签名、转账这条最核心的路已经跑通，签名速度 <100ms
- 六条 EVM 链（ETH/Base/Arbitrum/Optimism/BSC/Polygon）都能转账、查余额、看历史
- AI 聊天助手能用自然语言查余额、发起转账、查代币信息，双引擎（Bedrock Claude + DeepSeek）
- 移动端所有主要页面都能用，生物识别、语音输入、中英双语都没问题
- 分片加密存储、备份导出、WebSocket 实时通信都验证过

### 写完了但没验证的（有风险）

- 预签名池自动补充 — 后台任务，边界情况没测过
- 密钥轮换备份 — 出错可能丢钱，但没在真实环境跑过
- ERC-4337 账户抽象 — 链路复杂，等于没用
- DEX 兑换（Bridgers API）— 刚从 0x 切过来，还在修 bug
- EIP-712 结构化签名 — 目前没有 DApp 场景用到

### 还没做的（上线拦路虎）

- 集成测试覆盖率不够 — 改一行代码可能悄悄搞坏签名流程
- 没做第三方安全审计 — 自己写的密码学没人验证过
- 端到端恢复没跑通 — 用户真丢了手机，钱可能找不回来
- 推送签名审批流程没做 — 多方签名时只能同时在线

### 当前适合什么

内部测试和演示。距离公开发布还需要补齐安全审计、集成测试、恢复流程验证，预计 2-3 个月。

---

## 一、核心功能

### 1.1 MPC 钱包创建与管理
| 功能 | 状态 | 说明 |
|------|------|------|
| DKLS23 分布式密钥生成 (DKG) | 🧪 已验证 | 2-of-3 阈值签名，三方生成密钥分片 |
| 预签名 (Presign) | ✅ 已实现 | 后台自动生成预签名材料，加速在线签名 |
| 在线签名 (Sign) | 🧪 已验证 | 设备+服务器双方签名，<100ms 在线阶段 |
| 分片刷新 (Reshare) | ✅ 已实现 | 2-of-2 (设备+服务器) 参与轮换，不改变公钥 |
| 分片加密存储 | ✅ 已实现 | 🧪 已验证 | AES-GCM 加密，设备分片存 Secure Enclave/Keystore |
| 服务器分片管理 | ✅ 已实现 | 🧪 已验证 | HSM 级别加密存储，ENCRYPTION_KEY 保护 |
| 备份分片导出 | ✅ 已实现 | 🧪 已验证 | 加密导出用于离线备份 |
| Noise_XX 传输加密 | ✅ 已实现 | 🧪 已验证 | MPC 协议消息全程加密传输 |
| WebSocket MPC 会话 | ✅ 已实现 | 🧪 已验证 | 实时双向通信，支持 NATS 或 DB 轮询回退 |
| 预签名池自动补充 | ✅ 已实现 | 后台任务监控池水位，低于阈值自动生成 |
| 密钥轮换备份 | ✅ 已实现 | Reshare 前自动备份旧分片，轮换后验证新分片有效性 |

### 1.2 区块链功能
| 功能 | 状态 | 说明 |
|------|------|------|
| 多链 EVM 支持 | ✅ 已实现 | 🧪 已验证 | ETH, Base, Arbitrum, Optimism, BSC, Polygon |
| 原生代币转账 | 🧪 已验证 | ETH/BNB/POL/MATIC 等原生币转账（MPC签名成功，BSC提交已修复） |
| ERC-20 代币转账 | ✅ 已实现 | 🧪 已验证 | 任意 ERC-20 代币转账，自动解析合约 |
| Gas 估算 | ✅ 已实现 | 🧪 已验证 | eth_estimateGas + eth_gasPrice 并行查询 |
| 交易模拟 | ✅ 已实现 | eth_call 预执行检查 |
| 交易状态追踪 | ✅ 已实现 | 🧪 已验证 | 后台轮询 receipt，广播→确认→失败状态机 |
| 多链余额查询 | 🧪 已验证 | OKX Wallet API 聚合查询所有链余额 |
| 交易历史记录 | 🧪 已验证 | 多链交易记录，ERC-20 Transfer 事件解析 |
| ERC-4337 Account Abstraction | ✅ 已实现 | UserOp 构建、签名、提交到 Bundler |
| 多 RPC 回退 | ✅ 已实现 | 🧪 已验证 | 每条链 2-3 个公共 RPC，自动故障切换 |
| EIP-712 签名 | ✅ 已实现 | 结构化数据签名支持 |
| 代币价格查询 | ✅ 已实现 | 🧪 已验证 | 实时价格缓存，USD 估价 |

### 1.3 DEX 兑换 (Swap)
| 功能 | 状态 | 说明 |
|------|------|------|
| 报价查询 | ✅ 已实现 | 0x API 聚合路由获取最优报价 |
| 构建 Swap 交易 | ✅ 已实现 | 自动构建兑换交易数据 |
| AI 发起兑换 | ✅ 已实现 | 自然语言 → swap_token 工具调用 |
| 滑点控制 | ✅ 已实现 | 可配置滑点参数 |

### 1.4 DeFi 收益
| 功能 | 状态 | 说明 |
|------|------|------|
| 收益协议搜索 | ✅ 已实现 | DeFiLlama 数据源，按链/代币筛选 |
| 协议列表 | ✅ 已实现 | 展示各链收益率、TVL |
| DeFi Hub 页面 | ✅ 已实现 | 移动端 DeFi 入口页 |

---

## 二、AI 智能助手

### 2.1 AI 对话系统
| 功能 | 状态 | 说明 |
|------|------|------|
| 自然语言聊天 | ✅ 已实现 | 🧪 已验证 | 双 Provider：Bedrock Claude（默认）+ DeepSeek，客户端可切换，SSE 流式响应 |
| 会话管理 | ✅ 已实现 | 🧪 已验证 | 多会话切换、新建、历史加载 |
| 上下文记忆 | ✅ 已实现 | 🧪 已验证 | 聊天历史持久化到 PostgreSQL |
| 工具调用 (Function Calling) | ✅ 已实现 | 🧪 已验证 | AI 自动选择并执行钱包操作 |
| 二轮工具调用 | ✅ 已实现 | 🧪 已验证 | 工具结果反馈 AI 生成最终回复 |
| Widget 卡片持久化 | ✅ 已实现 | 🧪 已验证 | 工具调用结果以卡片形式保存和恢复 |

### 2.2 AI 工具能力
| 工具 | 状态 | 说明 |
|------|------|------|
| get_balance | ✅ 已实现 | 🧪 已验证 | 查询多链余额，展示余额卡片 |
| send_transaction | ✅ 已实现 | 🧪 已验证 | AI 解析转账意图 → 确认卡片 → 执行签名 |
| get_token_info | ✅ 已实现 | 🧪 已验证 | 查询代币信息(价格/合约/链) |
| get_supported_chains | ✅ 已实现 | 🧪 已验证 | 返回支持的链列表 |
| get_transaction_history | ✅ 已实现 | 🧪 已验证 | 查询交易记录 |
| get_wallet_address | ✅ 已实现 | 🧪 已验证 | 获取钱包地址，展示收款二维码 |
| swap_token | ✅ 已实现 | DEX 兑换报价 + 确认 + 执行 |
| list_yield_protocols | ✅ 已实现 | 查询 DeFi 收益协议 |

### 2.3 AI Chat Widget 卡片
| Widget | 状态 | 说明 |
|--------|------|------|
| 余额卡片 (balance) | ✅ 已实现 | 🧪 已验证 | 多链资产概览 |
| 收款卡片 (receive) | ✅ 已实现 | 🧪 已验证 | 地址 + 二维码 |
| 转账确认卡片 (sendConfirm) | ✅ 已实现 | 🧪 已验证 | 金额/地址/Gas 预览，确认/取消 |
| 交易结果卡片 (txResult) | ✅ 已实现 | 🧪 已验证 | 成功/失败/确认中状态，实时追踪 |
| 交易详情卡片 (txDetail) | ✅ 已实现 | 🧪 已验证 | 链上交易完整数据 |
| 兑换确认卡片 (swapConfirm) | ✅ 已实现 | Swap 报价 + 确认/取消 |
| 交易历史卡片 (history) | ✅ 已实现 | 🧪 已验证 | 交易记录列表 |
| 安全审计卡片 (audit) | ✅ 已实现 | 安全状态报告 |
| 代币信息卡片 (tokenInfo) | ✅ 已实现 | 🧪 已验证 | 代币详细信息 |
| 添加联系人卡片 (addContact) | ✅ 已实现 | 🧪 已验证 | 保存常用地址 |
| 澄清卡片 (clarify) | ✅ 已实现 | 🧪 已验证 | AI 追问选项 |

---

## 三、用户界面 (移动端)

### 3.1 主要页面
| 页面 | 状态 | 说明 |
|------|------|------|
| 首页 (Home) | ✅ 已实现 | 🧪 已验证 | 总资产、活动记录、快捷操作 |
| AI 聊天 (Chat) | ✅ 已实现 | 🧪 已验证 | 对话式操作入口，多 Widget 展示 |
| 钱包 (Wallet) | ✅ 已实现 | 🧪 已验证 | 多链资产分组、代币列表 |
| DeFi/赚钱 (Yield) | ✅ 已实现 | DeFi Hub + 收益协议浏览 |
| 设置 (Settings) | ✅ 已实现 | 🧪 已验证 | 用户偏好、备份管理 |
| 搜索 (Search) | ✅ 已实现 | 🧪 已验证 | 功能搜索、资产搜索、交易记录搜索 |
| 交易历史 (TxHistory) | ✅ 已实现 | 🧪 已验证 | 多链交易记录详情 |
| 密钥管理 (Keys) | ✅ 已实现 | 🧪 已验证 | 分片健康状态 |
| 恢复 (Recovery) | ✅ 已实现 | 🧪 已验证 | 密钥恢复流程 UI |
| 扫码 (Scan) | ✅ 已实现 | 🧪 已验证 | 二维码扫描 |
| 联系人 (Contacts) | ✅ 已实现 | 🧪 已验证 | 常用地址管理 |
| 备份分片 (BackupShard) | ✅ 已实现 | 🧪 已验证 | 备份分片导出 |

### 3.2 交互特性
| 特性 | 状态 | 说明 |
|------|------|------|
| 生物识别签名 | ✅ 已实现 | 🧪 已验证 | 指纹/Face ID 确认转账，不再误清聊天记录 |
| 实时交易状态 | ✅ 已实现 | 🧪 已验证 | 广播→确认动态更新 |
| 错误友好提示 | ✅ 已实现 | 🧪 已验证 | 技术错误映射为用户友好消息 |
| 本地推送通知 | ✅ 已实现 | 交易成功/失败通知 |
| 远程推送通知 (FCM) | ✅ 已实现 | Firebase Cloud Messaging，多通道(交易/安全/MPC签名) |
| 语音输入 | ✅ 已实现 | 🧪 已验证 | 语音转文字输入到 AI 聊天 |
| 搜索跳转 AI Chat | ✅ 已实现 | 🧪 已验证 | 搜索结果直接发起 AI 对话 |
| 代币 Logo 展示 | ✅ 已实现 | 🧪 已验证 | 网络图片 + 文字回退 |
| 中英双语 | ✅ 已实现 | 🧪 已验证 | 完整 l10n 支持 |
| Google Fonts 集成 | ✅ 已实现 | 🧪 已验证 | 自定义字体渲染 |
| 暗色/亮色主题 | 🔧 部分实现 | 主色调定义完成，暗色适配待完善 |

---

## 四、后端服务

### 4.1 API Server (Axum)
| 功能模块 | 状态 | 路由 |
|----------|------|------|
| 用户认证 | ✅ 已实现 | /auth (register, login, refresh, logout, session) |
| 邮箱 OTP | ✅ 已实现 | /auth/email/send-otp |
| 账户恢复 | ✅ 已实现 | /auth/recovery/initiate, verify |
| 审计日志 | ✅ 已实现 | /auth/audit-log |
| MPC 会话管理 | ✅ 已实现 | /mpc/session (CRUD + msg relay) |
| MPC WebSocket | ✅ 已实现 | /ws/mpc/:session_id |
| 预签名管理 | ✅ 已实现 | /mpc/presign/status, generate |
| 交易提交 | ✅ 已实现 | /tx/submit |
| 交易状态查询 | ✅ 已实现 | /tx/status/:hash |
| 交易模拟 | ✅ 已实现 | /tx/simulate |
| Gas 估算 | ✅ 已实现 | /tx/estimate-gas |
| 交易历史 | ✅ 已实现 | /tx/all-history (OKX) |
| 消费摘要 | ✅ 已实现 | /tx/summary |
| ERC-4337 UserOp | ✅ 已实现 | /tx/userop, /tx/userop/submit |
| 余额查询 | ✅ 已实现 | /balance/, /balance/all |
| 钱包管理 | ✅ 已实现 | /wallets (CRUD + chains) |
| 策略引擎 | ✅ 已实现 | /policy (CRUD + evaluate + limits) |
| AI 对话 | ✅ 已实现 | /ai (chat + sessions + history) |
| 价格查询 | ✅ 已实现 | /price |
| 链信息 | ✅ 已实现 | /chains |
| Swap 报价/构建 | ✅ 已实现 | /swap/quote, /swap/build |
| DeFi 收益 | ✅ 已实现 | /yield/search, /yield/protocols |
| 分片管理 | ✅ 已实现 | /shards (upload, get, status) |
| 推送通知 | ✅ 已实现 | /push |

### 4.2 中间件
| 中间件 | 状态 | 说明 |
|--------|------|------|
| JWT 认证 | ✅ 已实现 | Bearer token 验证 |
| 速率限制 | ✅ 已实现 | 不同路由组不同限制 |
| 审计日志 | ✅ 已实现 | 操作审计持久化 |
| 指标收集 | ✅ 已实现 | /metrics 端点 |
| CORS | ✅ 已实现 | 可配置跨域 |
| 安全头 | ✅ 已实现 | Security headers |
| 请求超时 | ✅ 已实现 | 30s 全局超时 |
| 断路器 | ✅ 已实现 | RPC/DeFi 断路器保护 |

### 4.3 后台服务
| 服务 | 状态 | 说明 |
|------|------|------|
| 交易确认追踪器 | ✅ 已实现 | 5s 快轮询 / 30s 慢轮询，自动更新状态 |
| 预签名池管理器 | ✅ 已实现 | 后台补充预签名材料 |
| MPC 会话清理 | ✅ 已实现 | 过期会话自动清理 |
| 价格缓存 | ✅ 已实现 | 代币价格定期刷新 |
| 多 RPC 故障转移 | ✅ 已实现 | 自动切换健康节点 |
| 邮件服务 | ✅ 已实现 | OTP 发送 |

### 4.4 数据库
| 表 | 状态 | 说明 |
|------|------|------|
| users | ✅ | 用户账户 |
| mpc_sessions | ✅ | MPC 协议会话 |
| mpc_messages | ✅ | MPC 轮次消息 |
| shard_metadata | ✅ | 分片加密存储 |
| transactions | ✅ | 交易记录 |
| policies | ✅ | 安全策略规则 |
| chat_sessions | ✅ | AI 会话 |
| chat_messages | ✅ | AI 聊天记录 (含 widget_type/widget_data) |
| wallets | ✅ | 钱包信息 |
| contacts | ✅ | 联系人 |

---

## 五、安全架构

| 特性 | 状态 | 说明 |
|------|------|------|
| 2-of-3 阈值签名 | ✅ 已实现 | 设备+服务器即可签名，备份离线 |
| 设备分片: Secure Enclave | ✅ 已实现 | iOS Keychain / Android Keystore |
| 服务器分片: AES-GCM 加密 | ✅ 已实现 | ENCRYPTION_KEY 保护 |
| Noise_XX 传输加密 | ✅ 已实现 | MPC 协议通信加密 |
| 生物识别授权 | ✅ 已实现 | 签名前必须生物认证 |
| 策略引擎 | ✅ 已实现 | 交易限额、风控规则评估 |
| JWT + 刷新令牌 | ✅ 已实现 | 短期令牌 + 安全刷新 |
| 速率限制 | ✅ 已实现 | 防暴力破解 |
| 审计日志 | ✅ 已实现 | 所有敏感操作可追溯 |
| 分片刷新 (Reshare) | ✅ 已实现 | 2-of-2 参与定期轮换，不改变地址 |
| 邮箱 OTP 恢复 | ✅ 已实现 | 账户恢复验证 |

---

## 六、部署与运维

| 特性 | 状态 | 说明 |
|------|------|------|
| Docker 容器化 | ✅ 已实现 | Dockerfile + docker-compose |
| GitHub Actions CI/CD | ✅ 已实现 | push main → 自动构建部署到 ECS |
| 健康检查端点 | ✅ 已实现 | /health, /ready, /live |
| 指标监控 | ✅ 已实现 | /metrics |
| 优雅停机 | ✅ 已实现 | SIGINT/SIGTERM 信号处理 |
| 数据库自动迁移 | ✅ 已实现 | 启动时自动执行 migrations |
| 本地开发 Makefile | ✅ 已实现 | make dev / make local-start |

---

## 七、近期修复 (2026-05-22)

| 修复项 | 说明 |
|--------|------|
| Android 启动卡死 | RustLib.init() 添加 5s 超时 + FCM getToken() 添加 3s 超时，兼容无 Google Play Services 设备 |
| 转账清空聊天记录 | 生物识别弹窗导致 pause/resume 循环触发清空，改为仅超过 30s 才清除 |
| Reshare 协议参与方 | 修正为 2-of-2 (设备+服务器) 参与，移除无关第三方 |
| AI 错误消息 | 清洗 AI 响应中的技术错误信息，展示用户友好提示 |

---

## 八、待完善项
| 优先级 | 项目 | 说明 |
|--------|------|------|
| 🔴 高 | 测试覆盖率 | MPC 协议和交易路径集成测试不足 |
| 🔴 高 | 安全审计 | 密码学实现需第三方审计 |
| 🔴 高 | 端到端恢复验证 | 恢复流程 UI 完成，全链路未验证 |
| 🟡 中 | 暗色模式 | 主题框架已有，细节适配待完善 |
| 🟡 中 | Push 通知交易审批流 | FCM 基础已完成，待接入 MPC 签名审批交互 |
| 🟡 中 | Indexer/Worker | 骨架已有，链上事件追踪完整性待验证 |
| 🟢 低 | 性能基准测试 | 签名延迟、预签名并发 benchmark |
| 🟢 低 | 代码覆盖率报告 | CI 缺少覆盖率工具 |

---

## 九、技术栈

- **后端**: Rust, Axum, SQLx, PostgreSQL, Redis, NATS, Tower
- **密码学**: DKLS23 (自研), secp256k1, AES-GCM, Noise Protocol
- **区块链**: Alloy (EVM), OKX Wallet API, Bridgers API (DEX 聚合)
- **移动端**: Flutter/Dart, flutter_rust_bridge v2
- **AI**: Bedrock Claude (默认) + DeepSeek (备选), AiProvider trait 抽象, AWS Event Stream 流式, Function Calling
- **部署**: Docker, GitHub Actions, AWS Cloud ECS

---

## 十、安全审计发现 (2026-05-28)

### 高危（可直接导致资金损失）

| # | 问题 | 位置 | 影响 |
|---|------|------|------|
| 1 | WebSocket MPC 会话不检查 Token 黑名单 | `routes/mpc_ws.rs:47` | logout 后旧 token 仍可参与签名 |
| 2 | JWT 明文放 URL 参数 (WebSocket) | `routes/mpc_ws.rs:22` | Token 泄露到日志/CDN，可重放 |
| 3 | ERC-20 无限授权 (MaxUint256) | `intent_executor.dart:392` | aggregator 被攻破则全部代币可被盗 |
| 4 | 用户数据注入 AI 上下文（间接注入） | `ai.rs:793-801` | 联系人名可注入恶意指令操控 AI |
| 5 | Prompt Injection 防御仅关键词匹配 | `ai.rs:438-445` | unicode/变体即可绕过 |

### 中危

| # | 问题 | 位置 |
|---|------|------|
| 6 | Device-ID 校验可绕过（不发 header 即跳过） | `middleware/auth.rs:199-207` |
| 7 | Argon2id 参数偏弱（默认 m=19MiB） | `mpc-core/shard/encrypt.rs:99` |
| 8 | HKDF 未使用 salt | `services/crypto.rs:63` |
| 9 | Shard 传输用裸 SHA-256 派生 AES key | `routes/shards.rs:281` |
| 10 | Covalent API Key 硬编码 | `state.rs:143-144` |

### 低危

| # | 问题 |
|---|------|
| 11 | root_key 存储在进程内存（标注 demo only） |
| 12 | Bedrock 用 Bearer token 而非 SigV4 |
| 13 | 助记词检测仅覆盖 5 种固定短语 |

---

## 十一、可靠性问题 (2026-05-28)

| 问题 | 风险等级 | 说明 |
|------|---------|------|
| Bedrock 流无 idle timeout | 🔴 高 | AI 不响应时连接永远挂起，占满服务器资源 |
| Bedrock buffer 无上限 | 🔴 高 | 故障帧可导致内存无限增长 (OOM) |
| 客户端流无超时 | 🟡 中 | 网络异常时 UI 永远显示"加载中" |
| 浮点精度丢失 | 🟡 中 | `double * 1e18` 对 >0.1 ETH 有精度误差 |
| `buffer.remove(0)` O(n) | 🟢 低 | 垃圾数据多时性能退化 |

---

## 十二、架构评估 (2026-05-28)

### 架构优点

- AI Provider trait 抽象干净，Bedrock + DeepSeek 统一接口 + 自动 fallback
- 中间件栈层次分明：request_id → audit → validation → CORS → trace → 限流 → auth
- MPC 会话管理设计正确：DashMap 并发安全，300s 超时清理，presign 池化
- 结构化错误码 (1000-1899 分段)，客户端可精确处理
- AI Agent Loop 完整：流式 token + tool_call + 二轮对话 + safety net

### 架构问题

- AppState 是 God Object（24 字段），应按领域拆分
- ai.rs 1330 行单文件，system prompt/tool/stream/session 全混一起
- chat_view.dart 700+ 行，职责过多
- ENCRYPTION_KEY 在 main.rs 和 state.rs 各解析一次

---

## 十三、改进路线图 (2026-05-28)

### P0 — 立即修复（上线阻塞）

| 建议 | 工作量 |
|------|--------|
| WS 连接加 Token 黑名单检查 | 1h |
| 删除硬编码 Covalent API Key | 5min |
| ERC-20 授权改为精确金额 | 30min |

### P1 — 短期修复（1-2 周内）

| 建议 | 工作量 |
|------|--------|
| WS 改用一次性 ticket 替代 URL 中的 JWT | 4h |
| AI 上下文中 contacts/portfolio 做结构化清洗 | 2h |
| Bedrock 流加 60s idle timeout + 4MB buffer cap | 2h |
| 浮点金额改用字符串精确解析 | 2h |
| 强制 Device-ID header | 30min |

### P2 — 中期优化（1 月内）

| 建议 | 工作量 |
|------|--------|
| 客户端流加 60s 无数据超时 | 1h |
| 升级 Argon2 参数 (m=64MB, t=3, p=4) | 30min |
| 拆分 ai.rs（prompt/tools/stream 分模块） | 半天 |
| Bedrock buffer 用 bytes::BytesMut 替代 Vec<u8> | 2h |

### P3 — 长期改进

| 建议 | 工作量 |
|------|--------|
| 拆分 AppState 为领域子 state | 1天 |
| 引入向量模型做 prompt injection 检测 | 1周 |
| 接入 KMS 替代内存中 root key | 1周 |
| 第三方安全审计 + 渗透测试 | 外部 |

### 上线时间线

```
现在 ──── 2周后 ──── 1月后 ──── 2月后
 │          │          │          │
 ├ P0 修复  ├ P1 修复  ├ 安全审计  ├ 公开测试
 ├ 集成测试 ├ 恢复验证 ├ 渗透测试  │
 └ 内部测试 └ 性能基准 └ 修复发现  └ 正式发布
```
