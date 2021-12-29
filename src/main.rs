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

fn main() {}
