mod analysis;
use llvm_ir::{Module};
use analysis::*;

fn main() {
    let bytecode_path_opt = std::env::args().nth(1);
    match bytecode_path_opt {
        Some(bytecode_path) => {
            let opt_module = Module::from_bc_path(bytecode_path);
            match opt_module {
                Ok(module) => {
                    let lifetimes:ProgramLifetimes = ProgramLifetimes::new(&module);
                    println!("{}", lifetimes);
                }
                Err(msg) => {
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }
            }
        },
        None => eprintln!("Usage: lifeline [compiled.bc]"),
    }
}