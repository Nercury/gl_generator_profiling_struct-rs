## Profiling struct generator

Yet another generator for `gl_generator`, very similar to `DebugStructGenerator`, but with these changes/additions:

- It does not log all calls. It only logs a call that caused an error.
- The corresponding explanation is included with the error code.
- Contains a profiler that tracks the number of GL calls and errors.

### Using the profiler

The generated `gl` module gains 3 additional methods:

- `profiler_reset()` - resets the profiler;
- `profiler_call_count() -> usize` - returns the number of calls since the last reset (or application start);
- `profiler_err_count() -> usize` - returns the number of errors since the last reset (or application start);

Example usage:

```rust
gl::profiler_reset();

// the code

println!("Number of GL calls: {}", gl::profiler_call_count());
println!("Number of GL errors: {}", gl::profiler_err_count());
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.