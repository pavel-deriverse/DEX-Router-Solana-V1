use anchor_lang::prelude::*;

#[constant]
pub const SEED_SA: &[u8] = b"okx_sa";
pub const BUMP_SA: u8 = 251;

pub const SA_AUTHORITY_SEED: &[&[&[u8]]] = &[&[SEED_SA, &[BUMP_SA]]];

// Actual amount_in lower bound ratio for post swap check
pub const ACTUAL_IN_LOWER_BOUND_NUM: u128 = 95; // 95%
pub const ACTUAL_IN_LOWER_BOUND_DEN: u128 = 100; // denominator for percentage

pub const ZERO_ADDRESS: Pubkey = Pubkey::new_from_array([0u8; 32]);

pub const DERIVERSE_INSTRUCTION_TAG: u8 = 26;
pub const DERIVERSE_VERSION: u32 = 1;

#[cfg(feature = "staging")]
pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("4DwLmWvMyWPPKa8jhmW6AZKGctUMe7GxAWrb2Wcw8ZUa"); //pre_deploy
}

#[cfg(not(feature = "staging"))]
pub mod authority_pda {
    use anchor_lang::declare_id;
    declare_id!("HV1KXxWFaSeriyFvXyx48FqG9BoFbfinB8njCJonqP7K");
}

// ******************** dex program ids ******************** //

pub mod deriverse_program {
    use anchor_lang::declare_id;
    declare_id!("DRVSpZ2YUYYKgZP8XtLhAGtT1zYSCKzeHfb4DgRnrgqD");
}
