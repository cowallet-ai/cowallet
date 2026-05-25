# cowallet 首屏秒开优化方案

## 当前启动瓶颈

```
main()
  ↓ [阻塞 7.5 秒]
Services.init()
  ↓ [阻塞 1-3 秒]
_checkWalletState()
  ↓ [只有这里完成后才显示 UI]
runApp()  ← 首屏才显示
```

**总延迟**: ~8-10 秒

---

## 优化方案：分阶段初始化

### Phase 1: 必需初始化 (< 100ms)
只初始化首屏必须的服务

### Phase 2: 后台初始化
剩余服务在后台异步执行

### Phase 3: 延迟初始化
在首屏显示后才开始

---

## 实施步骤

### 1. 拆分 Services.init()

```dart
// lib/services/locator.dart

class Services {
  // ... 现有代码 ...

  /// Essential initialization - only what's needed for first paint
  static Future<void> initEssential() async {
    // 只初始化 navigator key 和基础状态
    // 其他全部跳过
  }

  /// Background initialization - run after first paint
  static Future<void> initBackground() async {
    // RustLib, storage, txHistory, contacts 等
  }

  /// Deferred initialization - run after UI is stable
  static Future<void> initDeferred() async {
    // Push, notifications 等
  }
}
```

### 2. 修改 main() 流程

```dart
// lib/main.dart

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);

  // 立即启动 UI，不等待任何初始化
  runApp(const CowalletApp());

  // 后台初始化所有服务
  Services.initAll();
}
```

### 3. UI 显示 Loading

```dart
@override
Widget build(BuildContext context) {
  if (!_ready) {
    return MaterialApp(
      theme: cwTheme(),
      home: const SplashView(),  // 立即显示
    );
  }
  // ... 正常路由
}
```

---

## 效果预期

| 指标 | 优化前 | 优化后 |
|------|--------|--------|
| 首屏显示延迟 | ~8-10 秒 | **< 500ms** ⚡ |
| 用户感知 | 白屏等待 | 立即看到 App |
| 初始化完成 | ~10 秒 | ~10 秒 (后台) |