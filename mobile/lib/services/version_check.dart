import 'package:package_info_plus/package_info_plus.dart';

import '../api/config_api.dart';

/// Outcome of the startup version check.
class VersionCheckResult {
  /// True when this build is below the server's min_build and must hard-block.
  final bool mustUpgrade;
  final String iosStoreUrl;
  final String androidStoreUrl;

  const VersionCheckResult({
    required this.mustUpgrade,
    this.iosStoreUrl = '',
    this.androidStoreUrl = '',
  });

  static const ok = VersionCheckResult(mustUpgrade: false);
}

/// Client half of the two-layer forced-upgrade gate. Asks the server for the
/// minimum supported build and compares it to this build's number.
///
/// Fail OPEN on every uncertainty (network error, unparseable version, missing
/// config): a backend hiccup must never lock a user out of their wallet. The
/// server-side gate (426 on protected routes) is the hard stop that still
/// catches a stale client even when this check is skipped.
class VersionCheck {
  static Future<VersionCheckResult> check() async {
    try {
      final result = await ConfigApi.getAppVersion();
      if (!result.isSuccess || result.data == null) {
        return VersionCheckResult.ok;
      }
      final cfg = result.data!;
      if (cfg.minBuild <= 0) return VersionCheckResult.ok;

      final info = await PackageInfo.fromPlatform();
      final currentBuild = int.tryParse(info.buildNumber) ?? 0;

      return VersionCheckResult(
        mustUpgrade: currentBuild < cfg.minBuild,
        iosStoreUrl: cfg.iosStoreUrl,
        androidStoreUrl: cfg.androidStoreUrl,
      );
    } catch (_) {
      return VersionCheckResult.ok;
    }
  }
}
