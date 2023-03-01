use crate::dsu::DSU;
use crate::utilities::usize_to_string;
use llvm_ir::Function;
use llvm_ir::{function::Parameter, Name, TypeRef};
use petgraph::algo::tarjan_scc;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{prelude::StableDiGraph, Direction::Outgoing};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
};

pub type Indirection = u32;
pub type LifetimeID = u32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Lifetime {
    #[default]
    Local,
    Heap,
    Var(LifetimeID),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct FunctionLifetimes {
    parameters: Vec<Vec<Lifetime>>,
    return_variable: Vec<Lifetime>,
    constraints: Vec<(Lifetime, Lifetime)>,
}

impl fmt::Display for FunctionLifetimes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut lifetime_counter: u32 = 0;
        let mut printed_annotations = HashMap::<u32, String>::default();

        let mut get_or_increment = |lt| {
            let str_rep = match lt {
                Lifetime::Var(id) => match printed_annotations.entry(id) {
                    Entry::Occupied(o) => {
                        let curr_contents: String = o.get().clone();
                        curr_contents
                    }
                    Entry::Vacant(v) => {
                        let str_rep: String = usize_to_string(lifetime_counter as usize);
                        v.insert(str_rep.to_string());
                        lifetime_counter += 1;
                        str_rep
                    }
                },
                _ => lt.to_string(),
            };
            format!("'{}", str_rep)
        };

        let str_rep_params = self
            .parameters
            .iter()
            .map(|param| {
                let str_rep = param
                    .iter()
                    .skip(1)
                    .map(|lt| get_or_increment(*lt))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", str_rep)
            })
            .collect::<Vec<_>>()
            .join(" -> ");

        if self.constraints.len() > 0 {
            let str_rep_constraints = self
                .constraints
                .iter()
                .map(|(l, r)| {
                    let num_l = get_or_increment(*l);
                    let num_r = get_or_increment(*r);
                    format!("{} >= {}", num_l, num_r)
                })
                .collect::<Vec<_>>()
                .join(", ");

            write!(f, "{} where {}", str_rep_params, str_rep_constraints)
        } else {
            write!(f, "{}", str_rep_params)
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AbstractLocation {
    name: llvm_ir::Name,
    indirection: u32,
}

impl fmt::Display for AbstractLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.indirection)
    }
}

pub struct LifetimeCtx {
    lifetime_counter: u32,
    pub lifetime_ids: HashMap<AbstractLocation, Lifetime>,
    pub constraints: StableDiGraph<Lifetime, ()>,
    pub unification: DSU<Lifetime>,
    pub lifetime_node_indices: HashMap<Lifetime, NodeIndex>,
}

impl Default for LifetimeCtx {
    fn default() -> Self {
        LifetimeCtx {
            lifetime_counter: 0,
            lifetime_ids: HashMap::default(),
            constraints: StableGraph::default(),
            unification: DSU::default(),
            lifetime_node_indices: HashMap::default(),
        }
    }
}

