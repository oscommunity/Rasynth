use crate::ast;
use core::fmt;
use log::*;
use petgraph::dot::{Config, Dot};
use petgraph::graph::Graph;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::{borrow::BorrowMut, sync::Mutex};

pub static FLOW_GRAPH: Mutex<Option<FlowGraph>> = Mutex::new(None);

#[derive(Debug)]
pub struct Context {
    pub current_box: Option<Box<ModuleBox>>,
    pub current_box_op_suffix_cnt: HashMap<String, u64>,
}

#[derive(Debug, Clone)]
pub enum Constant {
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Float32Array(Vec<f32>),
    // Waveform(Waveform),
}

/// FlowGraph is the representation of the audio process flow
pub struct FlowGraph {
    pub timestamp: u64,
    pub ast: Mutex<Option<ast::TopDef>>,
    pub node_id_counter: u64,
    pub nodes: Vec<Box<Node>>,
    pub edge_id_counter: u64,
    pub edges: Vec<Box<Edge>>,
    pub boxes: Vec<Box<ModuleBox>>,
    pub ctx: Context,
    pub popped_nodes_hash: HashMap<Box<Node>, i32>,
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
    pub parent_box: Box<ModuleBox>,
    pub const_data: Option<Constant>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Node {}
impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone)]
pub struct Edge {
    pub id: u64,
    pub arg_no: u64,
    pub from: Box<Node>,
    pub to: Box<Node>,
}

