# cowallet 项目 AI 辅助开发总结

## 一、项目背景

cowallet 是一个 AI-native MPC 加密钱包，使用 Rust 后端 + Flutter 移动端，由单人 + AI（Claude Code）开发。本文档记录开发过程中 AI 的实际表现、遇到的问题和解决过程。

---

## 二、AI 做得好的地方

### 架构设计一次成型

给 AI 说"把 DeepSeek 单一实现改成双 provider 架构，支持客户端切换"，它直接输出了：
- `AiProvider` trait 定义（含统一消息类型、流式事件枚举）
- 路由层 provider 解析逻辑（根据请求 `model` 字段选择 provider）
- Flutter 设置页面 UI（底部弹窗选择模型）
- Docker Compose 环境变量配置

这些代码结构合理，编译一次通过。

### 跨栈改动保持一致

一次对话里同时改了：
- Rust trait 定义 (`ai_provider.rs`)
- Rust 实现 (`bedrock_provider.rs`, `claude.rs`)
- Rust 路由 (`routes/ai.rs`)
- Rust 状态 (`state.rs`)
- Flutter API 层 (`ai_api.dart`)
- Flutter UI (`settings_view.dart`, `chat_view.dart`)
- Docker 配置 (`docker-compose.yml`, `docker-compose.prod.yml`)

接口定义改了，所有调用方同步更新，没有遗漏。

### 编译错误修复快

Rust 编译器报错后，AI 能立即定位问题并修复：
- `ChatRequest` 类型不存在 → 改为实际的 trait 方法签名
- `StreamEvent::ToolCall` 字段不匹配 → 改为 `ToolCall(ToolCallInfo{...})`
- `BoxStream` 泛型参数缺失 → 补上 `<StreamEvent>`

每个编译错误基本一轮修复。

---

## 三、AI 做得不好的地方

### 错误 1：猜错认证方式

AI 一开始假设 Bedrock 用 AWS SigV4 签名认证，生成了一堆 `aws-sdk-bedrockruntime` 的代码。实际上我们用的是 Bearer token（API Key）方式，跟 Anthropic 直连类似但 endpoint 不同。

**纠正方式**：我把之前写的 Vercel Edge Function 代码贴给 AI 看，它立刻理解了正确的认证方式。

### 错误 2：猜错流式响应格式

AI 假设 Bedrock 返回标准 SSE 格式（`data: {...}\n\n`），写了一个基于文本行分割的解析器。实际 Bedrock 返回的是 AWS Event Stream 二进制帧格式：
- 每个帧有二进制头（event-type、content-type、message-type）
- 帧体是 JSON：`{"bytes":"<base64编码的事件数据>"}`
- 需要 base64 解码才能拿到实际的 Claude 事件 JSON

**发现过程**：部署后发现 200 OK 但没数据输出 → 加 debug 日志打印 raw bytes → 看到二进制帧结构 → 重写解析器。

### 错误 3：猜错 Model ID 格式

AI 用了 `anthropic.claude-sonnet-4-20250514-v1:0` 作为 model ID，Bedrock 返回 400 说不支持 on-demand throughput。实际需要用 inference profile 格式：`us.anthropic.claude-sonnet-4-20250514-v1:0`（带区域前缀）。

**纠正方式**：把 Bedrock 的错误信息贴给 AI，一轮修复。

### 错误 4：流式解析逻辑 bug

解析器写好后，数据能出来但不连贯。原因是 `extract_events_from_buffer` 遇到不产生事件的帧（如 `message_start`）时返回 None，导致 unfold 循环去等新数据，而不是继续处理 buffer 里剩余的帧。

**修复**：把函数改成内部循环，跳过无意义帧，直到找到有效事件或 buffer 为空。

---

## 四、解决问题的实际过程

### Bedrock 集成的完整调试时间线

```
第 1 轮：编译通过，部署后 400 错误
  原因：model ID 格式错
  修复：加 us. 前缀 → 1 分钟

第 2 轮：200 OK 但客户端没收到任何数据
  加日志：tracing::debug 打印 raw bytes
  发现：收到了二进制数据，不是 SSE 文本
  修复：重写解析器，改为提取 {"bytes":"..."} + base64 解码 → 30 分钟

第 3 轮：数据出来了但不连贯
  分析：unfold 逻辑在遇到 message_start 帧时停住等新数据
  修复：extract_events_from_buffer 改为循环跳过无事件帧 → 10 分钟
```

### 每次修复的关键信息来源

| 轮次 | AI 缺少的信息 | 人提供的信息 | 修复方式 |
|------|-------------|-------------|---------|
| 认证方式 | 不知道用 Bearer token | Vercel proxy 源码 | 重写 provider |
| Model ID | 不知道要 inference profile | Bedrock 400 错误信息 | 改环境变量默认值 |
| 响应格式 | 不知道是二进制帧 | raw bytes 日志输出 | 重写解析器 |
| 帧跳过 | 不知道有无事件帧 | "数据不连贯"反馈 | 修复循环逻辑 |

---

## 五、Bridgers 跨链兑换集成的问题

另一个典型的第三方 API 集成问题：

| 问题 | AI 生成的 | 实际需要的 | 怎么发现的 |
|------|----------|-----------|-----------|
| Coin code | `ETH` | `ETH(ETH)` | API 返回 404，对比文档 |
| Chain name | `ethereum` | `ETH` | 对比 token list API 返回值 |
| 响应嵌套 | `{data: [...]}` | `{data: {list: [...]}}` | 打印实际响应 JSON |
| Slippage | `0.01` | `100` (basis points) | 交易失败，看其他项目代码 |

每个都是小问题，但累积起来花了不少时间。共同点：AI 没见过这个 API 的真实响应，只能猜。

---

## 六、什么情况下 AI 一次就对

- WebSocket 消息中继（Axum + NATS pub/sub）— 标准架构模式
- JWT 认证中间件 — 成熟的 Tower middleware 模式
- SQLx 数据库查询 — 类型安全，编译器兜底
- Flutter 设置页面 UI — 标准 Material Design 组件
- Docker Compose 配置 — 声明式，格式固定
- Cargo workspace 依赖管理 — 规则明确

共同特征：这些都是"有标准答案"的东西，AI 训练数据里见过大量类似代码。

---

## 七、大白话总结

AI 写代码分两种情况：

**套路活** — 又快又好。你说"写个接口"，它唰唰唰就出来了，格式规范类型安全。这种活它比人快 5 倍以上。

**对接活** — 需要来回改。因为 AI 看不到真实的 API 响应长什么样，只能猜。猜错了就得你部署一次、看一次日志、再告诉它实际是什么样。Bedrock 集成来回改了 5 轮就是这个原因。

**怎么配合最省事：**
1. 对接第三方 API 之前，先自己 curl 一次，把真实响应贴给 AI
2. 出错了贴日志，别描述现象 — AI 看原始数据比看你的描述准
3. 有参考代码就贴参考代码 — 一段能跑的代码胜过十段需求描述
4. 同一个问题改两次还没好，停下来给 AI 新信息，别让它继续猜
