// Auto-generated from opcodes.json — do not edit manually.
// ISA: AVAP-ISA v0.1.0

pub mod op {
    /// No operation
    pub const NOP: u8 = 0x00;
    /// Push constant[const_idx] onto stack
    pub const PUSH: u8 = 0x01;
    /// Discard top of stack
    pub const POP: u8 = 0x02;
    /// Duplicate top of stack
    pub const DUP: u8 = 0x03;
    /// Push Null onto stack
    pub const LOAD_NONE: u8 = 0x04;
    /// pop b, pop a, push a+b
    pub const ADD: u8 = 0x10;
    /// pop b, pop a, push a-b
    pub const SUB: u8 = 0x11;
    /// pop b, pop a, push a*b
    pub const MUL: u8 = 0x12;
    /// pop b, pop a, push a/b
    pub const DIV: u8 = 0x13;
    /// pop b, pop a, push a%b
    pub const MOD: u8 = 0x14;
    /// pop a, push -a
    pub const NEG: u8 = 0x15;
    /// pop a, push !a
    pub const NOT: u8 = 0x16;
    /// pop b, pop a, push a==b
    pub const EQ: u8 = 0x20;
    /// pop b, pop a, push a<b
    pub const LT: u8 = 0x21;
    /// pop b, pop a, push a>b
    pub const GT: u8 = 0x22;
    /// pop b, pop a, push a<=b
    pub const LTE: u8 = 0x23;
    /// pop b, pop a, push a>=b
    pub const GTE: u8 = 0x24;
    /// pop b, pop a, push a!=b
    pub const NEQ: u8 = 0x25;
    /// pop container, pop item, push item in container
    pub const IN: u8 = 0x26;
    /// pop container, pop item, push item not in container
    pub const NOT_IN: u8 = 0x27;
    /// pop b, pop a, push a is b
    pub const IS: u8 = 0x28;
    /// pop b, pop a, push a is not b
    pub const IS_NOT: u8 = 0x29;
    /// pop b, pop a, push a and b
    pub const BOOL_AND: u8 = 0x2A;
    /// pop b, pop a, push a or b
    pub const BOOL_OR: u8 = 0x2B;
    /// Unconditional jump to target_ip
    pub const JMP: u8 = 0x30;
    /// Jump if top is truthy (no pop)
    pub const JMP_IF: u8 = 0x31;
    /// Jump if top is falsy (no pop)
    pub const JMP_IF_NOT: u8 = 0x32;
    /// Jump if top is truthy and pop
    pub const JMP_IF_POP: u8 = 0x33;
    /// Jump if top is falsy and pop
    pub const JMP_IF_NOT_POP: u8 = 0x34;
    /// Push exception handler at handler_ip
    pub const PUSH_TRY: u8 = 0x35;
    /// Pop exception handler
    pub const POP_TRY: u8 = 0x36;
    /// Raise top of stack as exception
    pub const RAISE: u8 = 0x37;
    /// Return top of stack from function
    pub const RETURN: u8 = 0x38;
    /// Pop iterable, push iterator
    pub const GET_ITER: u8 = 0x3A;
    /// Advance iterator; push next or jump to exit_ip
    pub const FOR_ITER: u8 = 0x3B;
    /// Push globals[name_idx] onto stack
    pub const LOAD: u8 = 0x40;
    /// Pop and store into globals[name_idx]
    pub const STORE: u8 = 0x41;
    /// Pop obj, push obj.name_idx
    pub const GET_ATTR: u8 = 0x50;
    /// Pop value, set obj.name_idx = value
    pub const SET_ATTR: u8 = 0x51;
    /// Pop key, pop obj, push obj[key]
    pub const GET_ITEM: u8 = 0x52;
    /// Pop value, pop key, set obj[key] = value
    pub const SET_ITEM: u8 = 0x53;
    /// Pop key, pop obj, del obj[key]
    pub const DELETE_ITEM: u8 = 0x54;
    /// Pop n items, push [item0..itemN]
    pub const BUILD_LIST: u8 = 0x55;
    /// Pop 2n items (k,v pairs), push {k:v...}
    pub const BUILD_DICT: u8 = 0x56;
    /// Pop n items, push (item0..itemN)
    pub const BUILD_TUPLE: u8 = 0x57;
    /// Call registered native function by func_id
    pub const CALL_EXT: u8 = 0x60;
    /// Pop n_args + callable, call callable(*args)
    pub const CALL_FUNC: u8 = 0x61;
    /// Pop n_args + obj, call obj.name(*args)
    pub const CALL_METHOD: u8 = 0x62;
    /// Push runtime.conector onto stack
    pub const LOAD_CONECTOR: u8 = 0x70;
    /// Push runtime.task onto stack
    pub const LOAD_TASK: u8 = 0x71;
    /// Push builtin function by name_idx
    pub const LOAD_BUILTIN: u8 = 0x72;
    /// Pop type, pop obj, push isinstance(obj, type)
    pub const IS_INSTANCE: u8 = 0x80;
    /// Pop obj, push obj is None
    pub const IS_NONE: u8 = 0x81;
    /// Pop obj, push type name string
    pub const TYPE_OF: u8 = 0x82;
    /// Push module proxy by name_idx
    pub const IMPORT_MOD: u8 = 0x90;
    /// Stop execution
    pub const HALT: u8 = 0xFF;
}

pub mod isa_meta {
    pub const NAME:    &str = "AVAP-ISA";
    pub const VERSION: (u8,u8,u8) = (0,1,0);
}