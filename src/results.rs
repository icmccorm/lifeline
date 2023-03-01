use crate::analysis::ProgramLifetimes;
use llvm_ir::Module;

pub fn pretty_print_module(pl: &mut ProgramLifetimes, module: &Module) -> String {
    module
        .functions
        .iter()
        .map(|func| {
            let function_lifetimes = pl.results.get_mut(&func.name);
            match function_lifetimes {
                Some(f) => f.to_string(),
                None => format!("Unable to generate lifetimes for function {}", func.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
