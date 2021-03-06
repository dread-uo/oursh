//! The ubiquitous POSIX shell command language.
//!
//! This shell language (often called `sh`) is at the heart of the most popular
//! shells, namely `bash` and `zsh`. While shells typically implement many
//! extensions to the POSIX standard we'll be implementing only the most basic
//! set of functionality and offloading all extensions to the `modern`
//! language.
//!
//! # Compatibility
//!
//! Shell languages like `bash` or `zsh` are **supersets** of the POSIX `sh`
//! language. This means two things:
//!
//! - All `sh` programs are valid `bash`, `zsh`, etc programs
//! - Not all `bash` programs, for example, are valid `sh` programs.
//!
//! This explains why some shell scripts will start with `#!/bin/sh` or
//! `#!/bin/bash`, depending on what features of the language the script needs.
//!
//! # Examples
//!
//! There are more than enough examples of `sh` scripts out there, but here is
//! a collection of examples tested in **this** shell's implementation of the
//! POSIX standard. This section will not be a complete description of the
//! syntax of the `sh` language, but will be updated with as many interesting
//! cases as possible.
//!
//! Running a command (like `date`) in a shell script is the simplest thing
//! you can do.
//!
//! ```sh
//! date
//! date --iso-8601
//!
//! # You can even run programs from outside the $PATH.
//! ./a.out
//! ```
//!
//! All variables start with a `$` when referring to them, but when assigning
//! you omit the `$`. It's also worth mentioning that the lack of whitespace
//! around the `=` in assignment **is required**. It's often conventional to
//! write your variable names in all caps, but this is not a limitation of the
//! language.
//!
//! ```sh
//! NAME="Nathan Lilienthal"
//! i=0
//!
//! echo $NAME
//! echo $i
//! ```
//!
//! In addition to variables beginning with a `$` being expanded to the value
//! they were set to, other syntax can perform expansion. See section 3§2.6 for
//! a complete description of word expansion.
//!
//! ```sh
//! # Same as echo $1.
//! echo ${1}
//! # Use a default.
//! echo ${1:-default}
//! # Assign and use a default.
//! echo ${1:=default}
//! # Fail with error if $1 is unset.
//! echo ${1:?}
//! # Replace if not null.
//! echo ${1:+new}
//! # String length of $1.
//! echo ${#1}
//! # Remove suffix/prefix strings.
//! echo ${1%.*}
//! echo ${1%%.*}
//! echo ${1#prefix_}
//! ```
//!
//! In addition to running a program at the top level, programs can be run in
//! a subshell with mechanisms to capture the output. This is called command
//! substitution.
//!
//! ```sh
//! # Assign $files to be the output of ls.
//! files=`ls`
//! files=$(ls)  # Same as above.
//! ```
//!
//! Conditionals in the wild are often written with the non-POSIX `[[` syntax,
//! traditional conditional checks use either `test` or `[`.
//!
//! ```sh
//! # Check if $1 is absent.
//! if test -z "$1"; then
//!     exit 1
//! fi
//!
//! # Check if $1 is equal to "foo".
//! if [ "$1" -eq "foo" ]; then
//!     echo "bar"
//! fi
//! ```
//!
//! # Specification
//!
//! The syntax and semantics of this module are strictly defined by the POSIX
//! (IEEE Std 1003.1) standard, in section 3§2 of "The Open Group Base
//! Specifications" [[1]].
//!
//! [1]: http://pubs.opengroup.org/onlinepubs/9699919799/

use std::ffi::CString;
#[cfg(feature = "bridge")]
use std::fs::{self, File};
use std::io::{Write, BufRead};
#[cfg(feature = "bridge")]
use std::os::unix::fs::PermissionsExt;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use std::process::{self, Stdio};
use job::Job;
use program::{Result, Error, Program as ProgramTrait};
use program::builtin::{self, Builtin};
#[cfg(feature = "bridge")]
use program::ast::Interpreter;


// Re-export the two trait implementing types.
pub use self::ast::Program;
pub use self::ast::Command;

/// The syntax and semantics of a single POSIX command.
///
/// ```
/// use std::io::Read;
/// use oursh::program::Program as ProgramTrait;
/// use oursh::program::posix::ast::Program;
///
/// assert!(Program::parse(b"ls" as &[u8]).is_ok());
/// ```
impl super::Program for Program {
    type Command = Command;

    fn parse<R: BufRead>(mut reader: R) -> Result<Self> {
        let mut string = String::new();
        if reader.read_to_string(&mut string).is_err() {
            return Err(Error::Read);
        }

        // TODO #8: Custom lexer here.
        if let Ok(parsed) = lalrpop::ProgramParser::new().parse(&string) {
            Ok(parsed)
        } else {
            Err(Error::Parse)
        }
    }

    fn commands(&self) -> &[Box<Self::Command>] {
        &self.0[..]
    }
}

