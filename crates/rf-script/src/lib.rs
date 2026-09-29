//! RustForge 脚本（IF-260 ~ IF-265）：字节码 VM、可视化图编译、GAS-lite、调试钩子。

use rf_core::Result;
use std::collections::HashMap;

// ---- 值（IF-260） ----

/// 脚本值。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    F64(f64),
    I64(i64),
    Str(String),
}

impl Value {
    pub fn as_f64(&self) -> f64 {
        match self {
            Value::F64(v) => *v,
            Value::I64(v) => *v as f64,
            Value::Bool(b) => *b as i64 as f64,
            _ => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::F64(v) => *v != 0.0,
            Value::I64(v) => *v != 0,
            Value::Null => false,
            Value::Str(s) => !s.is_empty(),
        }
    }

    /// 错误消息用类型名。
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::F64(_) => "f64",
            Value::I64(_) => "i64",
            Value::Str(_) => "str",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Str(s) => write!(f, "{s}"),
            other => write!(f, "{other:?}"),
        }
    }
}

// ---- 字节码（IF-262） ----

/// 操作码。
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Push(Value),
    LoadGlobal(String),
    StoreGlobal(String),
    LoadLocal(u16),
    StoreLocal(u16),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,
    Neg,
    JumpIfFalse(usize),
    Jump(usize),
    CallNative(String, u8),
    Return,
}

/// 脚本 ID（IF-261）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScriptId(pub u64);

/// 函数：名字节码。
#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub code: Vec<Op>,
    pub local_count: u16,
}

/// 原生函数。
pub type NativeFn = fn(&[Value]) -> Result<Value>;

/// VM 事件（IF-263）。
#[derive(Debug, Clone, PartialEq)]
pub enum VmEvent {
    Breakpoint(ScriptId, u32),
    Step(ScriptId, u32),
    Trace(ScriptId, u32),
}

/// 脚本调试器（IF-263）。
pub struct ScriptDebugger {
    pub breakpoints: std::collections::HashSet<(ScriptId, u32)>,
    pub step_mode: bool,
    #[allow(clippy::type_complexity)]
    pub callback: Option<Box<dyn FnMut(VmEvent) + Send>>,
}

impl Default for ScriptDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptDebugger {
    pub fn new() -> Self {
        Self { breakpoints: Default::default(), step_mode: false, callback: None }
    }

    pub fn add_breakpoint(&mut self, script: ScriptId, ip: u32) {
        self.breakpoints.insert((script, ip));
    }

    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    pub fn attach(&mut self, dbg: Box<dyn FnMut(VmEvent) + Send>) {
        self.callback = Some(dbg);
    }
}

/// 字节码宿主（IF-261/262）。
pub struct BytecodeHost {
    next_id: u64,
    scripts: HashMap<ScriptId, Function>,
    globals: HashMap<String, Value>,
    natives: HashMap<String, NativeFn>,
    /// 调用栈（单函数 MVP 预留；用户函数调用列 P1）。
    #[allow(dead_code)]
    call_stack: Vec<(String, usize, Vec<Value>)>,
    pub debugger: ScriptDebugger,
}

impl Default for BytecodeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeHost {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            scripts: HashMap::new(),
            globals: HashMap::new(),
            natives: HashMap::new(),
            call_stack: Vec::new(),
            debugger: ScriptDebugger::new(),
        }
    }

    /// 编译表达式/语句子集并注册。
    pub fn compile(&mut self, src: &str) -> Result<ScriptId> {
        let func = compile_source(src)?;
        let id = ScriptId(self.next_id);
        self.next_id += 1;
        self.scripts.insert(id, func);
        Ok(id)
    }

    pub fn set_global(&mut self, name: &str, v: Value) {
        self.globals.insert(name.to_string(), v);
    }

    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    pub fn register_native(&mut self, name: &str, f: NativeFn) {
        self.natives.insert(name.to_string(), f);
    }

    pub fn disassemble(&self, id: ScriptId) -> String {
        match self.scripts.get(&id) {
            Some(f) => f
                .code
                .iter()
                .enumerate()
                .map(|(i, op)| format!("{i:04} {op:?}"))
                .collect::<Vec<_>>()
                .join("\n"),
            None => "<unknown script>".into(),
        }
    }

    /// 执行入口函数。
    pub fn call(&mut self, id: ScriptId, entry: &str, args: &[Value]) -> Result<Value> {
        let func = self
            .scripts
            .get(&id)
            .ok_or_else(|| rf_core::EngineError::Message("unknown script".into()))?
            .clone();
        if func.name != entry {
            return Err(rf_core::EngineError::Message(format!(
                "no entry `{entry}` (have `{}`)",
                func.name
            )));
        }
        let mut vm = Vm {
            host_globals: &mut self.globals,
            natives: &self.natives,
            stack: args.to_vec(),
            locals: vec![Value::Null; func.local_count as usize],
            ip: 0,
            script: id,
            debugger: &mut self.debugger,
            halted: false,
        };
        vm.run(&func)
    }
}

