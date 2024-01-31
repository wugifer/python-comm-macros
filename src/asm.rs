use {
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::{quote, ToTokens},
    std::{collections::HashMap, iter::FromIterator},
    syn::{
        parse_macro_input, Data, DataStruct, DeriveInput, Field, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
        Type,
    },
};

/// 转换 Lit
fn lit_to_string(lit: &Lit) -> Option<String> {
    match *lit {
        Lit::Str(ref s) => Some(s.value()),
        _ => None,
    }
}

/// 在全部字段上执行
fn map_columns<M>(columns: &Vec<Column>, mut mapper: M) -> TokenStream2
where
    M: FnMut(usize, &Ident, &Type) -> TokenStream2,
{
    TokenStream2::from_iter(
        columns
            .iter()
            .enumerate()
            .map(|(i, x)| mapper(i, &x.ident, &x.field.ty)),
    )
}

/// 在全部字段上执行并 join
fn map_columns_and_join<F, M>(columns: &Vec<Column>, filter: F, mapper: M, sep: TokenStream2) -> TokenStream2
where
    F: FnMut(&&Column) -> bool,
    M: FnMut(&Column) -> TokenStream2,
{
    TokenStream2::from_iter(
        columns
            .iter()
            .filter(filter)
            .map(mapper)
            .enumerate()
            .map(|(i, x)| if i == 0 { vec![x] } else { vec![sep.clone(), x] })
            .flatten(),
    )
}

pub fn as_sql_model(input: TokenStream) -> TokenStream {
    // 解析输入
    let dvi = parse_macro_input!(input as DeriveInput);

    // 字段
    let fields = match dvi.data {
        Data::Struct(DataStruct { ref fields, .. }) => fields.iter().cloned().collect(),
        _ => panic!("仅用于 struct"),
    };

    // 类名
    let struct_ident = &dvi.ident;

    let mut table = Table::new();
    table.parse_struct_derive(&dvi);
    table.parse_struct_fields(&fields);

    let make_assign = table.make_assign();
    let make_create_table = table.make_create_table();
    let make_fields_b = table.make_fields_string("`", "`", ", ", true); // `a`, `b`, `c`
    let make_fields_bi = table.make_fields_string("`", "`", ", ", false); // `a`, `b`, `c`
    let make_fields_c = table.make_fields_c();
    let make_fields_e = table.make_fields_string2("`", "`=:", "", ", ", true); // a=:a, b=:b, c=:c
    let make_fields_ee = table.make_fields_ee(true);
    let make_fields_eei = table.make_fields_ee(false);
    let make_fields_ei = table.make_fields_string2("`", "`=:", "", ", ", false); // a=:a, b=:b, c=:c
    let make_fields_fi = table.make_fields_fi();
    let make_fields_from_row = table.make_fields_from_row();
    let make_fields_p = table.make_fields_string(":", "", ", ", true); // :a, :b, :c
    let make_fields_pi = table.make_fields_string(":", "", ", ", false); // :a, :b, :c
    let make_fields_q = table.make_fields_string("\"", "\"", ", ", true); // "a", "b", "c"
    let make_fields_qc = table.make_fields_string("\"", "\", ", "", true); // "a", "b", "c",
    let make_fields_v = table.make_fields_v(true);
    let make_fields_vi = table.make_fields_v(false);
    let table_name = table.name.to_string();
    let who = Ident::new(&table.who, Span::call_site());

    let impl_ast = quote!(
        impl #struct_ident {
            #[auto_func_name]
            /// 保存
            pub fn create_with(#make_fields_fi) -> Result<Option<u64>, MoreError> {
                let id = 0;
                Self {#make_fields_c}.create().m(m!(__func__))
            }

            #make_assign
        }

        impl SqlModel for #struct_ident {
            /// 比较两个 obj
            fn equal(&self, other: &Self) -> bool {
                #make_fields_ee
            }

            /// 比较两个 obj, 排除 id
            fn equal_without_id(&self, other: &Self) -> bool {
                #make_fields_eei
            }

            /// 返回加锁的 DbPool, 使用者需命名并引入 WhoCreateDbPool 或 who 属性指定的类名
            #[auto_func_name]
            fn lock() -> Result<std::sync::MutexGuard<'static, python_comm::use_sql::DbPool>, python_comm::use_m::MoreError> {
                #who::lock().m(m!(__func__))
            }

            fn make_create_table() -> &'static str {
                #make_create_table
            }

            fn make_fields_b() -> &'static str {
                #make_fields_b
            }

            fn make_fields_bi() -> &'static str {
                #make_fields_bi
            }

            fn make_fields_e() -> &'static str {
                #make_fields_e
            }

            fn make_fields_ei() -> &'static str {
                #make_fields_ei
            }

            fn make_fields_p() -> &'static str {
                #make_fields_p
            }

            fn make_fields_pi() -> &'static str {
                #make_fields_pi
            }

            fn make_fields_q() -> &'static str {
                #make_fields_q
            }

            fn make_fields_qc() -> &'static str {
                #make_fields_qc
            }

            fn make_fields_v(&self) -> mysql::params::Params {
                #make_fields_v
            }

            fn make_fields_vi(&self) -> mysql::params::Params {
                #make_fields_vi
            }

            fn table_name() -> &'static str {
                #table_name
            }
        }

        impl mysql::prelude::FromRow for #struct_ident {
            fn from_row_opt(mut row: mysql::Row) -> Result<Self, mysql::FromRowError> {
                #make_fields_from_row
                Ok(Self {
                    #make_fields_c
                })
            }
        }
    );

    // 仅用于调试
    // eprintln!("{}", impl_ast);

    impl_ast.into()
}

