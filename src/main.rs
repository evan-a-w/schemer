mod gc;
mod gc_obj;
mod parser;
mod ratio;
mod runtime;
mod stdlib;
mod tests;
mod types;

fn run_tests_in_main() {
    tests::test_basic_garbage_collection_manual_binding();
}

fn main() {
    use crate::runtime::run_file;
    let mut args = std::env::args();
    if args.len() != 2 {
        println!("Usage: {} <file>", args.nth(0).unwrap());
        return;
    }
    let file_name = args.nth(1).unwrap();
    let res = run_file(&file_name);
    println!("{:?}", res);
}
