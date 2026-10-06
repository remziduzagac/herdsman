# Security policy

## Supported versions

Security fixes go into the latest release and the preview channel. Older releases do not get
them; update with `herdsman update`, or through your package manager for Homebrew, mise, Nix,
apt, dnf and pacman installs.

## Reporting a vulnerability

Report it privately: open the repository's
[Security tab](https://github.com/remziduzagac/herdsman/security) and choose **Report a
vulnerability**, or go straight to the
[report form](https://github.com/remziduzagac/herdsman/security/advisories/new). Please do not
open a public issue or discussion for it.

Include the version (`herdsman --version`), how you installed it, your operating system, the
steps that show the problem and what an attacker could do with it.

The maintainer aims to acknowledge a report within seven days. A fix ships in a release or
preview, and the advisory is then published, crediting you unless you ask not to be named.

## Scope

In scope: the `herdsman` binary, including its socket API, CLI, client and server, the install
scripts, the signed apt, dnf and pacman repositories, the Homebrew formula and the release
workflows. Problems in agents or other programs you run inside herdsman are out of scope unless
herdsman makes them worse.
