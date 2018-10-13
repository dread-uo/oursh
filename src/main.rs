extern crate oursh;
extern crate termion;

use std::env;
use std::process::exit;
use std::io::{self, Read, Write};
use oursh::job::Job;
use oursh::program::{parse_primary, Program};
use oursh::repl;
use termion::is_tty;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

// Our shell, for the greater good. Ready and waiting.
fn main() {
    // Process text in raw mode style if we're attached to a tty.
    if is_tty(&io::stdin()) {
        // Standard input file descriptor (0), used for user input from the
        // user of the shell.
        let stdin = io::stdin();

        // Standard output file descriptor (1), used to display program output
        // to the user of the shell.
        let mut stdout = io::stdout().into_raw_mode()
            .expect("error opening raw mode");

        // A styled static (for now) prompt.
        let prompt = repl::Prompt::new()
            .long_style();

        prompt.display(&mut stdout);

        let mut text = String::new();
        for c in stdin.keys() {
            match c.unwrap() {
                Key::Esc => exit(0),
                Key::Char('\n') => {
                    print!("\n\r");
                    stdout.flush().unwrap();

                    stdout.suspend_raw_mode().unwrap();
                    parse_and_run(&text);
                    stdout.activate_raw_mode().unwrap();

                    // Reset the text for the next program.
                    text.clear();

                    // Print a boring static prompt.
                    prompt.display(&mut stdout);
                },
                Key::Char(c) => {
                    text.push(c);
                    print!("{}", c);
                    stdout.flush().unwrap();
                },
                Key::Backspace => {
                    if !text.is_empty() {
                        text.pop();
                        print!("{}{}",
                               termion::cursor::Left(1),
                               termion::clear::UntilNewline);
                        stdout.flush().unwrap();
                    }
                }
                Key::Ctrl('c') => {
                    text.clear();
                    print!("\n\r");
                    prompt.display(&mut stdout);
                },
                _ => {}
            }
        }
    } else {
        let stdin = io::stdin();
        let mut text = String::new();
        stdin.lock().read_to_string(&mut text).unwrap();
        parse_and_run(&text);
    }
}

fn parse_and_run(text: &String) {
    // Parse with the primary grammar and run each command in order.
    match parse_primary(text.as_bytes()) {
        Ok(program) => {
            if let Some(arg1) = env::args().nth(1) {
                if arg1 == "-v" || arg1 == "--verbose" {
                    println!("{:#?}", program);
                }
            }

            program.run()
                .expect(&format!("error running program: {:?}", program));
        },
        Err(()) => {
            println!("error parsing text: {}", text);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_has_a_test() {}
}