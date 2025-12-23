# Deriverse Adapter Integration Guide for OKX Developers

## 1. Overview of Main Changes

Integration for the Deriverse DEX has been added via two primary components:

### 1.1. `deriverse-accounts` Crate
- **Location**: `programs/deriverse-accounts/`
- **Purpose**: This new crate provides helper functions to deterministically calculate the addresses of the many Program Derived Addresses (PDAs) required by the Deriverse protocol.
- **Key Function**: `get_remaning_pdas()` in `lib.rs` can be used to generate the list of necessary PDAs for a given instrument. The comments within this function (`offset + X`) show where each account should be placed in the final instruction accounts array.

### 1.2. `deriverse.rs` Adapter
- **Location**: `programs/dex-solana/src/adapters/deriverse.rs`
- **Purpose**: This is the core adapter logic. It takes the flat list of 28 accounts, constructs the correct `swap` instruction for the Deriverse on-chain program, and invokes it.
- **Functionality**: The adapter internally handles the logic of determining the swap direction (buy vs. sell), creating the instruction data, and preparing the `AccountMeta` list for the CPI call.

## 2. Deriverse Instruction Account Structure

The Deriverse swap instruction requires a fixed array of **28 accounts** passed in the `remaining_accounts` slice. The order is critical. The `parse_accounts` function in the adapter defines the definitive order.

Below is the complete, 0-indexed list of required accounts:

| Index | Account Name                 | Description |
| :---  | :---                         | :--- |
| 0     | `authority`                  | The authority account (user's wallet) signing the transaction. |
| 1     | `source_token_acc`           | The user's token account from which tokens are sent. |
| 2     | `destination_token_acc`      | The user's token account where tokens are received. |
| 3     | `drvs_vault_asset_token_acc` | The Deriverse program's vault account for the asset token. |
| 4     | `drvs_vault_crncy_token_acc` | The Deriverse program's vault account for the currency token. |
| 5     | `asset_mint`                 | The mint account for the asset token. |
| 6     | `crncy_mint`                 | The mint account for the currency token. |
| 7     | `root`                       | **PDA**: The main Root state account for the Deriverse program. |
| 8     | `instrument`                 | **PDA**: The Instrument account defining the specific trading pair. |
| 9     | `bids_tree`                  | **PDA**: The Bids order book tree account. |
| 10    | `asks_tree`                  | **PDA**: The Asks order book tree account. |
| 11    | `bid_orders`                 | **PDA**: The account storing the array of individual bid orders. |
| 12    | `ask_orders`                 | **PDA**: The account storing the array of individual ask orders. |
| 13    | `lines`                      | **PDA**: The account managing order book depth visualization. |
| 14    | `map`                        | **PDA**: The account for memory allocation management in the order book. |
| 15    | `spot_client_infos`          | **PDA**: The primary account for storing spot client information. |
| 16    | `spot_client_infos2`         | **PDA**: The secondary account for storing additional spot client information. |
| 17    | `candles_1m`                 | **PDA**: The account for 1-minute candle historical data. |
| 18    | `candles_15m`                | **PDA**: The account for 15-minute candle historical data. |
| 19    | `candles_day`                | **PDA**: The account for daily candle historical data. |
| 20    | `community`                  | **PDA**: The Community account for protocol-wide settings (e.g., fees). |
| 21    | `drvs_asset_token_state`     | **PDA**: Deriverse's internal state management account for the asset token. |
| 22    | `drvs_crncy_token_state`     | **PDA**: Deriverse's internal state management account for the currency token. |
| 23    | `drvs_auth`                  | **PDA**: The Deriverse program's authority, used for signing transfers from vaults. |
| 24    | `system_program`             | **Program**: The Solana System Program. |
| 25    | `asset_token_program`        | **Program**: The SPL Token Program that governs the asset token. |
| 26    | `crncy_token_program`        | **Program**: The SPL Token Program that governs the currency token. |
| 27    | `associated_token_program`   | **Program**: The SPL Associated Token Account Program. |

## 3. Key Concept: Fixed Asset/Currency Roles

A critical concept for integrating with Deriverse is that **the roles of "asset" and "currency" are rigidly fixed for every trading pair.**

For example, in a SOL-USDC pair, the Deriverse on-chain program might be configured so that USDC is *always* the `asset` and SOL is *always* the `currency`.

### Implications for Integration

- **The Account List is Always the Same:** Because the asset/currency roles are fixed, the list of 28 accounts for a given pair (e.g., SOL-USDC) is **always the same and in the same order**, regardless of the swap direction.
  - The `asset_mint` (Index 5) is always the SOL mint.
  - The `crncy_mint` (Index 6) is always the USDC mint.
  - The `drvs_vault_asset_token_acc` (Index 3) is always the vault for SOL.
  - ...and so on for all other asset/currency-specific accounts.

- **Swap Direction is Handled Internally:** You do **not** need to change the account order for an A-to-B swap versus a B-to-A swap.
  - The adapter's `swap` function automatically detects whether the user's `source_token_acc` holds the `asset` or the `currency`.
  - Based on this detection, it sets a single boolean flag (`input_crncy`) inside the instruction data payload.
  - The on-chain Deriverse program reads this flag to understand the user's intent (e.g., "the user is providing currency to buy the asset").

Your primary responsibility is to assemble the correct and complete list of 28 accounts for the instrument being traded. The adapter handles the rest of the directional logic.
