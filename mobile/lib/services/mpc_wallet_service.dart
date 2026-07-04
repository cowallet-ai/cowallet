import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:convert/convert.dart';
import '../api/mpc_api.dart';
import '../bridge/mpc_bridge.dart';
import '../network/mpc_websocket.dart';
import '../platform/cloud_backup.dart';
import '../platform/secure_hardware.dart';
import '../utils/secure_storage.dart';
import '../utils/mpc_hmac.dart';
import 'backup_shard_service.dart';
import 'locator.dart';
import 'key_health_service.dart';
import 'step_timer.dart';
import 'wallet_service.dart';
import 'mpc_session_store.dart';

/// MPC 门限签名钱包服务
/// 实现 2-of-3 门限签名密钥生成、签名、密钥轮转、预签名
/// - Party 0: 本地设备 (Secure Enclave / StrongBox)
/// - Party 1: 后端服务 (自动参与协议)
/// - Party 2: 备份分片 (iCloud Keychain / Google Cloud Backup / 用户离线保管)
class MpcWalletService implements WalletService {
  String? _currentSessionId;
  /// Per-session HMAC key (hex) returned by create_session. Used to
  /// authenticate server-bound MPC messages (F-004).
  String? _currentHmacKey;
  BackupResult? _lastBackupResult;
  List<int>? _lastBackupShard;
  bool _backupNeedsReExport = false;
  int _lastMessageId = 0;
  bool _signInProgress = false;
  Completer<void>? _operationLock;
  static const int _deviceParty = 0;
  static const int _serverParty = 1;
  static const int _backupParty = 2;
  static const Duration _wsTimeout = Duration(seconds: 45);

  bool get signInProgress => _signInProgress;

  /// 执行完整的 DKG 密钥生成协议
  /// [walletId] 可选，用于多钱包场景
  Future<WalletInfo> runDkg({String? walletId}) async {
    // Ensure the Rust FFI bridge is ready before any DKG FFI call. Background
    // init may not have finished (or may have timed out) when the user reaches
    // wallet creation; this awaits/performs init idempotently to avoid the
    // "flutter_rust_bridge has not been initialized" race.
    await MpcBridge.ensureInitialized();

    final sessionResult = await MpcApi.createSession(
      sessionType: 'keygen',
      parties: [_deviceParty, _serverParty],
      threshold: 2,
      walletId: walletId,
    );

    if (!sessionResult.isSuccess || sessionResult.data == null) {
      throw MpcException('Failed to create DKG session: ${sessionResult.errorMessage}');
    }

    final sessionId = sessionResult.data!['session_id'] as String;
    _currentSessionId = sessionId;
    _currentHmacKey = sessionResult.data!['hmac_key'] as String?;
    _lastMessageId = 0;

    final ws = MpcWebSocket(sessionId: sessionId, partyIndex: _deviceParty);
    try {
      await ws.connect();

      // Subscribe to messages immediately after connect to capture catch-up messages
      final serverMessages = <MpcMessage>[];
      final messagesReady = Completer<List<MpcMessage>>();
      final subscription = ws.messages.listen((msg) {
        if (msg.fromParty == _serverParty) {
          serverMessages.add(msg);
          if (serverMessages.length >= 2 && !messagesReady.isCompleted) {
            messagesReady.complete(serverMessages);
          }
        }
      });

      final localSessionId = await MpcBridge.dkgSessionNew(_deviceParty);

      // Save initial session state for recovery
      await MpcSessionStore.saveSession(MpcSessionState(
        sessionId: localSessionId,
        remoteSessionId: sessionId,
        sessionType: 'keygen',
        currentRound: 0,
        createdAt: DateTime.now(),
      ));

      final round1Json = await MpcBridge.dkgGenerateRound1(localSessionId);
      // Server expects the raw payload bytes from the ProtocolMessage, not the full JSON.
      // Extract the "payload" field (array of ints) from the JSON.
      final round1Msg = jsonDecode(round1Json) as Map<String, dynamic>;
      final round1Payload = List<int>.from(round1Msg['payload'] as List);

      // Send Round 1 via HTTP (reliable delivery)
      await _sendToServer(
        sessionId: sessionId,
        round: 1,
        payload: round1Payload,
      );

      // Update progress
      await MpcSessionStore.updateCurrentRound(1);

      // Wait for server's Round 1 + Round 2 (listener started before send)
      final allServerMessages = await messagesReady.future.timeout(
        _wsTimeout,
        onTimeout: () async {
          await subscription.cancel();
          // Fallback to HTTP polling
          if (_currentSessionId != null) {
            return await _pollMessagesFallback(
              sessionId: _currentSessionId!,
              party: _deviceParty,
              expectedCount: 2,
            );
          }
          throw MpcException('Timeout waiting for server response via WebSocket');
        },
      );
      await subscription.cancel();

      // Server returns raw payload bytes; wrap them into ProtocolMessage JSON for the FFI.
      final serverRound1Msgs = allServerMessages
          .where((m) => m.round == 1)
          .map((m) => _wrapAsProtocolMessage(sessionId, m))
          .toList();

      await MpcBridge.dkgProcessRound1(localSessionId, serverRound1Msgs);

      final round2Msgs = await MpcBridge.dkgGenerateRound2(localSessionId);
      for (final msgJson in round2Msgs) {
        final msg = jsonDecode(msgJson) as Map<String, dynamic>;
        final to = msg['to'] as int;
        if (to == _serverParty) {
          final round2Payload = List<int>.from(msg['payload'] as List);
          // Send Round 2 via HTTP (reliable delivery)
          await _sendToServer(
            sessionId: sessionId,
            round: 2,
            payload: round2Payload,
          );
        }
      }

      // Update progress
      await MpcSessionStore.updateCurrentRound(2);

      // Process server's Round 2
      final serverRound2Msgs = allServerMessages
          .where((m) => m.round == 2 && m.fromParty == _serverParty)
          .map((m) => _wrapAsProtocolMessage(sessionId, m))
          .toList();

      if (serverRound2Msgs.isNotEmpty) {
        await MpcBridge.dkgProcessRound2(localSessionId, serverRound2Msgs);
      }

      final walletInfo = await MpcBridge.dkgFinalize(localSessionId);

      // Derive backup shard (device + server combined) and keep in memory.
      // The UI will prompt the user to choose a storage method.
      try {
        _lastBackupShard = await _deriveBackupShard(localSessionId);
        print('[MpcWalletService] Backup shard derived successfully (${_lastBackupShard!.length} bytes)');
      } catch (e) {
        print('[MpcWalletService] Backup shard derivation skipped: $e');
      }

      await SecureStorage.save('mpc_address', walletInfo.address);
      await SecureStorage.save('mpc_session_id', sessionId);

      // Fresh DKG: clear stale recovery commitment so verification uses Lagrange
      await SecureStorage.delete('mpc_server_commitment');

      // Save the public key. The device shard is intentionally NOT persisted
      // here: storing it under a hardware-backed (biometric-bound) key would
      // trigger a biometric prompt before the user has chosen their auth method
      // on the bio/pin screen. The shard stays in Rust memory; the onboarding
      // flow calls persistDeviceShard(...) after the user makes that choice.
      final pubKeyHex = walletInfo.publicKey.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
      await SecureStorage.save('mpc_public_key', pubKeyHex);

      // Clear session state on success
      await MpcSessionStore.clearSession();

      return walletInfo;
    } catch (e) {
      // Save state on error for potential recovery
      print('[MpcWalletService] DKG error: $e');
      throw MpcSessionInterruptedException(
        'DKG session interrupted: $e',
        sessionState: await MpcSessionStore.loadSession(),
      );
    } finally {
      await ws.disconnect();
    }
  }

