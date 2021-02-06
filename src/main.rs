use std::{
    io::{
        stdin,
        stdout,
        Stdout,
        Stdin,
        Write,
        Error
    },
    thread,
    time
};
use termion::{
    terminal_size,
    raw::{
        IntoRawMode,
        RawTerminal
    },
    cursor::{
        Goto
    },
    color,
    clear
};

// TermPos is a 1-indexed character cell in the Term.
#[derive(Debug)]
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
}
struct State {
    trails: Vec<Trail>,
    term_size: (u16, u16)
}

fn tick(mut state: &mut State) {
    for trail in &mut state.trails {
        trail.bottom.y += 1;
    }
}

fn render_trail(stdout: &mut RawTerminal<Stdout>, trail: &Trail) -> Result<(), Error> {
    for i in 0..trail.len {
        let y = (trail.bottom.y as i32) - (i as i32);
        let x = trail.bottom.x; 

        if y < 1 {
            continue;
        }

        write!(
            stdout,
            "{}{}{}",
            Goto(x as u16, y as u16),
            color::Fg(color::LightGreen),
            'x'
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

fn main() -> Result<(), Error> {
    let stdint = stdin();
    let mut stdout = stdout().into_raw_mode()?;
    let term_size: (u16, u16) = match terminal_size() {
        Ok(size) => size,
        Err(_) => panic!("cannot get term size")
    };

    let mut state: State = State {
        trails: vec![Trail {
            bottom: TermPos{
                x: 6,
                y: 12
            },
            len: 3
        }],
        term_size: term_size
    };


    write!(stdout, "{}{}{}", clear::All, Goto(1,1), color::Fg(color::Reset));
    stdout.flush()?;

    for _ in 0..10 {
        render(&mut stdout, &state);
        tick(&mut state);
        thread::sleep(time::Duration::from_millis(1000));
    }

    write!(stdout, "{}{}{}", clear::All, Goto(1,1), color::Fg(color::Reset));
    stdout.flush()?;

    return Ok(());
}