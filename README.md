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

or use constants:

```rust
polymorphic::polymorphic! {
    fn bar(a, b) [usize, usize] -> match {
        usize (u32, u64) [3, N] => {
            const {
                assert!(N == 2);
            }

            ((a as u64) + b) as usize
        }
        [u8; N] (u64, [u8; N]) [N, 0] => { b }
        usize (u64, u32) [_, 4] => { (a - (b as u64)) as usize }
        usize (u32, u32) [2, 4] => { (a - b) as usize }
    }
}
```

This macro is useful when defining a function that must be generic over only a few types or constants with no required traits. If you require the use of a trait that is implemented by users, this is not for you. If you need to be generic over mostly primitive types or explicit dependencies, `polymorphic` can make your code look much cleaner. Constants make this macro more usable when you need to be generic over them, but don't restrict the macro.

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
    fn __foo_internal(a: T, b: U) -> usize;
}

impl __foo_trait_internal<u32, u64> for usize {
    #[inline]
    fn __foo_internal(a: u32, b: u64) -> usize {
        ((a as u64) + b) as usize
    }
}

impl __foo_trait_internal<u64, u32> for usize {
    #[inline]
    fn __foo_internal(a: u64, b: u32) -> usize {
        (a - (b as u64)) as usize
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