struct Column {
    field: Field,                 // 字段
    ident: Ident,                 // 字段
    name: String,                 // 字段名
    sql_type: String,             // sql 类型
    opt: HashMap<String, String>, // 选项, auto, key, name ...
}

impl Column {
    /// 转换 rust 类型为 sql 类型
    fn convert_rust_type_to_sql(type_: &str) -> String {
        let type_ = TYPE_MAP1
            .iter()
            .find_map(|(x, y)| if x == &type_ { Some(*y) } else { None })
            .unwrap_or(type_);

        TYPE_MAP2
            .iter()
            .find_map(|(x, y)| if x == &type_ { Some(*y) } else { None })
            .unwrap_or(type_)
            .to_string()
    }

    /// 构造
    fn new(field: Field, opt: HashMap<String, String>) -> Self {
        let ident = field.ident.clone().unwrap();
        let sql_type = opt
            .get("type")
            .map(|x| x.clone())
            .unwrap_or_else(|| Self::convert_rust_type_to_sql(&field.ty.to_token_stream().to_string()));
        let name = ident.to_string();

        Self {
            field,
            ident,
            name,
            sql_type,
            opt,
        }
    }

    /// 在 sql 中的 auto 属性
    fn sql_auto(&self) -> &str {
        match self.opt.get("auto") {
            Some(_) => " AUTO_INCREMENT",
            None => "",
        }
    }

    /// 在 sql 中的 key 属性
    fn sql_key(&self) -> Option<String> {
        match self.opt.get("key") {
            Some(key) => {
                if key == "" {
                    Some(format!("KEY (`{}`)", self.sql_name()))
                } else {
                    Some(format!("{} KEY (`{}`)", key, self.sql_name()))
                }
            }
            _ => None,
        }
    }

    /// 在 sql 中的名字
    fn sql_name(&self) -> &str {
        self.opt.get("name").unwrap_or(&self.name)
    }
}

struct Table {
    name: String,         // table 名
    who: String,          // WhoCreateDbPool 类名
    columns: Vec<Column>, // 字段
}

