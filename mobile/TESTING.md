# cowallet Flutter 端测试策略

## 一、当前状况

### 1.1 架构变化
- **旧架构**: 独立的 `SendView`/`TxTrackingView` (已废弃)
- **新架构**: AI 聊天驱动的 Widget 系统

### 1.2 测试文件状态
| 文件 | 状态 | 操作 |
|------|------|------|
| `widget_test.dart` | ✅ 有效 | 保留 |
| `test/utils/bip39_test.dart` | ❌ 引用不存在文件 | ✅ 已删除 |
| `test/views/send/send_view_test.dart` | ❌ 引用废弃视图 | ✅ 已删除 |
| `test/views/send/tx_tracking_view_test.dart` | ❌ 引用废弃视图 | ✅ 已删除 |
| `test/platform/ios_se_test.dart` | ⚠️ 未验证运行 | 待检查 |
| `test/platform/android_strongbox_test.dart` | ⚠️ 未验证运行 | 待检查 |

### 1.3 测试覆盖缺口
| 测试类型 | 覆盖率 | 优先级 |
|----------|--------|--------|
| Widget 测试 | ~5% | 🔴 P0 |
| 单元测试 | ~2% | 🔴 P0 |
| 集成测试 | 0% | 🔴 P0 |
| E2E 测试 | 0% | 🟡 P1 |

---

## 二、测试架构设计

```
mobile/test/
├── unit/                          # 单元测试
│   ├── services/                  # Service 层测试
│   │   ├── wallet_service_test.dart
│   │   ├── mpc_wallet_service_test.dart
│   │   ├── balance_service_test.dart
│   │   ├── tx_service_test.dart
│   │   └── ai_bridge_test.dart
│   ├── widgets/                   # Widget 逻辑测试
│   │   ├── send_confirm_widget_test.dart
│   │   ├── balance_widget_test.dart
│   │   └── tx_result_widget_test.dart
│   └── utils/                     # 工具函数测试
│       └── secure_storage_test.dart
├── widget/                        # Widget 测试
│   ├── views/
│   │   ├── chat_view_test.dart
│   │   └── settings_view_test.dart
│   └── widgets/
│       ├── send_confirm_widget_test.dart
│       └── tx_result_widget_test.dart
├── integration/                   # 集成测试
│   ├── wallet_creation_test.dart
│   ├── mpc_signing_test.dart
│   └── ai_chat_flow_test.dart
├── e2e/                           # 端到端测试
│   ├── create_wallet_and_send_test.dart
│   └── mpc_key_rotation_test.dart
└── mock/                          # Mock 工具
    ├── mock_services.dart
    ├── mock_ffi.dart
    └── test_fixtures.dart
```

---

## 三、核心测试实现

### 3.1 Mock 基础设施

创建 `test/mock/mock_services.dart`:

```dart
import 'package:mockito/annotations.dart';
import 'package:mockito/mockito.dart';

@GenerateMocks([
  WalletService,
  MpcWalletService,
  BalanceService,
  TxService,
  AiBridge,
])
import 'mock_services.mocks.dart';
```

### 3.2 Widget 测试示例

```dart
// test/widgets/send_confirm_widget_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:cowallet/views/chat/widgets/send_confirm_widget.dart';
import 'package:cowallet/models/tx_params.dart';
import 'package:mockito/mockito.dart';

void main() {
  group('SendConfirmWidget', () {
    testWidgets('displays transaction details correctly', (tester) async {
      final txParams = TxParams(
        to: '0x123...',
        value: BigInt.from(1000000000000000000),
        token: 'ETH',
      );

      await tester.pumpWidget(
        MaterialApp(
          home: SendConfirmWidget(txParams: txParams),
        ),
      );

      expect(find.text('1.0 ETH'), findsOneWidget);
      expect(find.text('0x123...'), findsOneWidget);
    });

    testWidgets('shows error message on failed transaction', (tester) async {
      // ... 实现
    });
  });
}
```

### 3.3 集成测试示例

```dart
// test/integration/mpc_signing_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:cowallet/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('MPC signing flow', (tester) async {
    await tester.pumpWidget(const CowalletApp());
    await tester.pumpAndSettle();

    // 1. 导航到 Chat
    // 2. 发送 "send 0.1 ETH to 0x..."
    // 3. 验证确认 Widget 显示
    // 4. 模拟生物识别
    // 5. 验证交易发送成功
  });
}
```

---

## 四、端到端测试策略

### 4.1 测试场景

| 场景 | 描述 | 优先级 |
|------|------|--------|
| 钱包创建 | DKG → 分片存储 → 地址生成 | 🔴 P0 |
| 转账流程 | AI 指令 → 确认 → 签名 → 发送 | 🔴 P0 |
| Swap 流程 | AI 指令 → 报价 → 确认 → 执行 | 🟡 P1 |
| 密钥轮换 | Reshare → 新分片 → 验证 | 🟡 P1 |
| 恢复流程 | 备份导入 → 重建设备分片 | 🟡 P1 |

