use llvm_ir::{function::Parameter, Function, Instruction};

pub trait FunctionPass<T> {
    fn init_param(&mut self, param: &Parameter);
    fn transfer(&mut self, inst: &Instruction);
    fn on_completion(&mut self, func: &Function) -> T;
}

pub fn run_function_pass<T>(state: &mut dyn FunctionPass<T>, func: &Function) -> T {
    for param in &func.parameters {
        state.init_param(&param)
    }
    for block in func.basic_blocks.iter() {
        for inst in block.instrs.iter() {
            state.transfer(&inst);
        }
    }
    state.on_completion(func)
}
pub trait ProgramPass {}
