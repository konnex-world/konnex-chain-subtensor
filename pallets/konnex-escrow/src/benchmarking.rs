#![cfg(feature = "runtime-benchmarks")]
#![allow(clippy::unwrap_used)]

use crate::{BalanceOf, Ballot, Config, EscrowStatus, Pallet, TaskId, pallet::*};
use core::cmp::min;
use frame_benchmarking::{account, v2::*};
use frame_support::{
	pallet_prelude::BoundedVec,
	traits::{Get, tokens::fungible::Mutate},
};
use frame_system::{RawOrigin, pallet_prelude::BlockNumberFor};
use sp_runtime::traits::{One, SaturatedConversion, Saturating};
use sp_std::vec::Vec;

const SEED: u32 = 0;

fn resolvers_with_len<T: Config>(r: u32) -> BoundedVec<T::AccountId, T::MaxResolvers> {
	let count = min(r, T::MaxResolvers::get());
	let mut resolvers = Vec::with_capacity(count as usize);
	for i in 0..count {
		resolvers.push(account::<T::AccountId>("resolver", i, SEED));
	}
	BoundedVec::try_from(resolvers).unwrap()
}

fn funded_escrow<T: Config>(
	r: u32,
) -> (
	T::AccountId,
	TaskId<T>,
	BoundedVec<T::AccountId, T::MaxResolvers>,
	BlockNumberFor<T>,
) {
	let payer = account::<T::AccountId>("payer", 0, SEED);
	let payee = account::<T::AccountId>("payee", 0, SEED);
	let task_id: TaskId<T> = BoundedVec::try_from(b"task-1".to_vec()).unwrap();
	let now = frame_system::Pallet::<T>::block_number();
	let deadline = now + BlockNumberFor::<T>::from(10u32);
	let amount: BalanceOf<T> = 10_000u64.saturated_into();
	let resolvers = resolvers_with_len::<T>(r);

	T::Currency::set_balance(&payer, amount.saturating_mul(2u64.saturated_into()));

	let _ = Pallet::<T>::fund_escrow(
		RawOrigin::Signed(payer.clone()).into(),
		1u16,
		task_id.clone(),
		payee,
		amount,
		[1u8; 32],
		deadline,
		resolvers.clone(),
	);

	(payer, task_id, resolvers, deadline)
}

#[benchmarks(where BalanceOf<T>: One + Saturating + SaturatedConversion)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn fund_escrow(r: Linear<1, 32>) {
		let payer = whitelisted_caller();
		let payee = account::<T::AccountId>("payee", 0, SEED);
		let task_id: TaskId<T> = BoundedVec::try_from(b"task-fund".to_vec()).unwrap();
		let amount: BalanceOf<T> = 10_000u64.saturated_into();
		let deadline = frame_system::Pallet::<T>::block_number() + BlockNumberFor::<T>::from(10u32);
		let resolvers = resolvers_with_len::<T>(r);

		T::Currency::set_balance(&payer, amount.saturating_mul(2u64.saturated_into()));

		#[extrinsic_call]
		_(
			RawOrigin::Signed(payer),
			1u16,
			task_id,
			payee,
			amount,
			[7u8; 32],
			deadline,
			resolvers.clone(),
		);

		assert!(resolvers.len() as u32 >= 1);
	}

	#[benchmark]
	fn vote_on_escrow(r: Linear<1, 32>) {
		let (_payer, task_id, resolvers, _deadline) = funded_escrow::<T>(r);
		let voter = resolvers.first().cloned().unwrap();
		let now = frame_system::Pallet::<T>::block_number();
		let key = (1u16, task_id.clone());
		let rec = Escrows::<T>::get(&key).unwrap();
		assert_eq!(rec.status, EscrowStatus::AwaitingRelease);
		assert!(now <= rec.deadline);

		#[extrinsic_call]
		_(RawOrigin::Signed(voter), 1u16, task_id.clone(), true);

		match Escrows::<T>::get(&key) {
			Some(rec_after) => assert_eq!(rec_after.status, EscrowStatus::AwaitingRelease),
			None => {
				// Single-vote strict majority: row removed after release (N8).
			}
		}
	}

	#[benchmark]
	fn refund_after_deadline(r: Linear<1, 32>) {
		let (payer, task_id, resolvers, deadline) = funded_escrow::<T>(r);
		let key = (1u16, task_id.clone());
		for resolver in resolvers.iter() {
			Votes::<T>::insert(&key, resolver, Ballot::Approve);
		}

		frame_system::Pallet::<T>::set_block_number(deadline + BlockNumberFor::<T>::one());

		#[extrinsic_call]
		_(RawOrigin::Signed(payer), 1u16, task_id.clone());

		assert!(Escrows::<T>::get(&key).is_none());
	}
}
