<!--
[post]
created_on="2026-7-9"
-->

# LastDraw: How to build a plotter

**LastDraw**, a play on the [NextDraw from bantam tools](https://bantamtools.com/collections/bantam-tools-nextdraw) is an updated version of a very commonly made plotter called [Drawing Robot](https://www.thingiverse.com/thing:2349232).  It aims at using modern hardware and firmware to drive the plotter. 

## Requirements 
### Equipment
- 3d Printer
- Soldering iron

### Part list
- TODO: x Amount of PLA

**Hardware**
- Nema 17 Stepper Motors https://www.amazon.com/Stepper-Motor-Bipolar-64oz-Printer/dp/B00PNEQI7W
- (2) Linear Rod M8 x 450mm, X Axis https://www.amazon.com/dp/B07DPHDMDT
- (2) Linear Rod M8 x 350mm, Y Axis https://www.amazon.com/dp/B07JKTLFD7
  (Note: If longer Linear Rods were are purchased, you may cut them to length)
- (2) Linear Rod 3mm, Z Axis (from old CDROM)
- (1) Threaded Rod M8 x 480mm
- (8) LM8UU Bearings https://www.amazon.com/uxcell-Bushing-Linear-Motion-Double/dp/B00X9H22SO
- (1) Servo Sg90 https://www.amazon.com/Upenten-Micro-Helicopter-Remote-Control/dp/B07KVJ84FS
- (2) Spring (from ball point pen)
- (2) GT2 Pulley, 16 teeth https://www.amazon.com/Anet-Timing-Pulley-Aluminum-Printer/dp/B07D294B2T
- (5) Bearing 624zz https://www.amazon.com/uxcell-Bearing-4x13x5mm-Shielded-Bearings/dp/B07PLC6GY3Parts
- List
- Components
- (1) 2000mm GT2 belt https://www.amazon.com/Mercurry-Meters-timing-Rostock-GT2-6mm/dp/B071K8HYB4

**Electronics**
- (1) Arduino Uno
- (kit with CNC and 4988)
- https://www.amazon.com/kuman-Shield-Expansion-Stepper-Driver/dp/B06XHKSVTG
- (1) Arduino CNC Shield
- (2) A4988 Stepper Drivers
- (6) Jumpers (The kit above is missing these jumpers)
- https://www.amazon.com/ZYAMY-2-54mm-Standard-Circuit-Connection/dp/B077957RN7
- (1) 12V 2A Power Supply
- https://www.amazon.com/Adapter-100-240V-Transformers-Switching-Adaptor/dp/B019Q3U72M
- (2) Limit switches (optional)
- https://www.amazon.com/URBESTAC-Momentary-Hinge-Roller-Switches/dp/B00MFRMFS6

**Nuts**
- (7) M3-0.5
- (5) M4-0.7 x 35mm
- (4) M8-18

**Screws**
- (13) Phillips M3-0.5 x 16mm
- (4) Phillips M3-0.5 x 6mm
- (5) M4-0.7 x 35mm
- (1) Hex M3-0.5 x 20mm

**Washers**
- (4) M3 washer
- (4) M8 washer

## Firmware

We're gonna use [FluidNC](http://wiki.fluidnc.com/en/home) which is popular [CNC](https://en.wikipedia.org/wiki/Computer_numerical_control) firmware. Its actively maintained and widely supported project. We're going to follow the [documented setup for the Makerbase MKS DLC32](http://wiki.fluidnc.com/en/hardware/3rd-party/MKS_DLC32).

## Wiring

> ![WARNING]
> Don't plug or unplug the motor and driver to avoid malfuncation

See the [DLC32 manifacture's wiring manual](./last_draw/dlc32_wiring_manual.pdf) for more information.
