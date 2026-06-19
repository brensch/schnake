use serde::{Deserialize, Serialize};

pub const GRID_W: i32 = 32;
pub const GRID_H: i32 = 22;
pub const COLORS: [&str; 8] = [
    "#2f7d54", "#d14d41", "#2c6fb8", "#8f5bb3", "#c2851b", "#148184", "#bf4f8c", "#59636a",
];

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Snake {
    pub id: String,
    pub name: String,
    pub color: String,
    pub body: Vec<Point>,
    pub dir: Direction,
    pub pending: Direction,
    pub score: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GameState {
    pub apple: Point,
    pub snakes: Vec<Snake>,
    pub tick: u64,
    pub seed: u32,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            apple: Point { x: 15, y: 10 },
            snakes: Vec::new(),
            tick: 0,
            seed: 0x5eed_1234,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum NetMsg {
    Hello {
        id: String,
        name: String,
        color: String,
    },
    Input {
        id: String,
        dir: Direction,
    },
    State {
        state: GameState,
    },
}

#[derive(Deserialize, Serialize)]
pub struct Signal {
    pub sdp_type: String,
    pub sdp: String,
}

pub fn opposite(a: Direction, b: Direction) -> bool {
    matches!(
        (a, b),
        (Direction::Up, Direction::Down)
            | (Direction::Down, Direction::Up)
            | (Direction::Left, Direction::Right)
            | (Direction::Right, Direction::Left)
    )
}
