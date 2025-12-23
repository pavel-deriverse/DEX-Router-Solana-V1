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

    // A/B and B/A accounts
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
    pub drvs_vault_asset_token_acc: &'info AccountInfo<'info>,
    pub drvs_vault_crncy_token_acc: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub crncy_mint: &'info AccountInfo<'info>,
    pub drvs_asset_token_state: &'info AccountInfo<'info>,
    pub drvs_crncy_token_state: &'info AccountInfo<'info>,
    pub drvs_auth: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub asset_token_program: &'info AccountInfo<'info>,
    pub crncy_token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
}

const ACCOUNTS_LEN: usize = 28;

impl<'info> DeriverseSwapAccounts<'info> {
    #[inline]
    fn parse_accounts(accounts: &'info [AccountInfo<'info>], offset: usize) -> Result<Self> {
        let [
            authority,
            source_token_acc,
            destination_token_acc,

            // A/B and B/A remaining accounts

            // non calculated
            drvs_vault_asset_token_acc,
            drvs_vault_crncy_token_acc,
            asset_mint,
            crncy_mint,


            // calculated
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
            drvs_asset_token_state,
            drvs_crncy_token_state,
            drvs_auth,

            // programs
            system_program,
            asset_token_program,
            crncy_token_program,
            associated_token_program,

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
            asset_token_program,
            crncy_token_program,
            associated_token_program,
            drvs_vault_asset_token_acc,
            drvs_vault_crncy_token_acc,
            drvs_asset_token_state,
            drvs_crncy_token_state,
        })
    }

    #[inline]
    fn is_crncy_input(&self) -> Result<bool> {
        let input_is_crncy = if self.crncy_token_program.key() == *self.source_token_acc.owner {
            true
        } else {
            false
        };

        require_keys_eq!(
            if input_is_crncy {
                self.crncy_token_program.key()
            } else {
                self.asset_token_program.key()
            },
            *self.source_token_acc.owner,
            ErrorCode::InvalidSourceTokenAccount
        );
        require_keys_eq!(
            if input_is_crncy {
                self.asset_token_program.key()
            } else {
                self.crncy_token_program.key()
            },
            *self.destination_token_acc.owner,
            ErrorCode::InvalidDestinationTokenAccount
        );
        Ok(input_is_crncy)
    }

    #[inline]
    fn create_data(&self, input_is_crncy: bool, amount_in: u64) -> Result<Vec<u8>> {
        let instr_id = {
            let instr_data = self.instrument.try_borrow_data()?;
            u32::from_le_bytes(*array_ref![instr_data, 8, 4])
        };

        Ok(DeriverseSwapData {
            tag: DERIVERSE_INSTRUCTION_TAG,
            input_crncy: input_is_crncy as u8,
            padding_u16: 0,
            instr_id,
            price: 0,
            amount: amount_in as i64,
        }
        .to_bytes())
    }

    #[inline]
    fn get_account_metas(&self, input_is_crncy: bool) -> Vec<AccountMeta> {
        let (client_asset_token_acc, client_crncy_token_acc) = if input_is_crncy {
            (self.destination_token_acc, self.source_token_acc)
        } else {
            (self.source_token_acc, self.destination_token_acc)
        };

        vec![
            AccountMeta::new(self.authority.key(), true),
            AccountMeta::new_readonly(self.root.key(), false),
            AccountMeta::new(self.instrument.key(), false),
            AccountMeta::new(self.bids_tree.key(), false),
            AccountMeta::new(self.asks_tree.key(), false),
            AccountMeta::new(self.bid_orders.key(), false),
            AccountMeta::new(self.ask_orders.key(), false),
            AccountMeta::new(self.lines.key(), false),
            AccountMeta::new(self.map.key(), false),
            AccountMeta::new(self.spot_client_infos.key(), false),
            AccountMeta::new(self.spot_client_infos2.key(), false),
            AccountMeta::new(self.candles_1m.key(), false),
            AccountMeta::new(self.candles_15m.key(), false),
            AccountMeta::new(self.candles_day.key(), false),
            AccountMeta::new_readonly(self.community.key(), false),
            AccountMeta::new(self.drvs_vault_asset_token_acc.key(), false),
            AccountMeta::new(self.drvs_vault_crncy_token_acc.key(), false),
            AccountMeta::new_readonly(self.asset_mint.key(), false),
            AccountMeta::new_readonly(self.crncy_mint.key(), false),
            AccountMeta::new_readonly(self.drvs_asset_token_state.key(), false),
            AccountMeta::new_readonly(self.drvs_crncy_token_state.key(), false),
            AccountMeta::new(client_asset_token_acc.key(), false),
            AccountMeta::new(client_crncy_token_acc.key(), false),
            AccountMeta::new_readonly(self.drvs_auth.key(), false),
            AccountMeta::new_readonly(self.system_program.key(), false),
            AccountMeta::new_readonly(self.asset_token_program.key(), false),
            AccountMeta::new_readonly(self.crncy_token_program.key(), false),
            AccountMeta::new_readonly(self.associated_token_program.key(), false),
        ]
    }

    #[inline]
    fn get_account_infos(&self, input_is_crncy: bool) -> Vec<AccountInfo<'info>> {
        let (client_asset_token_acc, client_crncy_token_acc) = if input_is_crncy {
            (self.destination_token_acc, self.source_token_acc)
        } else {
            (self.source_token_acc, self.destination_token_acc)
        };

        vec![
            self.authority.to_account_info(),
            self.root.to_account_info(),
            self.instrument.to_account_info(),
            self.bids_tree.to_account_info(),
            self.asks_tree.to_account_info(),
            self.bid_orders.to_account_info(),
            self.ask_orders.to_account_info(),
            self.lines.to_account_info(),
            self.map.to_account_info(),
            self.spot_client_infos.to_account_info(),
            self.spot_client_infos2.to_account_info(),
            self.candles_1m.to_account_info(),
            self.candles_15m.to_account_info(),
            self.candles_day.to_account_info(),
            self.community.to_account_info(),
            self.drvs_vault_asset_token_acc.to_account_info(),
            self.drvs_vault_crncy_token_acc.to_account_info(),
            self.asset_mint.to_account_info(),
            self.crncy_mint.to_account_info(),
            self.drvs_asset_token_state.to_account_info(),
            self.drvs_crncy_token_state.to_account_info(),
            client_asset_token_acc.to_account_info(),
            client_crncy_token_acc.to_account_info(),
            self.drvs_auth.to_account_info(),
            self.system_program.to_account_info(),
            self.asset_token_program.to_account_info(),
            self.crncy_token_program.to_account_info(),
            self.associated_token_program.to_account_info(),
        ]
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

    let input_is_crncy = swap_accounts.is_crncy_input()?;

    let data = swap_accounts.create_data(input_is_crncy, amount_in)?;

    let account_metas = swap_accounts.get_account_metas(input_is_crncy);
    let instruction: Instruction = Instruction {
        program_id: deriverse_program::id(),
        accounts: account_metas,
        data,
    };

    let account_infos = swap_accounts.get_account_infos(input_is_crncy);

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
