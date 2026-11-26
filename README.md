# Sawbones

An open-source, agent-first Maker's bench, and a portable test bench that docks into it.

Sawbones is Old West slang for a field surgeon. This is one for machines: a carry-case of instruments that an AI agent operates over MCP, so it can observe, measure and, behind a safety layer, act on the thing under test. Inverters, battery packs, older vehicles, radios, appliances, a breadboard on the bench.

**Status: design phase.** Nothing here is built or bench-tested yet. This repo holds the design document and the research behind it. Code follows once the design has been through outside review.

## What is in here

| Path | What it is |
| --- | --- |
| [`docs/project-document.md`](docs/project-document.md) | The design document: concept, architecture, module standard, knowledge store, safety layer, hardware, budget, risks. |
| [`docs/DECISIONS.md`](docs/DECISIONS.md) | The decision log, including the calls that were reversed and why. |
| [`research/`](research/README.md) | The raw research notes behind the document, 21 lanes in three rounds, with confidence and known gaps for each. |

Work is tracked in the [GitHub Project](https://github.com/users/rafe-walker/projects) for this repo.

## The idea in five lines

1. **Agent-first.** Anything a person could read off a screen or a knob, the agent reads as data, along with the health of the instrument producing it.
2. **One supervisor.** A Raspberry Pi 5 runs one Rust supervisor that powers, identifies, reads and reflashes every instrument, and exposes them through one MCP server.
3. **A module standard.** Instruments are plug-in modules on a 42 mm grid with USB-C, a shared firmware core and a machine-readable manifest, so others can build modules.
4. **Evidence over assertion.** An equipment knowledge store keeps every fact on a hash chain with a trust tier. Only measured or owner-attested facts can justify a write to hardware.
5. **Safety matched to danger.** A physical-safety layer, working name Interlock, sorts every action into a hazard class. Readings need no approval. Hazardous or hard-to-undo actions need the owner's yes each time.

## Scope and limits

- Security testing is in scope only against equipment and networks the owner owns, on the owner's property.
- Jamming and deliberate disruption of radio are out of scope, always.
- The project does not ship exploit or malware code. It wraps established assessment tools.
- Mains and PV string switching are never done by the agent.
- Nothing in this repo is electrical, automotive or legal advice. The PV procedure in particular has not been reviewed by a qualified solar electrician.

## How this was written

The document is a working design conversation between the project owner and Claude, the AI agent the bench is designed for. "You" in the document is the owner and "I" is Claude. The research was run by Claude research agents on 25 November 2026. Every lane's confidence and main gap is listed in [`research/README.md`](research/README.md). Treat prices marked estimate or unverified as unchecked.

## Licence

Apache-2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
