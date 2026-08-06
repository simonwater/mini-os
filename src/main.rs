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

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use mini_os::println;
use x86_64::structures::paging::PageTable;

// 宏内部定义了真正的低级_start入口点。并会对函数进行类型检查，
// 避免函数签名错误（增加一个参数或改变参数类型）时只在运行时发生失败。
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use mini_os::memory::translate_addr;
    use x86_64::VirtAddr;

    println!("Welcome To {}", "Mini-OS!");
    mini_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];
    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = unsafe { translate_addr(virt, phys_mem_offset) };
        println!("{:?} -> {:?}", virt, phys);
    }

    #[cfg(test)]
    test_main();
    println!("It did not crash!");
    mini_os::hlt_loop()
}

/// 这个函数将在 panic 时被调用
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    mini_os::hlt_loop()
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
