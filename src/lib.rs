#![doc = include_str!("../README.md")]
#![deny(clippy::unwrap_used)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![cfg_attr(docs, feature(doc_cfg))]

#[macro_use]
pub mod macros;
pub mod bindings;
pub mod errors;
pub mod php;

/// Attribute used to annotate constants to be exported to PHP.
///
/// The declared constant is left intact (apart from the addition of the `#[allow(dead_code)]`
/// attribute in the case that you do not use the Rust constant).
///
/// These declarations must happen before you declare your [`macro@php_startup`] function (or
/// [`macro@php_module`] function if you do not have a startup function).
///
/// # Example
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_const]
/// const TEST_CONSTANT: i32 = 100;
///
/// #[php_const]
/// const ANOTHER_CONST: &str = "Hello, world!";
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
pub use ext_php_rs_derive::php_const;

/// Attribute used to annotate `extern` blocks which are deemed as PHP functions.
///
/// This allows you to 'import' PHP functions into Rust so that they can be called like regular
/// Rust functions. Parameters can be any type that implements [`IntoZval`], and the return type
/// can be anything that implements [`From<Zval>`] (notice how [`Zval`] is consumed rather than
/// borrowed in this case).
///
/// # Panics
///
/// The function can panic when called under a few circumstances:
///
/// * The function could not be found or was not callable.
/// * One of the parameters could not be converted into a [`Zval`].
/// * The actual function call failed internally.
/// * The output [`Zval`] could not be parsed into the output type.
///
/// The last point can be important when interacting with functions that return unions, such as
/// [`strpos`] which can return an integer or a boolean. In this case, a [`Zval`] should be
/// returned as parsing a boolean to an integer is invalid, and vice versa.
///
/// # Example
///
/// This `extern` block imports the [`strpos`] function from PHP. Notice that the string parameters
/// can take either [`String`] or [`&str`], the optional parameter `offset` is an [`Option<i64>`],
/// and the return value is a [`Zval`] as the return type is an integer-boolean union.
///
/// ```
/// # use ext_php_rs::prelude::*;
/// # use ext_php_rs::php::types::zval::Zval;
/// #[php_extern]
/// extern "C" {
///     fn strpos(haystack: &str, needle: &str, offset: Option<i64>) -> Zval;
/// }
///
/// #[php_function]
/// pub fn my_strpos() {
///     assert_eq!(unsafe { strpos("Hello", "e", None) }.long(), Some(1));
/// }
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
///
/// [`strpos`]: https://www.php.net/manual/en/function.strpos.php
/// [`IntoZval`]: ext_php_rs::php::types::zval::IntoZval
/// [`Zval`]: ext_php_rs::php::types::zval::Zval
pub use ext_php_rs_derive::php_extern;

