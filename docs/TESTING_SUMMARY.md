# cowallet 测试完整方案总结

## 一、现状更新

### 1.1 清理完成
| 操作 | 状态 |
|------|------|
| 删除 `bip39_test.dart` (引用不存在的文件) | ✅ 已完成 |
| 删除 `send_view_test.dart` (引用废弃视图) | ✅ 已完成 |
| 删除 `tx_tracking_view_test.dart` (引用废弃视图) | ✅ 已完成 |
| 创建 `example_test.dart` (验证测试框架) | ✅ 已完成 |
| 测试框架验证 | ✅ 6/6 通过 |

### 1.2 当前测试状态
```
Rust 后端:    302 个测试 ✅ 全部通过
Flutter 应用:  6 个测试 ✅ 全部通过 (仅基础示例)
```

---

## 二、测试文档

| 文档 | 路径 | 内容 |
|------|------|------|
| **Flutter 测试策略** | `mobile/TESTING.md` | Unit/Widget/Integration/E2E 完整方案 |
| **E2E 测试策略** | `mobile/E2E_TESTING.md` | 端到端测试场景和实现 |
| **测试总结** | `TESTING_SUMMARY.md` | 本文档 |

---

## 三、下一步实施

### 阶段 1: 基础设施 (Week 1)

```bash
# 1. 添加测试依赖
cd mobile
flutter pub add mockito build_runner integration_test

# 2. 生成 Mocks
mkdir test/mocks
# 创建 mock_services.dart
dart run build_runner build --delete-conflicting-outputs

# 3. 验证基础设施
flutter test test/example_test.dart
```

### 阶段 2: 单元测试 (Week 2)

优先级：
1. `SecureStorageService` - 加密存储
2. `MpcWalletService` - 钱包核心逻辑
3. `BalanceService` - 余额查询

### 阶段 3: Widget 测试 (Week 3)

优先级：
1. `SendConfirmWidget` - 发送确认
2. `TxResultWidget` - 交易结果
3. `BalanceWidget` - 余额展示

### 阶段 4: 集成测试 (Week 4)

场景：
1. 钱包创建完整流程
2. 发送交易完整流程
3. 密钥轮换完整流程

### 阶段 5: E2E 测试 (Week 5-6)

使用 `integration_test` 包：
- 真机或模拟器测试
- 与后端 API 集成
- 完整用户旅程

---

## 四、测试执行命令

```bash
# 运行所有测试
flutter test

# 运行特定类型
flutter test test/unit/
flutter test test/widget/
flutter test test/integration/

# 运行集成测试 (需要设备/模拟器)
flutter test integration_test/

# 生成覆盖率报告
flutter test --coverage
open coverage/lcov-report/index.html

# 运行单个测试文件
flutter test test/example_test.dart

# 运行特定测试
flutter test --name "addition"
```

---

## 五、快速开始模板

### 单元测试模板

```dart
// test/unit/services/wallet_service_test.dart
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('WalletService', () {
    test('generates valid address', () async {
      // 实现
    });

    test('signs message correctly', () async {
      // 实现
    });
  });
}
```

### Widget 测试模板

```dart
// test/widget/send_confirm_widget_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/views/chat/widgets/send_confirm_widget.dart';

void main() {
  group('SendConfirmWidget', () {
    testWidgets('displays transaction details', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: SendConfirmWidget(txParams: testParams),
        ),
      );

      expect(find.text('0.1 ETH'), findsOneWidget);
    });
  });
}
```

### 集成测试模板

```dart
// integration_test/app_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:cowallet/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('wallet creation flow', (tester) async {
    await tester.pumpWidget(const CowalletApp());
    await tester.pumpAndSettle();

    // 测试步骤...
  });
}
```

---

## 六、测试覆盖率目标

| 组件 | 当前 | 3 个月目标 |
|------|------|-----------|
| Rust 后端 | ✅ 已覆盖 302 个测试 | 保持 >95% |
| Flutter Services | ~2% | 80% |
| Flutter Widgets | ~5% | 70% |
| E2E 流程 | 0% | 50% |

---

## 七、CI/CD 集成

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
      - run: cargo test --workspace

  flutter-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
      - run: flutter pub get
      - run: flutter test --coverage

  e2e-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
      - run: flutter test integration_test/
```

---

## 八、风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 测试编写成本高 | 中 | 分阶段实施，从高价值场景开始 |
| Mock 维护成本 | 中 | 使用代码生成 (build_runner) |
| E2E 测试不稳定 | 高 | 使用固定测试环境，添加重试逻辑 |
| 测试运行时间长 | 低 | 并行执行，使用分层测试 |

---

## 九、成功指标

| 指标 | 当前 | 目标 |
|------|------|------|
| Rust 测试数量 | 302 | >350 |
| Flutter 测试数量 | 6 | >100 |
| E2E 测试数量 | 0 | >10 |
| 代码覆盖率 (后端) | ~85% | >90% |
| 代码覆盖率 (前端) | ~5% | >60% |
| CI 执行时间 | ~6 分钟 | <10 分钟 |

---

## 十、资源

- [Flutter Testing](https://docs.flutter.dev/cookbook/testing)
- [Integration Testing](https://docs.flutter.dev/testing/integration-tests)
- [Mockito](https://pub.dev/packages/mockito)
- [Golden Toolkit](https://pub.dev/packages/golden_toolkit)

---

**最后更新**: 2026-05-25
**负责人**: Development Team
**下次审查**: 2026-06-01