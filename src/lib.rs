//!
//! macros for python-comm
//!

use chrono::{Duration, Utc};
use proc_macro::TokenStream;
use quote::quote;

mod afn;
mod asx;

/// AsDefaultStruct
#[proc_macro_derive(AsDefaultStruct)]
pub fn as_default_struct(input: TokenStream) -> TokenStream {
    asx::as_default_struct(input)
}

/// AsPythonDict, no_simple_cov
#[proc_macro_derive(AsPythonDict)]
pub fn as_python_dict(input: TokenStream) -> TokenStream {
    asx::as_python_dict(input)
}

/// AsPythonObject, no_simple_cov
#[proc_macro_derive(AsPythonObject)]
pub fn as_python_object(input: TokenStream) -> TokenStream {
    asx::as_python_object(input)
}

/// AsSqlTable, no_simple_cov
#[proc_macro_derive(AsSqlTable)]
pub fn as_sql_table(input: TokenStream) -> TokenStream {
    asx::as_sql_table(input)
}

/// Create a local variable __func__ = "xxx" in fn xxx()
///
/// ## Bug
///
/// function with auto_func_name attribute, //! is not allowed internal!
///
#[proc_macro_attribute]
pub fn auto_func_name(args: TokenStream, func: TokenStream) -> TokenStream {
    afn::auto_func_name(args, func)
}

/// build time
#[proc_macro]
pub fn build_time(_input: TokenStream) -> TokenStream {
    let now = (Utc::now() + Duration::hours(8))
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    TokenStream::from(quote!(#now))
}