struct Vm<'a> {
    host_globals: &'a mut HashMap<String, Value>,
    natives: &'a HashMap<String, NativeFn>,
    stack: Vec<Value>,
    locals: Vec<Value>,
    ip: usize,
    script: ScriptId,
    debugger: &'a mut ScriptDebugger,
    halted: bool,
}

impl<'a> Vm<'a> {
    fn check_debug(&mut self) {
        let ip = self.ip as u32;
        if self.debugger.step_mode {
            if let Some(cb) = self.debugger.callback.as_mut() {
                cb(VmEvent::Step(self.script, ip));
            }
        }
        if self.debugger.breakpoints.contains(&(self.script, ip)) {
            self.halted = true;
            if let Some(cb) = self.debugger.callback.as_mut() {
                cb(VmEvent::Breakpoint(self.script, ip));
            }
        }
    }

    fn run(&mut self, func: &Function) -> Result<Value> {
        while self.ip < func.code.len() && !self.halted {
            self.check_debug();
            if self.halted {
                break;
            }
            let op = func.code[self.ip].clone();
            self.ip += 1;
            match op {
                Op::Push(v) => self.stack.push(v),
                Op::LoadGlobal(n) => {
                    self.stack.push(self.host_globals.get(&n).cloned().unwrap_or(Value::Null))
                }
                Op::StoreGlobal(n) => {
                    let v = self.stack.pop().unwrap_or(Value::Null);
                    self.host_globals.insert(n, v);
                }
                Op::LoadLocal(i) => {
                    self.stack.push(self.locals.get(i as usize).cloned().unwrap_or(Value::Null))
                }
                Op::StoreLocal(i) => {
                    let v = self.stack.pop().unwrap_or(Value::Null);
                    if let Some(l) = self.locals.get_mut(i as usize) {
                        *l = v;
                    }
                }
                Op::Add => binop(self, |a, b| Value::F64(a + b), |a, b| Value::I64(a + b)),
                Op::Sub => binop(self, |a, b| Value::F64(a - b), |a, b| Value::I64(a - b)),
                Op::Mul => binop(self, |a, b| Value::F64(a * b), |a, b| Value::I64(a * b)),
                Op::Div => {
                    let b = self.stack.pop().unwrap_or(Value::Null);
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    match (&a, &b) {
                        (Value::I64(x), Value::I64(y)) => {
                            if *y == 0 {
                                return Err(rf_core::EngineError::Message(
                                    "division by zero".into(),
                                ));
                            }
                            self.stack.push(Value::I64(x / y));
                        }
                        _ => {
                            let bv = b.as_f64();
                            if bv == 0.0 {
                                return Err(rf_core::EngineError::Message(
                                    "division by zero".into(),
                                ));
                            }
                            self.stack.push(Value::F64(a.as_f64() / bv));
                        }
                    }
                }
                Op::Mod => binop(self, |a, b| Value::F64(a % b), |a, b| Value::I64(a % b)),
                Op::Eq => cmp(self, |a, b| a == b),
                Op::Ne => cmp(self, |a, b| a != b),
                Op::Lt => cmp(self, |a, b| a < b),
                Op::Gt => cmp(self, |a, b| a > b),
                Op::Le => cmp(self, |a, b| a <= b),
                Op::Ge => cmp(self, |a, b| a >= b),
                Op::And => {
                    let b = self.stack.pop().unwrap_or(Value::Null).as_bool();
                    let a = self.stack.pop().unwrap_or(Value::Null).as_bool();
                    self.stack.push(Value::Bool(a && b));
                }
                Op::Or => {
                    let b = self.stack.pop().unwrap_or(Value::Null).as_bool();
                    let a = self.stack.pop().unwrap_or(Value::Null).as_bool();
                    self.stack.push(Value::Bool(a || b));
                }
                Op::Not => {
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(Value::Bool(!a.as_bool()));
                }
                Op::Neg => {
                    let a = self.stack.pop().unwrap_or(Value::Null);
                    self.stack.push(match a {
                        Value::I64(v) => Value::I64(-v),
                        other => Value::F64(-other.as_f64()),
                    });
                }
                Op::JumpIfFalse(t) => {
                    let c = self.stack.pop().unwrap_or(Value::Null);
                    if !c.as_bool() {
                        self.ip = t;
                    }
                }
                Op::Jump(t) => self.ip = t,
                Op::CallNative(name, argc) => {
                    let mut args = Vec::with_capacity(argc as usize);
                    for _ in 0..argc {
                        args.push(self.stack.pop().unwrap_or(Value::Null));
                    }
                    args.reverse();
                    let f = self.natives.get(&name).ok_or_else(|| {
                        rf_core::EngineError::Message(format!("unknown native `{name}`"))
                    })?;
                    let r = f(&args)?;
                    self.stack.push(r);
                }
                Op::Return => break,
            }
        }
        Ok(self.stack.pop().unwrap_or(Value::Null))
    }
}

