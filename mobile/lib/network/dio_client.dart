import 'package:flutter/foundation.dart';
import 'dart:async';
import 'dart:convert';
import 'package:dio/dio.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:pretty_dio_logger/pretty_dio_logger.dart';
import '../config/api_config.dart';
import '../utils/secure_storage.dart';
import 'result.dart';

class DioClient {
  static Dio? _instance;

  /// This build's integer version, stamped on every request as X-App-Version so
  /// the server-side version gate can reject outdated clients (v1.0.1 MPC
  /// protocol is not backward compatible). Read once from PackageInfo, cached.
  static int? _appBuild;

  static Future<int> _buildNumber() async {
    if (_appBuild != null) return _appBuild!;
    try {
      final info = await PackageInfo.fromPlatform();
      _appBuild = int.tryParse(info.buildNumber) ?? 0;
    } catch (_) {
      _appBuild = 0;
    }
    return _appBuild!;
  }

  /// Injected by the app layer to navigate to the blocking force-upgrade screen
  /// when the server answers 426 Upgrade Required. Kept as a callback so this
  /// low-level file doesn't import UI. Receives the parsed 426 JSON body.
  static void Function(Map<String, dynamic> body)? onUpgradeRequired;

  /// Injected by the app layer (main.dart) to recover a session on 401 —
  /// refresh-token first, then challenge-response re-login. Kept as a callback
  /// so this low-level network file doesn't import AuthApi (avoids a cycle) and
  /// so both the interceptor and startup share ONE single-flight recovery.
  /// Returns true if a valid token is now stored.
  static Future<bool> Function()? sessionRecoverer;

  /// Marks a request that has already been retried after a recovery, so a
  /// second 401 on the retry doesn't trigger another recovery loop.
  static const String _retriedFlag = 'x-session-retried';

  static Dio get instance {
    if (_instance == null) {
      _initDio();
    }
    return _instance!;
  }

  static void _initDio() {
    BaseOptions options = BaseOptions(
      baseUrl: ApiConfig.apiBaseUrl,
      connectTimeout: Duration(seconds: ApiConfig.connectTimeout),
      receiveTimeout: Duration(seconds: ApiConfig.receiveTimeout),
      headers: {
        "Content-Type": "application/json",
        "Accept": "application/json",
      },
    );

    _instance = Dio(options);

    // 添加拦截器
    _instance!.interceptors.addAll([
      // Token自动添加拦截器
      InterceptorsWrapper(
        onRequest: (options, handler) async {
          try {
            // 从安全存储拿token，自动加到请求头
            String? token = await SecureStorage.getToken();

            if (token != null && token.isNotEmpty) {
              options.headers["Authorization"] = "Bearer $token";
              // Do not log token contents, not even a prefix (F-021): a prefix
              // still leaks the JWT header/alg and narrows brute-force space.
              debugPrint("✅ [DioClient] Token attached");
            } else {
              debugPrint("⚠️  [DioClient] No token found in SecureStorage");
              // 尝试直接检查文件
              debugPrint("   Path: ${options.path}");
            }

            // Mandatory device binding (F-010): protected routes reject requests
            // without an X-Device-ID header matching the JWT's device_id.
            final deviceId = await SecureStorage.getDeviceId();
            if (deviceId != null && deviceId.isNotEmpty) {
              options.headers["X-Device-ID"] = deviceId;
            }

            // Version gate (F: forced upgrade). Server compares this to
            // MIN_APP_BUILD and returns 426 for stale clients.
            options.headers["X-App-Version"] = (await _buildNumber()).toString();
          } catch (e) {
            debugPrint("❌ [DioClient] Error reading token: $e");
          }
          return handler.next(options);
        },
        onResponse: (response, handler) {
          return handler.next(response);
        },
        onError: (DioException e, handler) async {
          final opts = e.requestOptions;
          final path = opts.path;

          // 426 Upgrade Required: server rejected this build. Trigger the
          // blocking upgrade screen and stop — no retry can succeed until the
          // user updates. Handled before the 401 recovery path.
          if (e.response?.statusCode == 426) {
            final body = e.response?.data;
            onUpgradeRequired?.call(
              body is Map<String, dynamic> ? body : <String, dynamic>{},
            );
            return handler.next(e);
          }

          // Only attempt recovery on a genuine 401, once per request, and never
          // for the auth endpoints themselves (they'd recurse). The recoverer
          // is single-flight, so concurrent 401s collapse into ONE refresh /
          // re-login and all retry afterwards.
          final isAuthPath = path.contains('/auth/refresh') ||
              path.contains('/auth/login') ||
              path.contains('/auth/register') ||
              path.contains('/auth/challenge');
          final alreadyRetried = opts.extra[_retriedFlag] == true;

          if (e.response?.statusCode != 401 ||
              isAuthPath ||
              alreadyRetried ||
              sessionRecoverer == null) {
            return handler.next(e);
          }

          try {
            final recovered = await sessionRecoverer!();
            if (!recovered) {
              // Leave stored tokens as-is: a transient failure or a declined
              // biometric prompt must not silently log the user out. The next
              // request (or app restart) retries recovery.
              debugPrint("⚠️  Session recovery failed for $path");
              return handler.next(e);
            }
            // Retry the original request once with the fresh token.
            final token = await SecureStorage.getToken();
            opts.headers["Authorization"] = "Bearer $token";
            opts.extra[_retriedFlag] = true;
            final response = await _instance!.fetch(opts);
            return handler.resolve(response);
          } catch (_) {
            return handler.next(e);
          }
        },
      ),
      // 日志拦截器，开发环境开启，生产环境关闭
      if (!const bool.fromEnvironment('dart.vm.product'))
        PrettyDioLogger(
          requestHeader: true,
          requestBody: true,
          responseHeader: true,
          responseBody: true,
          error: true,
          maxWidth: 120,
        ),
    ]);
  }