  /// Persist the device shard (Party 0) after the user has chosen their auth
  /// method on the bio/pin screen. Called by the onboarding flow once, after
  /// DKG completes and the choice is made — NOT during DKG — so the biometric
  /// prompt only appears when the user has opted into biometric protection.
  ///
  /// The shard is exported fresh from Rust memory (kept there by runDkg) and
  /// stored under the hardware-backed, auth-bound key (biometric / device
  /// credential). Safe to call again on re-provision.
  Future<void> persistDeviceShard() async {
    final deviceShardBytes = await MpcBridge.exportDeviceShard();
    await SecureHardware.storeDeviceShard(Uint8List.fromList(deviceShardBytes));
  }

  /// Persist the device shard encrypted with the user's PIN (app-layer
  /// Argon2id + AES-256-GCM, NO hardware/biometric key). Used for the PIN-only
  /// auth path so storing the shard does not trigger a biometric prompt. The
  /// ciphertext blob is kept in ordinary secure storage.
  Future<void> persistDeviceShardWithPin(String pin) async {
    final blob = await MpcBridge.exportDeviceShardEncrypted(pin);
    await SecureStorage.save(SecureStorage.keyPinEncryptedShard, blob);
  }

  /// Whether the device shard is stored under the PIN-only (non-hardware) path.
  Future<bool> hasPinEncryptedShard() async {
    final blob = await SecureStorage.get(SecureStorage.keyPinEncryptedShard);
    return blob != null && blob.isNotEmpty;
  }

