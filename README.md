#kobovnc

I am heavily assisted by AI. 

There is now touch input, scaling, padding, panning, and a way to quit the program via long tapping the screen and releasing. Power button and sleep cover can be used to exit as well, provided you have killed nickel beforehand. Using ClaudeAI I have obtained a custom high speed encoding, see my other repositories.

https://github.com/noobyme/libvncserver-1-bit-encoding/releases/tag/debug
https://github.com/noobyme/tightvnc-kobo-encoding/releases/tag/debug
https://github.com/noobyme/droidVNC-NG-Floydsteinberg-encoding/releases/tag/Kobo

This tool has been confirmed to work on Nia. It should work on the latest devices too

VNC server tested successfuly with TightVNC, x11vnc, DroidVNC NG.

## Warning

The screen can refresh up to 30 times per second, this will degrade the eInk display rapidly.
Do not use with fast changing content like videos.

**It is possible that it will damage yours.**
*I cannot be held responsible, use this tool at your own risk.*

## Installing

I have provided a KoboRoot.tgz for the GUI version which contains plato stuff, simply drop it into the hidden .kobo folder when you plug in your usb, you will need to install NickelMenu to launch it. If you prefer placing it somewhere else you can use the zip file. You can use this tool by connecting to the eInk device through SSH, or using NickelMenu

https://www.mobileread.com/forums/showthread.php?t=254214 This is NiLuJe's SSH

https://github.com/pgaskin/NickelMenu

https://github.com/baskerville/plato

Both are installed the same way, dropping the KoboRoot.tgz into the .kobo folder, which is a hidden file, you need to enable viewing them.
NickelMenu entries placed inside .adds/nm/nickelmenu.txt folder. 1. kills all programs, launches VNC session then restarts Kobo's UI using plato(youd need to have it installed), 2. launches gui, the launch script handles restarting Kobo UI

```
menu_item:main:VNCNoGui:nickel_wifi:enable 
chain_success:cmd_spawn:quiet:cd /mnt/onboard/.adds/plato-0.9.45/; killall -TERM nickel hindenburg sickel fickel adobehost foxitpdf iink dhcpcd-dbus dhcpcd fmon; /mnt/onboard/kobovncnogui 192.168.1.150 5900 --password password; /mnt/onboard/.adds/plato-0.9.45/nickel.sh

menu_item:main:VNCGui:cmd_spawn:quiet:exec /mnt/onboard/.adds/kobovncgui/launchvnc.sh; #/mnt/onboard/.adds/kobovncgui/nickel.sh

menu_item:main:ToggleForceWifi:nickel_setting:toggle:force_wifi
```
Edit the launchvnc file to include default parameters like host and password. I havent figured out why Wifi sometimes doesnt work yet, it seems if you have wifi enabled and attempt to enable it again it produces problems so the solution seems to be only enable it if you havent already done so. It may be worth trying MickelMenu's toggle force wifi

## Usage

To connect to a VNC server:
Replace kobovnc with whatever your file is called, or, rename the file to kobovnc. /mnt/onboard/ is the location of your kobo when you connect using usb, '.' in ./mnt/onboard/ means what the current directory is, if your current working directory is /mnt/onboard/ youd call the program by ./kobovnc 
``` shell
/mnt/onboard/kobovnc [OPTIONS]
```
For example:

``` shell
/mnt/onboard/kobovnc --host 192.168.2.1 --port 5900 --password abcde123 --contrast 2
./mnt/onboard/kobovnc --host 192.168.2.1 --port 5900 --password abcde123 --contrast 2
./kobovnc --host 192.168.2.1 --port 5900 --password abcde123 --contrast 2 
```

