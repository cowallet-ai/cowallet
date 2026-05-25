# cowallet 端到端测试策略

## 一、现状分析

### 1.1 架构变化
- **已废弃**: 独立 `SendView` / `TxTrackingView`
- **当前架构**: AI 聊天驱动的 Widget 系统
- **核心流程**: 用户输入 → AI 解析 → Widget 确认 → MPC 签名 → 执行

### 1.2 测试问题
| 问题 | 原因 | 解决方案 |
|------|------|----------|
| widget_test.dart 失败 | Services.push 未初始化 | 修复或删除 |
| send_view_test.dart 失败 | 视图已废弃 | ✅ 已删除 |
| bip39_test.dart 失败 | BIP39 在 Rust 端 | ✅ 已删除 |

---

## 二、测试金字塔

```
        /\
       /E2E\     10% (关键流程)
      /------\
     /Integration\  30% (服务集成)
    /------------\
   /    Widget     \  40% (UI组件)
  /----------------\
 /      Unit        \  20% (函数/工具)
/--------------------\
```

---

## 三、端到端测试场景

### 3.1 核心场景 (P0)

| 场景 | 步骤 | 验证点 |
|------|------|--------|
| **创建钱包** | Onboarding → DKG → 生成地址 | 地址有效，3 个分片已存储 |
| **发送 ETH** | AI: "send 0.1 ETH to X" → 确认 → 生物识别 → 发送 | Widget 显示、交易成功、余额更新 |
| **Swap 代币** | AI: "swap 1 ETH to USDC" → 确认 → 执行 | 报价正确、交易成功 |
| **密钥轮换** | 设置 → 密钥管理 → 轮换 | Reshare 完成、新分片有效 |
| **恢复钱包** | 导入备份 → 验证 → 重建设备分片 | 恢复成功、可正常签名 |

### 3.2 次要场景 (P1)

| 场景 | 步骤 | 验证点 |
|------|------|--------|
| 查询余额 | AI: "check balance" | 余额 Widget 显示 |
| 交易历史 | AI: "show transactions" | 历史列表加载 |
| 添加联系人 | AI: "save address X as Alice" | 联系人保存 |
| 安全告警 | 触发高风险交易 | 告警 Widget 显示 |

---

## 四、测试环境配置

### 4.1 Docker Compose 测试环境

```yaml
# docker-compose.test.yml
version: "3.9"
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: cowallet_test
      POSTGRES_USER: test_user
      POSTGRES_PASSWORD: test_pass
    ports:
      - "5434:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6381:6379"

  api-server:
    build:
      context: ..
      dockerfile: Dockerfile
    command: api-server
    environment:
      DATABASE_URL: postgresql://test_user:test_pass@postgres:5432/cowallet_test
      REDIS_URL: redis://redis:6379
      RPC_URL: https://sepolia.base.org
      RUST_LOG: debug
    depends_on:
      - postgres
      - redis
```

### 4.2 Flutter 测试配置

```yaml
# mobile/pubspec.yaml
dev_dependencies:
  flutter_test:
    sdk: flutter
  integration_test:
    sdk: flutter
  mockito: ^5.4.0
  build_runner: ^2.4.0
  golden_toolkit: ^0.15.0
  flutter_driver:
    sdk: flutter
```

---

## 五、E2E 测试实现

### 5.1 测试驱动模式

```dart
// test/e2e/create_wallet_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:cowallet/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('E2E: Wallet Creation', () {
    testWidgets('complete DKG flow', (tester) async {
      // 1. 启动应用
      await tester.pumpWidget(const CowalletApp());
      await tester.pumpAndSettle();

      // 2. 等待 onboarding
      expect(find.text('Create Wallet'), findsOneWidget);

      // 3. 点击创建钱包
      await tester.tap(find.text('Create Wallet'));
      await tester.pumpAndSettle();

      // 4. 等待 DKG 完成 (最多 30 秒)
      await tester.pumpAndSettle(const Duration(seconds: 30));

      // 5. 验证钱包地址显示
      expect(find.byType(WalletCard), findsOneWidget);
      final addressFinder = find.byType(Text).at(1);
      expect(
        (addressFinder.evaluate().first.widget as Text).data,
        matches(RegExp(r'^0x[a-fA-F0-9]{40}$')),
      );

      // 6. 验证 3 个分片状态
      expect(find.text('Device'), findsOneWidget);
      expect(find.text('Server'), findsOneWidget);
      expect(find.text('Backup'), findsOneWidget);
    });
  });
}
```

### 5.2 发送交易测试

```dart
// test/e2e/send_transaction_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:cowallet/main.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('E2E: Send Transaction', () {
    late String testAddress = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb';

    testWidgets('AI-driven ETH send', (tester) async {
      await tester.pumpWidget(const CowalletApp());
      await tester.pumpAndSettle();

      // 1. 导航到 Chat
      await tester.tap(find.byIcon(Icons.chat));
      await tester.pumpAndSettle();

      // 2. 输入发送指令
      await tester.enterText(
        find.byType(TextField),
        'Send 0.1 ETH to $testAddress',
      );
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();

      // 3. 等待 AI 响应和 Widget
      await tester.pumpAndSettle(const Duration(seconds: 5));

      // 4. 验证确认 Widget 显示
      expect(find.byType(SendConfirmWidget), findsOneWidget);
      expect(find.text('0.1 ETH'), findsOneWidget);
      expect(find.textContaining(testAddress.substring(0, 10)), findsOneWidget);

      // 5. 点击确认
      await tester.tap(find.text('Confirm'));
      await tester.pumpAndSettle();

      // 6. 模拟生物识别成功
      // (需要 Mock BiometricService)

      // 7. 等待交易完成
      await tester.pumpAndSettle(const Duration(seconds: 10));

      // 8. 验证结果 Widget
      expect(find.byType(TxResultWidget), findsOneWidget);
      expect(find.text('Success'), findsOneWidget);
      expect(find.textContaining('0x'), findsOneWidget);
    });
  });
}
```