  // 统一请求方法，返回Result封装
  static Future<Result<T>> request<T>(
    String path, {
    String method = "GET",
    Map<String, dynamic>? params,
    dynamic data,
    Options? options,
    CancelToken? cancelToken,
  }) async {
    try {
      Options requestOptions = options ?? Options();
      requestOptions.method = method;

      Response response = await instance.request(
        path,
        queryParameters: params,
        data: data,
        options: requestOptions,
        cancelToken: cancelToken,
      );

      // 响应处理 - 根据你的后端实际返回格式调整
      // 任意 2xx 均视为成功：DELETE /account 等接口返回 204 No Content（空 body），
      // 只认 200/201 会把 204 误判为失败——账户已在服务端删除，客户端却弹"删除失败"
      // 且跳过本地清理。Dio 默认 validateStatus 也只放行 2xx，与此判定一致。
      final statusCode = response.statusCode ?? 0;
      if (statusCode >= 200 && statusCode < 300) {
        // 如果后端直接返回数据，没有外层包装
        return Result.success(response.data as T);
        // 如果后端有标准包装格式：{ "code": 0, "msg": "success", "data": {} }
        // 取消上面的注释，用下面的逻辑：
        // if (response.data["code"] == 0) {
        //   return Result.success(response.data["data"] as T);
        // } else {
        //   return Result.error(
        //     response.data["msg"] ?? "请求失败",
        //     response.data["code"] ?? -1,
        //   );
        // }
      } else {
        return Result.error(
          "请求失败，状态码：${response.statusCode}",
          response.statusCode ?? -1,
        );
      }
    } on DioException catch (e) {
      String errorMsg = _handleError(e);
      return Result.error(errorMsg, e.response?.statusCode ?? -1);
    } catch (e) {
      return Result.error("未知错误：${e.toString()}", -1);
    }
  }

