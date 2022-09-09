//!
//! macros for python-comm
//!

use chrono::{Duration, Utc};
use proc_macro::TokenStream;
use quote::quote;

mod afn;
mod asm;

/// AsSqlModel
#[proc_macro_derive(AsSqlModel)]
pub fn as_sql_model(input: TokenStream) -> TokenStream {
    asm::as_sql_model(input)
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
