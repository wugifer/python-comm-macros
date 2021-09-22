//!
//! Create a local variable __func__ = "xxx" in fn xxx()
//!
//! ## Usage
//!
//! ```
//! use python_comm_macros::auto_func_name;
//!
//! #[auto_func_name]
//! fn test_name() -> String {
//!   return String::from(__func__);
//! }
//!
//! assert_eq!(test_name(), "test_name");
//! ```
//!
//! ## Bug
//!
//! function with auto_func_name2 attribute, //! is not allowed internal!
//!

use proc_macro::TokenStream;

mod afn;


/// Create a local variable __func__ = "xxx" in fn xxx()
#[proc_macro_attribute]
pub fn auto_func_name(args: TokenStream, func: TokenStream) -> TokenStream {
    afn::auto_func_name(args, func)
}
