import '../network/dio_client.dart';
import '../network/result.dart';

/// Swap API — calls backend Bridgers cross-chain swap endpoints.
///
/// POST /swap/quote   — Get swap price quote
/// POST /swap/build   — Build swap transaction calldata
/// GET  /swap/tokens  — Get supported token list
/// POST /swap/order   — Upload tx hash for cross-chain order tracking
/// GET  /swap/order/:id — Get order status
class SwapApi {
  static Future<Result<Map<String, dynamic>>> getQuote({
    required int fromChainId,
    required String sellToken,
    required String buyToken,
    required String sellAmount,
    int? toChainId,
    String? takerAddress,
  }) async {
    return await DioClient.post(
      "/swap/quote",
      data: {
        "from_chain_id": fromChainId,
        "to_chain_id": toChainId ?? fromChainId,
        "sell_token": sellToken,
        "buy_token": buyToken,
        "sell_amount": sellAmount,
        if (takerAddress != null) "taker_address": takerAddress,
      },
    );
  }

  static Future<Result<Map<String, dynamic>>> buildSwapTx({
    required int fromChainId,
    required String sellToken,
    required String buyToken,
    required String sellAmount,
    required String takerAddress,
    int? toChainId,
    String? toAddress,
    double slippage = 0.5,
  }) async {
    return await DioClient.post(
      "/swap/build",
      data: {
        "from_chain_id": fromChainId,
        "to_chain_id": toChainId ?? fromChainId,
        "sell_token": sellToken,
        "buy_token": buyToken,
        "sell_amount": sellAmount,
        "taker_address": takerAddress,
        if (toAddress != null) "to_address": toAddress,
        "slippage": slippage,
      },
    );
  }

  static Future<Result<List<dynamic>>> getTokens({String? chain}) async {
    return await DioClient.get(
      "/swap/tokens",
      queryParameters: {if (chain != null) "chain": chain},
    );
  }

  static Future<Result<Map<String, dynamic>>> uploadOrderHash({
    required String hash,
    required int fromChainId,
    required int toChainId,
    required String sellToken,
    required String buyToken,
    required String sellAmount,
    required String buyAmountMin,
    required String fromAddress,
    required String toAddress,
  }) async {
    return await DioClient.post(
      "/swap/order",
      data: {
        "hash": hash,
        "from_chain_id": fromChainId,
        "to_chain_id": toChainId,
        "sell_token": sellToken,
        "buy_token": buyToken,
        "sell_amount": sellAmount,
        "buy_amount_min": buyAmountMin,
        "from_address": fromAddress,
        "to_address": toAddress,
      },
    );
  }

  static Future<Result<Map<String, dynamic>>> getOrderStatus({
    required String orderId,
  }) async {
    return await DioClient.get("/swap/order/$orderId");
  }

  static Future<Result<dynamic>> getHistory({
    required String fromAddress,
    int pageNo = 1,
    int pageSize = 10,
  }) async {
    return await DioClient.get(
      "/swap/history",
      queryParameters: {
        "from_address": fromAddress,
        "page_no": pageNo,
        "page_size": pageSize,
      },
    );
  }
}