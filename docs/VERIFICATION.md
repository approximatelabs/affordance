# Verification (draft)

Some properties of the engine are proved, not only tested. This page lists each one: the claim
in a sentence or two, the exact statement the tool checked, what the proof assumes, and what it
does not cover. A claim that cannot be said in a sentence or two is not listed here.

Two tools are used. [Verus](https://github.com/verus-lang/verus) proves statements about Rust
code for every input, with no bound on sizes. [Kani](https://github.com/model-checking/kani)
checks statements for every input up to a stated size. Neither is part of an ordinary build or of
the PR gate.

## What is checked, and how it relates to the code that runs

The arithmetic that fits a box to its token limit is `src/fit.rs`. Layout (`Live::close` in
`src/emit.rs`) calls it for every box it closes. Verus reads that same file: there is no
separate model. The specifications are attributes under `cfg(verus_keep_ghost)`, which only
Verus sets, so `cargo build` compiles the functions as plain Rust with no new dependency.

```sh
./tools/verify.sh            # Verus on src/fit.rs; expects "20 verified, 0 errors"
cargo kani                   # the bounded harnesses in src/emit_proofs.rs
```

`tools/verify.sh` finds Verus through `VERUS=/path/to/verus`, then `PATH`, then
`~/.local/share/verus/*/verus`. Verus is a release binary from its GitHub releases; it names the
Rust toolchain it needs (`rustup install ...`). The proofs were last checked with Verus
`0.2026.11.02.8bf68ee` (toolchain 1.99.1) and Kani 0.68.0.

The proofs were also checked not to be empty: five deliberate bugs in `src/fit.rs` (a clip that
keeps one token too many, a scroll viewport that ignores the marker's room, a drop that does not
clip its last item, a marker that is not cut to its room, padding one token past the size) each
make Verus fail.

## Claim 1: a box never emits more tokens than its limit

**The story.** A box with `max-tokens` or `tok-size` never puts more tokens in the prompt than
that limit, whatever its content, its `more` marker and its overflow policy (clip-end,
clip-start, clip-middle, elide, scroll, drop-start, drop-end, drop-children, with or without
`drop-step`). There is no exception.

**The statement proved.** `a_box_emits_at_most_its_limit` in `src/fit.rs`. It is the sequence
of calls `Live::close` makes (`room`, `over`, then one of `whole`, `scroll`, `drop`, `clip`,
then `assemble`), with everything layout obtains elsewhere left arbitrary: the content's tokens,
`tok-size`, the cap in tokens, the policy, the scroll offset, the item boundaries, the step, and
both answers of the tokenizer (the marker's length at its longest, and the marker's tokens). For
all of them:

> if the box has a limit `lim` (its `tok-size`, else its cap), the tokens it emits number at
> most `lim`.

The proof also establishes, for every function in the file, that no index is out of range and
no arithmetic overflows, and that `assemble` returns exactly the shown ranges of the content,
the marker cut to its reserved room, and padding, in the box's order
(`Fit::assemble`'s postcondition, `Fit::output`).

**What the proof found.** The first version of this claim had an exception, and the proof would
not go through without stating it. A drop box decides whether it overflows on its *measured*
length: its content with graded results (`result:nth-last(...)`) counted at their static length,
so that grading never moves the cut. A sheet whose graded rule makes a result *larger* than its
static style made the measured length smaller than the real one, and a box whose measure was
within its limit was laid out whole, over the limit: with
`history { max-tokens: 1545tok; overflow: drop-start; }`, `result { max-tokens: 6tok; }` and
`result:nth-last(n+5 / 3) { max-tokens: 400tok; }`, a history of forty tool results laid out to
1865 tokens. The fix is one condition (a box is over when its measure *or its content* is over
its limit), the claim is proved without the exception, and the case is a regression test
(`a_history_cap_holds_when_old_results_grow` in `tests/turn_spans.rs`).

**Assumed (trusted, not proved).**

- *The glue in `Live::close`* (`src/emit.rs`, about eighty lines around the calls): that the
  limit it passes is the box's (`tok-size`, else `max-tokens` resolved against the window), that
  it passes the content it split off, and that it appends the tokens `assemble` returns
  unchanged. It is short and has no arithmetic on lengths left in it except the two below.
- *`Room::drop`'s precondition*: the item boundaries layout passes never decrease and end at or
  before the content's length, and there are as many measured boundaries as boundaries. Layout builds them from the positions of segment marks, in order; this is not
  proved. No other function the glue calls has a precondition on its data.
- *After the box closes*: nothing later adds tokens inside it. A box is a tokenization boundary
  (its content is encoded on its own, and text after it starts a new run), and an enclosing box
  can only remove tokens from it. This is not proved.
- *The tokenizer is not assumed to be anything*: the marker's tokens and length are arbitrary in
  the statement. The count is of the token ids the engine emits; whether a model server counts
  the same ids the same way is outside the engine.
- Verus itself, the Z3 solver, the Rust compiler, and Verus's specifications of the standard
  library calls used (`Vec::push`, `extend_from_slice`, `with_capacity`, slicing,
  `saturating_add`/`sub`/`mul`, `Option::unwrap_or`). Running out of memory is not modelled.

**Not covered.**

- *A box with no limit.* `max-tokens: 80win` when the model binding states no context length is
  no cap (a page with no viewport is not clipped). A box with neither property has no limit.
- *The prompt as a whole.* Layout does not bound the whole prompt; boxes do, where a sheet puts
  them. What the runtime guarantees instead is a check, not a layout property: a prompt that
  with its reply's room is over the model's context is never sent, and the call fails with a
  `ContextError` naming the largest boxes (`Instance::over_context` in `src/runtime/instance.rs`).
  That check is one comparison and is not verified here. With no stated context there is no
  check.
- *A hosted (Messages) history.* `history { max-tokens: ... }` on a hosted binding drops whole
  units before serialization (`drop_history` in `src/turns.rs`). It uses the verified cut
  (`drop_cut`) but never clips the unit it keeps, and it measures JSON with the bound tokenizer,
  not the provider's: the cap is approximate there and no bound is claimed.
- *Spans.* Where labels, actions and nested boxes land after a crop (`rebase`) is checked by
  Kani up to a size, not by Verus. `assemble` is proved to report each kept range correctly
  (the output holds that range of the content at the reported offset), which is what `rebase`
  starts from.
- *The values of style properties.* Parsing a sheet, the cascade, and `win` and `%` arithmetic
  (`Live::resolve_size`, `Step::tokens`) are outside the proof. The claim is about the limit in
  tokens that reaches layout.

**The gap between the story and the theorem.** The story says "puts in the prompt"; the theorem
says "`assemble` returns". The difference is the trusted glue and the two unproved facts above
(the boundaries are ordered; nothing is added to a closed box). The story says "its limit"; the
theorem's limit is the number layout computed from the sheet, not the sheet's text.

## Claim 2: a drop box drops whole items

**The story.** A box with `overflow: drop-start`, `drop-end` or `drop-children` makes room by
removing whole items. It never cuts into an item it keeps, unless it is down to the single item
it must keep and that item alone is too big (`drop-children` keeps none in that case, so it
never cuts).

**The statement proved.** The postcondition of `Room::drop` with `drop_cut`'s (`cut_spec`) in
`src/fit.rs`. For a box over its limit whose items are measured as they are (the measured
boundaries equal the real ones: no graded result inside), with `(lo, hi)` the cut returned:

> the number of tokens shown is exactly the length of the content between boundary `lo` and
> boundary `hi` (and the range shown starts at boundary `lo`, so it is those items); or the mode
> is not `drop-children` and `hi - lo` is the minimum it keeps (one item).

and, on any boundaries: `lo <= hi`, both are in range, `drop-end` keeps a prefix (`lo == 0`) and
the other modes a suffix (`hi` is the last boundary), and at least the minimum is kept.

**Assumed.** As for claim 1. In addition the statement excludes sizes that cannot occur (the
content's length plus the limit over `usize::MAX`).

**Not covered.** A drop box with graded results inside: the cut is chosen on measured lengths,
so kept items may then be clipped to the limit (this is what keeps claim 1 true for it). That
the cut is the *best* one (keeping one more item would not fit), that it equals the simple
one-item-at-a-time loop, and that with `drop-step` it only moves forward as content is appended:
Kani checks these in `src/emit_proofs.rs` for up to 8 items and 64 tokens, not for all sizes.
The error raised when a cut would fall inside a token two items share is in the glue.

## Checked up to a size (Kani)

`src/emit_proofs.rs` holds harnesses for `drop_cut` and `rebase`, each over every input within
its bounds (up to 8 items, lengths up to 64 tokens, every step up to 130): the cut is in range,
on the right side, fits unless only the minimum is left, is maximal without a step, equals the
reference loop without a step, is the first boundary past the stepped amount, and only moves
forward as items are appended; rebased spans land inside the output. These are evidence, not
proofs for all sizes; the first of them is subsumed by claims 1 and 2.

## Not attempted: replay

"Replaying a journal, or restoring a checkpoint and replaying the newer inputs, reaches the same
state as the original run" is the property the runtime is built around
([HARNESS.md](../spec/HARNESS.md) section 8) and it is tested, not proved. Nothing on this page
supports it.