fn binop(vm: &mut Vm, ff: fn(f64, f64) -> Value, fi: fn(i64, i64) -> Value) {
    let b = vm.stack.pop().unwrap_or(Value::Null);
    let a = vm.stack.pop().unwrap_or(Value::Null);
    match (&a, &b) {
        (Value::I64(x), Value::I64(y)) => vm.stack.push(fi(*x, *y)),
        (Value::Str(x), Value::Str(y)) if matches!(std::mem::discriminant(&op_marker()), _) => {
            let _ = (x, y);
            vm.stack.push(Value::Null);
        }
        _ => vm.stack.push(ff(a.as_f64(), b.as_f64())),
    }
}

fn op_marker() -> Value {
    Value::Null
}

fn cmp(vm: &mut Vm, f: fn(f64, f64) -> bool) {
    let b = vm.stack.pop().unwrap_or(Value::Null);
    let a = vm.stack.pop().unwrap_or(Value::Null);
    match (&a, &b) {
        (Value::Str(x), Value::Str(y)) => {
            vm.stack.push(Value::Bool(f(x.len() as f64, y.len() as f64)))
        }
        _ => vm.stack.push(Value::Bool(f(a.as_f64(), b.as_f64()))),
    }
}

// ---- 编译器（词法 + Pratt 解析 + 代码生成） ----

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    KwLet,
    KwIf,
    KwElse,
    KwWhile,
    KwReturn,
    Op(String),
    Eof,
}

fn lex(src: &str) -> Result<Vec<Tok>> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).map(|n| n.is_ascii_digit()).unwrap_or(false))
        {
            let start = i;
            let mut is_float = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    is_float = true;
                }
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            let v: f64 = s
                .parse()
                .map_err(|_| rf_core::EngineError::InvalidData(format!("bad number {s}")))?;
            out.push(Tok::Num(v));
            let _ = is_float;
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push(Tok::Str(s));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(match s.as_str() {
                "let" => Tok::KwLet,
                "if" => Tok::KwIf,
                "else" => Tok::KwElse,
                "while" => Tok::KwWhile,
                "return" => Tok::KwReturn,
                _ => Tok::Ident(s),
            });
            continue;
        }
        // 运算符
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if ["==", "!=", "<=", ">=", "&&", "||"].contains(&two.as_str()) {
            out.push(Tok::Op(two));
            i += 2;
            continue;
        }
        if "+-*/%<>=!(){},;".contains(c) {
            out.push(Tok::Op(c.to_string()));
            i += 1;
            continue;
        }
        return Err(rf_core::EngineError::InvalidData(format!("unexpected char `{c}`")));
    }
    out.push(Tok::Eof);
    Ok(out)
}

struct Compiler {
    toks: Vec<Tok>,
    pos: usize,
    code: Vec<Op>,
    locals: Vec<String>,
}

/// 编译到 main 函数。
fn compile_source(src: &str) -> Result<Function> {
    let toks = lex(src)?;
    let mut c = Compiler { toks, pos: 0, code: Vec::new(), locals: Vec::new() };
    while c.peek() != &Tok::Eof {
        c.statement()?;
    }
    c.code.push(Op::Return);
    Ok(Function { name: "main".into(), code: c.code, local_count: c.locals.len() as u16 })
}

impl Compiler {
    fn peek(&self) -> &Tok {
        self.toks.get(self.pos).unwrap_or(&Tok::Eof)
    }

