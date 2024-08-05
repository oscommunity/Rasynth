#[derive(Debug, Clone)]
pub enum Expr {
    Operator(String, Vec<Expr>), // An operator with a list of arguments
    NodeIdent(String),           // A wire in box
    Num(Numeric),                // A numeric value
}

#[derive(Debug, Clone)]
pub enum Numeric {
    Int32(i32),
    Float(f32),
}

#[derive(Debug, Clone)]
pub enum LetDefine {
    Let(String, Expr),
}

#[derive(Debug, Clone)]
pub enum BoxDefine {
    Box(String, Vec<Port>, Vec<Statement>),
}

#[derive(Debug, Clone)]
pub enum Statement {
    LetDefine(LetDefine),
}

#[derive(Debug, Clone)]
pub enum Port {
    In(String, Type),
    Out(String, Type),
}

#[derive(Debug, Clone)]
pub enum Type {
    Int32,
    Float,
    Waveform,
}

impl Type {
    pub fn from_str(s: &str) -> Option<Type> {
        match s {
            "i32" => Some(Type::Int32),
            "float" => Some(Type::Float),
            "waveform" => Some(Type::Waveform),
            _ => None,
        }
    }
}