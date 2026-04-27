//! # Konnex escrow (subnet-scoped task payments)
//!
//! Native-token escrow keyed by `(subnet_id, task_id)`. The **payer** locks funds on this
//! pallet's account, names a **payee**, commits to a task with **`description_hash`** (e.g.
//! SHA-256 of the description off-chain), nominates a **resolver committee**, and sets a
//! **`deadline` block**. This is a FRAME pallet (on-chain module), not an EVM contract.
//!
//! ## Extrinsics (public calls)
//!
//! - `fund_escrow` — payer signs; transfers `amount` to the pallet pot and opens the escrow.
//!   Must pass a future `deadline` (block number after which unresolved escrows may be
//!   reclaimed by the payer).
//! - `vote_on_escrow` — **resolver-only** (only while `block_number <= deadline`); one vote per
//!   resolver: **approve** pays the payee, **reject** refunds the payer, when a **strict majority**
//!   of resolvers agrees (`2*votes > n`).
//! - `refund_after_deadline` — **payer-only**; after `deadline` if the escrow is still awaiting
//!   resolution (no majority vote yet), the payer takes funds back. Not callable once funds
//!   moved to the payee (**released**) or already returned (**refunded**), including after a
//!   reject quorum.
//!
//! ## Typical workflow
//!
//! 1. **Payer** chooses `subnet_id`, unique `task_id`, `payee`, `amount`, `description_hash`,
//!    resolver list, and `deadline` > current block.
//! 2. Payer calls **`fund_escrow`** — balances move to the pallet; state is `AwaitingRelease`.
//! 3. **Resolvers** (and only they) call **`vote_on_escrow`** with `approve = true/false` **on or
//!    before the `deadline` block**.
//!    - If **strict majority approve** → full `amount` goes to **payee**; escrow `Released`.
//!      The payer **cannot** use `refund_after_deadline` (wrong status).
//!    - If **strict majority reject** → funds return to **payer**; escrow `Refunded`.
//! 4. If the committee **never reaches a majority** before **`deadline`**, the **payer** calls
//!    **`refund_after_deadline`** to recover funds (same economic outcome as “task not verified”).
//! 5. Indexers/wallets read **`EscrowFunded`**, **`EscrowVoted`**, **`EscrowReleased`**, **`EscrowRefunded`**.
//!
//! ## Resolver committee (payer-chosen)
//!
//! At `fund_escrow`, the payer supplies a **bounded list of accounts** that are the only
//! parties allowed to vote. **Who may arbitrate is explicit trust** chosen by the payer (no
//! on-chain link to Subtensor `ValidatorPermit` in this pallet).
//!
//! ## Voting and settlement
//!
//! Each resolver may cast **at most one** ballot per escrow: **Approve** (pay the payee) or
//! **Reject** (return funds to the payer). Settlement uses a **strict majority** of the resolver
//! count (`2 * votes > n`). There is **no Root/sudo** release path.
//!
//! ## Deadline rules
//!
//! - `deadline` is a **block height** (must be **strictly greater** than the current block when
//!   funding).
//! - Resolvers may call **`vote_on_escrow` only while `block_number <= deadline`**. After the
//!   deadline, new votes are rejected so they cannot race **`refund_after_deadline`** in the same
//!   block (N7).
//! - While status is `AwaitingRelease` and `block_number > deadline`, the **payer** may
//!   **`refund_after_deadline`** if resolvers never reached a majority (stalemate / no decision).
//! - After **`EscrowReleased`** (payee paid), the payer **cannot** reclaim funds on-chain via
//!   this pallet; those balances belong to the payee per committee vote.
//!
//! ## Limits and safety
//!
//! - `task_id` and resolver list lengths are bounded via `Config::MaxTaskIdLen` and
//!   `Config::MaxResolvers`.
//! - Resolver list must be **non-empty**, **unique** (no duplicate accounts).
//! - PoPW / off-chain verification is **not** enforced here; only resolver votes and deadlines.
//!
//! ## Storage layout
//!
//! - `Escrows`: **open** escrows only (`AwaitingRelease`). Rows are **`remove`d** after
//!   **Released**, **Refunded** (vote or deadline), or equivalent settlement — use events for
//!   history (N8).
//! - `Votes`: one ballot per `(escrow key, resolver)` while the escrow is open (cleared before
//!   removal).

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
	Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo,
)]
pub enum EscrowStatus {
	AwaitingRelease,
	Released,
	Refunded,
}

