# ◉ SYMBIOTE

**A living terminal organism.  
An artificial life system you don’t play… you witness.**

---

## ✦ Overview

SYMBIOTE is a terminal-based artificial life simulation written in Rust.

Inspired by particle-life systems, it uses simple attraction and repulsion rules between colored entities to produce **emergent behavior**, including:

- Clustering
- Flocking
- Orbital structures
- Predator-prey dynamics
- Membrane-like organism shells
- Evolution over time

The result is a **self-organizing digital ecosystem** that evolves the longer it runs.

---

## ✦ Features

### ◉ Living Particle System
- Hundreds of autonomous agents
- Each with unique genome traits:
  - Perception
  - Bonding
  - Hunger
  - Volatility
  - Orbit tendency
  - Membrane behavior

### ◉ Emergent Clusters
- True cluster identity tracking
- Clusters form, merge, split, and migrate
- Organisms behave as **cohesive moving bodies**

### ◉ Evolution Engine
- Natural selection via survival pressure
- Mutation of behavior rules
- Environmental shifts:
  - Calm
  - Bloom
  - Hunger
  - Storm
  - Drift

### ◉ Visual System
- ASCII / Unicode rendering
- Dynamic density-based glyphs
- Pulsing membranes
- Motion trails
- Directional cluster indicators

### ◉ Memory System
- Tracks:
  - Peak population
  - Cluster evolution
  - Merges / splits
  - Longest survival
- Stored at:
```
memory/session_memory.json
```

---

## ✦ Controls

```
space   pause / resume
e       toggle evolution
r       reset world
m       force mutation
n       new seed
+ / -   speed control
q       quit
```

---

## ✦ Installation

```bash
git clone <your-repo>
cd symbiote

cargo build --release
./target/release/symbiote
```

---

## ✦ Philosophy

SYMBIOTE is not a game in the traditional sense.

It is:

- A **digital petri dish**
- A **procedural life experiment**
- A system where:
  - order emerges from chaos
  - patterns appear, dissolve, and reappear
  - structure is never hardcoded

You don’t control it.

You **observe it evolve**.

---

## ✦ Future Direction

- Long-term evolutionary memory persistence
- Stable organism species emergence
- Complex predator hierarchies
- Persistent world states
- Exportable “organism seeds”

---

## ✦ Author

Built by Chris  
Powered by curiosity, chaos, and way too much terminal time

---

## ✦ License

MIT

