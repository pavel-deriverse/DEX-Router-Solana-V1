use anchor_lang::prelude::*;

#[derive(Debug)]
pub struct HopAccounts {
    pub last_to_account: Pubkey,
    pub from_account: Pubkey,
    pub to_account: Pubkey,
}
