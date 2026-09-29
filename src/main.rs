use std::cell::RefCell ;
use std::collections::HashMap ;

struct MyCache {
    d:  RefCell<HashMap<String, String>>
}

impl MyCache {
    fn get_or_insert(&self, k: &str, v: String) ->String {
        let mut m = 
            self
                .d
                .borrow_mut() ;

        m
            .entry(k.to_string())
            .or_insert(v)
            .clone()
    }
}

fn main() {
    let my_cache = MyCache{d: RefCell::new(HashMap::new())} ;

    let v = my_cache.get_or_insert("abc", "def".to_string()) ;

    println!("{}", v) ; // Iut: def
}
