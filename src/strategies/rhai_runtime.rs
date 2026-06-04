//! Isolated runtime for loading, compiling and executing Rhai scripts.
//!
//! This module is intentionally decoupled from Bootfrost's term
//! representation (`Term`, `PSTerms`, `PEnv`). It exposes a small, focused
//! API on top of the `rhai` crate that can be reused for any script-based
//! extension point: A* heuristics, custom predicates, distance metrics, etc.
//!
//! The module is organized in three parts:
//!   1. [`RhaiRuntimeError`] — domain error type that maps every Rhai failure
//!      mode (file not found, syntax error, type mismatch, runtime panic,
//!      missing function) onto a single, descriptive enum.
//!   2. [`RhaiRuntime`] — the runtime handle. It owns a `rhai::Engine`
//!      and an optional compiled `rhai::AST`. Native functions can be
//!      registered before the script is loaded; the script is then attached
//!      via [`RhaiRuntime::load_file`] or [`RhaiRuntime::load_source`],
//!      and individual functions are called via [`RhaiRuntime::call_fn`].
//!   3. A `#[cfg(test)]` module with smoke tests for the main code paths.

use std::fmt;
use std::fs;
use std::path::Path;

use rhai::{Engine, EvalAltResult, FuncArgs, Scope, Variant, AST};

// =========================================================================
//                              Error type
// =========================================================================

/// All errors that can be produced by [`RhaiRuntime`].
///
/// Each variant carries enough context for diagnostics, and the `Display`
/// implementation produces a single-line human-readable message that is
/// suitable both for logging and for assertion-style tests.
#[derive(Debug)]
pub enum RhaiRuntimeError {
    /// The script file path does not exist on disk.
    FileNotFound { path: String },

    /// The script file exists but could not be read (permissions, I/O, etc.).
    FileReadError { path: String, source: std::io::Error },

    /// The Rhai source contains a syntax or parse error.
    CompileError {
        source: Option<String>,
        message: String,
    },

    /// A script is required (e.g. for `call_fn`) but none has been loaded.
    ScriptNotLoaded,

    /// A function name was requested that the loaded script does not define.
    FunctionNotFound { name: String, available: Vec<String> },

    /// A native function call from Rhai received an argument of an
    /// unexpected type.
    ArgTypeMismatch {
        function: String,
        expected: String,
        actual: String,
    },

    /// A function was called from Rhai but its return type did not match
    /// the caller's expectations (e.g. the caller asked for `i64` but
    /// the script returned `()`).
    ReturnTypeMismatch {
        function: String,
        expected: String,
        actual: String,
    },

    /// A script function was called but raised an error during execution
    /// (panics, division by zero, explicit `throw`, etc.).
    ExecutionError {
        function: String,
        message: String,
    },
}

impl fmt::Display for RhaiRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound { path } => {
                write!(f, "Rhai script file not found: '{}'", path)
            }
            Self::FileReadError { path, source } => {
                write!(f, "failed to read Rhai script '{}': {}", path, source)
            }
            Self::CompileError { source, message } => {
                let loc = source.as_deref().unwrap_or("<inline source>");
                write!(f, "Rhai compile error in '{}': {}", loc, message)
            }
            Self::ScriptNotLoaded => write!(
                f,
                "no Rhai script has been loaded; call load_file() or load_source() first"
            ),
            Self::FunctionNotFound { name, available } => {
                if available.is_empty() {
                    write!(
                        f,
                        "Rhai function '{}' is not defined in the loaded script",
                        name
                    )
                } else {
                    write!(
                        f,
                        "Rhai function '{}' is not defined in the loaded script; available: [{}]",
                        name,
                        available.join(", ")
                    )
                }
            }
            Self::ArgTypeMismatch {
                function,
                expected,
                actual,
            } => write!(
                f,
                "Rhai function '{}' received argument of type '{}' where '{}' was expected",
                function, actual, expected
            ),
            Self::ReturnTypeMismatch {
                function,
                expected,
                actual,
            } => write!(
                f,
                "Rhai function '{}' returned '{}' but the caller expected '{}'",
                function, actual, expected
            ),
            Self::ExecutionError { function, message } => {
                write!(f, "Rhai function '{}' raised: {}", function, message)
            }
        }
    }
}

impl std::error::Error for RhaiRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileReadError { source, .. } => Some(source),
            _ => None,
        }
    }
}

// =========================================================================
//                                Runtime
// =========================================================================

/// A self-contained runtime for loading, compiling and executing Rhai scripts.
///
/// The runtime is fully decoupled from Bootfrost's term representation:
/// arguments and return values are passed as Rhai `Dynamic` values (or any
/// type implementing `Variant`), and the caller is responsible for
/// marshalling to/from `Term` / `TermId` at the integration boundary.
///
/// Typical lifecycle:
/// ```ignore
/// let mut rt = RhaiRuntime::new();
/// rt.engine_mut().register_fn("bf_get", |row: i64, col: i64, grid: rhai::Array| -> i64 {
///     // implementation
/// });
/// rt.load_file("user_funcs.rhai")?;
/// let n: i64 = rt.call_fn("manhattan", (1i64, 2i64, 3i64, 4i64))?;
/// ```
pub struct RhaiRuntime {
    engine: Engine,
    ast: Option<AST>,
    source_path: Option<String>,
}

