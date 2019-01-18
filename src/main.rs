#![no_main]
#![no_std]
#![feature(try_blocks)]

use panic_semihosting;
use stm32l0::stm32l0x3 as stm;
// use stm32l0::stm32l0x3::interrupt;
use cortex_m_rt::ExceptionFrame;
use embedded_hal as hal;
use embedded_sdmmc as sd;
// use embedded_sdmmc::BlockDevice;

use cortex_m_semihosting as sh;

#[macro_use]
mod util;

// Minimal HAL implementation for our specific setup

struct CS;
struct SPI;

impl hal::digital::OutputPin for CS {
    fn set_low(&mut self) {
        write!(GPIOA.bsrr: br15 = true);
    }
    fn set_high(&mut self) {
        write!(GPIOA.bsrr: bs15 = true);
    }
}

impl hal::spi::FullDuplex<u8> for SPI {
    type Error = &'static str;

    fn read(&mut self) -> nb::Result<u8, &'static str> {
        let sr = readr!(SPI1.sr);

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other("ovr")
        } else if sr.modf().bit_is_set() {
            nb::Error::Other("mode")
        } else if sr.crcerr().bit_is_set() {
            nb::Error::Other("crc")
        } else if sr.rxne().bit_is_set() {
            let byte = read!(SPI1.dr: dr) as u8;
            return Ok(byte);
        } else {
            nb::Error::WouldBlock
        })
    }

    fn send(&mut self, byte: u8) -> nb::Result<(), &'static str> {
        let sr = readr!(SPI1.sr);

        Err(if sr.ovr().bit_is_set() {
            nb::Error::Other("ovr")
        } else if sr.modf().bit_is_set() {
            nb::Error::Other("mode")
        } else if sr.crcerr().bit_is_set() {
            nb::Error::Other("crc")
        } else if sr.txe().bit_is_set() {
            write!(SPI1.dr: dr = byte as u16);
            return Ok(());
        } else {
            nb::Error::WouldBlock
        })
    }
}

// No timestamps necessary

struct Clock;

impl sd::TimeSource for Clock {
    fn get_timestamp(&self) -> sd::Timestamp {
        sd::Timestamp::from_calendar(1970, 1, 1, 0, 0, 0).unwrap()
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut pcore = arm::Peripherals::take().unwrap();

    // Switch to 16 MHz clock
    modif!(RCC.cr: hsi16on = true);
    wait_for!(RCC.cr: hsi16rdyf);
    modif!(RCC.cfgr: sw = 0b01);

    // Activate Standby mode on WFI
    pcore.SCB.set_sleepdeep();
    modif!(RCC.apb1enr: pwren = true);
    modif!(PWR.csr: bre = true);  // note: BRE is actually EWUP2
    modif!(PWR.cr: cwuf = true, ulp = true, pdds = true);

    // Enable LED output
    modif!(RCC.iopenr: iopaen = true, iopben = true, iopcen = true);
    modif!(GPIOA.moder: mode5 = 0b01);
    modif!(GPIOC.moder: mode13 = 0b00);
    modif!(GPIOC.pupdr: pupd13 = 0b01);

    // SD card: prepare SPI
    modif!(GPIOA.pupdr: pupd15 = 0b01);
    write!(GPIOA.bsrr: bs15 = true);
    modif!(GPIOA.moder: mode15 = 0b01);
    modif!(GPIOB.moder: mode3 = 0b10, mode4 = 0b10, mode5 = 0b10);
    modif!(GPIOB.ospeedr: ospeed3 = 0b10, ospeed5 = 0b10);
    modif!(GPIOB.afrl: afsel3 = 0, afsel4 = 0, afsel5 = 0);
    modif!(RCC.apb2enr: spi1en = true);
    write!(SPI1.cr1: mstr = true, br = 0b101, ssm = true, ssi = true, spe = true);

    let mut cont = sd::Controller::new(sd::SdMmcSpi::new(SPI, CS), Clock);
    sh::hprintln!("Init SD card...").unwrap();
    cont.device().init().unwrap();
    modif!(SPI1.cr1: br = 0b000);
    let res: Result<_, sd::Error<sd::SdMmcError>> = try {
        let vol = cont.get_volume(sd::VolumeIdx(0))?;
        let root = cont.open_root_dir(&vol)?;
        let mut fd = cont.open_file_in_dir(&vol, &root, "audio.raw", sd::Mode::ReadOnly)?;
        let mut buf = [0u8; 1024];
        while cont.read(&vol, &mut fd, &mut buf)? == 1024 {}
    };
    if let Err(e) = res {
        sh::hprintln!("Err: {:?}", e).unwrap();
    } else {
        sh::hprintln!("Reading done.").unwrap();
    }

    loop {}
}

#[cortex_m_rt::exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[cortex_m_rt::exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
