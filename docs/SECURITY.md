# Security

Affordance runs code that a model may have written, and it can serve that code over HTTP. This
page says what an app can reach, how to contain it, and what Affordance does not do for you.

## What an app can reach

An app is TSX run by Affordance's interpreter. It has no filesystem, no sockets and no
processes of its own. Everything outside it goes through something the deployment binds:

- **Machines** (`machines` in the deployment): `bash` on this host (`local`), in a bubblewrap
  sandbox (`bwrap`, no network by default), in a Docker container (`docker`), over SSH, or a
  command of your own. An agent with no machine bound cannot run a command.
- **HTTP roles** (`http` machines): `fetch` reaches only the origin the deployment bound, with
  bounded request and response sizes. Credentials are inserted by the host from its
  environment; app code never sees them, and they are not written to the journal.
- **Platform APIs** (`send`, `store.*`): limited per agent by `capabilities`. Reading another
  agent's history (`harness.inspect`) is off unless the agent's spec lists the ids it may read.

So the deployment file is the security boundary. Read it the way you would read a container's
mounts and network settings.

## Code the model writes

`loadModule` compiles TSX that arrives as data, including source the model wrote. Loading reads
no files and makes no requests, but the loaded code runs with the reach of the app that loaded
it: the same machines, HTTP roles and capabilities. If an app lets a model edit its own harness,
give that agent only what you would give the model directly. In practice:

- bind a `bwrap` or `docker` machine, not `local`. A `bwrap` machine shows a command the
  system's programs, its own home and a private /tmp, with no network by default: your home
  directory, keys and tokens are not there for it. A `local` machine is your own shell;
- give it no HTTP roles it does not need, and narrow `capabilities`;
- keep the loader in a parent app that decides which revision runs, so a bad revision can be
  replaced by an earlier one.

## Serving

`affordance serve` speaks HTTP/1.1 and has no login. It listens on `127.0.0.1:8000` by
default and prints a warning when told to listen beyond the machine. A served app sees every
request's headers, so it can check an `Authorization` header and answer 401 itself; for TLS, put
a proxy (Caddy, nginx) in front.

A request is an input to an agent, and inputs are journaled so a run can be replayed. A
request's headers are part of it, so an `Authorization` header or a cookie a client sends is
written to the run's journal. The run directory is made readable by its user alone. Treat it
as you would a log that holds credentials: do not share it, and where a proxy can check a
credential and strip it before the request reaches the app, prefer that.

## The network

Every request Affordance makes goes through one HTTP client (`src/runtime/http.rs`): model
calls of every kind and an app's `fetch`. It is reqwest, on hyper and rustls. A server's
certificate must chain to a root the operating system trusts or to the Mozilla roots, so a private
certificate authority installed on the machine works. Proxies come from the environment
(`HTTPS_PROXY`, `http_proxy`, `ALL_PROXY`, `NO_PROXY`) and are never used for loopback addresses.
Redirects are not followed. What a response may hold is bounded, so a hostile server cannot make
the process allocate what it sends. Errors never include the server's address, because they reach
journals and traces.

`affordance serve` parses requests with hyper's server, and bounds a request's head (64 KiB,
30 s) and body (`max_body_bytes`).

## We want to hear about it

Affordance is at v0.1. If you find a way out of the boundary above, or a way to read what an
app should not, please open a private security advisory on the GitHub repository and we will
work on it with you.

# Dependency security

The release gate runs `python3 tools/check-security.py`, using `cargo-audit` and the RustSec
database. Install the auditor with `cargo install cargo-audit --version 0.22.2 --locked`.
New advisories fail the check. Python is development tooling; it is excluded from the published Rust package.

Three advisory matches have narrow exceptions in `tools/security-exceptions.json`. The check
fails once the file's `review_by` date passes, so each exception gets reviewed again:

| Dependency and advisory | Reviewed exposure |
|---|---|
| quick-xml 0.26.0: [0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html), [0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html) | Optional `profile` → pprof 0.15.0 → inferno 0.11.21. These affect XML reading, duplicate-attribute checking and namespace allocation. Our benchmark programs call `Report::flamegraph` on profiler-generated data. Inferno writes SVG using `quick_xml::Writer`; this path does not parse XML or use `NsReader`. |
| paste 1.0.15: [0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) | An unmaintained build-time procedural macro, used by the `tokenizers` crate. Nothing of it runs in Affordance's binary. This is a maintenance exception; there is no patched release to select. |

These are exceptions for Affordance's reviewed uses, not claims that those dependencies are
fixed or safe for arbitrary use. The check verifies exact versions and registry checksums,
hashes the reviewed profiler callsite, rejects newly introduced direct uses
of affected APIs, and expires the review. Changed dependency boundaries or callsites require
another review; the guard is not a formal whole-program reachability proof. Remove exceptions
when upstream publishes compatible fixes.

For a reproducible local run with an already fetched database:

```sh
CARGO_AUDIT=/path/to/cargo-audit \
AFFORDANCE_RUSTSEC_DB=/path/to/advisory-db AFFORDANCE_SECURITY_OFFLINE=1 ./tools/check.sh
```

Ordinary release checks fetch the current database. The offline options are for local
reproduction; an old database cannot establish the absence of newly published advisories.
