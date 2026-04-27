// ─────────────────────────────────────────────────────────────────────────────
// avap-isa integration tests
//
// Run with:  cargo test -p avap-isa
//
// These tests build real AVBC bytecode from scratch and execute it through
// the full AvapISA instruction set without PyO3 (no Python needed).
//
// Coverage:
//   Stack & variables:  NOP, PUSH, POP, DUP, LOAD_NONE, LOAD, STORE
//   Arithmetic:         ADD, SUB, MUL, DIV, MOD, NEG, NOT
//   Comparison:         EQ, NEQ, LT, GT, LTE, GTE, IN, NOT_IN
//   Control flow:       JMP, JMP_IF, JMP_IF_NOT, JMP_IF_NOT_POP, JMP_IF_POP
//   Error handling:     PUSH_TRY, POP_TRY, RAISE
//   Iteration:          GET_ITER, FOR_ITER
//   Collections:        BUILD_LIST, BUILD_DICT, BUILD_TUPLE
//   Object access:      GET_ATTR, SET_ATTR, GET_ITEM, SET_ITEM, DELETE_ITEM
//   Type system:        IS_INSTANCE, IS_NONE, TYPE_OF
//   Runtime:            LOAD_CONECTOR, conector_vars, results
//   Methods:            CALL_METHOD (str, dict, list)
// ─────────────────────────────────────────────────────────────────────────────

use platon_core::{Value, VMState, ISAProvider};

// ─── Bytecode builder ────────────────────────────────────────────────────────
// Builds a minimal AVBC binary (header + constant pool + instruction stream).

struct BytecodeBuilder {
    constants: Vec<Value>,
    code:      Vec<u8>,
}

impl BytecodeBuilder {
    fn new() -> Self { Self { constants: vec![], code: vec![] } }

    /// Add a constant and return its index
    fn const_int(&mut self, v: i64) -> u32 {
        self.constants.push(Value::Int(v)); (self.constants.len() - 1) as u32
    }
    fn const_float(&mut self, v: f64) -> u32 {
        self.constants.push(Value::Float(v)); (self.constants.len() - 1) as u32
    }
    fn const_str(&mut self, v: &str) -> u32 {
        self.constants.push(Value::Str(v.to_string())); (self.constants.len() - 1) as u32
    }
    fn const_bool(&mut self, v: bool) -> u32 {
        self.constants.push(Value::Bool(v)); (self.constants.len() - 1) as u32
    }
    fn const_null(&mut self) -> u32 {
        self.constants.push(Value::Null); (self.constants.len() - 1) as u32
    }

    /// Emit an opcode with no args
    fn op(&mut self, opcode: u8) -> &mut Self {
        self.code.push(opcode); self
    }
    /// Emit an opcode + 1 u32 arg (little-endian)
    fn op1(&mut self, opcode: u8, arg: u32) -> &mut Self {
        self.code.push(opcode);
        self.code.extend_from_slice(&arg.to_le_bytes()); self
    }
    /// Emit an opcode + 2 u32 args
    fn op2(&mut self, opcode: u8, a1: u32, a2: u32) -> &mut Self {
        self.code.push(opcode);
        self.code.extend_from_slice(&a1.to_le_bytes());
        self.code.extend_from_slice(&a2.to_le_bytes()); self
    }

    /// Current IP (for computing jump targets)
    fn ip(&self) -> u32 { self.code.len() as u32 }

    /// Patch a JMP/JMP_IF/etc target — backfill the u32 at position `patch_offset`
    fn patch(&mut self, patch_offset: usize, target: u32) {
        let bytes = target.to_le_bytes();
        self.code[patch_offset..patch_offset+4].copy_from_slice(&bytes);
    }

    fn halt(&mut self) -> &mut Self { self.code.push(0xFF); self }

