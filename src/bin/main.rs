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

use core::sync::atomic::Ordering;
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

// Library crate imports from this repo
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

    // To reset persistent storage to factory defaults, uncomment the lines below,
    // flash the device, then re-comment and re-flash:
    // unsafe {
    //     clear_storage(&FlashStoragePersistentMemory::default());
    // }

    // Read the persistent configuration from flash so we can use it to initilize everything
    let config: &FlashStoragePersistentMemory = unsafe {
        let raw = &*(FLASH_STORAGE_BASE_ADDR as *const FlashStoragePersistentMemory);

        if !raw.has_been_written() {
            info!("Storage is fresh. Writing default configuration to flash...");
            let defaults = FlashStoragePersistentMemory::default();
            write_storage(&defaults);
            info!("Default configuration written.");

            // Re-read after cache flush so we see the freshly written data.
            &*(FLASH_STORAGE_BASE_ADDR as *const FlashStoragePersistentMemory)
        } else {
            info!("Storage is initialized. Using stored configuration.");
            raw
        }
    };

    // Shared timer for timed tasks, counts at 1_000_000 ticks per second (or 1 tick per us if you prefer)
    let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

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

    // Pin Setup/state array for all physical buttons connected to GPIO pins:
    let mut buttons: [ButtonState; NUM_BUTTONS] = [
        ButtonState::new(
            "P1_1",
            pins.gpio0.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[0].key),
            config.buttons[0].debounce_ticks,
        ),
        ButtonState::new(
            "P1_2",
            pins.gpio1.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[1].key),
            config.buttons[1].debounce_ticks,
        ),
        ButtonState::new(
            "P1_3",
            pins.gpio2.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[2].key),
            config.buttons[2].debounce_ticks,
        ),
        ButtonState::new(
            "P1_4",
            pins.gpio3.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[3].key),
            config.buttons[3].debounce_ticks,
        ),
        ButtonState::new(
            "P1_5",
            pins.gpio4.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[4].key),
            config.buttons[4].debounce_ticks,
        ),
        ButtonState::new(
            "P1_6",
            pins.gpio5.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[5].key),
            config.buttons[5].debounce_ticks,
        ),
        ButtonState::new(
            "P1_7",
            pins.gpio6.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[6].key),
            config.buttons[6].debounce_ticks,
        ),
        ButtonState::new(
            "P1_Start",
            pins.gpio7.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[7].key),
            config.buttons[7].debounce_ticks,
        ),
        ButtonState::new(
            "P1_Select",
            pins.gpio8.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[8].key),
            config.buttons[8].debounce_ticks,
        ),
        ButtonState::new(
            "P2_1",
            pins.gpio9.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[9].key),
            config.buttons[9].debounce_ticks,
        ),
        ButtonState::new(
            "P2_2",
            pins.gpio10.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[10].key),
            config.buttons[10].debounce_ticks,
        ),
        ButtonState::new(
            "P2_3",
            pins.gpio11.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[11].key),
            config.buttons[11].debounce_ticks,
        ),
        ButtonState::new(
            "P2_4",
            pins.gpio12.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[12].key),
            config.buttons[12].debounce_ticks,
        ),
        ButtonState::new(
            "P2_5",
            pins.gpio13.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[13].key),
            config.buttons[13].debounce_ticks,
        ),
        ButtonState::new(
            "P2_6",
            pins.gpio14.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[14].key),
            config.buttons[14].debounce_ticks,
        ),
        ButtonState::new(
            "P2_7",
            pins.gpio15.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[15].key),
            config.buttons[15].debounce_ticks,
        ),
        ButtonState::new(
            "P2_Start",
            pins.gpio16.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[16].key),
            config.buttons[16].debounce_ticks,
        ),
        ButtonState::new(
            "P2_Select",
            pins.gpio17.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[17].key),
            config.buttons[17].debounce_ticks,
        ),
        ButtonState::new(
            "Escape",
            pins.gpio18.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[18].key),
            config.buttons[18].debounce_ticks,
        ),
        ButtonState::new(
            "CC_Up",
            pins.gpio19.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[19].key),
            config.buttons[19].debounce_ticks,
        ),
        ButtonState::new(
            "CC_Down",
            pins.gpio20.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[20].key),
            config.buttons[20].debounce_ticks,
        ),
        ButtonState::new(
            "CC_Left",
            pins.gpio21.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[21].key),
            config.buttons[21].debounce_ticks,
        ),
        ButtonState::new(
            "CC_Right",
            pins.gpio22.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[22].key),
            config.buttons[22].debounce_ticks,
        ),
        ButtonState::new(
            "CC_Select",
            pins.gpio23.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[23].key),
            config.buttons[23].debounce_ticks,
        ),
        ButtonState::new(
            "Volume_Up",
            pins.gpio24.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[24].key),
            config.buttons[24].debounce_ticks,
        ),
        ButtonState::new(
            "Volume_Down",
            pins.gpio25.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[25].key),
            config.buttons[25].debounce_ticks,
        ),
        ButtonState::new(
            "Mute",
            pins.gpio26.into_pull_down_input().into_dyn_pin(),
            u8_to_key(config.buttons[26].key),
            config.buttons[26].debounce_ticks,
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

    // currently unused pins reserved for future:
    // Note: You'll need to change how the inter-core SIO FIFO data is sent to get these into core1 functions.
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

    // PIO Encoder Setup - Original ASM from adamgreen:
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

    let mut encoders: [EncoderState; NUM_ENCODERS] = [
        EncoderState::new(
            "P1 Encoder",
            u8_to_key(config.encoders[0].key_up),
            u8_to_key(config.encoders[0].key_down),
            &mut rx_p1,
            config.encoders[0].debounce_ticks,
            config.encoders[0].step_threshold,
            config.encoders[0].move_timeout_ticks,
        ),
        EncoderState::new(
            "P2 Encoder",
            u8_to_key(config.encoders[1].key_up),
            u8_to_key(config.encoders[1].key_down),
            &mut rx_p2,
            config.encoders[1].debounce_ticks,
            config.encoders[1].step_threshold,
            config.encoders[1].move_timeout_ticks,
        ),
    ];

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
            // don't use this area for shared peripherals, they should be set up outside this function.

            // These will handle processing input changes and triggering events based on the input state.
            let menu_handler = MenuHandler::new(&mut display, *config);
            let mut input_handler = InputHandler::new(menu_handler);

            // core1 loop state variables:
            let mut last_core1_heartbeat_tick = 0_u64;
            let mut last_screen_update_ticks = 0_u64;
            let mut last_led_update_ticks = 0_u64;

            // core1 loop:
            loop {
                // core1 heartbeat blink:
                if timer.get_counter().ticks() > (last_core1_heartbeat_tick + CORE1_HEARTBEAT_RATE)
                {
                    heartbeat_led_pin_core1.toggle().unwrap();
                    last_core1_heartbeat_tick = timer.get_counter().ticks();
                }

                //get core0 state variable info from SIO FIFO registers:
                while (sio.fifo.status() & 0b1) != 0 {
                    // Bit 0 VLD: Value is 1 if this core's RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
                    // These values are manually hardcoded. If you change something that effects them, you need to also fix things here.
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
                        ENCODER_DIRECTION_HEADER => {
                            input_handler.encoder_p1_direction =
                                EncoderDirection::from((word & 0b11) as u8);
                            input_handler.encoder_p2_direction =
                                EncoderDirection::from(((word >> 2) & 0b11) as u8);
                        }
                        _ => {} // unknown header, ignore
                    }
                }

                // Update inputs using any new SIO data first so that events can fire based on changes:
                input_handler.detect_input_changes();

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
                input_handler.previous_combined_button_state =
                    input_handler.current_combined_button_state;
            }
        })
        .unwrap();

    // core0 loop state variables
    let mut last_core0_heartbeat_tick = 0_u64;
    let mut last_usb_tick_ticks = 0_u64;
    let mut last_usb_key_state_send_ticks = 0_u64;
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

        // Used for idle timeout reset countdown reset
        if current_button_state != previous_button_state {
            last_button_update_ticks = timer.get_counter().ticks();
        }

        // read encoder positions from PIO FIFO rx buffers for use here AND on core1:
        read_encoder_fifos(&mut encoders, &timer, &mut last_button_update_ticks);

        // update encoder direction state based on the newly-read counts:
        update_encoders(&mut encoders, &timer);

        // After the idle timeout, we need to clear the encoders back to 0 to prevent rollover.
        // since the source of truth on the encoder counts is in the PIO registers, we need to
        // clear them ourselves. The easiest way I found to do this is to reset the chip after
        // the idle timeout.
        if timer.get_counter().ticks() > (last_button_update_ticks + IDLE_RESET_TIMEOUT_TICKS) {
            cortex_m::peripheral::SCB::sys_reset();
        }

        // send core0 data to core1 if it has room:
        let fifo_is_empty = (sio.fifo.status() & 0b1) == 0;
        // Bit 0 VLD: Value is 1 if this core's RX FIFO is not empty (i.e. if FIFO_RD is valid) - RP235x datasheet pg. 67
        // These values are manually hardcoded. If you change something that effects them, you need to also fix things here.
        if fifo_is_empty {
            let packed_current_button_state =
                add_header_to_word(CURRENT_BUTTON_STATE_HEADER, current_button_state);
            let packed_encoder_p1_count =
                add_header_to_word(ENCODER_P1_COUNT_HEADER, encoders[0].count as u32);
            let packed_encoder_p2_count =
                add_header_to_word(ENCODER_P2_COUNT_HEADER, encoders[1].count as u32);
            let encoder_direction_bits = (encoders[0].direction as u32) & 0b11
                | ((encoders[1].direction as u32) & 0b11) << 2;
            let packed_encoder_direction =
                add_header_to_word(ENCODER_DIRECTION_HEADER, encoder_direction_bits);
            sio.fifo.write(packed_current_button_state);
            sio.fifo.write(packed_encoder_p1_count);
            sio.fifo.write(packed_encoder_p2_count);
            sio.fifo.write(packed_encoder_direction);
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
            let encoder_keys = get_encoder_keys(&encoders);

            let mut all_keys = [Keyboard::NoEventIndicated; NUM_BUTTONS + NUM_ENCODERS * 2];
            all_keys[..NUM_BUTTONS].copy_from_slice(&keys);
            all_keys[NUM_BUTTONS..].copy_from_slice(&encoder_keys);

            match keyboard.device().write_report(all_keys) {
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
/// both the keyboard buttons as well as the control center buttons, or anything else in the buttons array.
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

/// Reads raw quadrature counts from each encoder's PIO Rx FIFO, applying
/// the per-encoder debounce gate, and updates `encoders[i].count`.
fn read_encoder_fifos(
    encoders: &mut [EncoderState; NUM_ENCODERS],
    timer: &Timer<CopyableTimer0>,
    last_button_update_ticks: &mut u64,
) {
    for enc in encoders.iter_mut() {
        while !enc.rx.is_empty() {
            if let Some(value) = enc.rx.read() {
                let now = timer.get_counter().ticks();
                if now > (enc.last_update_ticks + enc.debounce_ticks) {
                    if enc.count != value as i32 {
                        enc.last_update_ticks = now;
                        enc.count = value as i32;
                        *last_button_update_ticks = now;
                    }
                }
            }
        }
    }
}

/// Updates encoder direction state using per-encoder step-threshold + move-timeout logic.
/// Called every core0 loop iteration after reading the PIO FIFOs.
fn update_encoders(encoders: &mut [EncoderState; NUM_ENCODERS], timer: &Timer<CopyableTimer0>) {
    let now = timer.get_counter().ticks();
    for enc in encoders.iter_mut() {
        let delta = enc.count - enc.anchor_count;

        if delta > enc.step_threshold {
            enc.anchor_count = enc.count;
            enc.direction = EncoderDirection::Positive;
            enc.last_move_ticks = now;
        } else if delta < -enc.step_threshold {
            enc.anchor_count = enc.count;
            enc.direction = EncoderDirection::Negative;
            enc.last_move_ticks = now;
        } else if now > (enc.last_move_ticks + enc.move_timeout_ticks) {
            enc.direction = EncoderDirection::Stopped;
        }
    }
}

/// Builds the encoder key report to send presses via USB by iterating the encoder array.
/// Each encoder occupies two slots: index `i*2` for the positive-direction key
/// and index `i*2+1` for the negative-direction key.
fn get_encoder_keys(encoders: &[EncoderState; NUM_ENCODERS]) -> [Keyboard; NUM_ENCODERS * 2] {
    let mut key_report = [Keyboard::NoEventIndicated; NUM_ENCODERS * 2];
    for (i, enc) in encoders.iter().enumerate() {
        let up_idx = i * 2;
        let down_idx = i * 2 + 1;
        match enc.direction {
            EncoderDirection::Positive => {
                if let Some(k) = enc.key_up {
                    key_report[up_idx] = k;
                }
            }
            EncoderDirection::Negative => {
                if let Some(k) = enc.key_down {
                    key_report[down_idx] = k;
                }
            }
            EncoderDirection::Stopped => {
                // Both slots stay NoEventIndicated → keys are released
            }
        }
    }
    key_report
}

/// Convert a u8 HID key code to `Option<Keyboard>`. A value of 0 means
/// `NoEventIndicated` (i.e. no key mapped).
pub fn u8_to_key(value: u8) -> Option<Keyboard> {
    if value == 0 {
        None
    } else {
        Some(Keyboard::from(value))
    }
}

/// This function will encode the current button state of all buttons in the button array that
/// have a key mapped via the NKRO USB peripheral into an array that can be sent via USB as a keypress.
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

/// Write the storage struct to flash.
/// FLASH_STORAGE_OFFSET must be 256-byte aligned for program, 4096 aligned for erase
/// Uses an atomic guard (`FLASH_WRITE_IN_PROGRESS`) to prevent concurrent
/// calls from both cores. If the guard is already claimed, the function logs
/// a warning and returns immediately without writing.
unsafe fn write_storage(storage: &FlashStoragePersistentMemory) {
    // Atomic guard: if someone else is already writing, skip.
    if FLASH_WRITE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        info!("write_storage: another write is already in progress, skipping.");
        return;
    }

    const FLASH_SECTOR_SIZE: u32 = 4096;
    const FLASH_PAGE_SIZE: u32 = 256;

    let struct_size = core::mem::size_of::<FlashStoragePersistentMemory>();
    let struct_ptr = storage as *const FlashStoragePersistentMemory as *const u8;

    // Step 1: Erase enough 4 KB sectors to cover the entire struct.
    // flash_range_erase(addr, count, block_size, block_cmd)
    // block_size=4096, block_cmd=0x20 (sector erase command for most NOR flashes)
    // Round struct_size up to the next 4 KB boundary.
    let erase_size =
        ((struct_size as u32 + (FLASH_SECTOR_SIZE - 1)) / FLASH_SECTOR_SIZE) * FLASH_SECTOR_SIZE;
    unsafe {
        hal::rom_data::flash_range_erase(
            FLASH_STORAGE_OFFSET,
            erase_size as usize,
            FLASH_SECTOR_SIZE,
            0x20,
        );
    }

    // Step 2: Program the struct data one 256-byte page at a time.
    // flash_range_program(addr, data_ptr, count) requires a 256-byte aligned
    // address and a count that is a multiple of 256.
    let mut remaining = struct_size;
    let mut src = struct_ptr;
    let mut flash_offs = FLASH_STORAGE_OFFSET;
    while remaining > 0 {
        let mut page_buf = [0xFFu8; 256];
        let chunk_len = core::cmp::min(remaining, FLASH_PAGE_SIZE as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(src, page_buf.as_mut_ptr(), chunk_len);
        }
        unsafe {
            hal::rom_data::flash_range_program(
                flash_offs,
                page_buf.as_ptr(),
                FLASH_PAGE_SIZE as usize,
            );
        }
        remaining -= chunk_len;
        src = unsafe { src.add(chunk_len) };
        flash_offs += FLASH_PAGE_SIZE;
    }

    // Step 3: Flush the XIP cache so subsequent XIP reads see the new data.
    unsafe {
        hal::rom_data::flash_flush_cache();
    }

    // Release the atomic guard
    FLASH_WRITE_IN_PROGRESS.store(false, Ordering::Release);
}

/// Erase the persistent storage region, restoring flash to an uninitialised state.
/// On the next boot the device will detect fresh storage and re-write the defaults.
///
/// The erase size is calculated from `core::mem::size_of_val(storage)` so it
/// automatically covers the full struct regardless of future growth.
#[allow(dead_code)]
unsafe fn clear_storage(storage: &FlashStoragePersistentMemory) {
    // Atomic guard: if someone else is already writing, skip.
    if FLASH_WRITE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        info!("clear_storage: another write is already in progress, skipping.");
        return;
    }

    const FLASH_SECTOR_SIZE: u32 = 4096;

    let struct_size = core::mem::size_of_val(storage);
    let erase_size =
        ((struct_size as u32 + (FLASH_SECTOR_SIZE - 1)) / FLASH_SECTOR_SIZE) * FLASH_SECTOR_SIZE;

    unsafe {
        hal::rom_data::flash_range_erase(
            FLASH_STORAGE_OFFSET,
            erase_size as usize,
            FLASH_SECTOR_SIZE,
            0x20,
        );
    }

    // Flush the XIP cache so subsequent reads see the erased (0xFF) state.
    unsafe {
        hal::rom_data::flash_flush_cache();
    }

    FLASH_WRITE_IN_PROGRESS.store(false, Ordering::Release);
    info!("Persistent storage cleared. Next boot will re-initialise defaults.");
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
