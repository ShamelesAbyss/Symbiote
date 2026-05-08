# Symbiote

Persistent artificial life ecosystem written entirely in Rust.

Symbiote is a real-time terminal-rendered biosphere focused on:
- emergence
- territorial memory
- adaptive ecology
- lineage evolution
- Conway-inspired substrate behavior
- colony formation
- long-run ecosystem intelligence

This is not a traditional game and not a fixed simulation sandbox.

Symbiote is designed as a living procedural ecosystem where:
- organisms evolve
- ecological pressure accumulates
- migration lanes emerge
- territory stabilizes
- infrastructure persists
- species rise and collapse
- cells form Conway-inspired patterns
- colonies develop behavioral memory
- the world develops historical memory over time



---

# Current Ecosystem Focus

Symbiote now emphasizes:
- adaptive ecological behavior
- territorial reinforcement
- long-run ecosystem persistence
- meaningful sparse-space readability
- emergent migration systems
- lineage drift and species turnover
- ecological memory fields
- procedural infrastructure growth
- Conway-inspired cellular emergence
- colony propagation and behavioral pressure
- ecosystem storytelling through behavior
- trophic ecological cycling
- live ecological telemetry
- archetype observability
- computational biome readability

The ecosystem intentionally begins sparse and evolves naturally through:
- survival
- reproduction
- migration
- ecological pressure
- adaptive reinforcement
- local cell birth/survival rules
- cluster persistence
- colony memory

Density is earned by the ecosystem itself over time.



---

# Core Systems

## PatternField Ecology

The PatternField system acts as persistent ecological memory.

It stores and reinforces:
- danger
- growth
- cohesion
- drift
- stability
- migration traces
- territorial pressure
- ecosystem history

The field actively influences ecosystem behavior and long-run topology while remaining visually restrained to preserve readability.



---

## Territorial Reinforcement

The world gradually develops:
- ecological districts
- migration corridors
- root highways
- territorial seams
- persistent settlement regions
- abandoned ecological zones
- infrastructure-like reinforcement structures

The simulation preserves traces of prior ecological states, allowing the world to develop historical continuity.



---

## Conway-Inspired Cellular Ecology

The substrate layer now includes simple but fundamental cellular emergence rules:
- live cells survive with 2 or 3 live neighbors
- live cells die from underpopulation
- live cells die from overpopulation
- dead or empty substrate can birth life with exactly 3 live neighbors
- spores, nutrients, and dead cells can support near-birth propagation
- roots remain structural barriers instead of becoming ordinary cells

This gives the cell system room to generate:
- spontaneous local structures
- oscillating pockets
- propagation fronts
- terraces
- dead-cell wakes
- substrate recovery regions
- ecological chambers

without hardcoding specific pattern types.



---

## Colony Propagation Ecology

Clusters now act as a bridge between individual organisms and emergent species behavior.

Colonies can develop pressure based on:
- age
- stability
- density
- membrane strength
- movement speed
- territorial anchoring
- drift heat

This allows:
- old settled colonies to stabilize
- moving fronts to become more distinct
- dense clusters to reinforce membrane-like structures
- structural archetypes to emerge naturally

without adding hardcoded behavior scripting.



---

## Trophic Ecology

Symbiote now supports emergent trophic pressure cycles.

The ecosystem can naturally develop:
- prey blooms
- grazer pressure
- harvester cleanup behavior
- apex predator emergence
- collapse and recovery cycles
- ecological succession

Current trophic archetypes include:
- Grazers
- Harvesters
- Hunters
- Reapers
- structural colony organisms
- drifting ecological specialists

The ecosystem now visibly transitions through:
substrate growth → population bloom → trophic pressure → collapse → recovery.



---

## Artificial Life Simulation

Symbiote combines:
- artificial life systems
- Conway-inspired emergence pressure
- ecological balancing
- procedural biology
- cluster intelligence
- species mutation drift
- substrate growth systems
- terminal-rendered ecosystem visualization

without becoming deterministic or scripted.