### 5.3 密钥轮换测试

```dart
// test/e2e/key_rotation_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('E2E: Key Rotation', () {
    testWidgets('Reshare with server', (tester) async {
      // 1. 启动应用并登录
      await tester.pumpWidget(const CowalletApp());
      await tester.pumpAndSettle();

      // 2. 导航到密钥管理
      await tester.tap(find.byIcon(Icons.key));
      await tester.pumpAndSettle();

      // 3. 点击轮换
      await tester.tap(find.text('Rotate Keys'));
      await tester.pumpAndSettle();

      // 4. 验证警告对话框
      expect(find.text('Backup your old keys'), findsOneWidget);

      // 5. 确认轮换
      await tester.tap(find.text('Continue'));
      await tester.pumpAndSettle();

      // 6. 等待 Reshare 完成
      await tester.pumpAndSettle(const Duration(seconds: 15));

      // 7. 验证新密钥
      expect(find.text('Keys Rotated Successfully'), findsOneWidget);

      // 8. 验证地址不变
      expect(find.textContaining('0x'), findsOneWidget);
    });
  });
}
```

---

## 六、运行 E2E 测试

### 6.1 本地运行

```bash
# 1. 启动测试基础设施
cd /Users/mac/cat/cowallet
docker compose -f docker-compose.test.yml up -d

# 2. 运行 E2E 测试
cd mobile
flutter test integration_test/

# 3. 清理
docker compose -f docker-compose.test.yml down -v
```

### 6.2 CI/CD 运行

```yaml
# .github/workflows/e2e.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 30

    steps:
      - uses: actions/checkout@v4

      - name: Setup Flutter
        uses: subosito/flutter-action@v2

      - name: Setup infrastructure
        run: |
          docker compose -f docker-compose.test.yml up -d
          sleep 30

      - name: Run E2E tests
        run: |
          cd mobile
          flutter test integration_test/

      - name: Upload test results
        uses: actions/upload-artifact@v3
        with:
          name: e2e-results
          path: mobile/build/test_results/

      - name: Cleanup
        run: docker compose -f docker-compose.test.yml down -v
```

---

## 七、Mock 策略

### 7.1 服务 Mock

```dart
// test/mocks/mock_mpc_wallet_service.dart
import 'package:mockito/mockito.dart';
import 'package:cowallet/services/mpc_wallet_service.dart';

class MockMpcWalletService extends Mock implements MpcWalletService {
  @override
  Future<bool> hasWallet() async => true;

  @override
  Future<String> getAddress() async => '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb';

  @override
  Future<bool> rotateKeys() async => true;
}
```

### 7.2 生物识别 Mock

```dart
// test/mocks/mock_biometrics.dart
import 'package:mockito/mockito.dart';
import 'package:cowallet/platform/biometrics.dart';

class MockBiometricService extends Mock implements BiometricService {
  @override
  Future<bool> authenticate({required String reason}) async => true;
}
```

---

## 八、测试数据管理

### 8.1 测试账户

```dart
// test/data/test_accounts.dart
class TestAccounts {
  static const Map<String, String> accounts = {
    'alice': '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb',
    'bob': '0xD265685F8b5436e2B04757c0aB53506C503aF5F',
    'charlie': '0x9e73fF4D1EaFc7A7d4C6F7a0bA3e8c9F0d1E2A3',
  };
}
```

### 8.2 测试交易

```dart
// test/data/test_transactions.dart
class TestTransactions {
  static Map<String, dynamic> ethTransfer({
    required String to,
    required double amount,
  }) {
    return {
      'to': to,
      'value': (amount * 1e18).toInt(),
      'token': 'ETH',
      'gas_limit': 21000,
    };
  }
}
```

---

## 九、实施计划

| 周次 | 任务 | 交付物 |
|------|------|--------|
| Week 1 | 清理无效测试，建立测试基础设施 | Mock 服务、测试固件 |
| Week 2 | 实现钱包创建 E2E 测试 | create_wallet_test.dart |
| Week 3 | 实现发送交易 E2E 测试 | send_transaction_test.dart |
| Week 4 | 实现密钥轮换 E2E 测试 | key_rotation_test.dart |
| Week 5 | 实现恢复流程 E2E 测试 | recovery_test.dart |
| Week 6 | CI/CD 集成 + 报告 | GitHub Actions workflow |

---

## 十、覆盖率目标

| 场景类型 | 当前 | 目标 |
|----------|------|------|
| 核心钱包流程 | 0% | 100% |
| 交易发送 | 0% | 100% |
| Swap 流程 | 0% | 80% |
| 密钥管理 | 0% | 90% |
| 恢复流程 | 0% | 100% |

---

## 十一、常见问题

### Q: E2E 测试太慢怎么办？
A: 使用并行测试、预编译镜像、Mock 外部 RPC

### Q: 如何处理依赖的服务？
A: 使用 Docker Compose 启动完整基础设施

### Q: 测试数据污染怎么办？
A: 每次测试使用独立的数据库，测试后清理

### Q: 如何模拟区块链交易？
A: 使用本地区块链节点 (Anvil/Hardhat) 或 Mock RPC 响应