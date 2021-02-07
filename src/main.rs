use std::{
    io::{stdin, stdout, Stdout, Stdin, Write, Error, Read},
    cmp::{min, max},
    fmt,
    thread,
    time
};
use termion::{
    terminal_size,
    async_stdin,
    AsyncReader,
    input::{Keys, TermRead},
    raw::{IntoRawMode, RawTerminal},
    event::{Key, parse_event, Event},
    cursor,
    color,
    clear
};

use rand::{
    thread_rng,
    Rng
};

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
        return Trail {
            bottom: TermPos {
                x: x,
                y: y
            },
            speed: speed,
            len: len
        };
    }

    fn random(term_size: (u16, u16)) -> Trail {
        let x = thread_rng().gen_range(1..term_size.0);
        let y = thread_rng().gen_range(1..term_size.1);
        let len = thread_rng().gen_range(3..Trail::MAX_LEN);
        let speed = thread_rng().gen_range(1..Trail::MAX_SPEED);

        return Trail::new(x as u8, y as u8, len, speed);
    }

    fn is_visible(self: &Self, term_size: (u16, u16)) -> bool {
        let top = self.bottom.y as i32 - self.len as i32;
        return top < term_size.1 as i32;
    }

    const RAIN_CHARSET: &'static [char] = &[
        'x', 'A', 'z', 'O',
        '\u{00D8}', '\u{01C2}', '\u{03A9}', '\u{01E3}', '\u{03FC}',
        '\u{305B}', '\u{3091}'
    ];

    fn gen_char() -> char {
        return Trail::RAIN_CHARSET[thread_rng().gen_range(0..Trail::RAIN_CHARSET.len())];
    }

    fn render(self: &Self, stdout: &mut RawTerminal<Stdout>) -> Result<(), Error> {
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
                Trail::gen_char()
            );
            stdout.flush()?;
        }

        return Ok(());
    }
}

struct State {
    trails: Vec<Trail>,
    term_size: (u16, u16),
    num_trails: u32
}

impl State {
    fn new(term_size: (u16, u16), num_trails: u32) -> State {
        let mut trails: Vec<Trail> = vec![];

        for i in 0..num_trails {
            trails.push(Trail::random(term_size));
        }

        return State {
            trails: trails,
            term_size: term_size,
            num_trails: num_trails
        };
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

    return if step == 0 { 1 } else { step };
}

const ANSI_RGB_MAX: u8 = 5;
const ANSI_RGB_MIN: u8 = 0;

fn clip(val: u8, interpolatee_val: u8) -> u8 {
    let min_bound = min(interpolatee_val, ANSI_RGB_MIN);
    let max_bound = max(interpolatee_val, ANSI_RGB_MAX);

    return min(max(min_bound, val), max_bound);
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

    return interpolates;
}

fn tick(mut state: &mut State) {
    // Replace trails if they are no longer visible.
    for i in 0..state.trails.len() {
        if !state.trails[i].is_visible(state.term_size) {
            (&mut state).trails[i] = Trail::random(state.term_size);
        }
    }

    // Move each trail down.
    for trail in &mut state.trails {
        trail.bottom.y += trail.speed as u8;
    }
}

fn render(mut stdout: &mut RawTerminal<Stdout>, state: &State) {
    write!(stdout, "{}", clear::All);
    for trail in &state.trails {
        trail.render(&mut stdout);
    }
}

fn read_key(mut stdin: &mut AsyncReader) -> Option<u8> {
    return match stdin.bytes().next() {
        Some(event_res) => match event_res {
             Ok(evt) => Some(evt),
             Err(_) => None
        },
        _ => None
    }
}

fn clear_screen(mut stdout: &mut RawTerminal<Stdout>) -> Result<(), Error>  {
    write!(stdout, "{}{}{}", clear::All, cursor::Goto(1,1), color::Fg(color::Reset));
    return stdout.flush();
}

// Will render 1 trail per $TRAIL_DENSITY terminal squares.
const DEFAULT_TRAIL_DENSITY: u32 = 30;

fn main() -> Result<(), Error> {
    // Set up stdin/stdout.
    let mut stdin = async_stdin();
    let mut stdout = stdout().into_raw_mode()?;

    // Set up data.
    let term_size: (u16, u16) = match terminal_size() {
        Ok(size) => size,
        Err(_) => panic!("cannot get term size")
    };

    let num_trails = (term_size.0 as i32 * term_size.1 as i32) as i32 / DEFAULT_TRAIL_DENSITY as i32;
    let mut state: State = State::new(term_size, num_trails as u32);

    // Enter main loop.
    clear_screen(&mut stdout)?;
    loop {
        tick(&mut state);
        render(&mut stdout, &state);

        match read_key(&mut stdin) {
            Some(k) => match k {
                b'q' => break,
                _ => ()
            },
            _ => ()
        }

        thread::sleep(time::Duration::from_millis(150));
    }
    clear_screen(&mut stdout)?;

    return Ok(());
}