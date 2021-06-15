use proc_macro::TokenStream;
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