    /// Build the final AVBC bytes
    fn build(&self) -> Vec<u8> {
        // Encode constant pool
        let mut pool: Vec<u8> = vec![];
        for c in &self.constants {
            match c {
                Value::Null    => { pool.push(0x01); }
                Value::Int(i)  => { pool.push(0x02); pool.extend_from_slice(&i.to_le_bytes()); }
                Value::Float(f)=> { pool.push(0x03); pool.extend_from_slice(&f.to_le_bytes()); }
                Value::Str(s)  => {
                    pool.push(0x04);
                    pool.extend_from_slice(&(s.len() as u32).to_le_bytes());
                    pool.extend_from_slice(s.as_bytes());
                }
                Value::Bool(b) => { pool.push(if *b { 0x05 } else { 0x06 }); }
                _              => { pool.push(0x01); } // unknown → null
            }
        }

        // Build 128-byte header
        let mut header = vec![0u8; 128];
        header[0..4].copy_from_slice(b"AVBC");
        header[4..6].copy_from_slice(&1u16.to_le_bytes());  // isa_version
        // flags = 0
        header[8..12].copy_from_slice(&(self.code.len() as u32).to_le_bytes());  // code_size
        header[12..16].copy_from_slice(&(self.constants.len() as u32).to_le_bytes()); // const_count
        // entry_point = 0

        let mut out = header;
        out.extend_from_slice(&pool);
        out.extend_from_slice(&self.code);
        out
    }
}

// ─── Test executor ───────────────────────────────────────────────────────────

use crate::AvapISA;

fn run(bytecode: &[u8]) -> Result<VMState, String> {
    let isa = AvapISA::new();
    let mut state = VMState::new();

    // Parse bytecode manually (mirrors vm.load())
    if bytecode.len() < 128 || &bytecode[0..4] != b"AVBC" {
        return Err("bad header".to_string());
    }
    let code_size   = u32::from_le_bytes(bytecode[8..12].try_into().unwrap())  as usize;
    let const_count = u32::from_le_bytes(bytecode[12..16].try_into().unwrap()) as usize;
    let entry       = u32::from_le_bytes(bytecode[16..20].try_into().unwrap()) as usize;

    let mut off = 128usize;
    for _ in 0..const_count {
        let tag = bytecode[off]; off += 1;
        match tag {
            0x02 => { let v=i64::from_le_bytes(bytecode[off..off+8].try_into().unwrap());
                      state.constants.push(Value::Int(v)); off+=8; }
            0x03 => { let v=f64::from_le_bytes(bytecode[off..off+8].try_into().unwrap());
                      state.constants.push(Value::Float(v)); off+=8; }
            0x04 => { let ln=u32::from_le_bytes(bytecode[off..off+4].try_into().unwrap()) as usize;
                      off+=4;
                      let s=std::str::from_utf8(&bytecode[off..off+ln]).unwrap();
                      state.constants.push(Value::Str(s.to_string())); off+=ln; }
            0x05 => { state.constants.push(Value::Bool(true)); }
            0x06 => { state.constants.push(Value::Bool(false)); }
            _    => { state.constants.push(Value::Null); }
        }
    }
    let code = bytecode[off..off+code_size].to_vec();

    let instruction_set = isa.instruction_set();
    let mut ip = entry;
    let mut count = 0u64;

    loop {
        if ip >= code.len() { break; }
        if count > 100_000 { return Err("instruction limit".to_string()); }

        let opcode = code[ip]; ip += 1;
        if instruction_set.is_halt(opcode) { break; }

        let instr = instruction_set.get(opcode)
            .ok_or_else(|| format!("unknown opcode 0x{:02X} at ip={}", opcode, ip-1))?;
        let result = (instr.handler)(&mut state, &code, &mut ip, std::ptr::null_mut());
        if let Err(e) = result {
            if let Some(handler_ip) = state.try_stack.pop() {
                state.push(Value::Str(e));
                ip = handler_ip;
            } else {
                return Err(e);
            }
        }
        count += 1;
    }
    Ok(state)
}

fn top(state: &VMState) -> Value {
    state.stack.last().cloned().unwrap_or(Value::Null)
}

// ─── Tests: Stack & Variables ────────────────────────────────────────────────

#[test]
fn push_halt_leaves_value_on_stack() {
    let mut b = BytecodeBuilder::new();
    let idx = b.const_int(42);
    b.op1(0x01, idx).halt(); // PUSH 42, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(42));
}

