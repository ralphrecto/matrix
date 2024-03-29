use std::{
    io::{stdout, Stdout, Write, Error, Read},
    cmp::{min, max},
    thread,
    time,
    env
};
use termion::{
    terminal_size,
    async_stdin,
    AsyncReader,
    raw::{IntoRawMode, RawTerminal},
    cursor,
    color,
    clear
};
use rand::{
    thread_rng,
    Rng
};
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8 
}

impl Color {
    const PURE_GREEN: Color = Color { r: 0, g: 5, b: 0};
    const DARK_GREEN: Color = Color { r: 0, g: 1, b: 0 };
}

// TermPos is a 1-indexed character cell in the Term.
#[derive(Debug, Clone, Copy)]
struct TermPos {
    x: u8,
    y: u8
}

// A Trail is a vertical sequence of characters on the screen.
#[derive(Debug)]
struct Trail {
    // Trails are drawn from the bottom up for its len.
    // Generally, it should dim in color as its drawn up. 
    bottom: TermPos,
    len: usize,
    speed: i32
}

impl Trail {
    const MAX_LEN: usize = 12;
    const MAX_SPEED: i32 = 3;

    fn new(x: u8, y: u8, len: usize, speed: i32) -> Trail {
        Trail {
            bottom: TermPos {
                x,
                y
            },
            speed,
            len
        }
    }

    fn random(term_size: (u16, u16)) -> Trail {
        let x = thread_rng().gen_range(1..term_size.0);
        let y = thread_rng().gen_range(1..term_size.1);
        let len = thread_rng().gen_range(3..Trail::MAX_LEN);
        let speed = thread_rng().gen_range(1..Trail::MAX_SPEED);

        Trail::new(x as u8, y as u8, len, speed)
    }

    fn is_visible(&self, term_size: (u16, u16)) -> bool {
        let top = self.bottom.y as i32 - self.len as i32;
        top < term_size.1 as i32
    }

    fn gen_char(charset: &Vec<char>) -> char {
        charset[thread_rng().gen_range(0..charset.len())]
    }

    fn render(&self, stdout: &mut RawTerminal<Stdout>, rain_charset: &Vec<char>) -> Result<(), Error> {
        let interpolates: Vec<Color> = interpolate(Color::PURE_GREEN, Color::DARK_GREEN, self.len as u8);

        for i in 0..self.len {
            let y = (self.bottom.y as i32) - (i as i32);
            let x = self.bottom.x; 
            let color: Color = interpolates[i];

            if y < 1 {
                continue;
            }

            write!(
                stdout,
                "{}{}{}",
                cursor::Goto(x as u16, y as u16),
                color::Fg(color::AnsiValue::rgb(color.r, color.g, color.b)),
                Trail::gen_char(rain_charset)
            )?;
            stdout.flush()?;
        }

        Ok(())
    }
}

// Defaults for Config parameters.
const DEFAULT_TRAIL_DENSITY: u32 = 30;
const DEFAULT_RAIN_CHARSET: &'static [char] = &[
    'x', 'A', 'z', 'O',
    '\u{00D8}', '\u{01C2}', '\u{03A9}', '\u{01E3}', '\u{03FC}',
    '\u{305B}', '\u{3091}'
];


// User-controllable parameters that change rendering.
struct Config {
    // Will render 1 trail per $TRAIL_DENSITY terminal squares.
    trail_density: u32,
    // Set of characters to sample from when displaying the rain.
    rain_charset: Vec<char>
}

impl Config {
    pub fn create() -> Config {
        let trail_density_env: Option<u32> = env::var("TRAIL_DENSITY").ok()
            .and_then(|s| u32::from_str(&s).ok());

        let trail_density: u32 = match trail_density_env {
            Some(d) => d,
            _ => DEFAULT_TRAIL_DENSITY
        };

        let rain_charset_env: Option<Vec<char>> = env::var("RAIN_CHARSET").ok()
            .and_then(|s| Some(s.chars().collect()));

        let rain_charset: Vec<char> = match rain_charset_env {
            Some(cs) => cs,
            _ => DEFAULT_RAIN_CHARSET.iter().map(|c| *c).collect()
        };

        Config {
            trail_density: trail_density,
            rain_charset: rain_charset
        }
    }
}

