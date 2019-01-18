#![no_main]
#![no_std]

use core::mem::swap;
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

const BUFSIZE: usize = 2048;
static mut AUDIO1: [u8; BUFSIZE] = [0; BUFSIZE];
static mut AUDIO2: [u8; BUFSIZE] = [0; BUFSIZE];

static mut CONFIG: [u8; 512] = [0; 512];

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
    write!(SPI1.cr1: mstr = true, br = 0b101,
           ssi = true, ssm = true, spe = true);

    // Set up DMA, DAC and timer for audio output
    modif!(RCC.ahbenr: dmaen = true);
    modif!(RCC.apb1enr: dacen = true, tim6en = true);

    write!(DAC.cr: tsel1 = 0b000,  // trigger by TIM6 TRGO
           dmaen1 = true,
           en1 = true);
    write!(DAC.dhr8r1: dacc1dhr = 0);

    write!(DMA1.cpar2: pa = 0x4000_7410);  // DHR 12bit
    write!(DMA1.cselr: c2s = 0b1001);
    write!(DMA1.ccr2: circ = true, dir = true, minc = true, pl = 0b11, // very high
           msize = 0, psize = 0);
    write!(DMA1.cndtr2: ndt = BUFSIZE as u16);

    write!(TIM6.arr: arr = 180*4); // for 22050 kHz
    write!(TIM6.egr: ug = true);
    write!(TIM6.cr2: mms = 0b010); // trigger on update
    write!(TIM6.dier: ude = true);

    // Initialize SD card
    let mut cont = sd::Controller::new(sd::SdMmcSpi::new(SPI, CS), Clock);
    cont.device().init().unwrap();
    write!(GPIOA.bsrr: bs5 = true); // turn on LED

    // Speed up SPI to maximum (8 MHz)
    modif!(SPI1.cr1: br = 0b000);
    let vol = cont.get_volume(sd::VolumeIdx(0)).unwrap();
    let root = cont.open_root_dir(&vol).unwrap();

    let cfg = unsafe { &mut CONFIG };
    let mut fd = cont.open_file_in_dir(&vol, &root, "config.txt", sd::Mode::ReadOnly).unwrap();
    let n = cont.read(&vol, &mut fd, cfg).unwrap();
    let afile = core::str::from_utf8(&cfg[..n]).unwrap().trim();
    drop(fd);

    let mut fd = cont.open_file_in_dir(&vol, &root, afile, sd::Mode::ReadOnly).unwrap();

    'outer: loop {
        // Clear output
        write!(DAC.dhr8r1: dacc1dhr = 0);

        // Set up buffer pointers
        let (mut ptr1, mut ptr2) = unsafe { (&mut AUDIO1, &mut AUDIO2) };

        // Read first block into first buffer
        fd.seek_from_start(0).unwrap();
        cont.read(&vol, &mut fd, ptr1).unwrap();
        write!(DMA1.cmar2: ma = ptr1.as_ptr() as u32);

        // Wait for button press
        while readb!(GPIOC.idr: id13) {}

        write!(TIM6.cr1: cen = true);
        modif!(DMA1.ccr2: en = true);

        loop {
            swap(&mut ptr1, &mut ptr2);
            // Read next block into buffer
            if cont.read(&vol, &mut fd, ptr1).unwrap() == 0 {
                // On end of file, deactivate DMA
                write!(TIM6.cr1: cen = false);
                modif!(DMA1.ccr2: en = false);
                break;
            }

            // Wait for transfer complete flag
            while !readb!(DMA1.isr: tcif2) {}
            write!(DMA1.cmar2: ma = ptr1.as_ptr() as u32);
            // Clear DMA transfer flags
            write!(DMA1.ifcr: ctcif2 = true, chtif2 = true, cgif2 = true, cteif2 = true);
        }
    }
}

#[panic_handler]
fn panicking(_: &core::panic::PanicInfo) -> ! {
    write!(GPIOA.bsrr: br5 = true); // turn off LED
    let _ = sh::hprintln!("panic!");
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
