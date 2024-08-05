use lalrpop_util::lalrpop_mod;
use std::fs;
pub mod ast;

lalrpop_mod!(pub raslisp); // synthesized by LALRPOP

fn main() {
    let test1 = fs::read_to_string("../test/test.raslisp").unwrap();
    let r = raslisp::BoxDefineParser::new().parse(&test1).unwrap();
    println!("{:?}", r);
}
