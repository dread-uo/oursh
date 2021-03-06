mod common;

#[test]
fn hello_world() {
    assert_piped_oursh!("echo hello world", "hello world\n");
}

#[test]
fn builtins() {
    assert_piped_oursh!(":");
    // assert_piped_oursh!("cd /; cd /home; cd -", "/\n");
    // assert_piped_oursh!("cd /; pwd", "/\n");
    // assert_piped_oursh!("cd a b");  // TODO: Check output status somehow.
    // assert_piped_oursh!("cd; pwd", "$HOME\n");
    assert_piped_oursh!("exit");
    assert_piped_oursh!(! "exit 1");
}

#[test]
#[ignore]
fn forkbomb() {
    assert_piped_oursh!(":(){ :|: & };:");
}

#[test]
#[ignore]
fn hello_world_quoted() {
    assert_piped_oursh!("echo \"hello world\"", "hello world\n");
}

#[test]
fn simple_command() {
    assert_piped_oursh!("head README.md -n 1", "# oursh\n");
}

#[test]
fn chained_command() {
    assert_piped_oursh!("false; true; echo 1", "1\n");
}

#[test]
fn single_compound_command() {
    assert_piped_oursh!("{ echo pi; }", "pi\n");
}

#[test]
fn multiple_compound_command() {
    assert_piped_oursh!("{ echo pi; echo e; }", "pi\ne\n");
}

#[test]
fn not_command() {
    // TODO: Test status of the `oursh` command too.
    // `assert_piped_oursh!(! "! true");`
    assert_piped_oursh!("! true && echo 1", "");
}

#[test]
fn and_command() {
    assert_piped_oursh!("true && echo 1", "1\n");
    assert_piped_oursh!("false && echo 1", "");
}

#[test]
fn or_command() {
    assert_piped_oursh!("true || echo 1", "");
    assert_piped_oursh!("false || echo 1", "1\n");
}

#[test]
#[ignore]
fn subshell_command() {
    assert_piped_oursh!("( true )");
    assert_piped_oursh!("(echo 1)", "1\n");
}

#[test]
fn single_pipeline_command() {
    assert_piped_oursh!("echo pi | wc -c", "3\n");
}

#[test]
#[ignore]
fn chained_pipeline_command() {
    assert_piped_oursh!("cat README.md | head | wc -l", "10\n");
}

#[test]
#[ignore]
fn background_command() {
    assert_piped_oursh!("sleep 1 & echo 1", "1\n");
    // TODO: I'm thinking the Job status should go to STDERR.
}

#[test]
#[cfg(feature = "bridge")]
fn bridged_sh_command() {
    assert_piped_oursh!("{#!/bin/sh; echo '1'}", "1\n");
    assert_piped_oursh!(r#"
{#!/bin/sh;
    for i in 1 2 3 4 5
    do
        echo -n $i
    done
}"#, "12345");
}

#[test]
#[cfg(feature = "bridge")]
fn bridged_ruby_command() {
    assert_piped_oursh!("{#!/usr/bin/env ruby; puts 1}", "1\n");
}

#[test]
#[cfg(feature = "bridge")]
fn bridged_python_command() {
    assert_piped_oursh!("{#!/usr/bin/env python; print(1)}", "1\n");
    assert_piped_oursh!("{#!/usr/bin/env python  ;    print(1)}", "1\n");
    assert_piped_oursh!(r#"
{#!/usr/bin/env python;
print("hello world")
}"#, "hello world\n");
}

#[test]
#[cfg(feature = "bridge")]
fn bridged_racket_command() {
    assert_piped_oursh!(r#"
{#!/usr/bin/env racket;
    #lang racket/base
    (print "hello world!")
}"#, "\"hello world!\"");
}

#[test]
#[ignore]
#[cfg(feature = "bridge")]
fn bridged_rust_command() {
    assert_piped_oursh!(r#"
{#!/usr/bin/env cargo-script-run;
    fn main() {
        println!("hello world!");
    }
}"#, "hello world!\n");
}
