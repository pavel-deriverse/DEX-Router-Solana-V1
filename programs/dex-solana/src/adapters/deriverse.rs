use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_spl::{
    token::Token,
    token_interface::{Mint, TokenAccount},
};
use arrayref::array_ref;

use crate::{
    adapters::common::{before_check, invoke_process, DexProcessor},
    deriverse_program,
    error::ErrorCode,
    HopAccounts,
};

// #[repr(C)]
// pub struct DeriverseSwapData {
//     pub tag: u8,
//     pub input_crncy: u8,
//     pub padding_u16: u16,
//     pub instr_id: u32, // InstrId ??
//     pub price: i64,    // fixed point decimals ??
//     pub amount: i64,   //
// }

const ARGS_LEN: usize = 24;

pub struct DeriverseSwapAccounts<'info> {
    pub signer: &'info AccountInfo<'info>,
    pub root: &'info AccountInfo<'info>,
    pub instrument: &'info AccountInfo<'info>,
    pub bids_tree: &'info AccountInfo<'info>,
    pub asks_tree: &'info AccountInfo<'info>,
    pub bid_orders: &'info AccountInfo<'info>,
    pub ask_orders: &'info AccountInfo<'info>,
    pub lines: &'info AccountInfo<'info>,
    pub map: &'info AccountInfo<'info>,
    pub spot_client_infos: &'info AccountInfo<'info>,
    pub spot_client_infos2: &'info AccountInfo<'info>,
    pub candles_1m: &'info AccountInfo<'info>,
    pub candles_15m: &'info AccountInfo<'info>,
    pub candles_day: &'info AccountInfo<'info>,
    pub community: &'info AccountInfo<'info>,
    pub asset_token_program: Program<'info, Token>,
    pub crncy_token_program: Program<'info, Token>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub crncy_mint: &'info AccountInfo<'info>,
    pub asset_token: InterfaceAccount<'info, TokenAccount>,
    pub crncy_token: InterfaceAccount<'info, TokenAccount>,
    pub client_asset_token: InterfaceAccount<'info, TokenAccount>,
    pub client_crncy_token: InterfaceAccount<'info, TokenAccount>,
    pub drvs_auth: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub asset_token_program_id: &'info AccountInfo<'info>,
    pub crncy_token_program_id: &'info AccountInfo<'info>,
    pub associated_program_id: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 28;

impl<'info> DeriverseSwapAccounts<'info> {
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            signer,
            root,
            instruction,
            bids_tree,
            asks_tree,
            bid_orders,
            ask_orders,
            lines,
            map,
            spot_client_infos,
            spot_client_infos2,
            candles_1m,
            candles_15m,
            candles_day,
            community,
            asset_token_program,
            crncy_token_program,
            asset_mint,
            crncy_mint,
            asset_token,
            crncy_token,
            client_asset_token,
            client_crncy_token,
            drvs_auth,
            system_program,
            asset_token_program_id,
            crncy_token_program_id,
            associated_program_id,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] =
            array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            signer,
            root,
            instrument: instruction,
            bids_tree,
            asks_tree,
            bid_orders,
            ask_orders,
            lines,
            map,
            spot_client_infos,
            spot_client_infos2,
            candles_1m,
            candles_15m,
            candles_day,
            community,
            asset_token_program: Program::try_from(asset_token_program)?,
            crncy_token_program: Program::try_from(crncy_token_program)?,
            asset_mint,
            crncy_mint,
            asset_token: InterfaceAccount::try_from(asset_token)?,
            crncy_token: InterfaceAccount::try_from(crncy_token)?,
            client_asset_token: InterfaceAccount::try_from(client_asset_token)?,
            client_crncy_token: InterfaceAccount::try_from(client_crncy_token)?,
            drvs_auth,
            system_program,
            asset_token_program_id,
            crncy_token_program_id,
            associated_program_id,
        })
    }
}

pub struct DeriverseProcessor;
impl DexProcessor for DeriverseProcessor {}

