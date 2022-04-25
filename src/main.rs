use std::fmt::Display;

use bracket_lib::prelude::*;
use rand::distributions::WeightedIndex;
use rand::prelude::{Distribution, IteratorRandom};
use rand::thread_rng;
use strum::IntoEnumIterator;
use strum_macros::EnumIter; // etc.

#[derive(Default)]
struct Statistic {
    inflected: u32,
    immune: u32,
    susceptible: u32,
}
impl Statistic {
    fn total(&self) -> u32 {
        self.inflected + self.immune + self.susceptible
    }
}

impl Display for Statistic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Inflected: {}\n
            Immune: {}\n
            Susceptible: {}\n",
            self.inflected, self.immune, self.susceptible
        )
    }
}

impl Statistic {}
struct State {
    players: Vec<Player>,
    map: Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]>,
    frame_time: f32,
    init_fn: Box<
        dyn Fn() -> (
            Vec<Player>,
            Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]>,
            Statistic,
        ),
    >,
    statistic: Statistic,
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.frame_time += ctx.frame_time_ms;

        ctx.cls_bg(NAVY);

        self.show_info(ctx);

        self.players.iter_mut().for_each(|player| {
            // Add player to screen
            player.render(ctx);
        });

        if let Some(VirtualKeyCode::R) = ctx.key {
            self.restart();
        }

        if self.frame_time > FRAME_TIME {
            self.frame_time = 0.0; // reset

            let mut x_y_before_move = vec![];

            self.players.iter_mut().for_each(|player| {
                // Update map
                player.update_position_in_map(&mut self.map);
            });

            /*
            FIX STATE: State is a moment which be based on for next health-check
             */
            let fixed_map = self.map.clone();

            self.players.iter_mut().for_each(|player| {
                /* Handle something BUT don't change any state. */
                // Handle player health
                if player.meet_infected(&fixed_map) {
                    if player.health_state == HealthState::Susceptible {
                        player.health_state = HealthState::Inflected;
                        self.statistic.inflected += 1;
                        self.statistic.susceptible -= 1;
                    }
                }

                // Deal with movement
                if let Some(old_position) = player.keep_moving(&mut self.map) {
                    x_y_before_move.push(old_position);
                }

                // println!("Step: {}", player.steps);
            });

            // Update map by deleting block that player have been gone.
            x_y_before_move.iter().for_each(|p| {
                self.map[p.1][p.0].take();
            });
        }
    }
}