/// @TODO Refactor duplicated code
impl Table {
    /// 从 column meta 中解析属性
    fn extract_column_meta(meta_items: &Vec<&NestedMeta>) -> HashMap<String, String> {
        let mut opt = HashMap::new();

        for meta_item in meta_items {
            if let NestedMeta::Meta(ref item) = **meta_item {
                if let Meta::NameValue(MetaNameValue { ref path, ref lit, .. }) = *item {
                    opt.insert(
                        path.get_ident().unwrap().to_string(),
                        lit_to_string(lit).unwrap_or_default(),
                    );
                }
            }
        }

        opt
    }

    /// 从 table meta 中解析属性
    fn extract_table_meta(meta_items: &Vec<&NestedMeta>) -> (String, String) {
        let mut name = "unknown_table_name".to_string();
        let mut who = "WhoCreateDbPool".to_string();

        for meta_item in meta_items {
            if let NestedMeta::Meta(ref item) = **meta_item {
                if let Meta::NameValue(MetaNameValue { ref path, ref lit, .. }) = *item {
                    match path.get_ident().unwrap().to_string().as_ref() {
                        "name" => name = lit_to_string(lit).unwrap_or_default(),
                        "who" => who = lit_to_string(lit).unwrap_or_default(),
                        _ => {}
                    }
                }
            }
        }

        (name, who)
    }

    /// 快速设置每个字段
    fn make_assign(&self) -> TokenStream2 {
        map_columns(&self.columns, |_i, ident, ty| {
            quote!(
                pub fn #ident<T>(mut self, v: T) -> Self
                where
                    T: Into<#ty>,
                {
                    self.#ident = v.into();
                    self
                }
            )
        })
    }

    /// 创建表的 sql
    fn make_create_table(&self) -> String {
        // 字段定义
        let mut lines = self
            .columns
            .iter()
            .map(|column| {
                format!(
                    "`{}` {} NOT NULL{}",
                    column.sql_name(),
                    column.sql_type,
                    column.sql_auto()
                )
            })
            .collect::<Vec<String>>();

        // 键
        lines.append(
            &mut self
                .columns
                .iter()
                .filter_map(|column| column.sql_key())
                .collect::<Vec<String>>(),
        );

        format!("CREATE TABLE `{}` (\n    {}\n);", self.name, lines.join(",\n    "))
    }

    // C-有逗号结尾, Q-有双引号, B-有反引号, I-去掉 id, P-作为参数, E-赋值, V-Value, EE-相等, F-函数参数

