"""Build a standard tokenizer.json for a model whose tokenizer is a tiktoken or SentencePiece
data file read by a Python class, without running that class, and check it against what the
class itself produced in the sandbox (ref/sandbox/out/<model>): the same ids on every rendered
case text and probe text.

    .venv/bin/python ref/sandbox/build_tokenizer.py ORG__MODEL...
"""
import ast, json, os, sys, warnings
warnings.filterwarnings("ignore")
from tokenizers import Tokenizer, AddedToken
import transformers.convert_slow_tokenizer as conv

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..", "..")


def tiktoken_pattern(model_dir):
    """`pat_str` of the repository's tokenizer class, read from its source as data."""
    for f in os.listdir(model_dir):
        if not (f.startswith("tokenization") and f.endswith(".py")): continue
        tree = ast.parse(open(os.path.join(model_dir, f)).read())
        for node in ast.walk(tree):
            if isinstance(node, ast.Assign) and any(getattr(t, "id", None) == "pat_str" or getattr(t, "attr", None) == "pat_str" for t in node.targets):
                v = node.value
                # "|".join([...]) or a plain string
                try:
                    if isinstance(v, ast.Call) and isinstance(v.func, ast.Attribute) and v.func.attr == "join":
                        return ast.literal_eval(v.func.value).join(ast.literal_eval(v.args[0]))
                    return ast.literal_eval(v)
                except ValueError:
                    return None  # computed at run time
    return None


def sentencepiece_bpe(path):
    """A SentencePiece BPE model as the same tokenizer in the standard format: its pieces with
    their ids, the merges its scores imply, spaces written as its space mark, and bytes for
    what the vocabulary lacks."""
    from tokenizers import models, normalizers, decoders
    from transformers.tokenization_utils_base import generate_merges
    pb = conv.import_protobuf()
    m = pb.ModelProto(); m.ParseFromString(open(path, "rb").read())
    assert m.trainer_spec.model_type == 2 and m.normalizer_spec.name == "identity" and not m.normalizer_spec.remove_extra_whitespaces, "another kind of SentencePiece model"
    vocab = {p.piece: i for i, p in enumerate(m.pieces)}
    scores = {p.piece: p.score for p in m.pieces}
    # Only ordinary pieces merge; control, user-defined and byte pieces are whole.
    ordinary = {p.piece: i for i, p in enumerate(m.pieces) if p.type == 1}
    merges = generate_merges(ordinary, scores)
    tok = Tokenizer(models.BPE(vocab, merges, unk_token=m.pieces[m.trainer_spec.unk_id].piece, fuse_unk=True, byte_fallback=m.trainer_spec.byte_fallback))
    steps = [normalizers.Replace(" ", "\u2581")]
    if m.normalizer_spec.add_dummy_prefix: steps.insert(0, normalizers.Prepend("\u2581"))
    tok.normalizer = normalizers.Sequence(steps)
    tok.decoder = decoders.Sequence([decoders.Replace("\u2581", " "), decoders.ByteFallback(), decoders.Fuse()])
    # A user-defined piece is cut out of the text whole, longest first, before the merges run
    # (and after the space mark is put in front, which an added token would come before).
    import re
    from tokenizers import pre_tokenizers, Regex
    user = sorted((p.piece for p in m.pieces if p.type == 4), key=lambda x: -len(x))
    if len(user) > 5000:
        # Nearly every piece is user-defined (InternLM3): the text is cut into the longest piece
        # at each position, which is how added tokens are matched (after the space mark is in).
        tok.add_tokens([AddedToken(u, normalized=True, special=False) for u in user])
    elif user:
        tok.pre_tokenizer = pre_tokenizers.Split(Regex("|".join(re.escape(u) for u in user)), behavior="isolated")
        tok.model.ignore_merges = True
    return tok, f"SentencePiece BPE ({len(m.pieces)} pieces, {len(merges)} merges, {len(user)} user-defined)"