  /// 按需加载设备分片到 Rust 内存（签名前调用）
  /// Public so MpcSessionManager can call it during sign recovery.
  ///
  /// If the shard was stored under the PIN-only path, [pinProvider] is invoked
  /// to obtain the PIN for decryption; otherwise the hardware path is used
  /// (which prompts for biometric/device-credential natively).
  Future<void> ensureShardLoaded({Future<String?> Function()? pinProvider}) async {
    final pubKeyHex = await SecureStorage.get('mpc_public_key');
    if (pubKeyHex == null || pubKeyHex.isEmpty) {
      throw MpcException('Public key not found');
    }
    final publicKey = List<int>.generate(
      pubKeyHex.length ~/ 2,
      (i) => int.parse(pubKeyHex.substring(i * 2, i * 2 + 2), radix: 16),
    );

    // PIN-only path: decrypt the app-layer blob with the user's PIN (no biometric).
    final pinBlob = await SecureStorage.get(SecureStorage.keyPinEncryptedShard);
    if (pinBlob != null && pinBlob.isNotEmpty) {
      final pin = pinProvider != null
          ? await pinProvider()
          : await SecureStorage.get('wallet_pin');
      if (pin == null || pin.isEmpty) {
        throw MpcException('PIN required to unlock device shard');
      }
      final ok = await MpcBridge.importDeviceShardEncrypted(
        encryptedData: pinBlob,
        pin: pin,
        publicKey: publicKey,
      );
      if (!ok) {
        throw MpcException('Failed to decrypt device shard (wrong PIN?)');
      }
      return;
    }

    // Hardware path (biometric / device credential).
    final shardBytes = await SecureHardware.loadDeviceShard();
    if (shardBytes == null || shardBytes.isEmpty) {
      throw MpcException('Device shard not found in secure hardware');
    }
    await MpcBridge.importDeviceShard(
      shardBytes: shardBytes.toList(),
      publicKey: publicKey,
    );
  }

  /// 执行分布式签名协议 (2-party ECDSA, 私钥从未被重组)
  /// [msgHash] 32字节消息哈希
  /// [walletId] 可选，指定使用哪个钱包的密钥分片签名
  /// Build the MPC sign Round 1 payload as structured JSON bytes.
  ///
  /// Shape (matches backend `extract_signing_request`):
  /// `{ "r0": "0x..", "msg_hash": [..32 bytes..], "tx": {..Eip1559Fields..} }`.
  /// `r0` is the serialized client SignRound1Message; `msg_hash` is the
  /// device-computed digest (server cross-checks, never trusts); `tx` is the
  /// authoritative transaction the server recomputes the hash from.
  static List<int> buildSignRound1Payload(
    List<int> r0,
    List<int> msgHash,
    SignTxFields? txFields,
  ) {
    final map = <String, dynamic>{
      'r0': '0x${hex.encode(r0)}',
      'msg_hash': msgHash,
    };
    if (txFields != null) {
      map['tx'] = txFields.toJson();
    }
    return utf8.encode(jsonEncode(map));
  }

  Future<List<int>> runSign(List<int> msgHash, {String? walletId, SignTxFields? txFields}) async {
    if (msgHash.length != 32) {
      throw MpcException('Message hash must be exactly 32 bytes');
    }

    // Signal immediately so presign loop can bail out
    _signInProgress = true;

    // Wait for any in-progress operation to finish
    while (_operationLock != null) {
      await _operationLock!.future;
    }
    _operationLock = Completer<void>();

    try {
      return await _runSignInternal(msgHash, walletId: walletId, txFields: txFields);
    } finally {
      _signInProgress = false;
      final lock = _operationLock;
      _operationLock = null;
      lock?.complete();
    }
  }

