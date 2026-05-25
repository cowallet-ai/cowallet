# CoWallet 功能清单

> 更新日期: 2026-05-22 | 项目阶段: Alpha
>
> 状态说明: ✅ 已实现 | 🔧 部分实现 | ❌ 未实现 | 🧪 已验证（真机/生产环境实测通过）

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
| 多链余额查询 | 🧪 已验证 | Covalent API 聚合查询所有链余额 |
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
| 自然语言聊天 | ✅ 已实现 | 🧪 已验证 | DeepSeek LLM，SSE 流式响应 |
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
| 交易历史 | ✅ 已实现 | /tx/all-history (Covalent) |
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
- **区块链**: Alloy (EVM), Covalent API, 0x API
- **移动端**: Flutter/Dart, flutter_rust_bridge v2
- **AI**: Claude + DeepSeek (OpenAI-compatible), SSE streaming, Function Calling
- **部署**: Docker, GitHub Actions, AWS Cloud ECS