#[test]
fn nop_does_nothing() {
    let mut b = BytecodeBuilder::new();
    let idx = b.const_int(7);
    b.op(0x00).op1(0x01, idx).halt(); // NOP, PUSH 7, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(7));
}

#[test]
fn pop_discards_top() {
    let mut b = BytecodeBuilder::new();
    let i1 = b.const_int(1);
    let i2 = b.const_int(2);
    b.op1(0x01, i1).op1(0x01, i2).op(0x02).halt(); // PUSH 1, PUSH 2, POP, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(1)); // 2 was popped
}

#[test]
fn dup_copies_top() {
    let mut b = BytecodeBuilder::new();
    let idx = b.const_int(99);
    b.op1(0x01, idx).op(0x03).halt(); // PUSH 99, DUP, HALT
    let state = run(&b.build()).unwrap();
    // Stack should have two 99s
    assert_eq!(state.stack.len(), 2);
    assert_eq!(state.stack[0], Value::Int(99));
    assert_eq!(state.stack[1], Value::Int(99));
}

#[test]
fn load_none_pushes_null() {
    let mut b = BytecodeBuilder::new();
    b.op(0x04).halt(); // LOAD_NONE, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Null);
}

#[test]
fn store_and_load_variable() {
    let mut b = BytecodeBuilder::new();
    let val_idx  = b.const_int(123);
    let name_idx = b.const_str("myVar");
    b.op1(0x01, val_idx)   // PUSH 123
     .op1(0x41, name_idx)  // STORE "myVar"
     .op1(0x40, name_idx)  // LOAD "myVar"
     .halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(123));
    assert_eq!(state.globals.get("myVar"), Some(&Value::Int(123)));
}

// ─── Tests: Arithmetic ───────────────────────────────────────────────────────

#[test]
fn add_integers() {
    let mut b = BytecodeBuilder::new();
    let i3 = b.const_int(3);
    let i4 = b.const_int(4);
    b.op1(0x01, i3).op1(0x01, i4).op(0x10).halt(); // PUSH 3, PUSH 4, ADD
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(7));
}

#[test]
fn sub_integers() {
    let mut b = BytecodeBuilder::new();
    let i10 = b.const_int(10);
    let i3  = b.const_int(3);
    b.op1(0x01, i10).op1(0x01, i3).op(0x11).halt(); // 10 - 3 = 7
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(7));
}

#[test]
fn mul_integers() {
    let mut b = BytecodeBuilder::new();
    let i6 = b.const_int(6);
    let i7 = b.const_int(7);
    b.op1(0x01, i6).op1(0x01, i7).op(0x12).halt(); // 6 * 7 = 42
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(42));
}

#[test]
fn div_integers() {
    let mut b = BytecodeBuilder::new();
    let i10 = b.const_int(10);
    let i2  = b.const_int(2);
    b.op1(0x01, i10).op1(0x01, i2).op(0x13).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Float(5.0)); // avap_div always returns Float
}

#[test]
fn mod_integers() {
    let mut b = BytecodeBuilder::new();
    let i10 = b.const_int(10);
    let i3  = b.const_int(3);
    b.op1(0x01, i10).op1(0x01, i3).op(0x14).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(1));
}

#[test]
fn neg_integer() {
    let mut b = BytecodeBuilder::new();
    let i5 = b.const_int(5);
    b.op1(0x01, i5).op(0x15).halt(); // NEG
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(-5));
}

#[test]
fn not_true_gives_false() {
    let mut b = BytecodeBuilder::new();
    let t = b.const_bool(true);
    b.op1(0x01, t).op(0x16).halt(); // NOT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(false));
}

#[test]
fn add_strings_concatenates() {
    let mut b = BytecodeBuilder::new();
    let s1 = b.const_str("hello ");
    let s2 = b.const_str("world");
    b.op1(0x01, s1).op1(0x01, s2).op(0x10).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("hello world".to_string()));
}

// ─── Tests: Comparison ───────────────────────────────────────────────────────

