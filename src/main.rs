mod gc;
mod gc_obj;
mod parser;
mod ratio;
mod runtime;
mod stdlib;
mod tests;
mod types;
mod take_obj;

use types::RunRes;

fn run_tests_in_main() {
    tests::test_basic_garbage_collection_manual_binding();
}

fn main() -> RunRes<()> {
    use crate::runtime::run_file;
    let mut args = std::env::args();
    if args.len() != 2 {
        println!("Usage: {} <file>", args.nth(0).unwrap());
        return Ok(());
    }
    let file_name = args.nth(1).unwrap();
    let _res = run_file(&file_name)?;
    Ok(())
}
