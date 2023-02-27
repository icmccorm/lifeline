use ena::unify::{UnifyKey, EqUnifyValue};
use llvm_ir::{Instruction, Module, Name, TypeRef, Operand, instruction::Call};
use std::{collections::HashMap, fmt};
use either;

use crate::framework::Analysis;
use crate::framework::run_analysis;
use crate::lifetimes::LifetimeCtx;

const MALLOC:&str = "malloc";
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct IntKey(u32);
impl UnifyKey for IntKey {
    type Value = Option<IntKey>;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(u: u32) -> IntKey {
        IntKey(u)
    }
    fn tag() -> &'static str {
        "IntKey"
    }

    fn order_roots(
        _a: Self,
        _a_value: &Self::Value,
        _b: Self,
        _b_value: &Self::Value,
    ) -> Option<(Self, Self)> {
        None
    }
}
impl EqUnifyValue for IntKey {}

pub struct ProgramLifetimes {
    pub results: HashMap<String, IntraLifetimeAnalysis>,
}

#[derive(Default)]
pub struct IntraLifetimeAnalysis {
    _typ_ctx: HashMap<Name, TypeRef>,
    pub lt_ctx: LifetimeCtx
}

impl Analysis for IntraLifetimeAnalysis {
    fn init_param(&mut self, param: & llvm_ir::function::Parameter) {
        self.lt_ctx.register_parameter_lifetimes(&param);
    }

    fn on_completion(&mut self) {
        self.lt_ctx.finalize()
    }   

    fn transfer(&mut self, inst: &Instruction) {
        match inst {
            Instruction::Alloca(_) => (),
            Instruction::Load(load) => set_equal(self, &load.address, &load.dest),
            Instruction::Store(st) => {
                let source_lts = match &st.value {
                    llvm_ir::Operand::LocalOperand { name, ty } => self.lt_ctx.register_lifetimes(&name, &ty),
                    llvm_ir::Operand::MetadataOperand => vec![],
                    llvm_ir::Operand::ConstantOperand(_) => todo!(),
                };
                let dest_lts = match &st.address {
                    llvm_ir::Operand::LocalOperand { name, ty } => self.lt_ctx.register_lifetimes(&name, &ty),
                    llvm_ir::Operand::MetadataOperand => vec![],
                    llvm_ir::Operand::ConstantOperand(_) => todo!(),
                };

                match &dest_lts[..] {
                    [_, tail @ ..] => {
                        let mut constrained:Vec<(&u32, &u32)> = source_lts.iter().zip(tail.iter()).collect();
                        match & mut *constrained {
                            [(l,r), tail @ ..] => {
                                self.lt_ctx.generate_outlives(**l, **r);
                                for (tl, tr) in tail.iter() {
                                    self.lt_ctx.generate_equality(**tl, **tr);
                                }
                            },
                            _ => ()
                        }
                    },
                    _ => ()
                }          
            }
            Instruction::BitCast(bc) => set_equal(self, &bc.operand, &bc.dest),
            Instruction::Call(cl) => {
                match &cl.function {
                    either::Either::Left(_) => { todo!() },
                    either::Either::Right(func) => transfer_call(self, &cl, &func)
    
                }
            },
            _ => todo!(),
        }
    }
}

fn set_equal(lifetimes: &mut IntraLifetimeAnalysis, source: &Operand, dest: &Name) {
    match &*source{
        Operand::LocalOperand { name, ty } => {
            let source_lts = lifetimes.lt_ctx.register_lifetimes(&name, &ty);
            let dest_lts = lifetimes.lt_ctx.register_lifetimes(dest, &ty);
            source_lts.iter().zip(dest_lts.iter()).for_each(|(left, right)| {
                lifetimes.lt_ctx.generate_equality(*left, *right)
            });
        },
        Operand::ConstantOperand(_) => todo!(),
        Operand::MetadataOperand => todo!(),
    }
}

impl ProgramLifetimes {
    pub fn new(module: &Module) -> Self {
        let mut program_lifetimes = ProgramLifetimes {
            results: HashMap::new(),
        };
        for a in module.functions.iter() {
            let function_name = a.name.to_owned();
            let mut analysis = IntraLifetimeAnalysis::default();
            run_analysis(& mut analysis, &a);
            program_lifetimes.results.insert(function_name, analysis);
        }
        program_lifetimes
    }
}

fn transfer_call(_lifetimes: &mut IntraLifetimeAnalysis, cl: &Call, func: &Operand){
    match func {
        Operand::LocalOperand { name:_, ty:_ } => todo!(),
        Operand::ConstantOperand(cr) => {
            match cr.as_ref() {
                llvm_ir::Constant::GlobalReference { name, ty:_ } => {
                    match name.to_string().as_str() {
                        MALLOC => {
                            match &cl.dest {
                                Some(_nm) => {
          
                                }
                                None => {
                                    todo!();
                                }
                            }
                        },
                        _ => {
                            todo!()
                        }
                    }
                },
                _ => todo!(),
            }
        },
        Operand::MetadataOperand => todo!(),
    }
}

impl fmt::Display for IntraLifetimeAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.lt_ctx)
    }
}

impl fmt::Display for ProgramLifetimes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let res = self.results.iter().fold("".to_string(), |acc, res| {

            acc + format!("{}: {}\r\n", res.0, res.1).as_str()
        });
        write!(f, "{}", res)
    }
}