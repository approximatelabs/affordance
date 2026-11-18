//! Fitting a box's content to its token limit: which ranges of the content a box shows, how
//! much room its `more` marker gets, and the tokens it emits. Layout (`emit.rs`) calls this for
//! every box it closes.
//!
//! This file is checked by Verus (`docs/VERIFICATION.md`; `tools/verify.sh` runs it). It is
//! plain Rust: the specifications are attributes that exist only when Verus reads the file
//! (`cfg(verus_keep_ghost)`), so an ordinary build compiles the same functions with no
//! verifier, macro or dependency involved. The file stands alone (it uses nothing from the rest
//! of the crate) because Verus reads it as a crate of its own.
//!
//! What is proved, for every input (any content, any marker, any limit, any overflow policy):
//!
//! - A box with a limit emits at most that many tokens (`a_box_emits_at_most_its_limit`, at
//!   the end of the file). There is no exception.
//! - The tokens a box emits are the shown ranges of its content, the marker cut to its reserved
//!   room, and padding, in the box's order (`Fit::assemble`).
//! - A drop box whose items are measured as they are shows the items it keeps uncut, unless
//!   only the minimum is left (`Room::drop`).
//! - No index is out of range and no arithmetic overflows, given each function's stated
//!   precondition (only `drop_cut` and `Room::drop` have one).
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]
#![cfg_attr(verus_keep_ghost, allow(unused_imports))]

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

/// Proof steps are for Verus alone.
#[cfg(not(verus_keep_ghost))]
macro_rules! proof {
    ($($t:tt)*) => {};
}

/// Which part of an overflowing (non-drop) box is shown.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Clip {
    End,
    Start,
    Middle,
    Elide,
}

/// `overflow: drop-start | drop-end`: drop whole items (segments) instead of cutting tokens.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Drop {
    None,
    Start,
    End,
    /// Drop oldest whole children, including the last oversized child. Never clip it.
    Children,
}

/// The overflow rounded up to a multiple of `step` (`drop-step`; 0 for none).
#[cfg_attr(verus_keep_ghost, verus_spec(r =>
    ensures
        r >= need,
        step == 0 ==> r == need,
        // The least multiple of the step at or past the need (unless that is past `usize`).
        step > 0 && need + step <= usize::MAX ==> r % step == 0 && r < need + step,
))]
fn stepped(need: usize, step: usize) -> usize {
    if step == 0 {
        return need;
    }
    // `need.div_ceil(step) * step`.
    let whole = need / step;
    proof! {
        assert(need == step * whole + need % step && need % step < step && (step >= 2 ==> 2 * whole <= need))
            by (nonlinear_arith)
            requires step > 0, whole == need / step;
    }
    let rest = need % step;
    let q = if rest > 0 { whole + 1 } else { whole };
    proof! {
        assert(need <= q * step < need + step) by (nonlinear_arith)
            requires
                step > 0,
                need == step * whole + need % step,
                need % step < step,
                q == (if need % step != 0 { whole + 1 } else { whole as int });
        vstd::arithmetic::div_mod::lemma_mod_multiples_basic(q as int, step as int);
    }
    q.saturating_mul(step)
}

