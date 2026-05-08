use std::fs;

fn replace_once(src: &mut String, from: &str, to: &str) {
    if src.contains(to) {
        return;
    }

    if !src.contains(from) {
        panic!("missing patch anchor:\n{}", from);
    }

    *src = src.replacen(from, to, 1);
}

fn main() {
    let mut main_rs = fs::read_to_string("src/main.rs").expect("read src/main.rs");
    if !main_rs.contains("mod life;") {
        main_rs = main_rs.replace("mod memory;", "mod memory; mod life;");
        fs::write("src/main.rs", main_rs).expect("write src/main.rs");
    }

    let mut app = fs::read_to_string("src/app.rs").expect("read src/app.rs");

    replace_once(
        &mut app,
        "    field::{FieldConfig, PatternField},\n    memory::MemoryBank,",
        "    field::{FieldConfig, PatternField},\n    life::AxiomLattice,\n    memory::MemoryBank,",
    );

    replace_once(
        &mut app,
        "    pub substrate: CellularAutomata,\n    pub pattern_field: PatternField,\n    pub memory: MemoryBank,",
        "    pub substrate: CellularAutomata,\n    pub pattern_field: PatternField,\n    pub axiom_lattice: AxiomLattice,\n    pub memory: MemoryBank,",
    );

    replace_once(
        &mut app,
        "            substrate: CellularAutomata::new(seed ^ 0xC011, 96, 48),\n            pattern_field: PatternField::new(96, 48, FieldConfig::default()),\n            memory: MemoryBank::load_or_new(seed),",
        "            substrate: CellularAutomata::new(seed ^ 0xC011, 96, 48),\n            pattern_field: PatternField::new(96, 48, FieldConfig::default()),\n            axiom_lattice: AxiomLattice::new(seed ^ 0xA011_0C1C, 96, 48),\n            memory: MemoryBank::load_or_new(seed),",
    );

    replace_once(
        &mut app,
        "        self.pattern_field.step();\n        self.reinforce_pattern_field_from_clusters();",
        "        self.pattern_field.step();\n\n        if self.age % 4 == 0 {\n            self.axiom_lattice.tick_b3s23();\n        }\n\n        if self.age > 0 && self.age % 360 == 0 {\n            let axiom = self.axiom_lattice.stats();\n            self.push_event(&format!(\n                \"axiom lattice {:?} gen:{} live:{} birth:{} death:{}\",\n                axiom.state, axiom.generation, axiom.alive, axiom.births, axiom.deaths\n            ));\n        }\n\n        self.reinforce_pattern_field_from_clusters();",
    );

    replace_once(
        &mut app,
        "        self.substrate = CellularAutomata::new(self.seed ^ self.age ^ 0xC011, 96, 48);\n        self.pattern_field = PatternField::new(96, 48, FieldConfig::default());",
        "        self.substrate = CellularAutomata::new(self.seed ^ self.age ^ 0xC011, 96, 48);\n        self.pattern_field = PatternField::new(96, 48, FieldConfig::default());\n        self.axiom_lattice = AxiomLattice::new(self.seed ^ self.age ^ 0xA011_0C1C, 96, 48);",
    );

    replace_once(
        &mut app,
        "            substrate: state.substrate,\n            pattern_field: PatternField::new(96, 48, FieldConfig::default()),\n            memory: state.memory,",
        "            substrate: state.substrate,\n            pattern_field: PatternField::new(96, 48, FieldConfig::default()),\n            axiom_lattice: AxiomLattice::new(state.seed ^ state.age ^ 0xA011_0C1C, 96, 48),\n            memory: state.memory,",
    );

    fs::write("src/app.rs", app).expect("write src/app.rs");
}
