// 不链接 Rust 标准库
#![no_std]
// 禁用所有 Rust 层级的入口点
#![no_main]
// 使用自定义测试框架替代默认的测试框架(不依赖 std)，
// 收集所有标注了 #[test_case]属性的函数，然后将函数列表递给用户指定的runner函数
#![feature(custom_test_frameworks)]
// 指定测试的runner函数
#![test_runner(crate::test_runner)]
// 将自定义测试框架生成的函数的名称更改为与main不同的名称，该函数需要在_start中调用
#![reexport_test_harness_main = "test_main"]

mod serial;
mod vga_buffer;

use core::panic::PanicInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for &test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

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
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}
