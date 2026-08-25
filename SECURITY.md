# Security Policy

## Supported Versions

SpeakerLab is a desktop calculation tool. It does not open network ports,
run as a server, or execute downloaded code. Security issues are therefore
limited to local files and third-party dependencies.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | ✅ |

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not open a public
GitHub issue**.

Instead, report it privately:

1. Use GitHub's private vulnerability reporting:
   [Report a vulnerability](https://github.com/EgorLikhachev/SpeakerLab/security/advisories/new)
2. Or email the maintainer at **<your-email@example.com>**
   (replace with the project maintainer's contact address).

Please include:

- A description of the issue and its potential impact
- Steps to reproduce (input files, project files, or driver JSON that
  trigger it)
- Affected version (see the `version` field in `Cargo.toml`)

You will receive a response within a few days. We credit reporters in the
release notes unless you prefer to stay anonymous.

## Scope

In scope:

- Malformed `.spkproj` / driver-library JSON causing crashes or panics
- Path handling issues when saving/loading files
- Vulnerable third-party dependencies (`cargo audit` findings)

Out of scope:

- Physical accuracy of acoustic models (not a security matter — file a
  regular bug report instead)
- Building from a compromised toolchain
