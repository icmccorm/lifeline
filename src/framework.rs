use llvm_ir::{function::Parameter, Function, Instruction};

pub trait Analysis {
    fn init_param(&mut self, param: &Parameter);
    fn transfer(&mut self, inst: &Instruction);
    fn on_completion(&mut self);
}
pub fn run_analysis<'a>(state: &mut dyn Analysis, func: &'a Function) {
    for param in &func.parameters {
        state.init_param(&param)
    }
    for block in func.basic_blocks.iter() {
        for inst in block.instrs.iter() {
            state.transfer(&inst);
        }
    }
    state.on_completion()
}
