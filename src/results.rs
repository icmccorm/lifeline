use crate::analysis::{IntraLifetimeAnalysis, ProgramLifetimes};
use crate::utilities::usize_to_string;
use llvm_ir::{Function, Module /*Name, TypeRef*/};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub fn pretty_print_module(pl: &mut ProgramLifetimes, module: &Module) -> String {
    module
        .functions
        .iter()
        .map(|func| {
            let function_lifetimes = pl.results.get_mut(&func.name);
            match function_lifetimes {
                Some(mut f) => pretty_print_function(&mut f, &func),
                None => "".to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
pub fn pretty_print_function(ila: &mut IntraLifetimeAnalysis, func: &Function) -> String {
    let mut lifetime_counter: u32 = 0;
    let mut printed_annotations = HashMap::<u32, u32>::default();

    let mut get_or_increment = |id: u32| {
        let num_rep = match printed_annotations.entry(id) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                v.insert(lifetime_counter);
                lifetime_counter += 1;
                lifetime_counter - 1
            }
        };
        format!("'{}", usize_to_string(num_rep as usize))
    };

    let params = func
        .parameters
        .iter()
        .map(|param| {
            let solved_lifetimes = ila.lt_ctx.get_solved_parameter_lifetimes(&param);

            let str_rep = &solved_lifetimes[1..]
                .iter()
                .map(|solved_id| get_or_increment(*solved_id))
                .collect::<Vec<_>>()
                .join(", ");

            format!("({})", str_rep)
        })
        .collect::<Vec<_>>()
        .join(" -> ");

    let raw_constraints = ila.lt_ctx.get_constraints();
    if raw_constraints.len() > 0 {
        let constraints = raw_constraints
            .iter()
            .map(|(l, r)| {
                let num_l = get_or_increment(*l);
                let num_r = get_or_increment(*r);
                format!("{} >= {}", num_l, num_r)
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} where {}", params, constraints)
    } else {
        params
    }
}
