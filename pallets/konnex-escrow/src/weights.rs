//! Placeholder weights for prototype pallet (run benchmarks before production).

#![allow(missing_docs)]

use frame_support::traits::Get;
use frame_support::weights::{Weight, constants::RocksDbWeight};
use core::marker::PhantomData;

/// Weight hooks for `pallet_konnex_escrow`.
pub trait WeightInfo {
	fn fund_escrow() -> Weight;
	fn vote_on_escrow() -> Weight;
	fn refund_after_deadline() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn fund_escrow() -> Weight {
		Weight::from_parts(50_000_000, 0)
			.saturating_add(T::DbWeight::get().reads_writes(4, 4))
	}
	fn vote_on_escrow() -> Weight {
		Weight::from_parts(60_000_000, 0)
			.saturating_add(T::DbWeight::get().reads_writes(6, 5))
	}
	fn refund_after_deadline() -> Weight {
		Weight::from_parts(45_000_000, 0)
			.saturating_add(T::DbWeight::get().reads_writes(5, 4))
	}
}

impl WeightInfo for () {
	fn fund_escrow() -> Weight {
		Weight::from_parts(50_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(4, 4))
	}
	fn vote_on_escrow() -> Weight {
		Weight::from_parts(60_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(6, 5))
	}
	fn refund_after_deadline() -> Weight {
		Weight::from_parts(45_000_000, 0).saturating_add(RocksDbWeight::get().reads_writes(5, 4))
	}
}