/// The drop kernel: which items of an overflowing drop box stay. `bounds` are the item
/// boundaries (token offsets, strictly increasing, from 0 to the content's length); the result
/// `(lo, hi)` keeps the items between `bounds[lo]` and `bounds[hi]`. Whole items are dropped
/// from the box's drop side until the kept items plus the `more` marker (`more_len` tokens) fit
/// in `lim`; `drop-start` and `drop-end` keep at least one item (it is clipped later if it alone
/// is too big), `drop-children` may keep none.
///
/// With a `step` (`drop-step`, in tokens; 0 for none) the amount dropped is rounded up to a
/// multiple of it: the cut is the first item boundary at or past the smallest multiple of
/// `step` that makes the rest fit. Content that only grows on the far side then moves the cut
/// once per `step` tokens, not once per item (stable overflow).
#[cfg_attr(verus_keep_ghost, verus_spec(r =>
    requires
        bounds.len() >= 1,
    ensures
        r.0 <= r.1,
        r.1 < bounds.len(),
        cut_spec(bounds@, lim, more_len, drop, r.0, r.1),
))]
pub(crate) fn drop_cut(bounds: &[usize], lim: usize, more_len: usize, drop: Drop, step: usize) -> (usize, usize) {
    let last = bounds.len() - 1;
    let n = bounds[last];
    let min_items: usize = if matches!(drop, Drop::Children) { 0 } else { 1 };
    let min_kept = if min_items < last { min_items } else { last };
    let need = stepped(n.saturating_add(more_len).saturating_sub(lim), step);
    if matches!(drop, Drop::End) {
        // Keep the longest prefix leaving out at least `need` tokens.
        let mut hi = last;
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                hi <= last,
                last == bounds.len() - 1,
                n == bounds[last as int],
                forall|j: int| hi < j <= last ==> n - bounds[j] < need,
            decreases hi
        ))]
        while hi > 0 && n.saturating_sub(bounds[hi]) < need {
            hi -= 1;
        }
        (0, if hi > min_kept { hi } else { min_kept })
    } else {
        let mut lo = 0;
        #[cfg_attr(verus_keep_ghost, verus_spec(
            invariant
                lo <= last,
                last == bounds.len() - 1,
                forall|j: int| 0 <= j < lo ==> bounds[j] < need,
            decreases last - lo
        ))]
        while lo < last && bounds[lo] < need {
            lo += 1;
        }
        let cap = last - min_kept;
        (if lo < cap { lo } else { cap }, last)
    }
}

/// A closed box's content and its limit, before layout decides what to show.
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub(crate) struct Room<'a> {
    content: &'a [u32],
    /// The length overflow is decided on: the content's, or for a drop box its undegraded one.
    measured: usize,
    lim: Option<usize>,
    /// A fixed size (`tok-size`): the limit, and what padding fills up to.
    size: Option<usize>,
    scroll: bool,
}

/// The order of what a box emits.
#[cfg_attr(verus_keep_ghost, verus_verify)]
#[derive(Clone, Copy)]
enum Shape {
    /// The content, all of it.
    Whole,
    /// A range, then the marker.
    Head,
    /// The marker, then a range.
    Tail,
    /// A range, the marker, a second range.
    Middle,
    /// The marker alone.
    Marker,
}

/// What a box shows: ranges of its content and the room reserved for its marker.
#[cfg_attr(verus_keep_ghost, verus_verify)]
pub(crate) struct Fit<'a> {
    content: &'a [u32],
    /// A fixed size (`tok-size`), which padding fills up to.
    size: Option<usize>,
    /// The marker's room: `more` is cut to this many tokens.
    reserve: usize,
    shape: Shape,
    first: (usize, usize),
    second: (usize, usize),
}

/// A scroll box's viewport: `(offset, largest offset, tokens of content it can show)`.
pub(crate) type Viewport = (usize, usize, usize);

/// A closed box's room. `tok_size` is the box's fixed size and `max_tokens` its cap (already in
/// tokens); a size wins. `measured` is the length a drop box measures (`content.len()` for any
/// other box); `scroll`: the box is a scroll viewport.
#[cfg_attr(verus_keep_ghost, verus_spec(r =>
    ensures
        r.wf(),
        r.content() == content@,
        r.limit() == limit_spec(tok_size, max_tokens),
        r.over_spec() == over_spec(measured, content@.len(), limit_spec(tok_size, max_tokens), scroll),
))]
pub(crate) fn room<'a>(
    content: &'a [u32],
    measured: usize,
    tok_size: Option<usize>,
    max_tokens: Option<usize>,
    scroll: bool,
) -> Room<'a> {
    let lim = match tok_size {
        Some(t) => Some(t),
        None => max_tokens,
    };
    Room {
        content,
        measured,
        lim,
        size: tok_size,
        scroll,
    }
}

