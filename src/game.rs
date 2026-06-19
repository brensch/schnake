use crate::app::App;
use crate::dom::by_id;
use crate::protocol::{opposite, Direction, NetMsg, Point, Snake, GRID_H, GRID_W};
use web_sys::{HtmlInputElement, RtcDataChannelState};

impl App {
    pub(crate) fn sync_name(&mut self) {
        if let Ok(input) = by_id::<HtmlInputElement>(&self.document, "settings-name") {
            let trimmed = input.value().trim().to_string();
            if !trimmed.is_empty() {
                self.local_name = trimmed.chars().take(18).collect();
            }
        }
        self.sync_profile_from_dom();
    }

    pub(crate) fn add_snake(&mut self, id: String, name: String, color: String) {
        if self.state.snakes.iter().any(|snake| snake.id == id) {
            return;
        }
        let index = self.state.snakes.len() as i32;
        let head = Point {
            x: 5 + (index * 5) % (GRID_W - 8),
            y: 5 + (index * 3) % (GRID_H - 8),
        };
        let snake = Snake {
            id,
            name,
            color,
            body: vec![
                head,
                Point {
                    x: head.x - 1,
                    y: head.y,
                },
                Point {
                    x: head.x - 2,
                    y: head.y,
                },
            ],
            dir: Direction::Right,
            pending: Direction::Right,
            score: 0,
        };
        self.state.snakes.push(snake);
        self.place_apple_if_needed();
        self.render_scoreboard();
    }

    pub(crate) fn set_input(&mut self, id: &str, dir: Direction) {
        if let Some(snake) = self.state.snakes.iter_mut().find(|snake| snake.id == id) {
            if !opposite(snake.dir, dir) {
                snake.pending = dir;
            }
        }
    }

    pub(crate) fn start_game(&mut self) {
        if self.role == crate::app::Role::Host {
            self.state.started = true;
            self.set_status("Game started.");
            self.show_stage("game-screen");
            self.broadcast_state();
        }
    }

    pub(crate) fn tick_host(&mut self) {
        self.state.tick += 1;
        let occupied_before: Vec<Point> = self
            .state
            .snakes
            .iter()
            .flat_map(|snake| snake.body.iter().copied())
            .collect();

        let mut reset_ids = Vec::new();
        for snake in &mut self.state.snakes {
            snake.dir = snake.pending;
            let mut next = snake.body[0];
            match snake.dir {
                Direction::Up => next.y -= 1,
                Direction::Down => next.y += 1,
                Direction::Left => next.x -= 1,
                Direction::Right => next.x += 1,
            }

            let ate = next == self.state.apple;
            snake.body.insert(0, next);
            if !ate {
                snake.body.pop();
            } else {
                snake.score += 1;
            }

            let hit_wall = next.x < 0 || next.y < 0 || next.x >= GRID_W || next.y >= GRID_H;
            let hit_body = occupied_before
                .iter()
                .filter(|point| **point == next)
                .count()
                > 0;
            if hit_wall || hit_body {
                reset_ids.push(snake.id.clone());
            }
        }

        for id in reset_ids {
            self.reset_snake(&id);
        }
        self.place_apple_if_needed();
        self.render_scoreboard();
    }

    pub(crate) fn broadcast_state(&self) {
        let message = NetMsg::State {
            state: self.state.clone(),
        };
        for channel in &self.peer_channels {
            self.send_channel(channel, &message);
        }
    }

    pub(crate) fn send_to_host(&self, message: &NetMsg) {
        if let Some(channel) = &self.join_channel {
            self.send_channel(channel, message);
        }
    }

    pub(crate) fn send_channel(&self, channel: &web_sys::RtcDataChannel, message: &NetMsg) {
        if channel.ready_state() == RtcDataChannelState::Open {
            let _ = channel.send_with_str(&serde_json::to_string(message).unwrap());
        }
    }

    fn reset_snake(&mut self, id: &str) {
        let spawn = self.next_spawn();
        if let Some(snake) = self.state.snakes.iter_mut().find(|snake| snake.id == id) {
            snake.body = vec![
                spawn,
                Point {
                    x: spawn.x - 1,
                    y: spawn.y,
                },
                Point {
                    x: spawn.x - 2,
                    y: spawn.y,
                },
            ];
            snake.dir = Direction::Right;
            snake.pending = Direction::Right;
            snake.score = 0;
        }
    }

    fn next_spawn(&mut self) -> Point {
        Point {
            x: 4 + (self.next_rand() % (GRID_W as u32 - 8)) as i32,
            y: 4 + (self.next_rand() % (GRID_H as u32 - 8)) as i32,
        }
    }

    fn place_apple_if_needed(&mut self) {
        let occupied: Vec<Point> = self
            .state
            .snakes
            .iter()
            .flat_map(|snake| snake.body.iter().copied())
            .collect();
        if !occupied.contains(&self.state.apple) {
            return;
        }

        for _ in 0..128 {
            let point = Point {
                x: (self.next_rand() % GRID_W as u32) as i32,
                y: (self.next_rand() % GRID_H as u32) as i32,
            };
            if !occupied.contains(&point) {
                self.state.apple = point;
                return;
            }
        }
    }

    fn next_rand(&mut self) -> u32 {
        self.state.seed = self
            .state
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state.seed
    }
}
