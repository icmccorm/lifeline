

use std::{fmt, collections::HashMap};

use llvm_ir::Module;

pub struct ProgramLifetimes {
    results: HashMap<String, FunctionLifetimes>
}

#[derive(Default)]
struct FunctionLifetimes {

}

impl ProgramLifetimes {
    pub fn new(module: &Module) -> Self{
        let mut map = HashMap::new();
        for a in module.functions.iter(){
            map.insert(a.name.to_owned(), FunctionLifetimes::default());
        }
        ProgramLifetimes {
            results: map
        }
    }
}

impl fmt::Display for ProgramLifetimes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let res = self.results.iter().fold("".to_string(), |acc, res| {
            acc + format!("{}: {}\r\n",res.0, res.1).as_str()
        });
        write!(f, "{}", res)
    }
}

impl fmt::Display for FunctionLifetimes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unknown")
    }
}