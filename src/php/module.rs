//! Builder and objects for creating modules in PHP. A module is the base of a PHP extension.

use std::{
    ffi::{c_void, CString},
    mem, ptr,
};

use crate::{
    bindings::{
        ext_php_rs_php_build_id, zend_module_entry, USING_ZTS, ZEND_DEBUG, ZEND_MODULE_API_NO,
    },
    errors::Result,
};

use super::function::FunctionEntry;

/// A Zend module entry. Alias.
pub type ModuleEntry = zend_module_entry;
/// A function to be called when the extension is starting up or shutting down.
pub type StartupShutdownFunc = extern "C" fn(_type: i32, _module_number: i32) -> i32;
/// A function to be called when `phpinfo();` is called.
pub type InfoFunc = extern "C" fn(zend_module: *mut ModuleEntry);

/// Builds a Zend extension. Must be called from within an external function called `get_module`,
/// returning a mutable pointer to a `ModuleEntry`.
///
/// ```
/// use ext_php_rs::{
///     php::module::{ModuleEntry, ModuleBuilder},
///     info_table_start, info_table_end, info_table_row
/// };
///
/// #[no_mangle]
/// pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
///     info_table_start!();
///     info_table_row!("column 1", "column 2");
///     info_table_end!();
/// }
///
/// #[no_mangle]
/// pub extern "C" fn get_module() -> *mut ModuleEntry {
///     ModuleBuilder::new("ext-name", "ext-version")
///         .info_function(php_module_info)
///         .build()
///         .unwrap()
///         .into_raw()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ModuleBuilder {
    name: String,
    version: String,
    module: ModuleEntry,
    functions: Vec<FunctionEntry>,
}

impl ModuleBuilder {
    /// Creates a new module builder with a given name and version.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the extension.
    /// * `version` - The current version of the extension. TBD: Deprecate in favour of the `Cargo.toml` version?
    pub fn new<T: Into<String>, U: Into<String>>(name: T, version: U) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            module: ModuleEntry {
                size: mem::size_of::<ModuleEntry>() as u16,
                zend_api: ZEND_MODULE_API_NO,
                zend_debug: ZEND_DEBUG as u8,
                zts: USING_ZTS as u8,
                ini_entry: ptr::null(),
                deps: ptr::null(),
                name: ptr::null(),
                functions: ptr::null(),
                module_startup_func: None,
                module_shutdown_func: None,
                request_startup_func: None,
                request_shutdown_func: None,
                info_func: None,
                version: ptr::null(),
                globals_size: 0,
                #[cfg(not(php_zts))]
                globals_ptr: ptr::null::<c_void>() as *mut c_void,
                #[cfg(php_zts)]
                globals_id_ptr: ptr::null::<c_void>() as *mut crate::bindings::ts_rsrc_id,
                globals_ctor: None,
                globals_dtor: None,
                post_deactivate_func: None,
                module_started: 0,
                type_: 0,
                handle: ptr::null::<c_void>() as *mut c_void,
                module_number: 0,
                build_id: unsafe { ext_php_rs_php_build_id() },
            },
            functions: vec![],
        }
    }

    /// Sets the startup function for the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called on startup.
    pub fn startup_function(mut self, func: StartupShutdownFunc) -> Self {
        self.module.module_startup_func = Some(func);
        self
    }

    /// Sets the shutdown function for the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called on shutdown.
    pub fn shutdown_function(mut self, func: StartupShutdownFunc) -> Self {
        self.module.module_shutdown_func = Some(func);
        self
    }

    /// Sets the request startup function for the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called when startup is requested.
    pub fn request_startup_function(mut self, func: StartupShutdownFunc) -> Self {
        self.module.module_startup_func = Some(func);
        self
    }

    /// Sets the request shutdown function for the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called when shutdown is requested.
    pub fn request_shutdown_function(mut self, func: StartupShutdownFunc) -> Self {
        self.module.module_shutdown_func = Some(func);
        self
    }

    /// Sets the extension information function for the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be called to retrieve the information about the extension.
    pub fn info_function(mut self, func: InfoFunc) -> Self {
        self.module.info_func = Some(func);
        self
    }

    /// Adds a function to the extension.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be added to the extension.
    pub fn function(mut self, func: FunctionEntry) -> Self {
        self.functions.push(func);
        self
    }

    /// Builds the extension and returns a `ModuleEntry`.
    ///
    /// Returns a result containing the module entry if successful.
    pub fn build(mut self) -> Result<ModuleEntry> {
        self.functions.push(FunctionEntry::end());
        self.module.functions =
            Box::into_raw(self.functions.into_boxed_slice()) as *const FunctionEntry;
        self.module.name = CString::new(self.name)?.into_raw();
        self.module.version = CString::new(self.version)?.into_raw();

        Ok(self.module)
    }
}

impl ModuleEntry {
    /// Converts the module entry into a raw pointer, releasing it to the C world.
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

/// Called by startup functions registered with the `#[php_startup]` macro. Initializes all
/// classes that are defined by ext-php-rs (i.e. [`Closure`]).
///
/// [`Closure`]: ext_php_rs::php::types::closure::Closure
#[doc(hidden)]
#[inline(always)]
pub fn ext_php_rs_startup() {
    #[cfg(feature = "closure")]
    crate::php::types::closure::Closure::build();
}
