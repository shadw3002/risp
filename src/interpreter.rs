use super::{Expression, Environment};

pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
        }
    }

    pub fn eval(&mut self, exp: &Expression) {
        
    }
}