#[derive(
	Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo,
)]
pub enum Ballot {
	Approve,
	Reject,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	pub type TaskId<T> = BoundedVec<u8, <T as Config>::MaxTaskIdLen>;

	pub type EscrowKey<T> = (u16, TaskId<T>);

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct EscrowRecord<T: Config>
	where
		T::AccountId: MaxEncodedLen,
		BalanceOf<T>: MaxEncodedLen,
		BlockNumberFor<T>: MaxEncodedLen,
	{
		pub payer: T::AccountId,
		pub payee: T::AccountId,
		pub amount: BalanceOf<T>,
		/// Commitment to the task description (e.g. SHA-256); full text stays off-chain.
		pub description_hash: [u8; 32],
		/// Accounts that may vote to release or refund (chosen by the payer at funding).
		pub resolvers: BoundedVec<T::AccountId, T::MaxResolvers>,
		/// Block height after which the payer may `refund_after_deadline` if still unresolved.
		pub deadline: BlockNumberFor<T>,
		pub status: EscrowStatus,
	}

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
		/// Maximum resolvers the payer may nominate for one escrow.
		#[pallet::constant]
		type MaxResolvers: Get<u32>;
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
			description_hash: [u8; 32],
			deadline: BlockNumberFor<T>,
			resolver_count: u32,
		},
		EscrowVoted {
			subnet_id: u16,
			task_id: Vec<u8>,
			voter: T::AccountId,
			approve: bool,
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
		EmptyResolverSet,
		DuplicateResolver,
		NotResolver,
		AlreadyVoted,
		DeadlineInPast,
		DeadlineNotPassed,
		/// Resolver votes are not accepted after `deadline` (payer may `refund_after_deadline`).
		VotingAfterDeadline,
		NotPayer,
	}

	#[pallet::storage]
	pub type Escrows<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		EscrowKey<T>,
		EscrowRecord<T>,
		OptionQuery,
	>;

	/// One ballot per resolver per escrow key (`Approve` / `Reject`).
	#[pallet::storage]
	pub type Votes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EscrowKey<T>,
		Blake2_128Concat,
		T::AccountId,
		Ballot,
		OptionQuery,
	>;

	impl<T: Config> Pallet<T> {
		pub fn pallet_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn ensure_unique_resolvers(resolvers: &[T::AccountId]) -> DispatchResult {
			ensure!(!resolvers.is_empty(), Error::<T>::EmptyResolverSet);
			let mut seen = sp_std::collections::btree_set::BTreeSet::new();
			for resolver in resolvers.iter() {
				ensure!(
					seen.insert(resolver.encode()),
					Error::<T>::DuplicateResolver
				);
			}
			Ok(())
		}

		fn count_votes(key: &EscrowKey<T>, rec: &EscrowRecord<T>) -> (u32, u32) {
			let mut approve = 0u32;
			let mut reject = 0u32;
			for who in rec.resolvers.iter() {
				match Votes::<T>::get(key, who) {
					Some(Ballot::Approve) => approve = approve.saturating_add(1),
					Some(Ballot::Reject) => reject = reject.saturating_add(1),
					None => {}
				}
			}
			(approve, reject)
		}

		fn clear_votes(key: &EscrowKey<T>, rec: &EscrowRecord<T>) {
			for who in rec.resolvers.iter() {
				Votes::<T>::remove(key, who);
			}
		}

		/// Strict majority: `2 * votes > n` (e.g. 2-of-3, 3-of-5).
		fn strict_majority(votes: u32, n: u32) -> bool {
			n > 0 && votes.saturating_mul(2) > n
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Lock `amount` from the signer (payer), store `EscrowRecord` at `(subnet_id, task_id)`.
		///
		/// `AlreadyExists` only while an **open** escrow occupies the key; after release or refund
		/// the storage row is removed and the same `(subnet_id, task_id)` may be funded again (N8).
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::fund_escrow(resolvers.len() as u32))]
		pub fn fund_escrow(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
			payee: T::AccountId,
			amount: BalanceOf<T>,
			description_hash: [u8; 32],
			deadline: BlockNumberFor<T>,
			resolvers: BoundedVec<T::AccountId, T::MaxResolvers>,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::DeadlineInPast);

			Self::ensure_unique_resolvers(resolvers.as_slice())?;

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			ensure!(!Escrows::<T>::contains_key(&key), Error::<T>::AlreadyExists);

			let pot = Self::pallet_account();
			T::Currency::transfer(&payer, &pot, amount, Preservation::Expendable)?;

			let resolver_count = resolvers.len() as u32;
			Escrows::<T>::insert(
				key.clone(),
				EscrowRecord {
					payer: payer.clone(),
					payee: payee.clone(),
					amount,
					description_hash,
					resolvers,
					deadline,
					status: EscrowStatus::AwaitingRelease,
				},
			);

			Self::deposit_event(Event::EscrowFunded {
				subnet_id,
				task_id: task_id.into_inner(),
				payer,
				payee,
				amount,
				description_hash,
				deadline,
				resolver_count,
			});
			Ok(())
		}

		/// Resolver-only: vote to pay the payee (`approve == true`) or refund the payer (`false`).
		/// When a strict majority of resolvers agrees on one side, funds move and votes are cleared.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::vote_on_escrow(T::MaxResolvers::get()))]
		pub fn vote_on_escrow(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
			approve: bool,
		) -> DispatchResult {
			let voter = ensure_signed(origin)?;

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			let mut rec = Escrows::<T>::get(&key).ok_or(Error::<T>::NotFound)?;
			ensure!(
				rec.status == EscrowStatus::AwaitingRelease,
				Error::<T>::WrongStatus
			);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(now <= rec.deadline, Error::<T>::VotingAfterDeadline);
			ensure!(
				rec.resolvers.iter().any(|r| r == &voter),
				Error::<T>::NotResolver
			);
			ensure!(
				!Votes::<T>::contains_key(&key, &voter),
				Error::<T>::AlreadyVoted
			);

			let ballot = if approve {
				Ballot::Approve
			} else {
				Ballot::Reject
			};
			Votes::<T>::insert(&key, &voter, ballot);

			Self::deposit_event(Event::EscrowVoted {
				subnet_id,
				task_id: task_id.clone().into_inner(),
				voter,
				approve,
			});

			let n = rec.resolvers.len() as u32;
			let (approve_votes, reject_votes) = Self::count_votes(&key, &rec);

			let pot = Self::pallet_account();

			if Self::strict_majority(approve_votes, n) {
				let payee = rec.payee.clone();
				let amount = rec.amount;
				T::Currency::transfer(&pot, &payee, amount, Preservation::Expendable)?;
				Self::clear_votes(&key, &rec);
				Escrows::<T>::remove(&key);
				Self::deposit_event(Event::EscrowReleased {
					subnet_id,
					task_id: task_id.into_inner(),
					payee,
					amount,
				});
			} else if Self::strict_majority(reject_votes, n) {
				let payer = rec.payer.clone();
				let amount = rec.amount;
				T::Currency::transfer(&pot, &payer, amount, Preservation::Expendable)?;
				Self::clear_votes(&key, &rec);
				Escrows::<T>::remove(&key);
				Self::deposit_event(Event::EscrowRefunded {
					subnet_id,
					task_id: task_id.into_inner(),
					payer,
					amount,
				});
			} else {
				Escrows::<T>::insert(&key, rec);
			}

			Ok(())
		}

		/// Payer-only: if `block_number > deadline` and the escrow never reached a majority (still
		/// `AwaitingRelease`), return locked funds to the payer. See module-level docs.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::refund_after_deadline(T::MaxResolvers::get()))]
		pub fn refund_after_deadline(
			origin: OriginFor<T>,
			subnet_id: u16,
			task_id: TaskId<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let key: EscrowKey<T> = (subnet_id, task_id.clone());
			let rec = Escrows::<T>::get(&key).ok_or(Error::<T>::NotFound)?;
			ensure!(rec.payer == who, Error::<T>::NotPayer);
			ensure!(
				rec.status == EscrowStatus::AwaitingRelease,
				Error::<T>::WrongStatus
			);
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(now > rec.deadline, Error::<T>::DeadlineNotPassed);

			let pot = Self::pallet_account();
			let payer = rec.payer.clone();
			let amount = rec.amount;
			T::Currency::transfer(&pot, &payer, amount, Preservation::Expendable)?;

			Self::clear_votes(&key, &rec);
			Escrows::<T>::remove(&key);

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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
