"""Download tokenizer + template files only (never weights)."""
import sys, os
from huggingface_hub import snapshot_download
PAT=["tokenizer.json","tokenizer_config.json","special_tokens_map.json","chat_template.jinja","chat_template.json",
     "generation_config.json","config.json","tokenizer.model","vocab.json","merges.txt","*.tiktoken","tekken.json",
     "added_tokens.json","encoding*/*","*encoding*.py","*.jinja","README.md"]
root=os.path.join(os.path.dirname(__file__),"models")
for line in open(os.path.join(os.path.dirname(__file__),"models.txt")):
    line=line.strip()
    if not line or line.startswith("#"): continue
    fam,repo=[x.strip() for x in line.split("|")]
    d=os.path.join(root,repo.replace("/","__"))
    try:
        snapshot_download(repo,local_dir=d,allow_patterns=PAT)
        print("OK",repo,sorted(os.listdir(d)))
    except Exception as e:
        print("FAIL",repo,type(e).__name__,str(e)[:200])
