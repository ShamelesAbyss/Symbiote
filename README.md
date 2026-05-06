# Symbiote

A Rust artificial life simulation focused on emergent behavior, evolving particle ecosystems, Conway-inspired pattern formation, archetype evolution, cluster intelligence, ecological pressure systems, and persistent spatial memory.

Built entirely in Rust.

---

# Vision

Symbiote is not just a particle simulation.

The long-term goal is a living artificial ecosystem where:
- particles evolve into recognizable archetypes
- clusters form tribes, colonies, halos, nests, and swarmfronts
- local rules generate emergent structures
- spatial memory influences future movement
- organisms inherit pattern tendencies
- ecosystems develop migratory regions and territorial behavior
- Conway-style oscillators, gliders, and stable forms naturally emerge

The project combines:
- artificial life
- cellular automata
- particle systems
- ecology simulation
- procedural emergence
- terminal visualization
- evolutionary behavior

---

# Core Features

## Emergent Particle Ecosystem
Particles move, interact, cluster, reproduce, mutate, and evolve over time through simple local rules.

Each particle contains:
- energy
- mass
- health
- velocity
- genome traits
- species identity
- cluster identity
- rare traits
- archetype influence

---

## Cluster Formation
Particles naturally organize into:
- tribes
- swarms
- halos
- membranes
- lattice structures
- nests
- drifting colonies

Clusters track:
- cohesion
- stability
- drift heat
- membrane integrity
- dominant archetypes
- velocity
- lifetime
- formation complexity

---

## Archetype System
Symbiote contains evolving behavior archetypes such as:
- Harvester
- Reaper
- Swarmer
- Parasite
- Orbiter
- Rooted forms
- Bloom structures
- Oscillator-style entities

Archetypes influence:
- movement
- reproduction
- aggression
- clustering
- ecological interaction
- formation tendencies

---

# Conway-Inspired Pattern System

## src/pattern.rs
The pattern layer classifies local structures into higher-level emergent forms.

Current pattern types include:
- Dormant
- StillLife
- Oscillator
- Glider
- Halo
- Lattice
- Bloom
- Chain
- Swarmfront
- Nest

---

# Spatial Pattern Fields

## src/field.rs
The field layer creates persistent spatial memory across the world.

It stores:
- pattern intensity
- cohesion
- pulse
- drift
- stability
- danger
- motion vectors
- dominant pattern signatures

This allows the ecosystem to eventually develop:
- migration lanes
- territorial regions
- glider trails
- persistent nest zones
- attractor fields
- ecological scars
- pattern memory over time

---

# Runtime Architecture

## src/main.rs
Application entry point and module declarations.

## src/app.rs
Top-level orchestration layer.

## src/sim.rs
Core particle simulation engine.

## src/particle.rs
Defines particles and genomes.

## src/species.rs
Species identity and archetype behavior.

## src/cluster.rs
Cluster intelligence and formation behavior.

## src/ecology.rs
Environmental pressure and ecological zones.

## src/automata.rs
Cellular automata and substrate systems.

## src/render.rs
Terminal visualization layer.

## src/memory.rs
Telemetry and long-term ecosystem metrics.

---

# Build

## Clone

```bash
git clone git@github.com:ShamelesAbyss/Symbiote.git
cd Symbiote
```

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --release
```

---

# Development Workflow

Preferred workflow:
- Rust-only
- GitHub as source of truth
- surgical modifications
- preserve architecture
- compile/check after every integration layer

Validation cycle:

```bash
cargo fmt
cargo check
cargo build --release
cargo run --release
```

---

# GitHub

https://github.com/ShamelesAbyss/Symbiote
