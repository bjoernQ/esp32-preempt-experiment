#![no_std]
#![no_main]

use core::{cell::RefCell, fmt::Write};

use esp32_hal::{interrupt, Cpu};
use esp32_hal::{
    pac::{self, Peripherals, TIMG1, UART0},
    prelude::*,
    RtcCntl, Serial, Timer,
};
use xtensa_lx::mutex::{Mutex, SpinLockMutex};
use xtensa_lx_rt::entry;
use xtensa_lx_rt::exception::Context;

mod preempt;
use preempt::*;

static mut SERIAL: SpinLockMutex<RefCell<Option<Serial<UART0>>>> =
    SpinLockMutex::new(RefCell::new(None));
static mut TIMER1: SpinLockMutex<RefCell<Option<Timer<TIMG1>>>> =
    SpinLockMutex::new(RefCell::new(None));

const TIMER_DELAY: u64 = 10_000u64;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();

    // Disable the TIMG watchdog timer.
    let mut timer0 = Timer::new(peripherals.TIMG0);
    let mut timer1 = Timer::new(peripherals.TIMG1);
    let serial0 = Serial::new(peripherals.UART0).unwrap();
    let mut rtc_cntl = RtcCntl::new(peripherals.RTC_CNTL);

    // Disable MWDT and RWDT (Watchdog) flash boot protection
    timer0.disable();
    timer1.disable();
    rtc_cntl.set_wdt_global_enable(false);

    interrupt::enable(
        Cpu::ProCpu,
        pac::Interrupt::TG1_T0_LEVEL,
        interrupt::CpuInterrupt::Interrupt20LevelPriority2,
    );
    timer1.start(TIMER_DELAY);
    timer1.listen();

    unsafe {
        (&SERIAL).lock(|data| (*data).replace(Some(serial0)));
        (&TIMER1).lock(|data| (*data).replace(Some(timer1)));
    }

    task_create(worker_task1);
    task_create(worker_task2);

    unsafe {
        xtensa_lx::interrupt::disable();
        xtensa_lx::interrupt::enable_mask(
            xtensa_lx_rt::interrupt::CpuInterruptLevel::Level2.mask(),
        );
    }

    let mut z = 0;
    loop {
        unsafe {
            (&SERIAL).lock(|data| {
                let mut serial = data.borrow_mut();
                let serial = serial.as_mut().unwrap();
                writeln!(serial, "Task Main {}", z).ok();
            });
        }

        for _ in 0..200_000 {}
        z += 1;
    }
}

pub extern "C" fn worker_task1() {
    let mut cnt = 0;

    loop {
        unsafe {
            (&SERIAL).lock(|data| {
                let mut serial = data.borrow_mut();
                let serial = serial.as_mut().unwrap();
                writeln!(serial, "Task A: {}", cnt).ok();
            });
        }

        cnt += 1;

        for _ in 0..100_000 {}
    }
}

pub extern "C" fn worker_task2() {
    let mut cnt = 0;

    loop {
        unsafe {
            (&SERIAL).lock(|data| {
                let mut serial = data.borrow_mut();
                let serial = serial.as_mut().unwrap();
                writeln!(serial, "Task B: {}", cnt).ok();
            });
        }

        cnt += 1;

        for _ in 0..50_000 {}
    }
}

#[no_mangle]
pub fn level2_interrupt(context: &mut Context) {
    interrupt::clear(
        Cpu::ProCpu,
        interrupt::CpuInterrupt::Interrupt20LevelPriority2,
    );

    unsafe {
        (&TIMER1).lock(|data| {
            let mut timer1 = data.borrow_mut();
            let timer1 = timer1.as_mut().unwrap();
            timer1.clear_interrupt();
            timer1.start(TIMER_DELAY);
        });
    }

    task_switch(context);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        (&SERIAL).lock(|data| {
            let mut serial = data.borrow_mut();
            let serial = serial.as_mut().unwrap();
            writeln!(serial, "\n\n*** {:?}", info).ok();
        });
    }

    loop {}
}
