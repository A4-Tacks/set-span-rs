The `set_span` in the simulation process macro

This macro can be used to improve error messages, etc

# Note
Incorrect use may cause some damage to hygiene

# Example

```rust,compile_fail
macro_rules! foo {
    ($t:tt) => {
        foo! { ($t) ($t) }
    };
    ($t:tt (0)) => {
        set_span::set_span_all! {$t[0], {
            compile_error! {"input by zero"}
        }}
    };
    ($_:tt ($lit:literal)) => { /*...*/ };
}
foo!(0);
```

Output:

```ignore
error: input by zero
  --> src/lib.rs:25:6
   |
16 | foo!(0);
   |      ^
```

### If `set_span` is not used:

```rust,compile_fail
macro_rules! foo {
    ($t:tt) => {
        foo! { ($t) ($t) }
    };
    ($t:tt (0)) => {
        compile_error! {"input by zero"}
    };
    ($_:tt ($lit:literal)) => { /*...*/ };
}
foo!(0);
```

Output:

```ignore
error: input by zero
  --> src/lib.rs:45:9
   |
8  |         compile_error! {"input by zero"}
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
...
12 | foo!(0);
   | ------- in this macro invocation
   |
```
