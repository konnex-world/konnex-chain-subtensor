#![cfg(test)]
#![allow(clippy::unwrap_used)]

//! Integration-style unit tests for `pallet_konnex_escrow`.
//!
//! ## “1 out of N” vs текущая логика
//!
//! Паллета использует **строгий кворум** `2 * votes > n` по *всем* номинированным резолверам.
//! - **N = 1**: единственный резолвер — это вся комиссия; **один** его голос сразу завершает эскроу
//!   (эквивалентно «решает один арбитр», но это **1-of-1**, а не «любой один из многих»).
//! - **N > 1**: **одного** голоса недостаточно; нужна строгая большинственная доля (например 2 из 3,
//!   3 из 5). Тесты ниже это фиксируют.

use crate::{mock::*, Ballot, Error, EscrowStatus, Escrows, Pallet, Votes};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::Inspect, ConstU32},
	BoundedVec,
};

type MaxTask = ConstU32<32>;
type MaxRes = ConstU32<16>;

/// Block 1 is genesis in `new_test_ext`; keep deadline safely in the future at fund time.
const FAR_DEADLINE: u64 = 10_000;

fn task_id(bytes: &[u8]) -> BoundedVec<u8, MaxTask> {
	BoundedVec::try_from(bytes.to_vec()).unwrap()
}

fn resolvers(accounts: &[u64]) -> BoundedVec<u64, MaxRes> {
	BoundedVec::try_from(accounts.to_vec()).unwrap()
}

fn escrow_key(subnet: u16, tid: &BoundedVec<u8, MaxTask>) -> (u16, BoundedVec<u8, MaxTask>) {
	(subnet, tid.clone())
}

#[test]
fn fund_then_majority_approve_releases_to_payee() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let payee = 2u64;
		let r1 = 3u64;
		let r2 = 4u64;
		let r3 = 5u64;
		let tid = task_id(b"job-1");
		let hash = [7u8; 32];
		let amount = 100u64;

		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			1,
			tid.clone(),
			payee,
			amount,
			hash,
			FAR_DEADLINE,
			resolvers(&[r1, r2, r3]),
		));

		assert_eq!(Balances::balance(&1), 1_000 - amount);
		assert_eq!(Balances::balance(&pot), amount);

		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(r1),
			1,
			tid.clone(),
			true,
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(r2),
			1,
			tid.clone(),
			true,
		));

		assert!(!Escrows::<Test>::contains_key((1u16, tid.clone())));
		assert_eq!(Balances::balance(&payee), 1_000 + amount);
		assert_eq!(Balances::balance(&pot), 0);

		assert_noop!(
			Pallet::<Test>::refund_after_deadline(RuntimeOrigin::signed(1), 1, tid,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn majority_reject_refunds_payer() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let payee = 2u64;
		let amount = 50u64;
		let tid = task_id(b"job-2");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			payee,
			amount,
			[1u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4, 5]),
		));
		assert_eq!(Balances::balance(&1), 1_000 - amount);

		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(3),
			0,
			tid.clone(),
			false,
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(4),
			0,
			tid.clone(),
			false,
		));

		assert!(!Escrows::<Test>::contains_key((0u16, tid.clone())));
		assert_eq!(Balances::balance(&1), 1_000);
		assert_eq!(Balances::balance(&pot), 0);

		assert_noop!(
			Pallet::<Test>::refund_after_deadline(RuntimeOrigin::signed(1), 0, tid,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn non_resolver_cannot_vote() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"x");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4]),
		));
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(
				RuntimeOrigin::signed(2),
				0,
				tid.clone(),
				true,
			),
			Error::<Test>::NotResolver
		);
	});
}

#[test]
fn double_vote_fails() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"y");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4, 5]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(3),
			0,
			tid.clone(),
			true,
		));
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(
				RuntimeOrigin::signed(3),
				0,
				tid.clone(),
				false,
			),
			Error::<Test>::AlreadyVoted
		);
	});
}

#[test]
fn empty_resolvers_rejected() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"z");
		let empty: BoundedVec<u64, MaxRes> = BoundedVec::default();
		assert_noop!(
			Pallet::<Test>::fund_escrow(
				RuntimeOrigin::signed(1),
				0,
				tid,
				2,
				10,
				[0u8; 32],
				FAR_DEADLINE,
				empty,
			),
			Error::<Test>::EmptyResolverSet
		);
	});
}

#[test]
fn duplicate_resolver_rejected() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"d");
		assert_noop!(
			Pallet::<Test>::fund_escrow(
				RuntimeOrigin::signed(1),
				0,
				tid,
				2,
				10,
				[0u8; 32],
				FAR_DEADLINE,
				resolvers(&[3, 3]),
			),
			Error::<Test>::DuplicateResolver
		);
	});
}

