use crate::{
    app::{App, Environment},
    automata::{CellKind, SignalKind},
    particle::Tribe,
    pattern::{pattern_glyph, pattern_strength_bar, PatternKind, PatternMotion, PatternSignature},
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
    // force root layer to overwrite everything (no flicker)
    for y in 0..height {
        for x in 0..width {
            if app.substrate.sample_screen(x, y, width, height) == CellKind::Root {
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
                if app.substrate.sample_screen(px, py, width, height) == CellKind::Root {
                    continue; // do not draw particle over root
                }
            }
        }

        let x = (((particle.x + 1.2) / 2.4) * width as f32) as isize;
        let y = (((particle.y + 1.2) / 2.4) * height as f32) as isize;

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
            } else if cell.count == 0 {
                spans.push(Span::styled(
                    background_glyph(app.environment),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                let tribe = cell.dominant_tribe();
                let avg_health = cell.health / cell.count as f32;
                let avg_mass = cell.mass / cell.count as f32;
                let phase = cell.organic_phase(app.age);

                let glyph = if cell.reaper {
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

                let mut style = if cell.reaper {
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

fn draw_pattern_field(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    let (field_width, field_height) = app.pattern_field.dimensions();

    if field_width == 0 || field_height == 0 {
        return;
    }

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

            screen_cell.signal = Some((
                field_cell.glyph(),
                field_color(field_cell.kind, field_cell.danger, field_cell.intensity),
            ));
        }
    }
}

fn field_color(kind: PatternKind, danger: f32, intensity: f32) -> Color {
    if danger > 0.35 {
        return Color::Red;
    }

    if intensity > 0.82 {
        return match kind {
            PatternKind::Halo => Color::Cyan,
            PatternKind::Nest => Color::Green,
            PatternKind::Swarmfront => Color::Magenta,
            PatternKind::Glider => Color::LightCyan,
            PatternKind::Lattice => Color::Yellow,
            PatternKind::Bloom => Color::LightGreen,
            PatternKind::Chain => Color::LightMagenta,
            PatternKind::Oscillator => Color::LightBlue,
            PatternKind::StillLife => Color::Gray,
            PatternKind::Dormant => Color::DarkGray,
        };
    }

    match kind {
        PatternKind::Halo => Color::Blue,
        PatternKind::Nest => Color::Green,
        PatternKind::Swarmfront => Color::Magenta,
        PatternKind::Glider => Color::Cyan,
        PatternKind::Lattice => Color::Yellow,
        PatternKind::Bloom => Color::Green,
        PatternKind::Chain => Color::Magenta,
        PatternKind::Oscillator => Color::Blue,
        PatternKind::StillLife => Color::Gray,
        PatternKind::Dormant => Color::DarkGray,
    }
}

fn draw_substrate(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }

    for y in 0..height {
        for x in 0..width {
            let kind = app.substrate.sample_screen(x, y, width, height);

            if kind == CellKind::Empty {
                continue;
            }

            let Some((glyph, color)) = substrate_visual(app, kind, x, y, width, height) else {
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
) -> Option<(char, Color)> {
    match kind {
        CellKind::Empty => None,
        CellKind::Life => {
            let shimmer = visual_hash(app.age, x, y) % 9;
            if shimmer <= 1 {
                Some(('∙', Color::DarkGray))
            } else if shimmer == 2 {
                Some(('·', Color::DarkGray))
            } else {
                None
            }
        }
        CellKind::Nutrient => {
            let shimmer = visual_hash(app.age / 2, x, y) % 6;
            if shimmer == 0 {
                Some(('+', Color::Green))
            } else if shimmer <= 2 {
                Some(('.', Color::Green))
            } else {
                None
            }
        }
        CellKind::Dead => {
            if visual_hash(app.age / 3, x, y) % 5 == 0 {
                Some(('×', Color::DarkGray))
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
            let shimmer = visual_hash(app.age, x, y) % 7;
            if shimmer <= 1 {
                Some(('░', Color::DarkGray))
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
    let up = y > 0 && app.substrate.sample_screen(x, y - 1, width, height) == CellKind::Root;
    let down =
        y + 1 < height && app.substrate.sample_screen(x, y + 1, width, height) == CellKind::Root;
    let left = x > 0 && app.substrate.sample_screen(x - 1, y, width, height) == CellKind::Root;
    let right =
        x + 1 < width && app.substrate.sample_screen(x + 1, y, width, height) == CellKind::Root;

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
                '♣'
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
                    '♣'
                } else {
                    '♧'
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
    for y in 0..height {
        for x in 0..width {
            if cells[y][x].substrate.is_some() {
                continue;
            }

            let signal = app.substrate.sample_signal_screen(x, y, width, height);

            if let Some((kind, value)) = signal.strongest() {
                if value < 0.18 {
                    continue;
                }

                let color = signal_color(kind, value);
                cells[y][x].signal = Some((kind.glyph(), color));
            }
        }
    }
}

fn draw_ecology_zones(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    // force root layer to overwrite everything (no flicker)
    for y in 0..height {
        for x in 0..width {
            if app.substrate.sample_screen(x, y, width, height) == CellKind::Root {
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
    for zone in &app.ecology.zones {
        let x = (((zone.x + 1.2) / 2.4) * width as f32) as i32;
        let y = (((zone.y + 1.2) / 2.4) * height as f32) as i32;

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

        let cx = (((cluster.x + 1.2) / 2.4) * width as f32) as i32;
        let cy = (((cluster.y + 1.2) / 2.4) * height as f32) as i32;
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

        let cx = (((cluster.x + 1.2) / 2.4) * width as f32) as i32;
        let cy = (((cluster.y + 1.2) / 2.4) * height as f32) as i32;
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

fn render_rules(f: &mut Frame<'_>, area: Rect, app: &App) {
    let tribes = [
        Tribe::Blood,
        Tribe::Moss,
        Tribe::Deep,
        Tribe::Solar,
        Tribe::Dream,
        Tribe::Static,
    ];

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
        Span::styled(
            "Trunks  ",
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
        Span::styled("+ food ", Style::default().fg(Color::Green)),
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
        Span::styled("◆◇ drift ", Style::default().fg(Color::Magenta)),
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
                "⚡"
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
    let _density_status = app.memory.density_status_line();
    let items = app
        .events
        .iter()
        .rev()
        .map(|event| {
            ListItem::new(Line::from(Span::styled(
                event.clone(),
                Style::default().fg(Color::Cyan),
            )))
        })
        .collect::<Vec<_>>();

    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" EVOLUTION FEED "),
        ),
        area,
    );
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
            " space pause | r restart | n new ecosystem | +/- speed | q save+quit ",
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
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            count: 0,
            tribe_counts: [0; 6],
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

fn background_glyph(env: Environment) -> &'static str {
    match env {
        Environment::Calm => "·",
        Environment::Bloom => ".",
        Environment::Hunger => " ",
        Environment::Storm => "∴",
        Environment::Drift => "˙",
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
