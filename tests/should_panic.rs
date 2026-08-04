#![no_std]
#![no_main]

use core::panic::PanicInfo;
use mini_os::{QemuExitCode, exit_qemu, serial_print, serial_println};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    mini_os::hlt_loop()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    mini_os::hlt_loop()
}

fn should_fail() {
    serial_print!("should_fail... ");
    assert_eq!(0, 1);
}
