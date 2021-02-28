use serde::{Deserialize, Serialize};

// pub type OpFunc = Fn(impl Deserialize) -> (impl Serialize);

pub fn sum(args: Vec<u64>) -> u64 {
    args.into_iter().sum()
}

#[derive(Deserialize)]
pub struct AddArgs {
    a: u32,
    b: u32,
}
pub fn add(args: AddArgs) -> u32 {
    args.a + args.b
}

#[derive(Deserialize, Serialize)]
pub struct Person {
    first_name: String,
    last_name: String,
    age: u8,
}
pub fn promote(args: Person) -> Person {
    Person {
        first_name: args.first_name.to_uppercase(),
        last_name: args.last_name.to_ascii_uppercase(),
        age: args.age + 1,
    }
}
