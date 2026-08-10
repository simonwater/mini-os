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

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use mini_os::println;

// 宏内部定义了真正的低级_start入口点。并会对函数进行类型检查，
// 避免函数签名错误（增加一个参数或改变参数类型）时只在运行时发生失败。
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use mini_os::allocator;
    use mini_os::memory;
    use x86_64::VirtAddr;

    println!("Welcome To {}", "Mini-OS!");
    mini_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    // 堆上分配数字
    let heap_value = Box::new(42);
    println!("heap_value at: {:p}", heap_value);

    // 使用动态数组
    let mut vec = Vec::with_capacity(500);
    for i in 1..=500 {
        vec.push(i);
    }
    println!("vec at: {:p}", vec.as_slice());

    // 引用计数
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );

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
