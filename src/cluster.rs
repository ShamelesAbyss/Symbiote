use crate::particle::{Particle, Tribe};

#[derive(Clone)]
pub struct Cluster {
    pub id: u64,
    pub age: u64,
    pub size: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub radius: f32,
    pub dominant: Tribe,
    pub stability: f32,
    pub membrane: f32,
    pub last_seen: u64,
}

impl Cluster {
    pub fn speed(&self) -> f32 {
        (self.vx * self.vx + self.vy * self.vy).sqrt()
    }

    pub fn direction_glyph(&self) -> char {
        let speed = self.speed();

        if speed < 0.0002 {
            return '•';
        }

        let ax = self.vx.abs();
        let ay = self.vy.abs();

        if ax > ay {
            if self.vx > 0.0 { '→' } else { '←' }
        } else if self.vy > 0.0 {
            '↓'
        } else {
            '↑'
        }
    }
}

pub struct ClusterTracker {
    pub clusters: Vec<Cluster>,
    pub next_id: u64,
}

impl ClusterTracker {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            next_id: 1,
        }
    }

    pub fn update(&mut self, particles: &mut [Particle], age: u64) -> ClusterEvents {
        for p in particles.iter_mut() {
            p.cluster_id = None;
        }

        let groups = detect_groups(particles);
        let mut next_clusters = Vec::new();
        let mut events = ClusterEvents::default();

        for group in groups {
            if group.len() < 5 {
                continue;
            }

            let measured = measure_group(&group, particles);
            let mut best_match = None;
            let mut best_dist = f32::MAX;

            for existing in &self.clusters {
                let dx = existing.x - measured.x;
                let dy = existing.y - measured.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 0.26 && dist < best_dist {
                    best_match = Some(existing.clone());
                    best_dist = dist;
                }
            }

            let mut cluster = if let Some(old) = best_match {
                let mut c = measured;
                c.id = old.id;
                c.age = old.age + 1;
                c.stability = (old.stability * 0.9 + c.stability * 0.1).clamp(0.0, 100.0);
                c.membrane = (old.membrane * 0.94 + c.membrane * 0.06).clamp(0.0, 100.0);
                c.last_seen = age;
                c
            } else {
                let mut c = measured;
                c.id = self.next_id;
                self.next_id += 1;
                c.age = 1;
                c.last_seen = age;
                events.births += 1;
                c
            };

            if cluster.age > 50 && cluster.size > 14 {
                cluster.membrane = (cluster.membrane + 1.2).min(100.0);
            }

            if cluster.stability > 65.0 && cluster.size > 22 {
                cluster.membrane = (cluster.membrane + 0.9).min(100.0);
            }

            for &idx in &group {
                if let Some(p) = particles.get_mut(idx) {
                    p.cluster_id = Some(cluster.id);
                    p.mass = (p.mass + 0.004 * cluster.size as f32).clamp(0.55, 6.5);
                }
            }

            next_clusters.push(cluster);
        }

        let old_count = self.clusters.len();
        let new_count = next_clusters.len();

        if new_count < old_count {
            events.merges += old_count - new_count;
        }

        if new_count > old_count + events.births {
            events.splits += new_count.saturating_sub(old_count);
        }

        self.clusters = next_clusters;
        events
    }
}

#[derive(Default)]
pub struct ClusterEvents {
    pub births: usize,
    pub deaths: usize,
    pub merges: usize,
    pub splits: usize,
}

fn detect_groups(particles: &[Particle]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; particles.len()];
    let mut groups = Vec::new();

    for i in 0..particles.len() {
        if visited[i] {
            continue;
        }

        let mut stack = vec![i];
        let mut group = Vec::new();
        visited[i] = true;

        while let Some(idx) = stack.pop() {
            group.push(idx);

            for j in 0..particles.len() {
                if visited[j] {
                    continue;
                }

                let dx = particles[idx].x - particles[j].x;
                let dy = particles[idx].y - particles[j].y;
                let dist = (dx * dx + dy * dy).sqrt();

                let link = 0.082 + particles[idx].genome.bonding * 0.019 + particles[idx].mass * 0.004;

                if dist < link {
                    visited[j] = true;
                    stack.push(j);
                }
            }
        }

        groups.push(group);
    }

    groups
}

fn measure_group(indices: &[usize], particles: &[Particle]) -> Cluster {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    let mut tribe_counts = [0usize; 6];
    let mut membrane = 0.0;

    for &idx in indices {
        let p = particles[idx];
        x += p.x;
        y += p.y;
        vx += p.vx;
        vy += p.vy;
        membrane += p.genome.membrane;
        tribe_counts[p.tribe.index()] += 1;
    }

    let count = indices.len() as f32;
    x /= count;
    y /= count;
    vx /= count;
    vy /= count;
    membrane = (membrane / count * 74.0).clamp(0.0, 100.0);

    let mut radius = 0.0;

    for &idx in indices {
        let p = particles[idx];
        let dx = p.x - x;
        let dy = p.y - y;
        radius += (dx * dx + dy * dy).sqrt();
    }

    radius /= count;

    let mut best = 0;

    for i in 1..6 {
        if tribe_counts[i] > tribe_counts[best] {
            best = i;
        }
    }

    let stability = ((indices.len() as f32 * 4.5) - radius * 125.0).clamp(0.0, 100.0);

    Cluster {
        id: 0,
        age: 0,
        size: indices.len(),
        x,
        y,
        vx,
        vy,
        radius,
        dominant: Tribe::from_index(best),
        stability,
        membrane,
        last_seen: 0,
    }
}
