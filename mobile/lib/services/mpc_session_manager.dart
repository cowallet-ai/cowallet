import 'package:flutter/foundation.dart';
import 'dart:async';
import '../api/mpc_api.dart';
import '../bridge/mpc_bridge.dart';
import '../network/mpc_websocket.dart';
import 'mpc_session_store.dart';
import 'mpc_wallet_service.dart';
import 'wallet_service.dart';

/// Manages MPC session recovery and resumption after interruptions.
/// Wraps MpcWalletService with automatic session persistence and recovery.
class MpcSessionManager {
  final MpcWalletService _mpcService;

  MpcSessionManager(this._mpcService);

  /// Check if there's a session that can be resumed.
  Future<bool> canResume() async {
    return await MpcSessionStore.hasActiveSession();
  }

  /// Attempt to resume an interrupted session.
  /// Returns session info if resumable, null if session is stale/failed.
  Future<MpcSessionState?> checkResumableSession() async {
    final session = await MpcSessionStore.loadSession();
    if (session == null) return null;

    // Check backend session status
    try {
      final result = await MpcApi.getSession(session.remoteSessionId);
      if (!result.isSuccess || result.data == null) {
        // Session not found on backend, clear local state
        await MpcSessionStore.clearSession();
        return null;
      }

      final status = result.data!['status'] as String;
      final backendRound = result.data!['current_round'] as int;

      // Only resume if session is still active or interrupted
      if (status == 'active' || status == 'interrupted') {
        // Update local round if backend is ahead
        if (backendRound > session.currentRound) {
          await MpcSessionStore.updateCurrentRound(backendRound);
          return session.copyWith(currentRound: backendRound);
        }
        return session;
      } else {
        // Session failed/completed/expired on backend, clear local state
        await MpcSessionStore.clearSession();
        return null;
      }
    } catch (e) {
      debugPrint('[MpcSessionManager] Error checking session: $e');
      // Network error, keep local state for now
      return session;
    }
  }

  /// Run DKG with automatic session persistence and recovery.
  Future<WalletInfo> runDkgWithRecovery({String? walletId}) async {
    // Check for existing session
    final existing = await checkResumableSession();
    if (existing != null && existing.sessionType == 'keygen') {
      debugPrint('[MpcSessionManager] Attempting to resume DKG session ${existing.remoteSessionId}');
      try {
        final result = await _resumeDkg(existing);
        await MpcSessionStore.clearSession();
        return result;
      } catch (e) {
        debugPrint('[MpcSessionManager] Resume failed: $e, starting fresh');
        await MpcSessionStore.clearSession();
      }
    }

    // Start new session with persistence
    return await _runDkgWithPersistence(walletId: walletId);
  }

  /// Run Sign with automatic session persistence and recovery.
  Future<List<int>> runSignWithRecovery(List<int> msgHash, {String? walletId, SignTxFields? txFields}) async {
    // Check for existing session
    final existing = await checkResumableSession();
    if (existing != null && existing.sessionType == 'sign') {
      debugPrint('[MpcSessionManager] Attempting to resume Sign session ${existing.remoteSessionId}');
      try {
        final result = await _resumeSign(existing, msgHash, txFields: txFields);
        await MpcSessionStore.clearSession();
        return result;
      } catch (e) {
        debugPrint('[MpcSessionManager] Resume failed: $e, starting fresh');
        await MpcSessionStore.clearSession();
      }
    }

    // Start new session with persistence
    return await _runSignWithPersistence(msgHash, walletId: walletId, txFields: txFields);
  }

  /// Run Reshare with automatic session persistence and recovery.
  Future<WalletInfo> runReshareWithRecovery({String? walletId}) async {
    // Check for existing session
    final existing = await checkResumableSession();
    if (existing != null && existing.sessionType == 'reshare') {
      debugPrint('[MpcSessionManager] Attempting to resume Reshare session ${existing.remoteSessionId}');
      try {
        final result = await _resumeReshare(existing);
        await MpcSessionStore.clearSession();
        return result;
      } catch (e) {
        debugPrint('[MpcSessionManager] Resume failed: $e, starting fresh');
        await MpcSessionStore.clearSession();
      }
    }

    // Start new session with persistence
    return await _runReshareWithPersistence(walletId: walletId);
  }

  // ==================== DKG with Persistence ====================

  Future<WalletInfo> _runDkgWithPersistence({String? walletId}) async {
    // Delegate to original service implementation with wrapped error handling
    return await _mpcService.runDkg(walletId: walletId);
  }

  Future<WalletInfo> _resumeDkg(MpcSessionState session) async {
    // For DKG, we can't truly resume the Rust crypto state.
    // If the session was interrupted, the best we can do is restart.
    // However, we can check if the backend session completed while we were offline.
    final result = await MpcApi.getSession(session.remoteSessionId);
    if (!result.isSuccess || result.data == null) {
      throw MpcSessionInterruptedException('DKG session not found on backend');
    }

    final status = result.data!['status'] as String;
    if (status == 'completed') {
      // Backend completed the session, but we need the wallet info.
      // This is a rare edge case - for now, throw to start fresh.
      throw MpcSessionInterruptedException('DKG session completed on backend but local state lost');
    }

    throw MpcSessionInterruptedException('DKG cannot be resumed, restart required');
  }