  Future<List<int>> _runSignInternal(List<int> msgHash, {String? walletId, SignTxFields? txFields}) async {
    final timer = StepTimer('MpcSign');
    await ensureShardLoaded();
    timer.mark('ensureShardLoaded');

    final sessionResult = await MpcApi.createSession(
      sessionType: 'sign',
      parties: [_deviceParty, _serverParty],
      threshold: 2,
      walletId: walletId,
    );
    timer.mark('createSession');

    if (!sessionResult.isSuccess || sessionResult.data == null) {
      throw MpcException('Failed to create sign session: ${sessionResult.errorMessage}');
    }

    final remoteSessionId = sessionResult.data!['session_id'] as String;
    _currentSessionId = remoteSessionId;
    _currentHmacKey = sessionResult.data!['hmac_key'] as String?;
    _lastMessageId = 0;

    final ws = MpcWebSocket(sessionId: remoteSessionId, partyIndex: _deviceParty);
    try {
      await ws.connect();
      timer.mark('wsConnect');

      final round1 = await MpcBridge.signGenerateRound1(msgHash);
      final localSessionId = round1.sessionId;
      timer.mark('localRound1Gen');

      // Save initial session state for recovery
      await MpcSessionStore.saveSession(MpcSessionState(
        sessionId: localSessionId,
        remoteSessionId: remoteSessionId,
        sessionType: 'sign',
        currentRound: 0,
        createdAt: DateTime.now(),
        metadata: {'msg_hash': msgHash},
      ));

      // Send Round 1: structured JSON {r0, msg_hash, tx{...}} so the server
      // can independently recompute the signing hash and enforce policy.
      final round1Json = buildSignRound1Payload(round1.payload, msgHash, txFields);
      await _sendToServer(
        sessionId: remoteSessionId,
        round: 1,
        payload: round1Json,
      );
      timer.mark('sendRound1');

      await MpcSessionStore.updateCurrentRound(1);

      // Wait for server's Round 1 (R_1) — server tags its R1 reply round=1
      final serverR1 = await _waitForMessages(ws, expectedCount: 1, expectedRound: 1);
      final serverR1Payload = serverR1.first.payload;
      timer.mark('waitServerR1');

      // Sync _lastMessageId so Round 2 fallback skips Round 1 messages
      await _syncLastMessageId(remoteSessionId);

      // Process R_1 and generate Round 2
      final round2Payload = await MpcBridge.signProcessRound1AndGenerateRound2(
        localSessionId,
        serverR1Payload,
      );
      timer.mark('processR1+genR2');

      // Send DeviceContribution via HTTP (reliable delivery)
      await _sendToServer(
        sessionId: remoteSessionId,
        round: 2,
        payload: round2Payload,
      );
      timer.mark('sendRound2');

      await MpcSessionStore.updateCurrentRound(2);

      // Wait for server's signature — server tags ServerSignature round=3 (round+1)
      final serverR2 = await _waitForMessages(ws, expectedCount: 1, expectedRound: 3);
      final serverR2Payload = serverR2.first.payload;
      timer.mark('waitServerSig');

      final signature = await MpcBridge.signProcessRound2(
        localSessionId,
        serverR2Payload,
      );
      timer.mark('processR2+finalize');

      if (signature.length != 65) {
        throw MpcException('Invalid signature length: ${signature.length}');
      }

      // Clear session state on success
      await MpcSessionStore.clearSession();

      // Record key usage for health tracking
      final health = KeyHealthService();
      health.recordPhoneKeyUsage();
      health.recordServerKeyUsage();

      return signature;
    } catch (e) {
      print('[MpcWalletService] Sign error: $e');
      throw MpcSessionInterruptedException(
        'Sign session interrupted: $e',
        sessionState: await MpcSessionStore.loadSession(),
      );
    } finally {
      await ws.disconnect();
      timer.done();
    }
  }

  /// 执行密钥轮转协议 (Reshare)
  /// 刷新密钥分片，旧分片失效，公钥不变
  /// [walletId] 可选，指定要轮转的钱包
  ///
  /// Failure recovery strategy:
  /// - Before finalize: old shard still valid (in hardware + server), safe to retry
  /// - After finalize but before hardware persist: new shard in Rust memory,
  ///   export immediately to hardware. If hardware write fails, new shard is lost
  ///   on restart → use backup shard recovery.
  /// - After hardware persist: device done; backup refresh is best-effort
  Future<WalletInfo> runReshare({String? walletId}) async {
    // Ensure device shard is loaded into Rust memory
    final shardBytes = await SecureHardware.loadDeviceShard();
    if (shardBytes == null || shardBytes.isEmpty) {
      throw MpcException('Device shard not found in secure storage');
    }
    final pubKeyHex = await SecureStorage.get('mpc_public_key');
    final pubKey = pubKeyHex != null && pubKeyHex.isNotEmpty
        ? List<int>.generate(pubKeyHex.length ~/ 2,
            (i) => int.parse(pubKeyHex.substring(i * 2, i * 2 + 2), radix: 16))
        : <int>[];
    await MpcBridge.importDeviceShard(
      shardBytes: shardBytes.toList(),
      publicKey: pubKey,
    );

    final sessionResult = await MpcApi.createSession(
      sessionType: 'reshare',
      parties: [_deviceParty, _serverParty],
      threshold: 2,
      walletId: walletId,
    );

    if (!sessionResult.isSuccess || sessionResult.data == null) {
      throw MpcException('Failed to create reshare session: ${sessionResult.errorMessage}');
    }

    final remoteSessionId = sessionResult.data!['session_id'] as String;
    _currentSessionId = remoteSessionId;
    _currentHmacKey = sessionResult.data!['hmac_key'] as String?;

    final ws = MpcWebSocket(sessionId: remoteSessionId, partyIndex: _deviceParty);
    try {
      await ws.connect();

      // Initialize local reshare session
      final localSessionId = await MpcBridge.reshareSessionNew(_deviceParty);

      // Save initial session state for recovery
      await MpcSessionStore.saveSession(MpcSessionState(
        sessionId: localSessionId,
        remoteSessionId: remoteSessionId,
        sessionType: 'reshare',
        currentRound: 0,
        createdAt: DateTime.now(),
      ));

      // Generate Round 1 (new polynomial evaluations)
      final round1Messages = await MpcBridge.reshareGenerateRound1(localSessionId);

      // Extract device's new backup contribution before sending messages
      final deviceBackupContrib = await MpcBridge.reshareDeriveBackupShare(localSessionId);

      // Send evaluations addressed to server via WebSocket
      for (final msgJson in round1Messages) {
        final msg = jsonDecode(msgJson) as Map<String, dynamic>;
        final to = msg['to'] as int;
        if (to == _serverParty) {
          final rawPayload = List<int>.from(msg['payload'] as List);
          ws.sendRaw(
            toParty: _serverParty,
            round: 1,
            payload: rawPayload,
          );
        }
      }

      await MpcSessionStore.updateCurrentRound(1);

      // Wait for server's reshare Round 1 messages (its evaluations for us)
      final serverMessages = await _waitForMessages(ws, expectedCount: 1, expectedRound: 1);
      final serverMsgsJson = serverMessages.map((m) {
        return jsonEncode({
          'session_id': remoteSessionId,
          'from': m.fromParty,
          'to': m.toParty,
          'round': m.round,
          'payload': m.payload,
        });
      }).toList();

      // Process server's evaluations and compute new share
      await MpcBridge.reshareProcessRound1(localSessionId, serverMsgsJson);

      // === POINT OF NO RETURN ===
      // After finalize, Rust memory holds the new shard and old is gone.
      // Must persist to hardware immediately.
      final walletInfo = await MpcBridge.reshareFinalize(localSessionId);

      // Persist new device shard to hardware IMMEDIATELY after finalize
      final newShardBytes = await MpcBridge.exportDeviceShard();
      await SecureHardware.storeDeviceShard(Uint8List.fromList(newShardBytes));

      // Update stored address and public key
      await SecureStorage.save('mpc_address', walletInfo.address);
      final pubKeyHex = walletInfo.publicKey
          .map((b) => b.toRadixString(16).padLeft(2, '0'))
          .join();
      await SecureStorage.save('mpc_public_key', pubKeyHex);
      await SecureStorage.save('last_key_rotation', DateTime.now().toIso8601String());

      // Re-derive backup shard (best-effort — device+server are already safe)
      await _refreshBackupShard(remoteSessionId, deviceBackupContrib);

      // Clear session state on success
      await MpcSessionStore.clearSession();

      return walletInfo;
    } catch (e) {
      print('[MpcWalletService] Reshare error: $e');
      throw MpcSessionInterruptedException(
        'Reshare session interrupted: $e',
        sessionState: await MpcSessionStore.loadSession(),
      );
    } finally {
      await ws.disconnect();
    }
  }

