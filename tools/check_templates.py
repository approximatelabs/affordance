"""Has a publisher changed a chat template we have registered?

    tools/check_templates.py            compare every registered model's current template on
                                        Hugging Face with the one recorded here
    tools/check_templates.py --write    record the templates as they are in ref/models (after
                                        adding or regenerating models)

A registered model's view was made from its publisher's chat template as it was on one day.
`views/templates.lock.json` records each template's SHA-256. This compares that with what the
model's repository has now and lists the models whose template differs: those need
`tools/add_model.py` run again (the view is regenerated and checked against the new official
output). A repository that cannot be read without signing in is listed apart and is not a
failure; with a Hugging Face token in HF_TOKEN those are read too. Exits 1 if any template
changed.

Standard library only, so it runs anywhere with a Python interpreter: the weekly job in
.github/workflows/templates.yml runs it, and so can anyone by hand.
"""
import hashlib, json, os, sys, urllib.error, urllib.request
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOCK = os.path.join(ROOT, "views", "templates.lock.json")
# Where a repository keeps its chat template, in the order Hugging Face reads them.
FILES = ["chat_template.jinja", "tokenizer_config.json", "chat_template.json"]


def digest(template):
    """A template's identity: its text, or a model's named templates."""
    if isinstance(template, list): template = {t["name"]: t["template"] for t in template}
    return hashlib.sha256(json.dumps(template, sort_keys=True).encode()).hexdigest()


def read(name, text):
    if name.endswith(".jinja"): return text
    try:
        return json.loads(text).get("chat_template")
    except ValueError:
        return None


def local(model):
    for name in FILES:
        path = os.path.join(ROOT, "ref", "models", model, name)
        if os.path.exists(path):
            template = read(name, open(path).read())
            if template: return digest(template)
    return None


def remote(repo):
    """The digest of the repository's template now, or a word for why it could not be read."""
    for name in FILES:
        headers = {"User-Agent": "template-check"}
        # A Hugging Face token (HF_TOKEN) reads the repositories that need a signed-in account.
        if os.environ.get("HF_TOKEN"): headers["Authorization"] = "Bearer " + os.environ["HF_TOKEN"]
        request = urllib.request.Request(f"https://huggingface.co/{repo}/resolve/main/{name}", headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                # Line ends as reading the file gives them (a template stored with CRLF is read
                # with LF, by this script from ref/models and by Hugging Face's own loader).
                text = response.read().decode("utf-8", "replace").replace("\r\n", "\n").replace("\r", "\n")
                template = read(name, text)
        except urllib.error.HTTPError as error:
            if error.code in (401, 403): return "needs sign-in"
            if error.code == 404: continue
            return f"HTTP {error.code}"
        except Exception as error:
            return type(error).__name__
        if template: return digest(template)
    return "no template"


def main():
    index = json.load(open(os.path.join(ROOT, "views", "index.json")))
    models = sorted(k for k in index if not k.startswith("_"))
    if "--write" in sys.argv:
        lock = json.load(open(LOCK)) if os.path.exists(LOCK) else {}
        for model in models:
            found = local(model)
            if found: lock[model.replace("__", "/", 1)] = found
        lock = {repo: lock[repo] for repo in sorted(lock) if repo.replace("/", "__", 1) in index}
        open(LOCK, "w").write(json.dumps(lock, indent=1) + "\n")
        print(f"{len(lock)} of {len(models)} templates recorded")
        return 0
    lock = json.load(open(LOCK))
    repos = sorted(lock)
    with ThreadPoolExecutor(16) as pool:
        now = dict(zip(repos, pool.map(remote, repos)))
    changed = [r for r in repos if len(now[r]) == 64 and now[r] != lock[r]]
    unread = [r for r in repos if len(now[r]) != 64]
    print(f"{len(repos)} registered templates: {len(repos) - len(changed) - len(unread)} unchanged, {len(changed)} changed, {len(unread)} could not be read")
    for r in changed: print(f"  changed: {r}")
    for r in unread: print(f"  not read ({now[r]}): {r}")
    return 1 if changed else 0


if __name__ == "__main__":
    sys.exit(main())
