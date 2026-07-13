import 'dart:convert';
import 'package:convert/convert.dart';
import '../network/dio_client.dart';
import '../network/result.dart';
import '../platform/device_identity.dart';
import '../utils/secure_storage.dart';

/// 认证API - 匹配后端实际接口
class AuthApi {
  /// 发送邮箱验证码（注册前验证邮箱所有权）
  /// [turnstileToken] Cloudflare Turnstile 人机校验 token。后端仅在配置了
  /// secret 时强制校验，否则忽略（兼容模式）。
  static Future<Result<Map<String, dynamic>>> sendEmailOtp({
    required String email,
    bool force = false,
    String turnstileToken = '',
  }) async {
    return await DioClient.post(
      "/auth/email/send-otp",
      data: {
        "email": email,
        if (force) "force": true,
        if (turnstileToken.isNotEmpty) "turnstile_token": turnstileToken,
      },
    );
  }

  /// 注册新用户
  /// [deviceId] 设备唯一标识
  /// [email] 恢复邮箱（必填）
  /// [otp] 邮箱验证码
  /// 自动附带设备硬件公钥 + 算法，供后续挑战-响应登录验签
  /// 返回 token 和 user_id
  static Future<Result<Map<String, dynamic>>> register({
    required String deviceId,
    required String email,
    required String otp,
    bool force = false,
    String? backupShardHash,
  }) async {
    // Attach the hardware device public key so challenge-response login works.
    // register runs in the OTP stage, BEFORE the bio stage creates the key, so
    // ensure it exists first — otherwise device_pubkey would be omitted and the
    // user could never log in via challenge-response.
    await DeviceIdentity.ensureInitialized();
    final devicePubkey = await DeviceIdentity.publicKeyHex();
    final deviceAlg = DeviceIdentity.algorithm;

    Result<Map<String, dynamic>> result = await DioClient.post(
      "/auth/register",
      data: {
        "device_id": deviceId,
        "email": email,
        "otp": otp,
        if (force) "force": true,
        if (backupShardHash != null) "backup_shard_hash": backupShardHash,
        if (devicePubkey != null) "device_pubkey": devicePubkey,
        if (devicePubkey != null && deviceAlg != null) "device_pubkey_alg": deviceAlg,
      },
    );

    // 注册成功自动存储token
    if (result.isSuccess) {
      String? token = result.data?["token"];
      String? refreshToken = result.data?["refresh_token"];
      String? userId = result.data?["user_id"];

      // Do not log token material, not even a prefix (F-021).
      print("📝 AuthApi.register response: token=${token != null ? "<received>" : "<null>"}, userId=$userId");

      if (token != null) {
        await SecureStorage.saveToken(token);
        print("✅ Token saved to SecureStorage");
      } else {
        print("❌ Token is null in response");
      }

      if (refreshToken != null) {
        await SecureStorage.saveRefreshToken(refreshToken);
      }

      if (userId != null) {
        await SecureStorage.saveUserId(userId);
        print("✅ UserId saved to SecureStorage");
      }
    } else {
      print("❌ Registration failed: ${result.errorMessage}");
    }
    return result;
  }

  /// 请求登录挑战 nonce
  /// 返回服务器签发的随机挑战(hex)及有效期(秒)
  static Future<Result<Map<String, dynamic>>> requestChallenge({
    required String deviceId,
  }) async {
    return await DioClient.post(
      "/auth/challenge",
      data: {"device_id": deviceId},
    );
  }

  /// 挑战-响应登录
  /// [deviceId] 设备唯一标识
  /// 流程: 请求挑战 → 用设备硬件密钥签名 → 提交 {device_id, challenge, signature}
  /// 仅持有 device_id 已无法登录,必须证明持有设备私钥。
  /// 返回 token 和 user_id
  static Future<Result<Map<String, dynamic>>> login({
    required String deviceId,
    String reason = '登录验证',
  }) async {
    // 1) 向服务器索取一次性挑战 nonce
    final challengeResult = await requestChallenge(deviceId: deviceId);
    if (!challengeResult.isSuccess || challengeResult.data == null) {
      return Result.error(
        challengeResult.errorMessage ?? 'Failed to request login challenge',
        challengeResult.errorCode ?? 500,
      );
    }
    final challengeHex = challengeResult.data!["challenge"] as String?;
    if (challengeHex == null || challengeHex.isEmpty) {
      return Result.error('Server returned an empty challenge', 500);
    }

    // 2) 用设备硬件密钥(iOS P-256 / Android RSA)签名挑战
    final challengeBytes = hex.decode(challengeHex.replaceFirst('0x', ''));
    final String signatureHex;
    try {
      signatureHex = await DeviceIdentity.signChallenge(challengeBytes, reason);
    } catch (e) {
      return Result.error('Failed to sign login challenge: $e', 401);
    }

    // 3) 提交挑战 + 签名换取 token
    Result<Map<String, dynamic>> result = await DioClient.post(
      "/auth/login",
      data: {
        "device_id": deviceId,
        "challenge": challengeHex,
        "signature": signatureHex,
      },
    );

    // 登录成功自动存储token
    if (result.isSuccess) {
      String? token = result.data?["token"];
      String? refreshToken = result.data?["refresh_token"];
      String? userId = result.data?["user_id"];
      if (token != null) {
        await SecureStorage.saveToken(token);
      }
      if (refreshToken != null) {
        await SecureStorage.saveRefreshToken(refreshToken);
      }
      if (userId != null) {
        await SecureStorage.saveUserId(userId);
      }
    }
    return result;
  }