  /// 执行预签名协议 (Presign)
  /// 预计算签名材料，后续签名可瞬间完成
  /// [walletId] 钱包ID
  /// [count] 要生成的预签名数量
  Future<int> runPresign({required String walletId, int count = 5}) async {
    // Skip if a sign operation is in progress — presign can retry later
    if (_signInProgress) {
      print('[MpcWalletService] Skipping presign: sign operation in progress');
      return 0;
    }

    // Acquire operation lock to prevent sign from starting mid-presign
    while (_operationLock != null) {
      await _operationLock!.future;
    }
    _operationLock = Completer<void>();

    try {
      return await _runPresignInternal(walletId: walletId, count: count);
    } finally {
      final lock = _operationLock;
      _operationLock = null;
      lock?.complete();
    }
  }

  Future<int> _runPresignInternal({required String walletId, int count = 5}) async {
    int generated = 0;

    for (int i = 0; i < count; i++) {
      // Bail out if a sign operation is waiting
      if (_signInProgress) {
        print('[MpcWalletService] Aborting presign batch: sign operation pending');
        break;
      }

      final sessionResult = await MpcApi.createSession(
        sessionType: 'presign',
        parties: [_deviceParty, _serverParty],
        threshold: 2,
        walletId: walletId,
      );

      if (!sessionResult.isSuccess || sessionResult.data == null) {
        break;
      }

      final remoteSessionId = sessionResult.data!['session_id'] as String;

      final ws = MpcWebSocket(sessionId: remoteSessionId, partyIndex: _deviceParty);
      try {
        await ws.connect();

        // Generate presign Round 1
        final round1 = await MpcBridge.presignGenerateRound1();
        final localSessionId = round1.sessionId;

        // Send Round 1 to server
        ws.sendRaw(toParty: _serverParty, round: 1, payload: round1.payload);

        // Wait for server's Round 1 — server tags its R1 reply round=1
        final serverR1 = await _waitForMessages(ws, expectedCount: 1, expectedRound: 1);

        // Process and generate Round 2
        final round2Payload = await MpcBridge.presignProcessRound1AndGenerateRound2(
          localSessionId,
          serverR1.first.payload,
        );

        // Send Round 2
        ws.sendRaw(toParty: _serverParty, round: 2, payload: round2Payload);

        // Finalize presignature
        await MpcBridge.presignFinalize(localSessionId);
        generated++;
      } finally {
        await ws.disconnect();
      }
    }

    return generated;
  }

