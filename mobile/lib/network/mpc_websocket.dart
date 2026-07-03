import 'dart:async';
import 'dart:convert';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../config/api_config.dart';
import '../api/mpc_api.dart';

/// MPC协议消息
class MpcMessage {
  final int fromParty;
  final int toParty;
  final int round;
  final List<int> payload;

  const MpcMessage({
    required this.fromParty,
    required this.toParty,
    required this.round,
    required this.payload,
  });

  factory MpcMessage.fromJson(Map<String, dynamic> json) {
    return MpcMessage(
      fromParty: json['from_party'] as int,
      toParty: json['to_party'] as int,
      round: json['round'] as int,
      payload: (json['payload'] as List<dynamic>).cast<int>(),
    );
  }

  Map<String, dynamic> toJson() => {
        'from_party': fromParty,
        'to_party': toParty,
        'round': round,
        'payload': payload,
      };
}

/// WebSocket连接状态
enum MpcWebSocketState {
  disconnected,
  connecting,
  connected,
  reconnecting,
}

/// 管理MPC会话的WebSocket连接，用于实时传输MPC协议消息。
/// 连接地址: ws://host/api/v1/mpc/session/{sessionId}/ws?party={partyIndex}&ticket={ticket}
/// 票据为一次性、30秒有效，由 /auth/ws-ticket 用 JWT 换取——避免 JWT 进入 URL/日志。
class MpcWebSocket {
  final String sessionId;
  final int partyIndex;

  /// Optional callback invoked when the connection drops unexpectedly.
  /// Used by session recovery to persist state on disconnection.
  final void Function()? onUnexpectedDisconnect;

  WebSocketChannel? _channel;
  StreamController<MpcMessage>? _messageController;
  StreamSubscription? _subscription;
  Timer? _reconnectTimer;
  Timer? _heartbeatTimer;
  int _reconnectAttempts = 0;
  bool _disposed = false;
  MpcWebSocketState _state = MpcWebSocketState.disconnected;

  static const int _maxReconnectAttempts = 5;
  static const Duration _heartbeatInterval = Duration(seconds: 30);

  MpcWebSocket({
    required this.sessionId,
    required this.partyIndex,
    this.onUnexpectedDisconnect,
  });

  /// 当前连接状态
  MpcWebSocketState get state => _state;

  /// 是否已连接
  bool get isConnected => _state == MpcWebSocketState.connected;

  /// 消息流，监听此流以接收MPC消息
  Stream<MpcMessage> get messages {
    _messageController ??= StreamController<MpcMessage>.broadcast();
    return _messageController!.stream;
  }

  /// Buffer of server messages received from the raw socket. Because
  /// [messages] is a broadcast stream, any message that arrives before a
  /// consumer calls `.listen()` is silently dropped. The MPC flow sends its
  /// request over HTTP and only then subscribes to await the reply — and the
  /// server pushes that reply back over the socket within ~1ms, well before
  /// the subscription is established. Buffering every inbound message here lets
  /// [takeBuffered] recover the ones that arrived in that window, instead of
  /// waiting out the 45s socket timeout and falling back to slow HTTP polling.
  final List<MpcMessage> _buffer = [];
  static const int _maxBuffer = 100;

  /// Remove and return buffered messages matching [fromParty] (and [round] when
  /// given). Consumed messages are dropped from the buffer so a later wait for a
  /// different round does not re-read them.
  List<MpcMessage> takeBuffered({required int fromParty, int? round}) {
    final matched = <MpcMessage>[];
    _buffer.removeWhere((m) {
      final hit = m.fromParty == fromParty && (round == null || m.round == round);
      if (hit) matched.add(m);
      return hit;
    });
    return matched;
  }

  /// Drop a message from the buffer (used after it is delivered live via the
  /// stream, so it is not also returned by a later [takeBuffered]).
  void dropBuffered(MpcMessage message) {
    _buffer.remove(message);
  }

  /// 构建WebSocket URL
  /// 将HTTP URL转换为WS URL，并附加session/party/ticket参数。
  /// 每次连接(含重连)都换取一张新的一次性票据——票据30秒有效且仅可用一次。
  Future<Uri> _buildWsUri() async {
    final ticketResult = await MpcApi.getWsTicket();
    if (!ticketResult.isSuccess || ticketResult.data == null) {
      throw Exception(
          'Failed to obtain ws ticket: ${ticketResult.errorMessage ?? 'unknown error'}');
    }
    final String ticket = ticketResult.data!;

    // 将 http:// 或 https:// 转换为 ws:// 或 wss://
    String wsBase = ApiConfig.baseUrl
        .replaceFirst('http://', 'ws://')
        .replaceFirst('https://', 'wss://');

    String url =
        '$wsBase${ApiConfig.apiPrefix}/mpc/session/$sessionId/ws?party=$partyIndex&ticket=$ticket';

    return Uri.parse(url);
  }

