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
                    .starts_with("_renames_")
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

/// 在全部字段上寻找重命名字段
fn map_fields_get_rename(fields: &Fields) -> TokenStream2 {
    for field in fields {
        let ident = &field.ident;
        let s = ident.as_ref().unwrap().to_string();
        if s.starts_with("_renames_") {
            return quote!(#ident: 0);
        }
    }

    quote!()
}

/// 在全部字段上寻找重命名字段
fn map_fields_rename(fields: &Fields, name: &str) -> String {
    for field in fields {
        let s = field.ident.as_ref().unwrap().to_string();
        if !s.starts_with("_renames_") {
            continue;
        }

        let mut found = false;
        for token in s.split("_r_") {
            if found {
                return token.to_string();
            }
            if token == name {
                found = true;
            }
        }
    }

    name.to_string()
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

            let rename_field = map_fields_get_rename(&fields);

            // C-有逗号结尾, Q-有双引号, B-有反引号, I-去掉 id, P-作为参数, E-赋值, V-Value, EE-相等

            // a, b, c
            // let make_fields =
            //     map_fields_and_join(&field_idents, |ident| Some(quote!(#ident)), quote!(, ));

            // a, b, c,
            let make_fields_c =
                map_fields_and_join(&field_idents, |ident| Some(quote!(#ident, )), quote!());

            // `a`, `b`, `c`
            let make_fields_b = {
                let code = field_names
                    .iter()
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("`{}`", real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // `a`, `b`, `c`
            let make_fields_bi = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("`{}`", real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // a=:a, b=:b, c=:c
            let make_fields_e = {
                let code = field_names
                    .iter()
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("{}=:{}", real, real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // a=:a, b=:b, c=:c
            let make_fields_ei = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("{}=:{}", real, real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // self.a==other.a && self.b==other.b
            let make_fields_ee = map_fields_and_join(
                &field_idents,
                |ident| {
                    Some(quote!(
                         self.#ident == other.#ident
                    ))
                },
                quote!(&&),
            );

            // self.a==other.a && self.b==other.b
            let make_fields_eei = map_fields_and_join(
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

            // :a, :b, :c
            let make_fields_p = {
                let code = field_names
                    .iter()
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!(":{}", real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // :a, :b, :c
            let make_fields_pi = {
                let code = field_names
                    .iter()
                    .filter(|x| x.as_str() != "id")
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!(":{}", real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // "a", "b", "c"
            let make_fields_q = {
                let code = field_names
                    .iter()
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("\"{}\"", real)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                quote!(#code)
            };

            // "a", "b", "c",
            let make_fields_qc = {
                let code = field_names
                    .iter()
                    .map(|x| {
                        let real = map_fields_rename(&fields, &x);
                        format!("\"{}\", ", real)
                    })
                    .collect::<Vec<String>>()
                    .join("");
                quote!(#code)
            };

            // vec![("a", self.a), ("b", self.b)]
            let make_fields_v = {
                let code = map_fields_and_join(
                    &field_idents,
                    |ident| {
                        let real = map_fields_rename(&fields, &ident.to_string());
                        Some(quote!((stringify!(#real), self.#ident.clone().into())))
                    },
                    quote!(,),
                );
                quote!(
                    let v: Vec<(&str, mysql::Value)> = vec![ #code ];
                    mysql::params::Params::from(v)
                )
            };

            // vec![("a", self.a), ("b", self.b)]
            let make_fields_vi = {
                let code = map_fields_and_join(
                    &field_idents,
                    |ident| {
                        if ident != "id" {
                            let real = map_fields_rename(&fields, &ident.to_string());
                            Some(quote!((#real, self.#ident.clone().into())))
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

            // 汇总代码
            let result = quote!(
                impl #struct_ident {
                    /// 比较两个 obj
                    pub fn equal(&self, other: &Self) -> bool {
                        #make_fields_ee
                    }

                    /// 比较两个 obj, 排除 id
                    pub fn equal_without_id(&self, other: &Self) -> bool {
                        #make_fields_eei
                    }

                    #assign
                }

                impl SqlModelPlus for #struct_ident {
                    /// 返回加锁的 DbPool, 注意类名写死了, 使用者需命名并引入 WhoCreateDbPool
                    #[auto_func_name]
                    fn lock() -> Result<std::sync::MutexGuard<'static, python_comm::use_sql::DbPool>, python_comm::use_m::MoreError> {
                        WhoCreateDbPool::lock().m(m!(__func__))
                    }

                    /// `a`, `b`, `c`
                    fn make_fields_b() -> &'static str {
                        #make_fields_b
                    }

                    /// `a`, `b`, `c`
                    fn make_fields_bi() -> &'static str {
                        #make_fields_bi
                    }

                    /// a=:a, b=:b
                    fn make_fields_e() -> &'static str {
                        #make_fields_e
                    }

                    /// a=:a, b=:b
                    fn make_fields_ei() -> &'static str {
                        #make_fields_ei
                    }

                    /// :a, :b
                    fn make_fields_p() -> &'static str {
                        #make_fields_p
                    }

                    /// :a, :b
                    fn make_fields_pi() -> &'static str {
                        #make_fields_pi
                    }

                    /// "a", "b", "c"
                    fn make_fields_q() -> &'static str {
                        #make_fields_q
                    }

                    /// "a", "b", "c",
                    fn make_fields_qc() -> &'static str {
                        #make_fields_qc
                    }

                    /// vec![("a", self.a), ("b", self.b)]
                    fn make_fields_v(&self) -> mysql::params::Params {
                        #make_fields_v
                    }

                    /// vec![("a", self.a), ("b", self.b)]
                    fn make_fields_vi(&self) -> mysql::params::Params {
                        #make_fields_vi
                    }
                }

                impl mysql::prelude::FromRow for #struct_ident {
                    fn from_row_opt(mut row: mysql::Row) -> Result<Self, mysql::FromRowError> {
                        #field_from_row
                        Ok(Self {
                            #make_fields_c
                            #rename_field
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