### 4.2 使用 integration_test

```yaml
# pubspec.yaml
dev_dependencies:
  integration_test:
    sdk: flutter
  flutter_driver:
    sdk: flutter
  mockito: ^5.4.0
  build_runner: ^2.4.0
```

### 4.3 E2E 测试模板

```dart
// test/e2e/create_wallet_and_send_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:cowallet/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('End-to-End: Wallet & Send', () {
    testWidgets('create wallet and send ETH', (tester) async {
      app.main();

      // 等待 onboarding
      await tester.pumpAndSettle();
      expect(find.text('Create Wallet'), findsOneWidget);

      // 创建钱包
      await tester.tap(find.text('Create Wallet'));
      await tester.pumpAndSettle();

      // 等待 DKG 完成
      await tester.pump(Duration(seconds: 30));

      // 验证钱包地址
      expect(find.byType(WalletCard), findsOneWidget);

      // 通过 AI 发送
      await tester.enterText(
        find.byType(TextField),
        'Send 0.1 ETH to 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb',
      );
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      // 等待 AI 响应和 Widget
      await tester.pumpAndSettle(Duration(seconds: 5));

      // 验证确认 Widget
      expect(find.byType(SendConfirmWidget), findsOneWidget);

      // 模拟生物识别
      await tester.tap(find.text('Confirm'));
      await tester.pumpAndSettle();

      // 验证结果 Widget
      await tester.pumpAndSettle(Duration(seconds: 10));
      expect(find.byType(TxResultWidget), findsOneWidget);
      expect(find.text('Success'), findsOneWidget);
    });
  });
}
```

---

## 五、测试数据管理

### 5.1 测试固件

创建 `test/mock/test_fixtures.dart`:

```dart
class TestFixtures {
  static const String testAddress = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb';
  static const String testTxHash = '0xabc123...';

  static Map<String, dynamic> aiChatMessage({
    required String content,
    List<Map<String, dynamic>>? toolCalls,
  }) {
    return {
      'role': 'assistant',
      'content': content,
      'tool_calls': toolCalls,
    };
  }

  static Map<String, dynamic> sendToolCall({
    required String to,
    required String amount,
    String token = 'ETH',
  }) {
    return {
      'function': 'send_transaction',
      'arguments': {'to': to, 'value': amount, 'token': token},
    };
  }
}
```

---

## 六、CI/CD 集成

### 6.1 GitHub Actions 配置

```yaml
# .github/workflows/flutter-test.yml
name: Flutter Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Flutter
        uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.16.0'

      - name: Install dependencies
        run: flutter pub get

      - name: Generate mocks
        run: dart run build_runner build --delete-conflicting-outputs

      - name: Run unit tests
        run: flutter test test/unit/

      - name: Run widget tests
        run: flutter test test/widget/

      - name: Run integration tests
        run: flutter test integration_test/
```

---

## 七、实施计划

| 阶段 | 任务 | 预计时间 |
|------|------|----------|
| Week 1 | 清理无效测试，建立 Mock 基础设施 | 2 天 |
| Week 2 | 实现核心 Service 单元测试 | 3 天 |
| Week 3 | 实现 Widget 测试 (关键流程) | 3 天 |
| Week 4 | 实现集成测试 | 4 天 |
| Week 5 | 实现 E2E 测试 | 5 天 |
| Week 6 | CI/CD 集成 + 测试覆盖率报告 | 2 天 |

---

## 八、测试覆盖率目标

| 组件 | 当前 | 目标 |
|------|------|------|
| Services | ~2% | 80% |
| Widgets | ~5% | 70% |
| Views | 0% | 60% |
| Integration | 0% | 50% |

---

## 九、工具推荐

| 用途 | 工具 | 说明 |
|------|------|------|
| Mocking | mockito | 强大的 Mock 框架 |
| 代码覆盖率 | flutter test --coverage | 生成 lcov 报告 |
| E2E 测试 | integration_test | 官方集成测试包 |
| UI 测试 | flutter_driver | 已弃用，改用 integration_test |
| 可视化测试 | golden_toolkit | Widget 视觉回归测试 |

---

## 十、快速开始

```bash
# 1. 安装依赖
cd mobile
flutter pub get

# 2. 生成 Mocks
dart run build_runner build --delete-conflicting-outputs

# 3. 运行所有测试
flutter test

# 4. 运行特定测试
flutter test test/unit/services/wallet_service_test.dart

# 5. 生成覆盖率报告
flutter test --coverage
genhtml coverage/lcov.info -o coverage/html
open coverage/html/index.html

# 6. 运行集成测试
flutter test integration_test/
```