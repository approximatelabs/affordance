"""Add models: fetch each one's chat template and tokenizer, check a view of the template
against the model's official output token for token, measure what an agent needs to read its
replies, and register it.

    uv run python tools/add_model.py ORG/NAME [ORG/NAME ...]
    uv run python tools/add_model.py --from FILE.json      (a JSON list of names)
    uv run python tools/add_model.py --refresh ORG/NAME    (its publisher changed its template:
                                                            tools/check_templates.py names these)

Each step says what it did. When one fails for a model it says which and why, and that model
is not registered. When all pass, the model is in views/index.json and an agent can be bound
to it (`"template": "ORG/NAME"`); docs/ADDING_A_MODEL.md says how to send that as a pull
request.

A model whose template is the same text as a registered model's uses that model's view (a
fine-tune keeps its base model's template); otherwise its template is converted
(tools/jinja2tsx.py). Either way the view is checked against this model's own tokenizer.

Needs: network access to Hugging Face, the Python environment (`uv sync`), and a release build
(`cargo build --release --features reference-tests --examples`). A model whose tokenizer is a
Python class in its repository also needs Docker: that code is run only in a container with no
network (ref/sandbox), and a standard tokenizer built from the model's data file must agree with
it on every text.
"""
import contextlib, copy, hashlib, io, json, os, re, subprocess, sys, warnings
from multiprocessing import Pool
warnings.filterwarnings("ignore")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)
sys.path.insert(0, os.path.join(ROOT, "tools")); sys.path.insert(0, os.path.join(ROOT, "ref"))
FILES = ["tokenizer.json", "tokenizer_config.json", "special_tokens_map.json", "chat_template.jinja",
         "chat_template.json", "config.json", "added_tokens.json", "vocab.json", "merges.txt", "*.model", "*.tiktoken"]
SETS = ["dev", "heldout1", "heldout2", "heldout3", "heldout_a2", "agent_loops"]
BIN = "./target/release/affordance"


def key(name):
    """A template's identity: its text (a model's named templates, in order)."""
    import adopt_templates
    t = adopt_templates.template(name)
    return None if t is None else hashlib.sha256(json.dumps(t, sort_keys=True).encode()).hexdigest()


