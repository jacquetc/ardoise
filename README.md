# Ardoise: e-paper/e-ink monitor

## Goal:

Create a e-paper/e-ink monitorfor linux. My personal use is a typewriter.
Written in Rust

Latency is 0,2s when the zone to update is small (ex: characters) , sufficient to create a typewriter.

Mouse cursor isn't displayed

There is no e-Paper update if there is no movement

I confirm this is working fine, see the photo in resources/

![example](resources/e-paper.JPG)

## Idea behind

- Screenshoot a linux desktop
- Compare with previous screenshoot
- convert in greyscale colors the modified zone
- Update only the modified zone of e-Paper
- Repeat...

## Prerequisites:

- Raspeberry Pi 4 (not 3 ! Display isn't big enough)
- Usb-c 5V power
- Micro SD Card (8 Go minimum)
- 1872×1404, 10.3inch flexible E-Ink display HAT for Raspberry Pi found [here](https://www.waveshare.com/product/displays/e-paper/epaper-1/10.3inch-e-paper-hat-d.htm)

## Steps to install:

### Install Raspbian

Install official Raspbian distribution for Raspberry Pi 4:

And connect it to wifi.


### BMC2835 library
Download the latest version of the library, say bcm2835-1.xx.tar.gz [bcm2835](https://www.airspayce.com/mikem/bcm2835/)

```bash
tar zxvf bcm2835-1.xx.tar.gz
cd bcm2835-1.xx
./configure
make
sudo make check
sudo make install
```

### Build Ardoise

Install Rust:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

open a new terminal or reboot

in ardoise dir:

```
cargo build --release
sudo copy target/release/ardoise /usr/local/bin/
```

### Load at startup

copy resources/.xprofile in /home/pi/

Not sure if this works:

```
sudo copy resources/ardoise.service /etc/systemd/system/
```

But I'm sure this works:

```
sudo copy resources/launch_ardoise.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/launch_ardoise.sh
```

And make it load at startup in LXDE settings.

### Tip for writers:

Install FocusWriter, make it load at startup in LXDE settings.

Enjoy !