impl State {
    fn new(
        players: Vec<Player>,
        map: Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]>,
        init_fn: Box<
            dyn Fn() -> (
                Vec<Player>,
                Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]>,
                Statistic,
            ),
        >,
        statistic: Statistic,
    ) -> Self {
        State {
            players,
            map,
            frame_time: 0.0,
            init_fn,
            statistic,
        }
    }

    fn restart(&mut self) {
        let (players, map, statistic) = (self.init_fn)();
        self.players = players;
        self.map = map;
        self.statistic = statistic;
    }

    fn show_info(&self, ctx: &mut BTerm) {
        ctx.print(0, 0, "Press R to restart.");
        ctx.print_color(
            0,
            1,
            RED,
            BLACK,
            format!("   Infected: {}", &self.statistic.inflected),
        );
        ctx.print_color(
            0,
            2,
            GREEN,
            BLACK,
            format!("     Immune: {}", &self.statistic.immune),
        );
        ctx.print_color(
            0,
            3,
            YELLOW,
            BLACK,
            format!("Susceptible: {}", &self.statistic.susceptible),
        );
        ctx.print(0, 4, format!("      Total: {}", &self.statistic.total()));
        ctx.set_fancy(
            PointF { x: 0.0, y: 5.0 },
            0,
            Radians::new(0.0),
            PointF { x: 2.0, y: 2.0 },
            YELLOW,
            BLACK,
            to_cp437('@'),
        )
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
    ) -> Option<(usize, usize)> {
        self.change_dir(map);

        if !self.end_way(map) {
            /*
            We should not remove player from old position because it has been left.
            If we do that, successive player will recursively step in a empty position.
             */
            let ret = Some((self.x, self.y));
            self.move_1_step();

            /*
            But we should put player in new position to prevent other player step into same position.
            */
            let _ = map[self.y][self.x].insert(self.clone()); //
            return ret;
        }

        None
    }

    fn change_dir(
        &mut self,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    ) {
        let old_dir = self.dir;
        if self.is_lounging && self.steps > MIN_STEP {
            self.dir = Direction::iter().choose(&mut thread_rng()).unwrap();
            if self.dir != old_dir {
                self.steps = 0;
            }
        } else if self.end_way(map) {
            self.dir = Direction::iter()
                .filter(|x| x != &self.dir)
                .choose(&mut thread_rng())
                .unwrap();
        }
    }

    fn end_way(
        &self,
        map: &[[Option<Player>; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
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

    fn move_1_step(&mut self) {
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

        if self.is_lounging {
            self.steps += 1
        };
    }

    fn update_position_in_map(&self, map: &mut [[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]) {
        let _ = map[self.y][self.x].insert(self.clone());
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

    fn render(&mut self, ctx: &mut BTerm) {
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

const SCREEN_HEIGHT: usize = 70;
const SCREEN_WIDTH: usize = 100;
const FRAME_TIME: f32 = 80.0;
const SAFE_DISTANCE: usize = 5;

fn main() -> BError {
    let mut random = RandomNumberGenerator::new();
    // input
    let (infected, immune, susceptible) = (1, 300, 400);
    let peoples = 1000;

    let init_fn = Box::new(move || generate(peoples, infected, immune, susceptible));

    let (players, map, s) = init_fn();

    let context = BTermBuilder::new()
        .with_dimensions(SCREEN_WIDTH, SCREEN_HEIGHT)
        .with_tile_dimensions(8, 8)
        .with_title("Virus")
        .with_font("terminal8x8.png", 8, 8)
        .with_simple_console(SCREEN_WIDTH, SCREEN_HEIGHT, "terminal8x8.png")
        .build()?;

    main_loop(context, State::new(players, map, init_fn, s))
}

fn generate(
    peoples: u32,
    infected: u32,
    immune: u32,
    susceptible: u32,
) -> (
    Vec<Player>,
    Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]>,
    Statistic,
) {
    let mut random = RandomNumberGenerator::new();
    let is_lounging = if random.range(0, 2) == 1 { true } else { false };

    let mut statistic = Statistic::default();

    // begin
    let mut players = vec![];
    let mut map: Box<[[Option<Player>; SCREEN_WIDTH]; SCREEN_HEIGHT]> =
        Box::new([[None; SCREEN_WIDTH]; SCREEN_HEIGHT]);

    let mut count = 0;

    (0..peoples).for_each(|i| {
        let x = random.range(0, SCREEN_WIDTH);
        let y = random.range(0, SCREEN_HEIGHT);
        let dir = Direction::iter().choose(&mut thread_rng()).unwrap();
        let health_state = generate_health_state(infected, immune, susceptible);
        let player = Player::new(x, y, dir, is_lounging, health_state);
        if map[y][x].is_none() {
            players.push(player);
            let _ = map[y][x].insert(player);
            count += 1;
            match health_state {
                HealthState::Inflected => statistic.inflected += 1,
                HealthState::Immune => statistic.immune += 1,
                HealthState::Susceptible => statistic.susceptible += 1,
            }
        }
    });

    (players, map, statistic)
}

fn generate_health_state(inflected: u32, immune: u32, susceptible: u32) -> HealthState {
    let items = [
        (HealthState::Inflected, inflected),
        (HealthState::Immune, immune),
        (HealthState::Susceptible, susceptible),
    ];
    let dist2 = WeightedIndex::new(items.iter().map(|item| item.1)).unwrap();
    items[dist2.sample(&mut thread_rng())].0
}

fn generate_health_state_sequence(
    infected: u32,
    immune: u32,
    susceptible: u32,
) -> impl Iterator<Item = HealthState> {
    let mut v = vec![];

    (0..infected).for_each(|_| {
        v.push(HealthState::Inflected);
    });
    (0..immune).for_each(|_| {
        v.push(HealthState::Immune);
    });
    (0..susceptible).for_each(|_| {
        v.push(HealthState::Susceptible);
    });

    v.into_iter().cycle()
}
