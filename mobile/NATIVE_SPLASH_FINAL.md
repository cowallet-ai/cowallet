# Native Splash 集成 - 最终方案

## ✅ 清理完成

| 操作 | 状态 |
|------|------|
| 删除 `lib/views/splash_view.dart` | ✅ 完成 |
| 移除 `AppRouter.splash` 路由 | ✅ 完成 |
| 修复 `main.dart` 逻辑 | ✅ 完成 |
| 添加必要 imports | ✅ 完成 |
| 编译验证 | ✅ 通过 |

---

## 最终启动流程

```
T0: main() 调用
    └─> runApp()  ← Native Splash 立即显示 (白底 + Logo)
    └─> Services.initAll() [后台]
        ├─> initEssential()  ← storage, settings, wallet 等 (~500ms)
        ├─> initBackground()  ← RustLib 等 (~5s)
        └─> initDeferred()  ← push, history 等

T1: initEssential() 完成
    └─> _checkWalletState()
        ├─> 有钱包 → 导航到 Home
        └─> 无钱包 → 保持在 Onboarding

T2: initAll() 完成
    └─> FlutterNativeSplash.remove()
        └─> Native Splash 移除
```

---

## 用户体验

| 阶段 | 用户看到 | 耗时 |
|------|---------|------|
| T0 - T1 | Native Splash (Logo + 白底) | ~500ms |
| T1 - T2 | Flutter 首屏 | ~5s |
| T2+ | 完整功能可用 | - |

**核心改进**:
- ✅ 零白屏
- ✅ 立即显示品牌 Logo
- ✅ 首屏自动路由到正确页面

---

## 修改文件总结

| 文件 | 变更 |
|------|------|
| `pubspec.yaml` | 添加 `flutter_native_splash` |
| `flutter_native_splash.yaml` | 新增配置 |
| `lib/main.dart` | 集成 Native Splash + 自动路由 |
| `lib/router/app_router.dart` | 移除 splash 路由 |
| `lib/views/splash_view.dart` | **已删除** |

---

## 运行命令

```bash
# 1. 安装依赖
flutter pub get

# 2. 生成 Native Splash 配置
flutter pub run flutter_native_splash:create

# 3. 运行应用
flutter run
```

---

## 配置预览

```yaml
# flutter_native_splash.yaml
color: "#FFFFFF"           # 白色背景
image: assets/icon/app_icon.png  # App Logo
width: 120                  # 120% 屏幕宽度
gravity: center             # 居中显示
fullscreen: true             # 全屏
```

---

## 下一步

运行 `flutter pub run flutter_native_splash:create` 生成原生 Splash 配置，然后启动应用即可看到效果！