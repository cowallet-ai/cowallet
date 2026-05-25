# cowallet Native Splash 集成指南

## 概述

使用 `flutter_native_splash` 替代自定义 Splash 页面，在 Flutter 渲染层之前显示原生启动画面。

---

## 优势

| 对比 | 自定义 Splash | flutter_native_splash |
|------|-------------|-------------------|
| 显示时机 | Flutter 首帧后 | **立即显示** |
| 用户体验 | 有白屏 | **无缝启动** |
| 资源占用 | 需要 Widget | **原生原生** |
| 性能 | 加载 Flutter 需时间 | **零额外开销** |
| 跨平台 | 统一样式 | **平台原生样式** |

---

## 安装步骤

### 1. 添加依赖
```yaml
dependencies:
  flutter_native_splash: ^2.4.1
```

### 2. 创建配置文件
在项目根目录创建 `flutter_native_splash.yaml`：

```yaml
# 背景颜色
color: "#FFFFFF"

# Logo 图标
image: assets/icon/app_icon.png

# Logo 宽度 (屏幕百分比)
width: 120

# Logo 位置
gravity: center

# 全屏显示
fullscreen: true
```

### 3. 准备资源
将 App 图标放到 `assets/icon/app_icon.png` (建议 512x512)

### 4. 配置 Native Splash

```bash
# 生成配置
flutter pub get
flutter pub run flutter_native_splash:create
```

---

## 使用方式

### 自动移除 Splash

```dart
import 'package:flutter_native_splash/flutter_native_splash.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // 立即显示应用
  runApp(const MyApp());

  // 后台初始化
  await initializeServices();

  // 初始化完成后移除 Native Splash
  FlutterNativeSplash.remove();
}
```

### 延迟移除（带动画）

```dart
// 可选：延迟移除，让用户多看一会儿 Logo
Future.delayed(const Duration(seconds: 1), () {
  FlutterNativeSplash.remove();
});
```

---

## 配置选项

### Android (android_12)

```yaml
android:
  android_12:
    image: assets/icon/app_icon.png
    color: "#FFFFFF"
    # 隐藏状态栏
    hide_status_bar: false
    # 隐藏导航栏
    hide_navigation_bar: false
```

### iOS (ios)

```yaml
ios:
  ios_12:
    image: assets/icon/app_icon.png
    color: "#FFFFFF"
    # 隐藏状态栏
    hide_status_bar: false
```

### Dark Mode 支持

```yaml
# 亮色模式
color_dark: "#FFFFFF"
image_dark: assets/icon/app_icon_dark.png

# 暗色模式
android:
  android_12:
    color_dark: "#1A1A1A"
    image_dark: assets/icon/app_icon_dark.png
```

---

## 实施步骤

### 1. 安装依赖
```bash
cd mobile
flutter pub get
```

### 2. 生成 Native Splash 配置
```bash
flutter pub run flutter_native_splash:create
```

### 3. 运行应用
```bash
flutter run
```

---

## 效果

### 优化前
```
启动 → [白屏 1-2 秒] → Flutter 渲染 → 自定义 Splash → 首页
```

### 优化后
```
启动 → [Native Splash 立即显示] → Flutter 渲染 → 首页
```

---

## 故障排除

### Splash 不显示
- 检查 `flutter_native_splash.yaml` 是否存在
- 运行 `flutter pub run flutter_native_splash:create` 重新生成

### Logo 不居中
- 调整 `width` 参数
- 检查图片尺寸（建议 512x512）

### Android 12 上不显示
- 确保配置了 `android_12` 部分
- 重新生成配置：`flutter pub run flutter_native_splash:create`

### 背景颜色不对
- 确保使用 6 位 HEX 格式：`#FFFFFF`
- Dark mode 需要单独配置

---

## 资源要求

| 资源 | 建议尺寸 | 格式 |
|------|----------|------|
| Logo 图标 | 512x512 | PNG (透明) |
| 背景图 | 1080x1920 | PNG (可选) |

---

## 文件清单

| 文件 | 用途 |
|------|------|
| `pubspec.yaml` | 依赖配置 |
| `flutter_native_splash.yaml` | Splash 配置 |
| `assets/icon/app_icon.png` | Logo 资源 |

---

## 进一步优化

### 1. 品牌动画
使用 `flutter_native_splash` + Lottie 实现动画 Logo

### 2. 进度显示
在 Native Splash 后显示 Flutter Splash 进度条

### 3. 主题匹配
根据系统主题自动切换亮/暗色 Splash

---

## 参考资料

- [flutter_native_slush 文档](https://pub.dev/packages/flutter_native_splash)
- [示例配置](https://github.com/jonbhanson/flutter_native_splash)

---

**实施时间**: 2026-05-25
**预期效果**: 零白屏启动，用户体验显著提升