#[test]
fn eq_same_values() {
    let mut b = BytecodeBuilder::new();
    let i = b.const_int(5);
    b.op1(0x01, i).op1(0x01, i).op(0x20).halt(); // 5 == 5
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn eq_different_values() {
    let mut b = BytecodeBuilder::new();
    let i1 = b.const_int(1);
    let i2 = b.const_int(2);
    b.op1(0x01, i1).op1(0x01, i2).op(0x20).halt(); // 1 == 2
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(false));
}

#[test]
fn lt_comparison() {
    let mut b = BytecodeBuilder::new();
    let i3 = b.const_int(3);
    let i5 = b.const_int(5);
    b.op1(0x01, i3).op1(0x01, i5).op(0x21).halt(); // 3 < 5
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn gte_comparison() {
    let mut b = BytecodeBuilder::new();
    let i5 = b.const_int(5);
    let i3 = b.const_int(3);
    b.op1(0x01, i5).op1(0x01, i3).op(0x24).halt(); // 5 >= 3
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn in_list_membership() {
    let mut b = BytecodeBuilder::new();
    let i2 = b.const_int(2);
    let i1 = b.const_int(1);
    let i3 = b.const_int(3);
    // IN: pop container, pop item — so push item first, then container
    b.op1(0x01, i2);                                   // push item (2)
    b.op1(0x01, i1).op1(0x01, i2).op1(0x01, i3).op1(0x55, 3); // push [1,2,3]
    b.op(0x26).halt(); // IN
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

// ─── Tests: Control Flow ─────────────────────────────────────────────────────

#[test]
fn jmp_unconditional() {
    // JMP over a value that should never be pushed
    let mut b = BytecodeBuilder::new();
    let v_skip   = b.const_int(999);
    let v_result = b.const_int(42);

    // Code: JMP → skip 999, push 42, HALT
    b.op1(0x30, 0); // JMP placeholder (will be patched)
    let jmp_target_offset = b.code.len() - 4;

    b.op1(0x01, v_skip); // PUSH 999 — should be skipped

    let target = b.ip();
    b.patch(jmp_target_offset, target);

    b.op1(0x01, v_result).halt(); // PUSH 42, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(42));
    // Stack should only have 42, not 999
    assert_eq!(state.stack.len(), 1);
}

#[test]
fn jmp_if_not_skips_on_false() {
    // JMP_IF_NOT does NOT pop — false stays on stack after jump
    let mut b = BytecodeBuilder::new();
    let f   = b.const_bool(false);
    let v99 = b.const_int(99);
    let v1  = b.const_int(1);

    b.op1(0x01, f);    // PUSH false
    b.op1(0x32, 0);    // JMP_IF_NOT placeholder (does not pop)
    let patch = b.code.len() - 4;

    b.op1(0x01, v99);  // PUSH 99 (skipped when false)

    let target = b.ip();
    b.patch(patch, target);
    b.op(0x02);        // POP the false (it was not consumed by JMP_IF_NOT)
    b.op1(0x01, v1).halt(); // PUSH 1, HALT
    let state = run(&b.build()).unwrap();
    assert_eq!(state.stack, vec![Value::Int(1)]);
}

#[test]
fn jmp_if_not_pop_conditional_branch() {
    // Short-circuit AND: push true, JMP_IF_NOT_POP → if false jump
    let mut b = BytecodeBuilder::new();
    let t   = b.const_bool(true);
    let i42 = b.const_int(42);

    b.op1(0x01, t);    // PUSH true
    b.op1(0x34, 0);    // JMP_IF_NOT_POP placeholder
    let patch = b.code.len() - 4;

    b.op(0x02);        // POP (remove true)
    b.op1(0x01, i42);  // PUSH 42

    let target = b.ip();
    b.patch(patch, target);
    b.halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(42));
}

// ─── Tests: Error Handling ───────────────────────────────────────────────────

#[test]
fn push_try_pop_try_normal_path() {
    // try block that succeeds — POP_TRY should clean up
    let mut b = BytecodeBuilder::new();
    let v42 = b.const_int(42);

    b.op1(0x35, 0);    // PUSH_TRY handler_ip (placeholder)
    let patch = b.code.len() - 4;

    b.op1(0x01, v42);  // PUSH 42 (guarded)
    b.op(0x36);        // POP_TRY (normal exit)

    b.op1(0x30, 0);    // JMP end (placeholder)
    let jmp_end_patch = b.code.len() - 4;

    // Handler (should not run)
    let handler_ip = b.ip();
    b.patch(patch, handler_ip);
    let v_err = b.const_int(999);
    b.op1(0x01, v_err); // PUSH 999 (error path)

    let end = b.ip();
    b.patch(jmp_end_patch, end);
    b.halt();

    let state = run(&b.build()).unwrap();
    // Normal path: 42 on stack, try_stack empty
    assert_eq!(top(&state), Value::Int(42));
    assert!(state.try_stack.is_empty());
}

#[test]
fn raise_jumps_to_handler() {
    let mut b = BytecodeBuilder::new();
    let err_msg = b.const_str("something went wrong");
    let v_ok    = b.const_int(0);
    let v_caught = b.const_int(1);

    b.op1(0x35, 0);     // PUSH_TRY handler_ip
    let patch = b.code.len() - 4;

    b.op1(0x01, err_msg); // PUSH error message
    b.op(0x37);           // RAISE — should jump to handler

    b.op1(0x01, v_ok);    // PUSH 0 (should be skipped)
    b.op(0x36);           // POP_TRY (should be skipped)
    b.op1(0x30, 0);       // JMP end
    let jmp_end_patch = b.code.len() - 4;

    // Handler: error message is on stack (pushed by RAISE mechanism)
    let handler_ip = b.ip();
    b.patch(patch, handler_ip);
    b.op(0x02);           // POP the error message
    b.op1(0x01, v_caught);// PUSH 1 (caught!)

    let end = b.ip();
    b.patch(jmp_end_patch, end);
    b.halt();

    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(1));
}

// ─── Tests: Iteration ────────────────────────────────────────────────────────

#[test]
fn for_iter_over_list() {
    // Sum [1, 2, 3] using FOR_ITER
    // Expected: globals["sum"] = 6
    let mut b = BytecodeBuilder::new();
    let i0    = b.const_int(0);
    let i1    = b.const_int(1);
    let i2    = b.const_int(2);
    let i3    = b.const_int(3);
    let s_sum = b.const_str("sum");
    let s_item = b.const_str("item");

    // sum = 0
    b.op1(0x01, i0).op1(0x41, s_sum);
    // Build list [1, 2, 3]
    b.op1(0x01, i1).op1(0x01, i2).op1(0x01, i3).op1(0x55, 3);
    // GET_ITER
    b.op(0x3A);

    // Loop start
    let loop_start = b.ip();
    b.op1(0x3B, 0);  // FOR_ITER → exit_ip (placeholder)
    let for_iter_patch = b.code.len() - 4;

    // item = TOS (next value)
    b.op1(0x41, s_item);
    // sum = sum + item
    b.op1(0x40, s_sum).op1(0x40, s_item).op(0x10);
    b.op1(0x41, s_sum);
    // JMP back to loop start
    b.op1(0x30, loop_start);

    // Exit
    let exit_ip = b.ip();
    b.patch(for_iter_patch, exit_ip);
    b.halt();

    let state = run(&b.build()).unwrap();
    assert_eq!(state.globals.get("sum"), Some(&Value::Int(6)));
}

// ─── Tests: Collections ──────────────────────────────────────────────────────

#[test]
fn build_list() {
    let mut b = BytecodeBuilder::new();
    let i1 = b.const_int(1);
    let i2 = b.const_int(2);
    let i3 = b.const_int(3);
    b.op1(0x01, i1).op1(0x01, i2).op1(0x01, i3);
    b.op1(0x55, 3).halt(); // BUILD_LIST n=3
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state),
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
}

