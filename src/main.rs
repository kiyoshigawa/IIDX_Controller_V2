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

use core::fmt::Write;
use defmt::info;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_9X18_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal::digital::*;
use fugit::RateExtU32;
use panic_probe as _;
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
const NUM_BUTTONS: usize = 27;

/// Default debounce time in ticks (1,000,000 ticks per second)
const DEFAULT_DEBOUNCE_TICKS: u64 = 10_000;

/// Default usb device tick send time in ticks (1,000,000 ticks per second)
/// This should be 1ms per USB spec or device will lose connection
const USB_TICK_INTERVAL_TICKS: u64 = 1_000;

/// Default keyboard send rate in ticks (1,000,000 ticks per second)
const USB_SEND_INTERVAL_TICKS: u64 = 1_000;

/// oled sreen min ticks between refreshes. (1,000,000 ticks per second)
const SCREEN_REFRESH_TICKS: u64 = 100_000; //10Hz

/// stack size for core 1: increase as needed, defaulting to 32k of our 512k chip memory (with 2MB psram chip available as well)
static CORE_STACK_1: Stack<32768> = Stack::new();

/// size for the FmtBuf buffer in bytes. Screen can only hold ~14 chars per line, so it can be small for us
const BUF_SIZE: usize = 16;

/// Tell the Boot ROM about our application:
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

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
        ButtonState::new(
            "P1_1",
            pins.gpio0.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Z),
        ),
        ButtonState::new(
            "P1_2",
            pins.gpio1.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::S),
        ),
        ButtonState::new(
            "P1_3",
            pins.gpio2.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::X),
        ),
        ButtonState::new(
            "P1_4",
            pins.gpio3.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::D),
        ),
        ButtonState::new(
            "P1_5",
            pins.gpio4.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::C),
        ),
        ButtonState::new(
            "P1_6",
            pins.gpio5.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::F),
        ),
        ButtonState::new(
            "P1_7",
            pins.gpio6.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::V),
        ),
        ButtonState::new(
            "P1_Start",
            pins.gpio7.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Grave),
        ),
        ButtonState::new(
            "P1_Select",
            pins.gpio8.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Keyboard1),
        ),
        ButtonState::new(
            "P2_1",
            pins.gpio9.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::M),
        ),
        ButtonState::new(
            "P2_2",
            pins.gpio10.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::K),
        ),
        ButtonState::new(
            "P2_3",
            pins.gpio11.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Comma),
        ),
        ButtonState::new(
            "P2_4",
            pins.gpio12.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::L),
        ),
        ButtonState::new(
            "P2_5",
            pins.gpio13.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Dot),
        ),
        ButtonState::new(
            "P2_6",
            pins.gpio14.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Semicolon),
        ),
        ButtonState::new(
            "P2_7",
            pins.gpio15.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::ForwardSlash),
        ),
        ButtonState::new(
            "P2_Start",
            pins.gpio16.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::DeleteBackspace),
        ),
        ButtonState::new(
            "P2_Select",
            pins.gpio17.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Equal),
        ),
        ButtonState::new(
            "Escape",
            pins.gpio18.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Escape),
        ),
        ButtonState::new(
            "CC_Up",
            pins.gpio19.into_pull_up_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Down",
            pins.gpio20.into_pull_up_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Left",
            pins.gpio21.into_pull_up_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Right",
            pins.gpio22.into_pull_up_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Select",
            pins.gpio23.into_pull_up_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "Volume_Up",
            pins.gpio24.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::VolumeUp),
        ),
        ButtonState::new(
            "Volume_Down",
            pins.gpio25.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::VolumeDown),
        ),
        ButtonState::new(
            "Mute",
            pins.gpio26.into_pull_up_input().into_dyn_pin(),
            Some(Keyboard::Mute),
        ),
    ];

    // LED strip control pin
    let _led_strip_data_pin = pins.gpio27.into_push_pull_output();

    // encoder pins:
    let p1_encoder_pin_a = pins.gpio28.into_pull_up_input();
    let p1_encoder_pin_b = pins.gpio29.into_pull_up_input();
    let p2_encoder_pin_a = pins.gpio30.into_pull_up_input();
    let p2_encoder_pin_b = pins.gpio31.into_pull_up_input();

    //i2c bus pins usinf i2c0 device:
    let i2c_sda_pin = pins.gpio32.reconfigure();
    let i2c_scl_pin = pins.gpio33.reconfigure();

    //SPI bus pins using SPI0 device: (reserved for future peripherals, not currently in use)
    let _spi_cs_pin = pins.gpio34.into_pull_down_disabled();
    let _spi_sck_pin = pins.gpio35.into_pull_down_disabled();
    let _spi_tx_pin = pins.gpio36.into_pull_down_disabled();
    let _spi_rx_pin = pins.gpio37.into_pull_down_disabled();

    // heartbeat LEDs
    let mut heartbeat_led_pin_core1 = pins.gpio38.into_push_pull_output();
    let mut heartbeat_led_pin_core0 = pins.gpio39.into_push_pull_output(); // this is the led on the waveshare board

    //currently unused pins reserved for future:
    let _unused_pin_40 = pins.gpio40.into_pull_down_disabled();
    let _unused_pin_41 = pins.gpio41.into_pull_down_disabled();
    let _unused_pin_42 = pins.gpio42.into_pull_down_disabled();
    let _unused_pin_43 = pins.gpio43.into_pull_down_disabled();
    let _unused_pin_44 = pins.gpio44.into_pull_down_disabled();

    // DPS ADC pins:
    // let _dsp_left_channel_in_pin = hal::adc::AdcPin::new(pins.gpio45).unwrap();
    // let _dsp_right_channel_in_pin = hal::adc::AdcPin::new(pins.gpio46).unwrap();

    // pin 47 is being used for the psram cable select according to the Waveshare docs
    // Therefore it can't be used by us for anything.

    // i2c peripheral setup:
    let i2c = I2C::i2c0(
        pac.I2C0,
        i2c_sda_pin,
        i2c_scl_pin,
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

    // PIO Encoder test Setup - Original ASM from adamgreen:
    // Copyright 2021 Adam Green (https://github.com/adamgreen/QuadratureDecoder)
    // Licensed under the Apache License, Version 2.0
    // See: http://www.apache.org/licenses/LICENSE-2.0

    // Use the RP2040's PIO state machines to count quadrature encoder ticks.
    let program = pio::pio_asm!(
        ".origin 0",
        // 16 element jump table based on 4-bit encoder last state and current state.
        "    jmp delta0", // 00-00
        "    jmp minus1", // 00-01
        "    jmp plus1",  // 00-10
        "    jmp delta0", // 00-11
        "    jmp plus1",  // 01-00
        "    jmp delta0", // 01-01
        "    jmp delta0", // 01-10
        "    jmp minus1", // 01-11
        "    jmp minus1", // 10-00
        "    jmp delta0", // 10-01
        "    jmp delta0", // 10-10
        "    jmp plus1",  // 10-11
        "    jmp delta0", // 11-00
        "    jmp plus1",  // 11-01
        "    jmp minus1", // 11-10
        "    jmp delta0", // 11-11
        ".wrap_target",
        "delta0:",
        "    mov isr, null", // Make sure that the input shift register is cleared when table jumps to delta0.
        "    in y, 2", // Upper 2-bits of address are formed from previous encoder pin readings
        "    mov y, pins", // Lower 2-bits of address are formed from current encoder pin readings. Save in Y as well.
        "    in y, 2",
        "    mov pc, isr", // Jump into jump table which will then jump to delta0, minus1, or plus1 labels.
        "minus1:",
        "    jmp x-- output", // Decrement x
        "    jmp output",
        "plus1:",
        "    mov x, ~x", // Increment x by calculating x=~(~x - 1)
        "    jmp x-- next2",
        "next2:",
        "    mov x, ~x",
        "output:",
        "    mov isr, x", // Push out updated counter.
        "    push noblock",
        ".wrap"
    );

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let program = pio.install(&program.program).unwrap();
    let program2 = unsafe { program.share() };

    let p1_encoder_pin_a_pin = p1_encoder_pin_a.id().num;
    let p1_encoder_pin_b_pin = p1_encoder_pin_b.id().num;
    let p2_encoder_pin_a_pin = p2_encoder_pin_a.id().num;
    let p2_encoder_pin_b_pin = p2_encoder_pin_b.id().num;

    let (mut sm_p1, mut rx_p1, _) = hal::pio::PIOBuilder::from_installed_program(program)
        .in_pin_base(p1_encoder_pin_a_pin)
        .in_count(2)
        .in_shift_direction(rp235x_hal::pio::ShiftDirection::Left)
        .buffers(Buffers::OnlyRx)
        .build(sm0);
    sm_p1.set_pindirs([
        (p1_encoder_pin_a_pin, hal::pio::PinDir::Input),
        (p1_encoder_pin_b_pin, hal::pio::PinDir::Input),
    ]);
    sm_p1.start();

    let (mut sm_p2, mut rx_p2, _) = hal::pio::PIOBuilder::from_installed_program(program2)
        .in_pin_base(p2_encoder_pin_a_pin)
        .in_count(2)
        .in_shift_direction(rp235x_hal::pio::ShiftDirection::Left)
        .buffers(Buffers::OnlyRx)
        .build(sm1);
    sm_p2.set_pindirs([
        (p2_encoder_pin_a_pin, hal::pio::PinDir::Input),
        (p2_encoder_pin_b_pin, hal::pio::PinDir::Input),
    ]);
    sm_p2.start();

    // i2c SD1306 oled setup:
    // I also make 4 text buffers to use to writ the 4 viible lines of text on the screen
    let interface = ssd1306::I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_9X18_BOLD)
        .text_color(BinaryColor::On)
        .build();
    // Array of four 64 byte text buffers. Each buffer will be a line of text on the oled
    let mut line_bufs: [FmtBuf; 4] = [FmtBuf::new(), FmtBuf::new(), FmtBuf::new(), FmtBuf::new()];

    //Start second core (core1) and begin its program loop:
    core1
        .spawn(CORE_STACK_1.take().unwrap(), move || {
            info!("Core1 Program Start!");

            let _core = unsafe { cortex_m::Peripherals::steal() };
            let pac = unsafe { hal::pac::Peripherals::steal() };
            let mut sio = hal::Sio::new(pac.SIO);

            // core1 exclusive setup goes here:
            // Use this for things you want on the memory reserved for the core1 stack, not in main memory
            // don't use this area for shared peripherals, they should be set up outside this function

            // core1 loop state variables:
            let mut last_core1_heartbeat_tick = 0_u64; // last time core 1 toggled its LED
            let core1_heartbeat_rate = 1_000_000_u64 / 3; // 3Hz in timer ticks
            let mut last_screen_update_ticks = 0_u64;
            let mut frames_rendered = 0_u64; // variable for counting number of screen refreshes since reboot
            let mut _current_button_state = 0_u32;
            let mut _previous_button_state = 0_u32;
            let mut encoder_p1_count: i32 = 0;
            let mut encoder_p2_count: i32 = 0;

            // core1 loop:
            loop {
                //get core0 variable info:
                let fifo_is_empty = (sio.fifo.status() & 0b1) == 0; // Bit 0 VLD: Value is 1 if this core’s RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
                if !fifo_is_empty {
                    _current_button_state = sio.fifo.read_blocking();
                    _previous_button_state = sio.fifo.read_blocking();
                    encoder_p1_count = sio.fifo.read_blocking() as i32;
                    encoder_p2_count = sio.fifo.read_blocking() as i32;
                }

                // core1 heartbeat blink:
                if timer.get_counter().ticks() > (last_core1_heartbeat_tick + core1_heartbeat_rate)
                {
                    heartbeat_led_pin_core1.toggle().unwrap();
                    last_core1_heartbeat_tick = timer.get_counter().ticks();
                }

                // core1 LCD screen updates:
                if timer.get_counter().ticks() > (last_screen_update_ticks + SCREEN_REFRESH_TICKS) {
                    last_screen_update_ticks = timer.get_counter().ticks();

                    frames_rendered += 1;

                    // Update the lines to be written:
                    for line in &mut line_bufs {
                        line.reset();
                    }
                    write!(&mut line_bufs[0], "fc: {}", frames_rendered).unwrap();
                    write!(&mut line_bufs[1], "Line 2 is fixed.").unwrap();
                    write!(&mut line_bufs[2], "enc1: {}", encoder_p1_count).unwrap();
                    write!(&mut line_bufs[3], "enc2: {}", encoder_p2_count).unwrap();

                    // Empty the display:
                    let color = embedded_graphics::pixelcolor::BinaryColor::Off;
                    display.clear(color).unwrap();

                    // Write the buffers to the display buffer and update the screen:
                    for (i, line) in line_bufs.iter().enumerate() {
                        let line_top_pixel = 16 * i as i32;
                        Text::with_baseline(
                            line.as_str(),
                            Point::new(0, line_top_pixel),
                            text_style,
                            Baseline::Top,
                        )
                        .draw(&mut display)
                        .unwrap();
                    }
                    display.flush().unwrap();
                }
            }
        })
        .unwrap();

    // core0 loop state variables
    let core0_heartbeat_rate = 1_000_000_u64 / 4; // 4Hz in timer ticks
    let mut last_core0_heartbeat_tick = 0_u64; // last time core 0 toggled its LED
    let mut last_usb_tick_ticks = 0_u64;
    let mut last_usb_key_state_send_ticks = 0_u64;
    let mut encoder_p1_count: i32 = 0;
    let mut encoder_p2_count: i32 = 0;

    // core0 loop:
    loop {
        // core0 heartbeat blink:
        if timer.get_counter().ticks() > (last_core0_heartbeat_tick + core0_heartbeat_rate) {
            heartbeat_led_pin_core0.toggle().unwrap();
            last_core0_heartbeat_tick = timer.get_counter().ticks();
        }

        // put the current state of all the buttons (debounced) into the button array:
        update_buttons(&mut buttons, &timer);

        // prep button states for use on core1:
        let (current_button_state, previous_button_state) = encode_button_state(&buttons);

        // read encoder positions from FIFO buffers for use here AND on core1:
        while !rx_p1.is_empty() {
            if let Some(value) = rx_p1.read() {
                info!("Encoder P1 Position: {}", value as i32);
                encoder_p1_count = value as i32;
            }
        }
        while !rx_p2.is_empty() {
            if let Some(value) = rx_p2.read() {
                info!("Encoder P1 Position: {}", value as i32);
                encoder_p2_count = value as i32;
            }
        }

        // send core1 data to core1 if it has room:
        let fifo_is_empty = (sio.fifo.status() & 0b1) == 0; // Bit 0 VLD: Value is 1 if this core’s RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
        if fifo_is_empty {
            sio.fifo.write(current_button_state as u32);
            sio.fifo.write(previous_button_state as u32);
            sio.fifo.write(encoder_p1_count as u32);
            sio.fifo.write(encoder_p2_count as u32);
        }

        // Sends a USB tick at the 1ms interval specified by USB spec
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

/// This will iterate over all the buttons in the button array, and will update their state when it differs from the previous value.
/// States can only change if they occur more than debounce_ticks after the last state change. This will update the state of
/// both the keyboard buttons as well as the control center buttons.
fn update_buttons(buttons: &mut [ButtonState], timer: &Timer<CopyableTimer0>) {
    //we want to update the buttons per their individual debounce timings, and store the current value in the struct itself.
    for button in buttons {
        if timer.get_counter().ticks() > (button.last_update_ticks + button.debounce_ticks) {
            let current_button_state = button.pin.is_low().unwrap();
            if current_button_state != button.was_pressed {
                button.last_update_ticks = timer.get_counter().ticks();
                button.was_pressed = button.is_pressed;
                button.is_pressed = current_button_state;
            }
        } else {
            // too soon for post-debounce complete update, so we don't need to check the pin value, but still need to get the previous
            // state updated so changes only fire once
            button.was_pressed = button.is_pressed;
        }
    }
}

/// this function returns an two u32 values representing binary button states for all the NUM_BUTTONS buttons in the buttons array, in order.
/// (current_state, previous_state)
fn encode_button_state(buttons: &[ButtonState]) -> (u32, u32) {
    let mut current_state = 0_u32;
    let mut previous_state = 0_u32;

    for (position, button) in buttons.iter().enumerate() {
        let mut is_pressed = button.is_pressed as u32;
        is_pressed = is_pressed << position; // move the state of the button to the right position
        current_state |= is_pressed;
        let mut was_pressed = button.was_pressed as u32;
        was_pressed = was_pressed << position; // move the state of the button to the right position
        previous_state |= was_pressed;
    }

    (current_state, previous_state)
}

/// This function will send the current button state of all buttons in the button array what have a
/// key mapped via the NKRO USB peripheral
fn get_keys(buttons: &[ButtonState]) -> [Keyboard; NUM_BUTTONS] {
    // default to taking no action, and only update keys being pressed:
    let mut keyboard: [Keyboard; NUM_BUTTONS] = [Keyboard::NoEventIndicated; NUM_BUTTONS];
    for (i, button) in buttons.iter().enumerate() {
        if let Some(key) = button.key {
            if button.is_pressed {
                keyboard[i] = key;
            }
        }
    }
    keyboard
}

// struct for storing all the button info to make iterative upating and configuring easier:
pub struct ButtonState {
    pub name: &'static str,
    pub pin: Pin<DynPinId, FunctionSioInput, PullUp>,
    pub last_update_ticks: u64,
    pub debounce_ticks: u64,
    pub key: Option<Keyboard>,
    pub is_pressed: bool,
    pub was_pressed: bool,
}

impl ButtonState {
    /// Creates a new ButtonState struct using the default values where appropriate.
    fn new(
        name: &'static str,
        pin: Pin<DynPinId, FunctionSioInput, PullUp>,
        key: Option<Keyboard>,
    ) -> Self {
        Self {
            name,
            pin,
            key,
            last_update_ticks: 0,
            debounce_ticks: DEFAULT_DEBOUNCE_TICKS,
            is_pressed: false,
            was_pressed: false,
        }
    }

    /// Will return true if the button state changed from unpressed to pressed on the most recent update.
    /// Will return false if the button state is the same as its previous state or if the button was released
    fn _press_occurred_this_update(&self) -> bool {
        self.is_pressed && !self.was_pressed
    }

    /// Will return true if the button state changed from pressed to unpressed on the most recent update.
    /// Will return false if the button state is the same as its previous state or if the button was pressed
    fn _release_occurred_this_update(&self) -> bool {
        !self.is_pressed && self.was_pressed
    }
}

/// This is a very simple buffer to pre format a short line of text
/// limited arbitrarily to BUF_SIZE bytes.
struct FmtBuf {
    buf: [u8; BUF_SIZE],
    ptr: usize,
}

impl FmtBuf {
    fn new() -> Self {
        Self {
            buf: [0; BUF_SIZE],
            ptr: 0,
        }
    }

    fn reset(&mut self) {
        self.ptr = 0;
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[0..self.ptr]).unwrap()
    }
}

impl core::fmt::Write for FmtBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let rest_len = self.buf.len() - self.ptr;
        let len = if rest_len < s.len() {
            rest_len
        } else {
            s.len()
        };
        self.buf[self.ptr..(self.ptr + len)].copy_from_slice(&s.as_bytes()[0..len]);
        self.ptr += len;
        Ok(())
    }
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
