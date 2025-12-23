pub mod helper;

use crate::helper::Helper;
use drv_models::state::{
    instrument::InstrAccountHeader,
    types::account_type::{
        COMMUNITY, INSTR, ROOT, SPOT_15M_CANDLES, SPOT_1M_CANDLES, SPOT_ASKS_TREE, SPOT_ASK_ORDERS,
        SPOT_BIDS_TREE, SPOT_BID_ORDERS, SPOT_CLIENT_INFOS, SPOT_CLIENT_INFOS2, SPOT_DAY_CANDLES,
        SPOT_LINES, SPOT_MAPS,
    },
};
use solana_sdk::pubkey::Pubkey;

pub mod constants {
    pub const DRVS_SEED: &[u8; 5] = b"ndxnt";
}

pub mod program_id {
    use drv_models::new_types::version::Version;
    use solana_sdk::declare_id;

    declare_id!("DRVSpZ2YUYYKgZP8XtLhAGtT1zYSCKzeHfb4DgRnrgqD");
    pub const VERSION: Version = Version(1);
}

/// Generates a sequence of Program Derived Addresses (PDAs) for a specific instrument.
/// These PDAs are typically required by Deriverse instructions, and the `offset + X`
/// comments indicate their expected relative positions within a larger `remaining_accounts`
/// array passed to such instructions (e.g., in a DEX router). The indices are relative
/// to the structure defined in `programs/dex-solana/src/adapters/deriverse.rs::parse_accounts`.
pub fn get_remaning_pdas(instr_header: InstrAccountHeader) -> Vec<Pubkey> {
    vec![
        // remaining_accounts[offset + 7]: Root State PDA for Deriverse.
        Pubkey::new_acc(ROOT),
        // remaining_accounts[offset + 8]: Instrument Account PDA, defining the trading pair.
        Pubkey::new_spot_acc(
            INSTR,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 9]: Spot Bids Tree PDA, managing bid orders.
        Pubkey::new_spot_acc(
            SPOT_BIDS_TREE,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 10]: Spot Asks Tree PDA, managing ask orders.
        Pubkey::new_spot_acc(
            SPOT_ASKS_TREE,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 11]: Spot Bid Orders PDA, array of individual bid orders.
        Pubkey::new_spot_acc(
            SPOT_BID_ORDERS,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 12]: Spot Ask Orders PDA, array of individual ask orders.
        Pubkey::new_spot_acc(
            SPOT_ASK_ORDERS,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 13]: Spot Lines PDA, managing order book depth.
        Pubkey::new_spot_acc(
            SPOT_LINES,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 14]: Spot Maps PDA, for memory allocation management.
        Pubkey::new_spot_acc(
            SPOT_MAPS,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 15]: Spot Client Infos PDA (Primary).
        Pubkey::new_spot_acc(
            SPOT_CLIENT_INFOS,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 16]: Spot Client Infos PDA (Secondary).
        Pubkey::new_spot_acc(
            SPOT_CLIENT_INFOS2,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 17]: 1-minute Candles PDA for historical data.
        Pubkey::new_spot_acc(
            SPOT_1M_CANDLES,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 18]: 15-minute Candles PDA for historical data.
        Pubkey::new_spot_acc(
            SPOT_15M_CANDLES,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 19]: Day Candles PDA for historical data.
        Pubkey::new_spot_acc(
            SPOT_DAY_CANDLES,
            instr_header.asset_token_id,
            instr_header.crncy_token_id,
        ),
        // remaining_accounts[offset + 20]: Community Account PDA.
        Pubkey::new_acc(COMMUNITY),
        // remaining_accounts[offset + 21]: Deriverse Asset Token State PDA (program's internal state for the asset token).
        instr_header.asset_mint.new_token_acc(),
        // remaining_accounts[offset + 22]: Deriverse Currency Token State PDA (program's internal state for the currency token).
        instr_header.crncy_mint.new_token_acc(),
        // remaining_accounts[offset + 23]: Deriverse Program Authority PDA.
        Pubkey::get_drvs_auth(),
    ]
}
