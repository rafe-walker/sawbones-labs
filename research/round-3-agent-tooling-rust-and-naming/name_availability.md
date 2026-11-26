# Name availability and conflict check: agent-operated Maker's bench and sensor pack

Checked 2026-11-25/26 (UTC). This is a lookup, not legal advice and not a trademark clearance; no trademark-office search was run.

## Bottom line

- **Best venture name: Sawbones**, with the GitHub organization **`sawbones-labs`**. Crates `sawbones`, `sawbones-core` and `sawbones-mcp` are free, the unscoped npm name is free, `sawbones.dev` has no registration, and nothing with that name turned up in electronics, test gear, robotics, AI agents or developer tools.
- **Second: Sixgun**, with **`sixgun-labs`**. It has the cleanest registry picture of any single word (crate, npm package, npm scope and `sixgun.dev` all free), but it is a firearm word on a radio-sensing device. If that is unwelcome, **Bench Wraith** (`benchwraith`, free everywhere including the bare GitHub name) takes its place.
- **Dragoon fails**, despite the local tie: a drone company called Dragoon Technology exists, another project already owns a `dragoon-*` crate family, and the GitHub name, `.dev` and `.io` are gone.
- 47 of the 62 candidates are taken as bare GitHub names. Every bare name that appears free is a compound (`benchwraith`, the `*bench` and `*labs` forms, `assayoffice`) or a sub-name (`bunkhouse`, `conchos`).
- Several Western words were claimed during 2026 by AI-agent tooling: `waypost`, `tackroom`, `farrier`, `latigo` (npm) and `trailhand` (GitHub org and domains). These are the conflicts closest to this project.

## How each check was done, and its limits

| Check | Source | What a result means |
|---|---|---|
| GitHub login | `https://api.github.com/users/<name>` and `https://api.github.com/search/users?q=<name>+in:login&per_page=100`, with `https://github.com/<name>` for a few | TAKEN is firm (account data returned). APPEARS FREE means the login search has no exact match and the users endpoint returned an error. The fetch tool does not expose status codes, so a 404 could not be told apart from other client errors; two names that errored on the users endpoint (`outrider`, `packrat`) do exist. Confirm on the GitHub sign-up form. GitHub can also refuse a name that returns 404. |
| GitHub variants | Same login search, reading the result list for `-labs`, `labs`, `bench`, `-hq`, `-dev`, `get<name>`, `-project`, `-io` | Run for venture names only. Sub-names would be repositories under the new org, so their variants were not checked. Lists over 100 hits are incomplete and say so. |
| crates.io | `https://index.crates.io/<path>/<name>` for existence (hyphen and underscore spellings), `https://crates.io/api/v1/crates/<name>` for detail | Firm. |
| npm package | `https://registry.npmjs.org/<name>` and `https://api.npmjs.org/downloads/point/last-week/<name>` | Firm. |
| npm scope | `https://registry.npmjs.org/-/org/<name>/package` (200 means the scope belongs to a user or org; 404 says 'Scope not found') | The page `https://www.npmjs.com/org/<name>` was blocked by a bot challenge, so this registry endpoint was used instead. 'EXISTS (0 pkgs visible)' is usually a personal account holding the name. |
| Domains | Registry RDAP: `https://pubapi.registry.google/rdap/domain/<d>` (.dev), `https://rdap.identitydigital.services/rdap/domain/<d>` (.io .sh .tools .systems), `https://rdap.verisign.com/com/v1/domain/<d>` (.com) | R = registered. 'none' = no registration found; the name can still be registry-reserved or premium-priced, which was not checked. `rdap.org` has no route for .sh, so registries were queried directly. |
| Conflicts | Web search plus GitHub repository search and crates.io keyword search for the shortlist | [V] = seen in this run. [K] = general knowledge, not re-verified in this run. The web search tool returned titles and links only, so conflict coverage is thinner than the registry checks. |

Direct shell access to `github.com/<name>` was refused by this session's network policy (403), which is why GitHub went through the fetch tool.

## Venture / overall project: registries