  /// Re-derive backup shard after reshare using new polynomial contributions.
  /// Fetches server's new g_server(3), combines with device's g_device(3).
  /// Does NOT auto-store — saves to memory for UI to prompt user to choose backup method.
  Future<void> _refreshBackupShard(String remoteSessionId, List<int> deviceContribution) async {
    try {
      if (deviceContribution.length != 32) {
        print('[MpcWalletService] Invalid device backup contribution length after reshare');
        return;
      }

      final serverResult = await MpcApi.getBackupContribution(remoteSessionId);
      if (!serverResult.isSuccess || serverResult.data == null) {
        print('[MpcWalletService] Failed to fetch server reshare backup contribution');
        return;
      }

      final serverContribution = serverResult.data!;
      if (serverContribution.length != 32) {
        print('[MpcWalletService] Invalid server backup contribution length after reshare');
        return;
      }

      final newBackupShard = await MpcBridge.combineBackupShares(
        deviceShare: deviceContribution,
        serverShare: serverContribution,
      );

      if (newBackupShard.length != 32) {
        print('[MpcWalletService] Invalid combined backup shard after reshare');
        return;
      }

      _lastBackupShard = newBackupShard;

      // Persist the refreshed backup shard according to the method the user
      // already chose. Cloud backups can be overwritten automatically; file /
      // encrypted-file backups require the user to re-export (password + save
      // location), so we only flag that and let the UI prompt.
      final method = await Services.backup.getBackupMethod();
      if (method == BackupMethod.cloud) {
        try {
          await Services.backup.storeBackupShard(newBackupShard, useCloud: true);
          _backupNeedsReExport = false;
          print('[MpcWalletService] Cloud backup updated with refreshed shard');
        } catch (e) {
          // Cloud overwrite failed — fall back to prompting a manual re-export.
          _backupNeedsReExport = true;
          print('[MpcWalletService] Cloud backup update failed, needs manual re-export: $e');
        }
      } else {
        // file / encrypted_file / never-backed-up: user must re-export manually.
        // Stage the shard durably so that killing/backgrounding the app during
        // the mandatory re-export does not lose it (in-memory _lastBackupShard
        // would be gone on relaunch → unrecoverable backup). Startup re-routes
        // to the mandatory backup screen while the flag is set.
        _backupNeedsReExport = true;
        await SecureStorage.save(
            SecureStorage.keyRotationPendingShard, base64Encode(newBackupShard));
        await SecureStorage.save(SecureStorage.keyRotationPendingCreatedAt,
            DateTime.now().toIso8601String());
        await SecureStorage.save(SecureStorage.keyBackupReExportPending, '1');
        print('[MpcWalletService] New backup shard staged, awaiting user re-export');
      }
    } catch (e) {
      print('[MpcWalletService] Failed to refresh backup shard after reshare: $e');
    }
  }

  /// 提取并存储备份分片
  /// 计算完整备份分片 (f_device(3) + f_server(3))
  /// 返回 32 字节标量，不自动存储。UI 层负责让用户选择存储方式。
  Future<List<int>> _deriveBackupShard(String localSessionId) async {
    // Step 1: Compute device's contribution to backup shard (f_device(3))
    final deviceContribution = await MpcBridge.dkgDeriveBackupShare(
      localSessionId,
      backupPartyIndex: _backupParty,
    );

    if (deviceContribution.length != 32) {
      throw MpcException(
        'Invalid device backup contribution length: ${deviceContribution.length} bytes (expected 32)'
      );
    }

    // Step 2: Fetch server's contribution (f_server(3)) from API
    if (_currentSessionId == null) {
      throw MpcException('No active session ID for fetching server backup contribution');
    }

    final serverResult = await MpcApi.getBackupContribution(_currentSessionId!);
    if (!serverResult.isSuccess || serverResult.data == null) {
      throw MpcException(
        'Failed to fetch server backup contribution: ${serverResult.errorMessage}'
      );
    }

    final serverContribution = serverResult.data!;
    if (serverContribution.length != 32) {
      throw MpcException(
        'Invalid server backup contribution length: ${serverContribution.length} bytes (expected 32)'
      );
    }

    // Step 3: Combine both contributions via modular scalar addition
    final combinedBackupShard = await MpcBridge.combineBackupShares(
      deviceShare: deviceContribution,
      serverShare: serverContribution,
    );

    if (combinedBackupShard.length != 32) {
      throw MpcException(
        'Invalid combined backup shard length: ${combinedBackupShard.length} bytes (expected 32)'
      );
    }

    return combinedBackupShard;
  }

  /// 获取 DKG 后计算好的备份分片（内存中，未存储）
  /// UI 层应调用此方法获取数据，然后让用户选择存储方式
  List<int>? get lastBackupShard => _lastBackupShard;

  /// 轮换后备份分片需要重新导出（离线文件方式）
  bool get backupNeedsReExport => _backupNeedsReExport;

