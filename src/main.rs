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
struct State {
    trails: Vec<Trail>,
    term_size: (u16, u16),
    num_trails: u32
}

fn new_trail(x: u8, y: u8, len: usize, speed: i32) -> Trail {
    return Trail {
        bottom: TermPos {
            x: x,
            y: y
        },
        speed: speed,
        len: len
    };
}

const MAX_TRAIL_LEN: usize = 12;
const MAX_TRAIL_SPEED: i32 = 3;

fn new_random_trail(term_size: (u16, u16)) -> Trail {
    let x = thread_rng().gen_range(1..term_size.0);
    let y = thread_rng().gen_range(1..term_size.1);
    let len = thread_rng().gen_range(3..MAX_TRAIL_LEN);
    let speed = thread_rng().gen_range(1..MAX_TRAIL_SPEED);

    return new_trail(x as u8, y as u8, len, speed);
}

fn new_state(term_size: (u16, u16), num_trails: u32) -> State {
    let mut trails: Vec<Trail> = vec![];

    for i in 0..num_trails {
        trails.push(new_random_trail(term_size));
    }

    return State {
        trails: trails,
        term_size: term_size,
        num_trails: num_trails
    };
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

fn is_visible(trail: &Trail, term_size: (u16, u16)) -> bool {
    let top = trail.bottom.y as i32 - trail.len as i32;
    return top < term_size.1 as i32;
}

fn tick(mut state: &mut State) {
    // Replace trails if they are no longer visible.
    for i in 0..state.trails.len() {
        if !is_visible(&state.trails[i], state.term_size) {
            (&mut state).trails[i] = new_random_trail(state.term_size);
        }
    }

    // Move each trail down.
    for trail in &mut state.trails {
        trail.bottom.y += trail.speed as u8;
    }
}

const PURE_GREEN: Color = Color { r: 0, g: 5, b: 0};
const DARK_GREEN: Color = Color { r: 0, g: 1, b: 0 };

const RAIN_CHARSET: &'static [char] = &[
    'x', 'A', 'z', 'O',
    '\u{00D8}', '\u{01C2}', '\u{03A9}', '\u{01E3}', '\u{03FC}',
    '\u{305B}', '\u{3091}'
];

fn gen_char() -> char {
    return RAIN_CHARSET[thread_rng().gen_range(0..RAIN_CHARSET.len())];
}

fn render_trail(stdout: &mut RawTerminal<Stdout>, trail: &Trail) -> Result<(), Error> {
    let interpolates: Vec<Color> = interpolate(PURE_GREEN, DARK_GREEN, trail.len as u8);

    for i in 0..trail.len {
        let y = (trail.bottom.y as i32) - (i as i32);
        let x = trail.bottom.x; 
        let color: Color = interpolates[i];

        if y < 1 {
            continue;
        }

        write!(
            stdout,
            "{}{}{}",
            cursor::Goto(x as u16, y as u16),
            color::Fg(color::AnsiValue::rgb(color.r, color.g, color.b)),
            gen_char()
        );
        stdout.flush()?;
    }

    return Ok(());
}


fn render(mut stdout: &mut RawTerminal<Stdout>, state: &State) {
    write!(stdout, "{}", clear::All);
    for trail in &state.trails {
        render_trail(&mut stdout, trail);
    }
}

fn read_stdin(mut stdin: &mut AsyncReader) -> Option<Event> {
    return match stdin.bytes().next() {
        Some(event_res) => match event_res {
             Ok(partial_event) => {
                let event = parse_event(partial_event, &mut stdin.bytes());
                return Some(event.unwrap());
             },
             Err(_) => panic!("error reading stdin")
        },
        _ => None
    }
}

fn main() -> Result<(), Error> {
    let mut stdin = async_stdin();
    let mut stdout = stdout().into_raw_mode()?;
    let term_size: (u16, u16) = match terminal_size() {
        Ok(size) => size,
        Err(_) => panic!("cannot get term size")
    };

    let mut state: State = new_state(term_size, 600);

    write!(stdout, "{}{}{}{}", clear::All, cursor::Goto(1,1), color::Fg(color::Reset), cursor::Hide);
    stdout.flush()?;

    for _ in 0..100 {
        tick(&mut state);
        render(&mut stdout, &state);

        match read_stdin(&mut stdin) {
            Some(evt) => match evt {
                Event::Key(k) => match k {
                    Key::Esc => break,
                    Key::Char('q') => break,
                    _ => ()
                }
                _ => ()
            },
            None => ()
        }

        thread::sleep(time::Duration::from_millis(150));
    }

    write!(stdout, "{}{}{}", clear::All, cursor::Goto(1,1), color::Fg(color::Reset));
    stdout.flush()?;

    return Ok(());
}