| Name | GitHub login | crates.io | npm | Domains |
|---|---|---|---|---|
| `outrider` | TAKEN: user, 0 public repos (profile page). 101 logins contain the word, incl. Outrider-Labs, Outrider-HQ, Outrider-Dev, outrider-ai, outrider-eng | `outrider` taken (v0.3.0, 169 dl, 102 recent, updated 2026-10: LXMF boundary crate in the retinue family: message codec, delivery, pr); `-core` free; `-mcp` free | pkg taken (v2.0.0, updated 2022-06-23, 13/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `dragoon` | TAKEN: user, 19 repos, 44 followers, joined 2010, active Nov 2026 | `dragoon` free; `-core` free; `-mcp` free | pkg taken (v0.0.0, updated 2022-04-29, 4/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `sawbones` | TAKEN: user, 14 repos, 1 follower, joined 2013, profile last changed 2016 (looks dormant) | `sawbones` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (2 pkgs visible) | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `dowser` | TAKEN: user, 12 repos, joined 2016, active Nov 2026 | `dowser` taken (v0.19.0, 60223 dl, 1230 recent, updated 2026-10: A recursive, canonicalizing file finding library for Unix.); `-core` free; `-mcp` free | pkg free; @scope EXISTS (7 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `familiar` | TAKEN: user, 8 repos, 71 followers, joined 2023, active 2026 | `familiar` taken (v0.1.1, 1634 dl, 6 recent, updated 2024-12: reserve); `-core` free; `-mcp` free | pkg taken (v0.2.0, updated 2022-06-17, 23/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `synesthete` | TAKEN: user, 7 repos, joined 2009, active Nov 2026 | `synesthete` free; `-core` free; `-mcp` free | pkg taken (v1.1.1, updated 2022-06-27, 16/wk); @scope EXISTS (0 pkgs visible) | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `gremlinhunter` | TAKEN: user, 2 repos, 0 followers, joined 2024-10-28, untouched since that day (dormant) | `gremlinhunter` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none · bench.com none |
| `haruspex` | TAKEN: user, 1 repo, joined 2013, last changed 2025 | `haruspex` taken (v1.2.0, 17320 dl, 496 recent, updated 2026-11: Vulnerability research assistant that extracts pseudocode from the IDA); `-core` free; `-mcp` free | pkg taken (v0.0.0, updated 2026-11-17, 55/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools R · .systems R · .com R · bench.com none |
| `farrier` | TAKEN: user, 0 repos, 0 followers, joined 2012 (dormant, empty) | `farrier` free; `-core` free; `-mcp` free | pkg taken (v0.7.0, updated 2026-08-07, 17/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools R · .systems none · .com R · bench.com none |
| `lodestone` | TAKEN: user, 410 repos, 107 followers, joined 2008, active | `lodestone` taken (v0.5.0, 10252 dl, 12 recent, updated 2024-10: A website wrapper for FFXIV's lodestone.); `-core` taken; `-mcp` free | pkg taken (v0.1.2, updated 2022-06-19, 10/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems R · .com R · bench.com none |
| `benchwraith` | APPEARS FREE: login search 0 hits; users API and profile page both returned a client error | `benchwraith` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `sixgun` | TAKEN: user, 0 repos, 0 followers, joined 2014, last changed 2016 (dormant, empty) | `sixgun` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `wranglerbench` | APPEARS FREE: login search 0 hits | `wranglerbench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |
| `tinhorn` | TAKEN: user, 13 repos, joined 2013, active Nov 2026 | `tinhorn` taken (v0.2.2, 215 dl, 16 recent, updated 2026-07: A dice cup for your terminal: shake the cup, watch dice fly, hear them); `-core` taken; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `roughnecklabs` | APPEARS FREE: login search 0 hits | `roughnecklabs` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `chuckwagon` | TAKEN: user, 2 repos, joined 2014, last changed 2021 (dormant) | `chuckwagon` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `trailhand` | TAKEN: organization, 0 public repos, created 2026-10-20 (five weeks old) | `trailhand` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `waypost` | TAKEN: user, 1 repo, joined 2015, last changed 2026-05 | `waypost` free; `-core` free; `-mcp` free | pkg taken (v0.17.0, updated 2026-11-19, 238/wk); @scope EXISTS (2 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems none · .com R · bench.com none |
| `ridgelinebench` | APPEARS FREE: login search 0 hits | `ridgelinebench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |
| `ocotillo` | TAKEN: user, 1 repo, joined 2017, last changed 2020 (dormant) | `ocotillo` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `javelina` | TAKEN: user, 2 repos, joined 2014, last changed 2016 (dormant) | `javelina` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (1 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `saguarolabs` | APPEARS FREE: login search 0 hits | `saguarolabs` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `monsoonbench` | APPEARS FREE: login search 0 hits | `monsoonbench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |
| `caliche` | TAKEN: user, 1 repo, joined 2011, last changed 2020 (dormant) | `caliche` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `arroyo` | TAKEN: user, 22 repos, joined 2010, active Nov 2026 | `arroyo` taken (v0.7.0, 3351 dl, 13 recent, updated 2023-10: Arroyo is a distributed stream processor that lets users ask complex q); `-core` free; `-mcp` free | pkg taken (v1.0.0, updated 2022-06-13, 5/wk); @scope EXISTS (1 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `mesquite` | TAKEN: user, 1 repo, joined 2015, last changed 2016 (dormant) | `mesquite` taken (v0.26.1, 848 dl, 848 recent, updated 2026-11: A pure, safe Rust port of the tree-sitter 0.25.10 runtime); `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `dustdevil` | TAKEN: user, 0 repos, 0 followers, joined 2014, last changed 2016 (dormant, empty) | `dustdevil` free; `-core` free; `-mcp` free | pkg taken (v0.0.4, updated 2022-06-16, 8/wk); @scope not found | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `chaparral` | TAKEN: user, 1 repo, joined 2013, last changed 2017 (dormant) | `chaparral` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `coyotebench` | APPEARS FREE: login search 0 hits | `coyotebench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |
| `roadrunnerbench` | APPEARS FREE: login search 0 hits | `roadrunnerbench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |
| `packrat` | TAKEN: user, 3 public repos, 2 followers (profile page; dates not obtained) | `packrat` taken (v0.2.1, 2728 dl, 3 recent, updated 2022-05: A simple terminal UI for updating your package.json dependencies); `-core` free; `-mcp` free | pkg taken (v0.1.0, updated 2022-06-23, 4/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems none · .com R · bench.com none |
| `jackalope` | TAKEN: organization 'Jackalope', 15 repos, created 2009, last changed 2020 | `jackalope` free; `-core` free; `-mcp` free | pkg taken (vNone, updated 2025-06-19, None/wk); @scope EXISTS (1 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems R · .com R · bench.com none |
| `hoodoo` | TAKEN: user, 24 repos, joined 2010, last changed 2026-06 | `hoodoo` free; `-core` free; `-mcp` free | pkg taken (v0.0.1, updated 2022-06-18, 9/wk); @scope EXISTS (8 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `gila` | TAKEN: user, 38 repos, joined 2015, last changed 2025-11 | `gila` free; `-core` free; `-mcp` free | pkg taken (v0.1.0-beta.8, updated 2022-06-18, 13/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `thunderbirdbench` | APPEARS FREE: login search 0 hits | `thunderbirdbench` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com none |

## Venture / overall project: conflicts and meaning

| Name | Conflicts | Meaning and usability |
|---|---|---|
| `outrider` | SERIOUS. Outrider (autonomous yard-truck company; its staff accounts and outrider-ai / outrider-eng orgs show in the GitHub login search) [K+V]. Outriders video game [K]. Crate `outrider` is an active 2026 mesh-radio messaging crate [V]. | Mounted escort who rides ahead of or beside the main party. Positive, easy to say and spell; search results owned by the truck company and the game. |
| `dragoon` | SERIOUS. Dragoon Technology (dragoon.tech) builds long-range unmanned aircraft ('Cinder', DIU Project Artemis) [V: suasnews.com 2025-05]. A `dragoon-*` crate family (dragoon-proto, -server, -worker, -ctl, -testkit; Apr 2026) and `dragoonfri` already sit on crates.io [V]. The Legend of Dragoon, Final Fantasy 'Dragoon', Stryker Dragoon vehicle [K]. | Mounted infantryman; as a verb, to coerce. The Dragoon Mountains are local, which is the appeal. Easy to say and spell; search is dominated by games. |
| `sawbones` | LOW. SAWBONES is the brand of Pacific Research Laboratories for orthopedic and medical training models, with a US trademark record (serial 73226682; status not read) [V: sawbones.com, trademark.justia.com]. 'Sawbones' medical-history podcast [K]. Sawbones-Games GitHub org (game community servers) [V]. Nothing found in electronics, test gear, robotics, AI agents or dev tools. | Old slang for a surgeon, slightly irreverent; reads as 'frontier doctor for machines'. Easy to say and spell; search shares space with the podcast and the medical-model company. |
| `dowser` | MODERATE. Crate `dowser` is active (Blobfolio file finder, 60k downloads) [V]. Dowser Python memory profiler [K]. `dowserlabs` org exists [V]. | One who finds hidden water with a rod. Pseudoscience connotation is awkward for a measuring instrument. Easy to say and spell. |
| `familiar` | SERIOUS. Familiar Machines & Magic (iRobot co-founder Colin Angle) unveiled an AI companion robot called 'Familiar' in May 2026 [V: therobotreport.com, techradar.com, engadget.com]. GitHub orgs familiarmachines and familiar-ai exist [V]. | A witch's spirit companion; also an everyday adjective, so it cannot be searched for. |
| `synesthete` | LOW. No product found (no dedicated web search run); npm `synesthete` is a small CLI colorizer [V]. | Person whose senses blend (hears colours), apt for sensor fusion. Hard to spell and pronounce; many will not know it. |
| `gremlinhunter` | LOW to MODERATE. No product with this exact name found [V: search]. 'Gremlin' alone is Gremlin Inc. (chaos-engineering software), the Apache TinkerPop graph query language, the AMC car and the film franchise [K]; the compound lowers but does not remove the software-mark adjacency. | A gremlin is the imp blamed for unexplained equipment faults (WWII aviation slang), so this is 'hunter of intermittent faults'. Self-explanatory and fun; long (13 letters) but easy to say, spell and search. |
| `haruspex` | MODERATE to SERIOUS in security tools. Crate `haruspex` is an active vulnerability-research tool (IDA Hex-Rays pseudocode extraction, updated Nov 2026) [V]. npm `haruspex` claimed Nov 2026 [V]. Haruspex cyber-risk firm (Italy) [K]. | Roman diviner who read entrails. Divination connotation; hard to spell and say. |
| `farrier` | MODERATE. npm `farrier` is a 2026 tool to 'create, understand, validate, and evolve reliable coding-agent harnesses' [V], which is AI-agent tooling. | Craftsman who shoes horses. Positive craft word; many people will not know it; spelling is fine. |
| `lodestone` | SERIOUS / crowded. Crates `lodestone` and `lodestone-core` taken [V]. The Lodestone (Final Fantasy XIV community site), Lodestone Security [K]. LodestoneHQ, lodestonelabs and three more variant accounts exist [V]. | Naturally magnetic rock; something that attracts. Positive, easy. |
| `benchwraith` | LOW. Nothing found under the exact name [V: search]. benchwraith.com was registered 2026-08-09 by an unknown party [V: RDAP], so someone had the same idea recently. 'Wraith' alone: AMD Wraith coolers, Rolls-Royce Wraith, BBC Wraith testing tool [K]. | Wraith is a ghost, so 'the ghost at the bench'. Cool, a little ominous, not Western. Easy to say and spell; unique in search. |
| `sixgun` | LOW in tech. No electronics, test-gear or software product found [V: search]. 'SIX GUN' trademark filings exist in unrelated classes, e.g. clothing (serial 97791113, dead) [V: markinton.com]. Six-Guns mobile game [K]. sixgun.io was registered 2026-10-27 by an unknown party [V]. | Six-shot revolver; Tombstone in one word. It is a firearm word, which can be awkward on a radio-receiving device carried through airports, and with ad platforms or school and corporate filters. Easy to say, spell and search. |
| `wranglerbench` | SERIOUS. Wrangler is a Jeep model and a jeans brand (registered marks), and `wrangler` is Cloudflare's main developer CLI [K]. A vehicle-diagnostic-capable 'Wrangler Bench' sits too close to the Jeep mark. | One who handles horses or livestock. Positive; long. |
| `tinhorn` | MODERATE. Crates `tinhorn` and `tinhorn-core` taken (2026 terminal dice app) [V]. | A pretentious person of little substance; a small-time gambler. Negative for a measuring tool. |
| `roughnecklabs` | LOW to MODERATE. roughnecklabs.com registered 2025-10-16 by an unknown party [V]. Nothing found in search [V]. Rubbermaid 'Roughneck' storage brand [K]. | Oil-rig worker; also a rowdy person. Mildly negative. |
| `chuckwagon` | LOW. NSAPlayset/CHUCKWAGON (2014 open hardware-implant security research project, 20 stars, dormant) and ITV/chuckwagon (Scala AWS Lambda toolkit, dormant) [V]. Chuck Wagon dog food brand [K]. | The cattle drive's mobile kitchen and tool wagon; the chuck box's lid folds down into a work table. Warm and friendly; food association. Easy to say and spell. |
| `trailhand` | MODERATE (unknown party active). GitHub org created 2026-10-20; trailhand.dev and trailhand.io both registered 2025-04-30 [V]. Web search found no product yet [V]. Sounds like Salesforce Trailhead [K]. | Cowboy working a cattle drive. Positive, easy. |
| `waypost` | SERIOUS. npm `waypost` is an active tool for AI coding agents ('shared ground for AI coding agents', v0.17.0, updated 2026-11-19) [V]. waypost-io, Waypost-labs and WaypostHQ orgs exist [V]. | Trail marker post. Positive, easy. |
| `ridgelinebench` | MODERATE. Honda Ridgeline pickup (registered vehicle mark) [K]. | Line along a mountain crest. Bland; long. |
| `ocotillo` | LOW. Intel's Ocotillo campus in Chandler, AZ [K]. `ocotillolabs` org exists [V]. | Spiny desert shrub that leafs out after rain. Pretty; spelling and pronunciation (oh-koh-TEE-yo) trip people up. |
| `javelina` | LOW. Javelina DNS (listed on Product Hunt and BetaList) and javelinasoftware.com [V: search]. University mascot [K]. | Collared peccary: scrappy desert animal that travels in squads. Regional charm; J is said as H (hah-vuh-LEE-nuh), so people who hear it cannot spell it. |
| `saguarolabs` | MODERATE (crowded regionally). saguarolabs.com registered since 2015 [V]. Saguaro Technologies, Saguaro Defense Systems, and the near-homophone 'Sahuaro Labs' software agency [V: search]. | Giant cactus, icon of Arizona (more Tucson and Phoenix than Cochise County). Said suh-WAH-ro, so spelling is a trap. |
| `monsoonbench` | MODERATE to SERIOUS in test gear. Monsoon Solutions' 'Power Monitor' is a known bench instrument for measuring device current [V: msoon.github.io manual]. | The summer storm season of southern Arizona. Positive; long. |
| `caliche` | LOW. Caliche energy-storage company [K]. CalicheDev account exists [V]. | Cement-hard desert soil layer that every local builder fights. Hard to pronounce (kuh-LEE-chee) and spell. |
| `arroyo` | SERIOUS. Arroyo is a Rust stream-processing engine (ArroyoSystems, acquired by Cloudflare) and owns the `arroyo` crate [K+V]. | Dry creek bed that flash-floods. Easy. |
| `mesquite` | MODERATE. Crate `mesquite` is a new, active pure-Rust tree-sitter runtime port (created Oct 2026) [V]. Mesquite phylogenetics software [K]. Four variant accounts exist [V]. | Desert tree; barbecue wood. Positive; 'qu' spelling is fine. |
| `dustdevil` | LOW. PlaidCloud/dustdevil (Python session handler, 23 stars, dormant) [V]. Crate `dust-devil` exists, so `dustdevil` would sit beside it [V]. Sounds like the Dirt Devil vacuum brand [K]. dustdevil.io registered 2026-03 [V]. | Small desert whirlwind. Playful; 'devil' may bother a few people. Easy. |
| `chaparral` | MODERATE. Chaparral Boats, Chaparral Motorsports, Chaparral racing cars, the Chaparral missile [K]. ChaparralLabs account exists [V]. | Dense desert shrubland. Double-r and single-l spelling is error-prone. |
| `coyotebench` | LOW to MODERATE. Ford 'Coyote' V8 engine [K]. | Clever desert canine; trickster. In the border region 'coyote' is also slang for a people-smuggler, which is awkward locally. |
| `roadrunnerbench` | MODERATE. Road Runner (Warner Bros. character, Plymouth car), RoadRunner PHP application server [K]. | Fast desert bird. Friendly; 15 letters is long for a crate prefix. |
| `packrat` | MODERATE. `packrat` is Posit's R dependency manager and a parsing technique [K]. Crate `packrat` taken (dormant since 2022) [V]. packratlabs, packrat-dev, packrat-io accounts exist [V]. | Desert woodrat that hoards shiny objects; a hoarder. Mildly negative for a venture, apt for a store. |
| `jackalope` | MODERATE. Jackalope fuzzer from Google Project Zero (security tool) [K]. Jackalope PHP content-repository project owns the GitHub org [V]. Three variant accounts exist [V]. | Mythical antlered jackrabbit. Whimsical; 'fake animal' is an odd signal for a measuring tool. |
| `hoodoo` | LOW to MODERATE. Hoodoo Ruby service framework [K]; npm scope has 8 packages [V]. | A rock spire, and also an African-American folk-magic tradition; as a verb, to jinx. Bad-luck and cultural-sensitivity problems. |
| `gila` | LOW to MODERATE. Gila CMS [K]. Every 4-letter domain checked is registered [V]. Also the name of a tribal community. | Gila monster (venomous lizard); the river. Said HEE-la, so people who hear it cannot spell it. |
| `thunderbirdbench` | SERIOUS. Mozilla Thunderbird, Ford Thunderbird, USAF Thunderbirds [K]. | Mythic bird of several Native American traditions (appropriation concern). 16 letters. |

## Venture names: GitHub variants

| Name | Variant findings |
|---|---|
| `outrider` | Taken: Outrider-Labs, Outrider-HQ, Outrider-Dev. Not seen: outriderbench, getoutrider, outrider-project (99 of 101 hits read). |
| `dragoon` | dragoon-labs, dragoonbench, dragoon-hq: 0 search hits each (appear free). dragoon-dev, getdragoon, dragoon-project: not in the first 100 of 952 hits (not confirmed). |
| `sawbones` | None of -labs, labs, bench, -hq, hq, -dev, dev, get-, -project, -io present (complete list of 26). `sawbones-labs`, `sawbones-hq`, `sawbonesbench` also errored on the users API (consistent with free). |
| `dowser` | dowserlabs is TAKEN (organization). Others not present (complete list of 39). |
| `familiar` | TAKEN: familiar-dev, familiar-ai, Familiar-HQ, FAMILIAR-project, familiarmachines. List incomplete (550 hits). |
| `synesthete` | No variants present (complete list of 5). |
| `gremlinhunter` | Only gremlinhunter and gremlinhunter9546 exist. `gremlin-hunter`: 0 hits. `gremlinhunter-labs`: users API errored (consistent with free). |
| `haruspex` | No variants present (complete list of 47). |
| `farrier` | No variants present (complete list of 66). |
| `lodestone` | TAKEN: LodestoneHQ, lodestonelabs, Lodestone-Labs, lodestonedev, lodestone-dev. Others not seen (list truncated, 115 hits). |
| `sixgun` | No variants present (complete list of 68). `sixgun-labs`, `sixgun-hq` also errored on the users API (consistent with free). |
| `tinhorn` | No variants present (complete list of 15). |
| `chuckwagon` | No variants present (complete list of 25). `chuckwagon-labs`: 0 hits. |
| `trailhand` | The organization is the only login containing the word. |
| `waypost` | TAKEN: waypost-io, Waypost-labs, WaypostHQ (complete list of 15). |
| `ocotillo` | ocotillolabs is TAKEN (organization). Others not present (complete list of 15). |
| `javelina` | No variants present (complete list of 37). |
| `caliche` | CalicheDev is TAKEN. Others not in first 100 of 204 (not confirmed). |
| `arroyo` | TAKEN: arroyoDev, ArroyoLabs, ArroyoSystems. List incomplete (1,130 hits). |
| `mesquite` | TAKEN: MesquiteProject, mesquitedev, mesquite-labs, mesquitelabs (complete list of 60). |
| `dustdevil` | No variants present (complete list of 26). |
| `chaparral` | ChaparralLabs is TAKEN. Others not present (complete list of 52). |
| `packrat` | TAKEN: packratlabs, packrat-dev, packrat-io (complete list of 79). |
| `jackalope` | TAKEN: jackalopelabs, Jackalope-Dev, JackalopeDev. List truncated (219 hits). |
| `hoodoo` | None seen in the first 100 of 156 (not confirmed). |
| `gila` | None seen in the first 100 of 11,857 (not confirmed). |

Names that appear free as bare logins (`benchwraith`, `wranglerbench`, `roughnecklabs`, `ridgelinebench`, `saguarolabs`, `monsoonbench`, `coyotebench`, `roadrunnerbench`, `thunderbirdbench`) need no variant.

## Portable pack: registries

| Name | GitHub login | crates.io | npm | Domains |
|---|---|---|---|---|
| `outrider` | TAKEN: user, 0 public repos (profile page). 101 logins contain the word, incl. Outrider-Labs, Outrider-HQ, Outrider-Dev, outrider-ai, outrider-eng | `outrider` taken (v0.3.0, 169 dl, 102 recent, updated 2026-10: LXMF boundary crate in the retinue family: message codec, delivery, pr); `-core` free; `-mcp` free | pkg taken (v2.0.0, updated 2022-06-23, 13/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `packmule` | TAKEN: organization, 1 repo, created 2019, changed 2026-11-21 (active) | `packmule` free; `-core` free; `-mcp` free | pkg taken (v1.1.3, updated 2022-06-23, 11/wk); @scope EXISTS (36 pkgs visible) | .dev R · .io R · .sh none · .tools R · .systems none · .com R · bench.com none |
| `saddlebag` | TAKEN: organization, 0 public repos, created 2016, last changed 2025 | `saddlebag` free; `-core` free; `-mcp` free | pkg taken (v0.0.1, updated 2022-05-16, 3/wk); @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `scout` | TAKEN: user, 0 repos, 8 followers, joined 2018 | `scout` taken (v2.8.0, 28581 dl, 85 recent, updated 2023-09: Friendly fuzzy finder for the command line); `-core` free; `-mcp` free | pkg taken (v0.1.0, updated 2022-06-26, 15/wk); @scope EXISTS (1 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `pathfinder` | TAKEN: user, 3 repos, joined 2010, last changed 2021 | `pathfinder` taken (v0.6.5, 47012 dl, 138 recent, updated 2019-07: Create nodes, clusters of nodes and connection in between.); `-core` free; `-mcp` taken | pkg taken (v0.3.0-1, updated 2022-06-23, 59/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `longrider` | TAKEN: user, 0 repos, 0 followers, created 2025-03-19 (empty) | `longrider` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `packrat` | TAKEN: user, 3 public repos, 2 followers (profile page; dates not obtained) | `packrat` taken (v0.2.1, 2728 dl, 3 recent, updated 2022-05: A simple terminal UI for updating your package.json dependencies); `-core` free; `-mcp` free | pkg taken (v0.1.0, updated 2022-06-23, 4/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems none · .com R · bench.com none |
| `possibles` | TAKEN: user, 1 repo, joined 2024, last changed 2026-07 | `possibles` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |

## Portable pack: conflicts and meaning

| Name | Conflicts | Meaning and usability |
|---|---|---|
| `outrider` | SERIOUS. Outrider (autonomous yard-truck company; its staff accounts and outrider-ai / outrider-eng orgs show in the GitHub login search) [K+V]. Outriders video game [K]. Crate `outrider` is an active 2026 mesh-radio messaging crate [V]. | Mounted escort who rides ahead of or beside the main party. Positive, easy to say and spell; search results owned by the truck company and the game. |
| `packmule` | MODERATE. PACK MULE is a live registered mark (serial 87070423; goods not read), Pack Mule is a brand of electric utility vehicles, and PAKMULE has its own trademark records [V: trademarkelite.com, usgr.com, trademark.justia.com]. packmule.com is a shipping service [V]. Active `packmule` GitHub org and an npm scope with 36 packages [V]. | Load-carrying mule. Humble and apt; easy. |
| `saddlebag` | LOW. Saddlebag Exchange (game-market tool) and pb33f/saddlebag-js (in-memory store) [V]. No hardware conflict found. | Bags slung over a saddle; frontier doctors carried 'physician's saddlebags'. Apt for a portable kit; easy. |
| `scout` | SERIOUS. Scout Motors, Scout APM, Docker Scout, ScoutSuite and many more [K]. Crate `scout` taken; every domain checked is registered, including scoutbench.com [V]. | One sent ahead to observe. Apt but unusable and unsearchable. |
| `pathfinder` | SERIOUS. Nissan Pathfinder, NASA Mars Pathfinder, the Pathfinder role-playing game [K]. Crates `pathfinder` and `pathfinder-mcp` taken [V]. | One who finds the way. Unusable. |
| `longrider` | LOW. Nothing notable found [V: search]. 'The Long Riders' 1980 film [K]. The GitHub login is an empty 2025 account [V]. | Outlaw or horseman who covers long distances; implies endurance, good for a battery pack. Easy to say, spell and search. |
| `packrat` | MODERATE. `packrat` is Posit's R dependency manager and a parsing technique [K]. Crate `packrat` taken (dormant since 2022) [V]. packratlabs, packrat-dev, packrat-io accounts exist [V]. | Desert woodrat that hoards shiny objects; a hoarder. Mildly negative for a venture, apt for a store. |
| `possibles` | LOW. Nothing found; crates.io keyword search is useless here because it matches 'possible' [V]. | A mountain man's 'possibles bag' held everything he might possibly need. Charming but obscure; reads like a typo and is impossible to search. |

## Fixed bench: registries

| Name | GitHub login | crates.io | npm | Domains |
|---|---|---|---|---|
| `roundhouse` | UNCONFIRMED: no exact login among the first 30 of 80 search hits and the users API errored; treat as not verified (many roundhouse-* accounts exist) | `roundhouse` taken (v0.4.7, 205 dl, 205 recent, updated 2026-11: An OpenAPI-first axum framework: declarations in, router and document ); `-core` free; `-mcp` free | pkg taken (v1.0.6, updated 2022-05-16, 32/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems none · .com R · bench.com none |
| `smithy` | TAKEN: user, 0 repos, 3 followers, joined 2010 | `smithy` taken (v0.0.7, 14437 dl, 162 recent, updated 2019-09: Smithy, a framework for web development); `-core` taken; `-mcp` free | pkg taken (v0.6.3, updated 2022-06-26, 840/wk); @scope EXISTS (64 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems none · .com R · bench.com none |
| `homestead` | TAKEN: organization, 0 public repos, created 2012, last changed 2013 (dormant) | `homestead` taken (v0.1.2, 70 dl, 70 recent, updated 2026-09: Local-first SQLite store with schema migrations, models, mutators, and); `-core` free; `-mcp` free | pkg taken (v0.2.3, updated 2026-06-25, 13/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `bunkhouse` | APPEARS FREE: complete search list (10 logins) has no exact match; users API errored | `bunkhouse` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |
| `tackroom` | TAKEN: organization, 0 public repos, created 2026-03-25 | `tackroom` free; `-core` free; `-mcp` free | pkg taken (v3.1.0, updated 2026-11-22, 918/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `assayoffice` | APPEARS FREE: login search 0 hits | `assayoffice` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |

## Fixed bench: conflicts and meaning

| Name | Conflicts | Meaning and usability |
|---|---|---|
| `roundhouse` | MODERATE. Crate `roundhouse` is a new, active Rust web framework (created 2026-10-29) [V]. npm `roundhouse` is a database migration utility [V]. | Locomotive maintenance shed with a turntable; also a kick. Good bench metaphor; easy. |
| `smithy` | SERIOUS. Smithy is AWS's interface-definition language and SDK generator; the npm scope holds 64 packages; crates `smithy` and `smithy-core` taken [K+V]. | Blacksmith's workshop. Apt but unusable. |
| `homestead` | MODERATE / crowded. Laravel Homestead, Ethereum 'Homestead' release [K]. Crate `homestead` (2026 local-first SQLite store) and npm `homestead` (2026 git-worktree dev environments) [V]. | A settler's home claim. Warm; easy. |
| `bunkhouse` | LOW. braedonsaunders/bunkhouse ('open-source AI employees for small businesses', 5 stars, 2026) [V]. bunkhouse.dev registered 2025-01 [V]. | Ranch hands' sleeping quarters; where the crew lives. Friendly, though it does not say 'workbench'. Easy. |
| `tackroom` | SERIOUS for this field. npm `tackroom` is an active AI-agent tool ('Dotfiles for your AI agents', v3.1.0, about 900 downloads a week, updated 2026-11-22); GitHub org created 2026-03; tackroom.io and .dev registered 2026 [V]. | Room where saddles and bridles are kept. Apt, easy. |
| `assayoffice` | LOW. No software or hardware project found (0 repositories, 0 crates) [V]. 'Assay Office' is the name of the UK hallmarking bodies and of historic US government offices [K]; a descriptive, official term, not a tech brand. | Frontier office where ore was tested for what it really contained; the best literal fit for a test-and-measurement bench, and true to Tombstone's silver boom. Risks: 'assay' gets typed as 'essay', and the run-together form contains 'ass'. |

## Module standard: registries

| Name | GitHub login | crates.io | npm | Domains |
|---|---|---|---|---|
| `tack` | TAKEN: organization, 9 repos, created 2012, last changed 2016 | `tack` taken (v0.1.1-alpha, 2739 dl, 5 recent, updated 2021-12: A minimal syntax highlighter that outputs HTML); `-core` taken; `-mcp` free | pkg taken (v0.1.0, updated 2022-06-27, 8/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `saddle` | TAKEN: organization, 6 repos, created 2013, last changed 2016 | `saddle` taken (v0.1.2, 4194 dl, 51 recent, updated 2025-09: Async seat managment made easy); `-core` taken; `-mcp` free | pkg taken (v0.9.7, updated 2022-06-26, 178/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems R · .com R · bench.com none |
| `hitch` | TAKEN: user, 2 repos, joined 2016, last changed 2022 | `hitch` taken (v0.1.1, 52 dl, 7 recent, updated 2026-03: CLI tool for easily logging into accounts that require a browser from ); `-core` free; `-mcp` free | pkg taken (v0.0.1, updated 2022-06-18, 7/wk); @scope EXISTS (4 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `lariat` | TAKEN: organization 'LArIAT' (physics experiment), 9 repos, created 2015 | `lariat` free; `-core` free; `-mcp` free | pkg taken (v2.0.1, updated 2023-01-31, 794/wk); @scope EXISTS (2 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems R · .com R · bench.com none |
| `brand` | NOT CHECKED: users API errored; search has 86,183 hits and the exact login could not be confirmed either way | `brand` taken (v0.0.1, 24 dl, 4 recent, updated 2026-05: Reinforcement learning for Rust. Backend-agnostic over modern Rust ML ); `-core` free; `-mcp` free | pkg taken (v999999999999999.9.9, updated 2022-06-13, 25/wk); @scope EXISTS (1 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `latigo` | TAKEN: user, 4 repos, joined 2014, last changed 2026-06 | `latigo` free; `-core` free; `-mcp` free | pkg taken (v0.0.0, updated 2026-07-19, 3/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `cinch` | UNCONFIRMED: no exact login in the first 100 of 378 search hits and the users API errored (cinch-labs, cinchapi, cinchrb exist) | `cinch` taken (v0.1.1, 48 dl, 6 recent, updated 2026-01: Redis-compatible cache infrastructure. Affordable, auto-scaling, file-); `-core` free; `-mcp` free | pkg taken (v0.1.1, updated 2022-06-13, 18/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `conchos` | APPEARS FREE: complete search list (5 logins) has no exact match. Singular `concho` is TAKEN (user) | `conchos` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none |

## Module standard: conflicts and meaning

| Name | Conflicts | Meaning and usability |
|---|---|---|
| `tack` | MODERATE. Crates `tack` and `tack-core` taken [V]; every domain checked is registered [V]. | Horse gear as a whole; also a pin, a sailing manoeuvre. Apt for add-on gear; unsearchable. |
| `saddle` | MODERATE. Saddle Finance (DeFi) [K]. Crates `saddle` and `saddle-core` taken (active 2025) [V]. | Seat on a horse. Easy; unsearchable. |
| `hitch` | MODERATE. Hitch TLS proxy [K]. Crate `hitch` taken (2026) [V]. All domains checked are registered, including hitchbench.com [V]. | Fastening knot or trailer coupling; also 'a snag', which is the wrong signal for a standard. |
| `lariat` | MODERATE. 'Lariat' is a Ford truck trim level (vehicle mark) [K], relevant because this project has vehicle interfaces. npm `lariat` is a Playwright test framework (about 800 downloads a week) and Zetier/lariat manages Android device farms [V]. Crate `lariat` is free [V]. | Rope for catching livestock. Easy to say and spell. |
| `brand` | SERIOUS / hopeless. Generic word; crate taken; all domains checked are registered, including brandbench.com [V]. | Ownership mark burned on cattle; also the marketing sense. Cannot be searched. |
| `latigo` | LOW to MODERATE. npm `latigo` was claimed in Jul 2026 as a 'disciplined dev-harness for LLM/agentic development' (placeholder v0.0.0) and latigo.dev was registered 2026-05 [V]. Lattigo (two t's) is a known Go cryptography library [K]. Crate `latigo` is free [V]. | Leather strap that fastens the cinch to the saddle, literally the connecting strap. Spanish for whip. Obscure but easy to spell once heard. |
| `cinch` | SERIOUS for hardware. Cinch Connectivity Solutions (Bel Fuse) is a long-established electronic connector brand sold through Digi-Key, Mouser and Newark [V: mouser.com, belfuse.com]; 'Cinch' is also the German name for RCA connectors [K]. Crate `cinch` taken (2026); all domains registered, including cinchbench.com [V]. | Saddle girth; 'a cinch' means easy. Great word, wrong industry to use it in. |
| `conchos` | LOW. Nothing found in tech (0 crates; trivial repositories) [V]. Reads as 'conchOS' in mixed case (two such repositories exist) [V]. Concho Resources oil company [K]. | Silver discs on saddles and belts that also anchor the saddle strings (tie points). Sits close to a vulgar word in some Spanish dialects. The plural is awkward; singular `concho` is free as a crate, on npm and on .dev/.io [V]. |

## Knowledge store: registries

| Name | GitHub login | crates.io | npm | Domains |
|---|---|---|---|---|
| `almanac` | TAKEN: user, 0 repos, 2 followers, joined 2012, last changed 2022 (dormant, empty) | `almanac` taken (v0.4.0, 13454 dl, 65 recent, updated 2021-09: Display calendar .ics in command line); `-core` free; `-mcp` free | pkg taken (v0.0.1-security, updated 2026-10-13, 71/wk); @scope EXISTS (0 pkgs visible) | .dev none · .io R · .sh R · .tools R · .systems R · .com R · bench.com none |
| `cairn` | TAKEN: organization 'Cairn Software', 7 repos, created 2025-10-21, active 2026 | `cairn` taken (v0.0.0, 258 dl, 214 recent, updated 2026-01: Build-gated version control for Rust projects - EXPERIMENTAL, NOT PROD); `-core` taken; `-mcp` taken | pkg taken (v0.8.0, updated 2022-06-13, 30/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `ledger` | TAKEN: organization 'Ledger', 13 repos, 68 followers, created 2012 | `ledger` taken (v0.2.5, 55375 dl, 2952 recent, updated 2019-03: Library to exchange APDU with Ledger Nano S devices); `-core` taken; `-mcp` free | pkg taken (v0.1.0, updated 2026-09-30, 23/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `tallybook` | TAKEN: organization, 0 public repos, created 2016-04-22, never changed (dormant, empty) | `tallybook` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `assay` | TAKEN: organization (seen in login search; details not obtained) | `assay` taken (v0.1.1, 81202 dl, 4907 recent, updated 2022-02: A super powered testing macro for Rust); `-core` taken; `-mcp` free | pkg taken (v1.0.0, updated 2022-06-13, 8/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools R · .systems R · .com R · bench.com R |
| `lodestone` | TAKEN: user, 410 repos, 107 followers, joined 2008, active | `lodestone` taken (v0.5.0, 10252 dl, 12 recent, updated 2024-10: A website wrapper for FFXIV's lodestone.); `-core` taken; `-mcp` free | pkg taken (v0.1.2, updated 2022-06-19, 10/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh R · .tools none · .systems R · .com R · bench.com none |
| `strongbox` | TAKEN: organization, 24 repos, created 2013, active Nov 2026 | `strongbox` taken (v0.1.0, 1510 dl, 5 recent, updated 2023-08: Tiny encrypted and authenticated data container); `-core` free; `-mcp` free | pkg taken (v1.0.0, updated 2022-06-27, 6/wk); @scope EXISTS (0 pkgs visible) | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |
| `brandbook` | TAKEN: organization 'Brandbook.io', 3 repos, created 2018, last changed 2019 | `brandbook` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none |

## Knowledge store: conflicts and meaning

| Name | Conflicts | Meaning and usability |
|---|---|---|
| `almanac` | MODERATE. Almanac documentation SaaS [K]. Crate `almanac` taken (calendar CLI, last release 2021) [V]. npm `almanac` is a security holding package, so npm will not hand it out [V]. | Yearly reference book of tables. Apt, positive, easy. |
| `cairn` | SERIOUS. Crates `cairn`, `cairn-core` and `cairn-mcp` are all taken [V]. 'Cairn Software' GitHub org (created 2025-10, active) [V]. Every domain checked is registered, including cairnbench.com [V]. Cairn Energy [K]. | Stack of stones marking a trail. Apt but taken. |
| `ledger` | SERIOUS. Ledger (hardware-wallet company) holds the name in hardware and security; crates `ledger` and `ledger-core` taken; all domains registered [K+V]. | Book of accounts. Unusable. |
| `tallybook` | LOW. wangjjj/TallyBook bookkeeping app (122 stars, 2017), magna-nz/tallybook (prices Claude Code and Codex sessions, has an MCP server, 6 stars, 2026), a TallyBook oil-and-gas phone app [V]. Tally Solutions accounting software [K]. | Pocket notebook a cowboy or driller keeps counts in. Humble, apt for a record store; easy. |
| `assay` | MODERATE. Crates `assay` (popular Rust testing macro, 81k downloads) and `assay-core` taken; `assay` GitHub org exists; all domains registered [V]. | Test of what a material really contains. Apt but taken. |
| `lodestone` | SERIOUS / crowded. Crates `lodestone` and `lodestone-core` taken [V]. The Lodestone (Final Fantasy XIV community site), Lodestone Security [K]. LodestoneHQ, lodestonelabs and three more variant accounts exist [V]. | Naturally magnetic rock; something that attracts. Positive, easy. |
| `strongbox` | MODERATE to SERIOUS in security. Strongbox password manager; Android 'StrongBox' hardware keystore [K]. `strongbox` GitHub org is an active artifact-repository project; crate taken [V]. | Stagecoach lockbox. Easy. |
| `brandbook` | MODERATE (meaning collision). 'Brand book' is the everyday marketing term for brand guidelines: 561 repositories carry the name, including AI-agent skills that generate them [V]. brandbook.dev registered 2026-10-28 [V]. | In ranching, the official registry of cattle brands (who owns what), which fits an equipment registry perfectly; nearly everyone will read it as marketing guidelines. |

## Ranking: top 5 venture names

| Rank | Name | GitHub org to register | Crates | npm | Domain to take | Main risk |
|---|---|---|---|---|---|---|
| 1 | Sawbones | `sawbones-labs` (alt `sawbones-hq`) | `sawbones`, `-core`, `-mcp` free | `sawbones` free; `@sawbones` scope exists, `@sawbones-labs` not found | `sawbones.dev` none; `sawboneslabs.com` none | SAWBONES medical-model brand in an unrelated field; `sawbones.io` registered by someone else on 2026-10-30 |
| 2 | Sixgun | `sixgun-labs` (alt `sixgun-hq`) | `sixgun`, `-core`, `-mcp` free | `sixgun` free; `@sixgun` not found | `sixgun.dev` none; `sixgunlabs.com` none | Firearm word; `sixgun.io` registered by someone else on 2026-10-27 |
| 3 | Bench Wraith | `benchwraith` | all free | pkg and scope free | `benchwraith.dev` none | `benchwraith.com` registered 2026-08-09 by an unknown party; not Western |
| 4 | Chuckwagon | `chuckwagon-labs` | all free | `chuckwagon` free; `@chuckwagon` scope exists | `chuckwagon.dev` and `chuckwagon.io` none | Food association; old NSA Playset hardware project of the same name |
| 5 | Gremlin Hunter | `gremlinhunter-labs` (bare name held by a one-day 2024 account) | all free | pkg and scope free | `gremlinhunter.dev`, `.io` and `.com` none | Adjacent to the Gremlin software mark; long |

1. **Sawbones.** The only single word on the list that is free on crates.io and npm, has a free `.dev`, and has no conflict in any related field. The meaning carries the product: a frontier doctor who diagnoses and patches up machines. The bare GitHub login belongs to a personal account whose profile last changed in 2016, so the org needs a suffix; every suffix checked is absent. One thing to watch: `sawbones.io` was registered on 2026-10-30 by an unknown party.
2. **Sixgun.** Strongest Tombstone flavour and the cleanest registries, and the pack's six sensor families (RF, microphones, cameras, multimeter, vehicle, solar) give the name a reason. It ranks second because a firearm word limits where the name can be printed and promoted, and because someone else registered `sixgun.io` four weeks ago.
3. **Bench Wraith.** The only strong candidate whose bare GitHub name appears free, and the easiest to search. It ranks third because it does not carry the Old West theme, so the sub-names (all Western) sit oddly under it, and because the `.com` was taken three months ago by someone unknown.
4. **Chuckwagon.** Clean registries and both `.dev` and `.io` unregistered. The chuck box at the back of the wagon was a fold-down work table with drawers of tools, which fits a portable bench. It ranks fourth because most people hear 'food'.
5. **Gremlin Hunter.** Says what the bench does (chases intermittent faults) and every registry and domain is open. It ranks fifth because 'Gremlin' is an established software mark and the name is 13 letters long as a crate prefix.

Considered and left out: **Dragoon** (drone company, occupied crate family, GitHub and main domains gone), **Javelina** and **Dust Devil** (usable, but `.dev`/`.io` for Javelina are gone and neither says anything about the product), **Trailhand** (an unknown party created the GitHub org five weeks ago and holds the domains), **Outrider, Familiar, Arroyo, Waypost, Lodestone, Haruspex** (direct conflicts).

## Themed sets for the top two

Sub-projects are repositories under the org, so only the bare crate name and conflicts matter. Publishing crates under the venture prefix (`sawbones-saddlebag`, `sixgun-longrider`) avoids any dependence on the bare names; the bare status is listed anyway.

### Set A: Sawbones (the frontier doctor's kit)

| Role | Name | Bare crate | npm | Conflict level | Why |
|---|---|---|---|---|---|
| Venture | Sawbones (`sawbones-labs`) | free | free | low | see ranking |
| Portable pack | Saddlebag | `saddlebag` free | pkg taken (dead 2022 placeholder); scope not found | low | A frontier doctor's kit rode in his saddlebags |
| Fixed bench | Assay Office | `assayoffice` and `assay-office` free | free | low | Where things are tested for what they really are |
| Module standard | Concho (singular of Conchos) | `concho` and `conchos` free | both free | low | The discs that anchor tie strings: attachment points |
| Knowledge store | Tally Book | `tallybook` and `tally-book` free | free | low | The pocket record book; fits a hash-chained log |

### Set B: Sixgun (Tombstone and the trail)

| Role | Name | Bare crate | npm | Conflict level | Why |
|---|---|---|---|---|---|
| Venture | Sixgun (`sixgun-labs`) | free | free | low in tech | see ranking |
| Portable pack | Longrider | `longrider` and `long-rider` free | pkg and scope free | low | Endurance away from base; all five non-.com domains unregistered |
| Fixed bench | Assay Office (alt Bunkhouse) | free | free | low | Tombstone's silver-camp institution; Bunkhouse is also free on crates and npm and appears free as a GitHub login |
| Module standard | Latigo | `latigo` free | pkg taken (Jul 2026 placeholder for an LLM dev harness); scope exists | low to moderate | The strap that connects cinch to saddle; use Concho instead if the npm placeholder matters |
| Knowledge store | Tally Book (alt Brand Book) | free | free | low | Brand Book fits an equipment registry better in ranch terms, but reads as marketing guidelines |

Assay Office and Tally Book appear in both sets because they are the only clean names in their slots: every other bench name has an active crate or AI-agent-tool conflict except Bunkhouse, and every other store name has its bare crate taken except Brand Book.

Names to avoid as sub-names whatever the venture: **Cinch** (connector manufacturer), **Smithy** (AWS), **Tack Room** (active AI-agent npm tool), **Ledger**, **Scout**, **Pathfinder**, **Cairn** (crate plus `-core` and `-mcp` all taken), **Lariat** (Ford trim, and the pack talks to vehicles), **Packmule** (an electric-utility-vehicle brand with a live registered mark).

One adjacent finding: the working name for the safety layer has its bare crate taken. `interlock` exists on crates.io (about 7,000 downloads, last release 2023) and `interlock-memory` appeared in 2026; `interlock-core` and `sawbones-interlock` are free.

## Additional original suggestions (lighter checks)

The listed single words are mostly gone as bare GitHub names, so here are eight more, drawn from Cochise County's mining and Army history. They got the same registry checks but no conflict search; conflict notes are general knowledge only.

| Name | Idea | GitHub login | crates.io | npm | Domains | Known conflicts [K] |
|---|---|---|---|---|---|---|
| `tommyknocker` | Mine spirit of Cornish hard-rock miners that knocks on the timbers to warn of danger: an unseen helper plus a safety layer | TAKEN (user); -labs, -hq, -dev absent | `tommyknocker` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none | Stephen King's 'The Tommyknockers'; Tommyknocker Brewery |
| `singlejack` | Hard-rock drilling done by one miner with a hammer and steel: the solo maker | TAKEN (user); -labs, -hq, -dev absent | `singlejack` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none | none known |
| `muleskinner` | Driver of the mule teams that hauled Tombstone's ore; the agent that drives the pack | TAKEN (user); -labs, -hq, -dev absent | `muleskinner` free; `-core` free; `-mcp` free | pkg free; @scope EXISTS (0 pkgs visible) | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none | 'Mule Skinner Blues' song |
| `jerkline` | The single rein that steered a long freight team: one line commanding many units, apt for the module bus | APPEARS FREE (0 search hits) | `jerkline` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io none · .sh none · .tools none · .systems none · .com R · bench.com none | contains 'jerk' |
| `heliograph` | Sun-mirror signalling network the Army ran across southeast Arizona's peaks in 1886: solar plus communications | TAKEN (user); -labs, -hq, -dev absent | `heliograph` taken (v0.1.0, 1762 dl, 4 recent, updated 2021-01: Medium-level bindings to System V semaphores); `-core` free; `-mcp` free | pkg taken (v7.0.0, updated 2023-03-15, 271/wk); @scope not found | .dev R · .io R · .sh none · .tools none · .systems none · .com R · bench.com none | generic instrument name |
| `stampmill` | Mill that crushed ore before assay; a bench name | APPEARS FREE (no exact match in 3 hits; StampMill-Works org exists) | `stampmill` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io none · .sh none · .tools none · .systems none · .com R · bench.com none | none known |
| `hardscrabble` | Earning a living from poor ground; grit | TAKEN (organization) | `hardscrabble` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev none · .io R · .sh none · .tools none · .systems none · .com R · bench.com none | Scrabble sound-alike |
| `skyisland` | The Dragoons and Chiricahuas are 'sky islands', isolated ranges above the desert | TAKEN (user) | `skyisland` free; `-core` free; `-mcp` free | pkg free; @scope not found | .dev R · .io R · .sh R · .tools none · .systems none · .com R · bench.com none | Sky Island (tourism and craft-beer names) |

## Evidence links

- Dragoon Technology: https://www.suasnews.com/2025/05/dragoon-selects-teledyne-flir-oem-prism-software-for-long-range-unmanned-platform-prototyping/ and https://dronelife.com/tag/dragoon-technology/
- Familiar Machines & Magic: https://www.therobotreport.com/inside-colin-angle-bid-build-companion-robots-familiar/ and https://techradar.com/ai-platforms-assistants/this-is-the-robot-i-wanted-to-build-forever-former-irobot-chief-on-his-extraordinary-familiar-ai-companion
- Sawbones (medical models): https://sawbones.com/about-us/ and https://trademark.justia.com/732/26/sawbones-73226682.html
- SIX GUN filings: https://markinton.com/trademark/six-gun-97791113 and https://trademark.justia.com/764/26/six-gun-76426032.html (second link's class not read)
- PACK MULE marks: https://www.trademarkelite.com/trademark/trademark-detail/87070423/PACK-MULE and https://trademark.justia.com/887/19/pakmule-88719613.html
- Cinch Connectivity Solutions: https://www.mouser.com/cinch-connectivity/ and https://www.digikey.com/en/product-highlight/b/bel-fuse/bel-solutions
- Monsoon Power Monitor: https://msoon.github.io/powermonitor/PowerTool/doc/LVPM%20Manual.pdf
- Javelina DNS: https://www.producthunt.com/p/javelina-dns
- Sahuaro Labs / Saguaro Technologies: https://clutch.co/profile/sahuaro-labs and https://builtin.com/company/saguaro-technologies
- Per-name registry URLs follow the patterns in the method table, e.g. https://crates.io/api/v1/crates/sawbones, https://registry.npmjs.org/sixgun, https://api.github.com/users/trailhand, https://api.github.com/search/users?q=benchwraith+in:login, https://pubapi.registry.google/rdap/domain/sawbones.dev

