//! AVAP ISA — instruction set for the Platon kernel.
//!
//! Depends only on platon-core (pure Rust rlib) and pyo3.
//! Does NOT depend on the platon cdylib to avoid double-linking Python symbols.
//!
//! The VM communicates with this ISA via a Python protocol:
//!   isa = AvapISA()
//!   vm.register_isa(isa)   # platon calls isa._dispatch(opcode, state_ptr, code_ptr, ip_ptr, py_ptr)

use platon_core::{
    Value, VMState, ISAProvider, InstructionSet, InstructionMeta, read_u32, ISAError,
};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Opcode constants
// ---------------------------------------------------------------------------

// Opcodes generated from opcodes.json at compile time via build.rs
include!(concat!(env!("OUT_DIR"), "/opcodes.rs"));

// ---------------------------------------------------------------------------
// Arithmetic trait for Value
// ---------------------------------------------------------------------------

trait AVAPArith: Sized {
    fn avap_add(self, o: Self) -> Result<Self, String>;
    fn avap_sub(self, o: Self) -> Result<Self, String>;
    fn avap_mul(self, o: Self) -> Result<Self, String>;
    fn avap_div(self, o: Self) -> Result<Self, String>;
    fn avap_mod(self, o: Self) -> Result<Self, String>;
    fn avap_lt(&self, o: &Self)  -> Result<bool, String>;
    fn avap_gt(&self, o: &Self)  -> Result<bool, String>;
}

impl AVAPArith for Value {
    fn avap_add(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a+b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a+b)),
            (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64+b)),
            (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a+b as f64)),
            (Value::Str(a),   Value::Str(b))   => Ok(Value::Str(a+&b)),
            (a,b)=>Err(format!("Cannot add {} and {}",a.type_name(),b.type_name())),
        }
    }
    fn avap_sub(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a-b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a-b)),
            (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64-b)),
            (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a-b as f64)),
            (a,b)=>Err(format!("Cannot sub {} and {}",a.type_name(),b.type_name())),
        }
    }
    fn avap_mul(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a*b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a*b)),
            (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(a as f64*b)),
            (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a*b as f64)),
            (a,b)=>Err(format!("Cannot mul {} and {}",a.type_name(),b.type_name())),
        }
    }
    fn avap_div(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   if b!=0   => Ok(Value::Float(a as f64/b as f64)),
            (Value::Float(a), Value::Float(b)) if b!=0.0 => Ok(Value::Float(a/b)),
            (Value::Int(a),   Value::Float(b)) if b!=0.0 => Ok(Value::Float(a as f64/b)),
            (Value::Float(a), Value::Int(b))   if b!=0   => Ok(Value::Float(a/b as f64)),
            _=>Err("Division by zero".to_string()),
        }
    }
    fn avap_mod(self, o: Self) -> Result<Self, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   if b!=0   => Ok(Value::Int(a%b)),
            (Value::Float(a), Value::Float(b)) if b!=0.0 => Ok(Value::Float(a%b)),
            _=>Err("Modulo by zero".to_string()),
        }
    }
    fn avap_lt(&self, o: &Self) -> Result<bool, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   => Ok(a<b),
            (Value::Float(a), Value::Float(b)) => Ok(a<b),
            (Value::Int(a),   Value::Float(b)) => Ok((*a as f64)<*b),
            (Value::Float(a), Value::Int(b))   => Ok(*a<(*b as f64)),
            (a,b)=>Err(format!("Cannot compare {} < {}",a.type_name(),b.type_name())),
        }
    }
    fn avap_gt(&self, o: &Self) -> Result<bool, String> {
        match (self, o) {
            (Value::Int(a),   Value::Int(b))   => Ok(a>b),
            (Value::Float(a), Value::Float(b)) => Ok(a>b),
            (Value::Int(a),   Value::Float(b)) => Ok((*a as f64)>*b),
            (Value::Float(a), Value::Int(b))   => Ok(*a>(*b as f64)),
            (a,b)=>Err(format!("Cannot compare {} > {}",a.type_name(),b.type_name())),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: get Python GIL from the opaque pointer passed by the VM
// ---------------------------------------------------------------------------

unsafe fn py_from_ctx<'py>(_ctx: *mut ()) -> Python<'py> {
    Python::assume_gil_acquired()
}

// ---------------------------------------------------------------------------
// CALL_EXT handler needs access to NativeRegistry stored in VMState
// ---------------------------------------------------------------------------

struct RegistryRef(*mut ());

unsafe impl Send for RegistryRef {}
unsafe impl Sync for RegistryRef {}

impl RegistryRef {
    unsafe fn get<'a>(&self) -> Option<&'a pyo3::types::PyDict> {
        None
    }
}

// ---------------------------------------------------------------------------
// Value conversion helpers for CALL_EXT  [PyO3 0.22]
// ---------------------------------------------------------------------------

fn value_to_py<'py>(v: &Value, py: Python<'py>) -> Bound<'py, PyAny> {
    match v {
        Value::Null        => py.None().into_bound(py),
        Value::Bool(b)     => b.into_py(py).into_bound(py),
        Value::Int(i)      => i.into_py(py).into_bound(py),
        Value::Float(f)    => f.into_py(py).into_bound(py),
        Value::Str(s)      => s.into_py(py).into_bound(py),
        Value::List(items) => {
            let v: Vec<Bound<'_, PyAny>> = items.iter().map(|i| value_to_py(i, py)).collect();
            PyList::new_bound(py, &v).into_any()
        }
        Value::Dict(pairs) => {
            let d = PyDict::new_bound(py);
            for (k, v) in pairs { let _ = d.set_item(k, value_to_py(v, py)); }
            d.into_any()
        }
        Value::Iter(items, idx) => {
            let v: Vec<Bound<'_, PyAny>> = items[*idx..].iter().map(|i| value_to_py(i, py)).collect();
            PyList::new_bound(py, &v).into_any()
        }
    }
}

