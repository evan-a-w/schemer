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

fn run_tests_in_main() {
    tests::test_basic_garbage_collection_manual_binding();
}

fn run_file_main() -> RunRes<()> {
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

fn main() -> RunRes<()> {
    let parsed = pongascript_parser("
    (foldl cons '() '(1 2 3 4 5))
     (define (foldl func accu alist)
       (if (null? alist)
         accu
         (foldl func (func (car alist) accu) (cdr alist))))

     (define i (foldl cons '() '(1 2 3 4 5)))
     (display i)
     (equal? i '(5 4 3 2 1))
    ")
    .unwrap();
    let mut runtime = Runtime::new();
    let evald = parsed
        .1
        .into_iter()
        .map(|x| runtime.eval(x))
        .collect::<Vec<RunRes<Ponga>>>();
    println!("{:?}", evald);
    Ok(())
}
