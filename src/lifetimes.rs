use crate::dsu::DSU;
use crate::utilities::usize_to_string;
use llvm_ir::{function::Parameter, Name, TypeRef};
use petgraph::algo::tarjan_scc;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{prelude::StableDiGraph, Direction::Outgoing};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
};

const ALPHABET_SIZE: usize = 26;
const LOWERCASE_A_OFFSET: usize = 97;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Lifetime {
    Local,
    Heap,
    Var(llvm_ir::Name, u32),
}
pub type LifetimeID = u32;

pub struct LifetimeCtx {
    lifetime_counter: u32,
    pub lifetime_ids: HashMap<Lifetime, LifetimeID>,
    pub constraints: StableDiGraph<LifetimeID, ()>,
    pub unification: DSU<LifetimeID>,
    pub lifetime_node_indices: HashMap<LifetimeID, NodeIndex>,
}

impl Default for LifetimeCtx {
    fn default() -> Self {
        let map = HashMap::default();

        let mut ctx: LifetimeCtx = LifetimeCtx {
            lifetime_counter: 0,
            lifetime_ids: map,
            constraints: StableGraph::default(),
            unification: DSU::default(),
            lifetime_node_indices: HashMap::default(),
        };
        ctx.register(&Lifetime::Local);
        ctx.register(&Lifetime::Heap);
        ctx
    }
}

impl LifetimeCtx {
    fn register(&mut self, lt: &Lifetime) -> LifetimeID {
        let value = match self.lifetime_ids.entry(lt.to_owned()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                self.lifetime_counter += 1;
                v.insert(self.lifetime_counter)
            }
        };
        self.unification.add(*value);
        *value
    }

    pub fn register_parameter_lifetimes(&mut self, param: &Parameter) -> Vec<LifetimeID> {
        self.register_lifetimes(&param.name, &param.ty)
    }

    pub fn register_lifetimes(&mut self, name: &Name, ty: &TypeRef) -> Vec<LifetimeID> {
        get_lifetimes(name, ty)
            .iter()
            .map(|lt| self.register(lt))
            .collect()
    }

    pub fn generate_equality(&mut self, left: LifetimeID, right: LifetimeID) {
        self.unification.union(left, right);
    }

    fn lifetime_node_index(&mut self, id: LifetimeID) -> NodeIndex {
        match self.lifetime_node_indices.entry(id) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let new_node_index = self.constraints.add_node(id);
                v.insert(new_node_index);
                new_node_index
            }
        }
    }

    pub fn generate_outlives(&mut self, left: LifetimeID, right: LifetimeID) {
        if left != right {
            let shorter = self.lifetime_node_index(left);
            let longer = self.lifetime_node_index(right);
            self.constraints.add_edge(longer, shorter, ());
        }
    }

    pub fn finalize(&mut self) {
        let components = tarjan_scc(&self.constraints);
        for component in components.iter() {
            let ids: Vec<u32> = component
                .iter()
                .map(|n| *self.constraints.node_weight(*n).unwrap())
                .collect();
            match &*ids {
                [head, tail @ ..] => {
                    for elem in tail {
                        self.generate_equality(*head, *elem)
                    }
                }
                _ => (),
            }
        }
    }
    pub fn get_constraints(&self) -> Vec<(LifetimeID, LifetimeID)> {
        let lifetime_id_pairs: Vec<(u32, u32)> = self
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
            .fold(vec![], |mut acc, v: Vec<(u32, u32)>| {
                acc.extend(v.iter());
                acc
            });

        let local_lt_id = self
            .unification
            .find(*self.lifetime_ids.get(&Lifetime::Local).unwrap());

        let unification_id_pairs: Vec<(u32, u32)> = lifetime_id_pairs
            .iter()
            .map(|(l, r)| (self.unification.find(*l), self.unification.find(*r)))
            .filter(|(l, r)| *l != *r && *l != local_lt_id && *r != local_lt_id)
            .collect();
        unification_id_pairs
    }

    pub fn get_solved_parameter_lifetimes(&mut self, param: &Parameter) -> Vec<u32> {
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

        write!(f, "\r\n{}\r\n\r\n{}", output_keys, output_constriants)
    }
}

impl fmt::Display for Lifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lifetime::Local => write!(f, "'local"),
            Lifetime::Heap => write!(f, "'heap"),
            Lifetime::Var(nm, ind) => match nm {
                Name::Name(bx) => write!(f, "'{}", *bx),
                Name::Number(us) => {
                    write!(f, "{}{}", usize_to_string(*ind as usize), us)
                }
            },
        }
    }
}

pub fn get_lifetimes(name: &Name, ty: &TypeRef) -> Vec<Lifetime> {
    fn recurse_lifetimes(name: &Name, ty: &TypeRef, indirection: u32) -> Vec<Lifetime>
where {
        let curr = if indirection == 0 {
            vec![(Lifetime::Local)]
        } else {
            vec![(Lifetime::Var(name.to_owned(), indirection))]
        };

        match ty.as_ref() {
            llvm_ir::Type::IntegerType { bits: _ } => curr,
            llvm_ir::Type::PointerType {
                pointee_type,
                addr_space: _,
            } => vec![curr, recurse_lifetimes(name, pointee_type, indirection + 1)].concat(),
            _ => todo!(),
        }
    }
    recurse_lifetimes(name, ty, 0)
}