impl<'a> Room<'a> {
    /// The limit, when the box must make room for its marker: it scrolls, or its measured
    /// length or its content is over the limit. (A drop box measures graded results at their
    /// static length, so the two differ; a sheet that makes old results larger than new ones
    /// puts the content over while the measure is under.) None: the box is laid out whole.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r == self.over_spec(),
    ))]
    pub(crate) fn over(&self) -> Option<usize> {
        match self.lim {
            Some(lim) if self.scroll || self.measured > lim || self.content.len() > lim => Some(lim),
            _ => None,
        }
    }

    /// All of the content, no marker.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r.wf(),
            r.content() == self.content(),
            r.shown() == self.content().len(),
            r.most() >= self.content().len(),
            self.limit() matches Some(lim) ==> (r.most() <= lim <==> self.content().len() <= lim),
            self.limit() matches Some(lim) && self.content().len() > lim ==> r.most() == self.content().len(),
    ))]
    pub(crate) fn whole(self) -> Fit<'a> {
        let n = self.content.len();
        Fit {
            content: self.content,
            size: self.size,
            reserve: 0,
            shape: Shape::Whole,
            first: (0, n),
            second: (n, n),
        }
    }

    /// `overflow: scroll`: a viewport of the content at `offset`, the marker after it.
    /// `reserve` is the marker's length at its longest.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r.0.wf(),
            r.0.content() == self.content(),
            self.limit() matches Some(lim) ==> r.0.most() <= lim,
    ))]
    pub(crate) fn scroll(self, reserve: usize, offset: usize) -> (Fit<'a>, Option<Viewport>) {
        let n = self.content.len();
        let Some(lim) = self.lim else {
            return (self.whole(), None);
        };
        let reserve = if reserve < lim { reserve } else { lim };
        let over = n > lim;
        let capacity = if over { lim - reserve } else { lim };
        let max = n.saturating_sub(capacity);
        let offset = if offset < max { offset } else { max };
        let end = if n - offset > capacity { offset + capacity } else { n };
        let fit = Fit {
            content: self.content,
            size: self.size,
            // Content that fits hides nothing: no marker.
            reserve: if over { reserve } else { 0 },
            shape: Shape::Head,
            first: (offset, end),
            second: (n, n),
        };
        (fit, Some((offset, max, capacity)))
    }

    /// A box that cuts tokens (`clip-end`, `clip-start`, `clip-middle`, `elide`): what is kept
    /// of content over the limit, with the marker at the cut. `reserve` is the marker's length
    /// at its longest.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r.wf(),
            r.content() == self.content(),
            self.limit() matches Some(lim) ==> r.most() <= lim,
    ))]
    pub(crate) fn clip(self, reserve: usize, clip: Clip) -> Fit<'a> {
        let n = self.content.len();
        let lim = match self.lim {
            Some(lim) if n > lim => lim,
            _ => return self.whole(),
        };
        let reserve = if reserve < lim { reserve } else { lim };
        let keep = lim - reserve;
        let (shape, first, second) = match clip {
            Clip::End => (Shape::Head, (0, keep), (n, n)),
            Clip::Start => (Shape::Tail, (n - keep, n), (n, n)),
            // Head and tail: the head gets the odd token.
            Clip::Middle => (Shape::Middle, (0, keep - keep / 2), (n - keep / 2, n)),
            // The marker alone.
            Clip::Elide => (Shape::Marker, (0, 0), (n, n)),
        };
        Fit {
            content: self.content,
            size: self.size,
            reserve,
            shape,
            first,
            second,
        }
    }

    /// A box that drops whole items (`drop-start`, `drop-end`, `drop-children`) and is over its
    /// limit. `offsets` are the item boundaries in the content (from 0 to its length, never
    /// decreasing) and `measured` the same boundaries as the box measures them; the cut is
    /// chosen on the measured ones (`drop_cut`) and returned. What is kept is clipped to the
    /// limit if it is still too long (the one item a `drop-start` box always keeps).
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
            self.over_spec() is Some,
            offsets.len() == measured.len(),
            boundaries(offsets@, self.content().len()),
        ensures
            r.0.wf(),
            r.0.content() == self.content(),
            self.limit() matches Some(lim) ==> r.0.most() <= lim,
            r.1 <= r.2,
            r.2 < offsets.len(),
            r.0.first_shown() == offsets[r.1 as int],
            // Whole items (claim 2): measured as they are, the kept items are shown uncut,
            // unless only the minimum is left.
            measured@ == offsets@ && self.content().len() + self.limit()->0 <= usize::MAX ==> {
                ||| r.0.shown() == offsets[r.2 as int] - offsets[r.1 as int]
                ||| r.2 - r.1 == min_kept(drop, offsets.len() - 1) && !(drop is Children)
            },
    ))]
    pub(crate) fn drop(
        self,
        offsets: &[usize],
        measured: &[usize],
        reserve: usize,
        drop: Drop,
        step: usize,
    ) -> (Fit<'a>, usize, usize) {
        let n = self.content.len();
        // Layout calls this for a box over its limit: it has one.
        let lim = self.lim.unwrap_or(0);
        let reserve = if reserve < lim { reserve } else { lim };
        let keep = lim - reserve;
        let (lo, hi) = drop_cut(measured, lim, reserve, drop, step);
        let a = offsets[lo];
        let b = if offsets[hi] - a > keep { a + keep } else { offsets[hi] };
        let fit = Fit {
            content: self.content,
            size: self.size,
            reserve,
            shape: if matches!(drop, Drop::End) {
                Shape::Head
            } else {
                Shape::Tail
            },
            first: (a, b),
            second: (n, n),
        };
        (fit, lo, hi)
    }
}

