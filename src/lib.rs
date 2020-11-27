/*!

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

```rust,no_run,ignore
gl::profiler_reset();

// the code

println!("Number of GL calls: {}", gl::profiler_call_count());
println!("Number of GL errors: {}", gl::profiler_err_count());
```

## Setting up the build script

The build script is very similar to the one used by `gl` crate. Here is the example:

```rust,no_run,ignore
extern crate gl_generator;
extern crate gl_generator_profiling_struct;

use gl_generator::{Registry, Fallbacks, Api, Profile};
use gl_generator_profiling_struct::ProfilingStructGenerator;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut file_gl = File::create(&Path::new(&out_dir).join("bindings.rs")).unwrap();

    let registry = Registry::new(Api::Gl, (4, 5), Profile::Core, Fallbacks::All, [
        "GL_NV_command_list",
    ]);

    registry.write_bindings(
        ProfilingStructGenerator,
        &mut file_gl
    ).unwrap();
}
```

*/

extern crate gl_generator;

use gl_generator::{Registry, generators};

use std::io;

#[allow(missing_copy_implementations)]
pub struct ProfilingStructGenerator;

impl gl_generator::Generator for ProfilingStructGenerator {
    fn write<W>(&self, registry: &Registry, dest: &mut W) -> io::Result<()>
        where
            W: io::Write,
    {
        write_helper(dest)?;
        write_header(dest)?;
        write_type_aliases(registry, dest)?;
        write_enums(registry, dest)?;
        write_fnptr_struct_def(dest)?;
        write_panicking_fns(registry, dest)?;
        write_struct(registry, dest)?;
        write_impl(registry, dest)?;
        Ok(())
    }
}

/// Creates a `__gl_imports` module which contains all the external symbols that we need for the
///  bindings.
fn write_helper<W>(dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        "{}",
        r##"
static CALL_COUNT: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
static ERR_COUNT: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);

pub fn profiler_reset() {
    CALL_COUNT.store(0, ::std::sync::atomic::Ordering::SeqCst);
    ERR_COUNT.store(0, ::std::sync::atomic::Ordering::SeqCst);
}

pub fn profiler_call_count() -> usize {
    CALL_COUNT.load(::std::sync::atomic::Ordering::SeqCst)
}

fn inc_call() {
    CALL_COUNT.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
}

pub fn profiler_err_count() -> usize {
    ERR_COUNT.load(::std::sync::atomic::Ordering::SeqCst)
}

fn inc_err() {
    ERR_COUNT.fetch_add(1, ::std::sync::atomic::Ordering::SeqCst);
}

fn gl_error_to_str(error: u32) -> &'static str {
    match error {
        self::NO_ERROR => {
            "NO_ERROR = No error has been recorded.
                        The value of this \
                      symbolic constant is guaranteed to be 0."
        }
        self::INVALID_ENUM => {
            "INVALID_ENUM = An unacceptable value is specified for an enumerated argument.
                        \
                      The offending command is ignored
                        and has no other \
                      side effect than to set the error flag."
        }
        self::INVALID_VALUE => {
            "INVALID_VALUE = A numeric argument is out of range.
                        The offending command is ignored
                        and has no other side effect than to set the error flag."
        }
        self::INVALID_OPERATION => {
            "INVALID_OPERATION = The specified operation is not allowed in the current \
                      state.
                        The offending command is ignored
                        \
                      and has no other side effect than to set the error flag."
        }
        self::INVALID_FRAMEBUFFER_OPERATION => {
            "INVALID_FRAMEBUFFER_OPERATION = The command is trying to render to or read \
                      from the framebuffer
                        while the currently bound \
                      framebuffer is not framebuffer
                        complete (i.e. the \
                      return value from
                        glCheckFramebufferStatus
                        \
                      is not GL_FRAMEBUFFER_COMPLETE).
                        The offending \
                      command is ignored
                        and has no other side effect than \
                      to set the error flag."
        }
        self::OUT_OF_MEMORY => {
            "OUT_OF_MEMORY = There is not enough memory left to execute the command.
                        The state of the GL is undefined,
                        except for the state of the error flags,
                        after this error is recorded."
        }
        _ => "Unknown error",
    }
}
    "##
    )
}

