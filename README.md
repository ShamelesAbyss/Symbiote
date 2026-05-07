# Symbiote

Persistent artificial life ecosystem written entirely in Rust.

Symbiote is a real-time terminal-rendered biosphere focused on emergence, territorial memory, adaptive ecology, lineage evolution, and long-run ecosystem behavior.

This is not a traditional game and not a fixed simulation sandbox.

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

The ecosystem intentionally begins sparse and evolves naturally through:
- survival
- reproduction
- migration
- ecological pressure
- adaptive reinforcement

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

The field actively influences ecosystem behavior and long-run topology.

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

Instead the ecosystem emphasizes:
- contrast
- migration readability
- ecological topology
- persistent infrastructure
- territorial behavior
- ecosystem aging

The empty space is part of the ecology.

---

# Screenshots

## Long-Run Ecosystem State

- territorial lanes
- persistent root infrastructure
- adaptive migration behavior
- ecological segmentation
- long-run species turnover

(Add updated screenshots here from current builds.)

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
- reset/randomization

### sim.rs
Core simulation engine:
- movement
- ecology interaction
- reproduction
- field influence
- behavioral pressure
- archetype logic

### field.rs
Persistent ecological memory layer:
- migration traces
- stability fields
- danger pressure
- growth reinforcement
- territorial memory

### render.rs
Terminal ecosystem visualization:
- organism rendering
- overlays
- telemetry
- cluster visualization
- field underlays

### ecology.rs
Environmental pressure systems:
- ecological balancing
- adaptive ecosystem behavior
- environmental pressure shaping

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
- verify:

```bash
rustc --version
cargo --version
```

---

# Cloning Symbiote

## HTTPS (recommended for most users)

```bash
git clone https://github.com/ShamelesAbyss/Symbiote.git
cd Symbiote
```

## SSH (for contributors/dev environments)

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

---

# Philosophy

Symbiote is an experiment in:
- living procedural systems
- artificial ecology
- long-run emergence
- persistent digital environments
- ecosystem intelligence
- memory-driven simulation

The goal is not scripted gameplay.

The goal is believable artificial existence.

---

# License

MIT License