// TODO: lazy_static.
// const BUILTINS: HashMap<&'static str, &'static Builtin> = HashMap::new(...);

// The semantics of a single POSIX command.
impl super::Command for Command {
    fn run(&self) -> Result<WaitStatus> {
        #[allow(unreachable_patterns)]
        match *self {
            Command::Simple(ref words) => {
                let argv: Vec<CString> = words.iter().map(|w| {
                    CString::new(&w.0 as &str)
                        .expect("error in word UTF-8")
                }).collect();

                if let Some(command) = argv.clone().first() {
                    match command.to_string_lossy().as_ref() {
                        ":" => {
                            return builtin::Null::run(argv)
                        }
                        "exit" => {
                            return builtin::Exit::run(argv)
                        },
                        "cd" => {
                            return builtin::Cd::run(argv)
                        },
                        _ => {
                            return Job::new(argv).run()
                                          .map_err(|_| Error::Runtime)
                        },
                    }
                } else {
                    Ok(WaitStatus::Exited(Pid::this(), 0))
                }
            },
            Command::Compound(ref program) => {
                for command in program.0.iter() {
                    command.run()?;
                }
                Ok(WaitStatus::Exited(Pid::this(), 0))
            },
            Command::Not(ref command) => {
                match command.run() {
                    Ok(WaitStatus::Exited(p, c)) => {
                        Ok(WaitStatus::Exited(p, (c == 0) as i32))
                    }
                    Ok(s) => Ok(s),
                    Err(_) => Err(Error::Runtime),
                }
            },
            Command::And(ref left, ref right) => {
                match left.run() {
                    Ok(WaitStatus::Exited(_, c)) if c == 0 => {
                        right.run().map_err(|_| Error::Runtime)
                    },
                    Ok(s) => Ok(s),
                    Err(_) => Err(Error::Runtime),
                }
            },
            Command::Or(ref left, ref right) => {
                match left.run() {
                    Ok(WaitStatus::Exited(_, c)) if c != 0 => {
                        right.run().map_err(|_| Error::Runtime)
                    },
                    Ok(s) => Ok(s),
                    Err(_) => Err(Error::Runtime),
                }
            },
            Command::Subshell(ref program) => {
                // TODO #4: Run in a *subshell* ffs.
                program.run()
            },
            Command::Pipeline(ref left, ref right) => {
                // TODO: This is obviously a temporary hack.
                if let box Command::Simple(left_words) = left {
                    let mut child = process::Command::new(&left_words[0].0)
                        .args(left_words.iter().skip(1).map(|w| &w.0))
                        .stdout(Stdio::piped())
                        .spawn()
                        .expect("error swawning pipeline process");

                    let output = child.wait_with_output()
                        .expect("error reading stdout");

                    if let box Command::Simple(right_words) = right {
                        let mut child = process::Command::new(&right_words[0].0)
                            .args(right_words.iter().skip(1).map(|w| &w.0))
                            .stdin(Stdio::piped())
                            .spawn()
                            .expect("error swawning pipeline process");

                        {
                            let stdin = child.stdin.as_mut()
                                .expect("error opening stdin");
                            stdin.write_all(&output.stdout)
                                .expect("error writing to stdin");
                        }

                        child.wait()
                            .expect("error waiting for piped command");
                    }
                }
                Ok(WaitStatus::Exited(Pid::this(), 0))
            },
            Command::Background(ref program) => {
                program.run_background()?;
                println!("[?] ???");
                Ok(WaitStatus::Exited(Pid::this(), 0))
            },
            #[cfg(feature = "bridge")]
            Command::Bridgeshell(ref program) => {
                // TODO: Pass text off to another parser.
                if let Interpreter::Other(ref interpreter) = program.0 {
                    // TODO: Even for the Shebang interpretor, we shouldn't
                    // create files like this.
                    // XXX: Length is the worlds worst hash function.
                    let bridgefile = format!("/tmp/.oursh_bridge-{}", program.1.len());
                    {
                        // TODO: Use our job interface without creating any
                        // fucking files... The shebang isn't even a real
                        // POSIX standard.
                        let mut file = File::create(&bridgefile).unwrap();
                        let mut interpreter = interpreter.chars()
                                                       .map(|c| c as u8)
                                                       .collect::<Vec<u8>>();
                        interpreter.insert(0, '!' as u8);
                        interpreter.insert(0, '#' as u8);
                        // XXX: This is a huge gross hack.
                        interpreter = match &*String::from_utf8_lossy(&interpreter) {
                            "#!ruby"   => "#!/usr/bin/env ruby",
                            "#!node"   => "#!/usr/bin/env node",
                            "#!python" => "#!/usr/bin/env python",
                            "#!racket" => "#!/usr/bin/env racket",
                            i => i,
                        }.as_bytes().to_owned();
                        file.write_all(&interpreter).unwrap();
                        file.write_all(b"\n").unwrap();
                        let program = program.1.chars()
                                               .map(|c| c as u8)
                                               .collect::<Vec<u8>>();
                        file.write_all(&program).unwrap();

                        let mut perms = fs::metadata(&bridgefile).unwrap()
                                                               .permissions();
                        perms.set_mode(0o777);
                        fs::set_permissions(&bridgefile, perms).unwrap();
                    }
                    // TODO #4: Suspend and restore raw mode.
                    let mut child = process::Command::new(&format!("{}", bridgefile))
                        .spawn()
                        .expect("error swawning bridge process");
                    child.wait()
                        .expect("error waiting for bridge process");

                    Ok(WaitStatus::Exited(Pid::this(), 0))
                } else {
                    Err(Error::Runtime)
                }
            },
            #[cfg(not(feature = "bridge"))]
            Command::Bridgeshell(_) => {
                unimplemented!();
            },
        }
    }