fn value_from_py(obj: &Bound<'_, PyAny>) -> Result<Value, ISAError> {
    if obj.is_none() { return Ok(Value::Null); }
    if let Ok(b) = obj.extract::<bool>()   { return Ok(Value::Bool(b)); }
    if let Ok(i) = obj.extract::<i64>()    { return Ok(Value::Int(i)); }
    if let Ok(f) = obj.extract::<f64>()    { return Ok(Value::Float(f)); }
    if let Ok(s) = obj.extract::<String>() { return Ok(Value::Str(s)); }
    // dict BEFORE list
    if let Ok(d) = obj.downcast::<PyDict>() {
        let mut pairs = Vec::new();
        for (k, v) in d.iter() {
            let key = k.extract::<String>().map_err(|e| e.to_string())?;
            pairs.push((key, value_from_py(&v)?));
        }
        return Ok(Value::Dict(pairs));
    }
    if let Ok(list) = obj.extract::<Vec<Bound<'_, PyAny>>>() {
        let items: Result<Vec<_>, _> = list.iter().map(|i| value_from_py(i)).collect();
        return Ok(Value::List(items?));
    }
    Err(format!("Unsupported Python type in CALL_EXT result"))
}

// ---------------------------------------------------------------------------
// Instruction handlers
// ---------------------------------------------------------------------------

