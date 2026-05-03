use ratatui::style::Color;

#[derive(Clone, Copy, Debug)]
pub enum Tribe {
    Blood,
    Moss,
    Deep,
    Solar,
    Dream,
    Static,
}

impl Tribe {
    pub fn from_index(index: usize) -> Self {
        match index % 6 {
            0 => Self::Blood,
            1 => Self::Moss,
            2 => Self::Deep,
            3 => Self::Solar,
            4 => Self::Dream,
            _ => Self::Static,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Blood => 0,
            Self::Moss => 1,
            Self::Deep => 2,
            Self::Solar => 3,
            Self::Dream => 4,
            Self::Static => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Blood => "BLD",
            Self::Moss => "MOS",
            Self::Deep => "DEP",
            Self::Solar => "SOL",
            Self::Dream => "DRM",
            Self::Static => "STC",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Blood => Color::Red,
            Self::Moss => Color::Green,
            Self::Deep => Color::Blue,
            Self::Solar => Color::Yellow,
            Self::Dream => Color::Magenta,
            Self::Static => Color::Cyan,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Genome {
    pub perception: f32,
    pub hunger: f32,
    pub bonding: f32,
    pub volatility: f32,
    pub orbit: f32,
    pub membrane: f32,
}

#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub tribe: Tribe,
    pub age: u32,
    pub health: f32,
    pub mass: f32,
    pub cluster_id: Option<u64>,
    pub genome: Genome,
}
