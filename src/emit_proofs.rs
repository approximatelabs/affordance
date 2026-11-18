//! Kani proofs for the drop kernel `drop_cut` (run with `cargo kani`; ordinary builds ignore
//! this module). Every harness covers all inputs within the bounds: up to `MAX_ITEMS` items,
//! boundaries strictly increasing from 0 with the content length at most `MAX_LEN`, a marker of at
//! most `MAX_LEN` tokens, any `lim`, and every step up to `MAX_STEP` (0 is "no step").

use super::{Drop, drop_cut, rebase};

const MAX_ITEMS: usize = 8;
const MAX_LEN: usize = 64;
const MAX_STEP: usize = 2 * MAX_LEN + 2; // past the largest possible `need`, so every regime is covered

/// Item boundaries: `len` entries (`0..=MAX_ITEMS` items) in a fixed array, strictly increasing
/// from 0, the last at most `MAX_LEN`.
fn any_bounds() -> ([usize; MAX_ITEMS + 1], usize) {
    let b: [usize; MAX_ITEMS + 1] = kani::any();
    let len: usize = kani::any();
    kani::assume(len >= 1 && len <= MAX_ITEMS + 1);
    kani::assume(b[0] == 0);
    let mut i = 1;
    while i < len {
        kani::assume(b[i - 1] < b[i]);
        i += 1;
    }
    kani::assume(b[len - 1] <= MAX_LEN);
    (b, len)
}

fn any_drop() -> Drop {
    match kani::any::<u8>() % 3 {
        0 => Drop::Start,
        1 => Drop::End,
        _ => Drop::Children,
    }
}

fn any_more() -> usize {
    let m: usize = kani::any();
    kani::assume(m <= MAX_LEN);
    m
}

fn any_step() -> usize {
    let s: usize = kani::any();
    kani::assume(s <= MAX_STEP);
    s
}

/// The minimum number of items a mode keeps (bounded by how many there are).
fn min_kept(drop: Drop, items: usize) -> usize {
    (if drop == Drop::Children { 0 } else { 1 }).min(items)
}

/// The amount a step rounds the overflow up to: the smallest multiple of `step` at or past it.
fn stepped(need: usize, step: usize) -> usize {
    if step == 0 { need } else { need.div_ceil(step) * step }
}

/// The kernel before it was closed-form (f113693): drop one item at a time from the drop side
/// while the kept items plus the marker do not fit and more than the minimum is left.
fn reference(bounds: &[usize], lim: usize, more_len: usize, drop: Drop) -> (usize, usize) {
    let (mut lo, mut hi) = (0usize, bounds.len() - 1);
    let min_items = if drop == Drop::Children { 0 } else { 1 };
    while hi - lo > min_items && bounds[hi] - bounds[lo] + more_len > lim {
        if matches!(drop, Drop::Start | Drop::Children) {
            lo += 1
        } else {
            hi -= 1
        }
    }
    (lo, hi)
}

/// In range, at least the minimum kept, the kept side right, and it fits unless only the minimum
/// is left. Every mode, every step (stepping drops more, never less, so the fit still holds).
#[kani::proof]
#[kani::unwind(11)]
fn cut_is_in_range_on_the_kept_side_and_fits() {
    let (b, len) = any_bounds();
    let bounds = &b[..len];
    let (lim, more_len, drop, step) = (kani::any::<usize>(), any_more(), any_drop(), any_step());
    let (lo, hi) = drop_cut(bounds, lim, more_len, drop, step);
    let last = len - 1;
    let min = min_kept(drop, last);
    assert!(lo <= hi && hi < len);
    assert!(hi - lo >= min);
    match drop {
        Drop::End => assert!(lo == 0),
        _ => assert!(hi == last),
    }
    assert!(bounds[hi] - bounds[lo] + more_len <= lim || hi - lo == min);
}

/// Unstepped maximality: keeping one more item (the next one on the drop side) would not fit.
#[kani::proof]
#[kani::unwind(11)]
fn unstepped_cut_is_maximal() {
    let (b, len) = any_bounds();
    let bounds = &b[..len];
    let (lim, more_len, drop) = (kani::any::<usize>(), any_more(), any_drop());
    let (lo, hi) = drop_cut(bounds, lim, more_len, drop, 0);
    let fits = |lo: usize, hi: usize| bounds[hi] - bounds[lo] + more_len <= lim;
    if drop == Drop::End && hi < len - 1 {
        assert!(!fits(lo, hi + 1));
    }
    if drop != Drop::End && lo > 0 {
        assert!(!fits(lo - 1, hi));
    }
}

/// With no step the closed form is exactly the old one-item-at-a-time loop.
#[kani::proof]
#[kani::unwind(11)]
fn unstepped_cut_equals_the_reference_loop() {
    let (b, len) = any_bounds();
    let bounds = &b[..len];
    let (lim, more_len, drop) = (kani::any::<usize>(), any_more(), any_drop());
    assert!(drop_cut(bounds, lim, more_len, drop, 0) == reference(bounds, lim, more_len, drop));
}

/// The stepped amount is the smallest multiple of the step at or past the overflow.
#[kani::proof]
fn stepped_amount_is_the_least_multiple_at_or_past_the_overflow() {
    let (need, step) = (kani::any::<usize>(), any_step());
    kani::assume(need <= 2 * MAX_LEN && step > 0);
    let s = stepped(need, step);
    assert!(s % step == 0 && s >= need && s < need + step);
}