/// Creates a `__gl_imports` module which contains all the external symbols that we need for the
///  bindings.
fn write_header<W>(dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        r#"
        mod __gl_imports {{
            pub use std::mem;
            pub use std::marker::Send;
            pub use std::os::raw;
        }}
    "#
    )
}

/// Creates a `types` module which contains all the type aliases.
///
/// See also `generators::gen_types`.
fn write_type_aliases<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        r#"
        pub mod types {{
            #![allow(non_camel_case_types, non_snake_case, dead_code, missing_copy_implementations)]
    "#
    )?;

    generators::gen_types(registry.api, dest)?;

    writeln!(dest, "}}")
}

/// Creates all the `<enum>` elements at the root of the bindings.
fn write_enums<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    for enm in &registry.enums {
        generators::gen_enum_item(enm, "types::", dest)?;
    }

    Ok(())
}

/// Creates a `FnPtr` structure which contains the store for a single binding.
fn write_fnptr_struct_def<W>(dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        "
        #[allow(dead_code, missing_copy_implementations)]
        #[derive(Clone)]
        pub struct FnPtr {{
            /// The function pointer that will be used when calling the function.
            f: *const __gl_imports::raw::c_void,
            /// True if the pointer points to a real function, false if points to a `panic!` fn.
            is_loaded: bool,
        }}
        impl FnPtr {{
            /// Creates a `FnPtr` from a load attempt.
            fn new(ptr: *const __gl_imports::raw::c_void) -> FnPtr {{
                if ptr.is_null() {{
                    FnPtr {{
                        f: missing_fn_panic as *const __gl_imports::raw::c_void,
                        is_loaded: false
                    }}
                }} else {{
                    FnPtr {{ f: ptr, is_loaded: true }}
                }}
            }}
            /// Returns `true` if the function has been successfully loaded.
            ///
            /// If it returns `false`, calling the corresponding function will fail.
            #[inline]
            #[allow(dead_code)]
            pub fn is_loaded(&self) -> bool {{
                self.is_loaded
            }}
        }}
    "
    )
}

/// Creates a `panicking` module which contains one function per GL command.
///
/// These functions are the mocks that are called if the real function could not be loaded.
fn write_panicking_fns<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        "#[inline(never)]
        fn missing_fn_panic() -> ! {{
            panic!(\"{api} function was not loaded\")
        }}",
        api = registry.api
    )
}

