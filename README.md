# Symbiote

Persistent artificial life ecosystem written entirely in Rust.

Symbiote is a real-time terminal-rendered biosphere focused on:
- emergence
- territorial memory
- adaptive ecology
- lineage evolution
- morphology-aware behavior
- long-run ecosystem persistence
- migration topology
- ecological reinforcement
- ecosystem readability
- procedural infrastructure growth

This is not a traditional game.

Symbiote is designed as a living procedural ecosystem where:
- organisms evolve
- ecological pressure accumulates
- migration lanes emerge
- territory stabilizes
- infrastructure persists
- species rise and collapse
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
- ecosystem storytelling through behavior
- morphology-aware rendering
- behavioral readability
- adaptive substrate density
- field-responsive motion

The ecosystem intentionally begins sparse and evolves naturally through:
- survival
- reproduction
- migration
- ecological pressure
- adaptive reinforcement
- territorial attraction
- ecological avoidance
- lineage adaptation

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
- reinforcement corridors
- ecological affinity pressure

The field actively influences:
- ecosystem behavior
- territorial formation
- migration topology
- ecological balancing
- movement pressure
- long-run world structure

The field is evolving toward:
> an ecosystem nervous system.

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
- stabilized organism lanes
- corridor ecosystems
- ecological bottlenecks

The simulation preserves traces of prior ecological states, allowing the world to develop historical continuity.

---

## Morphology-Aware Organisms

Organisms now visually express:
- role specialization
- density state
- ecological pressure
- territorial behavior
- movement identity
- cluster structure
- adaptive behavior classes

Rendering is now heavily behavior-first rather than pure particle density.

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

# Visual Identity

Symbiote intentionally avoids:
- overwhelming particle spam
- unreadable density
- excessive visual clutter
- brute-force rendering
- meaningless chaos rendering

Instead the ecosystem emphasizes:
- contrast
- migration readability
- ecological topology
- persistent infrastructure
- territorial behavior
- ecosystem aging
- foreground organism clarity
- long-run readability
- behavioral visualization
- sparse-space ecology

The empty space is part of the ecology.

---

# Screenshots

## Long-Run Ecosystem State

Current mature worlds now visibly develop:
- territorial lanes
- persistent root infrastructure
- adaptive migration behavior
- ecological segmentation
- long-run species turnover
- corridor ecosystems
- stable migration highways
- ecological districts
- reinforcement structures
- colony-like formations

(Add updated screenshots from current v0.13+ builds here.)

---

# Architecture

## Core Modules

```text
src/
├── main.rs        # ultra-thin boot entry
├── app.rs         # ecosystem orchestration/runtime ownership
├── sim.rs         # core simulation logic
├── render.rs      # ecosystem rendering + readability systems
├── field.rs       # PatternField ecosystem memory
├── pattern.rs     # Conway-inspired pattern classification
├── cluster.rs     # formations + colony systems
├── species.rs     # lineage + mutation drift
├── particle.rs    # organism behavior/state
├── ecology.rs     # ecological balancing pressure
├── automata.rs    # substrate/root cellular systems
├── memory.rs      # ecosystem persistence systems
├── tree.rs        # trunk/root generation
└── density.rs     # adaptive density governance
```

---

## Important System Roles

### app.rs

Top-level ecosystem orchestration:
- lifecycle management
- spawning
- telemetry
- PatternField ownership
- reproduction pressure
- reset/randomization
- runtime ecosystem governance

---

### sim.rs

Core simulation engine:
- movement
- ecology interaction
- reproduction
- field influence
- behavioral pressure
- archetype logic
- territorial navigation
- adaptive response behavior

---

### field.rs

Persistent ecological memory layer:
- migration traces
- stability fields
- danger pressure
- growth reinforcement
- territorial memory
- corridor persistence
- ecological reinforcement

---

### render.rs

Terminal ecosystem visualization:
- organism rendering
- overlays
- telemetry
- cluster visualization
- field underlays
- substrate hierarchy
- morphology-aware readability
- behavioral foreground emphasis

---

### ecology.rs

Environmental pressure systems:
- ecological balancing
- adaptive ecosystem behavior
- environmental pressure shaping
- overcrowding response
- ecosystem stabilization

---

# Requirements

Symbiote requires:
- Rust
- Cargo
- a terminal supporting ANSI colors
- Unicode rendering support

Recommended:
- Linux
- macOS
- Windows Terminal
- Kitty
- Alacritty
- WezTerm
- modern true-color terminal emulators

---

# Installing Rust

If Rust is not already installed:

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
- restart VSCode/Windows Terminal if open

Verify:

```powershell
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

## Debug Build

```bash
cargo build
cargo run
```

---

## Optimized Release Build

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
- species adaptation pressure
- field-responsive navigation
- territorial migration fronts
- ecology-aware population balancing

---

# Release History

## v0.8.5 — Vertical Growth

Introduced major vertical ecosystem expansion systems.

---

## v0.8.6 — Root Growth Stable

Stabilized root infrastructure and substrate reinforcement systems.

---

## v0.9.0 — PatternField Emergence

Integrated persistent ecological memory and territorial reinforcement behavior.

---

## v0.10.0 — Territorial Intelligence

Introduced ecosystem-aware movement pressure and territorial affinity drift.

---

## v0.11.0 — Field Polarity Response

Added field polarity response systems and ecological corridor reinforcement.

---

## v0.11.1 — Visual Hierarchy Refinement

Reduced substrate rendering density and improved long-run ecosystem readability.

---

## v0.12.0 — Morphology-Aware Rendering

Introduced morphology-aware organism rendering and behavioral visual identity systems.

---

## v0.13.0 — Behavioral Readability

Improved mature ecosystem readability through:
- organism foreground clarity
- territorial structure visibility
- migration topology readability
- adaptive substrate thinning
- behavioral visualization refinement

---

# Philosophy

Symbiote is an experiment in:
- living procedural systems
- artificial ecology
- long-run emergence
- persistent digital environments
- ecosystem intelligence
- memory-driven simulation
- behavior-first visualization

The goal is not scripted gameplay.

The goal is believable artificial existence.

---

# License

MIT License
