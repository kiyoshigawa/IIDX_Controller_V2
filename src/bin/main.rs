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
//! - New code and modifications by kiyoshigawa:
//!   Licensed under GPLv3
//!
//! SPDX-License-Identifier: GPL-3.0-or-later

#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embedded_hal::digital::*;
use fugit::RateExtU32;
use panic_probe as _;
use rp235x_hal::{
    self as hal, Clock,
    clocks::init_clocks_and_plls,
    entry,
    multicore::{Multicore, Stack},
    pac,
    pio::{Buffers, PIOExt},
    timer::{CopyableTimer0, Timer},
};
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};
use usb_device::{bus::*, class_prelude::*, prelude::*};
use usbd_human_interface_device::{page::Keyboard, prelude::*};

// Library crate imports — single source of truth for shared types & constants.
use iidx_controller_v2::input_handler::InputHandler;
use iidx_controller_v2::menu_handler::MenuHandler;
use iidx_controller_v2::*;

// ── Startup / binary-exclusive statics ──────────────────────────────────────

/// stack size for core 1: 32k of our 512k chip memory (with 2MB psram chip available as well)
static CORE_STACK_1: Stack<32768> = Stack::new();

/// Tell the Boot ROM about our application:
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

// ── Core0 entry point ──────────────────────────────────────────────────────

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
    let clocks = init_clocks_and_plls(
        EXTERNAL_XTAL_FREQ,
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
            pins.gpio0.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Z),
        ),
        ButtonState::new(
            "P1_2",
            pins.gpio1.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::S),
        ),
        ButtonState::new(
            "P1_3",
            pins.gpio2.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::X),
        ),
        ButtonState::new(
            "P1_4",
            pins.gpio3.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::D),
        ),
        ButtonState::new(
            "P1_5",
            pins.gpio4.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::C),
        ),
        ButtonState::new(
            "P1_6",
            pins.gpio5.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::F),
        ),
        ButtonState::new(
            "P1_7",
            pins.gpio6.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::V),
        ),
        ButtonState::new(
            "P1_Start",
            pins.gpio7.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Grave),
        ),
        ButtonState::new(
            "P1_Select",
            pins.gpio8.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Keyboard1),
        ),
        ButtonState::new(
            "P2_1",
            pins.gpio9.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::M),
        ),
        ButtonState::new(
            "P2_2",
            pins.gpio10.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::K),
        ),
        ButtonState::new(
            "P2_3",
            pins.gpio11.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Comma),
        ),
        ButtonState::new(
            "P2_4",
            pins.gpio12.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::L),
        ),
        ButtonState::new(
            "P2_5",
            pins.gpio13.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Dot),
        ),
        ButtonState::new(
            "P2_6",
            pins.gpio14.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Semicolon),
        ),
        ButtonState::new(
            "P2_7",
            pins.gpio15.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::ForwardSlash),
        ),
        ButtonState::new(
            "P2_Start",
            pins.gpio16.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::DeleteBackspace),
        ),
        ButtonState::new(
            "P2_Select",
            pins.gpio17.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Equal),
        ),
        ButtonState::new(
            "Escape",
            pins.gpio18.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Escape),
        ),
        ButtonState::new(
            "CC_Up",
            pins.gpio19.into_pull_down_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Down",
            pins.gpio20.into_pull_down_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Left",
            pins.gpio21.into_pull_down_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Right",
            pins.gpio22.into_pull_down_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "CC_Select",
            pins.gpio23.into_pull_down_input().into_dyn_pin(),
            None,
        ),
        ButtonState::new(
            "Volume_Up",
            pins.gpio24.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::VolumeUp),
        ),
        ButtonState::new(
            "Volume_Down",
            pins.gpio25.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::VolumeDown),
        ),
        ButtonState::new(
            "Mute",
            pins.gpio26.into_pull_down_input().into_dyn_pin(),
            Some(Keyboard::Mute),
        ),
    ];

    // RP2350-E9 hack to make pull-down inputs function properly.
    for button in &mut buttons {
        button.pin.set_input_enable(false);
    }

    // LED strip control pin
    let _led_strip_data_pin = pins.gpio27.into_pull_down_disabled();

    // encoder pins:
    let p1_encoder_pin_a = pins.gpio28.into_pull_up_input();
    let p1_encoder_pin_b = pins.gpio29.into_pull_up_input();
    let p2_encoder_pin_a = pins.gpio30.into_pull_up_input();
    let p2_encoder_pin_b = pins.gpio31.into_pull_up_input();

    //i2c bus pins usinf i2c0 device:
    let i2c_sda_pin = pins.gpio32.reconfigure();
    let i2c_scl_pin = pins.gpio33.reconfigure();

    //SPI bus pins using SPI0 device: (reserved for future peripherals, not currently in use)
    let _spi_sck_pin = pins.gpio34.into_pull_down_disabled();
    let _spi_tx_pin = pins.gpio35.into_pull_down_disabled();
    let _spi_rx_pin = pins.gpio36.into_pull_down_disabled();
    let _spi_cs_pin = pins.gpio37.into_pull_down_disabled();

    // heartbeat LEDs
    let mut heartbeat_led_pin_core1 = pins.gpio38.into_push_pull_output();
    let mut heartbeat_led_pin_core0 = pins.gpio39.into_push_pull_output(); // this is the led on the waveshare board

    //currently unused pins reserved for future:
    let _unused_pin_40 = pins.gpio40.into_pull_down_disabled();
    let _unused_pin_41 = pins.gpio41.into_pull_down_disabled();
    let _unused_pin_42 = pins.gpio42.into_pull_down_disabled();
    let _unused_pin_43 = pins.gpio43.into_pull_down_disabled();
    let _unused_pin_44 = pins.gpio44.into_pull_down_disabled();

    // DSP ADC pins:
    // let _dsp_left_channel_in_pin = hal::adc::AdcPin::new(pins.gpio45).unwrap();
    // let _dsp_right_channel_in_pin = hal::adc::AdcPin::new(pins.gpio46).unwrap();

    // pin 47 is being used for the psram cable select according to the Waveshare docs
    // Therefore it can't be used by us for anything.

    // i2c peripheral setup:
    let i2c = hal::I2C::i2c0(
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
    // https://pid.codes
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

    let (mut encoder_pio, encoder_sm0, encoder_sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let program = encoder_pio.install(&program.program).unwrap();
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
        .build(encoder_sm0);
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
        .build(encoder_sm1);
    sm_p2.set_pindirs([
        (p2_encoder_pin_a_pin, hal::pio::PinDir::Input),
        (p2_encoder_pin_b_pin, hal::pio::PinDir::Input),
    ]);
    sm_p2.start();

    // i2c SD1306 oled setup:
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // Set up WS2812 LED PIO:
    // let (mut leds_pio, leds_sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
    // let mut led_strip = Ws2812Direct::new(
    //     led_strip_data_pin,
    //     &mut leds_pio,
    //     leds_sm0,
    //     EXTERNAL_XTAL_FREQ,
    // );

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

            // These will handle processing input changes and triggering events based on the input state.
            let menu_handler = MenuHandler::new(&mut display);
            let mut input_handler = InputHandler::new(menu_handler);

            // core1 loop state variables:
            let mut last_core1_heartbeat_tick = 0_u64; // last time core 1 toggled its LED
            let mut last_screen_update_ticks = 0_u64;
            let mut last_led_update_ticks = 0_u64;

            // core1 loop:
            loop {
                //get core0 variable info:
                while (sio.fifo.status() & 0b1) != 0 {
                    // Bit 0 VLD: Value is 1 if this core's RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
                    let packed = sio.fifo.read_blocking();
                    let (header, word) = extract_header_from_word(packed);
                    match header {
                        CURRENT_BUTTON_STATE_HEADER => input_handler.current_button_state = word,
                        ENCODER_P1_COUNT_HEADER => {
                            input_handler.encoder_p1_count = sign_extend_27bit(word)
                        }
                        ENCODER_P2_COUNT_HEADER => {
                            input_handler.encoder_p2_count = sign_extend_27bit(word)
                        }
                        // RESERVED_STATE_HEADER => {},
                        _ => {} // unknown header, ignore
                    }
                }

                // Update inputs using any new SIO data first so that events can fire based on changes:
                input_handler.detect_input_changes();

                // core1 heartbeat blink:
                if timer.get_counter().ticks() > (last_core1_heartbeat_tick + CORE1_HEARTBEAT_RATE)
                {
                    heartbeat_led_pin_core1.toggle().unwrap();
                    last_core1_heartbeat_tick = timer.get_counter().ticks();
                }

                // core1 led strip update:
                if timer.get_counter().ticks() > (last_led_update_ticks + LED_FRAME_TICKS) {
                    last_led_update_ticks = timer.get_counter().ticks();
                    // led_strip.write([color].iter().copied()).unwrap();
                }

                // core1 LCD screen updates:
                if timer.get_counter().ticks() > (last_screen_update_ticks + SCREEN_REFRESH_TICKS) {
                    last_screen_update_ticks = timer.get_counter().ticks();
                    input_handler.update_display();
                }

                // We update this last so everything that needs to react to an input change in the
                // handlers above can do so before we reset them.
                input_handler.previous_button_state = input_handler.current_button_state;
            }
        })
        .unwrap();

    // ── Core0 loop ────────────────────────────────────────────────────────

    // core0 loop state variables
    let mut last_core0_heartbeat_tick = 0_u64; // last time core 0 toggled its LED
    let mut last_usb_tick_ticks = 0_u64;
    let mut last_usb_key_state_send_ticks = 0_u64;
    let mut encoder_p1_count = 0_i32;
    let mut encoder_p2_count = 0_i32;
    let mut encoder_p1_last_update_ticks = 0_u64;
    let mut encoder_p2_last_update_ticks = 0_u64;
    let mut last_button_update_ticks = 0_u64;

    // core0 loop:
    loop {
        // core0 heartbeat blink:
        if timer.get_counter().ticks() > (last_core0_heartbeat_tick + CORE0_HEARTBEAT_RATE) {
            heartbeat_led_pin_core0.toggle().unwrap();
            last_core0_heartbeat_tick = timer.get_counter().ticks();
        }

        // put the current state of all the buttons (debounced) into the buttons array:
        update_buttons(&mut buttons, &timer);

        // prep button states for use on core1:
        let (current_button_state, previous_button_state) = encode_button_state(&buttons);

        if current_button_state != previous_button_state {
            last_button_update_ticks = timer.get_counter().ticks();
        }

        // read encoder positions from FIFO buffers for use here AND on core1:
        while !rx_p1.is_empty() {
            if let Some(value) = rx_p1.read() {
                if timer.get_counter().ticks()
                    > (encoder_p1_last_update_ticks + DEFAULT_ENCODER_DEBOUNCE_TICKS)
                {
                    if encoder_p1_count != value as i32 {
                        encoder_p1_last_update_ticks = timer.get_counter().ticks();
                        encoder_p1_count = value as i32;
                        last_button_update_ticks = timer.get_counter().ticks();
                    }
                }
            }
        }
        while !rx_p2.is_empty() {
            if let Some(value) = rx_p2.read() {
                if timer.get_counter().ticks()
                    > (encoder_p2_last_update_ticks + DEFAULT_ENCODER_DEBOUNCE_TICKS)
                {
                    if encoder_p2_count != value as i32 {
                        encoder_p2_last_update_ticks = timer.get_counter().ticks();
                        encoder_p2_count = value as i32;
                        last_button_update_ticks = timer.get_counter().ticks();
                    }
                }
            }
        }

        // since the source of truth on the encoder counts is in the PIO registers, we need to clear them ourselves ot reset the encoders.
        // The easiest way I found to do this is to reset the chip after the idle timeout.
        if timer.get_counter().ticks() > (last_button_update_ticks + IDLE_RESET_TIMEOUT_TICKS) {
            cortex_m::peripheral::SCB::sys_reset();
        }

        // send core1 data to core1 if it has room:
        let fifo_is_empty = (sio.fifo.status() & 0b1) == 0; // Bit 0 VLD: Value is 1 if this core's RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
        if fifo_is_empty {
            let packed_current_button_state =
                add_header_to_word(CURRENT_BUTTON_STATE_HEADER, current_button_state);
            let packed_encoder_p1_count =
                add_header_to_word(ENCODER_P1_COUNT_HEADER, encoder_p1_count as u32);
            let packed_encoder_p2_count =
                add_header_to_word(ENCODER_P2_COUNT_HEADER, encoder_p2_count as u32);
            // let reserved_state =
            //     add_header_to_word(RESERVED_STATE_HEADER, previous_button_state);
            sio.fifo.write(packed_current_button_state);
            sio.fifo.write(packed_encoder_p1_count);
            sio.fifo.write(packed_encoder_p2_count);
            // sio.fifo.write(reserved_state);
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

// ── Standalone helper functions ────────────────────────────────────────────

/// This will iterate over all the buttons in the button array, and will update their state when it differs from the previous value.
/// States can only change if they occur more than debounce_ticks after the last state change. This will update the state of
/// both the keyboard buttons as well as the control center buttons.
fn update_buttons(buttons: &mut [ButtonState], timer: &Timer<CopyableTimer0>) {
    //we want to update the buttons per their individual debounce timings, and store the current value in the struct itself.
    for button in buttons {
        if timer.get_counter().ticks() > (button.last_update_ticks + button.debounce_ticks) {
            button.pin.set_input_enable(true); // RP2350-E9 hack to make pull-down inputs function properly.
            let current_button_state = button.pin.is_high().unwrap();
            button.pin.set_input_enable(false); // RP2350-E9 hack to make pull-down inputs function properly.
            if current_button_state != button.was_pressed {
                button.last_update_ticks = timer.get_counter().ticks();
                button.was_pressed = button.is_pressed;
                button.is_pressed = current_button_state;
            }
        } else {
            // too soon for post-debounce complete update, so we don't need to check the pin value, but still need to set the previous
            // state updated so changes only fire once
            button.was_pressed = button.is_pressed;
        }
    }
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
