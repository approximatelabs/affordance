# Third-party template text

Everything in this directory is a model publisher's: a chat template converted so that prompts
for the model come out in its format, or prose a template carries. It is here for
interoperability. It is not licensed under this repository's Apache-2.0 license; each folder's
`NOTICE.md` names the model, the revision and the license its publisher gives it. A publisher
who wants a file removed can open an issue.

Some chat templates carry prose: a default system message, a tools introduction, a tool-call
format, reasoning-effort instructions. A template app reproduces the template's tokens exactly,
so it needs that text verbatim. Every such text lives here, one folder per upstream, as named
constants the template apps import; nothing else in `views/` copies upstream prose
(`tests/third_party.rs` checks).

Each folder's `NOTICE.md` names the models and revisions the text was copied from and the license
it is under. These files are not covered by this repository's Apache-2.0 license unless their
notice says so; the rest of `views/` is.