def prepare(repo):
    """Fetch the model's files and render the official output for every case: the repository's
    tokenizer and template, through Hugging Face's apply_chat_template."""
    name = repo.replace("/", "__", 1)
    try:
        from huggingface_hub import snapshot_download
        with contextlib.redirect_stderr(io.StringIO()):
            snapshot_download(repo, local_dir=f"ref/models/{name}", allow_patterns=FILES)
    except Exception as e:
        return repo, f"could not be fetched: {type(e).__name__}: {str(e)[:120]}"
    try:
        import render_ref  # pins the clock and the tokenizer backend
        import adopt_templates
        from transformers import AutoTokenizer
        d = f"ref/models/{name}"
        try:
            tok = AutoTokenizer.from_pretrained(d, trust_remote_code=False)
        except Exception as first:
            # A repository whose config the library cannot read, or whose tokenizer class is its
            # own code around a standard tokenizer.json, still has that file: it is the data.
            if not os.path.exists(f"{d}/tokenizer.json"): raise
            from transformers import PreTrainedTokenizerFast
            cfg = json.load(open(f"{d}/tokenizer_config.json")) if os.path.exists(f"{d}/tokenizer_config.json") else {}
            text_of = lambda v: v if isinstance(v, str) else (v or {}).get("content")
            named = {k: text_of(cfg[k]) for k in ["bos_token", "eos_token", "unk_token", "pad_token", "sep_token", "cls_token", "mask_token"] if cfg.get(k)}
            extra = [text_of(x) for x in cfg.get("additional_special_tokens", []) or []]
            tok = PreTrainedTokenizerFast(tokenizer_file=f"{d}/tokenizer.json", additional_special_tokens=extra, **named)
        if tok.chat_template is None:
            t = adopt_templates.template(name)
            if t is None: return repo, "its repository has no chat template"
            tok.chat_template = t if isinstance(t, str) else [{"name": k, "template": v} for k, v in t.items()]
        os.makedirs("ref/tokenizers", exist_ok=True)
        for old in [f"ref/tokenizers/{name}.json", f"ref/tokenizers/{name}.special.json"]:
            if os.path.exists(old): os.chmod(old, 0o644)
        open(f"ref/tokenizers/{name}.json", "w").write(tok.backend_tokenizer.to_str())
        json.dump(tok.special_tokens_map, open(f"ref/tokenizers/{name}.special.json", "w"), ensure_ascii=False)
        for s in SETS:
            res = {}
            for c in json.load(open(f"ref/cases_{s}.json")):
                try:
                    kw = dict(c["kwargs"])
                    text = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=False, **kw)
                    enc = tok.apply_chat_template(copy.deepcopy(c["messages"]), tools=copy.deepcopy(c["tools"]), tokenize=True, **kw)
                    res[c["id"]] = {"ids": list(enc["input_ids"] if hasattr(enc, "keys") else enc), "text": text}
                except Exception as e:
                    res[c["id"]] = {"error": f"{type(e).__name__}: {str(e)[:300]}"}
            os.makedirs(f"ref/out/{s}", exist_ok=True)
            json.dump(res, open(f"ref/out/{s}/{name}.json", "w"), ensure_ascii=False)
    except Exception as e:
        why = f"{type(e).__name__}: {str(e)[:160]}"
        if "custom code" in why or "trust_remote_code" in why:
            why = "its tokenizer is a Python class in the repository (see ref/sandbox in docs/ADDING_A_MODEL.md)"
        return repo, f"the official output could not be rendered: {why}"
    return repo, None


def sandboxed(repo):
    """For a model whose tokenizer is a Python class in its repository: run that class in the
    sandbox (ref/sandbox/run.sh: a container with no network, a read-only file system and no
    privileges) to get the official output, then build a standard tokenizer from the model's
    data file and keep it only if it gives the class's ids on every text
    (ref/sandbox/build_tokenizer.py). Returns None, or why it could not be done."""
    name = repo.replace("/", "__", 1)
    try:
        from huggingface_hub import snapshot_download
        with contextlib.redirect_stderr(io.StringIO()):
            snapshot_download(repo, local_dir=f"ref/models/{name}", allow_patterns=FILES + ["*.py"])
    except Exception as e:
        return f"its Python files could not be fetched: {type(e).__name__}: {str(e)[:120]}"
    if subprocess.run(["docker", "version"], capture_output=True).returncode != 0:
        return "its tokenizer is a Python class, which is run in a Docker container, and Docker is not available"
    ran = subprocess.run(["ref/sandbox/run.sh", name], capture_output=True, text=True)
    lines = [l for l in ran.stdout.splitlines() if l.strip() and "Calling super" not in l]
    if not any(l.startswith("done ") for l in lines):
        return f"its tokenizer class did not load in the sandbox: {(lines or ['no output'])[-1][:160]}"
    built = subprocess.run([sys.executable, "ref/sandbox/build_tokenizer.py", name], capture_output=True, text=True)
    out = [l for l in built.stdout.splitlines() if l.startswith(name) or l.strip().startswith(("first difference", "ids differ"))]
    if "1 of 1 tokenizers built" not in built.stdout:
        why = " ".join(out)[:220] or built.stderr.strip().splitlines()[-1][:200] if built.stderr.strip() else " ".join(out)[:220]
        return f"a standard tokenizer built from its data file does not match its class: {why}"
    return None


