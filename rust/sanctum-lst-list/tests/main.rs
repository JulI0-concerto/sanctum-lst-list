use std::collections::HashSet;

use borsh::BorshDeserialize;
use sanctum_lst_list::{PoolInfo, SanctumLst, SanctumLstList};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use spl_stake_pool_interface::{StakePool, ValidatorList};
use spl_token_2022::extension::StateWithExtensions;

const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

#[test]
fn load() {
    // make sure this doesnt panic
    let SanctumLstList { sanctum_lst_list } = SanctumLstList::load();

    // validate unique mints
    let mut dedup = HashSet::new();
    for SanctumLst { mint, .. } in &sanctum_lst_list {
        if !dedup.insert(mint) {
            panic!("Duplicate mint {mint} found");
        }
    }
    println!("{sanctum_lst_list:#?}");
    // TODO: sample 1 from each enum variant to check the fields values
    // to make sure everything is deserialized as expected
}

#[test]
fn it_serializes() {
    let loaded = SanctumLstList::load();
    let serialized = toml::to_string(&loaded).unwrap();
    println!("{}", serialized);
}

// this takes around 30s with around 70 pools
#[test]
fn verify_all_pools_valid() {
    let rpc = RpcClient::new(SOLANA_RPC_URL);
    let SanctumLstList { sanctum_lst_list } = SanctumLstList::load();
    // just do it sequentially to avoid rpc limits
    for sanctum_lst in sanctum_lst_list {
        verify_pool_valid(&rpc, &sanctum_lst);
    }
}

fn verify_pool_valid(
    rpc: &RpcClient,
    SanctumLst {
        mint,
        token_program,
        symbol,
        decimals,
        pool,
        ..
    }: &SanctumLst,
) {
    let spl_accounts = match pool {
        PoolInfo::Lido | PoolInfo::Marinade | PoolInfo::ReservePool | PoolInfo::SPool(_) => None,
        PoolInfo::SanctumSpl(accounts)
        | PoolInfo::SanctumSplMulti(accounts)
        | PoolInfo::Spl(accounts) => Some(accounts),
    };

    let mut accounts_to_fetch = match spl_accounts {
        None => vec![],
        Some(accounts) => {
            let mut v = vec![accounts.pool, accounts.validator_list];
            if let Some(vote) = accounts.vote_account {
                v.push(vote);
            }
            v
        }
    };
    accounts_to_fetch.push(*mint);

    let mut accounts = rpc.get_multiple_accounts(&accounts_to_fetch).unwrap();

    let mint_acc = accounts.pop().unwrap().unwrap();
    assert_eq!(
        mint_acc.owner, *token_program,
        "{symbol} wrong token program. Expected {}. Actual {}",
        token_program, mint_acc.owner
    );
    let StateWithExtensions { base, .. } =
        StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_acc.data).unwrap();
    assert_eq!(
        base.decimals, *decimals,
        "{symbol} wrong decimals. Expected {}. Actual {}",
        decimals, base.decimals
    );
    assert!(base.is_initialized, "{symbol} mint not initialized");

    if let Some(spl_accounts) = spl_accounts {
        if let Some(vote) = spl_accounts.vote_account {
            let vote_acc = accounts.pop().unwrap().unwrap();
            assert_eq!(
                vote_acc.owner,
                solana_program::vote::program::ID,
                "{symbol} vote_account {vote} is not a vote account"
            );
        }

        let spl_stake_pool_prog_id: Pubkey = pool.pool_program().into();

        let validator_list_acc = accounts.pop().unwrap().unwrap();
        assert_eq!(
            validator_list_acc.owner, spl_stake_pool_prog_id,
            "{symbol} wrong validator list owner. Expected {}. Actual {}",
            spl_stake_pool_prog_id, validator_list_acc.owner
        );
        // this shouldnt panic
        let _v = ValidatorList::deserialize(&mut validator_list_acc.data.as_slice()).unwrap();

        let stake_pool_acc = accounts.pop().unwrap().unwrap();
        assert_eq!(
            stake_pool_acc.owner, spl_stake_pool_prog_id,
            "{symbol} wrong stake pool owner. Expected {}. Actual {}",
            spl_stake_pool_prog_id, stake_pool_acc.owner
        );
        let pool = StakePool::deserialize(&mut stake_pool_acc.data.as_slice()).unwrap();
        assert_eq!(
            pool.validator_list, spl_accounts.validator_list,
            "{symbol} wrong validator list. Expected {}. Actual {}",
            spl_accounts.validator_list, pool.validator_list
        );
    }
}