  // 快捷请求方法
  static Future<Result<T>> get<T>(
    String path, {
    Map<String, dynamic>? params,
    Map<String, dynamic>? queryParameters,
    Options? options,
    CancelToken? cancelToken,
  }) async {
    return request<T>(
      path,
      method: "GET",
      params: queryParameters ?? params,
      options: options,
      cancelToken: cancelToken,
    );
  }

  static Future<Result<T>> post<T>(
    String path, {
    dynamic data,
    Map<String, dynamic>? params,
    Options? options,
    CancelToken? cancelToken,
  }) async {
    return request<T>(
      path,
      method: "POST",
      data: data,
      params: params,
      options: options,
      cancelToken: cancelToken,
    );
  }

  static Future<Result<T>> put<T>(
    String path, {
    dynamic data,
    Map<String, dynamic>? params,
    Options? options,
    CancelToken? cancelToken,
  }) async {
    return request<T>(
      path,
      method: "PUT",
      data: data,
      params: params,
      options: options,
      cancelToken: cancelToken,
    );
  }

  static Future<Result<T>> delete<T>(
    String path, {
    dynamic data,
    Map<String, dynamic>? params,
    Map<String, dynamic>? queryParameters,
    Options? options,
    CancelToken? cancelToken,
  }) async {
    return request<T>(
      path,
      method: "DELETE",
      data: data,
      params: queryParameters ?? params,
      options: options,
      cancelToken: cancelToken,
    );
  }

  /// POST request that returns an SSE stream (text/event-stream).
  static Future<Stream<String>?> postStream(
    String path, {
    dynamic data,
  }) async {
    try {
      String? token = await SecureStorage.getToken();
      final deviceId = await SecureStorage.getDeviceId();
      final dio = Dio(BaseOptions(
        baseUrl: ApiConfig.apiBaseUrl,
        connectTimeout: Duration(seconds: ApiConfig.connectTimeout),
        receiveTimeout: const Duration(seconds: 120),
        headers: {
          "Content-Type": "application/json",
          "Accept": "text/event-stream",
          if (token != null) "Authorization": "Bearer $token",
          // Mandatory device binding (F-010).
          if (deviceId != null && deviceId.isNotEmpty) "X-Device-ID": deviceId,
          // Version gate: this stream uses its own Dio instance, so the shared
          // interceptor that stamps X-App-Version does NOT run here. Without it
          // the server sees build 0 and returns 426 Upgrade Required.
          "X-App-Version": (await _buildNumber()).toString(),
        },
        responseType: ResponseType.stream,
      ));

      final response = await dio.post(
        path,
        data: data,
      );

      final stream = (response.data as ResponseBody).stream;
      return stream.transform(
        StreamTransformer<Uint8List, String>.fromHandlers(
          handleData: (Uint8List data, EventSink<String> sink) {
            sink.add(utf8.decode(data, allowMalformed: true));
          },
        ),
      );
    } catch (e) {
      debugPrint("❌ [DioClient] Stream request failed: $e");
      return null;
    }
  }

  // 错误处理
  static String _handleError(DioException e) {
    switch (e.type) {
      case DioExceptionType.connectionTimeout:
        return "连接超时，请检查网络";
      case DioExceptionType.sendTimeout:
        return "请求发送超时";
      case DioExceptionType.receiveTimeout:
        return "响应超时，请稍后重试";
      case DioExceptionType.badResponse:
        int? statusCode = e.response?.statusCode;
        if (statusCode == 400) return "请求参数错误";
        if (statusCode == 401) return "登录已过期，请重新登录";
        if (statusCode == 403) return "没有权限访问";
        if (statusCode == 404) return "请求的资源不存在";
        if (statusCode == 500) return "服务器内部错误";
        return "服务器错误，状态码：$statusCode";
      case DioExceptionType.cancel:
        return "请求已取消";
      case DioExceptionType.connectionError:
        return "网络连接失败，请检查网络设置";
      default:
        return "未知网络错误，请稍后重试";
    }
  }
}
