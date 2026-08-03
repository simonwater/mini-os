#![no_std] // 不链接 Rust 标准库
#![no_main] // 禁用所有 Rust 层级的入口点

mod vga_buffer;

use core::panic::PanicInfo;

// 因为链接器会寻找一个名为 `_start` 的函数，所以这个函数就是入口点
// 默认命名为 `_start`
#[unsafe(no_mangle)] // 不重整函数名
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");
    //panic!("Some panic message");
    loop {}
}

/// 这个函数将在 panic 时被调用
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
