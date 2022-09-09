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
            .filter(|x| {
                !x.1.ident
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .ends_with("_default")
            })
            .map(|x| {
                (
                    x.0,
                    x.1.ident.as_ref().unwrap(),
                    &x.1.ty,
                    x.0 == fields.len() - 1,
                )
            })
            .map(mapper),
    )
}

/// 在全部字段上执行并 join
fn map_fields_and_join<F>(fields: &Vec<Ident>, mapper: F, sep: TokenStream2) -> TokenStream2
where
    F: FnMut(&Ident) -> Option<TokenStream2>,
{
    TokenStream2::from_iter(
        fields
            .iter()
            .filter_map(mapper)
            .enumerate()
            .map(|(i, x)| {
                if i == 0 {
                    vec![x]
                } else {
                    vec![sep.clone(), x]
                }
            })
            .flatten(),
    )
}

pub fn as_sql_model(input: TokenStream) -> TokenStream {
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

            // 解析每个字段, 设置同名变量
            let field_from_row = map_fields(&fields, |(i, ident, ty, _last)| {
                quote!(
                    let #ident = match row.take(#i) {
                        Some(value) => match #ty::get_intermediate(value) {
                            Ok(ir) => ir,
                            Err(mysql::FromValueError(value)) => {
                                row.place(#i, value);
                                return Err(mysql::FromRowError(row));
                            }
                        },
                        None => return Err(mysql::FromRowError(row)),
                    }
                    .commit();
                )
            });

            // 用于测试, 快速设置每个字段
            let assign = map_fields(&fields, |(_i, ident, ty, _last)| {
                quote!(
                    #[cfg(test)]
                    pub fn #ident<T>(mut self, v: T) -> Self
                    where
                        T: Into<#ty>,
                    {
                        self.#ident = v.into();
                        self
                    }
                )
            });

            // 遍历每个字段, 得到变量名列表
            let mut field_idents: Vec<Ident> = vec![];
            let _ = map_fields(&fields, |(_i, ident, _ty, _last)| {
                field_idents.push(ident.clone());
                quote!(())
            });

            // 遍历每个字段, 得到变量名列表
            let mut field_names: Vec<String> = vec![];
            let _ = map_fields(&fields, |(_i, ident, _ty, _last)| {
                field_names.push(ident.to_string());
                quote!(())
            });

            // a, b, c
            let fields_strip_comma =
                map_fields_and_join(&field_idents, |ident| Some(quote!(#ident)), quote!(, ));

            // a, b, c,
            let fields_with_comma =
                map_fields_and_join(&field_idents, |ident| Some(quote!(#ident, )), quote!());

            // `a`, `b`, `c`
            let fields_with_backquote = {
                let code = field_names
                    .iter()
                    .map(|x| format!("`{}`", x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };
            let fields_with_backquote_without_id = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| format!("`{}`", x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // :a, :b, :c
            let field_saves = {
                let code = field_names
                    .iter()
                    .map(|x| format!(":{}", x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };
            let field_saves_without_id = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| format!(":{}", x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // a=:a, b=:b, c=:c
            let field_updates = {
                let code = field_names
                    .iter()
                    .map(|x| format!("{}=:{}", x, x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };
            let field_updates_without_id = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| format!("{}=:{}", x, x))
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // vec![("a", self.a), ("b", self.b)]
            let field_values = {
                let code = map_fields_and_join(
                    &field_idents,
                    |ident| Some(quote!((stringify!(#ident), self.#ident.clone().into()))),
                    quote!(,),
                );
                quote!(
                    let v: Vec<(&str, mysql::Value)> = vec![ #code ];
                    mysql::params::Params::from(v)
                )
            };
            let field_values_without_id = {
                let code = map_fields_and_join(
                    &field_idents,
                    |ident| {
                        if ident != "id" {
                            Some(quote!((stringify!(#ident), self.#ident.clone().into())))
                        } else {
                            None
                        }
                    },
                    quote!(,),
                );
                quote!(
                    let v: Vec<(&str, mysql::Value)> = vec![ #code ];
                    mysql::params::Params::from(v)
                )
            };

            // self.a==other.a && self.b==other.b
            let field_equal = map_fields_and_join(
                &field_idents,
                |ident| {
                    Some(quote!(
                         self.#ident == other.#ident
                    ))
                },
                quote!(&&),
            );
            let field_equal_without_id = map_fields_and_join(
                &field_idents,
                |ident| {
                    if ident != "id" {
                        Some(quote!(self.#ident == other.#ident))
                    } else {
                        None
                    }
                },
                quote!(&&),
            );

            // 汇总代码
            let result = quote!(
                impl #struct_ident {
                    /// 比较两个 obj
                    pub fn equal(&self, other: &Self) -> bool {
                        #field_equal
                    }

                    /// 比较两个 obj, 排除 id
                    pub fn equal_without_id(&self, other: &Self) -> bool {
                        #field_equal_without_id
                    }

                    /// 字段名: a, b, c
                    pub fn fields_strip_comma() -> &'static str {
                        stringify!(#fields_strip_comma)
                    }

                    /// 字段名: a, b, c,
                    pub fn fields_with_comma() -> &'static str {
                        stringify!(#fields_with_comma)
                    }

                    /// 字段保存: :a, :b
                    pub fn field_saves() -> &'static str {
                        #field_saves
                    }

                    /// 字段更新: a=:a, b=:b
                    pub fn field_updates() -> &'static str {
                        #field_updates
                    }

                    /// 字段内容
                    pub fn field_values(&self) -> mysql::params::Params {
                        #field_values
                    }

                    #assign
                }

                impl SqlModelPlus for #struct_ident {
                    /// 字段保存, 排除 id: :a, :b
                    fn field_saves_without_id() -> &'static str {
                        #field_saves_without_id
                    }

                    /// 字段更新, 排除 id: a=:a, b=:b
                    fn field_updates_without_id() -> &'static str {
                        #field_updates_without_id
                    }

                    /// 字段内容, 排除 id
                    fn field_values_without_id(&self) -> mysql::params::Params {
                        #field_values_without_id
                    }

                    /// 字段名, 排除 id: `a`, `b`, `c`
                    fn fields_with_backquote() -> &'static str {
                        #fields_with_backquote
                    }

                    /// 字段名, 排除 id: `a`, `b`, `c`
                    fn fields_with_backquote_without_id() -> &'static str {
                        #fields_with_backquote_without_id
                    }

                    /// 返回加锁的 DbPool, 注意类名写死了, 使用者需命名并引入 WhoCreateDbPool
                    #[auto_func_name]
                    fn lock() -> Result<std::sync::MutexGuard<'static, python_comm::use_sql::DbPool>, python_comm::use_m::MoreError> {
                        WhoCreateDbPool::lock().m(m!(__func__))
                    }
                }

                impl mysql::prelude::FromRow for #struct_ident {
                    fn from_row_opt(mut row: mysql::Row) -> Result<Self, mysql::FromRowError> {
                        #field_from_row
                        Ok(Self {
                            #fields_with_comma
                        })
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