#[test]
fn deadline_in_past_rejected() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"past");
		assert_noop!(
			Pallet::<Test>::fund_escrow(
				RuntimeOrigin::signed(1),
				0,
				tid,
				2,
				10,
				[0u8; 32],
				1u64,
				resolvers(&[3, 4]),
			),
			Error::<Test>::DeadlineInPast
		);
	});
}

#[test]
fn refund_after_deadline_returns_to_payer() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"timeout");
		let amount = 40u64;
		let deadline = 5u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[9u8; 32],
			deadline,
			resolvers(&[3, 4, 5]),
		));
		assert_eq!(Balances::balance(&1), 1_000 - amount);

		System::set_block_number(deadline);
		assert_noop!(
			Pallet::<Test>::refund_after_deadline(RuntimeOrigin::signed(1), 0, tid.clone(),),
			Error::<Test>::DeadlineNotPassed
		);

		System::set_block_number(deadline + 1);
		assert_ok!(Pallet::<Test>::refund_after_deadline(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
		));
		assert_eq!(Balances::balance(&1), 1_000);
		assert_eq!(Balances::balance(&pot), 0);
		assert!(!Escrows::<Test>::contains_key((0u16, tid.clone())));
		let key = escrow_key(0, &tid);
		assert!(Votes::<Test>::get(&key, &3u64).is_none());
	});
}

#[test]
fn vote_on_escrow_rejected_after_deadline() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"late-vote");
		let deadline = 5u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			20,
			[1u8; 32],
			deadline,
			resolvers(&[3, 4, 5]),
		));
		System::set_block_number(deadline + 1);
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(RuntimeOrigin::signed(3), 0, tid.clone(), true,),
			Error::<Test>::VotingAfterDeadline
		);
	});
}

#[test]
fn vote_on_escrow_ok_at_deadline_block() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"edge-deadline");
		let deadline = 5u64;
		let only_r = 3u64;
		let amount = 15u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[2u8; 32],
			deadline,
			resolvers(&[only_r]),
		));
		System::set_block_number(deadline);
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(only_r),
			0,
			tid.clone(),
			true,
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
		assert_eq!(Balances::balance(&2), 1_000 + amount);
		assert_eq!(Balances::balance(&pot), 0);
	});
}

#[test]
fn refund_after_deadline_succeeds_when_vote_blocked_after_deadline() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"race-refund");
		let amount = 30u64;
		let deadline = 5u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[3u8; 32],
			deadline,
			resolvers(&[3, 4]),
		));
		System::set_block_number(deadline + 1);
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(RuntimeOrigin::signed(3), 0, tid.clone(), true,),
			Error::<Test>::VotingAfterDeadline
		);
		assert_ok!(Pallet::<Test>::refund_after_deadline(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
	});
}

#[test]
fn only_payer_may_refund_after_deadline() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"np");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			2u64,
			resolvers(&[3, 4]),
		));
		System::set_block_number(3);
		assert_noop!(
			Pallet::<Test>::refund_after_deadline(RuntimeOrigin::signed(2), 0, tid,),
			Error::<Test>::NotPayer
		);
	});
}

/// При **ровно одном** резолвере строгий кворум `2*1>1` — первый же голос завершает эскроу
/// (плательщик номинировал комитет из одного лица = «этот аккаунт решает один»).
#[test]
fn single_resolver_approve_releases_on_first_vote() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"mono-approve");
		let only_r = 3u64;
		let amount = 25u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[2u8; 32],
			FAR_DEADLINE,
			resolvers(&[only_r]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(only_r),
			0,
			tid.clone(),
			true,
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
		assert_eq!(Balances::balance(&2), 1_000 + amount);
		assert_eq!(Balances::balance(&pot), 0);
	});
}

#[test]
fn single_resolver_reject_refunds_on_first_vote() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"mono-reject");
		let only_r = 4u64;
		let amount = 30u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[3u8; 32],
			FAR_DEADLINE,
			resolvers(&[only_r]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(only_r),
			0,
			tid.clone(),
			false,
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
		assert_eq!(Balances::balance(&1), 1_000);
		assert_eq!(Balances::balance(&pot), 0);
	});
}

/// При N>1 **один** голос approve **не** даёт кворума (это не «любой один из N решает»).
#[test]
fn single_approve_insufficient_when_five_resolvers() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"five-one-vote");
		let amount = 60u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[4u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4, 5, 6, 7]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(5),
			0,
			tid.clone(),
			true,
		));
		let rec = Escrows::<Test>::get(escrow_key(0, &tid)).unwrap();
		assert_eq!(rec.status, EscrowStatus::AwaitingRelease);
		assert_eq!(Balances::balance(&pot), amount);
		let key = escrow_key(0, &tid);
		assert_eq!(Votes::<Test>::get(&key, &5u64), Some(Ballot::Approve));
	});
}

