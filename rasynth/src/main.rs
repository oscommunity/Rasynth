use env_logger::Env;
use lalrpop_util::lalrpop_mod;
use log::*;
use std::fs;

pub mod ast;
pub mod graph;
pub mod symbol_table;

lalrpop_mod!(pub raslisp); // synthesized by LALRPOP

fn main() {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    info!("RASLISP Interpreter, version {}", env!("CARGO_PKG_VERSION"));
    info!("Author: {}", env!("CARGO_PKG_AUTHORS"));

    let input_file = "../test/osc1.raslisp";
    info!("Input Top File Path: {}", input_file);
    let test1 = fs::read_to_string(input_file).expect("Unable to read file");

    let top = raslisp::TopParser::new().parse(&test1).unwrap();
    info!("AST Parsed Successfully!");

    graph::FLOW_GRAPH
        .lock()
        .unwrap()
        .replace(graph::FlowGraph::new(Some(top)));

    let boxes = graph::FLOW_GRAPH
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .generate();

    graph::FLOW_GRAPH
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .node_create(&boxes);

    info!("Graph: {:?}", graph::FLOW_GRAPH.lock().unwrap());

    graph::FLOW_GRAPH
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .dump_dot();

    info!("Goodbye!");
}
