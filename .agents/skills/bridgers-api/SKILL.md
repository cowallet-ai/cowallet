---
name: bridgers-api
description: Comprehensive Bridgers API integration for user crypto exchange/swaps. Supports quoting, token fetching, calldata generation, order tracking, and transaction status queries. Use this when you need to swap, exchange, or bridge tokens using Bridgers API.
---

# Bridgers API Skill

This skill allows the agent to interact with the Bridgers API (https://docs-bridgers.bridgers.xyz/) to execute cross-chain and same-chain token swaps.

## Features Supported

- **Get Supported Tokens:** Fetch the list of supported tokens and their respective chains.
- **Request Quote:** Get an expected amount out, fee details, and min/max boundaries.
- **Obtain CallData:** Generate the `calldata` needed for EVM smart contract execution to swap tokens.
- **Generate Order:** Report a successful user deposit transaction hash to start tracking.
- **Query Orders:** Track the status of a specific order or a user's transaction history.

## How to use

The script `scripts/bridgers.js` is an executable CLI you can use via the `exec` tool. All command parameters should be passed as a single JSON string.

### 1. Get Token List
Returns the available tokens across supported chains.

```bash
node scripts/bridgers.js getTokens
```

### 2. Request Quote
Get price estimation and minimum/maximum exchange amounts.

```bash
node scripts/bridgers.js quote '{"fromTokenAddress": "0x...", "toTokenAddress": "0x...", "fromTokenAmount": "...", "fromTokenChain": "BSC", "toTokenChain": "HECO", "userAddr": "0x...", "equipmentNo": "random123", "sourceFlag": "widget"}'
```

### 3. Get CallData
Generates the swap transaction parameters to broadcast on-chain. Required fields include token addresses, amounts, and user address.

```bash
node scripts/bridgers.js getCallData '{"fromTokenAddress": "0x...", "amountOutMin": "...", "equipmentNo": "random123", "toAddress": "0x...", "toTokenChain": "HECO", "fromTokenAmount": "...", "fromTokenChain": "BSC", "toTokenAddress": "0x...", "fromAddress": "0x...", "sourceFlag": "widget", "fromCoinCode": "...", "toCoinCode": "...", "slippage": "0.1"}'
```

### 4. Generate Order Information
Once the transaction hash is generated, report it to Bridgers to start processing.

```bash
node scripts/bridgers.js generateOrder '{"equipmentNo": "...", "sourceFlag": "...", "hash": "0x...", "fromTokenAddress": "0x...", "toTokenAddress": "0x...", "fromAddress": "0x...", "toAddress": "0x...", "fromTokenChain": "...", "toTokenChain": "...", "fromTokenAmount": "...", "amountOutMin": "...", "fromCoinCode": "...", "toCoinCode": "..."}'
```

### 5. Query Transaction Records (History)
Fetch all transactions for a user.

```bash
node scripts/bridgers.js queryRecords '{"equipmentNo": "...", "sourceFlag": "...", "pageNo": 1, "pageSize": 10, "fromAddress": "0x..."}'
```

### 6. Query Transaction Details (Specific Order)
Get detailed order status by orderId.

```bash
node scripts/bridgers.js queryDetails '{"orderId": "..."}'
```

## Important Notes
- Always check minimum and maximum deposit limits provided in the `quote` step.
- Tokens must be approved (`approve`) to the swap contract on EVM chains before invoking the `calldata`.
- Status fields: `receive_complete` indicates success.
- `equipmentNo` is treated as a unique device identifier and should be generated/passed consistently.
