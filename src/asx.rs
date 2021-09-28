use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use std::iter::FromIterator;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

/// 在全部字段上执行, no_simple_cov
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

pub fn as_default_struct(input: TokenStream) -> TokenStream {
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
            let field_assign = map_fields(&fields, |(_i, ident, ty, _last)| {
                quote!(
                    #[allow(dead_code)]
                    pub fn #ident(mut self, v: #ty) -> Self {
                        self.#ident = v;
                        self
                    }
                )
            });

            // 汇总代码
            let result = quote!(
                impl #struct_ident {
                    #field_assign
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

pub fn as_python_dict(input: TokenStream) -> TokenStream {
    // 输入
    let input = parse_macro_input!(input as DeriveInput);

    // 类名
    let struct_ident = input.ident;
    let py_ident = Ident::new(&format!("Py{}", struct_ident), struct_ident.span());

    // 仅处理 struct
    if let Data::Struct(input_struct) = input.data {
        let fields = input_struct.fields;

        // 仅处理命名成员变量
        if matches!(&fields, Fields::Named(_)) {
            // 通过 map_fields 处理每个字段, 生成特定代码
            let field_def = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "Decimal" {
                    quote!( #ident: python_comm::types::PyDecimal, )
                } else if ty.to_token_stream().to_string() == "NaiveDate" {
                    quote!( #ident: python_comm::types::PyNaiveDate, )
                } else if ty.to_token_stream().to_string() == "NaiveDateTime" {
                    quote!( #ident: python_comm::types::PyNaiveDateTime, )
                } else {
                    quote!( #ident: #ty, )
                }
            });

            let field_from = map_fields(&fields, |(_i, ident, _ty, _last)| {
                quote!(
                    #ident: dict
                        .get_item(stringify!(#ident))
                        .ok_or(raise_error!(
                            "raw",
                            "py",
                            __func__,
                            format!(r#"get_item("{}") error"#, stringify!(#ident))
                        ))?
                        .extract()
                        .or_else(|err| raise_error!("py", __func__, "\n", err))?,
                )
            });

            let field_into = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "Decimal" {
                    quote!(
                        let _ = dict.set_item(
                            stringify!(#ident),
                            pyo3::IntoPy::<pyo3::PyObject>::into_py(
                                python_comm::types::PyDecimal(self.#ident),
                                python
                            ),
                        );
                    )
                } else if ty.to_token_stream().to_string() == "NaiveDate" {
                    quote!(
                        let _ = dict.set_item(
                            stringify!(#ident),
                            pyo3::IntoPy::<pyo3::PyObject>::into_py(
                                python_comm::types::PyNaiveDate(self.#ident),
                                python
                            ),
                        );
                    )
                } else if ty.to_token_stream().to_string() == "NaiveDateTime" {
                    quote!(
                        let _ = dict.set_item(
                            stringify!(#ident),
                            pyo3::IntoPy::<pyo3::PyObject>::into_py(
                                python_comm::types::PyNaiveDateTime(self.#ident),
                                python
                            ),
                        );
                    )
                } else {
                    quote!(
                        let _ = dict.set_item(
                            stringify!(#ident),
                            pyo3::IntoPy::<pyo3::PyObject>::into_py(self.#ident, python),
                        );
                    )
                }
            });

            let field_into_py = map_fields(
                &fields,
                |(_i, ident, _ty, _last)| quote!( #ident: self.#ident.into(), ),
            );

            // 汇总代码
            let result = quote!(
                struct #py_ident {
                    #field_def
                }

                impl pyo3::FromPyObject<'_> for #struct_ident {
                    #[auto_func_name]
                    fn extract(obj: &pyo3::types::PyAny) -> Result<Self, pyo3::PyErr> {
                        let pyobj = #py_ident::extract(obj)
                            .or_else(|err| raise_error!("py", __func__, "\n", err))?;
                        Ok(pyobj.into())
                    }
                }

                impl pyo3::IntoPy<pyo3::PyObject> for #struct_ident {
                    fn into_py(self, python: pyo3::Python) -> pyo3::PyObject {
                        let dict = pyo3::types::PyDict::new(python);
                        #field_into
                        dict.into()
                    }
                }

                impl pyo3::FromPyObject<'_> for #py_ident {
                    #[auto_func_name]
                    fn extract(obj: &pyo3::types::PyAny) -> Result<Self, pyo3::PyErr> {
                        let dict: &pyo3::types::PyDict = obj.cast_as()
                            .or_else(|err| raise_error!("py", __func__, "\n", err))?;
                        Ok(Self {
                            #field_from
                        })
                    }
                }

                impl Into<#struct_ident> for #py_ident {
                    fn into(self) -> #struct_ident {
                        #struct_ident {
                            #field_into_py
                        }
                    }
                }
            )
            .into();

            // 调试时输出代码
            // eprintln!("{}", result);
            return result;
        }
    }

    quote!().into()
}

pub fn as_python_object(input: TokenStream) -> TokenStream {
    // 输入
    let input = parse_macro_input!(input as DeriveInput);

    // 类名
    let struct_ident = input.ident;
    let py_ident = Ident::new(&format!("Py{}", struct_ident), struct_ident.span());

    // 仅处理 struct
    if let Data::Struct(input_struct) = input.data {
        let fields = input_struct.fields;
        // 仅处理命名成员变量
        if matches!(&fields, Fields::Named(_)) {
            // 通过 map_fields 处理每个字段, 生成特定代码
            let field_def = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "Decimal" {
                    quote!( #ident: python_comm::types::PyDecimal, )
                } else if ty.to_token_stream().to_string() == "NaiveDate" {
                    quote!( #ident: python_comm::types::PyNaiveDate, )
                } else if ty.to_token_stream().to_string() == "NaiveDateTime" {
                    quote!( #ident: python_comm::types::PyNaiveDateTime, )
                } else {
                    quote!( #ident: #ty, )
                }
            });

            let field_from = map_fields(&fields, |(_i, ident, _ty, _last)| {
                quote!(
                    #ident: obj
                        .getattr(stringify!(#ident))
                        .or_else(|err| raise_error!("py", __func__, "\n", err))?
                        .extract()
                        .or_else(|err| raise_error!("py", __func__, "\n", err))?,
                )
            });

            let field_into = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "Decimal" {
                    quote!(
                        let _ = obj.setattr(stringify!(#ident), python_comm::types::PyDecimal(self.#ident));
                    )
                } else if ty.to_token_stream().to_string() == "NaiveDate" {
                    quote!(
                        let _ = obj.setattr(stringify!(#ident), python_comm::types::PyNaiveDate(self.#ident));
                    )
                } else if ty.to_token_stream().to_string() == "NaiveDateTime" {
                    quote!(
                        let _ = obj.setattr(stringify!(#ident), python_comm::types::PyNaiveDateTime(self.#ident));
                    )
                } else {
                    quote!( let _ = obj.setattr(stringify!(#ident), self.#ident); )
                }
            });

            let field_into_py = map_fields(
                &fields,
                |(_i, ident, _ty, _last)| quote!( #ident: self.#ident.into(), ),
            );

            // 汇总代码
            let result = quote!(
                struct #py_ident {
                    #field_def
                }

                impl pyo3::FromPyObject<'_> for #struct_ident {
                    #[auto_func_name]
                    fn extract(obj: &pyo3::types::PyAny) -> Result<Self, pyo3::PyErr> {
                        let pyobj = #py_ident::extract(obj).or_else(|err| raise_error!("py", __func__, "\n", err))?;
                        Ok(pyobj.into())
                    }
                }

                impl pyo3::IntoPy<pyo3::PyObject> for #struct_ident {
                    fn into_py(self, python: pyo3::Python) -> pyo3::PyObject {
                        let out = python_comm::types::PyClassObject {}.into_py(python);
                        if let Ok(obj) = out.extract::<&pyo3::types::PyAny>(python) {
                            #field_into
                        }
                        out
                    }
                }

                impl pyo3::FromPyObject<'_> for #py_ident {
                    #[auto_func_name]
                    fn extract(obj: &pyo3::types::PyAny) -> Result<Self, pyo3::PyErr> {
                        Ok(Self {
                            #field_from
                        })
                    }
                }

                impl Into<#struct_ident> for #py_ident {
                    fn into(self) -> #struct_ident {
                        #struct_ident {
                            #field_into_py
                        }
                    }
                }
            )
            .into();

            // 调试时输出代码
            // eprintln!("{}", result);
            return result;
        }
    }

    quote!().into()
}

pub fn as_sql_table(input: TokenStream) -> TokenStream {
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

            // 用不上, 不要删除, 这种语法有用
            // let field_init = map_fields(&fields, |(i, ident, _ty, _last)| {
            //     let i = syn::Index::from(i);
            //     quote!(#ident: row.#i, )
            // });

            let field_name = map_fields(&fields, |(_i, ident, _ty, _last)| quote!(#ident, ));

            let mut field_set1_s = String::new();
            let mut field_set3_s = String::new();
            let mut field_set4_s = String::new();

            let _field_set1 = map_fields(&fields, |(_i, ident, _ty, last)| {
                if last {
                    field_set1_s += &format!("{}=:{}", ident, ident);
                    field_set3_s += &format!("\"{}\"", ident);
                    field_set4_s += &format!(":{}", ident);
                } else {
                    field_set1_s += &format!("{}=:{}, ", ident, ident);
                    field_set3_s += &format!("\"{}\", ", ident);
                    field_set4_s += &format!(":{}, ", ident);
                }
                quote!(())
            });

            let field_set2 = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "String" {
                    quote!(stringify!(#ident) => self.#ident.clone(), )
                } else if ty.to_token_stream().to_string() == "Date" {
                    quote!(stringify!(#ident) => self.#ident.s().clone(), )
                } else {
                    quote!(stringify!(#ident) => self.#ident, )
                }
            });

            let field_set5 = map_fields(&fields, |(_i, ident, ty, _last)| {
                if ty.to_token_stream().to_string() == "String" {
                    quote!(stringify!(#ident) => self.#ident.clone(),)
                } else if ty.to_token_stream().to_string() == "Date" {
                    quote!(stringify!(#ident) => self.#ident.s().clone(),)
                } else {
                    quote!(stringify!(#ident) => self.#ident,)
                }
            });

            // 汇总代码
            let result = quote!(
                impl #struct_ident {
                    /// 字段名
                    pub fn field_names() -> &'static str {
                        let x: &[_] = &[' ', ','];
                        stringify!(#field_name).trim_end_matches(x)
                    }

                    /// 字段名, 含替换规则
                    pub fn field_names_replace(pat: &[(&str, &str)]) -> String {
                        let mut names = Self::field_names().to_string();
                        for (from_str, to_str) in pat {
                            names = names.replace(from_str, to_str);
                        }
                        names
                    }

                    /// 获取多个记录
                    #[auto_func_name]
                    pub fn get_multi<P>(stmt: String, params: P) -> Result<Vec<Self>, anyhow::Error>
                    where
                        P: Into<mysql::params::Params>,
                    {
                        // 全部结果
                        let results = GlobalDbPool::get()
                            .or_else(|err| raise_error!(__func__, "\n", err))?
                            .exec_opt(stmt, params)
                            .or_else(|err| raise_error!(__func__, "\n", err))?;
                        // 如果有 FromRowError, 抛出异常, 这样后续可以 unwrap (map 中不可抛出异常)
                        for result in &results {
                            if let Err(err) = result {
                                return raise_error!(__func__, "\n", err);
                            }
                        }
                        // 已确认 x 不含异常
                        Ok(results.into_iter().map(|x| x.unwrap()).collect())
                    }

                    /// 获取单个记录
                    #[auto_func_name]
                    pub fn get_single<P>(stmt: String, params: P) -> Result<Option<Self>, anyhow::Error>
                    where
                        P: Into<mysql::params::Params>,
                    {
                        match GlobalDbPool::get()
                            .or_else(|err| raise_error!(__func__, "\n", err))?
                            .exec_first_opt(stmt, params)
                            .or_else(|err| raise_error!(__func__, "\n", err))?
                        {
                            Some(Ok(result)) => Ok(Some(result)),
                            Some(Err(err)) => raise_error!(__func__, "\n", err),
                            None => Ok(None),
                        }
                    }

                    /// 保存
                    #[auto_func_name]
                    pub fn save(&self) -> Result<(), anyhow::Error> {
                        let mut sql = GlobalDbPool::get().or_else(|err| raise_error!(__func__, "\n", err))?;
                        if self.id != 0 {
                            sql.exec_drop(
                                format!("UPDATE finance_net SET {}", #field_set1_s),
                                params! {#field_set2},
                            )
                            .or_else(|err| raise_error!(__func__, "\n", err))?;
                        } else {
                            sql.exec_drop(
                                format!(
                                    "INSERT INTO finance_net ({}) VALUES ({})",
                                    #field_set3_s,
                                    #field_set4_s,
                                ),
                                params! {#field_set5},
                            )
                            .or_else(|err| raise_error!(__func__, "\n", err))?;
                        }
                        Ok(())
                    }
                }

                impl mysql::prelude::FromRow for #struct_ident {
                    fn from_row_opt(mut row: mysql::Row) -> Result<Self, mysql::FromRowError> {
                        #field_from_row
                        Ok(Self {
                            #field_name
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
