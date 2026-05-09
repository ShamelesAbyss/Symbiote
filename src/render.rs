use crate::{
    app::{App, Environment},
    automata::{CellKind, SignalKind},
    life::AxiomPatternState,
    particle::Tribe,
    pattern::{
        pattern_glyph, pattern_strength_bar, MorphologyRole, PatternKind, PatternMotion,
        PatternSignature,
    },
    species::Archetype,
    tree::{self, TreeStage, TreeVisualPolicy},
};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame<'_>, app: &App) {
    let area = f.size();

    if area.width < 50 || area.height < 20 {
        let msg = Paragraph::new(vec![
            Line::from("SYMBIOTE"),
            Line::from("Terminal too small."),
            Line::from("Rotate phone or resize."),
            Line::from("q = quit"),
        ])
        .block(Block::default().borders(Borders::ALL).title(" SYMBIOTE "));

        f.render_widget(msg, area);
        return;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(f, root[0], app);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(root[1]);

    render_world(f, body[0], app);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(20),
            Constraint::Percentage(18),
            Constraint::Percentage(24),
        ])
        .split(body[1]);

    render_rules(f, side[0], app);
    render_clusters(f, side[1], app);
    render_species(f, side[2], app);
    render_events(f, side[3], app);
    render_metrics(f, root[2], app);
    render_footer(f, root[3]);
}

