# Standards

Versioned working standards that Sawbones Labs measures its work against. Everything under `standards/` is a protected path: changes need the `guards-change` label and the clean-room reviewer, and `ci/check_standards.py` checks the records.

| Standard | Versions | Latest | Status |
| --- | --- | --- | --- |
| Hardware Write Safety (HWS): requirements for agents that write to physical hardware | [0.1](hws/0.1/README.md) | 0.1 | working draft, not released |

Each standard keeps a `CHANGELOG.md`. Each version is immutable in its own folder once a `RELEASED` file is added.