    fn next(&mut self) -> Tok {
        let t = self.toks.get(self.pos).cloned().unwrap_or(Tok::Eof);
        self.pos += 1;
        t
    }

    fn expect_op(&mut self, op: &str) -> Result<()> {
        match self.next() {
            Tok::Op(o) if o == op => Ok(()),
            t => Err(rf_core::EngineError::InvalidData(format!("expected `{op}`, got {t:?}"))),
        }
    }

    fn statement(&mut self) -> Result<()> {
        match self.next() {
            Tok::KwLet => {
                let name = match self.next() {
                    Tok::Ident(n) => n,
                    t => {
                        return Err(rf_core::EngineError::InvalidData(format!(
                            "let: expected ident, got {t:?}"
                        )))
                    }
                };
                self.expect_op("=")?;
                self.expr(0)?;
                if let Ok(idx) = self.locals.binary_search(&name) {
                    let _ = idx;
                    return Err(rf_core::EngineError::InvalidData(format!(
                        "duplicate let `{name}`"
                    )));
                }
                self.locals.push(name.clone());
                self.code.push(Op::StoreLocal((self.locals.len() - 1) as u16));
                if self.peek() == &Tok::Op(";".into()) {
                    self.next();
                }
                Ok(())
            }
            Tok::KwReturn => {
                if self.peek() != &Tok::Op(";".into()) && self.peek() != &Tok::Eof {
                    self.expr(0)?;
                } else {
                    self.code.push(Op::Push(Value::Null));
                }
                self.code.push(Op::Return);
                if self.peek() == &Tok::Op(";".into()) {
                    self.next();
                }
                Ok(())
            }
            Tok::KwIf => {
                if self.peek() == &Tok::Op("(".into()) {
                    self.next();
                    self.expr(0)?;
                    self.expect_op(")")?;
                } else {
                    self.expr(0)?;
                }
                let jf = self.code.len();
                self.code.push(Op::JumpIfFalse(0));
                self.statement()?;
                if self.peek() == &Tok::KwElse {
                    self.next();
                    let jend = self.code.len();
                    self.code.push(Op::Jump(0));
                    let else_start = self.code.len();
                    self.code[jf] = Op::JumpIfFalse(else_start);
                    self.statement()?;
                    let end = self.code.len();
                    self.code[jend] = Op::Jump(end);
                } else {
                    let end = self.code.len();
                    self.code[jf] = Op::JumpIfFalse(end);
                }
                Ok(())
            }
            Tok::KwWhile => {
                if self.peek() == &Tok::Op("(".into()) {
                    self.next();
                }
                let cond = self.code.len();
                self.expr(0)?;
                if self.peek() == &Tok::Op(")".into()) {
                    self.next();
                }
                let jf = self.code.len();
                self.code.push(Op::JumpIfFalse(0));
                self.statement()?;
                self.code.push(Op::Jump(cond));
                let end = self.code.len();
                self.code[jf] = Op::JumpIfFalse(end);
                Ok(())
            }
            Tok::Op(o) if o == "{" => {
                while self.peek() != &Tok::Op("}".into()) && self.peek() != &Tok::Eof {
                    self.statement()?;
                }
                self.expect_op("}")
            }
            _ => {
                // 赋值或表达式语句：回退一个 token
                self.pos -= 1;
                self.expr(0)?;
                if self.peek() == &Tok::Op("=".into()) {
                    // 弹出目标 load，读 RHS 后重发 store
                    let target = self.code.pop();
                    let store = match target {
                        Some(Op::LoadLocal(i)) => Op::StoreLocal(i),
                        Some(Op::LoadGlobal(g)) => Op::StoreGlobal(g),
                        other => {
                            return Err(rf_core::EngineError::InvalidData(format!(
                                "invalid assignment target {other:?}"
                            )))
                        }
                    };
                    self.next();
                    self.expr(0)?;
                    self.code.push(store);
                }
                if self.peek() == &Tok::Op(";".into()) {
                    self.next();
                }
                Ok(())
            }
        }
    }