/// Attribute used to annotate a function as a PHP function.
///
/// Only types that implement [`FromZval`] can be used as parameter and return types. These include
/// but are not limited to the following:
///
/// - Most primitive integers ([`i8`], [`i16`], [`i32`], [`i64`], [`u8`], [`u16`], [`u32`], [`u64`],
/// [`usize`], [`isize`])
/// - Double-precision floating point numbers ([`f64`])
/// - [`bool`]
/// - [`String`]
/// - [`Vec<T>`] and [`HashMap<String, T>`](std::collections::HashMap) where `T: FromZval`.
/// - [`Binary<T>`] for passing binary data as a string, where `T: Pack`.
/// - [`Callable`] for receiving PHP callables, not applicable for return values.
/// - [`Option<T>`] where `T: FromZval`. When used as a parameter, the parameter will be
/// deemed nullable, and will contain [`None`] when `null` is passed. When used as a return type,
/// if [`None`] is returned the [`Zval`] will be set to null. Optional parameters *must* be of the
/// type [`Option<T>`].
///
/// Additionally, you are able to return a variant of [`Result<T, E>`]. `T` must implement
/// [`IntoZval`] and `E` must implement `Into<PhpException>`. If an error variant is returned, a
/// PHP exception is thrown using the [`PhpException`] struct contents.
///
/// You are able to implement [`FromZval`] on your own custom types to have arguments passed in
/// seamlessly. Similarly, you can implement [`IntoZval`] on values that you want to be able to be
/// returned from PHP fucntions.
///
/// Parameters may be deemed optional by passing the parameter name into the attribute options.
/// Note that all parameters that are optional (which includes the given optional parameter as well
/// as all parameters after) *must* be of the type [`Option<T>`], where `T` is a valid type.
///
/// Generics are *not* supported.
///
/// Behind the scenes, an `extern "C"` wrapper function is generated, which is actually called by
/// PHP. The first example function would be converted into a function which looks like so:
///
/// ```ignore
/// pub fn hello(name: String) -> String {
///     format!("Hello, {}!", name)
/// }
///
/// pub extern "C" fn _internal_php_hello(ex: &mut ExecutionData, retval: &mut Zval) {
///     let mut name = Arg::new("name", <String as FromZval>::TYPE);
///     let parser = ArgParser::new(ex)
///         .arg(&mut name)
///         .parse();
///
///     if parser.is_err() {
///         return;
///     }
///
///     let result = hello(match name.val() {
///         Some(val) => val,
///         None => {
///             PhpException::default("Invalid value given for argument `name`.".into())
///                 .throw()
///                 .expect("Failed to throw exception: Invalid value given for argument `name`.");
///             return;
///         }
///     });
///
///     match result.set_zval(retval, false) {
///         Ok(_) => {},
///         Err(e) => {
///             let e: PhpException = e.into();
///             e.throw().expect("Failed to throw exception: Failed to set return value.");
///         }
///     };
/// }
/// ```
///
/// This allows the original function to continue being used while also being exported as a PHP
/// function.
///
/// # Examples
///
/// Creating a simple function which will return a string. The function still must be declared in
/// the PHP module to be able to call.
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_function]
/// pub fn hello(name: String) -> String {
///     format!("Hello, {}!", name)
/// }
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
///
/// Parameters can also be deemed optional by passing the parameter name in the attribute options.
/// This function takes one required parameter (`hello`) and two optional parameters (`description`
/// and `age`).
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_function(optional = "description")]
/// pub fn hello(name: String, description: Option<String>, age: Option<i32>) -> String {
///     let mut response = format!("Hello, {}!", name);
///
///     if let Some(description) = description {
///         response.push_str(format!(" {}.", description).as_ref());
///     }
///
///     if let Some(age) = age {
///         response.push_str(format!(" I am {} year(s) old.", age).as_ref());
///     }
///
///     response
/// }
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
///
/// Defaults can also be given in a similar fashion. For example, the above function could have
/// default values for `description` and `age` by changing the attribute to the following:
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_function(optional = "description", defaults(description = "David", age = 10))]
/// pub fn hello(name: String, description: String, age: i32) -> String {
///     format!("Hello, {}! {}. I am {} year(s) old.", name, description, age)
/// }
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
///
/// [`Result<T, E>`]: std::result::Result
/// [`FunctionBuilder`]: crate::php::function::FunctionBuilder
/// [`FromZval`]: crate::php::types::zval::FromZval
/// [`IntoZval`]: crate::php::types::zval::IntoZval
/// [`Zval`]: crate::php::types::zval::Zval
/// [`Binary<T>`]: crate::php::types::binary::Binary
/// [`Callable`]: crate::php::types::callable::Callable
/// [`PhpException`]: crate::php::exceptions::PhpException
pub use ext_php_rs_derive::php_function;

/// Annotates a structs `impl` block, declaring that all methods and constants declared
/// inside the `impl` block will be declared as PHP methods and constants.
///
/// If you do not want to export a method to PHP, declare it in another `impl` block that is not
/// tagged with this macro.
///
/// The declared methods and functions are kept intact so they can continue to be called from Rust.
/// Methods do generate an additional function, with an identifier in the format
/// `_internal_php_#ident`.
///
/// Methods and constants are declared mostly the same as their global counterparts, so read the
/// documentation on the [`macro@php_function`] and [`macro@php_const`] macros for more details.
///
/// The main difference is that the contents of the `impl` block *do not* need to be tagged with
/// additional attributes - this macro assumes that all contents of the `impl` block are to be
/// exported to PHP.
///
/// The only contrary to this is setting the visibility, optional argument and default arguments
/// for methods. These are done through seperate macros:
///
/// - `#[defaults(key = value, ...)]` for setting defaults of method variables, similar to the
/// function macro. Arguments with defaults need to be optional.
/// - `#[optional(key)]` for setting `key` as an optional argument (and therefore the rest of the
/// arguments).
/// - `#[public]`, `#[protected]` and `#[private]` for setting the visibility of the method,
/// defaulting to public. The Rust visibility has no effect on the PHP visibility.
///
/// Methods can take a immutable or a mutable reference to `self`, but cannot consume `self`. They
/// can also take no reference to `self` which indicates a static method.
///
/// # Example
///
/// ```ignore
/// #[php_class]
/// #[derive(Debug, Default)]
/// pub struct Human {
///     name: String,
///     age: i32,
/// }
///
/// #[php_impl]
/// impl Test {
///     // Class constant - `Human::AGE_LIMIT`
///     const AGE_LIMIT: i32 = 100;
///
///     #[optional(age)]
///     #[default(age = 0)]
///     pub fn __construct(&mut self, name: String, age: i32) {
///         self.name = name;
///         self.age = age;
///     }
///
///     pub fn get_name(&self) -> String {
///         self.name.clone()
///     }
///
///     pub fn get_age(&self) -> i32 {
///         self.age
///     }
///
///     // Static method - `Human::get_age_limit()`
///     pub fn get_age_limit() -> i32 {
///         Self::AGE_LIMIT
///     }
/// }
/// ```
pub use ext_php_rs_derive::php_impl;

