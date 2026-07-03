import 'package:flutter/foundation.dart';

/// Lightweight step timer for measuring per-step latency of a multi-step flow.
///
/// All output is gated behind [kDebugMode], so release builds pay only the cost
/// of two [Stopwatch]es and a list — no logging. Uses [debugPrint] (not `print`)
/// so it does not trip the `avoid_print` lint and is stripped in release.
///
/// Usage:
/// ```dart
/// final t = StepTimer('TxSend');
/// final addr = await wallet.getAddress();
/// t.mark('getAddress');
/// ...
/// t.done();
/// ```
class StepTimer {
  final String _tag;
  final Stopwatch _total = Stopwatch()..start();
  final Stopwatch _lap = Stopwatch()..start();
  final List<MapEntry<String, int>> _steps = [];

  StepTimer(this._tag);

  /// Record milliseconds elapsed since the last [mark] (or construction)
  /// under [label], then reset the lap clock.
  void mark(String label) {
    if (!kDebugMode) return;
    _steps.add(MapEntry(label, _lap.elapsedMilliseconds));
    _lap.reset();
  }

  /// Emit all collected steps plus the total elapsed time as a single log line.
  /// Safe to call in a `finally` block — prints whatever was marked so far.
  void done() {
    if (!kDebugMode) return;
    final total = _total.elapsedMilliseconds;
    final buf = StringBuffer('[$_tag] timing (ms):');
    for (final s in _steps) {
      buf.write('\n  ${s.key.padRight(24)}${s.value}');
    }
    buf.write('\n  ${'TOTAL'.padRight(24)}$total');
    debugPrint(buf.toString());
  }
}