impl RhaiRuntime {
    /// Create a runtime with an empty engine and no loaded script.
    /// The caller can register native functions and then call
    /// [`load_file`](Self::load_file) or [`load_source`](Self::load_source).
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Rhai's default expression-depth limits (32/64) are too tight for
        // realistic user scripts: a small A* heuristic with a few
        // neighbour checks already trips the parser. Bump both to 256,
        // which is plenty for the algorithms Bootfrost users will write
        // while still preventing pathological scripts from consuming
        // arbitrary compile time.
        engine.set_max_expr_depths(128, 256);
        Self {
            engine,
            ast: None,
            source_path: None,
        }
    }

    /// Convenience constructor: create a runtime and load the given file.
    pub fn from_file(path: &str) -> Result<Self, RhaiRuntimeError> {
        let mut rt = Self::new();
        rt.load_file(path)?;
        Ok(rt)
    }

    /// Convenience constructor: create a runtime from an inline source string.
    /// Useful for unit tests and small embedded scripts.
    pub fn from_source(source: &str) -> Result<Self, RhaiRuntimeError> {
        let mut rt = Self::new();
        rt.load_source(source)?;
        Ok(rt)
    }

    /// Load and compile a script file from disk.
    /// Replaces any previously loaded script.
    pub fn load_file(&mut self, path: &str) -> Result<&mut Self, RhaiRuntimeError> {
        if !Path::new(path).exists() {
            return Err(RhaiRuntimeError::FileNotFound {
                path: path.to_string(),
            });
        }
        let source = fs::read_to_string(path).map_err(|e| RhaiRuntimeError::FileReadError {
            path: path.to_string(),
            source: e,
        })?;
        self.load_source_inner(&source, Some(path.to_string()))?;
        Ok(self)
    }

    /// Load and compile a script from an inline source string.
    /// Replaces any previously loaded script.
    pub fn load_source(&mut self, source: &str) -> Result<&mut Self, RhaiRuntimeError> {
        self.load_source_inner(source, None)?;
        Ok(self)
    }

    /// Compile `source` into an `AST` and store it, attaching the optional
    /// `path` label so any compile error can be reported with a useful origin.
    fn load_source_inner(
        &mut self,
        source: &str,
        path: Option<String>,
    ) -> Result<(), RhaiRuntimeError> {
        let ast = self.engine.compile(source).map_err(|e| RhaiRuntimeError::CompileError {
            source: path.clone(),
            message: e.to_string(),
        })?;
        self.ast = Some(ast);
        self.source_path = path;
        Ok(())
    }

    // --- Inspection ---

    /// Returns `true` if a script has been successfully loaded.
    pub fn is_loaded(&self) -> bool {
        self.ast.is_some()
    }

    /// Returns the path of the loaded script, if any.
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }

    /// Borrow the underlying engine (e.g. for inspection).
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Mutable access to the underlying engine. Use this to register native
    /// functions before invoking [`call_fn`](Self::call_fn).
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    // --- Invocation ---

    /// Call a named function defined in the loaded script, using a fresh,
    /// ephemeral `Scope`. The result is downcast to `T`.
    ///
    /// `T` is the Rust type to downcast the return value to. `A` is a Rhai
    /// args tuple (e.g. `(i64, i64)` for a 2-arg function). The arguments
    /// must implement `Variant`, which covers all primitive types and Rhai's
    /// `Array` / `Map` containers.
    pub fn call_fn<T, A>(&self, name: &str, args: A) -> Result<T, RhaiRuntimeError>
    where
        T: Variant + Clone,
        A: FuncArgs,
    {
        let mut scope = Scope::new();
        self.call_fn_with_scope(&mut scope, name, args)
    }

    /// Call a named function using a caller-provided `Scope`. Use this
    /// variant when the script needs to read or write variables that the
    /// Rust side has pushed into the scope.
    pub fn call_fn_with_scope<T, A>(
        &self,
        scope: &mut Scope,
        name: &str,
        args: A,
    ) -> Result<T, RhaiRuntimeError>
    where
        T: Variant + Clone,
        A: FuncArgs,
    {
        let ast = self
            .ast
            .as_ref()
            .ok_or(RhaiRuntimeError::ScriptNotLoaded)?;
        self.engine
            .call_fn(scope, ast, name, args)
            .map_err(|e| self.map_eval_error(name, *e))
    }

    /// Map a Rhai evaluation error into our domain error type.
    fn map_eval_error(&self, name: &str, err: EvalAltResult) -> RhaiRuntimeError {
        use EvalAltResult::*;
        match err {
            ErrorFunctionNotFound(s, _) => RhaiRuntimeError::FunctionNotFound {
                name: s.to_string(),
                available: Vec::new(),
            },
            ErrorMismatchDataType(expected, actual, _) => {
                RhaiRuntimeError::ArgTypeMismatch {
                    function: name.to_string(),
                    expected,
                    actual,
                }
            }
            ErrorMismatchOutputType(expected, actual, _) => {
                RhaiRuntimeError::ReturnTypeMismatch {
                    function: name.to_string(),
                    expected,
                    actual,
                }
            }
            ErrorRuntime(d, _) => RhaiRuntimeError::ExecutionError {
                function: name.to_string(),
                message: d.to_string(),
            },
            other => RhaiRuntimeError::ExecutionError {
                function: name.to_string(),
                message: other.to_string(),
            },
        }
    }
}

