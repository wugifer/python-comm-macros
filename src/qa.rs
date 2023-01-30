use {
    proc_macro::TokenStream,
    proc_macro2::TokenStream as TokenStream2,
    quote::quote,
    std::iter::FromIterator,
    syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Ident, Type},
};

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

pub fn quick_assign(input: TokenStream) -> TokenStream {
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
    table.parse_struct_fields(&fields);

    let make_assign = table.make_assign();

    let impl_ast = quote!(
        impl #struct_ident {
            #make_assign
        }
    );

    // 仅用于调试
    // eprintln!("{}", impl_ast);

    impl_ast.into()
}

struct Column {
    field: Field, // 字段
    ident: Ident, // 字段
}

impl Column {
    /// 构造
    fn new(field: Field) -> Self {
        let ident = field.ident.clone().unwrap();

        Self { field, ident }
    }
}

struct Table {
    columns: Vec<Column>, // 字段
}

/// @TODO Refactor duplicated code
impl Table {
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

    /// 构造
    fn new() -> Self {
        Self { columns: Vec::new() }
    }

    /// 解析 struct fields 属性
    fn parse_struct_fields(&mut self, fields: &Vec<Field>) {
        // 遍历每个 field
        for field in fields {
            // 记录
            self.columns.push(Column::new(field.clone()));
        }
    }
}