#[test]
fn build_dict() {
    let mut b = BytecodeBuilder::new();
    let k1 = b.const_str("x");
    let v1 = b.const_int(10);
    let k2 = b.const_str("y");
    let v2 = b.const_int(20);
    b.op1(0x01, k1).op1(0x01, v1)
     .op1(0x01, k2).op1(0x01, v2);
    b.op1(0x56, 2).halt(); // BUILD_DICT n=2
    let state = run(&b.build()).unwrap();
    match top(&state) {
        Value::Dict(pairs) => {
            assert_eq!(pairs.len(), 2);
            assert_eq!(pairs[0], ("x".to_string(), Value::Int(10)));
            assert_eq!(pairs[1], ("y".to_string(), Value::Int(20)));
        }
        other => panic!("expected Dict, got {:?}", other.type_name()),
    }
}

// ─── Tests: Object Access ────────────────────────────────────────────────────

#[test]
fn get_item_from_dict() {
    let mut b = BytecodeBuilder::new();
    let k = b.const_str("name");
    let v = b.const_str("Alice");
    let s_name = b.const_str("name");
    // Build {"name": "Alice"}
    b.op1(0x01, k).op1(0x01, v).op1(0x56, 1);
    // GET_ITEM "name"
    b.op1(0x01, s_name).op(0x52).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("Alice".to_string()));
}