impl Default for RhaiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RhaiRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RhaiRuntime")
            .field("loaded", &self.is_loaded())
            .field("source_path", &self.source_path)
            .finish()
    }
}

// =========================================================================
//                                Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_source_loads_simple_script_and_calls_function() {
        let rt = RhaiRuntime::from_source("fn add(a, b) { a + b }").unwrap();
        let result: i64 = rt.call_fn("add", (2i64, 3i64)).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn from_file_returns_file_not_found_for_missing_path() {
        let err = RhaiRuntime::from_file("definitely_does_not_exist_12345.rhai")
            .unwrap_err();
        match err {
            RhaiRuntimeError::FileNotFound { path } => {
                assert!(path.contains("definitely_does_not_exist_12345.rhai"));
            }
            other => panic!("expected FileNotFound, got {:?}", other),
        }
    }

    #[test]
    fn load_source_reports_compile_error_for_bad_syntax() {
        let err = RhaiRuntime::from_source("fn broken( {").unwrap_err();
        match err {
            RhaiRuntimeError::CompileError { source, message } => {
                assert!(source.is_none(), "inline source should report no path");
                assert!(!message.is_empty());
            }
            other => panic!("expected CompileError, got {:?}", other),
        }
    }

    #[test]
    fn call_fn_returns_function_not_found_for_unknown_name() {
        let rt = RhaiRuntime::from_source("fn add(a, b) { a + b }").unwrap();
        let err: RhaiRuntimeError = rt
            .call_fn::<i64, _>("subtract", (1i64, 2i64))
            .unwrap_err();
        match err {
            RhaiRuntimeError::FunctionNotFound { name, .. } => {
                assert_eq!(name, "subtract");
            }
            other => panic!("expected FunctionNotFound, got {:?}", other),
        }
    }

    #[test]
    fn call_fn_returns_execution_error_on_runtime_panic() {
        // `throw` is a language keyword that always raises a runtime error,
        // unlike the `panic` function which is only available behind
        // certain feature flags.
        let rt = RhaiRuntime::from_source(r#"fn boom() { throw "oops" }"#).unwrap();
        let err: RhaiRuntimeError = rt.call_fn::<(), _>("boom", ()).unwrap_err();
        match err {
            RhaiRuntimeError::ExecutionError { function, message } => {
                assert_eq!(function, "boom");
                assert!(!message.is_empty());
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }

    #[test]
    fn call_fn_without_loaded_script_returns_script_not_loaded() {
        let rt = RhaiRuntime::new();
        let err: RhaiRuntimeError = rt.call_fn::<i64, _>("anything", ()).unwrap_err();
        assert!(matches!(err, RhaiRuntimeError::ScriptNotLoaded));
    }

    #[test]
    fn native_function_registered_via_engine_mut_is_callable_from_script() {
        let mut rt = RhaiRuntime::from_source("fn call_it(x) { double(x) + 1 }").unwrap();
        rt.engine_mut().register_fn("double", |x: i64| x * 2);
        let result: i64 = rt.call_fn("call_it", (10i64,)).unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn load_source_replaces_previously_loaded_script() {
        let mut rt = RhaiRuntime::from_source("fn first() { 1 }").unwrap();
        assert!(rt.call_fn::<i64, _>("first", ()).is_ok());

        rt.load_source("fn second() { 2 }").unwrap();

        // first should be gone after replacement.
        let err = rt.call_fn::<i64, _>("first", ()).unwrap_err();
        assert!(matches!(err, RhaiRuntimeError::FunctionNotFound { .. }));

        // second should now be callable.
        let v: i64 = rt.call_fn("second", ()).unwrap();
        assert_eq!(v, 2);
    }

    #[test]
    fn is_loaded_reports_state_correctly() {
        let mut rt = RhaiRuntime::new();
        assert!(!rt.is_loaded());

        rt.load_source("fn x() { 1 }").unwrap();
        assert!(rt.is_loaded());

        let path = rt.source_path();
        assert!(path.is_none(), "inline source has no path");
    }

    #[test]
    fn call_fn_with_scope_exposes_pushed_variable_to_script() {
        // Push a variable into the scope and have the script read it.
        let rt = RhaiRuntime::from_source("fn read_pi() { pi }").unwrap();
        let mut scope = Scope::new();
        scope.push("pi", 314_i64);
        let result: i64 = rt.call_fn_with_scope(&mut scope, "read_pi", ()).unwrap();
        assert_eq!(result, 314);
    }
}
