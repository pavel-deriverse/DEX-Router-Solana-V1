use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use arrayref::array_ref;

use crate::{
    adapters::common::{before_check, invoke_process, DexProcessor},
    deriverse_program,
    error::ErrorCode,
    HopAccounts, DERIVERSE_INSTRUCTION_TAG,
};

#[repr(C)]
pub struct DeriverseSwapData {
    pub tag: u8,         // DERIVERSE_INSTRUCTION_TAG
    pub input_crncy: u8, // if 0 sell `crncy` else sell `asset`
    pub padding_u16: u16,
    pub instr_id: u32,
    pub price: i64,
    pub amount: i64,
}

impl DeriverseSwapData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(std::mem::size_of::<Self>());
        buf.push(self.tag);
        buf.push(self.input_crncy);
        buf.extend_from_slice(&self.padding_u16.to_le_bytes());
        buf.extend_from_slice(&self.instr_id.to_le_bytes());
        buf.extend_from_slice(&self.price.to_le_bytes());
        buf.extend_from_slice(&self.amount.to_le_bytes());

        buf
    }
}

pub struct DeriverseSwapAccounts<'info> {
    pub authority: &'info AccountInfo<'info>,
    pub source_token_acc: &'info AccountInfo<'info>,
    pub destination_token_acc: &'info AccountInfo<'info>,

    // A/B instrument
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
    pub asset_token_program_acc: &'info AccountInfo<'info>,
    pub crncy_token_program_acc: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub crncy_mint: &'info AccountInfo<'info>,
    pub asset_token_acc: &'info AccountInfo<'info>,
    pub crncy_token_acc: &'info AccountInfo<'info>,
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
            authority,
            source_token_acc,
            destination_token_acc,
            root,
            instrument,
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
            asset_token_program_acc,
            crncy_token_program_acc,
            asset_mint,
            crncy_mint,
            asset_token_acc,
            crncy_token_acc,
            drvs_auth,
            system_program,
            asset_token_program_id,
            crncy_token_program_id,
            associated_program_id,
        ]: &[AccountInfo<'info>; ACCOUNTS_LEN] =
            array_ref![accounts, offset, ACCOUNTS_LEN];

        Ok(Self {
            authority,
            source_token_acc,
            destination_token_acc,
            root,
            instrument,
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
            asset_mint,
            crncy_mint,
            drvs_auth,
            system_program,
            asset_token_program_id,
            crncy_token_program_id,
            associated_program_id,
            asset_token_program_acc,
            crncy_token_program_acc,
            asset_token_acc,
            crncy_token_acc,
        })
    }
}

pub struct DeriverseProcessor;
impl DexProcessor for DeriverseProcessor {}

#[repr(C)]
pub struct TokenState {
    pub discriminator: (u32, u32),
    pub address: Pubkey,
    pub program_address: Pubkey,
    pub id: u32,
    pub mask: u32,
    pub reserved: u32,
    pub base_crncy_index: u32,
}

impl TokenState {
    fn get_address_from_raw(data: &[u8]) -> Pubkey {
        // discriminator: (u32, u32)
        // address: Pubkey
        let address_bytes: [u8; 32] = data[8..8 + 32].try_into().unwrap();
        Pubkey::new_from_array(address_bytes)
    }
}

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
        swap_accounts.authority,
        &InterfaceAccount::try_from(swap_accounts.source_token_acc)?,
        swap_accounts.destination_token_acc.key(),
        hop_accounts,
        hop,
        proxy_swap,
        owner_seeds,
    )?;

    let instr_id = {
        let instr_data = swap_accounts.instrument.try_borrow_data()?;
        u32::from_le_bytes(*array_ref![instr_data, 8, 4])
    };

    let asset_mint =
        TokenState::get_address_from_raw(&swap_accounts.asset_token_acc.try_borrow_data()?);
    let crncy_mint =
        TokenState::get_address_from_raw(&swap_accounts.crncy_token_acc.try_borrow_data()?);

    let (input_is_crncy, crncy_acc, asset_acc) = {
        if crncy_mint == *swap_accounts.source_token_acc.owner {
            (
                true,
                swap_accounts.source_token_acc,
                swap_accounts.destination_token_acc,
            )
        } else {
            (
                false,
                swap_accounts.destination_token_acc,
                swap_accounts.source_token_acc,
            )
        }
    };

    require_keys_eq!(
        if input_is_crncy {
            crncy_mint
        } else {
            asset_mint
        },
        *swap_accounts.source_token_acc.owner,
        ErrorCode::InvalidSourceTokenAccount
    );
    require_keys_eq!(
        if input_is_crncy {
            asset_mint
        } else {
            crncy_mint
        },
        *swap_accounts.destination_token_acc.owner,
        ErrorCode::InvalidDestinationTokenAccount
    );

    let data = DeriverseSwapData {
        tag: DERIVERSE_INSTRUCTION_TAG,
        input_crncy: !input_is_crncy as u8,
        padding_u16: 0,
        instr_id,
        price: 0,
        amount: amount_in as i64,
    };

    let account_metas = vec![
        AccountMeta::new(swap_accounts.authority.key(), true),
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
        AccountMeta::new(swap_accounts.asset_token_program_acc.key(), false),
        AccountMeta::new(swap_accounts.crncy_token_program_acc.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_mint.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_token_acc.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_token_acc.key(), false),
        AccountMeta::new(asset_acc.key(), false),
        AccountMeta::new(crncy_acc.key(), false),
        AccountMeta::new_readonly(swap_accounts.drvs_auth.key(), false),
        AccountMeta::new_readonly(swap_accounts.system_program.key(), false),
        AccountMeta::new_readonly(swap_accounts.asset_token_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.crncy_token_program_id.key(), false),
        AccountMeta::new_readonly(swap_accounts.associated_program_id.key(), false),
    ];

    let account_infos = vec![
        swap_accounts.authority.to_account_info(),
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
        swap_accounts.asset_token_program_acc.to_account_info(),
        swap_accounts.crncy_token_program_acc.to_account_info(),
        swap_accounts.asset_mint.to_account_info(),
        swap_accounts.crncy_mint.to_account_info(),
        swap_accounts.asset_token_acc.to_account_info(),
        swap_accounts.crncy_token_acc.to_account_info(),
        asset_acc.to_account_info(),
        crncy_acc.to_account_info(),
        swap_accounts.drvs_auth.to_account_info(),
        swap_accounts.system_program.to_account_info(),
        swap_accounts.asset_token_program_id.to_account_info(),
        swap_accounts.crncy_token_program_id.to_account_info(),
        swap_accounts.associated_program_id.to_account_info(),
    ];

    let instruction: Instruction = Instruction {
        program_id: deriverse_program::id(),
        accounts: account_metas,
        data: data.to_bytes(),
    };

    let dex_processor = &DeriverseProcessor;
    let amount_out = invoke_process(
        amount_in,
        dex_processor,
        &account_infos,
        &mut InterfaceAccount::try_from(swap_accounts.source_token_acc)?,
        &mut InterfaceAccount::try_from(swap_accounts.destination_token_acc)?,
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
