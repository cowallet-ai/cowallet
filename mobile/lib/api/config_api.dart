import '../network/dio_client.dart';
import '../network/result.dart';

/// Server-declared client version policy. Fetched at startup to decide whether
/// this build must force an upgrade before it can be used (the v1.0.1 MPC
/// signing protocol is not backward compatible with older builds).
class AppVersionConfig {
  /// Builds strictly below this must hard-block and upgrade.
  final int minBuild;

  /// Newest build available (reserved for a soft "update available" nudge).
  final int latestBuild;
  final String iosStoreUrl;
  final String androidStoreUrl;

  const AppVersionConfig({
    required this.minBuild,
    required this.latestBuild,
    required this.iosStoreUrl,
    required this.androidStoreUrl,
  });

  factory AppVersionConfig.fromJson(Map<String, dynamic> json) {
    int asInt(dynamic v) => v is int ? v : int.tryParse('$v') ?? 0;
    return AppVersionConfig(
      minBuild: asInt(json['min_build']),
      latestBuild: asInt(json['latest_build']),
      iosStoreUrl: (json['ios_store_url'] as String?) ?? '',
      androidStoreUrl: (json['android_store_url'] as String?) ?? '',
    );
  }
}

class ConfigApi {
  /// GET /api/v1/config/app-version (public, no auth). baseUrl already carries
  /// the /api/v1 prefix, so the path here is relative to it.
  static Future<Result<AppVersionConfig>> getAppVersion() async {
    final result = await DioClient.get<Map<String, dynamic>>('/config/app-version');
    if (result.isSuccess && result.data != null) {
      return Result.success(AppVersionConfig.fromJson(result.data!));
    }
    return Result.error(result.errorMessage ?? 'Failed to fetch app version', 0);
  }
}
