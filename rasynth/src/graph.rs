use crate::ast;
use crate::symbol_table::SymbolTable;
use core::fmt;
use log::*;
use std::{borrow::BorrowMut, sync::Mutex};

pub static FLOW_GRAPH: Mutex<Option<FlowGraph>> = Mutex::new(None);

/// FlowGraph is the representation of the audio process flow
pub struct FlowGraph {
    pub timestamp: u64,
    pub ast: Mutex<Option<ast::TopDef>>,
    pub node_id_counter: u64,
    pub nodes: Vec<Box<Node>>,
    pub edge_id_counter: u64,
    pub edges: Vec<Box<Edge>>,
    pub boxes: Vec<ModuleBox>,
}

/// each node on ast like in/out port, intermediate node will
/// be represented as an actual process node in the graph
/// illustration of (+ in1 in2)
///     [in1]      [in2]
///       | oprd1   | oprd2
///        \       /
///           [+]
/// the graph uses the directed graph edge, and each node's
/// input edge should be marked with operand number, and
/// the order should match that one in ast arg vec
#[derive(Clone)]
pub struct Node {
    pub id: u64,
    pub name: String,
    pub inputs: Vec<Box<Edge>>,
    pub outputs: Vec<Box<Edge>>,
}

#[derive(Clone)]
pub struct Edge {
    pub id: u64,
    pub no_of_arg: u64,
    pub from: Box<Node>,
    pub to: Box<Node>,
}

/// corresponding to the box definition in grammar
pub struct ModuleBox {
    pub name: String,
    pub st: SymbolTable,
}

impl FlowGraph {
    pub fn new(ast: Option<ast::TopDef>) -> Self {
        FlowGraph {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ast: Mutex::new(ast),
            node_id_counter: 0,
            nodes: Vec::new(),
            edge_id_counter: 0,
            edges: Vec::new(),
            boxes: Vec::new(),
        }
    }
    pub fn add_edge(&mut self, from: &mut Node, to: &mut Node, no_of_arg: u64) {
        let edge = Box::new(Edge {
            id: self.edge_id_counter,
            no_of_arg,
            from: Box::new(from.clone()),
            to: Box::new(to.clone()),
        });
        from.outputs.push(edge.clone());
        to.inputs.push(edge.clone());
        self.edges.push(edge.clone());
    }
    pub fn new_node(&mut self, name: String) {
        let node = Box::new(Node {
            id: self.node_id_counter,
            name,
            inputs: Vec::new(),
            outputs: Vec::new(),
        });
        trace!("New Node: {:?}", node);
        self.nodes.push(node);
        self.node_id_counter += 1;
    }
    pub fn node_create(&mut self, boxes: &Vec<ast::BoxDef>) {
        for box_def in boxes {
            // create a new module box
            let module_box = ModuleBox {
                name: match box_def {
                    ast::BoxDef::ModuleBox(name, _, _) => name.clone(),
                },
                st: SymbolTable {
                    table: std::collections::HashMap::new(),
                },
            };
            self.boxes.push(module_box);
            match box_def {
                ast::BoxDef::ModuleBox(name, ports, stmts) => {
                    debug!("Box: {}", name);
                    // for every in/out ports, create a node
                    for port in ports {
                        match port {
                            ast::Port::In(name, _) => {
                                debug!("InPort: {}", name);
                                self.new_node(name.clone());
                            }
                            ast::Port::Out(name, _) => {
                                debug!("OutPort: {}", name);
                                self.new_node(name.clone());
                            }
                        }
                    }
                    for stmt in stmts {
                        match stmt {
                            ast::Stmt::LetDef(let_def) => match let_def {
                                ast::LetDef::Let(name, expr) => {
                                    debug!("Let: {}", name);
                                    self.new_node(name.clone());
                                    // (let x expr)
                                    // however, expr may be recursive, we need to add each
                                    // corresponding node from ast
                                    self.dfs_expr(expr);
                                }
                            },
                            ast::Stmt::BoxWire(box_wire) => match box_wire {
                                ast::BoxWire::Boxw(name, exprs) => {
                                    debug!("BoxWire: {}", name);
                                    self.new_node(name.clone());
                                }
                            },
                        }
                    }
                }
            }
        }
    }
    pub fn generate(&mut self) -> Vec<ast::BoxDef> {
        info!("Generating Graph...");
        // get ast from mutable reference self
        let ast = self.borrow_mut().ast.lock().unwrap();
        debug!("AST: {:?}", ast.as_ref().unwrap());
        // Top will consists of a vec of ModuleBox
        let boxes = match ast.as_ref().unwrap() {
            ast::TopDef::Boxes(boxes) => boxes,
        };
        boxes.clone()
    }
    fn dfs_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Num(numeral) => {
                // create a number node
                self.new_node(format!(
                    "CONSTANT:{}",
                    match numeral.clone() {
                        ast::Numeric::Int32(i) => i.to_string(),
                        ast::Numeric::Float(f) => f.to_string(),
                    }
                ));
            }
            ast::Expr::Operator(op, args) => {
                // create an operator node
                self.new_node(op.clone());
                for arg in args {
                    self.dfs_expr(arg);
                }
            }
            ast::Expr::NodeIdent(name) => {
                // create a node ident node
                self.new_node(name.clone());
            }
        }
    }
}

impl fmt::Debug for FlowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(flowgraph\n\t(timestamp: {},\n\tnodes: {:?},\n\tedges: {:?})",
            self.timestamp, self.nodes, self.edges
        )
    }
}

impl fmt::Debug for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(edge (id:{},no_of_arg:{},from:{:?},to:{:?})",
            self.id, self.no_of_arg, self.from, self.to
        )
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(node (id:{},name:{})", self.id, self.name)
    }
}
