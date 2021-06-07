//! Bindings for the Zend API to build PHP extensions natively in Rust.
//!
//! # Requirements
//!
//! - PHP 8.0 or later.
//!     - No support is planned for earlier versions.
//!     - The `php-config` executable is required.
//! - Linux or Darwin-based OS.
//!     - Windows is unsupported but support is planned.
//! - Rust.
//! - Clang 3.9 or later.
//!
//! # Usage
//!
//! PHP extensions are simply dynamically-linked libraries. Create a new Rust library:
//!
//! ```sh
//! $ cargo new ext_name --lib
//! ```
//!
//! Set the crate type to be a dynamically-linked library and require `ext-php-rs` in the
//! `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! ext-php-rs = "*"
//!
//! [lib]
//! crate-type = ["dylib"]
//! ```
//!
//! ## Module
//!
//! PHP extensions require one function to be exported - the `get_module` function, which defines
//! the module along with any functions and extra functions. This must be defined with the
//! `#[no_mangle]` attribute to prevent Rust from mangling the function name, preventing it from
//! being callable from PHP.
//!
//! It is recommended to provide an module information function, which is used by the `phpinfo()`
//! function to provide information about your module.
//!
//! There are four other function which you can provide depending on your requirements - the module and
//! request startup and shutdown functions. You can read up on these functions in the PHP Internals Book
//! [PHP lifecycle](https://www.phpinternalsbook.com/php7/extensions_design/php_lifecycle.html) chapter.
//!
//! Functions must also be registered with the module. This is covered in later sections.
//!
//! See the documentation for the [`ModuleBuilder`] class for more details.
//!
//! ```
//! # use ext_php_rs::{
//! #     info_table_start, info_table_row, info_table_end,
//! #     php::module::{
//! #         ModuleBuilder, ModuleEntry,
//! #     },
//! # };
//! #[no_mangle]
//! pub extern "C" fn get_module() -> *mut ModuleEntry {
//!     ModuleBuilder::new("ext_name", "0.0.1")
//!         .info_function(module_info)
//!         .build()
//!         .into_raw()
//! }
//!
//! pub extern "C" fn module_info(_mod: *mut ModuleEntry) {
//!     info_table_start!();
//!     
//!     // Provide information about your module
//!     info_table_row!("Name", "ext_name");
//!     info_table_row!("Version", "0.0.1");
//!     info_table_row!("some other field", "Hello!");
//!
//!     info_table_end!();
//! }
//! ```
//!
//! ## Functions
//!
//! Functions are defined in the `get_module` function when the module is defined. A function must
//! also be provided which will be called when the corresponding function is called from PHP.
//!
//! Arguments and return types must be defined when defining the function. These can be ommited
//! when the function does not take any arguments or does not return anything.
//!
//! The following code creates a function called `hello_world` which takes one argument, which is a
//! string, and returns a string.
//!
//! ```
//! # use ext_php_rs::{
//! #     parse_args,
//! #     php::{
//! #         module::{ModuleBuilder, ModuleEntry},
//! #         function::{FunctionBuilder},
//! #         args::{Arg},
//! #         enums::DataType,
//! #         execution_data::ExecutionData,
//! #         types::{
//! #             zval::Zval,
//! #         },
//! #     },
//! # };
//! #[no_mangle]
//! pub extern "C" fn get_module() -> *mut ModuleEntry {
//!     let hello_world = FunctionBuilder::new("hello_world", hello_world_func)
//!         .arg(Arg::new("name", DataType::String))
//!         .returns(DataType::String, false, false)
//!         .build();
//!
//!     ModuleBuilder::new("ext_name", "0.0.1")
//!         .function(hello_world)
//!         .build()
//!         .into_raw()
//! }
//!
//! pub extern "C" fn hello_world_func(ex: &mut ExecutionData, retval: &mut Zval) {
//!     let mut hello = Arg::new("name", DataType::String);
//!
//!     parse_args!(ex, hello);
//!
//!     retval.set_string(format!("Hello, {}!", hello.val::<String>().unwrap()));
//! }
//! ```
//!
//! Functions can also take optional arguments. These must be seperated in the function definition
//! by a `not_required()` function call on the function builder:
//!
//! ```ignore
//! let hello_world = FunctionBuilder::new("hello_world", hello_world_func)
//!     .arg(Arg::new("name", DataType::String))
//!     .not_required()
//!     .arg(Arg::new("not_req", DataType::Long))
//!     .arg(Arg::new("not_req_2", DataType::Double))
//!     .returns(DataType::String, false, false)
//!     .build();
//! ```
//!
//! The optional argument must also be passed to the `parse_args!` macro in the corresponding
//! function. Notice the semi-colon seperating the required and not required arguments.
//!
//! ```
//! # use ext_php_rs::{
//! #     parse_args,
//! #     php::{
//! #         args::{Arg},
//! #         enums::DataType,
//! #         execution_data::ExecutionData,
//! #         types::{
//! #             zval::Zval,
//! #         },
//! #     },
//! # };
//! pub extern "C" fn hello_world_func(ex: &mut ExecutionData, retval: &mut Zval) {
//!     let mut hello = Arg::new("name", DataType::String);
//!     let mut not_req = Arg::new("not_req", DataType::Long);
//!     let mut not_req_2 = Arg::new("not_req_2", DataType::Double);
//!
//!     parse_args!(ex, hello; not_req, not_req_2);
//!
//!     retval.set_string(format!("Hello, {}!", hello.val::<String>().unwrap()));
//! }
//! ```
//!
//! See the documentation for [`FunctionBuilder`] and [`parse_args!`] for more details.
//!
//! ## Global Constants
//!
//! Constants must be registered in the module startup function. Registering constants is provided
//! by the [`IntoConst`] trait, which is implemented for most string, boolean and integer types in
//! Rust. The full list can be viewed [here](php/constants/trait.IntoConst.html#foreign-impls).
//!
//! A couple examples of registering constants:
//!
//! ```
//! use ext_php_rs::php::constants::IntoConst;
//!
//! pub extern "C" fn module_init(_type: i32, module_number: i32) -> i32 {
//!     "Hello, world".register_constant("HELLO_WORLD", module_number);
//!     15125.register_constant("A_NUMBER", module_number);
//!     0 // return 0 for success, -1 for fail
//! }
//! ```
//!
//! Make sure to pass the function pointer to the module builder:
//!
//! ```ignore
//! ModuleBuilder::new("ext_name", "0.0.1")
//!     .startup_function(module_init)
//!     .build()
//!     .into_raw()
//! ```
//!
//! ## Classes
//!
//! Classes must also be registered in the module startup function. The [`ClassBuilder`] is used
//! for building classes. We can build a simple class called `TestClass` with one function called
//! `TestClass::hello` and a constant `TestClass::TEST_CONST`:
//!
//! ```
//! use ext_php_rs::php::{
//!     execution_data::ExecutionData,
//!     function::FunctionBuilder,
//!     class::ClassBuilder,
//!     flags::MethodFlags,
//!     types::zval::Zval,
//! };
//!
//! pub extern "C" fn hello(_: &mut ExecutionData, _: &mut Zval) {}
//!
//! pub extern "C" fn module_init(_type: i32, module_number: i32) -> i32 {
//!     let hello = FunctionBuilder::new("hello", hello)
//!         .build();
//!
//!     ClassBuilder::new("TestClass")
//!         .method(hello, MethodFlags::Public)
//!         .constant("TEST_CONST", "Constant!")
//!         .build();
//!     0
//! }
//! ```
//!
//! Methods are registered in the same way that functions are registered, however, they also have a
//! visibility flag - one of `Public`, `Protected` or `Private`, similar to a normal PHP class.
//!
//! You will likely want to store some data with the class object. You can define a Rust type which
//! will represent the class and pass it to PHP. The type *must* implement the [`Default`] trait,
//! as well as implementing the [`ZendObjectOverride`] trait (which can be derived using the
//! [`ZendObjectHandler`] macro):
//!
//! ```
//! use ext_php_rs::php::{
//!     ZendObjectHandler,
//!     execution_data::ExecutionData,
//!     function::FunctionBuilder,
//!     class::ClassBuilder,
//!     flags::MethodFlags,
//!     types::zval::Zval,
//! };
//!
//! #[derive(ZendObjectHandler)]
//! struct MyClass {
//!     a: i32,
//!     b: i32,
//!     c: String,
//! }
//!
//! impl Default for MyClass {
//!     fn default() -> Self {
//!         Self {
//!             a: 15,
//!             b: 30,
//!             c: "Hello".into(),
//!         }
//!     }
//! }
//!
//! impl MyClass {
//!     pub extern "C" fn constructor(ex: &mut ExecutionData, _: &mut Zval) {
//!
//!     }
//! }
//! ```
//!
//! [`IntoConst`]: crate::php::constants::IntoConst
//! [`ModuleBuilder`]: crate::php::module::ModuleBuilder
//! [`FunctionBuilder`]: crate::php::function::FunctionBuilder
//! [`ClassBuilder`]: crate::php::class::ClassBuilder
//! [`parse_args!`]: crate::parse_args
//! [`ZendObjectOverride`]: crate::php::types::object::ZendObjectOverride
//! [`ZendObjectHandler`]: crate::ZendObjectHandler

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[macro_use]
pub mod macros;
pub mod bindings;
pub mod errors;
pub mod functions;
pub mod php;

/// Derives the implementation of `ZendObjectOverride` for the given object. Allows the
/// object to be used as a Zend object, storing Rust information inside a PHP class.
///
/// The type that the macro is used on *must* implement the [`Default`] trait.
///
/// # Example
///
/// ```
/// use ext_php_rs::{ZendObjectHandler, php::class::ClassBuilder};
///
/// #[derive(Default, ZendObjectHandler)]
/// struct Class {
///     x: i32,
///     y: f64,
///     z: String,
/// }
///
/// pub extern "C" fn module_init(_type: i32, module_number: i32) -> i32 {
///     ClassBuilder::new("TestClass")
///         .object_override::<Class>()
///         .build();
///     0
/// }
/// ```
pub use ext_php_rs_derive::ZendObjectHandler;
