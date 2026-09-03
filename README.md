# `polymorphic`

Generates functions and traits to turn parametric polymorphism into ad-hoc polymorphism.

```rust
polymorphic::polymorphic! {
    #[must_use]
    fn foo(a, b) -> match {
        usize (u32, u64) => { ((a as u64) + b) as usize }
        usize (u64, u32) => { (a - (b as u64)) as usize }
    }

    #[must_use]
    pub fn zero() -> match {
        u32 () => { 0 }
        i32 () => { 0 }
        f32 () => { 0.0 }
        f64 () => { 0.0 }
    }
}

fn main() {
    let val: usize = foo(3, 2u32); // matches u64,u32 rule (subtract)
    assert_eq!(val, 1);
    let val: usize = foo(3u32, 2); // matches u32,u64 rule (add)
    assert_eq!(val, 5);
    let val: u32 = zero();
    assert_eq!(val, 0);
}
```

## Codegen

This macro invocation:

```rust
polymorphic::polymorphic! {
    #[must_use]
    fn foo(a, b) -> match {
        usize (u32, u64) => { ((a as u64) + b) as usize }
        usize (u64, u32) => { (a - (b as u64)) as usize }
    }
}
```

is roughly equal to this hand-written code:

```rust
trait __foo_trait_internal<T, U> {
    fn __foo_internal(a: T, b: U) -> Self;
}

impl __foo_trait_internal<u32, u64> for usize {
    #[inline]
    fn __foo_internal(a: u32, b: u64) -> Self {
        ((a as u64) + b) as Self
    }
}

impl __foo_trait_internal<u64, u32> for usize {
    #[inline]
    fn __foo_internal(a: u64, b: u32) -> Self {
        (a - (b as u64)) as Self
    }
}

#[must_use]
fn foo<T, U, V>(a: T, b: U) -> V
where
    V: __foo_trait_internal<T, U>,
{
    V::__foo_internal(a, b)
}
```
