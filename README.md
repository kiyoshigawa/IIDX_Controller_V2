# IIDX Controller V2

This is the software for a new controller board for my IIDX deck. Original build log is located here:

https://twa.ninja/blog/iidx_deck_-_build_log_-_part_1/

https://twa.ninja/blog/iidx_deck_-_build_log_-_part_2/

https://twa.ninja/blog/iidx_deck_-_build_log_-_part_3/

https://twa.ninja/blog/iidx_deck_-_build_log_-_part_4/

Original controller code for Teensy 3.1 located here:

https://github.com/kiyoshigawa/IIDX_Deck/tree/master/IIDX_Controller

## Project overview:

This is a rust program designed for the [Waveshare Core 2350B](https://www.waveshare.com/wiki/Core2350B0) board to control my Beatmania IIDX custom controller. It's designed to be a drop-in replacement for the original hand-soldered controller build using a Teensy 3.1 and the Arduino IDE. The controller will interface via USB and output as an NKRO HID keyboard. It will use my LED lighting controller code for lighting effects, and will also be able to take audio signals as inputs for lighting effects. I will try to keep this feature document up to date as I work through implementing everything.

## Feature List:

- [x] Must Enumerate as an NKRO Keyboard via USB
- [ ] Will accept Line-In Audio Signal for DSP Audio
- [x] Uses new rust lighting controller library
- [x] New PCB to accept all existing controller wiring
	- [x] Buttons w/ Responsive Lighting
	- [x] Both Encoders
	- [x] Lighting Power Control Relay Button
	- [x] Lighting Configuration Mode Button
	- [x] JTAG spring pin header for programming/debug
- [x] Add additional system control buttons with an OLED screen display to center panel
	- [x] Adjustable settings for gameplay in addition to lighting controller control
		- [x] Change encoder step thresholds ~~live~~ and save locally
		- [x] debounce time adjustments
		- [x] keybindings for all buttons
		- [x] Lighting controller config
	- [ ] Audio DSP adjustments in menus?

## Software:

- [x] Use RP2350B MCU with 48 IO pins to allow for maximum versatility
	- https://files.waveshare.com/wiki/Core2350B0/Core2350B.pdf
	- https://www.waveshare.com/wiki/Core2350B0#Pinout_Definition
	- I/O Requirements:
  	- 18 I/O Pins for buttons
  		- 14 key buttons + start/select buttons for each player
  	- 5 Pins for system control buttons
  		- Up, Down, Left, Right, and Select for menu navigation.
  	- 1 I/O Pin for WS2812B LED Strip Control
  	- 4 pins for encoders for wiki wikis, 2 pins each needed for quadrature encoding
  	- 6 I/O Pins Optional LED Screen Data Pins
  		- Screen uses 2 pins for I2C
      - I will leave a bank of SPI pins available for future peripherals
  	- 2 Analog Pins for Stereo Audio Input for DSP
  	- 9 currently unused Pins to be available for additional features
    - Pin 47 is used by the PSRAM chip on the board, so it is unavailable.
- [x] Dual-Cores Can be used to Prioritize Input over Cosmetic Features
	- [x] Main Core (High Priority, Low Latency Tasks):
		- [x] NKRO Keyboard and USB Bus
		- [x] Encoder Position Tracking
		- [x] encoder logic to decide when to send and release keypresses based on count changes
	- [ ] Second Core (lower priority cosmetic only features)
		- [x] System Control menu handling / screen updates
		- [x] Lighting Controller Updates
		- [ ] Audio DSP and FFT analysis
		- [x] Data through sio.fifo register from first core:
  		- [x] Button states, current and previous
      - [x] both encoder counts
      - [x] sio data unreliable getting 0s randomly
        - [x] Need a way to confirm data is valid. Options:
          - Use top 5 bits of each word to encode data type
- [x] Button Processing
	- [x] https://github.com/dlkj/usbd-human-interface-device <- Tested and working
	- [x] Send each button press via USB NKRO library at sample rate.
		- [x] 1000Hz works
	- [x] Planning to do button via polling, not interrupts.
	- [x] Individual debounce timers for each button
	- [x] USB updates sent to match the sample rate
	- [x] Buttons use internal pull-down resistors to allow for external transistor control of LED lighting.
	- [x] handle control center buttons with menu system
- [x] Encoder processing
	- [x] I used the asm from the [adamgreen github](https://github.com/adamgreen/QuadratureDecoder/blob/master/QuadratureDecoder.pio) example for PIO encoders
  	- [x] Need to verify the code is actually working and get the encoder counts out of the fifo buffer.
    - [x] implement encoder debounce - const for now, adjust later
	- [x] Revisit encoder position to input press logic from old controller to ensure it is working as intended
	- [x] Need to figure out how to send keyboard signals based on encoder position changes using the NKRO USB HID library
  	- [x] SIO FIFO messages work mostly, but I am getting some rogue 0s displayed. Need to investigate.
  - [x] add idle timeout to reset to 0 rotation if unused for a while. Like > 1hr
- [x] Lighting Controller [Github repo](https://github.com/kiyoshigawa/lighting_controller)
  - [x] WS2812 led strip controller:
  	- [x] https://github.com/rp-rs/ws2812-pio-rs ? Needs testing
      - Made my own modified version based on this library, also used DSP for non-blocking sends.
  - [x] Configure existing lighting modes that will actually be used to be accessible in control menus
    - [x] slow rotate rainbow
    - [x] fixed rotate rainbow tied to encoder position
    - [x] button-trigger-fired events over any background, configurable
  - [ ] New lighting mode for VU-Meter
		- [ ] Starts at base, and pulses up around the wikis depending on DSP input levels
		- [ ] Rainbow decides colors for volume levels
		- [ ] will require continuous input from DSP data via triggers noting current volume level
  - [ ] New lighting mode(s) for frequency responsive pulsing
    - [ ] Fixed position frequency regions with rainbow-aligned colors that pulse based on DSP input
			- [ ] We'll need to send frequency range data to the controller via triggers continuously
		- [ ] Same as fixed, but it can rotate
			- [ ] Constant rotation rate
			- [ ] wiki-aligned rotation
- [ ] System Control Functionality
	- [x] New control method (Better, but more work before it's ready):
		- [x] Add dedicated control buttons and screen indicator
			- [x] https://docs.rs/ssd1306/latest/ssd1306/index.html <- tested and working
		- [x] will require redoing the center panel on the case to incorporate the new parts/buttons
		- [x] Needs feature partiy with old config functions:
  		- [x] Change rainbows
      - [x] Change lighting modes
      - [x] New has all lighting paramaters full exposed and editable
    - [x] New planned features:
      - [x] debug input display
        - [x] shows what buttons are being pressed live
        - [x] encoder position counter values
        - [x] encoder key send indicators (shows up send or down send when 'key' is pressed for wikis)
      - [x] OLED pixel test (blank screen)
      - [x] permanent storage of settings changes in flash
        - [x] must be persistent after power cycle/reprogramming.
      - [x] key mapping per-button
      - [x] timing adjustments
        - [x] debounce for each button
        - [x] wiki steps/timeouts
        - [x] lighting mode durations
      - [x] Needs a working menu system allowing access to the features above.
      - [ ] idle animations to show on oled when not in menus
      - [ ] Add 'save slots' and preset lighting patterns to make it easier to cycle through changes
	- [ ] ~~Old method (was originally the MVP, Skipped straight to feature creep, lol)~~:
		- [ ] ~~Single button switches to control mode~~
  	- [ ] ~~The gameplay buttons then change things when pressed~~

### Hardware:

- [x] New Manufactured PCB, no longer hand-soldered on breadboards - Ordered PCBs 2026-03-10
	- [x] 0.2" o.c. Screw terminals for all existing wiring in case. I have these in abundance currently
		- [x] Buttons 
		- [x] Button LED Lighting to sink current when button is pressed, or find new way to handle that.
  		- [x] Using discrete transistors to handle current form button LEDs. See circuit schematic. Had to switch to pull-downs for buttons.
      - [x] Due to a hardware bug that causes latching when using oull-down resistors on GPIO pins, I had to hack in that the GPIO pins are not set as inputs until just before I read and set back just after. Google RP2350-E9 Errata for more info.
		- [x] ~~LED Lighting for Power Relay Button, and also power relay control for board~~
  		- Not needed, using old power board
		- [x] Encoder Connections
		- [x] LED Strip Connection
		- [x] System Control buttons/screen connection points
		- [x] Bonus volume, mute, and escape buttons
		- [x] 5 spare buttons with lighting circuit
	- [x] Socket footprint for waveshare RP2350B footprint
	- [x] Needs 10-pin 0.5mm header and 6-pin pogo pin JTAG footprints for easy programming/debug
	- [x] Mounting holes planned to work with IIDX deck case
	- [x] Figure out how to handle the flex cable USB connector
		- [x] ~~Option 1: use the flex cable as the USB input and mount it in the case somewhere~~
		- [x] Add a new USB connector using the U+/U- pins and mount that instead.
	- [x] Fix backwards transistors on PCB for every light
- [ ] Separate circuit board for line-in audio to limit output to protect ADC on RP2350B:
	- [ ] Design circuit for DSP Inputs
		- [ ] Handle stereo signal from barrel jack input(s)
		- [ ] Have a limiter circuit that will clip at 3.3V to protect PI pin inputs
		- [ ] Use voltage offset and/or half-wave-invert line level to allow capture by analog input pin
			- [ ] Could also use an audio chip with I2S or similar if circuit testing goes poorly
## Known issues to fix:
- [x] USB stops working when macbook goes to sleep, which prevents main loop from functioning at all until I reconnect with probe.
  - [x] Need to find a way to cleanly and automatically resume USB when the computer wakes up.
  - [x] It looks like the main loop breaks completely when this happens, no heartbeat LED
  - [x] encoder and button data should still make it to core 2 for lighting upates without USB
  - [x] Determined this was just the probe-rs connection closing tht caused the loop to crash. Expected behavior as above when computer sleeps without probe-rs attached.
- [x] core messages aren't consistent as written, sometimes get 0 values sent over for encoder
  - [x] Investigate what else might be using sio fifo buffer - Nothing found.
  - [x] Find a way to ensure the fifo data is tagged so I know what data is being sent?
    - [x] Ended up adding a 5-bit header to the data sent so it will know what data each word read is. Haven't seen any glitches since.

## License info:

This project contains code under multiple licenses:

- Original template code from rp-rs/rp235x-project-template:
  Dual-licensed under MIT OR Apache-2.0
- New code and modifications by [Your Name]:
  Licensed under GPLv3