Available options:
- Host
- Port
- Username
- Password
- Contrast: apply a post processing contrast filter
- White_cutoff: apply a post processing filter to turn colors greater than the specified value to white (255
- Exclusive: request a non-shared session
- Rotate:0-3 the default upright orientation is different for each device
- Scale: fit to width or height
- Longtap: Send right click for windows server by pressing and holding, android and linux servers seem to automatically implement this so no need
- pan: Enable panning instead of click drag
- disable_touch : Disable touch input during vnc session

Advanced users:

- partial_update: Choose 1=Fast/A2 2=Fastmono/A2 3=Gui/DU 4=Partial/GL16 5=Full/GC16. Testing on Kobo Nia with:DroidVNC without blue_noise turned on only mode 2 works and you end up with undithered 1 bit colour image, TightVNC both mode 1 and 2 work without the need for blue_noise but heavy detail loss and ghosting. With blue_noise turned on detail loss is less and ghosting seems better too, I think this is due to improved partial update but im not sure, but I thought a2 mode didnt differentiate between full and partial so im not sure. a2 mode does seem to improve cursor trails but only marginally. blue_noise slows down the frame rate though so its better to apply 1 bit dithering server side using custom encoding
- full_update: Choose 1=Fast 2=Fastmono 3=Gui 4=Partial 5=Full
- set_dither:Dithers 16 level grayscale. The input color is in {0 .. 255}. The output color is in G16. Grayscale 16
- set_monochrome:unsure exactly wat it do, plato function. sets monochrome flag for epd ioctl?
- refresh:how often to do full refresh, units is how many rects before full refresh
- fps: Decimal value, 30.0 or 0.5 etc
- blue_noise: For A2/DU mode, use client side blue noise dithering to produce 1 bit grayscale
- encoding: Request custom high speed encoding from server, best used with A2 or DU waveform

For faster framerates, use USB networking (see https://www.mobileread.com/forums/showthread.php?t=254214). This isnt present on some devices, like Nia.

Example of using ssh tunneling, you need NiLuJe's KoboStuff. To automate this youd need to set up ssh keys, 
running ssh-keygen on Kobo and then copying .pub key to server. 
https://www.mobileread.com/forums/showthread.php?t=254214
```
ip link set lo up
ip addr add 127.0.0.1/8 dev lo
ifconfig lo 127.0.0.1 up
ssh -L 15900:127.0.0.1:5900 -N user@vncserverip
ssh -L 15900:127.0.0.1:5900 -N user@vncserverip& # for automation with key based login
kobovnc --host 127.0.0.1 --port 15900 #Open new shell for this command
```

~~Use a resolution smaller than or exactly equal to your display. eg common resolution of 1024x768 will fail to work correctly on Kobo Nia because 1024x758 is the maximum. Custom resolution of 1024x758 works!~~

To stop all other programs use one of these two commands before launching kobo-vnc, so you can use touch input. From koreader/plato startup script

```
killall -q -TERM nickel hindenburg sickel fickel strickel fontickel adobehost foxitpdf iink dhcpcd-dbus dhcpcd bluealsa bluetoothd fmon nanoclock.lua
killall -TERM nickel hindenburg sickel fickel adobehost foxitpdf iink dhcpcd-dbus dhcpcd fmon
```
These commands can kill Kobo UI(Nickel), launch a program, then restart nickel from ssh when youre finished.
```
eval "$(tr '\0' '\n' < /proc/$(pidof -s nickel)/environ | grep -E '^(USER|SHLVL|LD_LIBRARY_PATH|HOME|TERM|PATH|LANG|SHELL|PWD|DBUS_SESSION_BUS_ADDRESS|NICKEL_HOME|PLATFORM|PRODUCT|WIFI_MODULE|INTERFACE|UBOOT_MMC|UBOOT_RECOVERY|runlevel|prevlevel|boot_port|waveform_p|waveform_sz|hwcfg_p|hwcfg_sz|ntxfw_p|ntxfw_sz|QT_GSTREAMER_PLAYBIN_AUDIOSINK|QT_GSTREAMER_PLAYBIN_AUDIOSINK_DEVICE_PARAMETER|LIBC_FATAL_STDERR_)=' | sed 's/^/export /')"
killall -TERM nickel hindenburg sickel fickel adobehost foxitpdf iink dhcpcd-dbus dhcpcd fmon
cd /mnt/onboard/.adds/plato-0.9.45
/mnt/onboard/.adds/plato-0.9.45/plato.sh #launch here
/mnt/onboard/.adds/plato-0.9.45/nickel.sh
```

Failed to fill whole buffer error? You messed up somewhere in login credentials or server side ip blocking. 

## Derivatives

The code responsible for rendering to the eInk display is written by baskerville and taken from https://github.com/baskerville/plato.
The code responsible for communicating using the VNC protocol is written by whitequark and taken from https://github.com/whitequark/rust-vnc.


Changelog:

- I have fixed ZRLE DroidVNC NG issues using Claude AI
  - zrle.rs 
	- adding (false, 17..=127) to zrle decode fn,
	- changing copy_indexed fn to return result,
	- remove '&& format.depth <= 24' condition in decode fn for 32bpp, as droid vnc server depth is 32 not 24, other servers have depth of 24 not 	  32. Forcing 8bpp allowed the oldest kobovnc to ignore this.
  - main.rs:forcing the colour format to 8bpp, as the oldest version did, call vnc.request_update only at end of each frame, instead of each event, apparently this combined with debug mode can cause server to close connection.
- Grayscale conversion using RGB instead of R
- I have changed rotation default to use plato's function
- I have added a contingency device detection using /mnt/onboard/.kobo/version instead of using environment variables because starting the tool over ssh does not pass those values. 
- I have also setup touchscreen functionality from plato and rustvnc.
- I have copied over the ClaraColor and LibraColor device.rs from plato's latest version, 
- added scaling, padding, added gesture swiping recognition from plato
- If killed nickel beforehand program will be able to read power button events, power button or sleep cover will quit vnc and restart nickel, unless you started from ssh in which case it will not be able to restart nickel. Hold touchscreen for more than 6 seconds to exit without restarting nickel regardless of ssh start or not.
- copied latest framebuffer code from plato, probably wasnt necessary, could have added small changes to add mark 12 code without needing to copy over all other rest of files but i have no device to test it with
- add panning for resolutions bigger than device can handle
- minor other changes
- Notes for documentation, copyrect is never sent by server, rect sizes are always under 100 pixels regardless of screen resolution except for raw encoding, so a full screen rect is never sent. End of frame happens much more often when no screen change and contains more than 1 temporal frame. Even if you request specific region, it will send pixels outside of that region due to rects lying partially inside/outside. Crashing was occuring because rects under threshold were updated immediately per rect in old update code. when no change occurs EOF is sent but no rects are. Nia framebuffer is 768, means x offset is 10, but device actual is 758

## Compilation instructions

To compile gui libaries compile plato, I simply reused plato.

To compile on wsl ubuntu noble 24.04, x86_64 CPU
Go to linux user home directory, Clone repository, Download linaro cross toolchain file (the toolchain itself will do no need for sys root file). We want gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf.tar.xzn Extract toolchain. Make cargo directory and config file. Add repositories and architecture for armv7, install arm libraries, copy libraries into toolchain directory. Install rustup and target. Build. 

```
cd /home/noobyme
git clone https://github.com/everydayanchovies/eink-vnc
wget https://releases.linaro.org/components/toolchain/binaries/4.9-2017.01/arm-linux-gnueabihf/gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf.tar.xz
tar -xf gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf.tar.xz
cd /eink-vnc/client/
mkdir .cargo
nano .cargo/config.toml
```

[target.armv7-unknown-linux-gnueabihf]
linker = "/home/noobyme/gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc"

```
sudo dpkg --add-architecture armhf
sudo add-apt-repository multiverse
sudo add-apt-repository universe
sudo nano /etc/apt/sources.list.d/ubuntu.sources
```

Types: deb
URIs: http://archive.ubuntu.com/ubuntu/
Suites: noble noble-updates noble-backports
Components: main universe restricted multiverse
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: http://security.ubuntu.com/ubuntu/
Suites: noble-security
Components: main universe restricted multiverse
Architectures: amd64
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

Types: deb
URIs: http://ports.ubuntu.com/ubuntu-ports/
Suites: noble noble-updates noble-backports noble-security
Components: main restricted universe multiverse
Architectures: armhf
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg

```
sudo apt update
sudo apt-get install libz-dev:armhf libbz2-dev:armhf libjpeg-dev:armhf libpng-dev:armhf libgumbo-dev:armhf libopenjp2-7-dev:armhf libjbig2dec-dev:armhf

cd /usr/lib/arm-linux-gnueabihf
cp libz.* libbz2.* libjpeg.* libpng16.* libgumbo.* libopenjp2.* libjbig2dec.* /home/noobyme/gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf/arm-linux-gnueabihf/libc/lib

sudo apt-get install rustup
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target armv7-unknown-linux-gnueabihf
```

Compiling using docker, instructions from chatgpt
Note:Copy and pasted commands from AI can fail because formatting differences
Ubuntu 24.04, non WSL

```
sudo apt update
sudo apt upgrade -y
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common lsb-release gnupg
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo
"deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg]
https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" |
sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -aG docker $USER (then log out and log in)
docker --version
docker run hello-world
docker pull ewpratten/kobo-cross-armhf:latest
sudo apt-get install cargo
sudo apt-get install rustup
rustup default stable
cargo install cross
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
cd /eink-vnc/client
cross build --target arm-unknown-linux-musleabihf --release
```
https://www.mobileread.com/forums/showthread.php?t=348481&page=2 Thanks elinkser/szybet for the toolchain info.

Compiling on Windows

Download gcc-linaro-4.9.4-2017.01-i686-mingw32_arm-linux-gnueabihf, extract it. Change config.toml file. Get rustup

Create .cargo/config.toml
```
[target.armv7-unknown-linux-gnueabihf]
#linker = "/home/jerry/gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc"
linker = "D:/gcc-linaro-4.9.4-2017.01-i686-mingw32_arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc.exe"
```
rustup target add armv7-unknown-linux-gnueabihf
cargo build --target armv7-unknown-linux-gnueabihf

rustup target add armv7-unknown-linux-gnueabihf

Compiling on rustrover Windows:
Change build.rs
Download gcc-linaro-4.9.4-2017.01-i686-mingw32_arm-linux-gnueabihf, extract it. 
```
use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    // Cross-compiling for Kobo.
    if target == "arm-unknown-linux-gnueabihf" {
        println!("cargo:rustc-env=PKG_CONFIG_ALLOW_CROSS=1");
        println!("cargo:rustc-link-search=libs");
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=jpeg");
        println!("cargo:rustc-link-lib=png16");
        println!("cargo:rustc-link-lib=gumbo");
        println!("cargo:rustc-link-lib=openjp2");
        println!("cargo:rustc-link-lib=jbig2dec");
    // Handle the Linux and macOS platforms.
    } else {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

        match target_os.as_ref() {
            "linux" => {
                println!("cargo:rustc-link-lib=dylib=stdc++");
            }
            "macos" => {
                println!("cargo:rustc-link-lib=dylib=c++");
            }
            "windows" => {
                // println!("cargo:rustc-link-lib=dylib=stdc++");
            }
            _ => panic!("Unsupported platform: {}.", target_os),
        }
    }
}
```
Change .cargo/config.toml to explicitly set target:
```
[target.armv7-unknown-linux-gnueabihf]
#linker = "/home/menooby/gcc-linaro-4.9.4-2017.01-x86_64_arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc"
linker = "D:/gcc-linaro-4.9.4-2017.01-i686-mingw32_arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc.exe"
[build]
target = "armv7-unknown-linux-gnueabihf"
```
