mod gc;
mod gc_obj;
mod parser;
mod ratio;
mod runtime;
mod stdlib;
mod tests;
mod types;
mod number;
mod instructions;

use types::*;
use parser::*;
use number::*;
use gc::*;
use gc_obj::*;
use runtime::*;
use std::io::{self, BufRead, Write};

fn run_tests_in_main() {
    tests::test_basic_garbage_collection_manual_binding();
}

fn run_main() -> RunRes<()> {
    let mut args = std::env::args();
    if args.len() > 2 {
        println!("Usage: {} <optional file>", args.nth(0).unwrap());
        return Ok(());
    }
    if args.len() == 2 {
        let file_name = args.nth(1).unwrap();
        let _res = run_file(&file_name)?;
    } else {
        let mut runtime = Runtime::new();
        let mut line = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        loop {
            print!("> ");
            stdout.flush()?;
            let read = stdin.lock().read_line(&mut line)?;
            if read == 0 {
                break;
            }
            runtime.run_str(&line)?;
        }
    }
    Ok(())
}

fn main() -> RunRes<()> {
    run_main()
}