  /// 连接WebSocket
  /// 如果已连接则先断开再重连
  Future<void> connect() async {
    if (_disposed) return;
    if (_state == MpcWebSocketState.connected ||
        _state == MpcWebSocketState.connecting) {
      return;
    }

    _state = MpcWebSocketState.connecting;
    _messageController ??= StreamController<MpcMessage>.broadcast();

    try {
      Uri uri = await _buildWsUri();
      print('[MpcWebSocket] Connecting to: $uri');

      _channel = WebSocketChannel.connect(uri);

      // 等待连接就绪（with timeout to avoid hanging）
      await _channel!.ready.timeout(const Duration(seconds: 5));

      if (_disposed) return;

      _state = MpcWebSocketState.connected;
      _reconnectAttempts = 0;
      print('[MpcWebSocket] Connected successfully');

      // 监听消息
      _subscription = _channel!.stream.listen(
        _onMessage,
        onError: _onError,
        onDone: _onDone,
      );

      // 启动心跳
      _startHeartbeat();
    } catch (e) {
      if (_disposed) return;
      print('[MpcWebSocket] Connection failed: $e');
      _state = MpcWebSocketState.disconnected;
      _scheduleReconnect();
    }
  }

  /// 断开WebSocket连接
  Future<void> disconnect() async {
    _disposed = true;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _heartbeatTimer?.cancel();
    _heartbeatTimer = null;
    _reconnectAttempts = _maxReconnectAttempts; // 阻止自动重连

    await _subscription?.cancel();
    _subscription = null;

    try {
      await _channel?.sink.close().timeout(const Duration(seconds: 2));
    } catch (_) {}
    _channel = null;

    _state = MpcWebSocketState.disconnected;
    print('[MpcWebSocket] Disconnected');

    await _messageController?.close();
    _messageController = null;
  }

  /// 发送MPC消息
  /// [message] 要发送的MPC消息
  void send(MpcMessage message) {
    if (_state != MpcWebSocketState.connected || _channel == null) {
      print('[MpcWebSocket] Cannot send: not connected');
      return;
    }

    String jsonStr = jsonEncode(message.toJson());
    _channel!.sink.add(jsonStr);
  }

  /// 发送原始MPC消息参数
  void sendRaw({
    required int toParty,
    required int round,
    required List<int> payload,
  }) {
    send(MpcMessage(
      fromParty: partyIndex,
      toParty: toParty,
      round: round,
      payload: payload,
    ));
  }

  /// 处理收到的消息
  void _onMessage(dynamic data) {
    try {
      Map<String, dynamic> json;
      if (data is String) {
        json = jsonDecode(data) as Map<String, dynamic>;
      } else {
        // 二进制消息，尝试UTF-8解码
        json = jsonDecode(utf8.decode(data as List<int>))
            as Map<String, dynamic>;
      }

      // 忽略心跳pong响应
      if (json.containsKey('type') && json['type'] == 'pong') {
        return;
      }

      MpcMessage message = MpcMessage.fromJson(json);
      // Buffer first (bounded), then broadcast. A consumer that has not yet
      // subscribed recovers this message via takeBuffered(); one already
      // listening gets it live and calls dropBuffered() to avoid a re-read.
      _buffer.add(message);
      if (_buffer.length > _maxBuffer) {
        _buffer.removeAt(0);
      }
      _messageController?.add(message);
    } catch (e) {
      print('[MpcWebSocket] Error parsing message: $e');
    }
  }

  /// 处理连接错误
  void _onError(dynamic error) {
    print('[MpcWebSocket] Error: $error');
    _state = MpcWebSocketState.disconnected;
    _heartbeatTimer?.cancel();
    onUnexpectedDisconnect?.call();
    _scheduleReconnect();
  }

  /// 处理连接关闭
  void _onDone() {
    print('[MpcWebSocket] Connection closed');
    final wasConnected = _state == MpcWebSocketState.connected;
    _state = MpcWebSocketState.disconnected;
    _heartbeatTimer?.cancel();
    if (wasConnected) {
      onUnexpectedDisconnect?.call();
    }
    _scheduleReconnect();
  }

  /// 启动心跳定时器
  void _startHeartbeat() {
    _heartbeatTimer?.cancel();
    _heartbeatTimer = Timer.periodic(_heartbeatInterval, (_) {
      if (_state == MpcWebSocketState.connected && _channel != null) {
        _channel!.sink.add(jsonEncode({'type': 'ping'}));
      }
    });
  }

  /// 安排自动重连（指数退避: 1s, 2s, 4s, 8s, 16s）
  void _scheduleReconnect() {
    if (_disposed) return;
    if (_reconnectAttempts >= _maxReconnectAttempts) {
      print('[MpcWebSocket] Max reconnect attempts reached');
      _messageController?.addError(
        Exception('WebSocket connection failed after $_maxReconnectAttempts attempts'),
      );
      return;
    }

    _state = MpcWebSocketState.reconnecting;
    int delaySeconds = 1 << _reconnectAttempts; // 1, 2, 4, 8, 16
    _reconnectAttempts++;

    print('[MpcWebSocket] Reconnecting in ${delaySeconds}s (attempt $_reconnectAttempts/$_maxReconnectAttempts)');

    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(Duration(seconds: delaySeconds), () async {
      if (_disposed) return;
      await _subscription?.cancel();
      _subscription = null;
      _channel = null;
      await connect();
    });
  }
}