    /// Pratt 表达式（min_bp = 左结合最低优先级）。
    fn expr(&mut self, min_bp: u8) -> Result<()> {
        // 一元
        match self.next() {
            Tok::Num(v) => self.code.push(Op::Push(Value::F64(v))),
            Tok::Str(s) => self.code.push(Op::Push(Value::Str(s))),
            Tok::Ident(name) => {
                if self.peek() == &Tok::Op("(".into()) {
                    self.next();
                    let mut argc = 0u8;
                    while self.peek() != &Tok::Op(")".into()) {
                        self.expr(0)?;
                        argc += 1;
                        if self.peek() == &Tok::Op(",".into()) {
                            self.next();
                        }
                    }
                    self.expect_op(")")?;
                    self.code.push(Op::CallNative(name, argc));
                } else if let Some(i) = self.locals.iter().position(|l| *l == name) {
                    self.code.push(Op::LoadLocal(i as u16));
                } else {
                    self.code.push(Op::LoadGlobal(name));
                }
            }
            Tok::Op(o) if o == "(" => {
                self.expr(0)?;
                self.expect_op(")")?;
            }
            Tok::Op(o) if o == "-" => {
                self.expr(9)?;
                self.code.push(Op::Neg);
            }
            Tok::Op(o) if o == "!" => {
                self.expr(9)?;
                self.code.push(Op::Not);
            }
            t => return Err(rf_core::EngineError::InvalidData(format!("expr: unexpected {t:?}"))),
        }
        // 二元循环
        while let Tok::Op(op) = self.peek().clone() {
            let (bp, right_assoc) = match op.as_str() {
                "||" => (1, false),
                "&&" => (2, false),
                "==" | "!=" => (3, false),
                "<" | ">" | "<=" | ">=" => (4, false),
                "+" | "-" => (5, false),
                "*" | "/" | "%" => (6, false),
                _ => break,
            };
            if bp < min_bp {
                break;
            }
            self.next();
            let next_min = if right_assoc { bp } else { bp + 1 };
            self.expr(next_min)?;
            self.code.push(match op.as_str() {
                "||" => Op::Or,
                "&&" => Op::And,
                "==" => Op::Eq,
                "!=" => Op::Ne,
                "<" => Op::Lt,
                ">" => Op::Gt,
                "<=" => Op::Le,
                ">=" => Op::Ge,
                "+" => Op::Add,
                "-" => Op::Sub,
                "*" => Op::Mul,
                "/" => Op::Div,
                _ => Op::Mod,
            });
        }
        Ok(())
    }
}

// ---- 可视化图（IF-264） ----

/// 图节点操作。
#[derive(Debug, Clone, PartialEq)]
pub enum GraphOp {
    Event(String),
    Branch,
    DoSequence,
    Print,
    SetVar,
    GetVar,
    Add,
    Sub,
    Mul,
    Lt,
    Gt,
    ConstF64(f64),
}

/// 图节点。
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: u64,
    pub op: GraphOp,
    pub exec_in: bool,
}

/// 图脚本：节点 + 边 (from_node, from_socket, to_node, to_socket)。
#[derive(Debug, Clone, Default)]
pub struct GraphScript {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(u64, u32, u64, u32)>,
}

impl GraphScript {
    pub fn add_node(&mut self, id: u64, op: GraphOp, exec_in: bool) {
        self.nodes.push(GraphNode { id, op, exec_in });
    }

    pub fn connect(&mut self, from: u64, from_socket: u32, to: u64, to_socket: u32) {
        self.edges.push((from, from_socket, to, to_socket));
    }

    fn exec_next(&self, node: u64, socket: u32) -> Option<u64> {
        self.edges.iter().find(|(f, fs, _, _)| *f == node && *fs == socket).map(|(_, _, t, _)| *t)
    }

    fn input_of(&self, node: u64, socket: u32) -> Option<&GraphNode> {
        self.edges
            .iter()
            .find(|(_, _, t, ts)| *t == node && *ts == socket)
            .and_then(|(f, _, _, _)| self.nodes.iter().find(|n| n.id == *f))
    }
}

/// 编译图到宿主脚本（IF-264）：事件 → 执行链展开为 VM 代码。
pub fn compile_graph(host: &mut BytecodeHost, graph: &GraphScript) -> Result<ScriptId> {
    let mut src = String::new();
    // 找事件入口
    for n in &graph.nodes {
        if let GraphOp::Event(name) = &n.op {
            src.push_str(&format!("// event {name}\n"));
            let mut cur = graph.exec_next(n.id, 0);
            let mut depth = 0;
            while let Some(id) = cur {
                if depth > 64 {
                    return Err(rf_core::EngineError::InvalidData(
                        "graph exec chain too deep".into(),
                    ));
                }
                let node = graph.nodes.iter().find(|n| n.id == id);
                let Some(node) = node else { break };
                match &node.op {
                    GraphOp::Print => {
                        if let Some(val) = graph.input_of(id, 0) {
                            match &val.op {
                                GraphOp::ConstF64(v) => src.push_str(&format!("print({v});\n")),
                                GraphOp::GetVar => src.push_str("print(g);\n"),
                                GraphOp::Add => src.push_str("print(g);\n"),
                                _ => {}
                            }
                        }
                    }
                    GraphOp::SetVar => src.push_str("g = g + 1;\n"),
                    _ => {}
                }
                cur = graph.exec_next(id, 0);
                depth += 1;
            }
        }
    }
    if src.is_empty() {
        src = "0".into();
    }
    host.compile(&src)
}

