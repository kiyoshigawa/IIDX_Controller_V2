//! IIDX_Controller_v2
//! Author: kiyoshigawa
//! Date Started: 2026-04-25
//!
//! This is a project for a beatmania IIDX controller controller board.
//! This project contains code under multiple licenses:
//!
//! - Original template code from rp-rs/rp235x-project-template:
//!   Dual-licensed under MIT OR Apache-2.0
//!
//! - New code and modifications by [Your Name]:
//!   Licensed under GPLv3
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{AtomicI32, Ordering};

use critical_section::Mutex;
use defmt::info;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_9X18_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::digital::*;
use fugit::{ExtU32, RateExtU32};
use panic_probe as _;
use rp235x_hal::pac::dma::TIMER0;
use rp235x_hal::pac::powman::TIMER;
use rp235x_hal::timer::{CopyableTimer0, Timer};
use rp235x_hal::{
    self as hal, Clock, I2C,
    clocks::init_clocks_and_plls,
    entry,
    gpio::{DynPinId, FunctionSioInput, Pin, PullUp},
    multicore::{Multicore, Stack},
    pac,
    pio::{Buffers, PIOExt},
};
use ssd1306::{Ssd1306, prelude::*};
use usb_device::{class_prelude::*, prelude::*};
use usbd_human_interface_device::{page::Keyboard, prelude::*};

/// The number of GPIO pins being used as buttons, both for the keyboard peripheral and for the control panel.
const NUM_BUTTONS: usize = 24;

/// Default debounce time in ticks (1,000,000 ticks per second)
const DEFAULT_DEBOUNCE_TICKS: u64 = 10_000;

/// Default usb device tick send time in ticks (1,000,000 ticks per second)
/// This should be 1ms per USB spec or device will lose connection
const USB_TICK_INTERVAL_TICKS: u64 = 1_000;

/// Default keyboard send rate in ticks (1,000,000 ticks per second)
const USB_SEND_INTERVAL_TICKS: u64 = 1_000;

//stack size for core 1: increase as needed
static CORE_STACK_1: Stack<32768> = Stack::new();

/// Tell the Boot ROM about our application:
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

// struct for storing simple button info to make iterative upating easier:
pub struct ButtonState {
    pub name: &'static str,
    pub pin: Pin<DynPinId, FunctionSioInput, PullUp>,
    pub last_update_ticks: u64,
    pub debounce_ticks: u64,
    pub key: Option<Keyboard>,
    pub is_pressed: bool,
}