fn h_nop(_s: &mut VMState, _c: &[u8], _ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> { Ok(()) }

fn h_push(s: &mut VMState, c: &[u8], ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    let idx = read_u32(c, ip)? as usize;
    let v = s.get_const(idx).cloned().ok_or_else(|| format!("PUSH: index {} OOB", idx))?;
    s.push(v); Ok(())
}
fn h_pop(s: &mut VMState, _c: &[u8], _ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    s.pop(); Ok(())
}
fn h_dup(s: &mut VMState, _c: &[u8], _ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    let v = s.peek().cloned().ok_or("DUP: empty stack")?;
    s.push(v); Ok(())
}
fn h_load_none(s: &mut VMState, _c: &[u8], _ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    s.push(Value::Null); Ok(())
}
fn h_load(s: &mut VMState, c: &[u8], ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    let idx = read_u32(c, ip)? as usize;
    let name = s.get_const_str(idx).ok_or("LOAD: not a string")?;
    let v = s.load_global(&name);
    s.push(v); Ok(())
}
fn h_store(s: &mut VMState, c: &[u8], ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
    let idx = read_u32(c, ip)? as usize;
    let name = s.get_const_str(idx).ok_or("STORE: not a string")?;
    let v = s.pop().ok_or("STORE: empty stack")?;
    s.store_global(name, v); Ok(())
}

macro_rules! binop {
    ($name:ident, $method:ident, $label:literal) => {
        fn $name(s: &mut VMState, _c: &[u8], _ip: &mut usize, _py: *mut ()) -> Result<(), ISAError> {
            let b = s.pop().ok_or(concat!($label, ": empty stack"))?;
            let a = s.pop().ok_or(concat!($label, ": empty stack"))?;
            s.push(a.$method(b)?); Ok(())
        }
    };
}
binop!(h_add, avap_add, "ADD");
binop!(h_sub, avap_sub, "SUB");
binop!(h_mul, avap_mul, "MUL");
binop!(h_div, avap_div, "DIV");
binop!(h_mod, avap_mod, "MOD");

fn h_neg(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let a = s.pop().ok_or("NEG: empty stack")?;
    match a {
        Value::Int(i) => s.push(Value::Int(-i)),
        Value::Float(f) => s.push(Value::Float(-f)),
        _ => return Err("NEG: not a number".to_string()),
    }
    Ok(())
}
fn h_not(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let a = s.pop().ok_or("NOT: empty stack")?;
    s.push(Value::Bool(!a.is_truthy())); Ok(())
}
fn h_eq(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("EQ: empty")?; let a=s.pop().ok_or("EQ: empty")?;
    s.push(Value::Bool(a.eq_val(&b))); Ok(())
}
fn h_neq(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("NEQ: empty")?; let a=s.pop().ok_or("NEQ: empty")?;
    s.push(Value::Bool(!a.eq_val(&b))); Ok(())
}
fn h_lt(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("LT: empty")?; let a=s.pop().ok_or("LT: empty")?;
    s.push(Value::Bool(a.avap_lt(&b)?)); Ok(())
}
fn h_gt(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("GT: empty")?; let a=s.pop().ok_or("GT: empty")?;
    s.push(Value::Bool(a.avap_gt(&b)?)); Ok(())
}
fn h_lte(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("LTE: empty")?; let a=s.pop().ok_or("LTE: empty")?;
    s.push(Value::Bool(!a.avap_gt(&b)?)); Ok(())
}
fn h_gte(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("GTE: empty")?; let a=s.pop().ok_or("GTE: empty")?;
    s.push(Value::Bool(!a.avap_lt(&b)?)); Ok(())
}
fn h_in(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let container=s.pop().ok_or("IN: empty")?; let item=s.pop().ok_or("IN: empty")?;
    let result = match &container {
        Value::Str(m) if m == "__CONECTOR_VARS__" =>
            matches!(&item, Value::Str(k) if s.conector_vars.contains_key(k)),
        Value::Str(m) if m == "__CONECTOR_RESULTS__" =>
            matches!(&item, Value::Str(k) if s.results.contains_key(k)),
        _ => container.contains(&item),
    };
    s.push(Value::Bool(result)); Ok(())
}
fn h_not_in(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let container=s.pop().ok_or("NOT_IN: empty")?; let item=s.pop().ok_or("NOT_IN: empty")?;
    let result = match &container {
        Value::Str(m) if m == "__CONECTOR_VARS__" =>
            !matches!(&item, Value::Str(k) if s.conector_vars.contains_key(k)),
        Value::Str(m) if m == "__CONECTOR_RESULTS__" =>
            !matches!(&item, Value::Str(k) if s.results.contains_key(k)),
        _ => !container.contains(&item),
    };
    s.push(Value::Bool(result)); Ok(())
}
fn h_is(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("IS: empty")?; let a=s.pop().ok_or("IS: empty")?;
    s.push(Value::Bool(matches!((&a,&b),(Value::Null,Value::Null)))); Ok(())
}
fn h_is_not(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("IS_NOT: empty")?; let a=s.pop().ok_or("IS_NOT: empty")?;
    s.push(Value::Bool(!matches!((&a,&b),(Value::Null,Value::Null)))); Ok(())
}
fn h_bool_and(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("AND: empty")?; let a=s.pop().ok_or("AND: empty")?;
    s.push(if a.is_truthy(){b}else{a}); Ok(())
}
fn h_bool_or(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let b=s.pop().ok_or("OR: empty")?; let a=s.pop().ok_or("OR: empty")?;
    s.push(if a.is_truthy(){a}else{b}); Ok(())
}
fn h_jmp(_: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let t=read_u32(c,ip)? as usize; *ip=t; Ok(())
}
fn h_jmp_if(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let t=read_u32(c,ip)? as usize;
    if s.peek().map(|v|v.is_truthy()).unwrap_or(false){*ip=t;} Ok(())
}
fn h_jmp_if_not(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let t=read_u32(c,ip)? as usize;
    if !s.peek().map(|v|v.is_truthy()).unwrap_or(true){*ip=t;} Ok(())
}
fn h_jmp_if_pop(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let t=read_u32(c,ip)? as usize;
    let v=s.pop().ok_or("JMP_IF_POP: empty")?;
    if v.is_truthy(){*ip=t;} Ok(())
}
fn h_jmp_if_not_pop(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let t=read_u32(c,ip)? as usize;
    let v=s.pop().ok_or("JMP_IF_NOT_POP: empty")?;
    if !v.is_truthy(){*ip=t;} Ok(())
}
fn h_push_try(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let handler=read_u32(c,ip)? as usize; s.try_stack.push(handler); Ok(())
}
fn h_pop_try(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    s.try_stack.pop(); Ok(())
}
fn h_raise(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let err=s.pop().ok_or("RAISE: empty")?;
    Err(err.to_string_repr())
}
fn h_return(_s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    Ok(())
}
fn h_get_iter(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let v=s.pop().ok_or("GET_ITER: empty")?;
    let iter = match v {
        Value::List(items) => Value::Iter(items, 0),
        Value::Iter(items,idx) => Value::Iter(items,idx),
        other => return Err(format!("GET_ITER: {} is not iterable", other.type_name())),
    };
    s.push(iter); Ok(())
}
fn h_for_iter(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let exit_ip = read_u32(c, ip)? as usize;
    let iter = s.peek_mut().ok_or("FOR_ITER: empty")?;
    if let Value::Iter(items, idx) = iter {
        if *idx < items.len() {
            let next = items[*idx].clone(); *idx+=1; s.push(next);
        } else {
            s.pop(); *ip=exit_ip;
        }
    } else {
        return Err("FOR_ITER: not an iterator".to_string());
    }
    Ok(())
}
fn h_get_attr(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let idx  = read_u32(c, ip)? as usize;
    let attr = s.get_const_str(idx).ok_or("GET_ATTR: not a string")?;
    let obj  = s.pop().ok_or("GET_ATTR: empty")?;
    let val  = match (&obj, attr.as_str()) {
        (Value::Str(m), "variables") if m == "__CONECTOR__" => {
            // Lazy-init conector_vars from the __conector__ global injected by Python
            if s.conector_vars.is_empty() {
                let snap = s.load_global("__conector__");
                if let Value::Dict(ref cp) = snap {
                    for (k, v) in cp {
                        if k == "variables" {
                            if let Value::Dict(vars) = v {
                                for (vk, vv) in vars {
                                    s.conector_vars.insert(vk.clone(), vv.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }
            s.push(Value::Str("__CONECTOR_VARS__".to_string()));
            return Ok(());
        }
        (Value::Str(m), "results") if m == "__CONECTOR__" => {
            // Lazy-init results from the __conector__ global injected by Python
            if s.results.is_empty() {
                let snap = s.load_global("__conector__");
                if let Value::Dict(ref cp) = snap {
                    for (k, v) in cp {
                        if k == "results" {
                            if let Value::Dict(vars) = v {
                                for (vk, vv) in vars {
                                    s.results.insert(vk.clone(), vv.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }
            s.push(Value::Str("__CONECTOR_RESULTS__".to_string()));
            return Ok(());
        }
        (Value::Str(m), name) if m == "__CONECTOR__" => {
            s.load_global(&format!("__cattr_{}__", name))
        }
        (Value::Str(m), key) if m == "__CONECTOR_VARS__" => {
            s.conector_vars.get(key).cloned().unwrap_or(Value::Null)
        }
        (Value::Dict(pairs), name) =>
            pairs.iter().find(|(k,_)|k==name).map(|(_,v)|v.clone()).unwrap_or(Value::Null),
        _ => Value::Null,
    };
    s.push(val); Ok(())
}
fn h_set_attr(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let idx   = read_u32(c, ip)? as usize;
    let attr  = s.get_const_str(idx).ok_or("SET_ATTR: not a string")?;
    let value = s.pop().ok_or("SET_ATTR: empty")?;
    let obj   = s.peek_mut().ok_or("SET_ATTR: empty")?;
    if let Value::Dict(pairs) = obj {
        if let Some(p)=pairs.iter_mut().find(|(k,_)|*k==attr){p.1=value;}
        else{pairs.push((attr,value));}
    }
    Ok(())
}
fn h_get_item(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let key=s.pop().ok_or("GET_ITEM: empty")?;
    let obj=s.pop().ok_or("GET_ITEM: empty")?;
    let result = match &obj {
        Value::Str(m) if m == "__CONECTOR_VARS__" =>
            if let Value::Str(k) = &key { s.conector_vars.get(k).cloned().unwrap_or(Value::Null) } else { Value::Null },
        Value::Str(m) if m == "__CONECTOR_RESULTS__" =>
            if let Value::Str(k) = &key { s.results.get(k).cloned().unwrap_or(Value::Null) } else { Value::Null },
        _ => obj.get_item(&key).unwrap_or(Value::Null),
    };
    s.push(result); Ok(())
}
fn h_set_item(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let value = s.pop().ok_or("SET_ITEM: empty")?;
    let key   = s.pop().ok_or("SET_ITEM: empty")?;
    let obj   = s.peek_mut().ok_or("SET_ITEM: empty")?;
    match obj {
        Value::Str(m) if m == "__CONECTOR_VARS__" => {
            if let Value::Str(k) = key { s.conector_vars.insert(k, value); }
        }
        Value::Str(m) if m == "__CONECTOR_RESULTS__" => {
            if let Value::Str(k) = key { s.results.insert(k, value); }
        }
        _ => { obj.set_item(key, value); }
    }
    Ok(())
}
fn h_delete_item(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let key=s.pop().ok_or("DELETE_ITEM: empty")?;
    let obj=s.peek_mut().ok_or("DELETE_ITEM: empty")?;
    if let (Value::Dict(pairs),Value::Str(k))=(obj,&key){pairs.retain(|(pk,_)|pk!=k);}
    Ok(())
}
fn h_build_list(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let n=read_u32(c,ip)? as usize;
    let mut items=Vec::with_capacity(n);
    for _ in 0..n{items.push(s.pop().ok_or("BUILD_LIST: empty")?);}
    items.reverse(); s.push(Value::List(items)); Ok(())
}
fn h_build_dict(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let n=read_u32(c,ip)? as usize;
    let mut pairs=Vec::with_capacity(n);
    for _ in 0..n{
        let v=s.pop().ok_or("BUILD_DICT: empty")?;
        let k=s.pop().ok_or("BUILD_DICT: empty")?;
        if let Value::Str(key)=k{pairs.push((key,v));}
    }
    pairs.reverse(); s.push(Value::Dict(pairs)); Ok(())
}
fn h_build_tuple(s: &mut VMState, c: &[u8], ip: &mut usize, py: *mut ()) -> Result<(), ISAError> {
    h_build_list(s,c,ip,py)
}

fn h_call_ext(s: &mut VMState, c: &[u8], ip: &mut usize, _py_ctx: *mut ()) -> Result<(), ISAError> {
    let func_id = read_u32(c, ip)?;
    if s.registry_ptr.is_null() {
        return Err(format!("CALL_EXT: no registry (func_id={})", func_id));
    }
    let dict_owned: Py<PyDict> = unsafe {
        let dict_py = &*(s.registry_ptr as *const Py<PyDict>);
        Python::with_gil(|py| dict_py.clone_ref(py))
    };
    let globals_snapshot: Vec<(String, Value)> = s.globals.iter()
        .map(|(k, v)| (k.clone(), v.clone())).collect();
    let stack_snapshot: Vec<Value> = s.stack.clone();

    let (ret_val, updated_globals) = Python::with_gil(|py| -> Result<(Option<Value>, Vec<(String,Value)>), ISAError> {
        let dict = dict_owned.bind(py);
        let func = dict.get_item(func_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("CALL_EXT: func_id {} not registered", func_id))?;

        let py_globals = PyDict::new_bound(py);
        for (k, v) in &globals_snapshot {
            py_globals.set_item(k, value_to_py(v, py)).map_err(|e: pyo3::PyErr| e.to_string())?;
        }
        let py_stack_items: Vec<PyObject> = stack_snapshot.iter()
            .map(|v| value_to_py(v, py).into()).collect();
        let py_stack = PyList::new_bound(py, &py_stack_items);
        let proxy = PyDict::new_bound(py);
        proxy.set_item("globals", &py_globals).map_err(|e: pyo3::PyErr| e.to_string())?;

        let ret = func.call1((&proxy, &py_stack))
            .map_err(|e| format!("CALL_EXT native fn (id={}): {}", func_id, e))?;

        let mut updated = Vec::new();
        for item in py_globals.iter() {
            let (k, v) = item;
            if let Ok(key) = k.extract::<String>() {
                if let Ok(val) = value_from_py(&v) {
                    updated.push((key, val));
                }
            }
        }
        let ret_val = if ret.is_none() { None } else { value_from_py(&ret).ok() };
        Ok((ret_val, updated))
    })?;

    for (k, v) in updated_globals { s.globals.insert(k, v); }
    if let Some(v) = ret_val { s.push(v); }
    Ok(())
}

fn h_call_func(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let n_args=read_u32(c,ip)? as usize;
    let mut args=Vec::with_capacity(n_args);
    for _ in 0..n_args{args.push(s.pop().ok_or("CALL_FUNC: empty")?);}
    args.reverse();
    let func=s.pop().ok_or("CALL_FUNC: empty")?;
    if let Value::Str(name)=&func{
        s.push(call_builtin(name,args)?);
    } else {
        return Err("CALL_FUNC: not a callable name".to_string());
    }
    Ok(())
}
fn h_call_method(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let name_idx=read_u32(c,ip)? as usize;
    let n_args=read_u32(c,ip)? as usize;
    let method=s.get_const_str(name_idx).ok_or("CALL_METHOD: not a string")?;
    let mut args=Vec::with_capacity(n_args);
    for _ in 0..n_args{args.push(s.pop().ok_or("CALL_METHOD: empty")?);}
    args.reverse();
    let obj=s.pop().ok_or("CALL_METHOD: empty")?;
    let result = match (&obj, method.as_str()) {
        (Value::Str(m), "get") if m == "__CONECTOR_VARS__" => {
            let key = match args.first() { Some(Value::Str(k)) => k.clone(), _ => return Ok(()) };
            let default = args.get(1).cloned().unwrap_or(Value::Null);
            s.conector_vars.get(&key).cloned().unwrap_or(default)
        }
        (Value::Str(m), "update") if m == "__CONECTOR_VARS__" => {
            if let Some(Value::Dict(pairs)) = args.first() {
                for (k,v) in pairs { s.conector_vars.insert(k.clone(), v.clone()); }
            }
            Value::Null
        }
        (Value::Str(m), "keys") if m == "__CONECTOR_VARS__" => {
            Value::List(s.conector_vars.keys().map(|k| Value::Str(k.clone())).collect())
        }
        (Value::Str(m), "get") if m == "__CONECTOR_RESULTS__" => {
            let key = match args.first() { Some(Value::Str(k)) => k.clone(), _ => return Ok(()) };
            let default = args.get(1).cloned().unwrap_or(Value::Null);
            s.results.get(&key).cloned().unwrap_or(default)
        }
        _ => call_method(obj, &method, args)?
    };
    s.push(result);
    Ok(())
}
fn h_load_conector(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    s.push(Value::Str("__CONECTOR__".to_string())); Ok(())
}
fn h_load_task(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    s.push(s.load_global("__task__")); Ok(())
}
fn h_load_builtin(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let idx=read_u32(c,ip)? as usize;
    let name=s.get_const_str(idx).ok_or("LOAD_BUILTIN: not a string")?;
    s.push(Value::Str(name)); Ok(())
}
fn h_import_mod(s: &mut VMState, c: &[u8], ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let idx=read_u32(c,ip)? as usize;
    let name=s.get_const_str(idx).ok_or("IMPORT_MOD: not a string")?;
    s.push(Value::Str(format!("__module__:{}", name))); Ok(())
}
fn type_matches(obj: &Value, type_name: &str) -> bool {
    match type_name {
        "str"   => matches!(obj, Value::Str(_)),
        "int"   => matches!(obj, Value::Int(_)),
        "float" => matches!(obj, Value::Float(_)) || matches!(obj, Value::Int(_)),
        "bool"  => matches!(obj, Value::Bool(_)),
        "list"  => matches!(obj, Value::List(_)),
        "dict"  => matches!(obj, Value::Dict(_)),
        "bytes" | "bytearray" => matches!(obj, Value::Str(_)),
        _       => obj.type_name() == type_name,
    }
}
fn h_is_instance(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let type_val = s.pop().ok_or("IS_INSTANCE: empty")?;
    let obj      = s.pop().ok_or("IS_INSTANCE: empty")?;
    let matches = match &type_val {
        Value::Str(t) => type_matches(&obj, t.as_str()),
        Value::List(types) | Value::Iter(types, _) => {
            types.iter().any(|t| {
                if let Value::Str(name) = t { type_matches(&obj, name.as_str()) }
                else { false }
            })
        }
        _ => false,
    };
    s.push(Value::Bool(matches)); Ok(())
}
fn h_is_none(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let v=s.pop().ok_or("IS_NONE: empty")?;
    s.push(Value::Bool(matches!(v,Value::Null))); Ok(())
}
fn h_type_of(s: &mut VMState, _c: &[u8], _ip: &mut usize, _: *mut ()) -> Result<(), ISAError> {
    let v=s.pop().ok_or("TYPE_OF: empty")?;
    s.push(Value::Str(v.type_name().to_string())); Ok(())
}

// ---------------------------------------------------------------------------
// Builtin and method dispatch
// ---------------------------------------------------------------------------

fn call_builtin(name: &str, args: Vec<Value>) -> Result<Value, ISAError> {
    match name {
        "str"   => Ok(Value::Str(args.into_iter().next().unwrap_or(Value::Null).to_string_repr())),
        "int"   => match args.into_iter().next().unwrap_or(Value::Null) {
            Value::Int(i)   => Ok(Value::Int(i)),
            Value::Float(f) => Ok(Value::Int(f as i64)),
            Value::Str(s)   => s.trim().parse::<i64>().map(Value::Int)
                .map_err(|_|format!("Cannot convert to int")),
            Value::Bool(b)  => Ok(Value::Int(b as i64)),
            other => Err(format!("Cannot convert {} to int", other.type_name())),
        },
        "float" => match args.into_iter().next().unwrap_or(Value::Null) {
            Value::Float(f) => Ok(Value::Float(f)),
            Value::Int(i)   => Ok(Value::Float(i as f64)),
            Value::Str(s)   => s.trim().parse::<f64>().map(Value::Float)
                .map_err(|_|format!("Cannot convert to float")),
            other => Err(format!("Cannot convert {} to float", other.type_name())),
        },
        "bool"  => Ok(Value::Bool(args.into_iter().next().unwrap_or(Value::Null).is_truthy())),
        "len"   => match args.into_iter().next().unwrap_or(Value::Null) {
            Value::Str(s)  => Ok(Value::Int(s.len() as i64)),
            Value::List(v) => Ok(Value::Int(v.len() as i64)),
            Value::Dict(d) => Ok(Value::Int(d.len() as i64)),
            other => Err(format!("len() on {}", other.type_name())),
        },
        "list"  => match args.into_iter().next().unwrap_or(Value::Null) {
            Value::List(v)        => Ok(Value::List(v)),
            Value::Iter(items,idx)=> Ok(Value::List(items[idx..].to_vec())),
            other => Err(format!("list() on {}", other.type_name())),
        },
        "dict"       => Ok(Value::Dict(Vec::new())),
        "isinstance" => {
            if args.len()<2{return Ok(Value::Bool(false));}
            let obj=&args[0]; let t=args[1].to_string_repr();
            Ok(Value::Bool(
                (t=="str"&&matches!(obj,Value::Str(_)))||
                (t=="int"&&matches!(obj,Value::Int(_)))||
                (t=="float"&&matches!(obj,Value::Float(_)))||
                (t=="bool"&&matches!(obj,Value::Bool(_)))||
                (t=="list"&&matches!(obj,Value::List(_)))||
                (t=="dict"&&matches!(obj,Value::Dict(_)))
            ))
        }
        "print" => {
            let parts:Vec<String>=args.iter().map(|a|a.to_string_repr()).collect();
            println!("{}",parts.join(" ")); Ok(Value::Null)
        }
        _ => Err(format!("Unknown builtin: {}", name)),
    }
}

fn call_method(obj: Value, method: &str, args: Vec<Value>) -> Result<Value, ISAError> {
    match (&obj, method) {
        (Value::Str(s),"strip")     => Ok(Value::Str(s.trim().to_string())),
        (Value::Str(s),"upper")     => Ok(Value::Str(s.to_uppercase())),
        (Value::Str(s),"lower")     => Ok(Value::Str(s.to_lowercase())),
        (Value::Str(s),"isdigit")   => Ok(Value::Bool(!s.is_empty()&&s.chars().all(|c|c.is_ascii_digit()))),
        (Value::Str(s),"startswith")=> {
            let p=match args.first(){Some(Value::Str(p))=>p.clone(),_=>"".to_string()};
            Ok(Value::Bool(s.starts_with(&*p)))
        }
        (Value::Str(s),"endswith")  => {
            let p=match args.first(){Some(Value::Str(p))=>p.clone(),_=>"".to_string()};
            Ok(Value::Bool(s.ends_with(&*p)))
        }
        (Value::Str(s),"replace")   => {
            let from=match args.first(){Some(Value::Str(f))=>f.clone(),_=>"".to_string()};
            let to=match args.get(1){Some(Value::Str(t))=>t.clone(),_=>"".to_string()};
            Ok(Value::Str(s.replace(&*from,&*to)))
        }
        (Value::Str(s),"split")     => {
            let sep=match args.first(){Some(Value::Str(d))=>d.clone(),_=>" ".to_string()};
            Ok(Value::List(s.split(&*sep).map(|p|Value::Str(p.to_string())).collect()))
        }
        (Value::Str(s),"encode")|(Value::Str(s),"decode") => Ok(Value::Str(s.clone())),
        (Value::Dict(pairs),"get")  => {
            let key=match args.first(){Some(Value::Str(k))=>k.clone(),_=>"".to_string()};
            let default=args.get(1).cloned().unwrap_or(Value::Null);
            Ok(pairs.iter().find(|(k,_)|*k==key).map(|(_,v)|v.clone()).unwrap_or(default))
        }
        (Value::Dict(pairs),"keys")   => Ok(Value::List(pairs.iter().map(|(k,_)|Value::Str(k.clone())).collect())),
        (Value::Dict(pairs),"values") => Ok(Value::List(pairs.iter().map(|(_,v)|v.clone()).collect())),
        (Value::Dict(pairs),"items")  => Ok(Value::List(pairs.iter().map(|(k,v)|
            Value::List(vec![Value::Str(k.clone()),v.clone()])).collect())),
        (Value::Dict(_),"update")     => Ok(Value::Null),
        (Value::List(_),"append")     => Ok(Value::Null),
        _ => Err(format!("Unknown method {}.{}()", obj.type_name(), method)),
    }
}

// ---------------------------------------------------------------------------
// AvapRegistry
// ---------------------------------------------------------------------------

use std::collections::HashMap;

struct AvapRegistry {
    functions: HashMap<u32, PyObject>,
    attrs_obj: PyObject,
}

impl AvapRegistry {
    fn get_function(&self, func_id: u32) -> Option<PyObject> {
        Python::with_gil(|py| self.functions.get(&func_id).map(|o| o.clone_ref(py)))
    }
}

// ---------------------------------------------------------------------------
// AvapISA — implements ISAProvider from platon-core
// ---------------------------------------------------------------------------

pub struct AvapISA {
    isa: InstructionSet,
}

impl AvapISA {
    pub fn new() -> Self {
        let mut isa = InstructionSet::new(op::HALT);
        macro_rules! reg {
            ($op:expr, $name:literal, $n:expr, $h:expr) => {
                isa.register(InstructionMeta { opcode: $op, name: $name, n_u32_args: $n }, $h);
            };
        }
        reg!(op::NOP,            "NOP",            0, h_nop);
        reg!(op::PUSH,           "PUSH",           1, h_push);
        reg!(op::POP,            "POP",            0, h_pop);
        reg!(op::DUP,            "DUP",            0, h_dup);
        reg!(op::LOAD_NONE,      "LOAD_NONE",      0, h_load_none);
        reg!(op::LOAD,           "LOAD",           1, h_load);
        reg!(op::STORE,          "STORE",          1, h_store);
        reg!(op::ADD,            "ADD",            0, h_add);
        reg!(op::SUB,            "SUB",            0, h_sub);
        reg!(op::MUL,            "MUL",            0, h_mul);
        reg!(op::DIV,            "DIV",            0, h_div);
        reg!(op::MOD,            "MOD",            0, h_mod);
        reg!(op::NEG,            "NEG",            0, h_neg);
        reg!(op::NOT,            "NOT",            0, h_not);
        reg!(op::EQ,             "EQ",             0, h_eq);
        reg!(op::NEQ,            "NEQ",            0, h_neq);
        reg!(op::LT,             "LT",             0, h_lt);
        reg!(op::GT,             "GT",             0, h_gt);
        reg!(op::LTE,            "LTE",            0, h_lte);
        reg!(op::GTE,            "GTE",            0, h_gte);
        reg!(op::IN,             "IN",             0, h_in);
        reg!(op::NOT_IN,         "NOT_IN",         0, h_not_in);
        reg!(op::IS,             "IS",             0, h_is);
        reg!(op::IS_NOT,         "IS_NOT",         0, h_is_not);
        reg!(op::BOOL_AND,       "BOOL_AND",       0, h_bool_and);
        reg!(op::BOOL_OR,        "BOOL_OR",        0, h_bool_or);
        reg!(op::JMP,            "JMP",            1, h_jmp);
        reg!(op::JMP_IF,         "JMP_IF",         1, h_jmp_if);
        reg!(op::JMP_IF_NOT,     "JMP_IF_NOT",     1, h_jmp_if_not);
        reg!(op::JMP_IF_POP,     "JMP_IF_POP",     1, h_jmp_if_pop);
        reg!(op::JMP_IF_NOT_POP, "JMP_IF_NOT_POP", 1, h_jmp_if_not_pop);
        reg!(op::PUSH_TRY,       "PUSH_TRY",       1, h_push_try);
        reg!(op::POP_TRY,        "POP_TRY",        0, h_pop_try);
        reg!(op::RAISE,          "RAISE",          0, h_raise);
        reg!(op::RETURN,         "RETURN",         0, h_return);
        reg!(op::GET_ITER,       "GET_ITER",       0, h_get_iter);
        reg!(op::FOR_ITER,       "FOR_ITER",       1, h_for_iter);
        reg!(op::GET_ATTR,       "GET_ATTR",       1, h_get_attr);
        reg!(op::SET_ATTR,       "SET_ATTR",       1, h_set_attr);
        reg!(op::GET_ITEM,       "GET_ITEM",       0, h_get_item);
        reg!(op::SET_ITEM,       "SET_ITEM",       0, h_set_item);
        reg!(op::DELETE_ITEM,    "DELETE_ITEM",    0, h_delete_item);
        reg!(op::BUILD_LIST,     "BUILD_LIST",     1, h_build_list);
        reg!(op::BUILD_DICT,     "BUILD_DICT",     1, h_build_dict);
        reg!(op::BUILD_TUPLE,    "BUILD_TUPLE",    1, h_build_tuple);
        reg!(op::CALL_EXT,       "CALL_EXT",       1, h_call_ext);
        reg!(op::CALL_FUNC,      "CALL_FUNC",      1, h_call_func);
        reg!(op::CALL_METHOD,    "CALL_METHOD",    2, h_call_method);
        reg!(op::LOAD_CONECTOR,  "LOAD_CONECTOR",  0, h_load_conector);
        reg!(op::LOAD_TASK,      "LOAD_TASK",      0, h_load_task);
        reg!(op::LOAD_BUILTIN,   "LOAD_BUILTIN",   1, h_load_builtin);
        reg!(op::IMPORT_MOD,     "IMPORT_MOD",     1, h_import_mod);
        reg!(op::IS_INSTANCE,    "IS_INSTANCE",    0, h_is_instance);
        reg!(op::IS_NONE,        "IS_NONE",        0, h_is_none);
        reg!(op::TYPE_OF,        "TYPE_OF",        0, h_type_of);
        Self { isa }
    }
}

impl ISAProvider for AvapISA {
    fn name(&self)            -> &str         { isa_meta::NAME }
    fn version(&self)         -> (u8, u8, u8) { isa_meta::VERSION }
    fn instruction_set(&self) -> &InstructionSet { &self.isa }
}

// ---------------------------------------------------------------------------
// PyAvapISA — Python-visible wrapper
// ---------------------------------------------------------------------------

use std::cell::RefCell;
thread_local! {
    static ACTIVE_ISA: RefCell<Option<Arc<AvapISA>>> = RefCell::new(None);
}

#[pyclass(name = "AvapISA")]
pub struct PyAvapISA {
    inner: Arc<AvapISA>,
}

#[pymethods]
impl PyAvapISA {
    #[new]
    fn new() -> Self {
        let isa = Arc::new(AvapISA::new());
        ACTIVE_ISA.with(|cell| { *cell.borrow_mut() = Some(isa.clone()); });
        Self { inner: isa }
    }
    fn name(&self) -> &str { self.inner.name() }
    fn opcode_count(&self) -> usize { self.inner.instruction_set().len() }
    fn _get_arc_ptr(&self) -> (u64, u64) {
        let arc_clone: Arc<dyn ISAProvider> = self.inner.clone();
        let raw: *const dyn ISAProvider = Arc::into_raw(arc_clone);
        unsafe { std::mem::transmute::<*const dyn ISAProvider, (u64, u64)>(raw) }
    }
    fn __repr__(&self) -> String {
        format!("<AvapISA v{}.{}.{} ({} opcodes)>",
            self.inner.version().0, self.inner.version().1, self.inner.version().2,
            self.inner.instruction_set().len())
    }
}

// ---------------------------------------------------------------------------
// Module entry point  [PyO3 0.22]
// ---------------------------------------------------------------------------

#[pymodule]
fn avap_isa(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAvapISA>()?;
    m.add("__version__", "0.1.0")?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/integration.rs"]
mod integration_tests;
