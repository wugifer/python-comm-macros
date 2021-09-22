use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Create a local variable __func__ = "xxx" in fn xxx()
pub fn auto_func_name(args: TokenStream, function: TokenStream) -> TokenStream {
    let debug = args.to_string() == "\"debug\"";
    if debug {
        println!("before: {:?}", function);
    }

    let function_clone = function.clone();

    // 解析 function, 获取 function name
    let func = parse_macro_input!(function as ItemFn);
    let sig = &func.sig;
    let func_ident = &sig.ident; // function name

    // 准备重构 function, 目前没找到替换 TokenStream 中一个元素的方法, 只能全部重构
    let mut all_stream: TokenStream = TokenStream::new();

    // 寻找 {} group, 增加 __func__
    for token in function_clone {
        let token_clone = token.clone();
        if let TokenTree::Group(group) = token {
            if group.delimiter() == Delimiter::Brace {
                // 找到的第一个合适 group 是函数体, new = let + old_stream
                let old_stream = proc_macro2::TokenStream::from(group.stream());
                let mut new_stream = quote! {
                    let __func__: &str = stringify!(#func_ident);
                };
                new_stream.extend(old_stream);
                all_stream.extend(TokenStream::from(TokenTree::Group(Group::new(
                    Delimiter::Brace,
                    new_stream.into(),
                ))));
                continue;
            }
        }

        all_stream.extend(TokenStream::from(token_clone));
    }

    if debug {
        println!(" after: {:?}", all_stream);
    }

    all_stream
}