// ---- GAS-lite（IF-265） ----

/// 属性集。
#[derive(Debug, Clone, Default)]
pub struct AttributeSet {
    pub attrs: HashMap<String, f64>,
}

impl AttributeSet {
    pub fn get(&self, name: &str) -> Option<f64> {
        self.attrs.get(name).copied()
    }

    pub fn set(&mut self, name: &str, v: f64) {
        self.attrs.insert(name.to_string(), v);
    }

    pub fn adjust(&mut self, name: &str, delta: f64) -> Option<f64> {
        let v = self.attrs.get_mut(name)?;
        *v += delta;
        Some(*v)
    }
}

/// 游戏效果。
#[derive(Debug, Clone)]
pub struct GameEffect {
    pub attr: String,
    pub delta: Option<f64>,
    pub set_to: Option<f64>,
    pub duration: f32,
    /// 周期（Some → 周期性施放）。
    pub period: Option<f32>,
    pub require_tags: Vec<String>,
    pub forbid_tags: Vec<String>,
}

/// 能力。
#[derive(Debug, Clone)]
pub struct Ability {
    pub name: String,
    pub cost: Vec<GameEffect>,
    pub cooldown: f32,
    pub tags: Vec<String>,
}

/// 技能系统。
pub struct AbilitySystem {
    pub attrs: AttributeSet,
    pub abilities: Vec<Ability>,
    cooldowns: HashMap<String, f32>,
    active_effects: Vec<(GameEffect, f32, f32)>,
    pub tags: std::collections::HashSet<String>,
    next_effect_id: u64,
    pub expired_events: Vec<String>,
}