    /// a, b, c,
    fn make_fields_c(&self) -> TokenStream2 {
        map_columns_and_join(
            &self.columns,
            |_| true,
            |column| {
                let ident = &column.ident;
                quote!(#ident, )
            },
            quote!(),
        )
    }

    /// self.a==other.a && self.b==other.b
    fn make_fields_ee(&self, use_id: bool) -> TokenStream2 {
        map_columns_and_join(
            &self.columns,
            if use_id {
                |_: &&Column| true
            } else {
                |column: &&Column| column.ident != "id"
            },
            |column| {
                let ident = &column.ident;
                quote!(self.#ident == other.#ident)
            },
            quote!(&&),
        )
    }

    /// a:A, b:B, c:C
    fn make_fields_fi(&self) -> TokenStream2 {
        map_columns_and_join(
            &self.columns,
            |column| column.ident != "id",
            |column| {
                let ident = &column.ident;
                let ty = &column.field.ty;
                quote!(#ident: #ty)
            },
            quote!(,),
        )
    }

    /// let a = match{}; let b = match{};
    fn make_fields_from_row(&self) -> TokenStream2 {
        // 解析每个字段, 设置同名变量
        map_columns(&self.columns, |i, ident, ty| {
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
        })
    }

    /// left name right sep ... left name right
    fn make_fields_string(&self, left: &str, right: &str, sep: &str, use_id: bool) -> TokenStream2 {
        let code = self
            .columns
            .iter()
            .filter(if use_id {
                |_: &&Column| true
            } else {
                |column: &&Column| column.ident != "id"
            })
            .map(|column| format!("{}{}{}", left, column.sql_name(), right))
            .collect::<Vec<String>>()
            .join(sep);
        quote!(#code)
    }

    /// left name mid name right sep ... left name mid name right
    fn make_fields_string2(&self, left: &str, mid: &str, right: &str, sep: &str, use_id: bool) -> TokenStream2 {
        let code = self
            .columns
            .iter()
            .filter(if use_id {
                |_: &&Column| true
            } else {
                |column: &&Column| column.ident != "id"
            })
            .map(|column| {
                let real = column.sql_name();
                format!("{}{}{}{}{}", left, real, mid, real, right)
            })
            .collect::<Vec<String>>()
            .join(sep);
        quote!(#code)
    }

    /// vec![("a", self.a), ("b", self.b)]
    fn make_fields_v(&self, use_id: bool) -> TokenStream2 {
        let code = map_columns_and_join(
            &self.columns,
            if use_id {
                |_: &&Column| true
            } else {
                |column: &&Column| column.ident != "id"
            },
            |column| {
                let real = column.sql_name();
                let ident = &column.ident;
                quote!((#real, self.#ident.clone().into()))
            },
            quote!(,),
        );
        quote!(
            let v: Vec<(&str, mysql::Value)> = vec![ #code ];
            mysql::params::Params::from(v)
        )
    }

    /// 构造
    fn new() -> Self {
        Self {
            name: String::new(),
            who: "WhoCreateDbPool".to_string(),
            columns: Vec::new(),
        }
    }

    /// 解析 struct derive 属性
    fn parse_struct_derive(&mut self, dvi: &DeriveInput) {
        // 遍历每个 #[table()], 更新 name
        for attr in dvi.attrs.iter() {
            if let Ok(Meta::List(MetaList {
                ref path, ref nested, ..
            })) = attr.parse_meta()
            {
                match path.get_ident().unwrap().to_string().as_ref() {
                    "table" => {
                        (self.name, self.who) = Table::extract_table_meta(&nested.iter().collect());
                    }
                    _ => {}
                }
            }
        }
    }

    /// 解析 struct fields 属性
    fn parse_struct_fields(&mut self, fields: &Vec<Field>) {
        // 遍历每个 field
        for field in fields {
            let mut sql_opt = HashMap::new();

            // 遍历每个 #[column()], 更新 sql_type, opt
            for attr in &field.attrs {
                if !attr.path.to_token_stream().to_string().contains("column") {
                    continue;
                }

                if let Ok(Meta::List(MetaList { ref nested, .. })) = attr.parse_meta() {
                    // 解析并处理 opt
                    sql_opt.extend(Self::extract_column_meta(&nested.iter().collect()).into_iter());
                }
            }

            // 记录
            self.columns.push(Column::new(field.clone(), sql_opt));
        }
    }
}

/// rust 类型 -> AsSqlModel 类型, 未命中的不变
const TYPE_MAP1: [(&'static str, &'static str); 9] = [
    ("i32", "int"),
    ("u32", "int"),
    ("i64", "bigint"),
    ("u64", "bigint"),
    ("f32", "double"),
    ("f64", "double"),
    ("String", "str"),
    ("SqlDate", "date"),
    ("SqlTime", "datetime"),
];

/// AsSqlModel 类型 -> sql 类型, 未命中的不变
const TYPE_MAP2: [(&'static str, &'static str); 9] = [
    ("str", "varchar(32)"),
    ("text", "text(65535)"),
    ("longtext", "longtext"),
    ("bool", "bool"),
    ("int", "int(11)"),
    ("bigint", "int(20)"),
    ("double", "double"),
    ("date", "date"),
    ("datetime", "datetime(6)"),
];
