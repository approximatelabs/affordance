# Adding a model

Affordance turns an app into the exact tokens a model reads. To do that for a model it needs
that model's chat format: how the model's publisher writes a conversation, its tools and its
replies as tokens. This page says how to check whether a model is supported, how to add one
that is not, and how to send the addition to the project.

## Check whether your model is supported

Models are listed in `views/index.json` under their Hugging Face name with `/` written as `__`:

```sh
grep '"Qwen__Qwen3.5-2B"' views/index.json
```

If the name is there, name the model as the `template` of your model binding and run:

```json
{"model": {"kind": "completions", "url": "http://127.0.0.1:8000", "template": "Qwen/Qwen3.5-2B"}}
```

A fine-tune usually keeps its base model's chat template. If your fine-tune is not listed but
its base model is, the base model's name works as the template, as long as the two repositories
have the same `chat_template.jinja` and tokenizer.

## Add a model with one command

You need a checkout of this repository, Python through `uv`, and a release build:

```sh
uv sync
cargo build --release --features reference-tests --examples
uv run python tools/add_model.py ORG/NAME
cargo build --release
```

The command does five things and says which one it is on:

1. It fetches the model's tokenizer and chat template files from Hugging Face into
   `ref/models/`. It does not fetch weights.
2. It renders the official output: the model's own template and tokenizer, run by Hugging
   Face's `apply_chat_template`, on six sets of test conversations (about 960 conversations).
3. It converts the template to a view (`tools/jinja2tsx.py`) under `views/third_party/`, and
   checks the view against the official output on every conversation, token for token.
4. It measures what an agent needs to read the model's replies: the tokens that end a reply,
   whether the prompt ends inside open thinking, and how the model writes a tool call.
5. It registers the model in `views/index.json`. For a model that takes pictures it also
   records how the model's processor turns a picture into tokens, where the processor is a
   design it knows, so a style sheet can size the model's pictures (`img { max-tokens: .. }`).

If a step fails, the command names the step and the reason, and registers nothing. The second
`cargo build` puts the new index into the binary.

## Try it

Render a conversation in the model's format without a server:

```sh
./target/release/affordance render tests/fixtures/any_model_agent.tsx \
  --template ORG/NAME --tokenizer ref/tokenizers/ORG__NAME.json \
  --state <(echo '{"log": [{"kind": "input", "content": "Hello."}]}')
```

Then step an agent on it, as the test suite does for every registered model:

```sh
AFFORDANCE_ALL_MODELS=1 cargo test --release --features reference-tests --test suite any_model::
```

## When the command fails

**"its tokenizer is a Python class in the repository."** Some publishers ship the tokenizer as
Python code and not as `tokenizer.json` (Kimi, InternLM, Phi-3-small). Affordance does not run a
model repository's code on your machine. `ref/sandbox/run.sh ORG__NAME` runs that code in a
Docker container with no network and a read-only file system, and
`ref/sandbox/build_tokenizer.py ORG__NAME` builds a standard tokenizer from the model's data
file and keeps it only if it gives the same ids as the publisher's code on every text. After
both pass, run `tools/add_model.py` again.

**"converter does not handle ..."** The template uses a Jinja feature the converter has not met.
Open an issue with the model's name and the message. The fix belongs in `tools/jinja2tsx.py`
or `views/common/jinja.tsx`, and it usually unlocks other models too.

**"the converted template does not match the official output."** The view renders, but some
conversation gives different tokens. `./target/release/affordance check --only ORG__NAME`
prints the first difference for each failing conversation. This is a bug in the converter or
its runtime; open an issue with that output.

**"its tool calls are not read yet."** The model is registered and an agent can talk to it,
but the model writes tool calls in a shape the reader does not handle. Open an issue with the
model's name.

## Send it to the project

A model addition is a pull request with these files, all written by the command:

- `views/third_party/<org>/<model>.tsx`, the converted template.
- `views/third_party/<org>/NOTICE.md`, which names the model, the revision the template was read
  at and its license. The template's text belongs to the model's publisher under that license.
- `views/index.json`, with the model's entry.
- `ref/models.txt`, with the model's line.
- `views/templates.lock.json`, with the hash of the model's template as it is today. A weekly
  job compares every registered template with its repository and names the ones a publisher
  has changed (`tools/check_templates.py` does the same by hand).

Steps:

```sh
git checkout -b add-ORG-NAME
git add views ref/models.txt
git commit -m "ORG/NAME's prompt format, matching its official template token for token"
git push -u origin add-ORG-NAME
gh pr create --fill
```

In the pull request, paste the command's output. A reviewer runs the same command and the
checks in `tools/check-models.sh`. The files under `ref/models`, `ref/tokenizers` and `ref/out`
stay on your machine: they are downloads and references, and the repository does not carry
them.