/// Creates a structure which stores all the `FnPtr` of the bindings.
///
/// The name of the struct corresponds to the namespace.
fn write_struct<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(
        dest,
        "
        #[allow(non_camel_case_types, non_snake_case, dead_code)]
        #[derive(Clone)]
        pub struct {api} {{",
        api = generators::gen_struct_name(registry.api)
    )?;

    for cmd in &registry.cmds {
        if let Some(v) = registry.aliases.get(&cmd.proto.ident) {
            writeln!(dest, "/// Fallbacks: {}", v.join(", "))?;
        }
        writeln!(dest, "pub {name}: FnPtr,", name = cmd.proto.ident)?;
    }
    writeln!(dest, "_priv: ()")?;

    writeln!(dest, "}}")
}

/// Creates the `impl` of the structure created by `write_struct`.
fn write_impl<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where
        W: io::Write,
{
    writeln!(dest,
                  "impl {api} {{
            /// Load each OpenGL symbol using a custom load function. This allows for the
            /// use of functions like `glfwGetProcAddress` or `SDL_GL_GetProcAddress`.
            ///
            /// ~~~ignore
            /// let gl = Gl::load_with(|s| glfw.get_proc_address(s));
            /// ~~~
            #[allow(dead_code, unused_variables)]
            pub fn load_with<F>(mut loadfn: F) -> {api} where F: FnMut(&'static str) -> *const __gl_imports::raw::c_void {{
                #[inline(never)]
                fn do_metaloadfn(loadfn: &mut dyn FnMut(&'static str) -> *const __gl_imports::raw::c_void,
                                 symbol: &'static str,
                                 symbols: &[&'static str])
                                 -> *const __gl_imports::raw::c_void {{
                    let mut ptr = loadfn(symbol);
                    if ptr.is_null() {{
                        for &sym in symbols {{
                            ptr = loadfn(sym);
                            if !ptr.is_null() {{ break; }}
                        }}
                    }}
                    ptr
                }}
                let mut metaloadfn = |symbol: &'static str, symbols: &[&'static str]| {{
                    do_metaloadfn(&mut loadfn, symbol, symbols)
                }};
                {api} {{",
                  api = generators::gen_struct_name(registry.api))?;

    for cmd in &registry.cmds {
        writeln!(
            dest,
            "{name}: FnPtr::new(metaloadfn(\"{symbol}\", &[{fallbacks}])),",
            name = cmd.proto.ident,
            symbol = generators::gen_symbol_name(registry.api, &cmd.proto.ident),
            fallbacks = match registry.aliases.get(&cmd.proto.ident) {
                Some(fbs) => fbs.iter()
                    .map(|name| format!("\"{}\"", generators::gen_symbol_name(registry.api, &name)))
                    .collect::<Vec<_>>()
                    .join(", "),
                None => format!(""),
            },
        )?
    }
    writeln!(dest, "_priv: ()")?;

    writeln!(
        dest,
        "}}
        }}"
    )?;

    for cmd in &registry.cmds {
        let idents = generators::gen_parameters(cmd, true, false);
        let typed_params = generators::gen_parameters(cmd, false, true);
        let println = format!(
            "println!(\"[OpenGL] {}({})\" {});",
            cmd.proto.ident,
            (0..idents.len())
                .map(|_| "{:?}".to_string())
                .collect::<Vec<_>>()
                .join(", "),
            idents
                .iter()
                .zip(typed_params.iter())
                .map(|(name, ty)| if ty.contains("GLDEBUGPROC") {
                    format!(", \"<callback>\"")
                } else {
                    format!(", {}", name)
                })
                .collect::<Vec<_>>()
                .concat()
        );

        writeln!(dest,
                      "#[allow(non_snake_case, unused_variables, dead_code)]
            #[inline] pub unsafe fn {name}(&self, {params}) -> {return_suffix} {{ \
                let r = __gl_imports::mem::transmute::<_, extern \"system\" fn({typed_params}) -> {return_suffix}>\
                    (self.{name}.f)({idents});
                {print_err}
                r
            }}",
                      name = cmd.proto.ident,
                      params = generators::gen_parameters(cmd, true, true).join(", "),
                      typed_params = typed_params.join(", "),
                      return_suffix = cmd.proto.ty,
                      idents = idents.join(", "),
                      print_err = if cmd.proto.ident != "GetError" &&
                          registry
                              .cmds
                              .iter()
                              .find(|cmd| cmd.proto.ident == "GetError")
                              .is_some() {
                          format!(r#"match __gl_imports::mem::transmute::<_, extern "system" fn() -> u32>
                    (self.GetError.f)() {{ 0 => inc_call(), r => {{ inc_err(); {println} println!("[OpenGL] ^ GL error triggered: {{}}, {{}}", r, gl_error_to_str(r))}} }}"#, println = println)
                      } else {
                          format!("")
                      })?
    }

    writeln!(
        dest,
        "}}
        unsafe impl __gl_imports::Send for {api} {{}}",
        api = generators::gen_struct_name(registry.api)
    )
}