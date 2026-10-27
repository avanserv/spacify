use std::fmt;
const LIMIT: usize = 3;
static NAME: &str = "items";
struct Point;
fn items() {
    use std::io;
    let a = 1;
    fn helper() {}
    let b = 2;
    struct Local;
    if a > b {
        helper();
    }
    const INNER: u32 = 1;
    let c = INNER;
    drop(c);
}
impl Point {
    fn new() -> Self {
        Point
    }
    fn other(&self) {}
}
