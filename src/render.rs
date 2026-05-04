use crate::{
    app::{App, Environment},
    automata::CellKind,
    particle::Tribe,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
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
            Constraint::Percentage(34),
            Constraint::Percentage(23),
            Constraint::Percentage(19),
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

fn render_header(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let status = if app.paused { "paused" } else { "alive" };
    let pulse = ["░", "▒", "▓", "█", "▓", "▒"][(app.age as usize / 4) % 6];

    let lines = vec![
        Line::from(vec![
            Span::styled(" ◉ SYMBIOTE ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("balanced cellular ecosystem ", Style::default().fg(Color::Magenta)),
            Span::styled(pulse.repeat(12), Style::default().fg(env_color(app.environment))),
        ]),
        Line::from(vec![
            Span::styled(" age: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.age), Style::default().fg(Color::Green)),
            Span::styled(" | gen: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.generation), Style::default().fg(Color::Magenta)),
            Span::styled(" | env: ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.environment.name(), Style::default().fg(env_color(app.environment))),
            Span::styled(" | species: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.species_bank.active_count()), Style::default().fg(Color::Yellow)),
            Span::styled(" | cells: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", app.substrate.living_cells()), Style::default().fg(Color::Green)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(status, Style::default().fg(if app.paused { Color::Yellow } else { Color::Green })),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" LIFE CORE ")),
        area,
    );
}

fn render_world(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let width = area.width.saturating_sub(2) as usize;
    let height = area.height.saturating_sub(2) as usize;

    let mut cells: Vec<Vec<Cell>> = vec![vec![Cell::default(); width]; height];

    draw_substrate(&mut cells, app, width, height);
    draw_ecology_zones(&mut cells, app, width, height);

    for particle in &app.particles {
        let x = (((particle.x + 1.2) / 2.4) * width as f32) as isize;
        let y = (((particle.y + 1.2) / 2.4) * height as f32) as isize;

        if x >= 0 && y >= 0 && x < width as isize && y < height as isize {
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
            } else if cell.count == 0 {
                spans.push(Span::styled(background_glyph(app.environment), Style::default().fg(Color::DarkGray)));
            } else {
                let tribe = cell.dominant_tribe();
                let avg_health = cell.health / cell.count as f32;
                let avg_mass = cell.mass / cell.count as f32;

                let glyph = if cell.rare {
                    '✦'
                } else if cell.clustered > 0 && avg_mass > 3.8 {
                    '█'
                } else if cell.clustered > 0 && cell.count >= 5 {
                    '⬤'
                } else if avg_mass > 2.5 {
                    '◉'
                } else if cell.count >= 3 {
                    '●'
                } else {
                    '•'
                };

                let mut style = Style::default().fg(tribe.color()).add_modifier(Modifier::BOLD);

                if cell.rare {
                    style = style.fg(Color::White).add_modifier(Modifier::BOLD);
                } else if avg_health < 24.0 {
                    style = style.fg(Color::DarkGray);
                }

                spans.push(Span::styled(glyph.to_string(), style));
            }
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" ORGANISM FIELD "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_substrate(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let kind = app.substrate.sample_screen(x, y, width, height);

            if kind == CellKind::Empty {
                continue;
            }

            let color = match kind {
                CellKind::Life => Color::DarkGray,
                CellKind::Nutrient => Color::Green,
                CellKind::Dead => Color::DarkGray,
                CellKind::Mutagen => Color::Magenta,
                CellKind::Nest => Color::Cyan,
                CellKind::Spore => Color::DarkGray,
                CellKind::Empty => Color::DarkGray,
            };

            cells[y][x].substrate = Some((kind.glyph(), color));
        }
    }
}

fn draw_ecology_zones(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
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
        if cluster.size < 14 || cluster.membrane < 25.0 {
            continue;
        }

        let cx = (((cluster.x + 1.2) / 2.4) * width as f32) as i32;
        let cy = (((cluster.y + 1.2) / 2.4) * height as f32) as i32;

        let pulse = ((app.age as f32 / 12.0).sin() * 1.2) as i32;
        let radius = ((cluster.radius * width as f32 * 0.9).max(2.0)).min(9.0) as i32 + pulse;

        for deg in (0..360).step_by(18) {
            let rad = deg as f32 * std::f32::consts::PI / 180.0;
            let x = cx + (rad.cos() * radius as f32) as i32;
            let y = cy + (rad.sin() * (radius as f32 * 0.62)) as i32;

            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                cells[y as usize][x as usize].membrane = true;
            }
        }
    }
}

fn draw_cluster_motion_trails(cells: &mut [Vec<Cell>], app: &App, width: usize, height: usize) {
    for cluster in &app.clusters.clusters {
        if cluster.speed() < 0.00035 {
            continue;
        }

        let cx = (((cluster.x + 1.2) / 2.4) * width as f32) as i32;
        let cy = (((cluster.y + 1.2) / 2.4) * height as f32) as i32;

        let tx = cx - (cluster.vx * 900.0) as i32;
        let ty = cy - (cluster.vy * 900.0) as i32;

        for i in 0..4 {
            let x = cx + ((tx - cx) * i) / 4;
            let y = cy + ((ty - cy) * i) / 4;

            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                cells[y as usize][x as usize].trail = true;
            }
        }
    }
}

fn render_rules(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let tribes = [
        Tribe::Blood,
        Tribe::Moss,
        Tribe::Deep,
        Tribe::Solar,
        Tribe::Dream,
        Tribe::Static,
    ];

    let mut lines = vec![Line::from(Span::styled(
        "Attraction Matrix",
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    ))];

    for a in 0..6 {
        let mut spans = vec![Span::styled(
            format!("{} ", tribes[a].name()),
            Style::default().fg(tribes[a].color()).add_modifier(Modifier::BOLD),
        )];

        for b in 0..6 {
            let value = app.rules[a][b];

            let symbol = if value > 0.62 {
                "++"
            } else if value > 0.18 {
                "+ "
            } else if value < -0.62 {
                "--"
            } else if value < -0.18 {
                "- "
            } else {
                "· "
            };

            let color = if value > 0.18 {
                Color::Green
            } else if value < -0.18 {
                Color::Red
            } else {
                Color::DarkGray
            };

            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(vec![
        Span::styled("Pop: ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("{}", app.particles.len()), Style::default().fg(Color::Green)),
        Span::styled(" Zones: ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("{}", app.ecology.zones.len()), Style::default().fg(Color::Cyan)),
        Span::styled(" Cells: ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("{}", app.substrate.living_cells()), Style::default().fg(Color::Green)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" SYMBIOSIS RULES "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_clusters(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Clusters: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", app.clusters.clusters.len()), Style::default().fg(Color::Green)),
        Span::styled(" Peak: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", app.memory.peak_clusters), Style::default().fg(Color::Yellow)),
    ])];

    for cluster in app.clusters.clusters.iter().take(4) {
        let archetype = cluster.archetype.map(|value| value.short()).unwrap_or("UNK");

        lines.push(Line::from(vec![
            Span::styled(format!("#{} ", cluster.id), Style::default().fg(Color::DarkGray)),
            Span::styled(cluster.direction_glyph().to_string(), Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(archetype, Style::default().fg(Color::Magenta)),
            Span::raw(" "),
            Span::styled(format!("{} ", cluster.size), Style::default().fg(cluster.dominant.color())),
            Span::styled(format!("a{}", cluster.age), Style::default().fg(Color::Cyan)),
        ]));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" CLUSTERS "))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_species(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let extinct = app.species_bank.species.iter().filter(|species| species.extinct).count();

    let mut lines = vec![Line::from(vec![
        Span::styled("Active: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", app.species_bank.active_count()), Style::default().fg(Color::Green)),
        Span::styled(" Extinct: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", extinct), Style::default().fg(Color::Red)),
    ])];

    for species in app.species_bank.species.iter().rev().filter(|species| !species.extinct).take(3) {
        let rare = species.rare_trait.short();

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", species.name), Style::default().fg(species.dominant_tribe.color())),
            Span::styled(species.archetype.short(), Style::default().fg(Color::Magenta)),
            Span::styled(format!(" p{}", species.peak_size), Style::default().fg(Color::Cyan)),
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

fn render_events(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let items = app
        .events
        .iter()
        .rev()
        .map(|event| ListItem::new(Line::from(Span::styled(event.clone(), Style::default().fg(Color::Cyan)))))
        .collect::<Vec<_>>();

    f.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(" EVOLUTION FEED ")),
        area,
    );
}

fn render_metrics(f: &mut Frame<'_>, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    f.render_widget(metric("ENERGY", app.energy as u16, Color::Cyan), chunks[0]);
    f.render_widget(metric("COHESION", app.cohesion as u16, Color::Green), chunks[1]);
    f.render_widget(metric("CHAOS", app.chaos as u16, Color::Magenta), chunks[2]);
    f.render_widget(metric("DRIFT", app.drift as u16, Color::Yellow), chunks[3]);
    f.render_widget(metric("POP", app.population as u16, Color::Red), chunks[4]);
}

fn render_footer(f: &mut Frame<'_>, area: ratatui::layout::Rect) {
    let line = Line::from(vec![
        Span::styled(
            " CONTROLS ",
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
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
    zone: Option<(char, Color)>,
    substrate: Option<(char, Color)>,
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
            zone: None,
            substrate: None,
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