impl<'a> Fit<'a> {
    /// How many tokens of the content are shown.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r == self.shown(),
            r <= self.content().len(),
    ))]
    pub(crate) fn shown_tokens(&self) -> usize {
        match self.shape {
            Shape::Whole | Shape::Head | Shape::Tail => self.first.1 - self.first.0,
            Shape::Middle => (self.first.1 - self.first.0) + (self.second.1 - self.second.0),
            Shape::Marker => 0,
        }
    }

    /// The box's tokens: the shown ranges of the content with the marker `more` (cut to its
    /// reserved room) where the box puts it, then `pad` up to a fixed size. With them, each shown
    /// range as `(content start, content end, offset in the output)`.
    #[cfg_attr(verus_keep_ghost, verus_spec(r =>
        requires
            self.wf(),
        ensures
            r.0@ == self.output(more@, pad),
            r.0.len() <= self.most(),
            // Each reported range of the output is that range of the content.
            forall|i: int| 0 <= i < r.1.len() ==> shows(self.content(), r.0@, #[trigger] r.1[i]),
    ))]
    pub(crate) fn assemble(&self, more: &[u32], pad: Option<u32>) -> (Vec<u32>, Vec<(usize, usize, usize)>) {
        let m = if more.len() < self.reserve {
            more.len()
        } else {
            self.reserve
        };
        let marker = &more[..m];
        let (a, b) = self.first;
        let (c, d) = self.second;
        let mut out: Vec<u32> = Vec::with_capacity(self.shown_tokens() + m);
        // Each kept range: (content start, content end, output offset).
        let mut ranges: Vec<(usize, usize, usize)> = Vec::new();
        match self.shape {
            Shape::Whole => {
                ranges.push((a, b, out.len()));
                out.extend_from_slice(&self.content[a..b]);
            }
            Shape::Head => {
                ranges.push((a, b, out.len()));
                out.extend_from_slice(&self.content[a..b]);
                out.extend_from_slice(marker);
            }
            Shape::Tail => {
                out.extend_from_slice(marker);
                ranges.push((a, b, out.len()));
                out.extend_from_slice(&self.content[a..b]);
            }
            Shape::Middle => {
                ranges.push((a, b, out.len()));
                out.extend_from_slice(&self.content[a..b]);
                out.extend_from_slice(marker);
                ranges.push((c, d, out.len()));
                out.extend_from_slice(&self.content[c..d]);
            }
            Shape::Marker => out.extend_from_slice(marker),
        }
        proof! {
            assert(out@ == self.unpadded(more@));
            assert forall|i: int| 0 <= i < ranges.len() implies shows(self.content@, out@, #[trigger] ranges[i]) by {
                let (s, e, at) = ranges[i];
                assert(out@.subrange(at as int, at + (e - s)) =~= self.content@.subrange(s as int, e as int));
            }
        }
        if let (Some(size), Some(p)) = (self.size, pad) {
            #[cfg_attr(verus_keep_ghost, verus_spec(
                invariant
                    out@ == self.unpadded(more@) + Seq::new((out.len() - self.unpadded(more@).len()) as nat, |i: int| p),
                    out.len() >= self.unpadded(more@).len(),
                    out.len() <= size || out.len() == self.unpadded(more@).len(),
                    forall|i: int| 0 <= i < ranges.len() ==> shows(self.content@, self.unpadded(more@), #[trigger] ranges[i]),
                decreases size - out.len()
            ))]
            while out.len() < size {
                out.push(p);
            }
        }
        proof! {
            assert(out@.subrange(0, self.unpadded(more@).len() as int) =~= self.unpadded(more@));
            assert forall|i: int| 0 <= i < ranges.len() implies shows(self.content@, out@, #[trigger] ranges[i]) by {
                shows_in_prefix(self.content@, self.unpadded(more@), out@, ranges[i]);
            }
        }
        (out, ranges)
    }
}