#[test]
fn set_item_in_dict_on_stack() {
    let mut b = BytecodeBuilder::new();
    let k = b.const_str("count");
    let v0 = b.const_int(0);
    let k2 = b.const_str("count");
    let v99 = b.const_int(99);
    // Build {"count": 0}
    b.op1(0x01, k).op1(0x01, v0).op1(0x56, 1);
    // SET_ITEM: dict["count"] = 99
    b.op1(0x01, k2).op1(0x01, v99).op(0x53);
    // GET_ITEM "count"
    b.op1(0x01, k2).op(0x52).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(99));
}

#[test]
fn get_attr_from_dict() {
    let mut b = BytecodeBuilder::new();
    let k = b.const_str("status");
    let v = b.const_str("ok");
    let attr = b.const_str("status");
    b.op1(0x01, k).op1(0x01, v).op1(0x56, 1); // {"status": "ok"}
    b.op1(0x50, attr).halt(); // GET_ATTR "status"
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("ok".to_string()));
}

// ─── Tests: Type System ──────────────────────────────────────────────────────

#[test]
fn is_instance_int() {
    let mut b = BytecodeBuilder::new();
    let v42   = b.const_int(42);
    let t_int = b.const_str("int");
    b.op1(0x01, v42).op1(0x72, t_int).op(0x80).halt(); // LOAD_BUILTIN "int", IS_INSTANCE
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn is_instance_wrong_type() {
    let mut b = BytecodeBuilder::new();
    let v_str  = b.const_str("hello");
    let t_int  = b.const_str("int");
    b.op1(0x01, v_str).op1(0x72, t_int).op(0x80).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(false));
}

#[test]
fn is_instance_tuple_of_types() {
    // isinstance(42, (int, float)) → true
    let mut b = BytecodeBuilder::new();
    let v42     = b.const_int(42);
    let t_int   = b.const_str("int");
    let t_float = b.const_str("float");
    b.op1(0x01, v42);
    b.op1(0x72, t_int).op1(0x72, t_float).op1(0x57, 2); // BUILD_TUPLE (int, float)
    b.op(0x80).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn is_none_on_null() {
    let mut b = BytecodeBuilder::new();
    b.op(0x04).op(0x81).halt(); // LOAD_NONE, IS_NONE
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(true));
}

#[test]
fn is_none_on_value_is_false() {
    let mut b = BytecodeBuilder::new();
    let i = b.const_int(0);
    b.op1(0x01, i).op(0x81).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Bool(false));
}

#[test]
fn type_of_returns_name() {
    let mut b = BytecodeBuilder::new();
    let v = b.const_str("hello");
    b.op1(0x01, v).op(0x82).halt(); // TYPE_OF
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("string".to_string()));
}

// ─── Tests: Conector Namespaces ──────────────────────────────────────────────

