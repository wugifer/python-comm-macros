use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::iter::FromIterator;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

/// 在全部字段上执行
fn map_fields<F>(fields: &Fields, mapper: F) -> TokenStream2
where
    F: FnMut((usize, &Ident, &Type, bool)) -> TokenStream2,
{
    TokenStream2::from_iter(
        fields
            .iter()
            .enumerate()
            .filter(|x| !x.1.ident.as_ref().unwrap().to_string().starts_with("_renames_"))
            .map(|x| (x.0, x.1.ident.as_ref().unwrap(), &x.1.ty, x.0 == fields.len() - 1))
            .map(mapper),
    )
}

pub fn limit_pack(input: TokenStream) -> TokenStream {
    // 输入
    let input = parse_macro_input!(input as DeriveInput);

    // 类名
    let struct_ident = input.ident;

    // 仅处理 struct
    if let Data::Struct(input_struct) = input.data {
        let fields = input_struct.fields;

        // 仅处理命名成员变量
        if matches!(&fields, Fields::Named(_)) {
            // 通过 map_fields 处理每个字段, 生成特定代码

            let field_to_limit_str = map_fields(&fields, |(_i, ident, _ty, _last)| {
                quote!(
                    python_comm::use_limit_pack::ForStruct{
                        k: stringify!(#ident).to_limit_str(limit),
                        v: &self.#ident,
                    },
                )
            });

            // 汇总代码
            let result = quote!(
                impl python_comm::use_limit_pack::LimitPackAble for #struct_ident {
                    fn to_limit_str(&self, limit: &mut Limit) -> String {
                        limit.push_and_inc();
                        let data = (
                            #field_to_limit_str
                        );
                        let pair_seq = limit.pop_start();
                        let text = data.to_limit_str(limit);
                        limit.pop_end(pair_seq);
                        format!("{}{}", stringify!(#struct_ident), text)
                    }
                }
            )
            .into();

            // 调试时输出代码
            // eprintln!("{}", result);
            return result;
        }
    }

    // 缺省, 空
    quote!().into()
}
