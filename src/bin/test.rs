use python_comm_macros::auto_func_name;

/// test0, 注意 test1/2 用了 auto_func_name 有注释但 rust doc 无法显示
fn test0() {}

/// test3, 注意 test1 用了 auto_func_name 有注释但 rust doc 无法显示
///
/// 去掉 #[allow(unused_variables)] 可查看报错信息
#[allow(unused_variables)]
#[auto_func_name]
fn test3() {
    println!("{}", fname);
    let a: i32 = 0;
}

/// test4, 内部 //! 的注释和这里的合并了
fn test4() {
    //!
    //! auto_func_name 不允许这行注释, 直接报错 an inner attribute is not permitted in this context
}

/// test5, 内部 //! 的注释和这里的合并了
fn test5() {
    //! auto_func_name 不允许这行注释, 直接报错 an inner attribute is not permitted in this context
}

/// test6
///
/// &mut || 使得 auto_func_name 不能给出准确的报错, 但可以定位到 test6()
// #[auto_func_name]
// fn test6() {
//     &mut |x: i8| x;

//     // y
// }

fn main() {
    test0();
    test3();
    test4();
    test5();
    // test6();
}