fn render_header(f: &mut Frame<'_>, area: Rect, app: &App) {
    let status = if app.paused { "paused" } else { "alive" };
    let pulse = ["░", "▒", "▓", "█", "▓", "▒"][(app.age as usize / 4) % 6];

    let override_count = app
        .clusters
        .clusters
        .iter()
        .filter(|cluster| cluster.archetype_override.is_some())
        .count();

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " ◉ SYMBIOTE ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "trunk-root lattice adaptive matrix signal ecology ",
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                pulse.repeat(12),
                Style::default().fg(env_color(app.environment)),
            ),
        ]),
        Line::from(vec![
            Span::styled(" age: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.age), Style::default().fg(Color::Green)),
            Span::styled(" | gen: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.generation),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(" | env: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.environment.name(),
                Style::default().fg(env_color(app.environment)),
            ),
            Span::styled(" | species: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.species_bank.active_count()),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(" | matrix: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}", app.matrix_pressure),
                Style::default().fg(matrix_color(app.matrix_pressure)),
            ),
            Span::styled(" | drift: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", override_count),
                Style::default().fg(if override_count > 0 {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled(" | trunk roots: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.substrate.protected_cells()),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | eaten: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.memory.total_cells_consumed),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" | reaped: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.memory.total_harvesters_consumed),
                Style::default().fg(Color::Red),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                status,
                Style::default().fg(if app.paused {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" LIFE CORE ")),
        area,
    );
}

fn render_world(f: &mut Frame<'_>, area: Rect, app: &App) {
    let width = area.width.saturating_sub(2) as usize;
    let height = area.height.saturating_sub(2) as usize;

    let mut cells: Vec<Vec<Cell>> = vec![vec![Cell::default(); width]; height];

    draw_substrate(&mut cells, app, width, height);
    draw_signal_trails(&mut cells, app, width, height);
    draw_ecology_zones(&mut cells, app, width, height);
    draw_pattern_field(&mut cells, app, width, height);
    draw_axiom_lattice(&mut cells, app, width, height);
    // force root layer to overwrite everything (no flicker)
    for y in 0..height {
        for x in 0..width {
            if camera_sample_substrate(app, x, y, width, height) == CellKind::Root {
                cells[y][x].substrate = Some((
                    match (x + y) % 4 {
                        0 => "│",
                        1 => "─",
                        2 => "┼",
                        _ => "┤",
                    }
                    .chars()
                    .next()
                    .unwrap(),
                    Color::Blue,
                ));
            }
        }
    }

    for particle in &app.particles {
        // --- ROOT VISUAL PRIORITY GUARD ---
        let px = particle.x as isize;
        let py = particle.y as isize;

        if px >= 0 && py >= 0 {
            let (px, py) = (px as usize, py as usize);
            if py < height && px < width {
                if camera_sample_substrate(app, px, py, width, height) == CellKind::Root {
                    continue; // do not draw particle over root
                }
            }
        }

        let Some((sx, sy)) = camera_world_to_screen(app, particle.x, particle.y, width, height)
        else {
            continue;
        };

        if camera_sample_substrate(app, sx, sy, width, height) == CellKind::Root {
            continue;
        }

        let x = sx as isize;
        let y = sy as isize;

        if x >= 0 && y >= 0 && x < width as isize && y < height as isize {
            let archetype = particle
                .cluster_id
                .and_then(|cluster_id| {
                    app.clusters
                        .clusters
                        .iter()
                        .find(|cluster| cluster.id == cluster_id)
                        .and_then(|cluster| cluster.effective_archetype())
                })
                .or_else(|| {
                    particle.species_id.and_then(|id| {
                        app.species_bank
                            .species
                            .iter()
                            .find(|species| species.id == id)
                            .map(|species| species.archetype)
                    })
                }); // RENDER_ARCHETYPE_SOURCE_UNIFIED

            let cell = &mut cells[y as usize][x as usize];

            cell.count += 1;
            cell.tribe_counts[particle.tribe.index()] += 1;

            if let Some(archetype) = archetype {
                cell.archetype_counts[archetype.index()] += 1;
            }

            cell.health += particle.health;
            cell.energy += particle.energy;
            cell.mass += particle.mass;

            if particle.cluster_id.is_some() {
                cell.clustered += 1;
            }

            if !particle.rare_trait.short().is_empty() {
                cell.rare = true;
            }

            if archetype == Some(Archetype::Harvester) {
                cell.harvester = true;
            }

            if archetype == Some(Archetype::Reaper) {
                cell.reaper = true;
            }

            if let Some(cluster_id) = particle.cluster_id {
                if app
                    .clusters
                    .clusters
                    .iter()
                    .any(|cluster| cluster.id == cluster_id && cluster.archetype_override.is_some())
                {
                    cell.drifting = true;
                }
            }
        }
    }

    draw_cluster_membranes(&mut cells, app, width, height);
    draw_cluster_motion_trails(&mut cells, app, width, height);

    let mut lines = Vec::new();

    for row in cells {
        let mut spans = Vec::new();

        for cell in row {
            if cell.trail {
                spans.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
            } else if cell.membrane {
                spans.push(Span::styled("○", Style::default().fg(Color::Gray)));
            } else if cell.count == 0 && cell.zone.is_some() {
                let (glyph, color) = cell.zone.unwrap();
                spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
            } else if cell.count == 0 && cell.substrate.is_some() {
                let (glyph, color) = cell.substrate.unwrap();
                spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
            } else if cell.count == 0 && cell.signal.is_some() {
                let (glyph, color) = cell.signal.unwrap();
                spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
            } else if cell.count == 0 && cell.axiom.is_some() {
                let (glyph, color) = cell.axiom.unwrap();
                spans.push(Span::styled(glyph.to_string(), Style::default().fg(color)));
            } else if cell.count == 0 {
                spans.push(Span::raw(" "));
            } else {
                let tribe = cell.dominant_tribe();
                let avg_health = cell.health / cell.count as f32;
                let avg_mass = cell.mass / cell.count as f32;
                let phase = cell.organic_phase(app.age);

                let morphology_role = cell_morphology_role(&cell, avg_mass);
                let morphology_flash =
                    morphology_render_glyph(morphology_role, app.age, cell.organic_phase(app.age));

                let dominant_archetype = cell
                    .archetype_counts
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, count)| *count)
                    .and_then(|(idx, count)| if *count > 0 { Some(idx) } else { None });

                let archetype_render_ready = dominant_archetype.is_some()
                    || cell.clustered > 0
                    || cell.count >= 3
                    || cell.harvester
                    || cell.reaper
                    || cell.drifting
                    || cell.rare;

                let archetype_visual = if archetype_render_ready {
                    dominant_archetype.and_then(archetype_visual)
                } else {
                    None
                };

                let mut glyph = if let Some((glyph, _)) = archetype_visual {
                    glyph
                } else if cell.reaper {
                    match phase % 4 {
                        0 => 'Ω',
                        1 => 'ϟ',
                        2 => '◉',
                        _ => '○',
                    }
                } else if cell.harvester {
                    match phase % 4 {
                        0 => '♻',
                        1 => '◌',
                        2 => '○',
                        _ => '∙',
                    }
                } else if cell.drifting {
                    match phase % 4 {
                        0 => '◆',
                        1 => '◇',
                        2 => '✧',
                        _ => '◌',
                    }
                } else if cell.rare {
                    match phase % 4 {
                        0 => '✦',
                        1 => '✧',
                        2 => '◇',
                        _ => '○',
                    }
                } else if cell.clustered > 0 && avg_mass > 3.8 {
                    match phase % 5 {
                        0 => '◉',
                        1 => '◎',
                        2 => '◍',
                        3 => '○',
                        _ => '◌',
                    }
                } else if cell.clustered > 0 && cell.count >= 5 {
                    match phase % 5 {
                        0 => '◍',
                        1 => '◎',
                        2 => '○',
                        3 => '◌',
                        _ => 'o',
                    }
                } else if avg_mass > 2.5 {
                    match phase % 4 {
                        0 => '◉',
                        1 => '◎',
                        2 => '○',
                        _ => 'o',
                    }
                } else if cell.count >= 3 {
                    match phase % 4 {
                        0 => '●',
                        1 => '○',
                        2 => 'o',
                        _ => '∙',
                    }
                } else {
                    match phase % 4 {
                        0 => '•',
                        1 => '∙',
                        2 => '·',
                        _ => 'o',
                    }
                };

                if let Some(morphology_glyph) = morphology_flash {
                    glyph = morphology_glyph;
                }

                let mut style = if let Some((_, color)) = archetype_visual {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else if cell.reaper {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if cell.harvester {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if cell.drifting {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(tribe.color())
                        .add_modifier(Modifier::BOLD)
                };

                if cell.rare && !cell.reaper && !cell.drifting {
                    style = style.fg(Color::White).add_modifier(Modifier::BOLD);
                } else if avg_health < 24.0 && !cell.reaper && !cell.drifting {
                    style = style.fg(Color::DarkGray);
                }

                spans.push(Span::styled(glyph.to_string(), style));
            }
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ORGANISM FIELD "),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn camera_bounds(app: &App) -> (f32, f32, f32, f32) {
    let zoom = app.camera_zoom.clamp(1.0, 6.0);
    let half = 1.2 / zoom;

    let max_center = (1.2 - half).max(0.0);
    let cx = app.camera_x.clamp(-max_center, max_center);
    let cy = app.camera_y.clamp(-max_center, max_center);

    (cx - half, cx + half, cy - half, cy + half)
}

fn camera_screen_to_world(
    app: &App,
    sx: usize,
    sy: usize,
    width: usize,
    height: usize,
) -> Option<(f32, f32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let (min_x, max_x, min_y, max_y) = camera_bounds(app);

    let fx = if width <= 1 {
        0.5
    } else {
        sx as f32 / width.saturating_sub(1).max(1) as f32
    };

    let fy = if height <= 1 {
        0.5
    } else {
        sy as f32 / height.saturating_sub(1).max(1) as f32
    };

    Some((min_x + (max_x - min_x) * fx, min_y + (max_y - min_y) * fy))
}

fn camera_world_to_screen(
    app: &App,
    wx: f32,
    wy: f32,
    width: usize,
    height: usize,
) -> Option<(usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }

    let (min_x, max_x, min_y, max_y) = camera_bounds(app);

    if wx < min_x || wx > max_x || wy < min_y || wy > max_y {
        return None;
    }

    let nx = ((wx - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
    let ny = ((wy - min_y) / (max_y - min_y)).clamp(0.0, 1.0);

    let sx = (nx * width.saturating_sub(1).max(1) as f32).round() as usize;
    let sy = (ny * height.saturating_sub(1).max(1) as f32).round() as usize;

    Some((
        sx.min(width.saturating_sub(1)),
        sy.min(height.saturating_sub(1)),
    ))
}

fn camera_sample_substrate(
    app: &App,
    sx: usize,
    sy: usize,
    width: usize,
    height: usize,
) -> CellKind {
    let Some((wx, wy)) = camera_screen_to_world(app, sx, sy, width, height) else {
        return CellKind::Empty;
    };

    app.substrate.influence_at(wx, wy)
}

fn camera_sample_signal(
    app: &App,
    sx: usize,
    sy: usize,
    width: usize,
    height: usize,
) -> crate::automata::Signal {
    let Some((wx, wy)) = camera_screen_to_world(app, sx, sy, width, height) else {
        return crate::automata::Signal::default();
    };

    app.substrate.signal_at(wx, wy)
}

fn draw_axiom_lattice(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    let stats = app.axiom_lattice.stats();

    if stats.alive == 0 || stats.generation == 0 {
        return;
    }

    for y in 0..height {
        for x in 0..width {
            let screen_cell = &mut cells[y][x];

            if screen_cell.count > 0
                || screen_cell.trail
                || screen_cell.membrane
                || screen_cell.zone.is_some()
                || screen_cell.substrate.is_some()
                || screen_cell.signal.is_some()
            {
                continue;
            }

            if !app
                .axiom_lattice
                .sample_screen(x, y, width, height)
                .is_alive()
            {
                continue;
            }

            if !should_render_axiom_cell(stats.generation, x, y, stats.state) {
                continue;
            }

            screen_cell.axiom = Some(axiom_visual(stats.generation, x, y, stats.state));
        }
    }
}

fn should_render_axiom_cell(generation: u64, x: usize, y: usize, state: AxiomPatternState) -> bool {
    let spacing = match state {
        AxiomPatternState::Oscillating => 31,
        AxiomPatternState::Translating => 37,
        AxiomPatternState::Expanding => 43,
        _ => return false,
    };

    let stable_epoch = generation / 18 + axiom_state_offset(state);

    match state {
        AxiomPatternState::Expanding => {
            visual_hash(stable_epoch, x, y) % spacing == 0
                && visual_hash(stable_epoch + 17, x / 2, y / 2) % 11 == 0
        }
        _ => visual_hash(stable_epoch, x, y) % spacing == 0,
    }
}

fn axiom_visual(generation: u64, x: usize, y: usize, state: AxiomPatternState) -> (char, Color) {
    let slow_phase = visual_hash(generation / 8 + axiom_state_offset(state), x, y) % 8;

    match state {
        AxiomPatternState::Dormant => (' ', Color::DarkGray),
        AxiomPatternState::Static => (' ', Color::DarkGray),
        AxiomPatternState::Oscillating => {
            if slow_phase < 4 {
                ('◦', Color::Cyan)
            } else {
                ('∘', Color::Blue)
            }
        }
        AxiomPatternState::Translating => match slow_phase {
            0 | 1 | 2 => ('⊚', Color::Magenta),
            3 | 4 | 5 => ('◌', Color::Cyan),
            _ => ('°', Color::Blue),
        },
        AxiomPatternState::Expanding => {
            if slow_phase < 3 {
                ('✦', Color::White)
            } else {
                ('°', Color::Cyan)
            }
        }
        AxiomPatternState::Collapsing => (' ', Color::DarkGray),
        AxiomPatternState::Chaotic => {
            if slow_phase == 0 {
                ('░', Color::DarkGray)
            } else {
                ('·', Color::DarkGray)
            }
        }
    }
}

fn axiom_state_offset(state: AxiomPatternState) -> u64 {
    match state {
        AxiomPatternState::Dormant => 1,
        AxiomPatternState::Static => 2,
        AxiomPatternState::Oscillating => 3,
        AxiomPatternState::Translating => 4,
        AxiomPatternState::Expanding => 5,
        AxiomPatternState::Collapsing => 6,
        AxiomPatternState::Chaotic => 7,
    }
}

#[derive(Clone, Copy)]
struct VisualMood {
    maturity: f32,
    mutation: f32,
    corridor: f32,
    throttle: f32,
    recovery: f32,
    crowding: f32,
    refill: f32,
}

impl VisualMood {
    fn from_app(app: &App) -> Self {
        Self {
            maturity: (app.age as f32 / 4_800.0).clamp(0.0, 1.0),
            mutation: app.memory.mutation_pressure(),
            corridor: app.memory.corridor_pressure(),
            throttle: app.memory.substrate_throttle_pressure(),
            recovery: app.memory.substrate_recovery_bias(),
            crowding: app.memory.density_crowding_pressure as f32 / 1_000.0,
            refill: app.memory.density_refill_pressure as f32 / 1_000.0,
        }
    }

    fn quieting(self) -> f32 {
        (self.maturity * 0.22 + self.throttle * 0.26 + self.crowding * 0.18).clamp(0.0, 0.52)
    }

    fn volatility(self) -> f32 {
        (self.mutation * 0.48 + self.refill * 0.20 + self.recovery * 0.16).clamp(0.0, 0.72)
    }

    fn corridor_bias(self) -> f32 {
        self.corridor.clamp(0.0, 1.0)
    }
}

fn draw_pattern_field(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    let (field_width, field_height) = app.pattern_field.dimensions();
    if field_width == 0 || field_height == 0 {
        return;
    }

    let mood = VisualMood::from_app(app);
    let field_cells = app.pattern_field.cells();

    for y in 0..height {
        for x in 0..width {
            let screen_cell = &mut cells[y][x];

            if screen_cell.count > 0
                || screen_cell.trail
                || screen_cell.membrane
                || screen_cell.zone.is_some()
                || screen_cell.substrate.is_some()
                || screen_cell.signal.is_some()
            {
                continue;
            }

            let field_x = (x * field_width / width).min(field_width.saturating_sub(1));
            let field_y = (y * field_height / height).min(field_height.saturating_sub(1));
            let field_idx = field_y * field_width + field_x;

            let Some(field_cell) = field_cells.get(field_idx).copied() else {
                continue;
            };

            if !field_cell.is_active() {
                continue;
            }

            if !should_render_field_haze(
                app.age,
                x,
                y,
                field_cell.kind,
                field_cell.danger,
                field_cell.intensity,
                mood,
            ) {
                continue;
            }

            let Some((glyph, color)) = field_haze_visual(
                field_cell.kind,
                field_cell.danger,
                field_cell.intensity,
                mood,
            ) else {
                continue;
            };

            screen_cell.signal = Some((glyph, color));
        }
    }
}

fn should_render_field_haze(
    age: u64,
    x: usize,
    y: usize,
    kind: PatternKind,
    danger: f32,
    intensity: f32,
    mood: VisualMood,
) -> bool {
    let directional = matches!(
        kind,
        PatternKind::Chain | PatternKind::Glider | PatternKind::Swarmfront
    );

    let extreme_danger = danger > 0.86;
    let extreme_front = directional && intensity > 0.92 && mood.corridor_bias() > 0.50;
    let extreme_mutation = mood.mutation > 0.66 && intensity > 0.88;

    if !(extreme_danger || extreme_front || extreme_mutation) {
        return false;
    }

    let spacing: usize = if extreme_danger {
        13
    } else if extreme_front {
        17
    } else {
        23
    };

    let quiet_extra = (mood.quieting() * 22.0).round() as usize;
    let spacing = spacing.saturating_add(quiet_extra).max(9);

    let kind_offset = match kind {
        PatternKind::Halo => 1,
        PatternKind::Nest => 2,
        PatternKind::Swarmfront => 3,
        PatternKind::Glider => 4,
        PatternKind::Lattice => 5,
        PatternKind::Bloom => 6,
        PatternKind::Chain => 7,
        PatternKind::Oscillator => 8,
        PatternKind::StillLife => 9,
        PatternKind::Dormant => 10,
    };

    visual_hash(age / 12 + kind_offset, x, y) % spacing == 0
}

fn field_haze_visual(
    kind: PatternKind,
    danger: f32,
    intensity: f32,
    mood: VisualMood,
) -> Option<(char, Color)> {
    if danger > 0.86 {
        return Some((
            '×',
            if mood.mutation > 0.50 {
                Color::Magenta
            } else {
                Color::Red
            },
        ));
    }

    if mood.corridor > 0.50
        && intensity > 0.92
        && matches!(
            kind,
            PatternKind::Chain | PatternKind::Glider | PatternKind::Swarmfront
        )
    {
        return Some((',', Color::DarkGray));
    }

    if mood.mutation > 0.66 && intensity > 0.88 {
        return Some(('.', Color::Magenta));
    }

    None
}

fn draw_substrate(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    let mood = VisualMood::from_app(app);

    for y in 0..height {
        for x in 0..width {
            let kind = camera_sample_substrate(app, x, y, width, height);

            if kind == CellKind::Empty {
                continue;
            }

            let Some((glyph, color)) = substrate_visual(app, kind, x, y, width, height, mood)
            else {
                continue;
            };

            cells[y][x].substrate = Some((glyph, color));
        }
    }
}

fn substrate_visual(
    app: &App,
    kind: CellKind,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    mood: VisualMood,
) -> Option<(char, Color)> {
    let quiet = mood.quieting();
    let volatile = mood.volatility();

    match kind {
        CellKind::Empty => None,
        CellKind::Life => {
            let spacing = (22.0 + quiet * 18.0 - volatile * 8.0).round().max(12.0) as u64;
            let shimmer = visual_hash(app.age / 3, x, y) as u64 % spacing;

            if shimmer == 0 {
                None
            } else if shimmer == 1 && volatile > 0.36 {
                None
            } else {
                None
            }
        }
        CellKind::Nutrient => {
            let spacing = (8.0 + quiet * 8.0 - mood.recovery * 3.0).round().max(5.0) as u64;
            let shimmer = visual_hash(app.age / 2, x, y) as u64 % spacing;

            if shimmer == 0 {
                Some(('+', Color::Green))
            } else if shimmer <= 1 && mood.recovery > 0.30 {
                Some(('.', Color::Green))
            } else {
                None
            }
        }
        CellKind::Dead => {
            let spacing = (7.0 + quiet * 8.0 - mood.mutation * 3.0).round().max(4.0) as u64;

            if visual_hash(app.age / 4, x, y) as u64 % spacing == 0 {
                Some((
                    '×',
                    if mood.mutation > 0.46 {
                        Color::Magenta
                    } else {
                        Color::DarkGray
                    },
                ))
            } else {
                None
            }
        }
        CellKind::Mutagen => {
            let glyph = if visual_hash(app.age, x, y) % 2 == 0 {
                '*'
            } else {
                '✶'
            };
            Some((glyph, Color::Magenta))
        }
        CellKind::Nest => Some(('◎', Color::Cyan)),
        CellKind::Spore => {
            let spacing = (18.0 + quiet * 10.0 - mood.recovery * 5.0)
                .round()
                .max(10.0) as u64;
            let shimmer = visual_hash(app.age / 2, x, y) as u64 % spacing;

            if shimmer == 0 {
                None
            } else {
                None
            }
        }
        CellKind::Root => Some((
            root_screen_glyph(app, x, y, width, height),
            root_color(y, height),
        )),
    }
}

fn root_color(y: usize, height: usize) -> Color {
    if height == 0 {
        return Color::Blue;
    }

    let ratio = y as f32 / height as f32;

    if ratio > 0.78 {
        Color::Blue
    } else if ratio > 0.50 {
        Color::Cyan
    } else {
        Color::LightBlue
    }
}

fn root_screen_visual(app: &App, x: usize, y: usize, width: usize, height: usize) -> (char, Color) {
    let up = y > 0 && camera_sample_substrate(app, x, y - 1, width, height) == CellKind::Root;
    let down =
        y + 1 < height && camera_sample_substrate(app, x, y + 1, width, height) == CellKind::Root;
    let left = x > 0 && camera_sample_substrate(app, x - 1, y, width, height) == CellKind::Root;
    let right =
        x + 1 < width && camera_sample_substrate(app, x + 1, y, width, height) == CellKind::Root;

    let stage = tree::tree_stage_for_height(y, height);
    let visuals = TreeVisualPolicy::default();
    let height_ratio = y as f32 / height.max(1) as f32;
    let phase =
        ((app.age / visuals.wiggle_rate.max(1)) as usize + visual_hash(app.age / 3, x, y)) % 6;

    let color = match stage {
        TreeStage::Root => Color::Blue,
        TreeStage::Trunk => Color::Yellow,
        TreeStage::Branch => Color::LightYellow,
        TreeStage::Canopy => Color::Green,
    };

    let glyph = match (up, down, left, right) {
        (true, true, true, true) => {
            if phase % 2 == 0 {
                '┼'
            } else {
                '╋'
            }
        }
        (true, true, true, false) => {
            if phase % 3 == 0 {
                '┤'
            } else {
                '┫'
            }
        }
        (true, true, false, true) => {
            if phase % 3 == 0 {
                '├'
            } else {
                '┣'
            }
        }
        (true, false, true, true) => {
            if phase % 2 == 0 {
                '┴'
            } else {
                '┻'
            }
        }
        (false, true, true, true) => {
            if phase % 2 == 0 {
                '┬'
            } else {
                '┳'
            }
        }
        (true, true, false, false) => match phase {
            0 | 3 => '│',
            1 | 4 => '╽',
            _ => '╿',
        },
        (false, false, true, true) => match phase {
            0 | 3 => '─',
            1 | 4 => '╼',
            _ => '╾',
        },
        (false, true, false, true) => {
            if phase % 2 == 0 {
                '┌'
            } else {
                '╭'
            }
        }
        (false, true, true, false) => {
            if phase % 2 == 0 {
                '┐'
            } else {
                '╮'
            }
        }
        (true, false, false, true) => {
            if phase % 2 == 0 {
                '└'
            } else {
                '╰'
            }
        }
        (true, false, true, false) => {
            if phase % 2 == 0 {
                '┘'
            } else {
                '╯'
            }
        }
        (true, false, false, false) => {
            if height_ratio < 0.38 {
                '╵'
            } else {
                '╵'
            }
        }
        (false, true, false, false) => '╷',
        (false, false, true, false) => {
            if phase % 2 == 0 {
                '╴'
            } else {
                '╌'
            }
        }
        (false, false, false, true) => {
            if phase % 2 == 0 {
                '╶'
            } else {
                '╍'
            }
        }
        _ => {
            if height_ratio < 0.38 {
                if phase % 2 == 0 {
                    '╵'
                } else {
                    '╵'
                }
            } else if phase % 2 == 0 {
                '│'
            } else {
                '╎'
            }
        }
    };

    (glyph, color)
}

fn draw_signal_trails(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    let mood = VisualMood::from_app(app);

    for y in 0..height {
        for x in 0..width {
            if cells[y][x].substrate.is_some() {
                continue;
            }

            let signal = camera_sample_signal(app, x, y, width, height);

            if let Some((kind, value)) = signal.strongest() {
                let threshold = match kind {
                    SignalKind::Danger | SignalKind::Fear => 0.42 - mood.mutation * 0.08,
                    SignalKind::Hunger => 0.48 - mood.recovery * 0.06,
                    SignalKind::Growth => 0.56 - mood.recovery * 0.08,
                }
                .clamp(0.34, 0.62);

                if value < threshold {
                    continue;
                }

                let urgent = value > 0.72
                    || matches!(kind, SignalKind::Danger | SignalKind::Fear)
                        && value > 0.54
                        && mood.mutation > 0.38;

                if !urgent && visual_hash(app.age / 7, x, y) % 9 != 0 {
                    continue;
                }

                let color = signal_color(kind, value);
                cells[y][x].signal = Some((kind.glyph(), color));
            }
        }
    }
}

fn draw_ecology_zones(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    for zone in &app.ecology.zones {
        if zone.strength < 0.82 {
            continue;
        }

        let Some((zone_x, zone_y)) = camera_world_to_screen(app, zone.x, zone.y, width, height)
        else {
            continue;
        };

        let x = zone_x as i32;
        let y = zone_y as i32;

        let rare_flash = visual_hash(app.age / 10, x.max(0) as usize, y.max(0) as usize) % 19 == 0;
        if !rare_flash {
            continue;
        }

        let color = match zone.kind.name() {
            "nutrient" => Color::Green,
            "dead" => Color::Red,
            "turbulent" => Color::Yellow,
            "mutagen" => Color::Magenta,
            "nest" => Color::Cyan,
            _ => Color::DarkGray,
        };

        if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
            cells[y as usize][x as usize].zone = Some((zone.kind.glyph(), color));
        }
    }
}

fn draw_cluster_membranes(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    for cluster in &app.clusters.clusters {
        if cluster.size < 7 || cluster.membrane < 12.0 || cluster.age < 24 {
            // CLUSTER_VISUALS_REVEALED {
            continue;
        }

        let Some((cluster_x, cluster_y)) =
            camera_world_to_screen(app, cluster.x, cluster.y, width, height)
        else {
            continue;
        };

        let cx = cluster_x as i32;
        let cy = cluster_y as i32;
        let pulse = ((app.age as f32 / 18.0 + cluster.id as f32).sin() * 0.9) as i32;
        let radius = ((cluster.radius * width as f32 * 1.15).max(2.0)).min(12.0) as i32 + pulse;

        for deg in (0..360).step_by(24) {
            let rad = deg as f32 * std::f32::consts::PI / 180.0;
            let wobble = ((app.age as f32 * 0.015 + deg as f32 + cluster.id as f32).sin() * 0.6)
                .round() as i32;
            let x = cx + (rad.cos() * (radius + wobble) as f32) as i32;
            let y = cy + (rad.sin() * ((radius + wobble) as f32 * 0.62)) as i32;

            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                cells[y as usize][x as usize].membrane = true;
            }
        }
    }
}

fn draw_cluster_motion_trails(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    for cluster in &app.clusters.clusters {
        if cluster.speed() < 0.00014 || cluster.age < 18 {
            continue;
        }

        let Some((cluster_x, cluster_y)) =
            camera_world_to_screen(app, cluster.x, cluster.y, width, height)
        else {
            continue;
        };

        let cx = cluster_x as i32;
        let cy = cluster_y as i32;
        let tx = cx - (cluster.vx * 1180.0) as i32;
        let ty = cy - (cluster.vy * 1180.0) as i32;

        for i in 0..3 {
            let x = cx + ((tx - cx) * i) / 3;
            let y = cy + ((ty - cy) * i) / 3;

            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                cells[y as usize][x as usize].trail = true;
            }
        }
    }
}

fn archetype_short_from_index(index: usize) -> &'static str {
    match index {
        0 => "SWR",
        1 => "HNT",
        2 => "GRZ",
        3 => "ORB",
        4 => "PAR",
        5 => "ARC",
        6 => "LEV",
        7 => "MYC",
        8 => "PHM",
        9 => "HRV",
        10 => "RPR",
        _ => "UNK",
    }
}

fn archetype_glyph_from_index(index: usize) -> &'static str {
    match index {
        0 => "›",
        1 => "▲",
        2 => "+",
        3 => "◌",
        4 => "×",
        5 => "▣",
        6 => "◉",
        7 => "§",
        8 => "◇",
        9 => "♻",
        10 => "Ω",
        _ => "?",
    }
}

fn archetype_color_from_index(index: usize) -> Color {
    match index {
        0 => Color::Cyan,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Blue,
        4 => Color::Magenta,
        5 => Color::Yellow,
        6 => Color::LightYellow,
        7 => Color::LightMagenta,
        8 => Color::DarkGray,
        9 => Color::Green,
        10 => Color::Red,
        _ => Color::DarkGray,
    }
}

fn archetype_count_line(
    label: &'static str,
    counts: &[usize; 11],
    indexes: &[usize],
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        label,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )];

    for index in indexes {
        spans.push(Span::styled(
            format!(
                "{}{}:{} ",
                archetype_glyph_from_index(*index),
                archetype_short_from_index(*index),
                counts[*index]
            ),
            Style::default()
                .fg(archetype_color_from_index(*index))
                .add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

fn trophic_count_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "Trophic ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.memory.trophic_status_line(),
            Style::default().fg(match app.memory.trophic_balance_label.as_str() {
                "cycling" => Color::Magenta,
                "grazing" => Color::Green,
                "predatory" => Color::Red,
                "prey bloom" => Color::Cyan,
                "basal" => Color::DarkGray,
                _ => Color::Yellow,
            }),
        ),
    ])
}