    fn run_background(&self) -> Result<()> {
        match *self {
            Command::Simple(ref words) => {
                unimplemented!();
                // let argv: Vec<CString> = words.iter().map(|w| {
                //     CString::new(&w.0 as &str)
                //         .expect("error in word UTF-8")
                // }).collect();

                // match argv.first() {
                //     Some(command) => {
                //         Job::new(argv).run_background()
                //                       .map_err(|_| Error::Runtime)
                //     }
                //     None => Ok(())
                // }
            },
            _ => unimplemented!(),
        }
    }
}


/// Abstract Syntax Tree for the POSIX language.
pub mod ast {
    use program::ast::Interpreter;

    /// A program is the result of parsing a sequence of commands.
    #[derive(Debug, Clone)]
    pub struct Program(pub Vec<Box<Command>>);

    /// A program's text and the interpreter to be used.
    // TODO #8: Include grammar separate from interpreter?
    #[derive(Debug, Clone)]
    pub struct BridgedProgram(pub Interpreter, pub String);

    /// A command is a *highly* mutually-recursive node with the main features
    /// of the POSIX language.
    #[derive(Debug, Clone)]
    pub enum Command {
        /// Just a single command, with it's arguments.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// date --iso-8601
        /// ```
        // TODO #8: Simple should not just be a vec of words.
        Simple(Vec<Word>),

        /// A full program embedded in a compound command.
        ///
        /// ```sh
        /// { ls ; }
        /// ```
        // TODO #10: We are currently overpermissive here.
        Compound(Box<Program>),

        /// Performs boolean negation to the status code of the inner
        /// command.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// ! grep 'password' data.txt
        /// ```
        Not(Box<Command>),

        /// Perform the first command, conditionally running the next
        /// upon success.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// mkdir tmp && cd tmp
        /// ```
        And(Box<Command>, Box<Command>),

        /// Perform the first command, conditionally running the next
        /// upon failure.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// kill $1 || kill -9 $1
        /// ```
        Or(Box<Command>, Box<Command>),

        /// Run the inner **program** in a sub-shell environment.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// DATE=(date)
        /// ```
        Subshell(Box<Program>),

        /// Run a command's output through to the input of another.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// cat $1 | wc -l
        /// ```
        Pipeline(Box<Command>, Box<Command>),

        /// Run a command in the background.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// while true; do
        ///   sleep 1; echo "ping";
        /// done &
        /// ```
        Background(Box<Program>),

        /// Run a program through another parser/interpreter.
        ///
        /// ### Examples
        ///
        /// ```sh
        /// {#ruby puts (Math.sqrt(32**2/57.2))}
        /// ```
        ///
        /// ### Compatibility
        ///
        /// This is **non-POSIX**
        ///
        /// TODO: How bad is it?
        Bridgeshell(Box<BridgedProgram>),
    }

    /// A parsed word, already having gone through expansion.
    // TODO #8: How can we expand things like $1 or $? from the lexer?
    // TODO #8: This needs to handle escapes and all kinds of fun. We first
    //       need to decide on our custom Tokens and lexer.
    #[derive(Debug, Clone)]
    pub struct Word(pub String);


    impl Program {
        pub(crate) fn push(mut self, command: Box<Command>) -> Self {
            self.0.push(command);
            self
        }

        pub(crate) fn insert(mut self, command: Box<Command>) -> Self {
            self.0.insert(0, command);
            self
        }
    }
}

// Following with the skiing analogy, the code inside here is black level.
// Many of the issues in a grammar rule cause conflicts in seemingly unrelated
// rules. Some issues are known to be harder to solve, and while LALRPOP does
// a fantasic job of helping, it's not perfect. Avoid the rocks, trees, and
// enjoy.
//
// The code for this module is located in `src/program/posix.lalrpop`.
lalrpop_mod!(lalrpop, "/program/posix.rs");