// Holds all relevant state for rendering the digital rain.
struct State {
    // Current trails that are rendered on the terminal.
    trails: Vec<Trail>,
    // Dimensions (in characters) of the terminal.
    term_size: (u16, u16),
    // Other params used when rendering.
    config: Config
}

impl State {
    fn new(term_size: (u16, u16)) -> State {
        let config =  Config::create();
        let num_trails = (term_size.0 as i32 * term_size.1 as i32) as i32 / config.trail_density as i32;

        let mut trails: Vec<Trail> = vec![];
        for _i in 0..num_trails {
            trails.push(Trail::random(term_size));
        }

        State {
            trails,
            term_size,
            config
        }
    }
}

fn compute_step_size(c1: u8, c2: u8, steps: u8) -> i32 {
    if c1 == c2 {
        return 0;
    }

    let f1 = c1 as f32;
    let f2 = c2 as f32;
    let fsteps = steps as f32;

    let step = ((f2 - f1) / fsteps).floor() as i32;

    if step == 0 { 1 } else { step }
}

const ANSI_RGB_MAX: u8 = 5;
const ANSI_RGB_MIN: u8 = 0;

fn clip(val: u8, interpolatee_val: u8) -> u8 {
    let min_bound = min(interpolatee_val, ANSI_RGB_MIN);
    let max_bound = max(interpolatee_val, ANSI_RGB_MAX);

    min(max(min_bound, val), max_bound)
}

fn interpolate(c1: Color, c2: Color, steps: u8) -> Vec<Color> {
    let mut interpolates : Vec<Color> = vec![];
    let rdelta = compute_step_size(c1.r, c2.r, steps);
    let gdelta = compute_step_size(c1.g, c2.g, steps);
    let bdelta = compute_step_size(c1.b, c2.b, steps);

    for i in 0i32..(steps as i32) {
        (&mut interpolates).push(Color {
            r: clip((c1.r as i32 + (i * rdelta)) as u8, c2.r),
            g: clip((c1.g as i32 + (i * gdelta)) as u8, c2.g),
            b: clip((c1.b as i32 + (i * bdelta)) as u8, c2.b)
        });
    }

    interpolates
}

fn tick(state: &mut State) {
    // Replace trails if they are no longer visible.
    for i in 0..state.trails.len() {
        if !state.trails[i].is_visible(state.term_size) {
            state.trails[i] = Trail::random(state.term_size);
        }
    }

    // Move each trail down.
    for trail in &mut state.trails {
        trail.bottom.y += trail.speed as u8;
    }
}

fn render(mut stdout: &mut RawTerminal<Stdout>, state: &State) -> Result<(), Error> {
    write!(stdout, "{}", clear::All)?;
    for trail in &state.trails {
        trail.render(&mut stdout, &state.config.rain_charset)?;
    }

    Ok(())
}

fn read_key(stdin: &mut AsyncReader) -> Option<u8> {
    match stdin.bytes().next() {
        Some(event_res) => match event_res {
             Ok(evt) => Some(evt),
             Err(_) => None
        },
        _ => None
    }
}

fn clear_screen(stdout: &mut RawTerminal<Stdout>) -> Result<(), Error>  {
    write!(stdout, "{}{}{}", clear::All, cursor::Goto(1,1), color::Fg(color::Reset))?;
    stdout.flush()
}

fn main() -> Result<(), Error> {
    // Set up stdin/stdout.
    let mut stdin = async_stdin();
    let mut stdout = stdout().into_raw_mode()?;

    // Set up data.
    let term_size: (u16, u16) = match terminal_size() {
        Ok(size) => size,
        Err(_) => panic!("cannot get term size")
    };

    let mut state: State = State::new(term_size);

    // Enter main loop.
    clear_screen(&mut stdout)?;
    loop {
        tick(&mut state);
        render(&mut stdout, &state)?;

        match read_key(&mut stdin) {
            Some(b'q') => break,
            _ => ()
        }

        thread::sleep(time::Duration::from_millis(150));
    }
    clear_screen(&mut stdout)?;

    Ok(())
}