  /// In-flight session recovery, shared across ALL callers (the 401 interceptor
  /// AND the app-startup refresh). The backend refresh token is one-time-use
  /// (rotated + blacklisted on each refresh), so two concurrent refreshes would
  /// race — the loser presents an already-blacklisted token and gets 401,
  /// wiping the session. Single-flight collapses every concurrent 401 into one
  /// recovery attempt; the rest await the same Future and then retry.
  static Future<bool>? _recovering;

  /// Recover a valid access token. Tries the refresh token first; if that fails
  /// (e.g. the 7-day refresh token expired or was already rotated), falls back
  /// to challenge-response re-login using the device hardware key.
  ///
  /// [allowInteractiveLogin] gates the challenge-response fallback, which fires
  /// a biometric/device-credential prompt via signChallenge. Single-flight
  /// means at most ONE prompt per expiry event, not one per pending request.
  ///
  /// Does NOT clear auth data on failure: a transient failure (network, 503
  /// blacklist-check) or a cancelled biometric prompt must not log the user
  /// out. Returns false and leaves stored tokens intact for the next attempt.
  static Future<bool> recoverSession({bool allowInteractiveLogin = true}) {
    final existing = _recovering;
    if (existing != null) return existing;
    final future = _doRecoverSession(allowInteractiveLogin);
    _recovering = future;
    return future.whenComplete(() {
      if (identical(_recovering, future)) _recovering = null;
    });
  }

  static Future<bool> _doRecoverSession(bool allowInteractiveLogin) async {
    // 1) Refresh token path (no user interaction). Goes through DioClient so the
    //    X-Device-ID header is attached — the refresh endpoint 403s without it.
    if (await refreshToken()) return true;

    // 2) Fallback: challenge-response re-login with the device hardware key.
    //    Requires a biometric/device-credential prompt.
    if (!allowInteractiveLogin) return false;
    final deviceId = await SecureStorage.getDeviceId();
    if (deviceId == null || deviceId.isEmpty) return false;
    try {
      final result = await login(deviceId: deviceId, reason: '会话已过期，请验证以继续');
      return result.isSuccess;
    } catch (_) {
      return false;
    }
  }

  /// 使用 refresh_token 刷新 access_token
  static Future<bool> refreshToken() async {
    final refreshToken = await SecureStorage.getRefreshToken();
    if (refreshToken == null || refreshToken.isEmpty) return false;

    try {
      final result = await DioClient.post<Map<String, dynamic>>(
        "/auth/refresh",
        data: {"refresh_token": refreshToken},
      );

      if (result.isSuccess) {
        final newToken = result.data?["token"] as String?;
        final newRefresh = result.data?["refresh_token"] as String?;
        if (newToken != null) {
          await SecureStorage.saveToken(newToken);
        }
        if (newRefresh != null) {
          await SecureStorage.saveRefreshToken(newRefresh);
        }
        return newToken != null;
      }
    } catch (_) {}
    return false;
  }

  /// 获取当前会话信息
  static Future<Result<Map<String, dynamic>>> getSessionInfo() async {
    return await DioClient.get("/auth/session");
  }

  /// 退出登录 - 仅清除认证数据，不影响钱包和设置
  static Future<void> logout() async {
    await SecureStorage.clearAuthData();
  }

  /// 永久删除账户 (App Store 5.1.1(v) 合规要求)
  ///
  /// 调用后端 `DELETE /account`，服务器在一个事务内删除该用户的所有数据
  /// (钱包、分片、交易、策略、会话、聊天记录等)。成功后清空本地全部安全存储，
  /// 使 App 回到未注册的初始状态。
  ///
  /// 注意: 这不可撤销。设备上的密钥分片一并清除，账户无法恢复。
  static Future<Result<void>> deleteAccount() async {
    final result = await DioClient.delete("/account");
    if (result.isSuccess) {
      // 服务器已删除账户，抹掉本地所有 token / 钱包地址 / 分片 / 设置。
      await SecureStorage.clearAll();
      return Result.success(null);
    }
    return Result.error(
      result.errorMessage ?? 'Failed to delete account',
      result.errorCode ?? 500,
    );
  }

  /// 检查是否已登录且 token 未过期
  static Future<bool> isLoggedIn() async {
    String? token = await SecureStorage.getToken();
    if (token == null || token.isEmpty) return false;
    return !_isTokenExpired(token);
  }

  /// 解析 JWT payload 检查 exp 是否过期（留 60s 余量）
  static bool _isTokenExpired(String token) {
    try {
      final parts = token.split('.');
      if (parts.length != 3) return true;
      final payload = utf8.decode(
        base64Url.decode(base64Url.normalize(parts[1])),
      );
      final map = jsonDecode(payload) as Map<String, dynamic>;
      final exp = map['exp'] as int?;
      if (exp == null) return true;
      final expTime = DateTime.fromMillisecondsSinceEpoch(exp * 1000);
      return DateTime.now().isAfter(expTime.subtract(const Duration(seconds: 60)));
    } catch (_) {
      return true;
    }
  }
}