#[cfg(verus_keep_ghost)]
verus! {

/// What layout knows about a drop box when it closes: its item boundaries in the content, the
/// same boundaries as measured (undegraded), the measured length, the mode and the step.
pub struct DropBox<'b> {
    pub offsets: &'b [usize],
    pub bounds: &'b [usize],
    pub measured: usize,
    pub mode: Drop,
    pub step: usize,
}

/// THE CLAIM (docs/VERIFICATION.md, claim 1). This is the sequence of calls layout makes when it
/// closes a box (`Live::close` in `emit.rs`), with everything layout gets from elsewhere left
/// arbitrary: the content's tokens, the limit, the policy, and both answers of the tokenizer
/// (`reserve`, the marker's length at its longest, and `more`, the marker's tokens).
///
/// A box with a limit emits at most that many tokens.
fn a_box_emits_at_most_its_limit(
    content: &[u32],
    tok_size: Option<usize>,
    max_tokens: Option<usize>,
    scroll: Option<usize>,
    drop: Option<DropBox<'_>>,
    clip: Clip,
    reserve: usize,
    more: &[u32],
    pad: Option<u32>,
) -> (out: Vec<u32>)
    requires
        drop matches Some(d) ==> d.offsets.len() == d.bounds.len() && boundaries(d.offsets@, content@.len()),
    ensures
        limit_spec(tok_size, max_tokens) matches Some(lim) ==> out.len() <= lim,
{
    let measured = match &drop {
        Some(d) => d.measured,
        None => content.len(),
    };
    let room = room(content, measured, tok_size, max_tokens, scroll.is_some());
    let over = room.over();
    let fit = match (over, scroll, drop) {
        (None, _, _) => room.whole(),
        (Some(_), Some(offset), _) => room.scroll(reserve, offset).0,
        (Some(_), None, Some(d)) => room.drop(d.offsets, d.bounds, reserve, d.mode, d.step).0,
        (Some(_), None, None) => room.clip(reserve, clip),
    };
    fit.assemble(more, pad).0
}

/// A box's limit: its fixed size if it has one, else its cap.
pub open spec fn limit_spec(tok_size: Option<usize>, max_tokens: Option<usize>) -> Option<usize> {
    match tok_size {
        Some(t) => Some(t),
        None => max_tokens,
    }
}

/// Item boundaries of content `n` tokens long: from 0 to `n`, never decreasing.
pub open spec fn boundaries(offsets: Seq<usize>, n: nat) -> bool {
    &&& offsets.len() >= 1
    &&& offsets[offsets.len() - 1] <= n
    &&& forall|i: int, j: int| 0 <= i <= j < offsets.len() ==> offsets[i] <= offsets[j]
}

/// `range` is `(content start, content end, output offset)`: the output holds that range of
/// the content at that offset.
pub open spec fn shows(content: Seq<u32>, out: Seq<u32>, range: (usize, usize, usize)) -> bool {
    let (a, b, at) = range;
    &&& a <= b <= content.len()
    &&& at + (b - a) <= out.len()
    &&& out.subrange(at as int, at + (b - a)) == content.subrange(a as int, b as int)
}

/// A range the box's tokens show is still shown once padding follows them.
proof fn shows_in_prefix(content: Seq<u32>, body: Seq<u32>, out: Seq<u32>, range: (usize, usize, usize))
    requires
        shows(content, body, range),
        body.len() <= out.len(),
        out.subrange(0, body.len() as int) == body,
    ensures
        shows(content, out, range),
{
    let (a, b, at) = range;
    assert(out.subrange(at as int, at + (b - a)) =~= body.subrange(at as int, at + (b - a)));
}

