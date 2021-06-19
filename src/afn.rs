use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn auto_func_name(function: TokenStream) -> TokenStream {
    //* 自动设置 __func__ 变量为当前函数名

    let func = parse_macro_input!(function as ItemFn);
    let func_vis = &func.vis; // like pub
    let sig = &func.sig;
    let func_block = &func.block; // { some statement or expression here }

    let func_ident = &sig.ident; // function name
    let func_generics = &sig.generics;
    let func_inputs = &sig.inputs;
    let func_output = &sig.output;

    let caller = quote! {
        // rebuild the function, add a func named is_expired to check user login session expire or not.
        #func_vis fn #func_ident #func_generics(#func_inputs) #func_output {
            let __func__: &str = stringify!(#func_ident);
            #func_block
        }
    };

    caller.into()
}

pub fn auto_func_name2(function: TokenStream) -> TokenStream {
    //* 自动设置 __func__ 变量为当前函数名
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

    all_stream
}
