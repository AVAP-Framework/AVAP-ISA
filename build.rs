// build.rs — runs at cargo compile time.
// Reads opcodes.json and generates src/generated_opcodes.rs
// with pub const definitions for every instruction.

use std::fs;
use std::path::Path;
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let json_path = Path::new(&manifest_dir).join("opcodes.json");
    let out_dir   = std::env::var("OUT_DIR").unwrap();
    let out_path  = Path::new(&out_dir).join("opcodes.rs");

    // Tell cargo to re-run this script if opcodes.json changes
    println!("cargo:rerun-if-changed=opcodes.json");

    let json_str = fs::read_to_string(&json_path)
        .expect("opcodes.json not found — it must be in the avap-isa root directory");

    // Minimal JSON parser — we only need the "instructions" object
    // Use serde_json if available, otherwise parse manually
    let data: serde_json::Value = serde_json::from_str(&json_str)
        .expect("opcodes.json is not valid JSON");

    let name    = data["name"].as_str().unwrap_or("unknown");
    let version = data["version"].as_array().unwrap();
    let v0 = version[0].as_u64().unwrap_or(0);
    let v1 = version[1].as_u64().unwrap_or(0);
    let v2 = version[2].as_u64().unwrap_or(0);

    let instructions = data["instructions"].as_object()
        .expect("opcodes.json must have an 'instructions' object");

    let mut lines = vec![
"// Auto-generated from opcodes.json — do not edit manually.".to_string(),
format!("// ISA: {} v{}.{}.{}", name, v0, v1, v2),
        String::new(),
        "pub mod op {".to_string(),
    ];

    // Sort by opcode value for readability
    let mut sorted: Vec<(&String, &serde_json::Value)> = instructions.iter().collect();
    sorted.sort_by_key(|(_, v)| v["opcode"].as_u64().unwrap_or(0));

    for (instr_name, info) in &sorted {
        let opcode = info["opcode"].as_u64().unwrap_or(0) as u8;
        let desc   = info["description"].as_str().unwrap_or("");
        lines.push(format!("    /// {}", desc));
        lines.push(format!("    pub const {}: u8 = 0x{:02X};", instr_name, opcode));
    }

    lines.push("}".to_string());
    lines.push(String::new());

    // Also generate ISA metadata constants
    lines.push("pub mod isa_meta {".to_string());
    lines.push(format!("    pub const NAME:    &str = {:?};", name));
    lines.push(format!("    pub const VERSION: (u8,u8,u8) = ({},{},{});", v0, v1, v2));
    lines.push("}".to_string());

    fs::write(&out_path, lines.join("\n"))
        .expect("Failed to write generated opcodes.rs");
}