---

# Observability and Telemetry

The ecosystem now includes live ecological instrumentation.

Current telemetry includes:
- active archetype counts
- peak archetype counts
- trophic balance reporting
- live species emergence tracking
- ecological density reporting
- cluster telemetry
- substrate telemetry
- corridor pressure analysis
- root pressure observation
- long-run ecological memory

This allows the simulation to function like a readable computational ecology console instead of opaque particle noise.



---

# Visual Identity

Symbiote intentionally avoids:
- overwhelming particle spam
- unreadable density
- excessive visual clutter
- over-rendered atmosphere layers

Instead the ecosystem emphasizes:
- contrast
- migration readability
- ecological topology
- persistent infrastructure
- territorial behavior
- ecosystem aging
- meaningful negative space
- readable cellular emergence
- structural readability
- organism motion clarity

The empty space is part of the ecology.



---

## Long-Run Ecosystem State

Long-running worlds can develop:
- territorial lanes
- persistent root infrastructure
- adaptive migration behavior
- ecological segmentation
- long-run species turnover
- Conway-style cellular terraces
- moving colony fronts
- dead-cell wake trails
- rooted ecological barriers
- propagation seams
- trophic collapse/recovery cycles
- apex predator waves
- structural ecological districts



---

# Architecture

Core modules:

```text
src/
├── main.rs
├── app.rs
├── sim.rs
├── render.rs
├── field.rs
├── pattern.rs
├── cluster.rs
├── species.rs
├── particle.rs
├── ecology.rs
├── automata.rs
├── memory.rs
├── tree.rs
└── density.rs
```



## Important System Roles

### app.rs

Top-level ecosystem orchestration:
- lifecycle management
- spawning
- telemetry
- PatternField ownership
- reproduction pressure
- substrate cadence
- Conway generation timing
- reset/randomization
- archetype accounting
- ecological observation routing



### sim.rs

Core simulation engine:
- movement
- ecology interaction
- reproduction
- field influence
- behavioral pressure
- archetype logic
- particle/cell interaction
- field polarity response
- trophic pressure
- substrate cleanup behavior
- apex predator pressure



### field.rs

Persistent ecological memory layer:
- migration traces
- stability fields
- danger pressure
- growth reinforcement
- territorial memory
- motion memory



### pattern.rs

Morphology interpretation layer:
- Conway-inspired pattern signatures
- oscillator/still life/front classification
- morphology roles
- structural pressure
- pattern glyph/readout support



### render.rs

Terminal ecosystem visualization:
- organism rendering
- overlays
- telemetry
- cluster visualization
- field underlays
- substrate thinning
- visual hierarchy refinement
- archetype observability
- trophic telemetry
- readability-first rendering philosophy



### cluster.rs

Colony and group identity layer:
- cluster detection
- colony age
- stability
- membranes
- drift heat
- territorial anchoring
- behavioral colony pressure



### species.rs

Lineage and taxonomy layer:
- species creation
- genome blending
- archetype derivation
- extinction tracking
- lineage naming
- trophic role emergence
- structural archetype emergence
- environmental identity drift



### ecology.rs

Environmental pressure systems:
- ecological balancing
- adaptive ecosystem behavior
- environmental pressure shaping
- sparse macro-pressure zones



### automata.rs

Cellular substrate layer:
- root barriers
- life cells
- nutrients
- dead cells
- spores
- mutagens
- nests
- Conway-style survival/birth rules
- near-birth propagation pressure



### tree.rs

Root infrastructure policy:
- root growth
- trunk/branch/canopy staging
- root caps
- directional growth pressure
- barriers and surface flow



### density.rs

Density observation layer:
- cell density
- root density
- particle pressure
- crowding/refill signals
- ecological occupancy analysis



### memory.rs

Long-run ecosystem memory:
- peak population
- peak cells
- peak clusters
- dominant archetype
- historical pressure
- adaptive substrate/corridor signals
- archetype telemetry
- trophic state tracking
- long-run ecological statistics



---