/// corresponding to the box definition in grammar
#[derive(Debug, Clone)]
pub struct ModuleBox {
    pub name: String,
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
            ctx: Context {
                current_box: None,
                current_box_op_suffix_cnt: HashMap::new(),
            },
            popped_nodes_hash: HashMap::new(),
        }
    }
    pub fn dump_dot(&self) {
        let mut dot_graph = Graph::<String, u64>::new();
        for node in self.nodes.iter() {
            let mut dot_node_name = node.name.clone();
            // if is contant, append real value
            if node.const_data.is_some() {
                dot_node_name += format!("\n{:?}", node.const_data.as_ref().unwrap()).as_str();
            }
            dot_graph.add_node(dot_node_name);
        }
        for edge in self.edges.iter() {
            // prefix match
            let a = dot_graph
                .node_indices()
                .find(|ni| dot_graph[*ni].starts_with(&edge.from.name));
            let b = dot_graph
                .node_indices()
                .find(|ni| dot_graph[*ni].starts_with(&edge.to.name));
            let w = edge.arg_no;
            if a.is_some() && b.is_some() {
                dot_graph.add_edge(a.unwrap(), b.unwrap(), w);
            }
        }
        // println!("{:?}", Dot::with_config(&dot_graph, &[Config::EdgeNoLabel]));
        let mut f = File::create("flow.dot").unwrap();
        let output = format!("{}", Dot::new(&dot_graph));
        f.write_all(&output.as_bytes()).expect("write failed");
    }
    pub fn add_edge(&mut self, from: &mut Box<Node>, to: &mut Box<Node>, arg_no: u64) {
        let edge = Box::new(Edge {
            id: self.edge_id_counter,
            arg_no,
            from: from.clone(),
            to: to.clone(),
        });
        from.outputs.push(edge.clone());
        to.inputs.push(edge.clone());
        self.edges.push(edge.clone());
        self.edge_id_counter += 1;
    }
    pub fn new_node(&mut self, name: String, parent_box: Box<ModuleBox>) {
        let node = Box::new(Node {
            id: self.node_id_counter,
            name: parent_box.name.clone() + "/" + &name,
            inputs: Vec::new(),
            outputs: Vec::new(),
            parent_box: parent_box.clone(),
            const_data: None,
        });
        trace!("New Node: {:?}", node);
        self.nodes.push(node.clone());
        self.node_id_counter += 1;
    }
    pub fn pop_node_by_name(&mut self, name: &String) -> Box<Node> {
        debug!("pop_node_by_name: {}", name);
        let mut ret: Option<Box<Node>> = None;
        // name should be box_name/ident without @ suffix
        for node in self.nodes.iter() {
            // two senarios: with/without @
            // because @ is specifically used for operators
            if node.name.contains("@") {
                let raw_name = node.name.split("@").collect::<Vec<&str>>()[0];
                if raw_name == *name {
                    debug!("pop_node_by_name->raw_name: {}", raw_name);
                    if self.popped_nodes_hash.contains_key(node) {
                        debug!("pop_node_by_name->node already popped: {:?}", node);
                        continue;
                    }
                    self.popped_nodes_hash.insert(node.clone(), 1);
                    ret = Some(node.clone());
                    break;
                }
            } else {
                // just return the node with raw_name matched
                if node.name == *name {
                    self.popped_nodes_hash.insert(node.clone(), 1);
                    ret = Some(node.clone());
                    break;
                }
            }
        }
        if ret.is_none() {
            error!("Node not found: {}", name);
            panic!("Node not found: {}", name);
        }
        debug!("pop_node_by_name->ret: {:?}", ret);
        ret.unwrap()
    }
    pub fn node_create(&mut self, boxes: &Vec<ast::BoxDef>) {
        // first iteration, create all the nodes
        info!(">>> ITERATION 1: Creating Nodes...");
        for box_def in boxes {
            // create a new module box
            let module_box = ModuleBox {
                name: match box_def {
                    ast::BoxDef::ModuleBox(name, _, _) => name.clone(),
                },
            };

            // update ctx
            self.boxes.push(Box::new(module_box.clone()));
            self.ctx.current_box = Some(Box::new(module_box.clone()));
            self.ctx.current_box_op_suffix_cnt = HashMap::new();

            match box_def {
                ast::BoxDef::ModuleBox(name, ports, stmts) => {
                    debug!("Box: {}", name);
                    // for every in/out ports, create a node
                    for port in ports {
                        match port {
                            ast::Port::In(name, _) => {
                                debug!("InPort: {}", name);
                                self.new_node(name.clone(), self.ctx.current_box.clone().unwrap());
                            }
                            ast::Port::Out(name, _) => {
                                debug!("OutPort: {}", name);
                                self.new_node(name.clone(), self.ctx.current_box.clone().unwrap());
                            }
                        }
                    }
                    for stmt in stmts {
                        match stmt {
                            ast::Stmt::LetDef(let_def) => match let_def {
                                ast::LetDef::Let(name, expr) => {
                                    debug!("Let: {}", name);
                                    self.new_node(
                                        name.clone(),
                                        self.ctx.current_box.clone().unwrap(),
                                    );
                                    // (let x expr)
                                    // however, expr may be recursive, we need to add each
                                    // corresponding node from ast
                                    self.dfs_expr(expr);
                                }
                            },
                            ast::Stmt::BoxWire(box_wire) => match box_wire {
                                ast::BoxWire::Boxw(name, _exprs) => {
                                    debug!("BoxWire: {}", name);
                                    self.new_node(
                                        name.clone(),
                                        self.ctx.current_box.clone().unwrap(),
                                    );
                                }
                            },
                        }
                    }
                }
            }
        }

        info!(">>> ITERATION 2: Merging Nodes...");
        // second iteration, merge nodes with identical names
        // name: box_name/ident with optional suffix on operator
        // if the name is the same, we consider them as the same node
        // and merge them
        let mut node_map: std::collections::HashMap<String, Box<Node>> =
            std::collections::HashMap::new();
        for node in self.nodes.iter() {
            let name = node.name.clone();
            if node_map.contains_key(&name) {
                // merge the node
                let merged_node = node_map.get_mut(&name).unwrap();
                merged_node.inputs.append(&mut node.inputs.clone());
                merged_node.outputs.append(&mut node.outputs.clone());
            } else {
                node_map.insert(name.clone(), node.clone());
            }
        }
        // update the nodes and sort by name
        self.nodes = node_map.values().map(|v| v.clone()).collect();
        self.nodes.sort_by(|a, b| a.name.cmp(&b.name));
        info!("Nodes: {:?}", self.nodes);

        info!(">>> ITERATION 3: Creating Edges...");
        // third iteration, create edges
        for box_def in boxes {
            // update ctx
            self.ctx.current_box = Some(Box::new(ModuleBox {
                name: match box_def {
                    ast::BoxDef::ModuleBox(name, _, _) => name.clone(),
                },
            }));

            match box_def {
                ast::BoxDef::ModuleBox(_, _, stmts) => {
                    for stmt in stmts {
                        match stmt {
                            ast::Stmt::LetDef(let_def) => match let_def {
                                ast::LetDef::Let(name, expr) => {
                                    let cat_name =
                                        self.ctx.current_box.clone().unwrap().name.clone()
                                            + "/"
                                            + name;
                                    let mut node = self.pop_node_by_name(&cat_name);
                                    debug!("Let: popped node: {:?} for {}", node, cat_name);
                                    let mut nd = self.dfs_edge(expr, &node);
                                    self.add_edge(&mut nd, &mut node, 0);
                                }
                            },
                            _ => {
                                warn!("not implemented for stmt: {:?}", stmt);
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn get_nodes(&self) -> Vec<Box<Node>> {
        self.nodes.clone()
    }
    pub fn dfs_edge(&mut self, expr: &ast::Expr, parent: &Box<Node>) -> Box<Node> {
        let mut this_node: Option<Box<Node>> = None;
        match expr {
            ast::Expr::Num(x) => {
                // first get all const nodes
                for nd in self.get_nodes() {
                    if nd.const_data.is_some() {
                        // first check box name is the same
                        if nd.parent_box.name != self.ctx.current_box.clone().unwrap().name {
                            continue;
                        }
                        // check if the node's const_data is the same as x
                        match x {
                            ast::Numeric::Int32(val) => {
                                if let Constant::Int32(cval) = nd.const_data.as_ref().unwrap() {
                                    if cval == val {
                                        this_node = Some(nd.clone());
                                    }
                                }
                            }
                            ast::Numeric::Float(val) => {
                                if let Constant::Float32(cval) = nd.const_data.as_ref().unwrap() {
                                    if cval == val {
                                        this_node = Some(nd.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ast::Expr::NodeIdent(name) => {
                let cat_name = self.ctx.current_box.clone().unwrap().name.clone() + "/" + name;
                this_node = Some(self.pop_node_by_name(&cat_name));
            }
            ast::Expr::Operator(op, args) => {
                let cat_name = self.ctx.current_box.clone().unwrap().name.clone() + "/" + op;
                this_node = Some(self.pop_node_by_name(&cat_name));
                let mut this_node = this_node.clone().unwrap();
                let mut arg_no = 0;
                for arg in args {
                    let mut nd = self.dfs_edge(arg, &this_node);
                    self.add_edge(&mut nd, &mut this_node, arg_no);
                    arg_no += 1;
                }
            }
        }
        if this_node.is_none() {
            error!("Node not found for expr: {:?}", expr);
            panic!("Node not found for expr: {:?}", expr);
        }
        let this_node = this_node.unwrap();
        return this_node;
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
            ast::Expr::Num(x) => {
                // create a number node with name const@suff
                let cnt = self.ctx.current_box_op_suffix_cnt.get("const");
                if cnt.is_none() {
                    self.ctx
                        .current_box_op_suffix_cnt
                        .insert("const".to_string(), 0);
                }
                let name = format!("const@{}", self.ctx.current_box_op_suffix_cnt["const"]);
                self.ctx.current_box_op_suffix_cnt.insert(
                    "const".to_string(),
                    self.ctx.current_box_op_suffix_cnt["const"] + 1,
                );
                self.new_node(name, self.ctx.current_box.clone().unwrap());
                match x {
                    ast::Numeric::Int32(val) => {
                        self.nodes.last_mut().unwrap().const_data = Some(Constant::Int32(*val));
                    }
                    ast::Numeric::Float(val) => {
                        self.nodes.last_mut().unwrap().const_data = Some(Constant::Float32(*val));
                    }
                }
            }
            ast::Expr::Operator(op, args) => {
                // create an operator node
                let cnt = self.ctx.current_box_op_suffix_cnt.get(op);
                if cnt.is_none() {
                    self.ctx.current_box_op_suffix_cnt.insert(op.clone(), 0);
                }
                let name = format!("{}@{}", op, self.ctx.current_box_op_suffix_cnt[op]);
                self.ctx
                    .current_box_op_suffix_cnt
                    .insert(op.clone(), self.ctx.current_box_op_suffix_cnt[op] + 1);
                self.new_node(name, self.ctx.current_box.clone().unwrap());
                for arg in args {
                    self.dfs_expr(arg);
                }
            }
            ast::Expr::NodeIdent(name) => {
                // create a node ident node
                self.new_node(name.clone(), self.ctx.current_box.clone().unwrap());
            }
        }
    }
}

impl fmt::Debug for FlowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(flowgraph\n\t(timestamp: {}", self.timestamp)?;
        write!(f, "\n\tnodes: \n")?;
        for node in self.nodes.iter() {
            write!(f, "\t\t{:?}\n", node)?;
        }
        write!(f, "\n\tedges: \n")?;
        for edge in self.edges.iter() {
            write!(f, "\t\t{:?}\n", edge)?;
        }
        Ok(())
    }
}

impl fmt::Debug for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(edge (id:{},arg_no:{},from:{:?},to:{:?})",
            self.id, self.arg_no, self.from, self.to
        )
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(node (id:{},name:{}", self.id, self.name)?;
        if self.const_data.is_some() {
            write!(f, ",const_data:{:?}", self.const_data)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}