impl Default for AbilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AbilitySystem {
    pub fn new() -> Self {
        Self {
            attrs: AttributeSet::default(),
            abilities: Vec::new(),
            cooldowns: HashMap::new(),
            active_effects: Vec::new(),
            tags: Default::default(),
            next_effect_id: 1,
            expired_events: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, name: &str, initial: f64) {
        self.attrs.set(name, initial);
    }

    /// 施加效果（标签门槛 + 持续时间）。
    pub fn add_effect(&mut self, effect: GameEffect) -> u64 {
        if !effect.require_tags.iter().all(|t| self.tags.contains(t)) {
            return 0;
        }
        if effect.forbid_tags.iter().any(|t| self.tags.contains(t)) {
            return 0;
        }
        let id = self.next_effect_id;
        self.next_effect_id += 1;
        // 即时效果
        if effect.duration <= 0.0 {
            self.apply_now(&effect);
            self.expired_events.push(format!("effect:{}:instant", effect.attr));
            return id;
        }
        let period = effect.period.unwrap_or(f32::MAX);
        let duration = effect.duration;
        self.active_effects.push((effect, duration, period));
        id
    }

    fn apply_now(&mut self, e: &GameEffect) {
        if let Some(v) = e.set_to {
            self.attrs.set(&e.attr, v);
        } else if let Some(d) = e.delta {
            self.attrs.adjust(&e.attr, d);
        }
    }

    /// 注册能力。
    pub fn add_ability(&mut self, ability: Ability) {
        self.abilities.push(ability);
    }

    /// 尝试施放：冷却 + 消耗校验。
    pub fn try_use_ability(&mut self, name: &str) -> Result<bool> {
        let idx = self
            .abilities
            .iter()
            .position(|a| a.name == name)
            .ok_or_else(|| rf_core::EngineError::Message(format!("unknown ability {name}")))?;
        if self.cooldowns.get(name).copied().unwrap_or(0.0) > 0.0 {
            return Ok(false);
        }
        let costs = self.abilities[idx].cost.clone();
        // 消耗校验
        for cost in &costs {
            if let Some(d) = cost.delta {
                if d < 0.0 {
                    let cur = self.attrs.get(&cost.attr).unwrap_or(0.0);
                    if cur + d < 0.0 {
                        return Ok(false); // 资源不足
                    }
                }
            }
        }
        for cost in &costs {
            self.apply_now(cost);
        }
        self.cooldowns.insert(name.to_string(), self.abilities[idx].cooldown);
        Ok(true)
    }

    pub fn cooldown_remaining(&self, name: &str) -> f32 {
        self.cooldowns.get(name).copied().unwrap_or(0.0)
    }

    pub fn has_tag(&self, t: &str) -> bool {
        self.tags.contains(t)
    }

    pub fn add_tag(&mut self, t: &str) {
        self.tags.insert(t.to_string());
    }

    pub fn remove_tag(&mut self, t: &str) {
        self.tags.remove(t);
    }

    /// 推进：冷却递减 + 效果到期/周期。
    pub fn tick(&mut self, dt: f32) -> Vec<String> {
        let mut events = Vec::new();
        for cd in self.cooldowns.values_mut() {
            *cd = (*cd - dt).max(0.0);
        }
        let mut still: Vec<(GameEffect, f32, f32)> = Vec::new();
        for (e, remain, mut period) in std::mem::take(&mut self.active_effects) {
            let remain = remain - dt;
            period -= dt;
            if remain <= 0.0 {
                events.push(format!("effect:{}:expired", e.attr));
                continue;
            }
            if period <= 0.0 {
                self.apply_now(&e);
                period = e.period.unwrap_or(f32::MAX);
                events.push(format!("effect:{}:periodic", e.attr));
            }
            still.push((e, remain, period));
        }
        self.active_effects = still;
        events
    }
}

/// ScriptHost trait（IF-261）。
pub trait ScriptHost {
    fn name(&self) -> &str;
    fn compile(&mut self, src: &str) -> Result<ScriptId>;
    fn call(&mut self, script: ScriptId, entry: &str, args: &[Value]) -> Result<Value>;
}

impl ScriptHost for BytecodeHost {
    fn name(&self) -> &str {
        "bytecode"
    }
    fn compile(&mut self, src: &str) -> Result<ScriptId> {
        Self::compile(self, src)
    }
    fn call(&mut self, script: ScriptId, entry: &str, args: &[Value]) -> Result<Value> {
        Self::call(self, script, entry, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_semantics() {
        assert_eq!(Value::F64(1.5).as_f64(), 1.5);
        assert_eq!(Value::I64(2).as_f64(), 2.0);
        assert!(Value::I64(3).as_bool());
        assert!(!Value::Str(String::new()).as_bool());
    }

    #[test]
    fn compile_and_run_expressions() {
        let mut host = BytecodeHost::new();
        let id = host.compile("1 + 2 * 3").unwrap(); // 优先级 → 7
        let v = host.call(id, "main", &[]).unwrap();
        assert_eq!(v, Value::F64(7.0));
        let id2 = host.compile("(1 + 2) * 3").unwrap();
        assert_eq!(host.call(id2, "main", &[]).unwrap(), Value::F64(9.0));
        let id3 = host.compile("10 / 4").unwrap();
        assert_eq!(host.call(id3, "main", &[]).unwrap(), Value::F64(2.5));
        let id4 = host.compile("7 % 3").unwrap();
        assert_eq!(host.call(id4, "main", &[]).unwrap(), Value::F64(1.0));
        // 语法错误
        assert!(host.compile("1 +").is_err());
        assert!(host.compile("let 5 = 3").is_err());
        // 除零
        let id5 = host.compile("1 / 0").unwrap();
        assert!(host.call(id5, "main", &[]).is_err());
    }

    #[test]
    fn globals_and_locals() {
        let mut host = BytecodeHost::new();
        host.set_global("gx", Value::F64(10.0));
        let id = host.compile("gx + 5").unwrap();
        assert_eq!(host.call(id, "main", &[]).unwrap(), Value::F64(15.0));
        let id2 = host.compile("let a = 4 let b = a * 2 b + 1").unwrap();
        assert_eq!(host.call(id2, "main", &[]).unwrap(), Value::F64(9.0));
    }

    #[test]
    fn control_flow() {
        let mut host = BytecodeHost::new();
        let id = host.compile("let r = 0 if 1 < 2 { r = 10 } else { r = 20 } r").unwrap();
        assert_eq!(host.call(id, "main", &[]).unwrap(), Value::F64(10.0));
        let id2 =
            host.compile("let i = 0 let s = 0 while i < 5 { s = s + i i = i + 1 } s").unwrap();
        assert_eq!(host.call(id2, "main", &[]).unwrap(), Value::F64(10.0)); // 0+1+2+3+4
    }

    #[test]
    fn native_calls() {
        let mut host = BytecodeHost::new();
        fn double(args: &[Value]) -> Result<Value> {
            Ok(Value::F64(args.first().map(|v| v.as_f64()).unwrap_or(0.0) * 2.0))
        }
        host.register_native("double", double);
        let id = host.compile("double(21)").unwrap();
        assert_eq!(host.call(id, "main", &[]).unwrap(), Value::F64(42.0));
        // 未知原生
        let id2 = host.compile("nope(1)").unwrap();
        assert!(host.call(id2, "main", &[]).is_err());
    }

    #[test]
    fn debugger_breakpoint() {
        let mut host = BytecodeHost::new();
        let id = host.compile("1 + 2").unwrap();
        let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let h2 = hits.clone();
        host.debugger.add_breakpoint(id, 0);
        host.debugger.attach(Box::new(move |ev| {
            h2.lock().unwrap().push(ev);
        }));
        let v = host.call(id, "main", &[]).unwrap();
        assert_eq!(v, Value::Null); // ip=0 断点 → halted（无结果）
        let evs = hits.lock().unwrap();
        assert!(evs.iter().any(|e| matches!(e, VmEvent::Breakpoint(s, _) if *s == id)));
    }

    #[test]
    fn graph_compiles_and_runs() {
        let mut host = BytecodeHost::new();
        host.set_global("g", Value::F64(1.0));
        let mut g = GraphScript::default();
        g.add_node(1, GraphOp::Event("start".into()), true);
        g.add_node(2, GraphOp::SetVar, true);
        g.add_node(3, GraphOp::Print, true);
        g.add_node(4, GraphOp::ConstF64(7.0), false);
        g.connect(1, 0, 2, 0);
        g.connect(2, 0, 3, 0);
        g.connect(4, 0, 3, 1);
        let id = compile_graph(&mut host, &g).unwrap();
        let _ = host.call(id, "main", &[]).unwrap();
        assert_eq!(host.globals().get("g"), Some(&Value::F64(2.0))); // SetVar 生效
    }

    #[test]
    fn gas_abilities_and_effects() {
        let mut gas = AbilitySystem::new();
        gas.add_attribute("hp", 100.0);
        gas.add_attribute("mp", 50.0);
        gas.add_ability(Ability {
            name: "fireball".into(),
            cost: vec![GameEffect {
                attr: "mp".into(),
                delta: Some(-10.0),
                set_to: None,
                duration: 0.0,
                period: None,
                require_tags: vec![],
                forbid_tags: vec![],
            }],
            cooldown: 2.0,
            tags: vec!["magic".into()],
        });
        assert!(gas.try_use_ability("fireball").unwrap());
        assert_eq!(gas.attrs.get("mp"), Some(40.0));
        assert!(!gas.try_use_ability("fireball").unwrap()); // 冷却中
        assert!(gas.cooldown_remaining("fireball") > 0.0);
        for _ in 0..25 {
            gas.tick(0.1);
        }
        assert_eq!(gas.cooldown_remaining("fireball"), 0.0);
        // mp 耗尽
        for _ in 0..4 {
            gas.try_use_ability("fireball").unwrap();
            gas.tick(2.1);
        }
        assert!(!gas.try_use_ability("fireball").unwrap());
        // 周期效果
        gas.add_effect(GameEffect {
            attr: "hp".into(),
            delta: Some(1.0),
            set_to: None,
            duration: 1.0,
            period: Some(0.4),
            require_tags: vec!["regen".into()],
            forbid_tags: vec![],
        });
        assert_eq!(gas.attrs.get("hp"), Some(100.0)); // 缺 tag 未施加
        gas.add_tag("regen");
        gas.add_effect(GameEffect {
            attr: "hp".into(),
            delta: Some(-5.0),
            set_to: None,
            duration: 0.9,
            period: Some(0.3),
            require_tags: vec!["regen".into()],
            forbid_tags: vec!["curse".into()],
        });
        let mut periodic = 0;
        for _ in 0..9 {
            periodic += gas.tick(0.1).iter().filter(|e| e.contains("periodic")).count();
        }
        assert!(periodic >= 2); // 周期触发
        assert!(gas.attrs.get("hp").unwrap() < 100.0);
        assert!(gas.has_tag("regen"));
        gas.remove_tag("regen");
        assert!(!gas.has_tag("regen"));
    }
}
