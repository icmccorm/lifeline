use std::collections::HashMap;
use std::hash::Hash;
#[derive(Debug, Default)]
pub struct DSU<T:Eq + Hash> {
    map: HashMap<T, T>
}

impl<T:Eq + Hash + Copy> DSU<T> {
    pub fn add(& mut self, item: T) -> T {
        self.map.insert(item, item);
        item
    }
    pub fn find(&self, item: T) -> T{
        match self.map.get(&item) {
            Some(parent) => if item == *parent {
                item
            }else{
                self.find(*parent)
            },
            None => {
                item
            }

        }
    }
    pub fn union(& mut self, target: T, incoming: T) -> T {
        let target_rep = self.find(target);
        let incoming_rep = self.find(incoming);
        self.map.insert(incoming_rep, target_rep);
        incoming_rep
    }
}