fn ecosystem_phase_label(
    app: &App,
    active_species: usize,
    harvesters: usize,
    reapers: usize,
    substrate_ratio: f32,
    corridor_pressure: f32,
) -> &'static str {
    if reapers > 0 && harvesters > 0 {
        "predator balance"
    } else if harvesters > 0 {
        "harvester bloom"
    } else if substrate_ratio > 0.34 {
        "substrate bloom"
    } else if corridor_pressure > 0.62 && app.clusters.clusters.len() >= 16 {
        "migration lattice"
    } else if app.clusters.clusters.len() >= 18 {
        "territorial"
    } else if active_species < 12 {
        "lineage bottleneck"
    } else {
        "adaptive drift"
    }
}

fn render_rules(f: &mut Frame<'_>, area: Rect, app: &App) {
    let tribes = [
        Tribe::Blood,
        Tribe::Moss,
        Tribe::Deep,
        Tribe::Solar,
        Tribe::Dream,
        Tribe::Static,
    ];

    let active_species = app.species_bank.active_count();
    let mut archetype_counts = [0usize; 11];

    for species in app
        .species_bank
        .species
        .iter()
        .filter(|species| !species.extinct)
    {
        archetype_counts[species.archetype.index()] += 1;
    }

    let mut dominant_archetype = 0usize;

    for idx in 1..11 {
        if archetype_counts[idx] > archetype_counts[dominant_archetype] {
            dominant_archetype = idx;
        }
    }

    let harvesters = archetype_counts[Archetype::Harvester.index()];
    let reapers = archetype_counts[Archetype::Reaper.index()];
    let substrate_ratio =
        app.substrate.living_cells() as f32 / app.substrate.total_cells().max(1) as f32;
    let corridor_pressure = app.memory.corridor_pressure();

    let phase = ecosystem_phase_label(
        app,
        active_species,
        harvesters,
        reapers,
        substrate_ratio,
        corridor_pressure,
    );

    let mut lines = vec![
        Line::from(Span::styled(
            "Adaptive Attraction Matrix",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("PRS ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                pressure_bar(app.matrix_pressure),
                Style::default().fg(matrix_color(app.matrix_pressure)),
            ),
            Span::styled(
                format!(" {:.0}", app.matrix_pressure),
                Style::default().fg(matrix_color(app.matrix_pressure)),
            ),
            Span::styled(" | ATTR ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}", app.matrix_attraction),
                Style::default().fg(Color::Green),
            ),
            Span::styled(" | REP ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}", app.matrix_repulsion),
                Style::default().fg(Color::Red),
            ),
        ]),
    ];

    for a in 0..6 {
        let mut spans = vec![Span::styled(
            format!("{} ", tribes[a].name()),
            Style::default()
                .fg(tribes[a].color())
                .add_modifier(Modifier::BOLD),
        )];

        for b in 0..6 {
            let value = app.rules[a][b];

            let symbol = if value > 0.72 {
                "▓▓"
            } else if value > 0.42 {
                "++"
            } else if value > 0.16 {
                "+ "
            } else if value < -0.72 {
                "██"
            } else if value < -0.42 {
                "--"
            } else if value < -0.16 {
                "- "
            } else {
                "· "
            };

            let color = if value > 0.42 {
                Color::Green
            } else if value > 0.16 {
                Color::Cyan
            } else if value < -0.42 {
                Color::Red
            } else if value < -0.16 {
                Color::Magenta
            } else {
                Color::DarkGray
            };

            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(vec![
        Span::styled("Pop: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}", app.particles.len()),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" Cells: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}", app.substrate.living_cells()),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" Roots: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}", app.substrate.protected_cells()),
            Style::default().fg(Color::Blue),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Eco: ", Style::default().fg(Color::Yellow)),
        Span::styled(phase, Style::default().fg(env_color(app.environment))),
        Span::styled(" | Dom ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            archetype_short_from_index(dominant_archetype),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", archetype_counts[dominant_archetype]),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" | HRV ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", harvesters), Style::default().fg(Color::Green)),
        Span::styled(" RPR ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", reapers), Style::default().fg(Color::Red)),
    ]));

    lines.push(archetype_count_line(
        "Roles A ",
        &app.memory.archetype_live_counts,
        &[0, 1, 2, 3],
    ));

    lines.push(archetype_count_line(
        "Roles B ",
        &app.memory.archetype_live_counts,
        &[4, 5, 6, 7],
    ));

    lines.push(archetype_count_line(
        "Roles C ",
        &app.memory.archetype_live_counts,
        &[8, 9, 10],
    ));

    lines.push(trophic_count_line(app));
    lines.push(Line::from(vec![
        Span::styled(
            "Evolution ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.memory.evolution_status_line(),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Field: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{}", app.pattern_field.active_cells()),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" active avg ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.2}", app.pattern_field.average_intensity()),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            app.pattern_field.strongest_kind().short(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" | corridor ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:.0}%", corridor_pressure * 100.0),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Trunks ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("╋ base ", Style::default().fg(Color::Blue)),
        Span::styled("┃ vertical ", Style::default().fg(Color::Cyan)),
        Span::styled("┏┓ bends ", Style::default().fg(Color::LightBlue)),
        Span::styled("╹ tips", Style::default().fg(Color::LightBlue)),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Cells   ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("∙ life ", Style::default().fg(Color::DarkGray)),
        Span::styled("+ food/nutrient ", Style::default().fg(Color::Green)),
        Span::styled("× dead ", Style::default().fg(Color::DarkGray)),
        Span::styled("░ spore ", Style::default().fg(Color::DarkGray)),
        Span::styled("*✶ mutagen ", Style::default().fg(Color::Magenta)),
        Span::styled("◎ nest", Style::default().fg(Color::Cyan)),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Signals ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("∿ hunger ", Style::default().fg(Color::Yellow)),
        Span::styled("! fear ", Style::default().fg(Color::Red)),
        Span::styled("∙ growth ", Style::default().fg(Color::Green)),
        Span::styled("× danger", Style::default().fg(Color::Magenta)),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Bodies  ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("•∙·o solo ", Style::default().fg(Color::Cyan)),
        Span::styled("●○ group ", Style::default().fg(Color::Cyan)),
        Span::styled("◍◎◉ dense ", Style::default().fg(Color::Cyan)),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Special ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("✦✧ rare ", Style::default().fg(Color::White)),
        Span::styled("♻ harvester ", Style::default().fg(Color::Green)),
        Span::styled("Ωϟ reaper ", Style::default().fg(Color::Red)),
        Span::styled("◆ drift ", Style::default().fg(Color::Magenta)),
        Span::styled("○ membrane", Style::default().fg(Color::Gray)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SYMBIOSIS RULES + FIELD GLOSSARY "),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_clusters(f: &mut Frame<'_>, area: Rect, app: &App) {
    let drifting = app
        .clusters
        .clusters
        .iter()
        .filter(|cluster| cluster.archetype_override.is_some())
        .count();

    let mut lines = vec![Line::from(vec![
        Span::styled("Clusters: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.clusters.clusters.len()),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" Drift: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", drifting),
            Style::default().fg(if drifting > 0 {
                Color::Magenta
            } else {
                Color::DarkGray
            }),
        ),
        Span::styled(" Peak: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.memory.peak_clusters),
            Style::default().fg(Color::Yellow),
        ),
    ])];

    for cluster in app.clusters.clusters.iter().take(4) {
        let base = cluster
            .archetype
            .map(|value| value.short())
            .unwrap_or("UNK");
        let effective = cluster
            .effective_archetype()
            .map(|value| value.short())
            .unwrap_or("UNK");
        let overridden = cluster.archetype_override.is_some();

        let archetype_color = if effective == "RPR" {
            Color::Red
        } else if effective == "HRV" {
            Color::Green
        } else if overridden {
            Color::Magenta
        } else {
            Color::Cyan
        };

        let drift_marker = if overridden {
            if cluster.drift_heat > 80.0 {
                "✦"
            } else if cluster.drift_heat > 60.0 {
                "≋"
            } else {
                "~"
            }
        } else {
            " "
        };

        let pattern_kind = if overridden && cluster.drift_heat > 78.0 {
            PatternKind::Oscillator
        } else if effective == "ORB" {
            PatternKind::Halo
        } else if effective == "SWR" && cluster.size >= 18 {
            PatternKind::Swarmfront
        } else if effective == "PAR" && cluster.size >= 9 {
            PatternKind::Chain
        } else if effective == "HRV" && cluster.size >= 12 {
            PatternKind::Nest
        } else if cluster.membrane > 62.0 && cluster.size >= 18 {
            PatternKind::Lattice
        } else if cluster.age < 160 && cluster.size >= 7 {
            PatternKind::Bloom
        } else if cluster.drift_heat < 28.0 && cluster.size >= 14 {
            PatternKind::StillLife
        } else {
            PatternKind::Glider
        };

        let pattern_motion = match pattern_kind {
            PatternKind::StillLife | PatternKind::Lattice | PatternKind::Nest => {
                PatternMotion::Static
            }
            PatternKind::Oscillator | PatternKind::Halo => PatternMotion::Pulse,
            PatternKind::Bloom => PatternMotion::Expand,
            PatternKind::Swarmfront | PatternKind::Glider => PatternMotion::Translate,
            PatternKind::Chain => PatternMotion::Drift,
            PatternKind::Dormant => PatternMotion::Static,
        };

        let pattern_signature = PatternSignature {
            kind: pattern_kind,
            motion: pattern_motion,
            stability: (cluster.stability / 100.0).clamp(0.0, 1.0),
            pulse: (cluster.drift_heat / 100.0).clamp(0.0, 1.0),
            drift: if overridden {
                0.86
            } else {
                (cluster.speed() * 900.0).clamp(0.0, 1.0)
            },
            cohesion: (cluster.size as f32 / 42.0).clamp(0.0, 1.0),
            fertility: (cluster.membrane / 100.0).clamp(0.0, 1.0),
            danger: if effective == "RPR" { 0.88 } else { 0.0 },
        };

        let pattern_symbol = pattern_glyph(pattern_signature, app.age);
        let pattern_bar = pattern_strength_bar(pattern_signature.intensity(), 5); // RENDER_PATTERN_CLUSTER_READOUT

        lines.push(Line::from(vec![
            Span::styled(
                format!("#{} ", cluster.id),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                cluster.direction_glyph().to_string(),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" "),
            Span::styled(
                effective,
                Style::default()
                    .fg(archetype_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if overridden {
                    format!("({})", base)
                } else {
                    String::new()
                },
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{} ", cluster.size),
                Style::default().fg(cluster.dominant.color()),
            ),
            Span::styled(
                format!("a{} ", cluster.age),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("h{:.0}{}", cluster.drift_heat, drift_marker),
                Style::default().fg(if overridden {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{} {} ", pattern_symbol, pattern_signature.label()),
                Style::default().fg(archetype_color),
            ),
            Span::styled(
                pattern_bar,
                Style::default().fg(if overridden {
                    Color::Magenta
                } else {
                    Color::DarkGray
                }),
            ),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" CLUSTERS "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_species(f: &mut Frame<'_>, area: Rect, app: &App) {
    let extinct = app
        .species_bank
        .species
        .iter()
        .filter(|species| species.extinct)
        .count();

    let harvesters = app
        .species_bank
        .species
        .iter()
        .filter(|species| !species.extinct && species.archetype == Archetype::Harvester)
        .count();

    let reapers = app
        .species_bank
        .species
        .iter()
        .filter(|species| !species.extinct && species.archetype == Archetype::Reaper)
        .count();

    let mut lines = vec![Line::from(vec![
        Span::styled("Active: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.species_bank.active_count()),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" HRV: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", harvesters), Style::default().fg(Color::Green)),
        Span::styled(" RPR: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", reapers), Style::default().fg(Color::Red)),
    ])];

    lines.push(Line::from(vec![
        Span::styled("Extinct: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", extinct), Style::default().fg(Color::Red)),
        Span::styled(" Reaped: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}", app.memory.total_harvesters_consumed),
            Style::default().fg(Color::Red),
        ),
    ]));

    for species in app
        .species_bank
        .species
        .iter()
        .rev()
        .filter(|species| !species.extinct)
        .take(3)
    {
        let rare = species.rare_trait.short();

        let archetype_color = if species.archetype == Archetype::Reaper {
            Color::Red
        } else if species.archetype == Archetype::Harvester {
            Color::Green
        } else {
            Color::Magenta
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", species.name),
                Style::default().fg(species.dominant_tribe.color()),
            ),
            Span::styled(
                species.archetype.short(),
                Style::default().fg(archetype_color),
            ),
            Span::styled(
                format!(" p{}", species.peak_size),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(format!(" {}", rare), Style::default().fg(Color::White)),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" SPECIES "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_events(f: &mut Frame<'_>, area: Rect, app: &App) {
    let density_status = app.memory.density_status_line();
    let mood = VisualMood::from_app(app);
    let phase = memory_phase_label(mood);

    let mut items = vec![
        ListItem::new(Line::from(vec![
            Span::styled(
                "density ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                density_status,
                Style::default().fg(density_color(&app.memory.density_band)),
            ),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(
                "memory ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{} root:{:.2} cor:{:.2} sub:{:.2} mut:{:.2}",
                    phase,
                    app.memory.root_avoidance_pressure(),
                    app.memory.corridor_pressure(),
                    app.memory.substrate_throttle_pressure(),
                    app.memory.mutation_pressure()
                ),
                Style::default().fg(memory_phase_color(mood)),
            ),
        ])),
    ];

    items.extend(
        app.events
            .iter()
            .rev()
            .map(|event| {
                ListItem::new(Line::from(Span::styled(
                    event.clone(),
                    Style::default().fg(Color::Cyan),
                )))
            })
            .collect::<Vec<_>>(),
    );

    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" EVOLUTION FEED "),
        ),
        area,
    );
}

fn memory_phase_label(mood: VisualMood) -> &'static str {
    if mood.mutation > 0.58 {
        "unstable"
    } else if mood.corridor > 0.54 {
        "channeling"
    } else if mood.throttle > 0.58 {
        "cooling"
    } else if mood.recovery > 0.48 {
        "recovering"
    } else if mood.maturity > 0.62 {
        "settled"
    } else {
        "adapting"
    }
}

fn memory_phase_color(mood: VisualMood) -> Color {
    if mood.mutation > 0.58 {
        Color::Magenta
    } else if mood.corridor > 0.54 {
        Color::Blue
    } else if mood.throttle > 0.58 {
        Color::DarkGray
    } else if mood.recovery > 0.48 {
        Color::Green
    } else {
        Color::Cyan
    }
}

fn archetype_visual(index: usize) -> Option<(char, Color)> {
    match index {
        0 => Some(('≋', Color::Cyan)),         // Swarmer
        1 => Some(('⌁', Color::Red)),          // Hunter
        2 => Some(('˖', Color::Green)),        // Grazer
        3 => Some(('⊙', Color::Blue)),         // Orbiter
        4 => Some(('⁘', Color::Magenta)),      // Parasite
        5 => Some(('▣', Color::Yellow)),       // Architect
        6 => Some(('◈', Color::White)),        // Leviathan
        7 => Some(('§', Color::LightMagenta)), // Mycelial
        8 => Some(('⟡', Color::Gray)),         // Phantom
        9 => Some(('♻', Color::LightGreen)),   // Harvester
        10 => Some(('Ω', Color::Red)),         // Reaper
        _ => None,
    }
}

fn density_color(label: &str) -> Color {
    match label {
        "Starved" => Color::Red,
        "Sparse" => Color::Yellow,
        "Balanced" => Color::Green,
        "Crowded" => Color::Magenta,
        "Saturated" => Color::Red,
        _ => Color::DarkGray,
    }
}

fn render_metrics(f: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(16),
            Constraint::Percentage(16),
        ])
        .split(area);

    f.render_widget(metric("ENERGY", app.energy as u16, Color::Cyan), chunks[0]);
    f.render_widget(
        metric("COHESION", app.cohesion as u16, Color::Green),
        chunks[1],
    );
    f.render_widget(metric("CHAOS", app.chaos as u16, Color::Magenta), chunks[2]);
    f.render_widget(metric("DRIFT", app.drift as u16, Color::Yellow), chunks[3]);
    f.render_widget(metric("POP", app.population as u16, Color::Red), chunks[4]);
    f.render_widget(
        metric(
            "MATRIX",
            app.matrix_pressure as u16,
            matrix_color(app.matrix_pressure),
        ),
        chunks[5],
    );
}

fn render_footer(f: &mut Frame<'_>, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            " CONTROLS ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " space pause | arrows pan | wheel zoom | 0 reset view | r restart | n new | +/- speed | q save+quit ",
            Style::default().fg(Color::Gray),
        ),
    ]);

    f.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn metric(title: &'static str, value: u16, color: Color) -> Gauge<'static> {
    Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .gauge_style(Style::default().fg(color).bg(Color::Black))
        .percent(value.min(100))
}

#[derive(Clone)]
struct Cell {
    count: usize,
    tribe_counts: [usize; 6],
    archetype_counts: [usize; 11],
    health: f32,
    energy: f32,
    mass: f32,
    clustered: usize,
    membrane: bool,
    trail: bool,
    rare: bool,
    harvester: bool,
    reaper: bool,
    drifting: bool,
    zone: Option<(char, Color)>,
    substrate: Option<(char, Color)>,
    signal: Option<(char, Color)>,
    axiom: Option<(char, Color)>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            count: 0,
            tribe_counts: [0; 6],
            archetype_counts: [0; 11],
            health: 0.0,
            energy: 0.0,
            mass: 0.0,
            clustered: 0,
            membrane: false,
            trail: false,
            rare: false,
            harvester: false,
            reaper: false,
            drifting: false,
            zone: None,
            substrate: None,
            signal: None,
            axiom: None,
        }
    }
}

impl Cell {
    fn dominant_tribe(&self) -> Tribe {
        let mut best = 0;

        for i in 1..6 {
            if self.tribe_counts[i] > self.tribe_counts[best] {
                best = i;
            }
        }

        Tribe::from_index(best)
    }

    fn organic_phase(&self, age: u64) -> usize {
        let mut value = age as usize;
        value ^= self.count.wrapping_mul(97);
        value ^= self.clustered.wrapping_mul(131);
        value ^= self.dominant_tribe().index().wrapping_mul(389);
        value = (value ^ (value >> 11)).wrapping_mul(1_103_515_245);
        value ^ (value >> 15)
    }
}

fn cell_morphology_role(cell: &Cell, avg_mass: f32) -> MorphologyRole {
    if cell.reaper {
        MorphologyRole::PredatorFront
    } else if cell.drifting {
        MorphologyRole::Migrator
    } else if cell.harvester {
        MorphologyRole::Seeder
    } else if cell.clustered > 0 && avg_mass > 3.8 {
        MorphologyRole::Membrane
    } else if cell.clustered > 0 && cell.count >= 5 {
        MorphologyRole::Oscillator
    } else if cell.clustered > 0 {
        MorphologyRole::Anchor
    } else if cell.rare {
        MorphologyRole::Migrator
    } else {
        MorphologyRole::Dormant
    }
}

fn morphology_render_glyph(role: MorphologyRole, age: u64, phase: usize) -> Option<char> {
    match role {
        MorphologyRole::Dormant => None,
        MorphologyRole::Anchor => {
            if phase % 5 <= 3 {
                Some('●')
            } else {
                Some('•')
            }
        }
        MorphologyRole::Oscillator => {
            if (age / 4 + phase as u64) % 2 == 0 {
                Some('◐')
            } else {
                Some('◑')
            }
        }
        MorphologyRole::Migrator => match phase % 4 {
            0 => Some('›'),
            1 => Some('»'),
            2 => Some('·'),
            _ => None,
        },
        MorphologyRole::Seeder => match phase % 5 {
            0 => Some('✦'),
            1 => Some('✧'),
            2 => Some('∙'),
            _ => None,
        },
        MorphologyRole::Membrane => match phase % 4 {
            0 => Some('◎'),
            1 => Some('◉'),
            2 => Some('◌'),
            _ => Some('○'),
        },
        MorphologyRole::PredatorFront => match phase % 4 {
            0 => Some('▲'),
            1 => Some('ϟ'),
            2 => Some('Ω'),
            _ => None,
        },
    }
}

fn pressure_bar(value: f32) -> String {
    let filled = ((value.clamp(0.0, 100.0) / 100.0) * 8.0).round() as usize;
    let empty = 8usize.saturating_sub(filled);

    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn matrix_color(value: f32) -> Color {
    if value > 72.0 {
        Color::Red
    } else if value > 48.0 {
        Color::Yellow
    } else if value > 28.0 {
        Color::Cyan
    } else {
        Color::Green
    }
}

#[allow(dead_code)]
fn visual_maturity(age: u64) -> f32 {
    (age as f32 / 4_800.0).clamp(0.0, 1.0)
}

#[allow(dead_code)]
fn density_band_color(band: &str) -> Color {
    match band {
        "Starved" => Color::Red,
        "Sparse" => Color::Yellow,
        "Balanced" => Color::Green,
        "Crowded" => Color::Magenta,
        "Saturated" => Color::Red,
        _ => Color::DarkGray,
    }
}

fn signal_color(kind: SignalKind, value: f32) -> Color {
    match kind {
        SignalKind::Hunger => {
            if value > 0.55 {
                Color::Yellow
            } else {
                Color::DarkGray
            }
        }
        SignalKind::Fear => {
            if value > 0.55 {
                Color::Red
            } else {
                Color::DarkGray
            }
        }
        SignalKind::Growth => {
            if value > 0.55 {
                Color::Green
            } else {
                Color::DarkGray
            }
        }
        SignalKind::Danger => {
            if value > 0.55 {
                Color::Magenta
            } else {
                Color::DarkGray
            }
        }
    }
}

fn env_color(env: Environment) -> Color {
    match env {
        Environment::Calm => Color::Green,
        Environment::Bloom => Color::Magenta,
        Environment::Hunger => Color::Red,
        Environment::Storm => Color::Yellow,
        Environment::Drift => Color::Cyan,
    }
}

fn visual_hash(age: u64, x: usize, y: usize) -> usize {
    let mut value = age as usize;
    value ^= x.wrapping_mul(374_761_393);
    value ^= y.wrapping_mul(668_265_263);
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    value ^ (value >> 16)
}

fn root_screen_glyph(app: &App, x: usize, y: usize, width: usize, height: usize) -> char {
    root_screen_visual(app, x, y, width, height).0
}