  /// Whether a post-rotation backup re-export is still pending, checked durably.
  /// Survives app kill/relaunch (the in-memory flag is only a fast path). Used
  /// by startup routing to force the user back to the mandatory backup screen.
  Future<bool> isBackupReExportPending() async {
    if (_backupNeedsReExport) return true;
    final flag = await SecureStorage.get(SecureStorage.keyBackupReExportPending);
    return flag == '1';
  }

  /// Load the staged (refreshed) backup shard persisted during reshare, so the
  /// mandatory export screen can recover it after an app relaunch even though
  /// [_lastBackupShard] was cleared. Returns null if none staged.
  Future<List<int>?> loadStagedBackupShard() async {
    if (_lastBackupShard != null && _lastBackupShard!.length == 32) {
      return _lastBackupShard;
    }
    final staged = await SecureStorage.get(SecureStorage.keyRotationPendingShard);
    if (staged == null || staged.isEmpty) return null;
    final bytes = base64Decode(staged);
    return bytes.length == 32 ? bytes : null;
  }

  /// Mark the post-rotation backup re-export as satisfied. Called by the
  /// mandatory export screen after the user has exported the refreshed shard
  /// via a path that bypasses [storeBackupShard] (e.g. encrypted-file export).
  /// Clears both the in-memory state and the durable staging slot.
  Future<void> markBackupReExported() async {
    _lastBackupShard = null;
    _backupNeedsReExport = false;
    await SecureStorage.delete(SecureStorage.keyBackupReExportPending);
    await SecureStorage.delete(SecureStorage.keyRotationPendingShard);
    await SecureStorage.delete(SecureStorage.keyRotationPendingCreatedAt);
  }

  /// 用户选择存储方式后调用此方法。
  Future<BackupResult> storeBackupShard(
    List<int> shardBytes, {
    required bool useCloud,
  }) async {
    final backupService = BackupShardService(PlatformCloudBackup());
    final addr = await getAddress();
    if (addr.isNotEmpty) {
      backupService.setWalletAddress(addr);
    }
    _lastBackupResult = await backupService.storeBackupShard(
      shardBytes,
      useCloud: useCloud,
    );
    _lastBackupShard = null;
    _backupNeedsReExport = false;
    return _lastBackupResult!;
  }

  /// 获取上次 DKG 的备份结果
  BackupResult? get lastBackupResult => _lastBackupResult;

  @override
  Future<List<int>> sign(List<int> msgHash, {SignTxFields? txFields, String? walletId}) async {
    return await runSign(msgHash, txFields: txFields, walletId: walletId);
  }

  @override
  Future<SignResult> signWithSession(List<int> msgHash, {SignTxFields? txFields, String? walletId}) async {
    final signature = await runSign(msgHash, txFields: txFields, walletId: walletId);
    return SignResult(signature: signature, sessionId: _currentSessionId);
  }

  @override
  Future<String> getAddress() async {
    final addr = await SecureStorage.get('mpc_address');
    if (addr == null || addr.isEmpty) {
      throw StateError('No MPC wallet found');
    }
    return addr;
  }

  @override
  Future<bool> hasWallet() async {
    final addr = await SecureStorage.get('mpc_address');
    return addr != null && addr.isNotEmpty;
  }

  @override
  Future<void> deleteWallet() async {
    await SecureStorage.delete('mpc_address');
    await SecureStorage.delete('mpc_session_id');
    await SecureStorage.delete('mpc_key_share_0');
    await SecureStorage.delete('mpc_public_key');
    await SecureStorage.delete('mpc_chain_code');
  }

  /// 通过 WebSocket 流等待指定数量的服务器消息
  ///
  /// [expectedRound] 若指定，仅接受该 round 的消息。WebSocket 重连后服务器会
  /// 从头重放历史消息（ORDER BY round ASC），不按 round 过滤会让等待 Round 2
  /// 的调用拿到被重放的 Round 1 消息，导致 bincode 反序列化把 SignRound1Message
  /// 的 session_id 长度前缀（UUID = 36）误读成枚举变体下标。
  Future<List<MpcMessage>> _waitForMessages(
    MpcWebSocket ws, {
    required int expectedCount,
    int? expectedRound,
  }) async {
    final messages = <MpcMessage>[];
    final completer = Completer<List<MpcMessage>>();

    final subscription = ws.messages.listen((msg) {
      if (msg.fromParty == _serverParty &&
          (expectedRound == null || msg.round == expectedRound)) {
        // Delivered live — drop the buffered copy so it isn't double-counted.
        ws.dropBuffered(msg);
        messages.add(msg);
        if (messages.length >= expectedCount && !completer.isCompleted) {
          completer.complete(messages);
        }
      }
    });

    // Drain messages that arrived before this listener was attached. The server
    // pushes its reply over the socket within ~1ms of our HTTP send, usually
    // before `.listen()` above runs; the broadcast stream drops those, so we
    // recover them from the socket's buffer here. Without this the wait always
    // times out (45s) and limps along on the HTTP poll fallback.
    for (final buffered in ws.takeBuffered(fromParty: _serverParty, round: expectedRound)) {
      messages.add(buffered);
    }
    if (messages.length >= expectedCount && !completer.isCompleted) {
      completer.complete(messages);
    }

    // Fallback timeout with HTTP polling
    final timer = Timer(_wsTimeout, () {
      if (!completer.isCompleted) {
        subscription.cancel();
        completer.completeError(
          MpcException('Timeout waiting for server response via WebSocket'),
        );
      }
    });

    try {
      final result = await completer.future;
      timer.cancel();
      await subscription.cancel();
      return result;
    } catch (e) {
      timer.cancel();
      await subscription.cancel();

      // Fallback to HTTP polling if WebSocket failed
      if (e is MpcException && _currentSessionId != null) {
        return await _pollMessagesFallback(
          sessionId: _currentSessionId!,
          party: _deviceParty,
          expectedCount: expectedCount,
          afterId: _lastMessageId,
          expectedRound: expectedRound,
        );
      }
      rethrow;
    }
  }