#[entry]
fn main() -> ! {
    info!("Core0 Program start!");
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = cortex_m::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let mut sio = hal::Sio::new(pac.SIO);
    // ADC needs help:
    // https://github.com/rp-rs/rp-hal/issues/892
    // https://github.com/rp-rs/rp-hal/pull/920
    // let mut _adc = hal::Adc::new(pac.ADC, &mut pac.RESETS);
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    // Core clock setup. External crystal is 12MHz, CPU default clock is 125MHz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // Shared timer for times tasks, counts at 1_000_000 ticks per second
    let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    //USB bus peripheral initialization. used by NKRO library
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Pin Setup/state array for all NKRO key pins and control center buttons:
    let mut buttons: [ButtonState; NUM_BUTTONS] = [
        ButtonState {
            name: "P1_1",
            pin: pins.gpio0.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::A),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_2",
            pin: pins.gpio1.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::B),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_3",
            pin: pins.gpio2.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::C),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_4",
            pin: pins.gpio3.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::D),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_5",
            pin: pins.gpio4.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::E),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_6",
            pin: pins.gpio5.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::F),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_7",
            pin: pins.gpio6.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::G),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_Start",
            pin: pins.gpio7.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::H),
            is_pressed: false,
        },
        ButtonState {
            name: "P1_Select",
            pin: pins.gpio8.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::I),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_1",
            pin: pins.gpio9.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::J),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_2",
            pin: pins.gpio10.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::K),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_3",
            pin: pins.gpio11.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::L),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_4",
            pin: pins.gpio12.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::M),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_5",
            pin: pins.gpio13.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::N),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_6",
            pin: pins.gpio14.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::O),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_7",
            pin: pins.gpio15.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::P),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_Start",
            pin: pins.gpio16.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::Q),
            is_pressed: false,
        },
        ButtonState {
            name: "P2_Select",
            pin: pins.gpio17.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::R),
            is_pressed: false,
        },
        ButtonState {
            name: "Escape",
            pin: pins.gpio18.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: Some(Keyboard::Escape),
            is_pressed: false,
        },
        ButtonState {
            name: "CC_Up",
            pin: pins.gpio19.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: None,
            is_pressed: false,
        },
        ButtonState {
            name: "CC_Down",
            pin: pins.gpio20.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: None,
            is_pressed: false,
        },
        ButtonState {
            name: "CC_Left",
            pin: pins.gpio21.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: None,
            is_pressed: false,
        },
        ButtonState {
            name: "CC_Right",
            pin: pins.gpio22.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: None,
            is_pressed: false,
        },
        ButtonState {
            name: "CC_Select",
            pin: pins.gpio23.into_pull_up_input().into_dyn_pin(),
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            key: None,
            is_pressed: false,
        },
    ];

    // encoder pins:
    let _p1_encoder_pin_a = pins.gpio24.into_pull_up_input();
    let _p1_encoder_pin_b = pins.gpio25.into_pull_up_input();
    let _p2_encoder_pin_a = pins.gpio26.into_pull_up_input();
    let _p2_encoder_pin_b = pins.gpio27.into_pull_up_input();

    // LED strip control pin
    let _led_strip_data_pin = pins.gpio28.into_push_pull_output();

    //currently unused pins reserved for future:
    let _unused_pin_29 = pins.gpio29.into_pull_down_disabled();
    let _unused_pin_30 = pins.gpio30.into_pull_down_disabled();
    let _unused_pin_31 = pins.gpio31.into_pull_down_disabled();

    //SPI bus pins: (reserved for future peripherals, not currently in use)
    let _spi_rx_pin = pins.gpio32.into_pull_down_disabled();
    let _spi_cs_pin = pins.gpio33.into_pull_down_disabled();
    let _spi_sck_pin = pins.gpio34.into_pull_down_disabled();
    let _spi_tx_pin = pins.gpio35.into_pull_down_disabled();

    //i2c bus pins:
    let oled_sda_pin = pins.gpio36.reconfigure();
    let oled_scl_pin = pins.gpio37.reconfigure();

    // heartbeat LEDs
    let mut heartbeat_led_pin_core1 = pins.gpio38.into_push_pull_output();
    let mut heartbeat_led_pin_core0 = pins.gpio39.into_push_pull_output();

    // DPS ADC pins:
    // let _dsp_left_channel_in_pin = hal::adc::AdcPin::new(pins.gpio40).unwrap();
    // let _dsp_right_channel_in_pin = hal::adc::AdcPin::new(pins.gpio41).unwrap();

    //currently unused pins reserved for future:
    let _unused_pin_42 = pins.gpio42.into_pull_down_disabled();
    let _unused_pin_43 = pins.gpio43.into_pull_down_disabled();
    let _unused_pin_44 = pins.gpio44.into_pull_down_disabled();
    let _unused_pin_45 = pins.gpio45.into_pull_down_disabled();
    let _unused_pin_46 = pins.gpio46.into_pull_down_disabled();

    // pin 47 is being used for the psram cable select according to the Waveshare docs
    // Therefore it can't be used by us for anything.

    //i2c peripheral setup:
    let i2c = I2C::i2c0(
        pac.I2C0,
        oled_sda_pin,
        oled_scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );

    // usb keyboard peripheral setup:
    let mut keyboard = UsbHidClassBuilder::new()
        .add_device(
            usbd_human_interface_device::device::keyboard::NKROBootKeyboardConfig::default(),
        )
        .build(&usb_bus);

    //https://pid.codes
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x0001))
        .strings(&[StringDescriptors::default()
            .manufacturer("Tim Inc")
            .product("IIDX Deck")
            .serial_number("IIDX")])
        .unwrap()
        .build();

    //Start second core and begin its program loop:
    core1
        .spawn(CORE_STACK_1.take().unwrap(), move || {
            let _core = unsafe { cortex_m::Peripherals::steal() };
            info!("Core1 Program Start!");
            // Second core exclusive setup goes here:

            let mut last_core1_heartbeat_tick = 0_u64; // last time core 1 toggled its LED
            let core1_heartbeat_rate = 1_000_000_u64 / 3; // 3Hz in timer ticks

            // Second core loop:
            loop {
                // core1 heartbeat blink:
                if timer.get_counter().ticks() > (last_core1_heartbeat_tick + core1_heartbeat_rate)
                {
                    heartbeat_led_pin_core1.toggle().unwrap();
                    last_core1_heartbeat_tick = timer.get_counter().ticks();
                }
            }
        })
        .unwrap();

    // core0 loop state variables:
    let core0_heartbeat_rate = 1_000_000_u64 / 4; // 4Hz in timer ticks
    let mut last_core0_heartbeat_tick = 0_u64; // last time core 0 toggled its LED
    let mut last_usb_tick_ticks = 0_u64;
    let mut last_usb_key_state_send_ticks = 0_u64;

    // Main core loop:
    loop {
        // core0 heartbeat blink:
        if timer.get_counter().ticks() > (last_core0_heartbeat_tick + core0_heartbeat_rate) {
            heartbeat_led_pin_core0.toggle().unwrap();
            last_core0_heartbeat_tick = timer.get_counter().ticks();
        }

        // put the current state of all the duttons (debounced) into the button array:
        update_buttons(&mut buttons, &timer);

        // Sends a keyboard update at the specified interval
        if timer.get_counter().ticks() > (last_usb_key_state_send_ticks + USB_SEND_INTERVAL_TICKS) {
            last_usb_key_state_send_ticks = timer.get_counter().ticks();
            let keys = get_keys(&buttons);

            match keyboard.device().write_report(keys) {
                Err(UsbHidError::WouldBlock) => {}
                Err(UsbHidError::Duplicate) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write keyboard report: {:?}", e)
                }
            }
        }

        // Senda a USB tick at the 1ms interval specified by USB spec
        if timer.get_counter().ticks() > (last_usb_tick_ticks + USB_TICK_INTERVAL_TICKS) {
            last_usb_tick_ticks = timer.get_counter().ticks();
            match keyboard.tick() {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to process keyboard tick: {:?}", e)
                }
            };
        }

        // We need to read from the keyboard if it sends things or USB doesn't work:
        if usb_dev.poll(&mut [&mut keyboard]) {
            match keyboard.device().read_report() {
                Err(UsbError::WouldBlock) => {}
                Err(e) => {
                    core::panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(_leds) => {}
            }
        }
    }
}

fn update_buttons(buttons: &mut [ButtonState], timer: &Timer<CopyableTimer0>) {
    //we want to update the buttons per their individual debounce timings, and store the current value in the struct itself.
    for button in buttons {
        if timer.get_counter().ticks() > (button.last_update_ticks + button.debounce_ticks) {
            button.last_update_ticks = timer.get_counter().ticks();
            button.is_pressed = button.pin.is_low().unwrap();
        }
    }
}

/// This function will send the current button state of all buttons in the button array what have a
/// key mapped via the NKRO USB peripheral
fn get_keys(buttons: &[ButtonState]) -> [Keyboard; NUM_BUTTONS] {
    // default to taking no action, and only update keys being pressed:
    let mut keyboard: [Keyboard; NUM_BUTTONS] = [Keyboard::NoEventIndicated; 24];
    for (i, button) in buttons.iter().enumerate() {
        if let Some(key) = button.key {
            if button.is_pressed {
                keyboard[i] = key;
            }
        }
    }
    keyboard
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [rp235x_hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"RP2350 Template"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];

// End of file