def build(name, nfc=False):
    d = os.path.join(ROOT, "ref", "models", name)
    out = os.path.join(HERE, "out", name)
    info = json.load(open(os.path.join(out, "info.json")))
    files = os.listdir(d)
    vocab = next((f for f in files if f == "tiktoken.model" or f.endswith(".tiktoken")), None)
    if vocab:
        pattern = tiktoken_pattern(d)
        if not pattern and vocab == "cl100k_base.tiktoken":
            # The class takes its pattern from the tiktoken library's cl100k_base at run time.
            import tiktoken_ext.openai_public as public, inspect, re
            pattern = ast.literal_eval(re.search(r'def cl100k_base\(\):.*?"pat_str": (r"""[^\n]*"""),', inspect.getsource(public), re.S).group(1))
        # tiktoken reads `{1,3}+` as possessive; the standard tokenizer's regex reads it as a repeat.
        if pattern: pattern = pattern.replace("{1,3}+", "{1,3}")
        kw = {"pattern": pattern} if pattern else {}
        tok = conv.TikTokenConverter(vocab_file=os.path.join(d, vocab), **kw).converted()
        how = f"tiktoken ({vocab}, {'its own' if pattern else 'the default'} split pattern)"
    elif "tokenizer.model" in files:
        tok, how = sentencepiece_bpe(os.path.join(d, "tokenizer.model"))
    elif "vocab.json" in files and "merges.txt" in files:
        from tokenizers import models, pre_tokenizers, decoders, normalizers, Regex
        # Qwen's layout: byte-level BPE, split first on Qwen's pattern.
        tok = Tokenizer(models.BPE.from_file(os.path.join(d, "vocab.json"), os.path.join(d, "merges.txt")))
        tok.pre_tokenizer = pre_tokenizers.Sequence([
            pre_tokenizers.Split(Regex(r"""(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+"""), behavior="isolated", invert=False),
            pre_tokenizers.ByteLevel(add_prefix_space=False, use_regex=False)])
        # Some classes normalise text to NFC before splitting it: tried when the plain one differs.
        if nfc: tok.normalizer = normalizers.NFC()
        tok.decoder = decoders.ByteLevel()
        how = "byte-level BPE (vocab.json, merges.txt" + (", text normalised to NFC)" if nfc else ")")
    else:
        raise SystemExit(f"{name}: no tiktoken, SentencePiece or vocab.json file")
    # The added tokens are the class's own list, with their ids.
    added = {int(i): a for i, a in info["added"].items()}
    # The class's own table names every reserved id; all of them are matched in text.
    for content, i in (info.get("special_ids") or {}).items():
        added.setdefault(i, {"content": content, "special": True})
    added = sorted(added.items(), key=lambda x: x[0])
    # Written into the tokenizer's JSON with the class's ids (they need not follow the vocabulary).
    # (An added token takes the id the vocabulary gives it, so the vocabulary names each one.)
    doc = json.loads(tok.to_str())
    for i, a in added: doc["model"]["vocab"][a["content"]] = i
    have = {t["content"] for t in doc["added_tokens"]}
    doc["added_tokens"] = [t for t in doc["added_tokens"] if t["content"] not in {a["content"] for _, a in added}] + [{"id": i, "content": a["content"], "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": bool(a["special"])} for i, a in added]
    for t in doc["added_tokens"]: t["id"] = doc["model"]["vocab"][t["content"]]
    doc["added_tokens"].sort(key=lambda t: t["id"])
    tok = Tokenizer.from_str(json.dumps(doc))
    wrong = [(a["content"], i, tok.token_to_id(a["content"])) for i, a in added if tok.token_to_id(a["content"]) != i]
    probes = json.load(open(os.path.join(out, "probes.json")))
    bad = []
    for text, ids in probes.items():
        if isinstance(ids, dict): continue
        got = tok.encode(text, add_special_tokens=False).ids
        if got != ids: bad.append(text)
    print(f"{name}: {how}; {len(added)} added tokens ({len(wrong)} with another id); {len(probes) - len(bad)} of {len(probes)} texts encode the same")
    if wrong: print("   ids differ:", wrong[:4])
    if bad:
        t = bad[0]; got = tok.encode(t, add_special_tokens=False).ids; want = probes[t]
        k = next((j for j in range(min(len(got), len(want))) if got[j] != want[j]), min(len(got), len(want)))
        print("   first difference:", repr(tok.decode(want[max(0, k - 3):k + 3])), "official", want[max(0, k - 2):k + 4], "ours", got[max(0, k - 2):k + 4])
    if not wrong and not bad:
        tok.save(os.path.join(ROOT, "ref", "tokenizers", name + ".json"))
        json.dump(json.load(open(os.path.join(out, "special.json"))), open(os.path.join(ROOT, "ref", "tokenizers", name + ".special.json"), "w"), ensure_ascii=False)
        for f in os.listdir(out):
            s = f[:-5]
            if f.endswith(".json") and os.path.exists(os.path.join(ROOT, "ref", f"cases_{s}.json")) and not s.startswith("heldout_") or s == "heldout_a2":
                os.makedirs(os.path.join(ROOT, "ref", "out", s), exist_ok=True)
                json.dump(json.load(open(os.path.join(out, f))), open(os.path.join(ROOT, "ref", "out", s, name + ".json"), "w"), ensure_ascii=False)
        return True
    if bad and not nfc and "vocab.json" in files and "merges.txt" in files and not vocab and "tokenizer.model" not in files:
        return build(name, nfc=True)
    return False


if __name__ == "__main__":
    ok = [build(n) for n in sys.argv[1:]]
    print(sum(ok), "of", len(ok), "tokenizers built and equal to the official class")