def checked(names, sets):
    """model -> None if exact on every one of `sets`, else what differed."""
    result = {}
    for i in range(0, len(names), 40):
        chunk = names[i:i + 40]
        out = subprocess.run([BIN, "check", "--set", ",".join(sets), "--only", ",".join(chunk)], capture_output=True, text=True).stdout
        seen = {n: 0 for n in chunk}
        for line in out.splitlines():
            m = re.match(r"(\S+)\s+(\d+)/\s*(\d+) exact \(\d+ match, (\d+) both-error, (\d+) same kind\)", line)
            if m and m.group(1) in seen:
                seen[m.group(1)] += 1
                if m.group(2) != m.group(3): result[m.group(1)] = f"{m.group(2)} of {m.group(3)} cases exact on one set"
                elif m.group(4) != m.group(5): result[m.group(1)] = "it fails cases for another kind of reason than the official template"
            m = re.match(r"(\S+)\s+LOAD ERROR (.*)", line)
            if m and m.group(1) in seen: result[m.group(1)] = "its view does not load: " + m.group(2)[-120:]
        for n in chunk:
            if n not in result: result[n] = None if seen[n] == len(sets) else "the check did not finish"
    return result


def main(repos, refresh=False):
    import adopt_templates
    index = json.load(open("views/index.json"))
    if refresh:
        # A publisher changed these models' templates: each is registered afresh from what its
        # repository has now. (A model with a hand-written view of its own is not touched.)
        for repo in repos:
            name = repo.replace("/", "__", 1)
            entry = index.get(name)
            if not entry or not (entry.get("converted") or entry.get("shared")): continue
            del index[name]
            view = entry["view"]
            if view.startswith("third_party/") and not any(e.get("view") == view for k, e in index.items() if not k.startswith("_")):
                os.remove("views/" + view)
        open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
    report = {r: "already registered" for r in repos if r.replace("/", "__", 1) in index}
    todo = [r for r in repos if r not in report]
    with Pool(min(16, max(1, len(todo)))) as pool:
        for n, (repo, error) in enumerate(pool.imap_unordered(prepare, todo), 1):
            if error: report[repo] = "FAILED: " + error
            if len(todo) > 10 and n % 25 == 0: print(f"  fetched and rendered {n} of {len(todo)}", flush=True)
    # A tokenizer that is a Python class: the sandbox gives the official output.
    for repo in [r for r in todo if "Python class" in report.get(r, "") or "max_position_embeddings" in report.get(r, "")]:
        print(f"  {repo}: its tokenizer is a Python class; running it in the sandbox", flush=True)
        why = sandboxed(repo)
        if why is None: del report[repo]
        else: report[repo] = "FAILED: " + why
    ready = [r.replace("/", "__", 1) for r in todo if r not in report]
    print(f"{len(ready)} of {len(todo)} fetched, with their official output rendered", flush=True)
    # A model whose template is a registered model's text uses that model's view.
    by_template = {}
    for name, e in index.items():
        if name.startswith("_") or not os.path.isdir(f"ref/models/{name}"): continue
        k = key(name)
        # (A view with a first step of its own is that model's alone.)
        if k and name not in adopt_templates.PREPARE: by_template.setdefault(k, name)
    shared, fresh = [], []
    for name in ready:
        base = by_template.get(key(name))
        if base:
            index[name] = {k: v for k, v in index[base].items() if k in ("view", "entry", "converted")}
            index[name]["shared"] = base
            shared.append(name)
        else:
            fresh.append(name)
    open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
    # The rest are converted, one view for each distinct template.
    firsts = {}
    for name in fresh: firsts.setdefault(key(name), name)
    with contextlib.redirect_stdout(io.StringIO()):
        adopt_templates.main(list(firsts.values()))
    adopted = json.load(open("target/adopt_report.json"))
    index = json.load(open("views/index.json"))
    for name in fresh:
        first = firsts[key(name)]
        if adopted.get(first) != "exact":
            report[name.replace("__", "/", 1)] = f"FAILED: its converted template does not match the official output: {adopted.get(first)}"
        elif name != first:
            index[name] = dict(index[first]); index[name].pop("agent", None)
            index[name]["shared"] = first
            shared.append(name)
    open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
    registered = [n for n in ready if n in index]
    # Every registered model, on every case set, against its own tokenizer.
    result = checked(registered, SETS)
    # A shared view that fails for this model (it was written for another tokenizer): the
    # model's own template is converted instead.
    again = [n for n in registered if result[n] is not None and n in shared]
    if again:
        for n in again: del index[n]; shared.remove(n)
        open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
        with contextlib.redirect_stdout(io.StringIO()):
            adopt_templates.main(again)
        adopted = json.load(open("target/adopt_report.json"))
        index = json.load(open("views/index.json"))
        for n in again:
            if adopted.get(n) != "exact": result[n] = f"converted on its own after a shared view failed: {adopted.get(n)}"
        result.update(checked([n for n in again if n in index], SETS))
    for name, why in result.items():
        if why is None or name not in index: 
            if why is not None: report[name.replace("__", "/", 1)] = f"FAILED: its view does not match the official output: {why}"
            continue
        report[name.replace("__", "/", 1)] = f"FAILED: its view does not match the official output: {why}"
        view = index.pop(name)["view"]
        if not any(e.get("view") == view for k, e in index.items() if not k.startswith("_")) and view.startswith("third_party/"):
            os.remove("views/" + view)
    open("views/index.json", "w").write(json.dumps(index, indent=1) + "\n")
    kept = [n for n in registered if n in index]
    listed = open("ref/models.txt").read()
    with open("ref/models.txt", "a") as f:
        for name in kept:
            if f"| {name.replace('__', '/', 1)}\n" not in listed: f.write(f"converted | {name.replace('__', '/', 1)}\n")
    with contextlib.redirect_stdout(io.StringIO()):
        adopt_templates.notices(index)
    # What an agent needs to read each one's replies.
    if kept:
        rows = {}
        for name in kept if len(kept) <= 4 else [None]:
            env = dict(os.environ) if name is None else {**os.environ, "SURVEY_ONLY": name}
            rows.update(json.loads(subprocess.run(["./target/release/examples/protocol_survey"], capture_output=True, text=True, env=env).stdout))
        json.dump({n: rows[n] for n in kept if n in rows}, open("target/protocol_survey_added.json", "w"))
        subprocess.run([sys.executable, "tools/measure_agents.py", "target/protocol_survey_added.json"], check=True, capture_output=True)
        # How each one's processor turns a picture into tokens, for layout (src/media.rs).
        subprocess.run([sys.executable, "tools/measure_media.py", *[n.replace("__", "/", 1) for n in kept]], check=True, capture_output=True)
        for name in kept:
            row = rows.get(name, {"error": "not surveyed"})
            reads = row.get("parsers") or []
            tools = "no tool calls in its format" if not row.get("renders_tools") else f"tool calls read ({reads[0]})" if reads else "tool calls not read yet"
            how = "the view of a model with the same template" if name in shared else "its template converted"
            agent = f"an agent cannot run on it yet ({row['error'][:80]})" if "error" in row else f"replies end at {row.get('stop')}; {tools}"
            report[name.replace("__", "/", 1)] = f"added ({how}; exact on all six case sets; {agent})"
    # The templates as they are today, for the weekly check that a publisher has not changed one.
    subprocess.run([sys.executable, "tools/check_templates.py", "--write"], check=True, capture_output=True)
    json.dump(report, open("target/add_model_report.json", "w"), indent=1)
    for repo in repos:
        if len(repos) <= 20 or report[repo].startswith("FAILED"): print(f"{repo}: {report[repo]}")
    print(f"{len(kept)} of {len(repos)} added. Rebuild (cargo build --release) so the binary carries the new index.")
    return all(not v.startswith("FAILED") for v in report.values())


if __name__ == "__main__":
    args = sys.argv[1:]
    refresh = "--refresh" in args
    args = [a for a in args if a != "--refresh"]
    if args[:1] == ["--from"]: args = json.load(open(args[1]))
    if not args: raise SystemExit(__doc__)
    sys.exit(0 if main(args, refresh) else 1)