/// Кворум достигается любым подмножеством строгого большинства — здесь решающий голос от «пятого».
#[test]
fn five_resolvers_majority_on_third_approve() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"five-third");
		let amount = 20u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[5u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4, 5, 6, 7]),
		));
		for r in [3u64, 4u64, 7u64] {
			assert_ok!(Pallet::<Test>::vote_on_escrow(
				RuntimeOrigin::signed(r),
				0,
				tid.clone(),
				true,
			));
		}
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
		assert_eq!(Balances::balance(&2), 1_000 + amount);
	});
}

/// n=2: нужны **оба** approve (`2*2>2`), один голос оставляет эскроу открытым.
#[test]
fn two_resolvers_first_vote_does_not_settle() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"two-pending");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			15,
			[6u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(3),
			0,
			tid.clone(),
			true,
		));
		assert_eq!(
			Escrows::<Test>::get(escrow_key(0, &tid)).unwrap().status,
			EscrowStatus::AwaitingRelease
		);
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(4),
			0,
			tid.clone(),
			true,
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
	});
}

/// 2 approve vs 2 reject при 4 резолверах — нет строгого большинства ни с одной стороны; после дедлайна payer.
#[test]
fn four_resolvers_stalemate_two_two_then_refund_after_deadline() {
	new_test_ext().execute_with(|| {
		let pot = Pallet::<Test>::pallet_account();
		let tid = task_id(b"stale-4");
		let amount = 35u64;
		let deadline = 20u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			amount,
			[8u8; 32],
			deadline,
			resolvers(&[3, 4, 5, 6]),
		));
		for (r, approve) in [(3u64, true), (4u64, true), (5u64, false), (6u64, false)] {
			assert_ok!(Pallet::<Test>::vote_on_escrow(
				RuntimeOrigin::signed(r),
				0,
				tid.clone(),
				approve,
			));
		}
		assert_eq!(
			Escrows::<Test>::get(escrow_key(0, &tid)).unwrap().status,
			EscrowStatus::AwaitingRelease
		);
		assert_eq!(Balances::balance(&pot), amount);

		System::set_block_number(deadline + 1);
		assert_ok!(Pallet::<Test>::refund_after_deadline(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
		));
		assert_eq!(Balances::balance(&1), 1_000);
		assert_eq!(Balances::balance(&pot), 0);
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
	});
}

#[test]
fn fund_same_task_id_after_release_allowed() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"reuse-key");
		let only_r = 3u64;
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[only_r]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(only_r),
			0,
			tid.clone(),
			true,
		));
		assert!(!Escrows::<Test>::contains_key(escrow_key(0, &tid)));
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			5,
			[1u8; 32],
			FAR_DEADLINE,
			resolvers(&[4]),
		));
		assert!(Escrows::<Test>::contains_key(escrow_key(0, &tid)));
	});
}

#[test]
fn fund_twice_same_key_fails() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"dup-key");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3]),
		));
		assert_noop!(
			Pallet::<Test>::fund_escrow(
				RuntimeOrigin::signed(1),
				0,
				tid,
				2,
				5,
				[1u8; 32],
				FAR_DEADLINE,
				resolvers(&[4]),
			),
			Error::<Test>::AlreadyExists
		);
	});
}

#[test]
fn zero_amount_rejected() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"zero");
		assert_noop!(
			Pallet::<Test>::fund_escrow(
				RuntimeOrigin::signed(1),
				0,
				tid,
				2,
				0,
				[0u8; 32],
				FAR_DEADLINE,
				resolvers(&[3]),
			),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn vote_on_missing_escrow_fails() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"ghost");
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(RuntimeOrigin::signed(3), 0, tid, true,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn refund_after_deadline_missing_escrow_fails() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"ghost2");
		assert_noop!(
			Pallet::<Test>::refund_after_deadline(RuntimeOrigin::signed(1), 0, tid,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn vote_after_released_fails_not_found() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"post-rel");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(3),
			0,
			tid.clone(),
			true,
		));
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(RuntimeOrigin::signed(3), 0, tid.clone(), false,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn vote_after_reject_refund_fails_not_found() {
	new_test_ext().execute_with(|| {
		let tid = task_id(b"post-rej");
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3, 4]),
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(3),
			0,
			tid.clone(),
			false,
		));
		assert_ok!(Pallet::<Test>::vote_on_escrow(
			RuntimeOrigin::signed(4),
			0,
			tid.clone(),
			false,
		));
		assert_noop!(
			Pallet::<Test>::vote_on_escrow(RuntimeOrigin::signed(3), 0, tid, true,),
			Error::<Test>::NotFound
		);
	});
}

#[test]
fn fund_escrow_accepts_max_len_task_id() {
	new_test_ext().execute_with(|| {
		let tid = task_id(&[b'm'; 32]);
		assert_eq!(tid.len(), 32);
		assert_ok!(Pallet::<Test>::fund_escrow(
			RuntimeOrigin::signed(1),
			0,
			tid.clone(),
			2,
			10,
			[0u8; 32],
			FAR_DEADLINE,
			resolvers(&[3]),
		));
		assert!(Escrows::<Test>::contains_key(escrow_key(0, &tid)));
	});
}
