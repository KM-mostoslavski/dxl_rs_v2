//! Proves that exchanges never allocate.
//!
//! Installs a counting global allocator for the test binary. The counter is
//! per thread, so tests running in parallel do not disturb each other.

extern crate std;

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::time::Duration;

use std::alloc::System;

use super::vectors::{p1 as v1, p2 as v2};
use super::{Exchange, Link, P1, P2};
use crate::Id;
use crate::transport::{MockTime, MockTransport};

struct Counting;

std::thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: forwards every call to `System` unchanged; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

fn id(raw: u8) -> Id {
    Id::new(raw).unwrap()
}

#[test]
fn exchanges_never_allocate() {
    let time = MockTime::new();
    let mut clock = time.ticker(1);
    let mut p2_line = MockTransport::new(&time);
    p2_line
        .reply_to(1, v2::PING_REPLY)
        .reply_to(2, v2::READ_REPLY_1)
        .reply_to(3, v2::OK_REPLY)
        .reply_to(5, v2::READ_REPLY_1)
        .reply_to(5, v2::READ_REPLY_2)
        .reply_to(6, v2::PING_REPLY)
        .reply_to(6, v2::BROADCAST_PING_REPLY_2)
        .reply_to(7, v2::PING_REPLY);
    let mut p1_line = MockTransport::new(&time);
    p1_line
        .reply_to(1, v1::OK_REPLY)
        .reply_to(2, v1::READ_REPLY)
        .reply_to(3, v1::OK_REPLY)
        .reply_to(5, v1::READ_REPLY);
    let data = [0u8; 4];
    let mut out = [0u8; 4];
    let mut scratch = [0u8; 4];
    let mut replies = 0;

    let before = allocations();
    {
        let mut link = Link {
            transport: &mut p2_line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
        };
        P2::ping(&mut link, id(1)).unwrap();
        P2::read(&mut link, id(1), 132, &mut out).unwrap();
        P2::write(&mut link, id(1), 116, &data).unwrap();
        P2::sync_write(&mut link, 116, 4, [(id(1), &data[..])].into_iter()).unwrap();
        P2::sync_read(
            &mut link,
            132,
            4,
            [id(1), id(2)].into_iter(),
            &mut scratch,
            |_, r| replies += usize::from(r.is_ok()),
        )
        .unwrap();
        P2::broadcast_ping(&mut link, |_| replies += 1).unwrap();
        P2::scan(&mut link, |r| replies += usize::from(r.is_ok())).unwrap();
    }
    {
        let mut link = Link {
            transport: &mut p1_line,
            clock: &mut clock,
            timeout: Duration::from_millis(1),
        };
        P1::ping(&mut link, id(1)).unwrap();
        P1::read(&mut link, id(1), 43, &mut out[..1]).unwrap();
        P1::write(&mut link, id(1), 12, &data[..2]).unwrap();
        P1::sync_write(&mut link, 30, 4, [(id(1), &data[..])].into_iter()).unwrap();
        P1::read_many(
            &mut link,
            43,
            1,
            [id(1)].into_iter(),
            &mut scratch,
            |_, r| replies += usize::from(r.is_ok()),
        )
        .unwrap();
    }
    let after = allocations();

    assert_eq!(replies, 6);
    assert_eq!(after - before, 0, "exchanges allocated");
}

#[test]
fn the_counter_sees_allocations() {
    let before = allocations();
    let v = std::vec![1u8; 16];
    assert!(allocations() > before);
    drop(v);
}