  /// Sync _lastMessageId by querying current max message ID from server.
  Future<void> _syncLastMessageId(String sessionId) async {
    try {
      final result = await MpcApi.receiveMessages(
        sessionId,
        party: _deviceParty,
        afterId: 0,
      );
      if (result.isSuccess && result.data != null) {
        for (final raw in result.data!) {
          final m = Map<String, dynamic>.from(raw as Map);
          final id = m['id'] as int;
          if (id > _lastMessageId) _lastMessageId = id;
        }
      }
    } catch (_) {}
  }

  /// HTTP 轮询回退（WebSocket 不可用时）
  Future<List<MpcMessage>> _pollMessagesFallback({
    required String sessionId,
    required int party,
    required int expectedCount,
    int afterId = 0,
    int? expectedRound,
  }) async {
    const pollInterval = Duration(seconds: 1);
    const pollTimeout = Duration(seconds: 30);
    const statusCheckInterval = 5;
    final deadline = DateTime.now().add(pollTimeout);
    List<MpcMessage> allMessages = [];
    int lastId = afterId;
    int pollCount = 0;

    while (DateTime.now().isBefore(deadline)) {
      final result = await MpcApi.receiveMessages(
        sessionId,
        party: party,
        afterId: lastId,
      );

      if (result.isSuccess && result.data != null) {
        for (final raw in result.data!) {
          final m = Map<String, dynamic>.from(raw as Map);
          if (m['from_party'] == _serverParty &&
              (expectedRound == null || m['round'] == expectedRound)) {
            allMessages.add(MpcMessage(
              fromParty: m['from_party'] as int,
              toParty: m['to_party'] as int,
              round: m['round'] as int,
              payload: (m['payload'] as List<dynamic>).cast<int>(),
            ));
            final id = m['id'] as int;
            if (id > lastId) lastId = id;
            if (id > _lastMessageId) _lastMessageId = id;
          }
        }

        if (allMessages.length >= expectedCount) {
          return allMessages;
        }
      }

      pollCount++;
      // Periodically check session status to detect server-side failures
      if (pollCount % statusCheckInterval == 0) {
        final statusResult = await MpcApi.getSession(sessionId);
        if (statusResult.isSuccess && statusResult.data != null) {
          final status = statusResult.data!['status'] as String;
          if (status == 'failed' || status == 'expired') {
            throw MpcException(
              'MPC session $status on server (round processing failed)',
            );
          }
        }
      }

      await Future.delayed(pollInterval);
    }

    if (allMessages.isEmpty) {
      throw MpcException('Timeout waiting for server response (HTTP fallback)');
    }
    return allMessages;
  }

  /// 获取当前会话ID
  String? get currentSessionId => _currentSessionId;

  /// Wrap raw server payload bytes into a ProtocolMessage JSON string
  /// that the Rust FFI expects.
  String _wrapAsProtocolMessage(String sessionId, MpcMessage msg) {
    return jsonEncode({
      'session_id': sessionId,
      'from': msg.fromParty,
      'to': msg.toParty,
      'round': msg.round,
      'payload': msg.payload,
    });
  }

  /// Send a message to the server party, authenticated with the per-session
  /// HMAC key (F-004). Server rejects server-bound messages without a valid
  /// HMAC over (session_id ‖ round ‖ payload).
  Future<void> _sendToServer({
    required String sessionId,
    required int round,
    required List<int> payload,
  }) async {
    String? hmac;
    final key = _currentHmacKey;
    if (key != null && key.isNotEmpty) {
      hmac = MpcHmac.compute(
        sessionHmacKeyHex: key,
        sessionId: sessionId,
        round: round,
        payload: payload,
      );
    }
    await MpcApi.sendMessage(
      sessionId: sessionId,
      fromParty: _deviceParty,
      toParty: _serverParty,
      round: round,
      payload: payload,
      hmac: hmac,
    );
  }
}
