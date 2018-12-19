#![no_main]
#![no_std]

use panic_semihosting;
use stm32l0::stm32l0x3 as stm;
// use stm32l0::stm32l0x3::interrupt;
use cortex_m_rt::ExceptionFrame;

#[macro_use]
mod util;

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut pcore = arm::Peripherals::take().unwrap();

    // Activate Standby mode on WFI
    pcore.SCB.set_sleepdeep();
    modif!(RCC.apb1enr: pwren = true);
    modif!(PWR.csr: bre = true);  // note: BRE is actually EWUP2
    modif!(PWR.cr: cwuf = true, ulp = true, pdds = true);

    // Enable LED output
    modif!(RCC.iopenr: iopaen = true, iopcen = true);
    modif!(GPIOA.moder: mode5 = 0b01);
    modif!(GPIOC.moder: mode13 = 0b00);
    modif!(GPIOC.pupdr: pupd13 = 0b01);

    let cnt = if !readb!(GPIOC.idr: id13) { 4 } else { 1 };
    for _ in 0..cnt {
        for _ in 0..100000 {
            write!(GPIOA.bsrr: bs5 = true);
        }
        for _ in 0..100000 {
            write!(GPIOA.bsrr: br5 = true);
        }
    }

    loop {
        arm::asm::wfe();
    }
}

#[cortex_m_rt::exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[cortex_m_rt::exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