pub fn swap<'a>(
    remaining_accounts: &'a [AccountInfo<'a>],
    amount_in: u64,
    offset: &mut usize,
    hop_accounts: &mut HopAccounts,
    hop: usize,
    proxy_swap: bool,
    owner_seeds: Option<&[&[&[u8]]]>,
) -> Result<u64> {
    msg!(
        "Dex::DeriverseSwap amount_in: {}, offset: {}",
        amount_in,
        offset
    );
    require!(
        remaining_accounts.len() >= *offset + ACCOUNTS_LEN,
        ErrorCode::InvalidAccountsLength
    );

    let swap_accounts = DeriverseSwapAccounts::parse_accounts(remaining_accounts, *offset)?;

    before_check(
        swap_accounts.signer,
        &swap_accounts.client_crncy_token,
        swap_accounts.client_asset_token.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let instr_data = swap_accounts.instrument.data.borrow();
    let instr_id = u32::from_le_bytes(*array_ref![instr_data, 8, 4]);

    let mut data = Vec::<u8>::with_capacity(ARGS_LEN);
    data.extend_from_slice(&[26, { todo!("input_crncy") }, 0, 0]);
    data.extend_from_slice(&instr_id.to_le_bytes());
    data.extend_from_slice(&[0; 8]);
    data.extend_from_slice(&amount_in.to_le_bytes());

    let account_metas = vec![
        AccountMeta::new(swap_accounts.signer.key(), true),
        AccountMeta::new_readonly(swap_accounts.root.key(), false),
        AccountMeta::new(swap_accounts.instrument.key(), false),
        AccountMeta::new(swap_accounts.bids_tree.key(), false),
        AccountMeta::new(swap_accounts.asks_tree.key(), false),
        AccountMeta::new(swap_accounts.bid_orders.key(), false),
        AccountMeta::new(swap_accounts.ask_orders.key(), false),
        AccountMeta::new(swap_accounts.lines.key(), false),
        AccountMeta::new(swap_accounts.map.key(), false),
        AccountMeta::new(swap_accounts.spot_client_infos.key(), false),
        AccountMeta::new(swap_accounts.spot_client_infos2.key(), false),
        AccountMeta::new(swap_accounts.candles_1m.key(), false),
        AccountMeta::new(swap_accounts.candles_15m.key(), false),
        AccountMeta::new(swap_accounts.candles_day.key(), false),
        AccountMeta::new_readonly(swap_accounts.community.key(), false),
        AccountMeta::new(swap_accounts.asset_token_program.key(), false),
        AccountMeta::new(swap_accounts.crncy_token_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_token.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_token.key(), false),
        AccountMeta::new(swap_accounts.client_asset_token.key(), false),
        AccountMeta::new(swap_accounts.client_crncy_token.key(), false),
        AccountMeta::new_readonly(swap_accounts.drvs_auth.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_token_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_token_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.signer.to_account_info(),
        swap_accounts.root.to_account_info(),
        swap_accounts.instrument.to_account_info(),
        swap_accounts.bids_tree.to_account_info(),
        swap_accounts.asks_tree.to_account_info(),
        swap_accounts.bid_orders.to_account_info(),
        swap_accounts.ask_orders.to_account_info(),
        swap_accounts.lines.to_account_info(),
        swap_accounts.map.to_account_info(),
        swap_accounts.spot_client_infos.to_account_info(),
        swap_accounts.spot_client_infos2.to_account_info(),
        swap_accounts.candles_1m.to_account_info(),
        swap_accounts.candles_15m.to_account_info(),
        swap_accounts.candles_day.to_account_info(),
        swap_accounts.community.to_account_info(),
        swap_accounts.asset_token_program.to_account_info(),
        swap_accounts.crncy_token_program.to_account_info(),
        swap_accounts.asset_mint.to_account_info(),
        swap_accounts.crncy_mint.to_account_info(),
        swap_accounts.asset_token.to_account_info(),
        swap_accounts.crncy_token.to_account_info(),
        swap_accounts.client_asset_token.to_account_info(),
        swap_accounts.client_crncy_token.to_account_info(),
        swap_accounts.drvs_auth.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.asset_token_program_id.to_account_info(),
        swap_accounts.crncy_token_program_id.to_account_info(),
        swap_accounts.associated_program_id.to_account_info(),
    ];

    let instruction: Instruction = Instruction {
        program_id: deriverse_program::id(),
        accounts: account_metas,
        data,
    };

    let dex_processor = &DeriverseProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut swap_accounts.client_crncy_token,
        &mut swap_accounts.client_asset_token,
        hop_accounts,
        instruction,
        hop,
        offset,
        ACCOUNTS_LEN,
        proxy_swap,
        owner_seeds,
    )?;

    Ok(amount_out)
}
