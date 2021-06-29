//!
//! 在函数中自动设置局部变量 \_\_func\_\_ 为当前函数名
//!
//! ## 用法
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
//! 有 auto_func_name 属性的函数, 用 /// 产生的文档注释在 cargo doc 中为空, 改为 auto_func_name2 解决
//!
//! 有 auto_func_name 属性的函数, 用 //! 在内部注释导致 rust 报错时找不到代码
//!
//! 有 auto_func_name2 属性的函数, 内部不允许用 //! 生成注释
//!

use proc_macro::TokenStream;

mod afn;

/// 在函数中自动设置局部变量 \_\_func\_\_ 为当前函数名

#[proc_macro_attribute]
pub fn auto_func_name(_args: TokenStream, func: TokenStream) -> TokenStream {
    //* 自动设置 __func__ 变量为当前函数名

    afn::auto_func_name(func)
}

#[proc_macro_attribute]
pub fn auto_func_name2(args: TokenStream, func: TokenStream) -> TokenStream {
    //* 自动设置 __func__ 变量为当前函数名

    afn::auto_func_name2(args, func)
}
