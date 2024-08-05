use crate::ast;
use log::*;
use std::sync::Mutex;

pub static FLOW_GRAPH: Mutex<Option<FlowGraph>> = Mutex::new(None);

#[derive(Debug)]
pub struct FlowGraph {
    pub timestamp: u64,
    pub ast: Mutex<Option<ast::TopDef>>,
}

impl FlowGraph {
    pub fn new(ast: Option<ast::TopDef>) -> Self {
        FlowGraph {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ast: Mutex::new(ast),
        }
    }
    pub fn generate(&self) {
        info!("Generating Graph...");
    }
}