# Requirements

Symbiote requires:
- Rust
- Cargo
- A terminal that supports ANSI colors and Unicode rendering

Recommended:
- Linux
- macOS
- Windows Terminal
- modern terminal emulators with true color support



---

# Installing Rust

## Linux / macOS

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation:

```bash
source "$HOME/.cargo/env"
```

Verify installation:

```bash
rustc --version
cargo --version
```



---

## Windows

Install Rust using:
https://rustup.rs/

After installation:
- restart terminal
- verify:

```bash
rustc --version
cargo --version
```



---

# Cloning Symbiote

## HTTPS

```bash
git clone https://github.com/ShamelesAbyss/Symbiote.git
cd Symbiote
```

## SSH

```bash
git clone git@github.com:ShamelesAbyss/Symbiote.git
cd Symbiote
```

---

# Building Symbiote

## Debug build

```bash
cargo build
cargo run
```

## Optimized release build

```bash
cargo build --release
cargo run --release
```



---

# Controls

| Key | Action |
|-----|--------|
| q | Quit |
| space | Pause simulation |
| r | Reset ecosystem |
| n | Generate new world seed |
| + | Increase simulation speed |
| - | Decrease simulation speed |



---

# Ecosystem Evolution Roadmap

Current active development targets:
- adaptive population pressure
- territorial intelligence
- field-guided navigation
- migration reinforcement
- ecological affinity systems
- cluster colony behavior
- lineage inheritance
- emergent sub-archetypes
- ecosystem nervous system behavior
- Conway-style substrate emergence
- spontaneous pattern propagation
- colony memory pressure
- behavioral species drift
- long-run ecological succession
- structural ecology
- biome-scale adaptation
- ecosystem observability
- trophic evolution



---

# Release History

## v0.8.5 — Vertical Growth
Introduced major vertical ecosystem expansion systems.

## v0.8.6 — Root Growth Stable
Stabilized root infrastructure and substrate reinforcement systems.

## v0.9.0 — PatternField Emergence
Integrated persistent ecological memory and territorial reinforcement behavior.

## v0.10.0 — Territorial Intelligence
Introduced ecosystem-aware movement pressure and territorial affinity drift.

## v0.11.0 — Field Polarity Response
Added archetype-aware PatternField polarity response, improving territorial settlement, danger corridor differentiation, and stable-region inhabitation.

## v0.12.0 — Morphology-Aware Rendering
Improved pattern visibility and ecosystem readability through morphology-driven visual refinement.

## v0.13.0 — Observability and Visual Hierarchy
Improved readouts, visual hierarchy, substrate thinning, negative space clarity, and long-run ecosystem observation.

## v0.14.0 — Emergent Colony Propagation
Added Conway-style cell survival and birth pressure, near-birth propagation support, reduced particle overwrite pressure, and behavioral colony emergence pressure.

## v0.15.0 — Trophic Ecology Foundations
Introduced early trophic balancing systems, archetype ecological restructuring, and predator/prey emergence tuning.

## v0.16.0 — Emergent Trophic Ecology
Expanded ecological role emergence, renderer restraint philosophy, and long-run trophic pressure cycling.

## v0.16.1 — Trophic Emergence Rebalance
Rebalanced Harvester and Reaper emergence thresholds, improved archetype stabilization, and enabled healthier ecological cycling.

## v0.17.0 — Observable Computational Ecology
Introduced full live archetype accounting, trophic telemetry, structural archetype emergence improvements, renderer readability breakthroughs, and long-run ecological observability.



---

# Philosophy

Symbiote is an experiment in:
- living procedural systems
- artificial ecology
- long-run emergence
- persistent digital environments
- ecosystem intelligence
- memory-driven simulation
- Conway-inspired cellular propagation
- colony behavior emerging from local pressure
- observable computational ecosystems

The goal is not scripted gameplay.

The goal is believable artificial existence.

The project is guided by one core principle:

```text
Complex life should emerge from simple interacting pressure systems.
```


---

# License

MIT License
