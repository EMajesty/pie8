#![no_main]
#![no_std]

use core::{fmt::Write, panic::PanicInfo};

use pie8_core::{RUST_READY_MESSAGE, core_id, exception_level};
use pie8_rpi::{cpu, mini_uart::MiniUart};

core::arch::global_asm!(include_str!("boot.S"));

#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    let current_el = cpu::current_el();
    let mpidr = cpu::mpidr();
    let mut uart = MiniUart::new();

    writeln!(uart, "PIE8 bare-metal firmware").ok();
    writeln!(uart, "version: {}", env!("CARGO_PKG_VERSION")).ok();
    writeln!(uart, "build: {}", env!("PIE8_BUILD_ID")).ok();
    writeln!(uart, "exception level: EL{}", exception_level(current_el)).ok();
    writeln!(uart, "CPU core: {}", core_id(mpidr)).ok();
    writeln!(uart, "{RUST_READY_MESSAGE}").ok();

    cpu::wait_forever()
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut uart = MiniUart::new();
    writeln!(uart, "PANIC: {info}").ok();
    cpu::wait_forever()
}
