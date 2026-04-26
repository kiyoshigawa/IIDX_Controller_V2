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
- [ ] Uses new rust lighting controller library
- [ ] New PCB to accept all existing controller wiring
	- [ ] Buttons w/ Responsive Lighting
	- [ ] Both Encoders
	- [ ] Lighting Power Control Relay Button
	- [ ] Lighting Configuration Mode Button
	- [ ] JTAG spring pin header for programming/debug
- [ ] Add additional system control buttons with an OLED screen display to center panel
	- [ ] Adjustable settings for gameplay in addition to lighting controller control?
		- [ ] Change encoder step thresholds live and save locally
		- [ ] debounce time adjustments
		- [ ] Audio DSP adjustments

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
	- Main Core (High Priority, Low Latency Tasks):
		- NKRO Keyboard and USB Bus
		- Encoder Position Tracking
	- Second Core (lower priority cosmetic only features)
		- System Control menu handling / screen updates
		- Lighting Controller Updates
		- Audio DSP and FFT analysis
- [ ] Button Processing
	- [x] https://github.com/dlkj/usbd-human-interface-device <- Tested and working
	- [ ] Send each button press via USB NKRO library at sample rate.
		- [ ] 500Hz sample rate min, see if 1000Hz works
	- [ ] Planning to do button via polling, not interrupts.
	- [ ] Individual debounce timers for each button
	- [ ] USB updates sent to match the sample rate
	- [ ] Separate button polling loop for core 1 menu buttons. Core 0 only gets gameplay buttons.
- [ ] Encoder processing
	- [x] I used the asm from the [adamgreen github](https://github.com/adamgreen/QuadratureDecoder/blob/master/QuadratureDecoder.pio) example for PIO encoders
  	- [ ] Need to verify the code is actually working and get the encoder counts out of the fifo buffer.
		- [ ] might switch from an interrupt or polling the PIO FIFO buffer to dual DMA peripherals (if they're not getting used by something else)
	- [ ] Revisit encoder position to input press logic from old controller to ensure it is working as intended
	- [ ] Need to figure out how to send keyboard signals based on encoder position changes using the NKRO USB HID library
- [ ] Lighting Controller [Github repo](https://github.com/kiyoshigawa/lighting_controller)
  - [ ] WS2812 led strip controller:
  	- [ ] https://github.com/rp-rs/ws2812-pio-rs ? Needs testing
  - [ ] Configure lighting modes that will actually be used to be accessible in control menus
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
	- [ ] Features:
		- [ ] Change Rainbows
		- [ ] Change Lighting Modes
		- [ ] Adjust debounce time
		- [ ] Adjust Sample time
		- [ ] Adjust encoder position thresholds
		- [ ] change key bindings?
		- [ ] display debug data for things like encoder positions, button presses, DSP analysis, etc.?
	- [ ] Old method (MVP):
		- [ ] Single button switches to control mode
		- [ ] The gameplay buttons then change things when pressed
	- [ ] New method (Better, but more work before it's ready):
		- [ ] Add dedicated control buttons and screen indicator
			- [x] https://docs.rs/ssd1306/latest/ssd1306/index.html <- tested and working
		- [ ] will require redoing the center panel on the case to incorporate the new parts/buttons

### Hardware:

- [ ] New Manufactured PCB, no longer hand-soldered on breadboards
	- [ ] 0.2" o.c. Screw terminals for all existing wiring in case. I have these in abundance currently
		- [ ] Buttons 
		- [ ] Button LED Lighting to sink current when button is pressed, or find new way to handle that.
		- [ ] LED Lighting for Power Relay Button, and also power relay control for board
		- [ ] Encoder Connections
		- [ ] LED Strip Connection
		- [ ] System Control buttons/screen connection points
	- [x] Socket footprint for waveshare RP2350B footprint
	- [ ] Design circuit for DSP Inputs
		- [ ] Handle stereo signal from barrel jack input(s)
		- [ ] Have a limiter circuit that will clip at 3.3V to protect PI pin inputs
		- [ ] Use voltage offset and/or half-wave-invert line level to allow capture by analog input pin
			- [ ] Could also use an audio chip with I2S or similar if circuit testing goes poorly
	- [ ] Needs 10-pin 0.5mm header and 6-pin pogo pin JTAG footprints for easy programming/debug
	- [ ] Mounting holes planned to work with IIDX deck case
	- [ ] Figure out how to handle the flex cable USB connector
		- [ ] Option 1: use the flex cable as the USB input and mount it in the case somewhere
		- [ ] Option 2: Add a new USB connector using the U+/U- pins and mount that instead.
	- [ ]


## License info:

This project contains code under multiple licenses:

- Original template code from rp-rs/rp235x-project-template:
  Dual-licensed under MIT OR Apache-2.0
- New code and modifications by [Your Name]:
  Licensed under GPLv3
