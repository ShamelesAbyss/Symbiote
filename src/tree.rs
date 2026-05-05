//! Symbiote tree ecology brain.
//!
//! This module centralizes root, trunk, branch, canopy, terrain,
//! collision, rendering, and future memory policy so tree behavior
//! is not scattered across the simulation layers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeStage {
    Root,
    Trunk,
    Branch,
    Canopy,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct TreePolicy {
    pub root_cap_divisor: usize,
    pub min_parent_age: u32,
    pub max_neighbor_roots: usize,
    pub max_local_roots: usize,
    pub allow_soft_invasion: bool,
    pub allow_wall_collision: bool,
    pub allow_surface_flow: bool,
}

impl Default for TreePolicy {
    fn default() -> Self {
        Self {
            root_cap_divisor: 26,
            min_parent_age: 3,
            max_neighbor_roots: 5,
            max_local_roots: 8,
            allow_soft_invasion: true,
            allow_wall_collision: true,
            allow_surface_flow: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeForces {
    pub avoidance_radius: f32,
    pub force_scale: f32,
    pub channel_force: f32,
    pub bounce: f32,
    pub surface_flow: f32,
}

impl TreeForces {
    pub const DEFAULT: Self = Self {
        avoidance_radius: 0.092,
        force_scale: 1.58,
        channel_force: 0.72,
        bounce: 0.72,
        surface_flow: 0.42,
    };
}

impl Default for TreeForces {
    fn default() -> Self {
        Self {
            avoidance_radius: 0.092,
            force_scale: 1.58,
            channel_force: 0.72,
            bounce: 0.72,
            surface_flow: 0.42,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeVisualPolicy {
    pub root_zone: f32,
    pub trunk_zone: f32,
    pub branch_zone: f32,
    pub wiggle_rate: u64,
}

impl Default for TreeVisualPolicy {
    fn default() -> Self {
        Self {
            root_zone: 0.72,
            trunk_zone: 0.38,
            branch_zone: 0.18,
            wiggle_rate: 5,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeProfile {
    pub policy: TreePolicy,
    pub forces: TreeForces,
    pub visuals: TreeVisualPolicy,
}

pub fn tree_stage_for_height(y: usize, height: usize) -> TreeStage {
    let ratio = y as f32 / height.max(1) as f32;
    let visuals = TreeVisualPolicy::default();

    if ratio > visuals.root_zone {
        TreeStage::Root
    } else if ratio > visuals.trunk_zone {
        TreeStage::Trunk
    } else if ratio > visuals.branch_zone {
        TreeStage::Branch
    } else {
        TreeStage::Canopy
    }
}

pub fn root_cap(total_cells: usize, width: usize, policy: TreePolicy) -> usize {
    (total_cells / policy.root_cap_divisor).max((width / 2).max(12))
}

#[allow(dead_code)]
pub fn is_soft_root_target_name(kind_name: &str) -> bool {
    matches!(kind_name, "Empty" | "Life" | "Nutrient" | "Dead" | "Spore")
}

pub fn allow_root_direction(
    near_wall: bool,
    vertical_parent: bool,
    diagonal_parent: bool,
    lateral_parent: bool,
) -> bool {
    vertical_parent || diagonal_parent || (near_wall && lateral_parent)
}

pub fn growth_pressure(
    cycle: u64,
    height_ratio: f32,
    vertical_parent: bool,
    diagonal_parent: bool,
    lateral_wall_parent: bool,
    parent_age: u32,
    root_count: usize,
    root_cap: usize,
) -> usize {
    let early_pressure: usize = if cycle < 4_200 && height_ratio > 0.42 {
        34
    } else if cycle < 6_800 && height_ratio > 0.28 {
        16
    } else {
        0
    };

    let parent_bias: usize = if vertical_parent {
        62
    } else if diagonal_parent {
        38
    } else if lateral_wall_parent {
        9
    } else {
        0
    };

    let maturity_bias: usize = if parent_age > 260 {
        18
    } else if parent_age > 120 {
        13
    } else if parent_age > 40 {
        8
    } else {
        4
    };

    let taper: usize = if height_ratio < 0.22 {
        28
    } else if height_ratio < 0.36 {
        14
    } else if height_ratio < 0.48 {
        6
    } else {
        0
    };

    let population_pressure: usize = if root_count > root_cap.saturating_mul(3) / 4 {
        18
    } else if root_count > root_cap / 2 {
        8
    } else {
        0
    };

    parent_bias
        .saturating_add(maturity_bias)
        .saturating_add(early_pressure)
        .saturating_sub(taper + population_pressure)
}

pub fn accept_wiggle(diagonal_parent: bool, wall_parent: bool, wiggle_roll: usize) -> bool {
    if diagonal_parent && wiggle_roll > 5_200 {
        return false;
    }

    if wall_parent && wiggle_roll > 2_200 {
        return false;
    }

    true
}
