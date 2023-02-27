use lifeline::analysis::*;
use lifeline::*;
use llvm_ir::Module;

fn main() {
    let bytecode_path_opt = std::env::args().nth(1);
    match bytecode_path_opt {
        Some(bytecode_path) => {
            let opt_module = Module::from_bc_path(bytecode_path);
            match opt_module {
                Ok(module) => {
                    let mut lifetimes: ProgramLifetimes = ProgramLifetimes::new(&module);
                    print!("{}", results::pretty_print_module(&mut lifetimes, &module));
                }
                Err(msg) => {
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }
            }
        }
        None => eprintln!("Usage: lifeline [compiled.bc]"),
    }
}
