use lifeline;

use lifeline::analysis::ProgramLifetimes;
use llvm_ir::Module;
use std::env::temp_dir;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;

fn run_inference(str: &str) -> String {
    let mut tempfile = temp_file();
    tempfile.1.write_all(str.as_bytes()).unwrap();
    tempfile.1.flush().unwrap();
    let bc_path = format!("{}.bc", tempfile.0.clone());
    let bytecode_compilation_succeeded = Command::new("clang")
        .args(["-emit-llvm", "-o", bc_path.as_str(), "-c", &tempfile.0])
        .output()
        .is_ok();
    if bytecode_compilation_succeeded {
        fs::remove_file(tempfile.0).unwrap();
        let module = Module::from_bc_path(bc_path.as_str()).unwrap();
        fs::remove_file(bc_path.as_str()).unwrap();
        let mut lifetimes: ProgramLifetimes = ProgramLifetimes::new(&module);
        lifeline::results::pretty_print_module(&mut lifetimes, &module)
    } else {
        eprintln!("Failed to compile test binary.");
        std::process::exit(1);
    }
}

fn temp_file() -> (String, File) {
    let file_name = format!("{}.c", uuid::Uuid::new_v4());
    let file_ref = File::create(file_name.as_str()).unwrap();
    println!("{}", file_name);
    (file_name, file_ref)
}

fn assert_lt(c_function: &str, contract: &str) {
    let compiled_contract = run_inference(&c_function);
    assert!(compiled_contract == contract);
}

#[test]
fn test_single_ptr() {
    assert_lt(
        "
    void test(int * a) {
        int get = *a;
    };
    ",
        "('a)",
    );
}

#[test]
fn test_double_ptr() {
    assert_lt(
        "
    void test(int * * a) {
        int * get = *a;
    };
    ",
        "('a, 'b)",
    );
}