/// Boundaries that never decrease.
pub open spec fn monotone(bounds: Seq<usize>) -> bool {
    forall|i: int, j: int| 0 <= i <= j < bounds.len() ==> bounds[i] <= bounds[j]
}

/// The fewest items a drop mode keeps of `items`: one, or none for `drop-children`.
pub open spec fn min_kept(drop: Drop, items: int) -> int {
    if drop is Children || items < 1 { 0 } else { 1 }
}

/// What `drop_cut` promises about its cut `(lo, hi)`. On any boundaries: the kept side is the
/// right one and at least the minimum number of items stays. On boundaries that never decrease:
/// the kept items and the marker fit the limit, or only the minimum is left.
pub open spec fn cut_spec(bounds: Seq<usize>, lim: usize, more_len: usize, drop: Drop, lo: usize, hi: usize) -> bool {
    let last = bounds.len() - 1;
    let min = min_kept(drop, last);
    &&& hi - lo >= min
    &&& if drop is End { lo == 0 } else { hi == last }
    &&& monotone(bounds) && bounds[last] + more_len <= usize::MAX ==> {
        ||| bounds[hi as int] - bounds[lo as int] + more_len <= lim
        ||| hi - lo == min
    }
}

impl<'a> Room<'a> {
    pub closed spec fn wf(self) -> bool {
        self.size is Some ==> self.lim == self.size
    }

    pub closed spec fn content(self) -> Seq<u32> {
        self.content@
    }

    pub closed spec fn limit(self) -> Option<usize> {
        self.lim
    }

    pub closed spec fn over_spec(self) -> Option<usize> {
        over_spec(self.measured, self.content@.len(), self.lim, self.scroll)
    }
}

/// The limit, when a box must make room for its marker: it scrolls, or its measured length or
/// its content's length is over the limit.
pub open spec fn over_spec(measured: usize, len: nat, limit: Option<usize>, scroll: bool) -> Option<usize> {
    match limit {
        Some(lim) if scroll || measured > lim || len > lim => Some(lim),
        _ => None,
    }
}

impl<'a> Fit<'a> {
    /// The shown ranges are inside the content, in order, and do not overlap.
    pub closed spec fn wf(self) -> bool {
        &&& self.first.0 <= self.first.1 <= self.second.0 <= self.second.1 <= self.content@.len()
        &&& self.shown() + self.reserve <= usize::MAX
    }

    pub closed spec fn content(self) -> Seq<u32> {
        self.content@
    }

    pub closed spec fn shown(self) -> nat {
        match self.shape {
            Shape::Whole | Shape::Head | Shape::Tail => (self.first.1 - self.first.0) as nat,
            Shape::Middle => (self.first.1 - self.first.0 + self.second.1 - self.second.0) as nat,
            Shape::Marker => 0,
        }
    }

    /// Where in the content the first shown range starts.
    pub closed spec fn first_shown(self) -> usize {
        self.first.0
    }

    /// The most tokens the box can emit, whatever its marker is: the shown content with the
    /// marker's whole room, or its fixed size if padding fills more.
    pub closed spec fn most(self) -> nat {
        let body = self.shown() + self.reserve;
        match self.size {
            Some(size) if size > body => size as nat,
            _ => body as nat,
        }
    }

    /// The box's tokens before padding: the shown ranges with the marker, cut to its room, at
    /// the cut.
    pub closed spec fn unpadded(self, more: Seq<u32>) -> Seq<u32> {
        let marker = more.subrange(0, if more.len() < self.reserve { more.len() as int } else { self.reserve as int });
        let first = self.content@.subrange(self.first.0 as int, self.first.1 as int);
        let second = self.content@.subrange(self.second.0 as int, self.second.1 as int);
        match self.shape {
            Shape::Whole => first,
            Shape::Head => first + marker,
            Shape::Tail => marker + first,
            Shape::Middle => first + marker + second,
            Shape::Marker => marker,
        }
    }

    /// The box's tokens: `unpadded`, then the pad token up to a fixed size.
    pub closed spec fn output(self, more: Seq<u32>, pad: Option<u32>) -> Seq<u32> {
        let body = self.unpadded(more);
        match (self.size, pad) {
            (Some(size), Some(p)) if body.len() < size => body + Seq::new((size - body.len()) as nat, |i: int| p),
            _ => body,
        }
    }
}

} // verus!