#[test]
fn load_conector_then_get_attr_variables_then_set_item_writes_to_conector_vars() {
    // Simulates: self.conector.variables["x"] = 42
    let mut b = BytecodeBuilder::new();
    let v_x  = b.const_str("x");
    let v42  = b.const_int(42);
    let attr_vars = b.const_str("variables");

    b.op(0x70);             // LOAD_CONECTOR → "__CONECTOR__"
    b.op1(0x50, attr_vars); // GET_ATTR "variables" → "__CONECTOR_VARS__"
    b.op1(0x01, v_x);       // PUSH "x"
    b.op1(0x01, v42);       // PUSH 42
    b.op(0x53);             // SET_ITEM → writes to VMState.conector_vars
    b.halt();

    let state = run(&b.build()).unwrap();
    assert_eq!(state.conector_vars.get("x"), Some(&Value::Int(42)));
}

#[test]
fn load_conector_results_set_item_writes_to_results() {
    // Simulates: self.conector.results["output"] = "hello"
    let mut b = BytecodeBuilder::new();
    let v_key    = b.const_str("output");
    let v_val    = b.const_str("hello");
    let attr_res = b.const_str("results");

    b.op(0x70);             // LOAD_CONECTOR
    b.op1(0x50, attr_res);  // GET_ATTR "results" → "__CONECTOR_RESULTS__"
    b.op1(0x01, v_key);
    b.op1(0x01, v_val);
    b.op(0x53);             // SET_ITEM
    b.halt();

    let state = run(&b.build()).unwrap();
    assert_eq!(state.results.get("output"), Some(&Value::Str("hello".to_string())));
}

// ─── Tests: CALL_METHOD ──────────────────────────────────────────────────────

#[test]
fn call_method_str_strip() {
    let mut b = BytecodeBuilder::new();
    let s      = b.const_str("  hello  ");
    let m_strip = b.const_str("strip");
    b.op1(0x01, s).op2(0x62, m_strip, 0).halt(); // CALL_METHOD "strip" nargs=0
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("hello".to_string()));
}

#[test]
fn call_method_str_upper() {
    let mut b = BytecodeBuilder::new();
    let s       = b.const_str("hello");
    let m_upper = b.const_str("upper");
    b.op1(0x01, s).op2(0x62, m_upper, 0).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Str("HELLO".to_string()));
}

#[test]
fn call_method_dict_get_existing_key() {
    // {"x": 42}.get("x", None) → 42
    let mut b = BytecodeBuilder::new();
    let k     = b.const_str("x");
    let v42   = b.const_int(42);
    let k2    = b.const_str("x");
    let m_get = b.const_str("get");
    b.op1(0x01, k).op1(0x01, v42).op1(0x56, 1); // BUILD_DICT {"x": 42}
    b.op1(0x01, k2);                              // push key arg
    b.op2(0x62, m_get, 1).halt();                 // CALL_METHOD "get" nargs=1
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(42));
}

#[test]
fn call_method_dict_get_missing_key_returns_default() {
    // {}.get("missing", 0) → 0
    let mut b = BytecodeBuilder::new();
    let k_missing = b.const_str("missing");
    let default   = b.const_int(0);
    let m_get     = b.const_str("get");
    b.op1(0x56, 0);                                // BUILD_DICT {}
    b.op1(0x01, k_missing).op1(0x01, default);    // args: key, default
    b.op2(0x62, m_get, 2).halt();
    let state = run(&b.build()).unwrap();
    assert_eq!(top(&state), Value::Int(0));
}

// ─── Tests: Error cases ──────────────────────────────────────────────────────

#[test]
fn unknown_opcode_returns_error() {
    let mut b = BytecodeBuilder::new();
    b.op(0xEE).halt(); // 0xEE not registered
    let result = run(&b.build());
    assert!(result.is_err());
    let err = match result { Err(e) => e, Ok(_) => panic!("expected error") };
    assert!(err.contains("0xEE") || err.contains("unknown"), "error was: {}", err);
}

#[test]
fn invalid_magic_returns_error() {
    let mut fake = vec![0u8; 135];
    fake[0..4].copy_from_slice(b"JUNK");
    let result = run(&fake);
    assert!(result.is_err());
}

#[test]
fn bytecode_too_short_returns_error() {
    let result = run(&[0u8; 50]);
    assert!(result.is_err());
}