  // ==================== Sign with Persistence ====================

  Future<List<int>> _runSignWithPersistence(List<int> msgHash, {String? walletId, SignTxFields? txFields}) async {
    // Delegate to original service implementation
    return await _mpcService.runSign(msgHash, walletId: walletId, txFields: txFields);
  }

  /// Resume an interrupted sign session.
  ///
  /// The resume endpoint on the backend clears stale protocol messages and
  /// re-initializes the server's crypto state. The client then runs the
  /// normal sign flow against the same session ID (which has been reactivated
  /// with a fresh expiry window). This avoids the complexity of mid-round
  /// state reconstruction since the Rust crypto state is ephemeral.
  Future<List<int>> _resumeSign(MpcSessionState session, List<int> msgHash, {SignTxFields? txFields}) async {
    // Call the resume endpoint to reactivate and reset the session
    final resumeResult = await MpcApi.resumeSession(session.remoteSessionId);
    if (!resumeResult.isSuccess || resumeResult.data == null) {
      throw MpcSessionInterruptedException(
        'Failed to resume session on backend: ${resumeResult.errorMessage}',
      );
    }

    // Session is now reactivated with fresh state on the server.
    // Run the sign protocol from scratch using the same session ID.
    await _mpcService.ensureShardLoaded();

    final round1 = await MpcBridge.signGenerateRound1(msgHash);
    final localSessionId = round1.sessionId;

    final ws = MpcWebSocket(
      sessionId: session.remoteSessionId,
      partyIndex: 0,
    );

    try {
      await ws.connect();

      // Send Round 1: structured JSON {r0, msg_hash, tx{...}} matching the
      // backend signing gate (see MpcWalletService._buildSignRound1Payload).
      final round1Json = MpcWalletService.buildSignRound1Payload(
        round1.payload,
        msgHash,
        txFields,
      );
      ws.sendRaw(toParty: 1, round: 1, payload: round1Json);

      await MpcSessionStore.updateCurrentRound(1);

      // Wait for server's Round 1 (R_1) — server tags its R1 reply round=1
      final serverR1 = await _waitForServerMessages(ws, expectedCount: 1, expectedRound: 1);

      // Process R_1 and generate Round 2
      final round2Payload = await MpcBridge.signProcessRound1AndGenerateRound2(
        localSessionId,
        serverR1.first.payload,
      );

      // Send DeviceContribution
      ws.sendRaw(toParty: 1, round: 2, payload: round2Payload);

      await MpcSessionStore.updateCurrentRound(2);

      // Wait for server's signature response — server tags ServerSignature round=3
      final serverR2 = await _waitForServerMessages(ws, expectedCount: 1, expectedRound: 3);

      final signature = await MpcBridge.signProcessRound2(
        localSessionId,
        serverR2.first.payload,
      );

      if (signature.length != 65) {
        throw MpcSessionInterruptedException(
          'Invalid signature length after recovery: ${signature.length}',
        );
      }

      return signature;
    } finally {
      await ws.disconnect();
    }
  }

  // ==================== Reshare with Persistence ====================

  Future<WalletInfo> _runReshareWithPersistence({String? walletId}) async {
    // Delegate to original service implementation
    return await _mpcService.runReshare(walletId: walletId);
  }

  Future<WalletInfo> _resumeReshare(MpcSessionState session) async {
    // Reshare protocol cannot be resumed from interruption.
    throw MpcSessionInterruptedException('Reshare session cannot be resumed, restart required');
  }

  // ==================== Helpers ====================

  /// Wait for server messages via WebSocket with timeout.
  ///
  /// [expectedRound] 若指定，仅接受该 round 的消息。WS 重连后服务器会按
  /// round 升序重放历史消息，不过滤会让等待 Round 2 的调用拿到被重放的
  /// Round 1，导致 bincode 把 SignRound1Message 误读为 SignRound2Message。
  Future<List<MpcMessage>> _waitForServerMessages(
    MpcWebSocket ws, {
    required int expectedCount,
    int? expectedRound,
  }) async {
    final messages = <MpcMessage>[];
    final completer = Completer<List<MpcMessage>>();

    final subscription = ws.messages.listen((msg) {
      if (msg.fromParty == 1 &&
          (expectedRound == null || msg.round == expectedRound)) {
        messages.add(msg);
        if (messages.length >= expectedCount && !completer.isCompleted) {
          completer.complete(messages);
        }
      }
    });

    try {
      return await completer.future.timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          throw MpcSessionInterruptedException(
            'Timeout waiting for server response during recovery',
          );
        },
      );
    } finally {
      await subscription.cancel();
    }
  }

}
