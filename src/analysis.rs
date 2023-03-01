use either;
use ena::unify::{EqUnifyValue, UnifyKey};
use llvm_ir::{instruction::Call, Instruction, Module, Name, Operand, TypeRef};
use std::{collections::HashMap, fmt};

use crate::framework::run_function_pass;
use crate::framework::FunctionPass;
use crate::lifetimes::FunctionLifetimes;
use crate::lifetimes::Lifetime;
use crate::lifetimes::LifetimeCtx;
use crate::utilities::dereference_type;
use llvm_ir::Function;

const MALLOC: &str = "malloc";
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
    pub results: HashMap<String, FunctionLifetimes>,
}

#[derive(Default)]
pub struct IntraLifetimeAnalysis {
    _typ_ctx: HashMap<Name, TypeRef>,
    pub lt_ctx: LifetimeCtx,
}

impl FunctionPass<FunctionLifetimes> for IntraLifetimeAnalysis {

    fn init_param(&mut self, param: & llvm_ir::function::Parameter) {
        let lifetimes = self.lt_ctx.register_parameter_lifetimes(&param);
        for (l, r) in lifetimes.iter().zip(lifetimes.iter()) {
            self.lt_ctx.generate_outlives(*l, *r);
        }
    }

    fn on_completion(&mut self, func: & Function) -> FunctionLifetimes {
        self.lt_ctx.finalize(func)
    }

    fn transfer(&mut self, inst: & Instruction) {
        match inst { 
            Instruction::Alloca(_) => (),
            Instruction::Load(load) => match &load.address {
                Operand::LocalOperand { name, ty } => {
                    let source_lts = self.lt_ctx.register_lifetimes(name, &ty);
                    if let Some(dest_ty) = &dereference_type(&ty) {
                        let dest_lts = self.lt_ctx.register_lifetimes(&load.dest, dest_ty);
                        if let [_local, _, tail @ ..] = &source_lts[..] {
                            let constraints: Vec<(&Lifetime, &Lifetime)> =
                                tail.iter().zip(dest_lts.iter().skip(1)).collect();

                            if let [(l, r), tl @ ..] = &constraints[..] {
                                self.lt_ctx.generate_outlives(**l, **r);
                                tl.iter().for_each(|(l, r)| {
                                    self.lt_ctx.generate_equality(**l, **r);
                                });
                            }
                        }
                    }
                }
                Operand::ConstantOperand(_) => todo!(),
                Operand::MetadataOperand => todo!(),
            },
            Instruction::Store(st) => {
                let source_lts = match &st.value {
                    llvm_ir::Operand::LocalOperand { name, ty } => {
                        self.lt_ctx.register_lifetimes(name, &ty)
                    }
                    llvm_ir::Operand::MetadataOperand => vec![],
                    llvm_ir::Operand::ConstantOperand(_) => return,
                };
                let dest_lts = match &st.address {
                    llvm_ir::Operand::LocalOperand { name, ty } => {
                        self.lt_ctx.register_lifetimes(name, &ty)
                    }
                    llvm_ir::Operand::MetadataOperand => vec![],
                    llvm_ir::Operand::ConstantOperand(_) => todo!(),
                };
                if let [_, dest_tail @ ..] = &dest_lts[..] {
                    let mut constrained: Vec<(&Lifetime, &Lifetime)> =
                        source_lts.iter().zip(dest_tail.iter()).collect();
                    if let [(l, r), tail @ ..] = &mut *constrained {
                        self.lt_ctx.generate_outlives(**l, **r);
                        for (tl, tr) in tail.iter() {
                            self.lt_ctx.generate_equality(**tl, **tr);
                        }
                    }
                }
            }
            Instruction::BitCast(bc) => set_equal(self, &bc.operand, &bc.dest),

            Instruction::Call(cl) => match &cl.function {
                either::Either::Left(_) => {
                    todo!()
                }
                either::Either::Right(func) => transfer_call(self, &cl, &func),
            },
            _ => return,
        }
    }
}

fn set_equal(lifetimes: &mut IntraLifetimeAnalysis, source: &  Operand, dest: &  Name)  {
    match &*source {
        Operand::LocalOperand { name, ty } => {
            let source_lts = lifetimes.lt_ctx.register_lifetimes(&name, &ty);
            let dest_lts = lifetimes.lt_ctx.register_lifetimes(&dest, &ty);
            source_lts
                .iter()
                .zip(dest_lts.iter())
                .for_each(|(left, right)| lifetimes.lt_ctx.generate_equality(*left, *right));
        }
        Operand::ConstantOperand(_) => todo!(),
        Operand::MetadataOperand => todo!(),
    }
}

impl ProgramLifetimes{
    pub fn new(module: & Module) -> Self {
        let mut program_lifetimes = ProgramLifetimes {
            results: HashMap::new(),
        };
        for a in module.functions.iter() {
            let function_name = a.name.to_owned();
            let mut analysis = IntraLifetimeAnalysis::default();
            let results = run_function_pass(&mut analysis, &a);
            program_lifetimes.results.insert(function_name, results);
        }
        program_lifetimes
    }
}

fn transfer_call(_lifetimes: &mut IntraLifetimeAnalysis, cl: &Call, func: &Operand) {
    match func {
        Operand::LocalOperand { name: _, ty: _ } => todo!(),
        Operand::ConstantOperand(cr) => match cr.as_ref() {
            llvm_ir::Constant::GlobalReference { name, ty: _ } => match name.to_string().as_str() {
                MALLOC => match &cl.dest {
                    Some(_nm) => {}
                    None => {
                        todo!();
                    }
                },
                _ => return,
            },
            _ => todo!(),
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

