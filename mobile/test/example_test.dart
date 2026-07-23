/// 简单的测试示例 - 验证 Flutter 测试框架正常工作
library;

import 'package:flutter_test/flutter_test.dart';

void main() {
  group('Basic Tests', () {
    test('addition works', () {
      expect(2 + 2, equals(4));
    });

    test('string concatenation works', () {
      expect('Hello' ' ' 'World', equals('Hello World'));
    });

    test('list operations work', () {
      final list = [1, 2, 3];
      list.add(4);
      expect(list, equals([1, 2, 3, 4]));
      expect(list.length, equals(4));
    });

    test('exception throwing works', () {
      expect(() => throw Exception('test'), throwsException);
    });
  });

  group('Async Tests', () {
    test('Future completes successfully', () async {
      final result = await Future.delayed(
        const Duration(milliseconds: 100),
        () => 'done',
      );
      expect(result, equals('done'));
    });

    test('Stream emits values', () async {
      final stream = Stream.fromIterable([1, 2, 3]);
      final values = await stream.toList();
      expect(values, equals([1, 2, 3]));
    });
  });
}