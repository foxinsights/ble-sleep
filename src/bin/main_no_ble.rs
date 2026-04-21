#![no_std]
#![no_main]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use core::time::Duration;
use defmt::info;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rtc_cntl::Rtc;
use esp_hal::rtc_cntl::sleep::TimerWakeupSource;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;

#[esp_hal::main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 140 * 1024);
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);

    let mut rtc = Rtc::new(peripherals.LPWR);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    info!("ble-sleep-minimal booting");
    let reset_reason =
        esp_hal::system::reset_reason().unwrap_or(esp_hal::rtc_cntl::SocResetReason::ChipPowerOn);
    let wakeup_reason = esp_hal::system::wakeup_cause();
    info!(
        "boot reset_reason={} wakeup_reason={}",
        defmt::Debug2Format(&reset_reason),
        wakeup_reason,
    );

    Delay::new().delay_millis(10_000);

    info!("entering sleep_deep with 60 s RTC timer wake");
    let timer_wake = TimerWakeupSource::new(Duration::from_secs(60));
    rtc.sleep_deep(&[&timer_wake]);
}