impl<'a> LifetimeCtx {
    fn register(&mut self, lt: AbstractLocation) -> Lifetime {
        let registered_lifetime = if lt.indirection == 0 {
            Lifetime::Local
        } else {
            match self.lifetime_ids.entry(lt) {
                Entry::Occupied(o) => *o.get(),
                Entry::Vacant(v) => {
                    self.lifetime_counter += 1;
                    *v.insert(Lifetime::Var(self.lifetime_counter))
                }
            }
        };
        self.unification.add(registered_lifetime)
    }

    pub fn get_lifetimes(&mut self, name: &Name, ty: &TypeRef) -> Vec<Lifetime> {
        self.recurse_lifetimes(name, ty, 0)
    }

    fn recurse_lifetimes(&mut self, name: &Name, ty: &TypeRef, indirection: u32) -> Vec<Lifetime> {
        let curr_loc = AbstractLocation {
            name: name.to_owned(),
            indirection,
        };
        let curr_lifetime = vec![self.register(curr_loc)];

        match ty.as_ref() {
            llvm_ir::Type::IntegerType { bits: _ } => curr_lifetime,
            llvm_ir::Type::PointerType {
                pointee_type,
                addr_space: _,
            } => vec![
                curr_lifetime,
                self.recurse_lifetimes(name, pointee_type, indirection + 1),
            ]
            .concat(),
            _ => todo!(),
        }
    }
    pub fn register_parameter_lifetimes(&mut self, param: &Parameter) -> Vec<Lifetime> {
        self.register_lifetimes(&param.name, &param.ty)
    }

    pub fn register_lifetimes(&mut self, name: &Name, ty: &TypeRef) -> Vec<Lifetime> {
        self.get_lifetimes(name, ty)
    }

    pub fn generate_equality(&mut self, left: Lifetime, right: Lifetime) {
        self.unification.union(left, right);
    }

    fn lifetime_node_index(&mut self, id: Lifetime) -> NodeIndex {
        match self.lifetime_node_indices.entry(id) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let new_node_index = self.constraints.add_node(id);
                v.insert(new_node_index);
                new_node_index
            }
        }
    }

    pub fn generate_outlives(&mut self, left: Lifetime, right: Lifetime) {
        if left != right {
            let shorter = self.lifetime_node_index(left);
            let longer = self.lifetime_node_index(right);
            self.constraints.add_edge(longer, shorter, ());
        }
    }

    pub fn finalize(&mut self, func: &Function) -> FunctionLifetimes {
        let components = tarjan_scc(&self.constraints);
        for component in components.iter() {
            let ids: Vec<Lifetime> = component
                .iter()
                .map(|n| *self.constraints.node_weight(*n).unwrap())
                .collect();

            if let [head, tail @ ..] = &*ids {
                for elem in tail {
                    self.generate_equality(*head, *elem)
                }
            }
        }

        let parameters = func
            .parameters
            .iter()
            .map(|p| self.get_solved_parameter_lifetimes(&p))
            .collect();

        // TODO: Fix constraints generation.
        // Have it take a set of parameter lifetimes from 'parameters' above 
        // and simplify constraints to only contain those lifetimes.

        let _constraints = self.get_constraints();

        FunctionLifetimes {
            parameters: parameters,
            return_variable: vec![],
            constraints: vec![],
        }
    }

    fn get_constraints(&self) -> Vec<(Lifetime, Lifetime)> {

        let lifetime_id_pairs: Vec<(Lifetime, Lifetime)> = self
            .constraints
            .node_indices()
            .map(|idx| {
                self.constraints
                    .neighbors_directed(idx, Outgoing)
                    .map(|neighbor_idx| {
                        (
                            *self.constraints.node_weight(idx).unwrap(),
                            *self.constraints.node_weight(neighbor_idx).unwrap(),
                        )
                    })
                    .collect()
            })

            .fold(vec![], |mut acc, v: Vec<(Lifetime, Lifetime)>| {
                acc.extend(v.iter());
                acc
            });

        let unification_id_pairs: Vec<(Lifetime, Lifetime)> = lifetime_id_pairs
            .iter()
            .map(|(l, r)| (self.unification.find(*l), self.unification.find(*r)))
            //TODO: ensure that constraints involving 'local are eliminated
            //TODO: ensure that constraints involving non-parameter lifetimes are eliminated
            .filter(|(l, r)| *l != *r)
            .collect();
        unification_id_pairs
    }

    fn get_solved_parameter_lifetimes(&mut self, param: &Parameter) -> Vec<Lifetime> {
        self.register_parameter_lifetimes(param)
            .iter()
            .map(|id| self.unification.find(*id))
            .collect()
    }
}

impl fmt::Display for LifetimeCtx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output_keys = self.lifetime_ids.iter().fold("".to_string(), |acc, pair| {
            acc + format!("{}:{}\r\n", pair.0, pair.1).as_str()
        });
        let output_constriants =
            self.constraints
                .node_indices()
                .fold("".to_string(), |acc, node_idx| {
                    let root = self.constraints.node_weight(node_idx).unwrap();
                    let lt_list = self
                        .constraints
                        .neighbors_directed(node_idx, Outgoing)
                        .map(|neighbor_idx| {
                            self.constraints
                                .node_weight(neighbor_idx)
                                .unwrap()
                                .to_string()
                        })
                        .collect::<Vec<_>>();
                    if lt_list.len() > 0 {
                        acc + format!("{} >= {{{}}}\r\n", root, lt_list.join(",")).as_str()
                    } else {
                        acc
                    }
                });

        write!(f, "\r\n{}\r\n{}", output_keys, output_constriants)
    }
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lifetime::Local => write!(f, "local"),
            Lifetime::Heap => write!(f, "heap"),
            Lifetime::Var(id) => write!(f, "var({})", id),
        }
    }
}
