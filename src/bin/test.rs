use python_comm_macros::{auto_func_name, auto_func_name2};

/// test0, 注意 test1/2 用了 auto_func_name 有注释但 rust doc 无法显示
fn test0() {}

/// test1
///
/// #[allow(unused_variables)] 不生效, 把 _a 改为 a 可查看报错信息
#[auto_func_name]
fn test1() {
    println!("{}", __func__);
    let _a: i32 = 0;
}

/// test2
///
/// 把 _a 改为 a 可查看报错信息
#[auto_func_name]
fn test2() {
    //! 这行注释导致 rust 报错时找不到代码, 注意报错时的 ^ 符号, 换成 //, //* 都没问题

    println!("{}", __func__);
    let _a: i32 = 0;
}

/// test3, 注意 test1 用了 auto_func_name 有注释但 rust doc 无法显示
///
/// 去掉 #[allow(unused_variables)] 可查看报错信息
#[allow(unused_variables)]
#[auto_func_name2]
fn test3() {
    println!("{}", __func__);
    let a: i32 = 0;
}

/// test4, 内部 //! 的注释和这里的合并了
fn test4() {
    //!
    //! auto_func_name2 不允许这行注释, 直接报错 an inner attribute is not permitted in this context
}

/// test4, 内部 //! 的注释和这里的合并了
fn test5() {
    //! auto_func_name2 不允许这行注释, 直接报错 an inner attribute is not permitted in this context
}

fn main() {
    test0();
    test1();
    test2();
    test3();
    test4();
    test5();
}
