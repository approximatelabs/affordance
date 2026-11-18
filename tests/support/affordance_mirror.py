#!/usr/bin/env python3
"""A mirror of other agents' state, kept in a directory on a machine.

An overseer reads the fleet with `harness.inspect(targets, {select, since, as: "text"})` and sends
the text to its machine as `stdin`:

    affordance_mirror.py apply DIR            # JSON lines on stdin, one per agent

Analytics then read the mirror as ordinary data:

    from affordance_mirror import Mirror
    agents = Mirror(DIR)
    agents["agent-0042"]["fold"]              # a hook, decoded (a Map is a dict, a Set a set)
    agents.version("agent-0042"), agents.status("agent-0042")["phase"]

Each agent is one file, `DIR/<sha256-of-id>.json`: {version, status, state, offsets}, so applying a read
rewrites only the agents it changed. A line with `changed: false` changes nothing; a change is
`{key, path?, set}` (the value) or `{key, path?, from, items, length}` (a list grew: items from
index `from`). A list first read with `tail` starts at an offset, kept in `offsets`.
"""
import json
import hashlib
import os
import sys


def _name(change):
    return json.dumps([change["key"], change.get("path", [])], separators=(",", ":"))


def _place(state, change):
    """The container and key the change's path ends at, creating objects on the way."""
    keys = [change["key"]] + change.get("path", [])
    node = state
    for k in keys[:-1]:
        if isinstance(node, list):
            node = node[int(k)]
            continue
        if not isinstance(node.get(k), (dict, list)):
            node[k] = {}
        node = node[k]
    return node, int(keys[-1]) if isinstance(node, list) else keys[-1]


def _path(directory, agent_id):
    return os.path.join(directory, hashlib.sha256(agent_id.encode("utf-8")).hexdigest() + ".json")


def apply_line(directory, line):
    rec = json.loads(line)
    if "error" in rec or not rec.get("changed", True):
        return
    path = _path(directory, rec["id"])
    try:
        with open(path) as f:
            agent = json.load(f)
    except FileNotFoundError:
        agent = {"id": rec["id"], "state": {}, "offsets": {}}
    agent["version"], agent["status"] = rec["version"], rec.get("status")
    for change in rec.get("changes", []):
        node, key = _place(agent["state"], change)
        if "set" in change:
            node[key] = change["set"]
            agent["offsets"].pop(_name(change), None)
            continue
        offset = agent["offsets"].get(_name(change), 0)
        current = node[key] if isinstance(node, list) else node.get(key)
        have = current if isinstance(current, list) else None
        at = change["from"] - offset
        if have is None or at < 0 or at > len(have):
            node[key] = change["items"]
            agent["offsets"][_name(change)] = change["from"]
        else:
            node[key] = have[:at] + change["items"]
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(agent, f)
    os.replace(tmp, path)


def decode(v):
    """Tagged JSON (`{"$": ...}`) as Python values."""
    if isinstance(v, list):
        return [decode(x) for x in v]
    if not isinstance(v, dict):
        return v
    tag = v.get("$")
    if tag is None:
        return {k: decode(x) for k, x in v.items()}
    if tag == "map":
        return {_hashable(decode(k)): decode(x) for k, x in v["entries"]}
    if tag == "set":
        return {_hashable(decode(x)) for x in v["values"]}
    if tag == "object":
        return {k: decode(x) for k, x in v["fields"].items()}
    if tag == "instance":
        return dict({k: decode(x) for k, x in v["fields"].items()}, __class__=v["class"])
    if tag == "undefined":
        return None
    if tag == "number":
        return float(v["value"].replace("Infinity", "inf"))
    return v


def _hashable(v):
    if isinstance(v, dict):
        return tuple(sorted(v.items()))
    if isinstance(v, list):
        return tuple(v)
    return v


class Mirror:
    """The agents in a mirror directory, read fresh on each access."""

    def __init__(self, directory):
        self.directory = directory

    def _load(self, agent_id):
        with open(_path(self.directory, agent_id)) as f:
            agent = json.load(f)
        if agent["id"] != agent_id:
            raise ValueError("mirror identity does not match the requested agent")
        return agent

    def __getitem__(self, agent_id):
        return decode(self._load(agent_id)["state"])

    def __iter__(self):
        return iter(self.ids())

    def __len__(self):
        return len(self.ids())

    def ids(self):
        agents = []
        for name in os.listdir(self.directory):
            if name.endswith(".json"):
                with open(os.path.join(self.directory, name)) as f:
                    agents.append(json.load(f)["id"])
        return sorted(agents)

    def version(self, agent_id):
        return self._load(agent_id)["version"]

    def status(self, agent_id):
        return self._load(agent_id)["status"]

    def offset(self, agent_id, name):
        """Where a list read with `tail` starts (`name`: "key" or "key.path")."""
        return self._load(agent_id)["offsets"].get(name, 0)


def main():
    if len(sys.argv) != 3 or sys.argv[1] != "apply":
        sys.exit("usage: affordance_mirror.py apply DIR  (JSON lines on stdin)")
    os.makedirs(sys.argv[2], exist_ok=True)
    n = 0
    for line in sys.stdin:
        if line.strip():
            apply_line(sys.argv[2], line)
            n += 1
    print(json.dumps({"applied": n}))


if __name__ == "__main__":
    main()
