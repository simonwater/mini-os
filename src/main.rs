// 不链接 Rust 标准库
#![no_std]
// 禁用所有 Rust 层级的入口点
#![no_main]
// 使用自定义测试框架替代默认的测试框架(不依赖 std)，
// 收集所有标注了 #[test_case]属性的函数，然后将函数列表递给用户指定的runner函数
#![feature(custom_test_frameworks)]
// 指定测试的runner函数
#![test_runner(mini_os::test_runner)]
// 将自定义测试框架生成的函数的名称更改为与main不同的名称，该函数需要在_start中调用
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use mini_os::println;

// 因为链接器会寻找一个名为 `_start` 的函数，所以这个函数就是入口点
// 默认命名为 `_start`
#[unsafe(no_mangle)] // 不重整函数名
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");
    //panic!("Some panic message");

    #[cfg(test)]
    test_main();
    loop {}
}

/// 这个函数将在 panic 时被调用
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    mini_os::test_panic_handler(info)
}

#[test_case]
fn test_println() {
    println!("test_println output in main");
}
