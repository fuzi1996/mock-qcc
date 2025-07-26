use std::collections::HashMap;

pub trait ParseHandler {
    fn is_match(&self, path: &str) -> bool;

    fn parse(&self, params: HashMap<String, String>) -> (Vec<String>, String);
}
