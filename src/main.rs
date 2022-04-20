use bracket_lib::prelude::*;
use rand::prelude::IteratorRandom;
use rand::seq::SliceRandom;
use rand::thread_rng;
use strum::IntoEnumIterator;
use strum_macros::EnumIter; // etc.

struct State {
    players: Vec<Player>,
    map: [[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    frame_time: f32,
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.frame_time += ctx.frame_time_ms;

        if self.frame_time > FRAME_TIME {
            ctx.cls_bg(NAVY);
            self.frame_time = 0.0;
            self.players.iter_mut().for_each(|player| {
                player.keep_moving(&mut self.map);
                player.render(ctx, &self.map);
                if player.meet_infected(&self.map) {
                    if player.health_state == HealthState::Susceptible {
                        player.health_state = HealthState::Inflected;
                    }
                }
                // println!("Step: {}", player.steps);
            });
        }
    }
}

impl State {
    fn new(players: Vec<Player>, map: [[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]) -> Self {
        State {
            players,
            map,
            frame_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq)]
enum HealthState {
    Inflected,
    Immune,
    Susceptible,
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Player {
    x: usize,
    y: usize,
    dir: Direction,
    is_lounging: bool,
    steps: u32,
    health_state: HealthState,
}
const MIN_STEP: u32 = 20;
// I am a player backend, and responsible for behavior of players.
impl Player {
    fn new(
        x: usize,
        y: usize,
        dir: Direction,
        is_lounging: bool,
        health_state: HealthState,
    ) -> Self {
        Player {
            x,
            y,
            dir,
            is_lounging,
            health_state,
            steps: 0,
        }
    }

    fn keep_moving(
        &mut self,
        map: &mut [[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) {
        self.change_dir(map);
        self.move_1_step(map);
    }

    fn change_dir(
        &mut self,
        map: &mut [[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) {
        let old_dir = self.dir;
        if self.is_lounging && self.steps > MIN_STEP {
            self.dir = Direction::iter().choose(&mut thread_rng()).unwrap();
        } else if self.end_way(map) {
            self.dir = Direction::iter()
                .filter(|x| x != &self.dir)
                .choose(&mut thread_rng())
                .unwrap();
        } else {
            return;
        }

        if self.is_lounging && self.steps > MIN_STEP && self.dir != old_dir {
            self.steps = 0;
        }
    }

    fn end_way(
        &self,
        map: &mut [[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) -> bool {
        match self.dir {
            Direction::Up => self.y == 0 || map[self.y - 1][self.x].is_some(),
            Direction::Down => self.y == SCREEN_HEIGHT - 1 || map[self.y + 1][self.x].is_some(),
            Direction::Left => self.x == 0 || map[self.y][self.x - 1].is_some(),
            Direction::Right => self.x == SCREEN_WIDTH - 1 || map[self.y][self.x + 1].is_some(),
        }
    }

    fn around_people(
        &self,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) -> Vec<Player> {
        let mut ret = vec![];
        for y in self.y.saturating_sub(SAFE_DISTANCE)..(self.y + SAFE_DISTANCE).min(SCREEN_HEIGHT) {
            for x in
                self.x.saturating_sub(SAFE_DISTANCE)..(self.x + SAFE_DISTANCE).min(SCREEN_WIDTH)
            {
                let distance = (((self.x as i64 - x as i64).pow(2)
                    + (self.y as i64 - y as i64).pow(2)) as f64)
                    .sqrt();
                if distance <= SAFE_DISTANCE as f64 {
                    map[y][x].map(|player| ret.push(player));
                }
            }
        }
        ret
    }

    fn move_1_step(&mut self, map: &mut [[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]) {
        if self.end_way(map) {
            return;
        }

        map[self.y][self.x].take();

        match self.dir {
            Direction::Up => {
                self.y = self.y.saturating_sub(1);
            }
            Direction::Down => {
                if self.y < SCREEN_HEIGHT - 1 {
                    self.y += 1;
                }
            }
            Direction::Left => {
                self.x = self.x.saturating_sub(1);
            }
            Direction::Right => {
                if self.x < SCREEN_WIDTH - 1 {
                    self.x += 1;
                }
            }
        }

        map[self.y][self.x].insert(self.clone());

        if self.is_lounging {
            self.steps += 1
        };
    }

    fn meet_infected(
        &mut self,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) -> bool {
        let around_people = self.around_people(map);
        around_people
            .iter()
            .filter(|player| player.health_state == HealthState::Inflected)
            .count()
            > 0
    }

    fn render(
        &mut self,
        ctx: &mut BTerm,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) {
        // self.render_position(ctx, map);

        let mut fg = GREEN;
        match self.health_state {
            HealthState::Immune => fg = GREEN,
            HealthState::Inflected => fg = RED,
            HealthState::Susceptible => fg = YELLOW,
        }
        ctx.set(self.x, self.y, fg, BLACK, to_cp437('@'));
    }

    fn render_position(
        &self,
        ctx: &mut BTerm,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) {
        for y in self.y.saturating_sub(SAFE_DISTANCE)..(self.y + SAFE_DISTANCE).min(SCREEN_HEIGHT) {
            for x in
                self.x.saturating_sub(SAFE_DISTANCE)..(self.x + SAFE_DISTANCE).min(SCREEN_WIDTH)
            {
                let distance = (((self.x as i64 - x as i64).pow(2)
                    + (self.y as i64 - y as i64).pow(2)) as f64)
                    .sqrt();
                if distance <= SAFE_DISTANCE as f64 {
                    if x != self.x || y != self.y {
                        ctx.set(x, y, GRAY, ORANGE, to_cp437(' '));
                    }
                }
            }
        }
    }
}

struct Obstacle {
    x: i32,     // where this obstacle put
    gap_y: i32, // position in obstacle where has a gap
    size: i32,  // size of gap
}

const SCREEN_HEIGHT: usize = 50;
const SCREEN_WIDTH: usize = 90;
const FRAME_TIME: f32 = 70.0;
const SAFE_DISTANCE: usize = 10;

fn main() -> BError {
    let mut random = RandomNumberGenerator::new();
    let mut players = vec![];

    let mut map: [[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT] =
        [[None; SCREEN_WIDTH]; SCREEN_HEIGHT];

    (0..600).for_each(|i| {
        let x = random.range(0, SCREEN_WIDTH);
        let y = random.range(0, SCREEN_HEIGHT);
        let dir = Direction::iter().choose(&mut thread_rng()).unwrap();
        let is_lounging = if random.range(0, 2) == 1 { true } else { false };
        let health_state = HealthState::iter().choose(&mut thread_rng()).unwrap();

        let player = Player::new(x, y, dir, true, HealthState::Susceptible);
        players.push(player);
        map[y][x].insert(player);
    });
    let player = Player::new(0, 0, Direction::Right, true, HealthState::Inflected);

    map[0][0].insert(player);
    // players.push(Player::new(HealthState::Inflected));
    // players.push(Player::new(HealthState::Immune));
    // players.push(Player::new(HealthState::Immune));
    // players.push(Player::new(HealthState::Susceptible));
    // players.push(Player::new(HealthState::Susceptible));
    // players.push(Player::new(
    //     0,
    //     1,
    //     Direction::Right,
    //     true,
    //     HealthState::Inflected,
    // ));

    // let context = BTermBuilder::simple80x50().with_title("flappy").build()?;

    let context = BTermBuilder::new()
        .with_dimensions(SCREEN_WIDTH, SCREEN_HEIGHT)
        .with_tile_dimensions(8, 8)
        .with_title("Virus")
        .with_font("terminal8x8.png", 8, 8)
        .with_simple_console(SCREEN_WIDTH, SCREEN_HEIGHT, "terminal8x8.png")
        .build()?;
    main_loop(context, State::new(players, map))
}
