# cowallet 关键服务优先初始化策略

## 服务分类

### 🔴 Critical（核心功能必需）
影响用户首次操作的必须服务，必须在首屏显示前完成。

| 服务 | 优先级 | 超时 | 用途 |
|------|--------|------|------|
| `storage` | P0 | 50ms | 读取钱包状态 |
| `biometrics` | P0 | 50ms | 签名授权 |
| `settings` | P0 | 50ms | 用户偏好 |
| `wallet/mpcWallet` | P0 | 100ms | 钱包操作 |
| `chain` | P0 | 100ms | 区块链交互 |
| `balance` | P0 | 100ms | 余额显示 |
| `gas` | P0 | 50ms | Gas 估算 |
| `tx` | P0 | 50ms | 交易构建 |
| `policy` | P0 | 50ms | 风控策略 |

**初始化时间**: ~500ms

---

### 🟡 Background（首屏后加载）
重要但不阻塞首次交互的服务。

| 服务 | 优先级 | 超时 | 用途 |
|------|--------|------|------|
| `RustLib` | P1 | 5s | FFI 桥接 |
| `backup` | P1 | 100ms | 云备份 |
| `mpcSessionManager` | P1 | 50ms | MPC 会话 |
| `pendingSign` | P1 | 50ms | 待签名队列 |

**初始化时间**: ~5s

---

### 🟢 Deferred（后台延迟加载）
不影响核心功能的可选服务。

| 服务 | 优先级 | 超时 | 用途 |
|------|--------|------|------|
| `txHistory` | P2 | - | 交易历史 |
| `contacts` | P2 | - | 联系人 |
| `intent` | P2 | - | AI 执行器 |
| `presignPool` | P2 | - | 预签名池 |
| `notifications` | P2 | - | 本地通知 |
| `push` | P2 | - | 推送通知 |

**初始化时间**: ~1s

---

## 初始化时序

```
T0: main() 调用 runApp()
    └─> Splash 显示
    └─> initEssential() [并行]
        ├─> storage ✓
        ├─> biometrics ✓
        ├─> settings ✓
        ├─> wallet ✓
        ├─> chain ✓
        ├─> balance ✓
        └─> 完成 (~500ms)

T1: Splash 中调用 initAll()
    ├─> initBackground() [并行]
    │   ├─> RustLib (最多 5s)
    │   └─> 其他后台服务
    │
    └─> initDeferred() [并行]
        └─> 延迟加载服务

T2: initAll() 完成后导航到 Home/Onboarding
```

---

## 关键决策

### 为什么 RustLib 放在 Background？
- FFI 初始化可能需要 5 秒超时
- 不影响首次签名（签名会等待 FFI 准备好）
- Splash 期间用户可以完成其他操作（如设置）

### 为什么 Push 放在 Deferred？
- 推送通知不影响核心功能
- 用户登录后才需要注册 token
- 网络不稳定时不会阻塞启动

### 为什么 txHistory/Contacts 放在 Deferred？
- 使用缓存优先策略
- 首次加载时可以显示 "loading" 状态
- 不影响余额显示和交易发送

---

## 用户体验

| 操作 | 可用时间 | 说明 |
|------|----------|------|
| 查看余额 | T0 + 500ms | balance 已初始化 |
| 发送交易 | T0 + 500ms | tx, wallet, chain 已初始化 |
| 生物识别授权 | T0 + 50ms | biometrics 已初始化 |
| 查看交易历史 | T0 + 1.5s | 需等待 txHistory 加载 |
| 推送通知 | T0 + 1s | 需等待 push 初始化 |

---

## 后续优化

### 1. 懒加载 txHistory
```dart
// 首次访问时才加载
Future<void> ensureHistoryLoaded() async {
  if (!_historyLoaded) {
    await txHistory.load();
    _historyLoaded = true;
  }
}
```

### 2. 延迟加载联系人
```dart
// 在需要时才加载
Future<void> ensureContactsLoaded() async {
  if (!_contactsLoaded) {
    await contacts.load();
    _contactsLoaded = true;
  }
}
```

### 3. RustLib 进度回调
```dart
// 显示 FFI 初始化进度
RustLib.setProgressCallback((progress) {
  // 更新 Splash 进度条
});
```

---

## 验证

```bash
# 检查编译
flutter analyze lib/services/locator.dart

# 运行应用
flutter run

# 观察日志时间差
# [Services] Essential init complete  ← ~500ms 后
# [Services] Background init complete  ← ~5s 后
# [Services] Deferred init complete   ← ~6s 后
```

---

## 总结

| 指标 | 值 |
|------|-----|
| Critical 服务数 | 8 |
| Essential 初始化时间 | ~500ms |
| Background 初始化时间 | ~5s |
| Deferred 初始化时间 | ~1s |
| 首屏显示延迟 | < 500ms |
| 核心功能可用时间 | < 500ms |