/// Annotates a function that will be used by PHP to retrieve information about the module.
///
/// In the process, the function is wrapped by an `extern "C"` function which is called from PHP,
/// which then calls the given function.
///
/// As well as wrapping the function, the `ModuleBuilder` is initialized and functions which have
/// already been declared with the [`macro@php_function`] attribute will be registered with the
/// module, so ideally you won't have to do anything inside the function.
///
/// The attribute must be called on a function *last*, i.e. the last proc-macro to be compiled, as
/// the attribute relies on all other PHP attributes being compiled before the module. If another
/// PHP attribute is compiled after the module attribute, an error will be thrown.
///
/// Note that if the function is not called `get_module`, it will be renamed.
///
/// If you have defined classes using the [`macro@php_class`] macro and you have not defined
/// a startup function, it will be automatically declared and registered.
///
/// # Example
///
/// The `get_module` function is required in every PHP extension. This is a bare minimum example,
/// since the function is declared above the module it will automatically be registered when the
/// module attribute is called.
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_function]
/// pub fn hello(name: String) -> String {
///     format!("Hello, {}!", name)
/// }
///
/// #[php_module]
/// pub fn module(module: ModuleBuilder) -> ModuleBuilder {
///     module
/// }
/// ```
pub use ext_php_rs_derive::php_module;

/// Annotates a struct that will be exported to PHP as a class.
///
/// The struct that this attribute is used on must implement [`Default`], as this is used to
/// initialize the struct before the constructor is called. You may define a constructor with
/// the [`macro@php_impl`] macro which can modify the properties of the struct.
///
/// This attribute takes a set of optional arguments:
///
/// * `name` - The name of the exported class, if it is different from the Rust struct name. This
///    can be useful for namespaced classes, as you cannot place backslashes in Rust struct names.
///
/// Any struct that uses this attribute can also provide an optional set of extra attributes, used
/// to modify the class. These attributes must be used **underneath** this attribute, as they are
/// not valid Rust attributes, and instead are parsed by this attribute:
///
/// * `#[extends(ce)]` - Sets the parent class of this new class. Can only be used once, and `ce`
///   may be any valid expression.
/// * `#[implements(ce)]` - Implements an interface on the new class. Can be used multiple times,
///   and `ce` may be any valid expression.
///
/// This attribute (and its associated structs) must be defined *above* the startup function (which
/// is annotated by the [`macro@php_startup`] macro, or automatically generated just above the
/// [`macro@php_module`] function).
///
/// Fields defined on the struct *are not* the same as PHP properties, and are only accessible from
/// Rust.
///
/// # Example
///
/// Export a simple class called `Example`, with 3 Rust fields.
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_class]
/// #[derive(Default)]
/// pub struct Example {
///     x: i32,
///     y: String,
///     z: bool
/// }
///
/// #[php_module]
/// pub fn module(module: ModuleBuilder) -> ModuleBuilder {
///     module
/// }
/// ```
///
/// Create a custom exception `RedisException` inside the namespace `Redis\Exception`:
///
/// ```
/// # use ext_php_rs::prelude::*;
/// use ext_php_rs::php::exceptions::PhpException;
/// use ext_php_rs::php::class::ClassEntry;
///
/// #[php_class(name = "Redis\\Exception\\RedisException")]
/// #[extends(ClassEntry::exception())]
/// #[derive(Default)]
/// pub struct Example;
///
/// #[php_function]
/// pub fn throw_exception() -> Result<i32, PhpException<'static>> {
///     Err(PhpException::from_class::<Example>("Bad things happen".into()))
/// }
///
/// #[php_module]
/// pub fn module(module: ModuleBuilder) -> ModuleBuilder {
///     module
/// }
/// ```
pub use ext_php_rs_derive::php_class;

/// Annotates a function that will be called by PHP when the module starts up. Generally used to
/// register classes and constants.
///
/// As well as annotating the function, any classes and constants that had been declared using the
/// [`macro@php_class`], [`macro@php_const`] and [`macro@php_impl`] attributes will be registered
/// inside this function.
///
/// This function *must* be declared before the [`macro@php_module`] function, as this function
/// needs to be declared when building the module.
///
/// This function will automatically be generated if not already declared with this macro if you
/// have registered any classes or constants when using the [`macro@php_module`] macro.
///
/// # Example
///
/// ```
/// # use ext_php_rs::prelude::*;
/// #[php_startup]
/// pub fn startup_function() {
///     // do whatever you need to do...
/// }
/// # #[php_module]
/// # pub fn module(module: ModuleBuilder) -> ModuleBuilder {
/// #     module
/// # }
/// ```
pub use ext_php_rs_derive::php_startup;

/// A module typically glob-imported containing the typically required macros and imports.
pub mod prelude {
    pub use crate::php::module::ModuleBuilder;
    pub use crate::php::types::callable::Callable;
    #[cfg(any(docs, feature = "closure"))]
    #[cfg_attr(docs, doc(cfg(feature = "closure")))]
    pub use crate::php::types::closure::Closure;
    pub use crate::php_class;
    pub use crate::php_const;
    pub use crate::php_extern;
    pub use crate::php_function;
    pub use crate::php_impl;
    pub use crate::php_module;
    pub use crate::php_startup;
}
