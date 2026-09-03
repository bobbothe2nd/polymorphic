use polymorphic::polymorphic;

polymorphic! {
    #[must_use]
    pub fn foo(a, b) -> match {
        usize (u32, u64) => { ((a as u64) + b) as usize }
        usize (u64, u32) => { (a - (b as u64)) as usize }
    }
}

#[test]
fn by_input() {
    let val: usize = foo(3, 2u32);
    assert_eq!(val, 1);
    let val: usize = foo(3u32, 2);
    assert_eq!(val, 5);
}

polymorphic! {
    #[must_use]
    pub fn zero(a) -> match {
        u32 (u32) => { a }
        i32 (u32) => { 0 }
        f32 (u32) => { 0.0 }
        f64 (u32) => { 0.0 }
    }

    #[must_use]
    pub async unsafe fn zero2() -> match {
        u32 () => { 0 }
        i32 () => { 0 }
        f32 () => { 0.0 }
        f64 () => { 0.0 }
    }

    pub fn one() -> match {
        u32 () => { 1 }
        i32 () => { 1 }
        f32 () => { 1.0 }
        f64 () => { 1.0 }
    }

    fn two() -> match {
        u32 () => { 2 }
        i32 () => { 2 }
        f32 () => { 2.0 }
        f64 () => { 2.0 }
    }
}

#[test]
fn by_output() {
    let a: u32 = zero(0);
    let b: f32 = two();

    assert_eq!(a, 0);
    assert_eq!(b, 2.0);
}
