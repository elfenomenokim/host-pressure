# Security policy

## Supported versions

Security fixes go into the latest release. Upgrade before reporting, in case the problem is
already fixed.

| Version | Supported |
| --- | --- |
| 0.1.3 | Yes |
| Older | No |

## Reporting a vulnerability

Report it privately, not in a public issue: use the
[Report a vulnerability](https://github.com/elfenomenokim/host-pressure/security/advisories/new)
form on the repository's Security tab.

Include the command, the version (`host-pressure --version`), your platform, and what an attacker
could do with the result. You get a reply within a few days. Say how you want to be credited if you
want a name in the release note.

## In scope

- A write outside the documented baseline file path, or a file created with wider permissions than
  the caller's umask allows.
- Reading a host file that the documented command has no reason to read.
- Output that reports an `ok` verdict when the documented rules say otherwise, in a way an attacker
  can trigger (for example, a forged baseline file).
- A crash or unbounded loop driven by a crafted baseline file, threshold file or template.

## Not a vulnerability

- A verdict you disagree with because the thresholds do not fit your workload. That is a
  configuration issue; open an issue with the readings.
- A missing sensor reading on a platform or a virtual machine that does not expose it. A missing
  reading is reported as `null` on purpose.
- Running `--watch` for as long as you leave it running.

## Disclosure

Once a fix is released, the advisory is published with credit to the reporter unless they prefer
otherwise.
