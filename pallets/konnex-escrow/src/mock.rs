#![cfg(test)]
#![allow(clippy::unwrap_used)]

use crate as pallet_konnex_escrow;
use frame_support::{
    PalletId, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_runtime::{BuildStorage, traits::IdentityLookup};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system = 1,
        Balances: pallet_balances = 2,
        KonnexEscrow: pallet_konnex_escrow = 3,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type Balance = u64;
}

parameter_types! {
    pub const KonnexEscrowPalletId: PalletId = PalletId(*b"knexscrw");
}

impl pallet_konnex_escrow::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type PalletId = KonnexEscrowPalletId;
    type WeightInfo = ();
    type MaxTaskIdLen = ConstU32<32>;
    type MaxResolvers = ConstU32<16>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 1_000), (2, 1_000), (3, 1_000), (4, 1_000), (5, 1_000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
