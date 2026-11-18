# Security

Report a vulnerability privately through GitHub's "Report a vulnerability" form on this
repository's Security tab. Please do not open a public issue for it.

Affordance runs untrusted model output against your machines: an agent's tools run the commands
its app allows, on the machines its deployment binds. The engine contains what a reply can do
(it can only name an action the render shows, with arguments its schema accepts), but the actions
themselves are the app's. Bind agents to machines you are prepared to let them use.

Dependency advisories and their reviewed exceptions are listed in
[docs/SECURITY.md](docs/SECURITY.md).
