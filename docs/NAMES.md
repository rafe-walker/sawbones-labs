# Component names

The owner-approved names, one line each. The machine-readable copy is [`design/names.toml`](../design/names.toml); `ci/conformance.py` checks that every crate named there exists and that every name appears on this page. Requirement ids use the lower-case key: `SAW-CORTEX-LATCH-1` belongs to `cortex`.

| Name | Stands for | Crate(s) | What it is |
| --- | --- | --- | --- |
| C.O.R.T.E.X. | Coordinator Of Readings, Tools & EXperiments | `sawbones-supervisor`, `sawbones-policy`, `sawbones-approvals`, `sawbones-isolation`, `sawbones-reading` | The supervisor: decides what the agent may do, tracks facts and approvals, keeps outside text fenced. |
| N.E.R.V.E. | Node Exchange: Reliable, Verified, Enrolled | `sawbones-module-protocol` | The module protocol between the supervisor and each module. |
| P.U.L.S.E. | Panel-Unlocked Liveness & Safety Exchange | `sawbones-trust-root` (firmware in `firmware/trust-root`); the heartbeat itself is in `sawbones-interlock` | The heartbeat challenge between supervisor and trust root. |
| C.H.A.R.T. | Chain of Hashed, Attested Readings & Tests | none yet ([#109](https://github.com/rafe-walker/sawbones-labs/issues/109)) | The knowledge store. |
| T.R.I.A.G.E. | Tools Routed by Interlock And Gated Effects | `sawbones-mcp` | The MCP server. |
| Interlock | The physical-safety layer | `sawbones-interlock`, `sawbones-busguard`, `sawbones-modbus-encode` | Rails, leases, panel and bus guard enforced outside the Pi. |
| modules | Instruments and pods | `sawbones-sim-modules` | Anything with a manifest that plugs in. |
| plugins | Sandboxed decoders | `sawbones-plugin-host`, `sawbones-regmap` | Zero-import WebAssembly decoders. |
| S.A.W. | Sensory Agent Workstation | none | The portable pack. |
| T.A.B.L.E. | Test, Assembly, Build & Lab Environment | none | The home bench. |
| X-ray | Design conformance CI | none (`ci/conformance.py`) | Checks that the code matches the design records, including its own rules. |

The crate columns for Interlock, modules, plugins and the extra CORTEX crates are the working mapping and can change; the owner approved the names and the first five crate mappings.
