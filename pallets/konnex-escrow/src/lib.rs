//! Prototype escrow: funds sit on the pallet account; Root releases to payee or refunds payer.
//! PoPW is **not** enforced on-chain yet — `release_escrow` stands in for "finalized attestation".
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	PalletId,
	pallet_prelude::*,
	sp_runtime::{
		RuntimeDebug,
		traits::{AccountIdConversion, Zero},
	},
	traits::{
		fungible::{Inspect, Mutate},
		tokens::Preservation,
	},
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;

pub mod weights;
pub use weights::WeightInfo;

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo,
)]
pub enum EscrowStatus {
	AwaitingRelease,
	Released,
	Refunded,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct EscrowRecord<AccountId, Balance> {
	pub payer: AccountId,
	pub payee: AccountId,
	pub amount: Balance,
	pub status: EscrowStatus,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	pub type TaskId<T> = BoundedVec<u8, <T as Config>::MaxTaskIdLen>;

	pub type EscrowKey<T> = (u16, TaskId<T>);

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: Inspect<Self::AccountId> + Mutate<Self::AccountId>;
		type PalletId: Get<PalletId>;
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		type MaxTaskIdLen: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		EscrowFunded {
			subnet_id: u16,
			task_id: Vec<u8>,
			payer: T::AccountId,
			payee: T::AccountId,
			amount: BalanceOf<T>,
		},
		EscrowReleased {
			subnet_id: u16,
			task_id: Vec<u8>,
			payee: T::AccountId,
			amount: BalanceOf<T>,
		},
		EscrowRefunded {
			subnet_id: u16,
			task_id: Vec<u8>,
			payer: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyExists,
		NotFound,
		WrongStatus,
		ZeroAmount,
	}

	#[pallet::storage]
	pub type Escrows<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		EscrowKey<T>,
		EscrowRecord<T::AccountId, BalanceOf<T>>,
		OptionQuery,
	>;

	impl<T: Config> Pallet<T> {
		pub fn pallet_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Lock `amount` from origin and record escrow for `(subnet_id, task_id)`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::fund_escrow())]
		pub fn fund_escrow(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
			payee: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			ensure!(!Escrows::<T>::contains_key(&key), Error::<T>::AlreadyExists);

			let pot = Self::pallet_account();
			T::Currency::transfer(&payer, &pot, amount, Preservation::Expendable)?;

			Escrows::<T>::insert(
				key,
				EscrowRecord {
					payer: payer.clone(),
					payee: payee.clone(),
					amount,
					status: EscrowStatus::AwaitingRelease,
				},
			);

			Self::deposit_event(Event::EscrowFunded {
				subnet_id,
				task_id: task_id.into_inner(),
				payer,
				payee,
				amount,
			});
			Ok(())
		}

		/// Prototype: Root only. Transfers locked funds to `payee` (simulates PoPW-approved release).
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::release_escrow())]
		pub fn release_escrow(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			let mut rec = Escrows::<T>::get(&key).ok_or(Error::<T>::NotFound)?;
			ensure!(
				rec.status == EscrowStatus::AwaitingRelease,
				Error::<T>::WrongStatus
			);

			let pot = Self::pallet_account();
			let payee = rec.payee.clone();
			let amount = rec.amount;
			T::Currency::transfer(&pot, &payee, amount, Preservation::Expendable)?;

			rec.status = EscrowStatus::Released;
			Escrows::<T>::insert(&key, rec);

			Self::deposit_event(Event::EscrowReleased {
				subnet_id,
				task_id: task_id.into_inner(),
				payee,
				amount,
			});
			Ok(())
		}

		/// Prototype: Root returns funds to payer.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::refund_escrow())]
		pub fn refund_escrow(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			let mut rec = Escrows::<T>::get(&key).ok_or(Error::<T>::NotFound)?;
			ensure!(
				rec.status == EscrowStatus::AwaitingRelease,
				Error::<T>::WrongStatus
			);

			let pot = Self::pallet_account();
			let payer = rec.payer.clone();
			let amount = rec.amount;
			T::Currency::transfer(&pot, &payer, amount, Preservation::Expendable)?;

			rec.status = EscrowStatus::Refunded;
			Escrows::<T>::insert(&key, rec);

			Self::deposit_event(Event::EscrowRefunded {
				subnet_id,
				task_id: task_id.into_inner(),
				payer,
				amount,
			});
			Ok(())
		}
	}
}

pub use pallet::*;
