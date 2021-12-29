mod gc;
mod gc_obj;
mod runtime;
mod tests;
mod types;
mod parser;
mod ratio;
mod stdlib;

fn run_tests_in_main() {
    tests::test_basic_garbage_collection_manual_binding();
}

fn main() {
}
