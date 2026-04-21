#![no_std]
#![no_main]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use bt_hci::controller::ExternalController;
use core::time::Duration;
use defmt::{error, info, warn};
use embassy_futures::block_on;
use embassy_futures::select::{Either, select};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rtc_cntl::Rtc;
use esp_hal::rtc_cntl::sleep::TimerWakeupSource;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use esp_radio::ble::controller::BleConnector;
use trouble_host::prelude::*;

const MAX_CONNECTIONS: usize = 1;
const MAX_CHANNELS: usize = 2;
const L2CAP_MTU: usize = 251;
const ADV_NAME: &str = "BleSleepMinimal";

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

    {
        let connector = BleConnector::new(
            peripherals.BT,
            esp_radio::ble::Config::default().with_acl_buf_count(64),
        )
        .unwrap();
        let controller = ExternalController::<_, 20>::new(connector);
        let address = Address::random([0xAE, 0xEB, 0xE6, 0xFF, 0xFF, 0xFA]);

        let mut resources =
            HostResources::<DefaultPacketPool, MAX_CONNECTIONS, MAX_CHANNELS, L2CAP_MTU>::new();
        let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
        let Host {
            mut peripheral,
            mut runner,
            ..
        } = stack.build();

        info!("ble session begin: advertising as {}", ADV_NAME);

        let mut adv_data = [0u8; 31];
        let _len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::CompleteLocalName(ADV_NAME.as_bytes()),
            ],
            &mut adv_data[..],
        )
        .unwrap();

        let session = async {
            match peripheral
                .advertise(
                    &Default::default(),
                    Advertisement::ConnectableScannableUndirected {
                        adv_data: &adv_data,
                        scan_data: &[],
                    },
                )
                .await
            {
                Ok(advertiser) => match advertiser.accept().await {
                    Ok(conn) => {
                        info!("ble connected");
                        loop {
                            match conn.next().await {
                                ConnectionEvent::Disconnected { .. } => {
                                    info!("ble disconnected");
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => warn!("advertiser.accept failed: {:?}", defmt::Debug2Format(&e)),
                },
                Err(e) => warn!("advertise failed: {:?}", defmt::Debug2Format(&e)),
            }
        };

        let runner_fut = async {
            loop {
                if let Err(e) = runner.run().await {
                    error!("trouble runner error: {:?}", defmt::Debug2Format(&e));
                    break;
                }
            }
        };

        block_on(async {
            match select(session, runner_fut).await {
                Either::First(_) => info!("session ended normally"),
                Either::Second(_) => info!("runner exited"),
            }
        });

        info!("ble session end: dropping controller");
    }
    info!("ble controller dropped");

    Delay::new().delay_millis(200);

    let d = Duration::from_secs(10);
    info!("entering sleep_deep with {} s RTC timer wake", d.as_secs());
    let timer_wake = TimerWakeupSource::new(d);
    rtc.sleep_deep(&[&timer_wake]);
}