/// With a step (and without: step 0 is the unit case), the cut is the first boundary at or past
/// the stepped amount, measured from the drop side, unless the item minimum caps it.
/// drop-start / drop-children: bounds[lo] >= need and bounds[lo - 1] < need, or lo is the cap.
/// drop-end, mirrored: n - bounds[hi] >= need and n - bounds[hi + 1] < need, or hi is the cap.
#[kani::proof]
#[kani::unwind(11)]
fn stepped_cut_is_the_first_boundary_past_the_stepped_amount() {
    let (b, len) = any_bounds();
    let bounds = &b[..len];
    let (lim, more_len, drop, step) = (kani::any::<usize>(), any_more(), any_drop(), any_step());
    let (lo, hi) = drop_cut(bounds, lim, more_len, drop, step);
    let last = len - 1;
    let n = bounds[last];
    let min = min_kept(drop, last);
    let need = stepped((n + more_len).saturating_sub(lim), step);
    if drop == Drop::End {
        assert!(hi == last || n - bounds[hi + 1] < need);
        assert!(n - bounds[hi] >= need || hi == min);
    } else {
        assert!(lo == 0 || bounds[lo - 1] < need);
        assert!(bounds[lo] >= need || lo == last - min);
    }
}

/// Stability as content grows on the far side: if `bounds2` extends `bounds` (same prefix, more
/// items appended), the drop-start / drop-children cut never moves back: bounds2[lo2] >= bounds[lo].
#[kani::proof]
#[kani::unwind(11)]
fn cut_only_moves_forward_as_items_are_appended() {
    let (b2, len2) = any_bounds();
    let len: usize = kani::any();
    kani::assume(len >= 1 && len <= len2);
    let (bounds, bounds2) = (&b2[..len], &b2[..len2]);
    let (lim, more_len, step) = (kani::any::<usize>(), any_more(), any_step());
    let drop = if kani::any() { Drop::Start } else { Drop::Children };
    let (lo, _) = drop_cut(bounds, lim, more_len, drop, step);
    let (lo2, _) = drop_cut(bounds2, lim, more_len, drop, step);
    assert!(bounds2[lo2] >= bounds[lo]);
}

/// The drop-end mirror: if `bounds2` is `bounds` with items prepended (the content grows at the
/// start, the far side for drop-end), the amount dropped from the end never shrinks.
#[kani::proof]
#[kani::unwind(11)]
fn drop_end_cut_only_moves_back_as_items_are_prepended() {
    let (b2, len2) = any_bounds();
    let len: usize = kani::any();
    kani::assume(len >= 1 && len <= len2);
    let k = len2 - len; // items prepended
    let d = b2[k];
    let mut b = [0usize; MAX_ITEMS + 1];
    let mut i = 0;
    while i < len {
        b[i] = b2[k + i] - d;
        i += 1;
    }
    let (bounds, bounds2) = (&b[..len], &b2[..len2]);
    let (lim, more_len, step) = (kani::any::<usize>(), any_more(), any_step());
    let (_, hi) = drop_cut(bounds, lim, more_len, Drop::End, step);
    let (_, hi2) = drop_cut(bounds2, lim, more_len, Drop::End, step);
    assert!(bounds2[len2 - 1] - bounds2[hi2] >= bounds[len - 1] - bounds[hi]);
}

/// Kept ranges as a box builds them: up to three, in content order, not overlapping, each
/// placed in the output after the ones before it (with room for a marker between).
fn any_ranges() -> ([(usize, usize, usize); 3], usize) {
    let r: [(usize, usize, usize); 3] = kani::any();
    let n: usize = kani::any();
    kani::assume(n <= 3);
    let (mut prev_end, mut out_end) = (0usize, 0usize);
    let mut i = 0;
    while i < n {
        let (a, b, off) = r[i];
        kani::assume(a >= prev_end && a < b && b <= MAX_LEN);
        kani::assume(off >= out_end && off <= 2 * MAX_LEN);
        prev_end = b;
        out_end = off + (b - a);
        i += 1;
    }
    (r, n)
}

/// `rebase` never panics; a shown span lands inside the output, with start <= end, inside the
/// first range that shows it and the last; `whole` holds exactly when one range holds the span.
#[kani::proof]
#[kani::unwind(5)]
fn rebased_spans_land_in_the_output() {
    let (r, n) = any_ranges();
    let ranges = &r[..n];
    let (s, e): (usize, usize) = (kani::any(), kani::any());
    kani::assume(s <= e && e <= MAX_LEN);
    let point = s == e;
    let shows = |&(a, b, _): &(usize, usize, usize)| if point { a <= s && s < b } else { s < b && e > a };
    match rebase(ranges, s, e) {
        None => assert!(!ranges.iter().any(shows)),
        Some(x) => {
            assert!(x.start <= x.end);
            let first = ranges.iter().find(|q| shows(q)).unwrap();
            let last = ranges.iter().rev().find(|q| shows(q)).unwrap();
            assert!(x.start >= first.2 && x.start <= first.2 + (first.1 - first.0));
            assert!(x.end >= last.2 && x.end <= last.2 + (last.1 - last.0));
            let held = ranges
                .iter()
                .any(|&(a, b, _)| if point { a <= s && s < b } else { a <= s && e <= b });
            assert_eq!(x.whole, held);
            // A span one range holds whole keeps its length exactly.
            if x.whole && !point {
                assert_eq!(x.end - x.start, e - s);
            }
        }
    }
}
