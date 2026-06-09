extern crate alloc;
extern crate byteorder;
extern crate flate2;
#[macro_use]
extern crate log;

mod device;
mod framebuffer;

#[macro_use]
mod geom;
mod color;
mod gesture;
mod input;
mod security;
mod settings;
mod unit;
mod vnc;

mod font;
mod view;
mod context;
mod helpers;
mod document;
mod event_handling;
mod scaling;

use crate::framebuffer::transform::transform_dither_g2;
use crate::framebuffer::{Framebuffer, KoboFramebuffer1, KoboFramebuffer2, Pixmap, UpdateMode};
use crate::geom::{Dir, Rectangle};
use crate::vnc::{client, Client, Encoding, Rect};
use clap::{value_t, App, Arg};
use input::{
    button_scheme_event, device_events, display_rotate_event, raw_events, usb_events, ButtonCode,
    ButtonStatus, DeviceEvent, FingerStatus,
};
use log::{debug, error, info};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use vnc::PixelFormat;

use crate::color::{Color, WHITE};
use crate::device::CURRENT_DEVICE;
use crate::gesture::*;
use std::fs::File;
use std::io::Read;
use std::mem;
use std::path::Path;
use std::process::Command;
use std::slice;
use std::sync::mpsc;

use crate::context::Context;
use crate::font::Fonts;
use crate::geom::{DiagDir, Region};
use crate::gesture::{gesture_events, GestureEvent};
use crate::helpers::{load_toml, save_toml};
use crate::scaling::scale_parameters;
use crate::settings::{ButtonScheme, RotationLock, Settings, SETTINGS_PATH, };
use crate::view::common::{locate, locate_by_id, overlapping_rectangle};
use crate::view::common::{toggle_input_history_menu, toggle_keyboard_layout_menu};
use crate::view::gui::Gui;
use crate::view::input_field::InputField;
use crate::view::menu::{Menu, MenuKind};
use crate::view::{gui, EntryId, EntryKind, Event, RenderData, RenderQueue, UpdateData, View, ViewId};
use crate::view::{handle_event, process_render_queue, wait_for_all};
use anyhow::{format_err, Context as ResultExt, Error};
use chrono::Local;
use std::collections::VecDeque;
use std::env;

const FB_DEVICE: &str = "/dev/fb0";

const TOUCH_INPUTS: [&str; 5] = [
    "/dev/input/by-path/platform-2-0010-event",
    "/dev/input/by-path/platform-1-0038-event",
    "/dev/input/by-path/platform-1-0010-event",
    "/dev/input/by-path/platform-0-0010-event",
    "/dev/input/event1",
];

const BUTTON_INPUTS: [&str; 4] = [
    "/dev/input/by-path/platform-gpio-keys-event",
    "/dev/input/by-path/platform-ntx_event0-event",
    "/dev/input/by-path/platform-mxckpd-event",
    "/dev/input/event0",
];
const POWER_INPUTS: [&str; 3] = [
    "/dev/input/by-path/platform-bd71828-pwrkey.6.auto-event",
    "/dev/input/by-path/platform-bd71828-pwrkey.4.auto-event",
    "/dev/input/by-path/platform-bd71828-pwrkey-event",
];

#[repr(align(256))]
pub struct PostProcBin {
    data: [u8; 256],
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let matches = App::new("einkvnc")
        .about("VNC client")
        .arg(
            Arg::new("host")
                .help("server hostname or IP")
                .long("host")
                .takes_value(true)
        )
        .arg(
            Arg::new("port")
                .help("server port (default: 5900)")
                .long("port")
                .takes_value(true)
        )
        .arg(
            Arg::new("username")
                .help("server username")
                .long("username")
                .takes_value(true),
        )
        .arg(
            Arg::new("password")
                .help("server password")
                .long("password")
                .takes_value(true),
        )
        .arg(
            Arg::new("exclusive")
                .help("request a non-shared session")
                .long("exclusive"),
        )
        .arg(
            Arg::new("contrast_exp")
                .help("apply a post processing contrast filter")
                .long("contrast")
                .takes_value(true),
        )
        .arg(
            Arg::new("gray")
                .help("the gray point of the post processing contrast filter")
                .long("graypoint")
                .takes_value(true),
        )
        .arg(
            Arg::new("white")
                .help("apply a post processing filter to turn colors greater than the specified value to white (255)")
                .long("whitecutoff")
                .takes_value(true),
        )
        .arg(
            Arg::new("rotation")
                .help("rotation (1-4), tested on a Clara HD, try at own risk")
                .long("rotate")
                .takes_value(true),
        )
        .arg(
            Arg::new("scale")
                .help("fit to height or width")
                .long("scale"),
        )
        .arg(
            Arg::new("long_tap")
                .help("long tap to send right click, for pc servers. not necessary for touchscreen servers or linux servers")
                .long("long_tap"),
        )
        .arg(
            Arg::new("fu")
                .help("Choose 1=Fast 2=Fastmono 3=Gui 4=Partial 5=Full")
                .long("full_update")
                .takes_value(true),
        )
        .arg(
            Arg::new("pu")
                .help("Choose 1=Fast 2=Fastmono 3=Gui 4=Partial 5=Full")
                .long("partial_update")
                .takes_value(true),
        )
        .arg(
            Arg::new("sd")
                .help("true or false")
                .long("set_dither")
                .takes_value(true),
        )
        .arg(
            Arg::new("sm")
                .help("true or false")
                .long("set_monochrome")
                .takes_value(true),
        )
        .arg(
            Arg::new("fr")
                .help("Choose how often to full refresh")
                .long("full_refresh")
                .takes_value(true),
        )
        .arg(
            Arg::new("fps")
                .help("Choose how often to request update")
                .long("fps")
                .takes_value(true),
        )
        .arg(
            Arg::new("bn")
                .help("Blue noise dithering for 1bit output")
                .long("blue_noise")
                .short('b')
        )
        .arg(
            Arg::new("pan")
                .help("Swipe to pan instead of swipe to drag")
                .long("pan")
                .short('p')
        )
        .arg(
            Arg::new("disable_touch")
                .help("Disable touch input")
                .long("disable_touch")
        )
        .arg(
            Arg::new("gui")
                .help("launch gui")
                .long("gui")
                .short('g')
        )
        .arg(
            Arg::new("enc")
                .help("Choose custom 1bpp encoding")
                .long("encoding")
                .short('e')
        )
        .arg(
            Arg::new("cf")
                .help("Choose 1= RGB222 2=RGBA2222 3=RGB332 4=RGB565 5=RGB888 6=RGBA8888")
                .long("colour_format")
                .short('f')
                .takes_value(true),
        )
        .arg(
            Arg::new("irs")
                .help("swap red and blue index")
                .long("invert_red_shift")
        )
        .arg(
            Arg::new("disable_exit_via_hold")
                .help("disable exit via holding the screen")
                .long("disable_exit_via_hold")
        )
        .arg(
            Arg::new("exit_duration")
                .help("how long to hold screen for before quitting")
                .long("exit_duration")
                .takes_value(true),
        )
        .get_matches();

    // let mut host = matches.value_of("host");
    let mut host: Option<String> = matches.value_of("host").map(str::to_string);
    let mut port = value_t!(matches.value_of("port"), u16).unwrap_or(5900);     // String
    let mut username = matches.value_of("username").map(str::to_string);      // Option<String>
    let mut password = matches.value_of("password").map(str::to_string);
    let mut contrast_exp = value_t!(matches.value_of("contrast_exp"), f32).unwrap_or(1.0);
    let mut contrast_gray_point = value_t!(matches.value_of("gray"), f32).unwrap_or(224.0);
    let mut white_cutoff = value_t!(matches.value_of("white"), u8).unwrap_or(255);
    let mut exclusive = matches.is_present("exclusive");
    let mut rotate = value_t!(matches.value_of("rotation"), i8).unwrap_or(CURRENT_DEVICE.startup_rotation());
    let mut scale = matches.is_present("scale");
    let mut long_tap = matches.is_present("long_tap");
    let mut full_update = value_t!(matches.value_of("fu"), i8).unwrap_or(5);
    let mut partial_update = value_t!(matches.value_of("pu"), i8).unwrap_or(4);
    let mut refresh = value_t!(matches.value_of("fr"), u32).unwrap_or(500);
    let mut fps = value_t!(matches.value_of("fps"), f32).unwrap_or(30.0);
    let invert_red_shift = matches.is_present("irs");

    let blue_noise = matches.is_present("bn");
    let mut panning = matches.is_present("pan");
    let mut disable_touch = matches.is_present("disable_touch");
    let colour_format = value_t!(matches.value_of("cf"), u8).unwrap_or(0);

    let mut gui_enabled = matches.is_present("gui");
    let mut gui_active = gui_enabled.clone();
    let mut encoding = matches.is_present("enc");

    let set_dither = value_t!(matches.value_of("sd"), bool).unwrap_or(false);
    let set_monochrome = value_t!(matches.value_of("sm"), bool).unwrap_or(false);

    let disable_exit_via_hold = matches.is_present("disable_exit_via_hold");
    let exit_duration = value_t!(matches.value_of("exit_duration"), u32).unwrap_or(6);

    // std::process::Command::new("scripts/wifi-enable.sh").spawn().ok();

    let mut fb_red_index = 0;
    #[cfg(feature = "eink_device")]
    let mut fb: Box<dyn Framebuffer> = if CURRENT_DEVICE.mark() != 8 {
        let raw_fb = KoboFramebuffer1::new(FB_DEVICE)
            .context("can't create framebuffer")
            .unwrap();
        fb_red_index = if raw_fb.var_info.red.offset > 0 && !invert_red_shift { 2 } else { 0 };
        Box::new(raw_fb)
    } else {
        let raw_fb = KoboFramebuffer2::new(FB_DEVICE)
            .context("can't create framebuffer")
            .unwrap();
        fb_red_index = if raw_fb.var_info.red.offset > 0 && !invert_red_shift { 2 } else { 0 };
        Box::new(raw_fb)
    };

    let RGB222: PixelFormat = PixelFormat {
        bits_per_pixel: 8,
        depth: 6,
        big_endian: false,
        true_colour: true,
        red_max: 3,
        green_max: 3,
        blue_max: 3,
        red_shift: if fb_red_index == 0 { 0 } else { 4 },
        green_shift: 2,
        blue_shift: if fb_red_index == 0 { 4 } else { 0 },
    };
    let RGBA2222: PixelFormat = PixelFormat {
        bits_per_pixel: 8,
        depth: 8,
        big_endian: false,
        true_colour: true,
        red_max: 3,
        green_max: 3,
        blue_max: 3,
        red_shift: if fb_red_index == 0 { 0 } else { 4 },
        green_shift: 2,
        blue_shift: if fb_red_index == 0 { 4 } else { 0 },
    };
    let RGB332: PixelFormat = PixelFormat {
        bits_per_pixel: 8,
        depth: 8,
        big_endian: false,
        true_colour: true,
        red_max: 7,
        green_max: 7,
        blue_max: 3,
        red_shift: if fb_red_index == 0 { 0 } else { 6 },
        green_shift: 3,
        blue_shift: if fb_red_index == 0 { 6 } else { 0 },
    };
    let RGB565: PixelFormat = PixelFormat {
        bits_per_pixel: 16,
        depth: 16,
        big_endian: false,
        true_colour: true,
        red_max: 31,
        green_max: 63,
        blue_max: 31,
        red_shift: if fb_red_index == 0 { 0 } else { 11 },
        green_shift: 5,
        blue_shift: if fb_red_index == 0 { 11 } else { 0 },
    };
    let RGB888: PixelFormat = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian: false,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: if fb_red_index == 0 { 0 } else { 16 },
        green_shift: 8,
        blue_shift: if fb_red_index == 0 { 16 } else { 0 },
    };
    let RGBA8888: PixelFormat = PixelFormat {
        bits_per_pixel: 32,
        depth: 32,
        big_endian: false,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: if fb_red_index == 0 { 0 } else { 16 },
        green_shift: 8,
        blue_shift: if fb_red_index == 0 { 16 } else { 0 },
    };

    let mut SD_COLOR_FORMAT: PixelFormat = PixelFormat {
        bits_per_pixel: 8,
        depth: 6,
        big_endian: false,
        true_colour: true,
        red_max: 3,
        green_max: 3,
        blue_max: 3,
        red_shift:  if fb_red_index == 0 { 0 } else { 4 },
        green_shift: 2,
        blue_shift:  if fb_red_index == 0 { 4 } else { 0 },
    };

    match colour_format {
        0 => {}
        1 => SD_COLOR_FORMAT = RGB222,
        2 => SD_COLOR_FORMAT = RGBA2222,
        3 => SD_COLOR_FORMAT = RGB332,
        4 => SD_COLOR_FORMAT = RGB565,
        5 => SD_COLOR_FORMAT = RGB888,
        6 => SD_COLOR_FORMAT = RGBA8888,
        _ => {}
    };

    let mut stream;
    let mut vnc: Option<Client> = None;;

    let mut width = 0;
    let mut height = 0;
    let mut vnc_format;

    if !gui_enabled {
        info!("connecting to {}:{}", host.as_deref().unwrap_or(""), port);
        stream = match std::net::TcpStream::connect((host.as_deref().unwrap_or(""), port)) {
            Ok(stream) => stream,
            Err(error) => {
                error!("cannot connect to {}:{}: {}", host.as_deref().unwrap_or(""), port, error);
                std::process::exit(1)
            }
        };

        vnc = Some(match Client::from_tcp_stream(stream, !exclusive, |methods| {
            debug!("available authentication methods: {:?}", methods);
            for method in methods {
                match method {
                    client::AuthMethod::None => return Some(client::AuthChoice::None),
                    client::AuthMethod::Password => {
                        return match password {
                            None => None,
                            Some(ref password) => {
                                let mut key = [0; 8];
                                for (i, byte) in password.bytes().enumerate() {
                                    if i == 8 {
                                        break;
                                    }
                                    key[i] = byte
                                }
                                Some(client::AuthChoice::Password(key))
                            }
                        }
                    }
                    client::AuthMethod::AppleRemoteDesktop => match (username.as_deref(), password.as_deref()) {
                        (Some(username), Some(password)) => {
                            return Some(client::AuthChoice::AppleRemoteDesktop(
                                username.to_owned(),
                                password.to_owned(),
                            ))
                        }
                        _ => (),
                    },
                }
            }
            None
        }) {
            Ok(vnc) => vnc,
            Err(error) => {
                error!("cannot initialize VNC session: {}", error);
                std::process::exit(1)
            }
        });

        (width, height) = vnc.as_mut().unwrap().size();
        info!(
            "connected to \"{}\", {}x{} framebuffer",
            vnc.as_mut().unwrap().name(),
            width,
            height
        );
        vnc_format = vnc.as_mut().unwrap().format();
        info!("received {:?}", vnc_format);
        vnc.as_mut().unwrap().set_format(SD_COLOR_FORMAT).unwrap();
        info!("request {:?}", SD_COLOR_FORMAT);
        vnc_format = vnc.as_mut().unwrap().format();
        info!("received {:?}", vnc_format);

        if encoding {
            vnc.as_mut().unwrap().set_encodings(&[Encoding::RfbEncodingMono1bppZlib]).unwrap()
        } else {
            vnc.as_mut().unwrap().set_encodings(&[Encoding::Zrle]).unwrap()
        }

        if scale {
            if vnc.as_mut().unwrap()
                .request_update(
                    Rect {
                        left: 0,
                        top: 0,
                        width,
                        height,
                    },
                    false,
                )
                .is_err()
            {
                error!("server disconnected");
            }
        } else {
            if vnc.as_mut().unwrap()
                .request_update(
                    Rect {
                        left: 0,
                        top: 0,
                        width: if width < fb.width() as u16 {
                            width
                        } else {
                            fb.width() as u16
                        },
                        height: if height < fb.height() as u16 {
                            height
                        } else {
                            fb.height() as u16
                        },
                    },
                    false,
                )
                .is_err()
            {
                error!("server disconnected");
            }
        }
    } else {

    };


    #[cfg(feature = "eink_device")]
    info!(
        "running on device model=\"{}\" /dpi={} /dims={}x{}",
        CURRENT_DEVICE.model, CURRENT_DEVICE.dpi, CURRENT_DEVICE.dims.0, CURRENT_DEVICE.dims.1
    );

    #[cfg(feature = "eink_device")]
    {
        let startup_rotation = rotate;
        fb.set_rotation(startup_rotation).ok();
    }

    let post_proc_bin = PostProcBin {
        data: (0..=255)
            .map(|i| {
                if contrast_exp == 1.0 {
                    i
                } else {
                    let gray = contrast_gray_point;

                    let rem_gray = 255.0 - gray;
                    let inv_exponent = 1.0 / contrast_exp;

                    let raw_color = i as f32;
                    if raw_color < gray {
                        (gray * (raw_color / gray).powf(contrast_exp)) as u8
                    } else if raw_color > gray {
                        (gray + rem_gray * ((raw_color - gray) / rem_gray).powf(inv_exponent)) as u8
                    } else {
                        gray as u8
                    }
                }
            })
            .map(|i| -> u8 {
                if i > white_cutoff {
                    255
                } else {
                    i
                }
            })
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap(),
    };

    //const FRAME_MS: u64 = 1000 / 30;
    let FRAME_MS: u64 = (1000.0 / (fps as f64)) as u64;

    //const max_dirty_refreshes: usize = 500;
    let max_dirty_refreshes: usize = refresh as usize;

    let mut dirty_rects: Vec<Rectangle> = Vec::new();
    let mut dirty_rects_since_refresh: Vec<Rectangle> = Vec::new();
    let mut has_drawn_once = false;
    let mut dirty_update_count = 0;

    let mut time_at_last_draw = Instant::now();

    let mut paths = Vec::new();
    for ti in &TOUCH_INPUTS {
        if Path::new(ti).exists() {
            paths.push(ti.to_string());
            break;
        }
    }
    for bi in &BUTTON_INPUTS {
        if Path::new(bi).exists() {
            paths.push(bi.to_string());
            break;
        }
    }
    for pi in &POWER_INPUTS {
        if Path::new(pi).exists() {
            paths.push(pi.to_string());
            break;
        }
    }
    fn build_context(fb: Box<dyn Framebuffer>) -> Result<Context, Error> {
        let path = Path::new(SETTINGS_PATH);
        let mut settings = if path.exists() {
            load_toml::<Settings, _>(path).context("can't load settings")?
        } else {
            Default::default()
        };

        let fonts = Fonts::load().context("can't load fonts")?;

        Ok(Context::new(fb, settings, fonts))
    }

    let mut fb_gui: Box<dyn Framebuffer> = if CURRENT_DEVICE.mark() != 8 {
        Box::new(KoboFramebuffer1::new(FB_DEVICE).context("can't create framebuffer")?)
    } else {
        Box::new(KoboFramebuffer2::new(FB_DEVICE).context("can't create framebuffer")?)
    };//new fb
    // let startup_rotation = rotate;
    // fb_gui.set_rotation(startup_rotation).ok();
    let mut context = build_context(fb_gui).context("can't build context")?; //new context?

    // println!("{:?}",paths);
    let (raw_sender, raw_receiver) = raw_events(paths);
    let touch_screen = gesture_events(device_events(raw_receiver, context.display, context.settings.button_scheme));
    //let usb_port = usb_events();

    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();

    thread::spawn(move || {
        while let Ok(evt) = touch_screen.recv() {
            tx2.send(evt).ok();
        }
    });

    // let mut fit_width: bool = false;
    // let mut fit_height: bool = false;
    let mut scale_factor: f32 = 1.0;
    //dbg!(fb.width(),width,fb.height(),height);

    let mut x_padding = 0;
    let mut y_padding = 0;

    let mut x_offset: u32 = 0;
    let mut y_offset: u32 = 0;

    let mut device_fb_rect= rect!(0,0,0,0);
    let mut _cropped_vnc_fb_rect = rect!(0,0,0,0);
    let mut _original_vnc_fb_rect= rect!(0,0,0,0);
    let mut scaled_fb_rect= rect!(0,0,0,0);

    if !gui_enabled {
        device_fb_rect = rect![0, 0, fb.width() as i32, fb.height() as i32];
        _cropped_vnc_fb_rect = rect![
            0 + x_padding as i32,
            0 + y_padding as i32,
            fb.width() as i32 + x_padding as i32,
            fb.height() as i32 + y_padding as i32
        ];
        _original_vnc_fb_rect = rect![0, 0, width as i32, height as i32];
        scaled_fb_rect = rect![
            0 + x_padding as i32,
            0 + y_padding as i32,
            width as i32 + x_padding as i32,
            height as i32 + y_padding as i32
        ];

        if scale {
            let scale_parameters = scale_parameters::new(true, width, height, fb.width(), fb.height(), x_offset, y_offset);
            scale_factor = scale_parameters.scale_factor;
            x_padding = scale_parameters.x_padding;
            y_padding = scale_parameters.y_padding;
            device_fb_rect = scale_parameters.device_fb_rect;
            _cropped_vnc_fb_rect = scale_parameters.cropped_vnc_fb_rect;
            _original_vnc_fb_rect = scale_parameters.original_vnc_fb_rect;
            scaled_fb_rect = scale_parameters.scaled_fb_rect;
        } else {
            let scale_parameters = scale_parameters::new(false, width, height, fb.width(), fb.height(), x_offset, y_offset);
            scale_factor = scale_parameters.scale_factor;
            x_padding = scale_parameters.x_padding;
            y_padding = scale_parameters.y_padding;
            // device_fb_rect = scale_parameters.device_fb_rect;
            // cropped_vnc_fb_rect = scale_parameters.cropped_vnc_fb_rect;
            // original_vnc_fb_rect = scale_parameters.original_vnc_fb_rect;
            // scaled_fb_rect = scale_parameters.scaled_fb_rect;
        }
        fb.draw_rectangle(&device_fb_rect, WHITE);
        fb.update(&device_fb_rect, UpdateMode::Full).ok(); //when gui is not enabled device fb rect is 0,0,0,0 causing crash

    }

    //dbg!(fb.width(),width,fb.height(),height,(width as f32*scale_factor) as i32,(height as f32*scale_factor) as i32);

    let mut full_update_mode = match full_update {
        1 => UpdateMode::Fast,     //a2
        2 => UpdateMode::FastMono, //a2
        3 => UpdateMode::Gui,      //gc16 full
        4 => UpdateMode::Partial,  //gc16 hybrid
        5 => UpdateMode::Full,
        _ => UpdateMode::Full, //fast and fastmono are the same...
    };
    let mut partial_update_mode = match partial_update {
        1 => UpdateMode::Fast,     //a2
        2 => UpdateMode::FastMono, //a2
        3 => UpdateMode::Gui,      //gc16 full
        4 => UpdateMode::Partial,  //gc16 hybrid
        5 => UpdateMode::Full,
        _ => UpdateMode::Partial, //fast and fastmono are the same...
    };
    match set_dither {
        true => fb.set_dithered(true),
        false => fb.set_dithered(false),
    };
    match set_monochrome {
        true => fb.set_monochrome(true),
        false => fb.set_monochrome(false),
    };

    // Build once at startup — 2KB, permanently L1-resident
    static EXPAND_1BPP: [[u8; 8]; 256] = {
        let mut table = [[0u8; 8]; 256];
        let mut byte = 0usize;
        while byte < 256 {
            let mut bit = 0;
            while bit < 8 {
                table[byte][bit] = if (byte >> (7 - bit)) & 1 == 1 { 0xFF } else { 0x00 };
                bit += 1;
            }
            byte += 1;
        }
        table
    };


    let mut finger_down_count = Instant::now();
    let finger_seconds = Duration::from_secs(2);

    'running: loop {
        let time_at_sol = Instant::now();
        debug!("Loop start {:?}", time_at_sol);
        //dbg!(left_x_truncate,right_x_truncate,top_y_truncate,bottom_y_truncate);
        'gui: while gui_active {

            let mut inactive_since = Instant::now();

            context.load_keyboard_layouts(); //load keyboard, require this one

            let mut rq = RenderQueue::new();

            let mut updating = Vec::new();
            let current_dir = env::current_dir()?;

            println!("The framebuffer resolution is {} by {}.", context.fb.rect().width(),
                     context.fb.rect().height());

            let mut bus = VecDeque::with_capacity(4);

            let mut view: Box<dyn View> = Box::new(Gui::new(context.fb.rect(), &tx, &mut rq,
                                                            &host.as_deref(), &port, &username, &password,
                                                            encoding, scale,
                                                            &mut context, panning, disable_touch, long_tap));
            process_render_queue(view.as_ref(), &mut rq, &mut context, &mut updating);

            while let Ok(evt) = rx.recv() {
                match evt {
                    Event::ToggleNear(ViewId::KeyboardLayoutMenu, rect) => {
                        toggle_keyboard_layout_menu(view.as_mut(), rect, Some(true), &mut rq, &mut context);
                    },
                    Event::Close(id) => {
                        if let Some(index) = locate_by_id(view.as_ref(), id) {
                            let rect = overlapping_rectangle(view.child(index));
                            rq.add(RenderData::expose(rect, UpdateMode::Gui));
                            view.children_mut().remove(index);
                            // view.toggle_keyboard(false, None, hub, rq, context);
                        }
                    },
                    Event::ToggleInputHistoryMenu(id, rect) => {
                        toggle_input_history_menu(view.as_mut(), id, rect, None, &mut rq, &mut context);
                    },
                    Event::Submit(ViewId::GuiInputField1, ref text) => {
                        if !text.is_empty() {
                            // view.toggle_keyboard(false, None, hub, rq, context);
                            // self.define(Some(text), rq, context);
                            host = Some(text.clone());
                        }
                    },
                    Event::Submit(ViewId::GuiInputField2, ref text) => {
                        if !text.is_empty() {
                            // view.toggle_keyboard(false, None, hub, rq, context);
                            // self.define(Some(text), rq, context);
                            port = text.parse::<u16>().unwrap();
                        }
                    },
                    Event::Submit(ViewId::GuiInputField3, ref text) => {
                        if !text.is_empty() {
                            // view.toggle_keyboard(false, None, hub, rq, context);
                            // self.define(Some(text), rq, context);
                            username = Option::from(text.clone());
                        }
                    },
                    Event::Submit(ViewId::GuiPasswordField1, ref text) => {
                        if !text.is_empty() {
                            // view.toggle_keyboard(false, None, hub, rq, context);
                            // self.define(Some(text), rq, context);
                            password = Option::from(text.clone());
                        }
                    },
                    Event::Select(EntryId::Portrait) => {
                        let n = CURRENT_DEVICE.startup_rotation();
                        if let Ok(dims) = context.fb.set_rotation(n) {
                            raw_sender.send(display_rotate_event(n)).ok();
                            context.display.rotation = n;
                            let fb_rect = Rectangle::from(dims);
                            if context.display.dims != dims {
                                context.display.dims = dims;
                            }
                        }
                        view = Box::new(Gui::new(context.fb.rect(), &tx, &mut rq,
                                                 &host.as_deref(), &port, &username, &password,
                                                 encoding, scale, &mut context, panning, disable_touch, long_tap));
                    },
                    Event::Select(EntryId::IPortrait) => {
                        let n = CURRENT_DEVICE.startup_rotation() - 2;
                        if let Ok(dims) = context.fb.set_rotation(n) {
                            raw_sender.send(display_rotate_event(n)).ok();
                            context.display.rotation = n;
                            let fb_rect = Rectangle::from(dims);
                            if context.display.dims != dims {
                                context.display.dims = dims;
                            }
                        }
                        view = Box::new(Gui::new(context.fb.rect(), &tx, &mut rq,
                                                 &host.as_deref(), &port, &username, &password,
                                                 encoding, scale,  &mut context, panning, disable_touch, long_tap));
                    },
                    Event::Select(EntryId::Landscape) => {
                        let n = CURRENT_DEVICE.startup_rotation() - 1;
                        if let Ok(dims) = context.fb.set_rotation(n) {
                            raw_sender.send(display_rotate_event(n)).ok();
                            context.display.rotation = n;
                            let fb_rect = Rectangle::from(dims);
                            if context.display.dims != dims {
                                context.display.dims = dims;
                            }
                        }
                        view = Box::new(Gui::new(context.fb.rect(), &tx, &mut rq,
                                                 &host.as_deref(), &port, &username, &password,
                                                 encoding, scale, &mut context, panning, disable_touch, long_tap));
                    },
                    Event::Select(EntryId::ILandscape) => {
                        let n = CURRENT_DEVICE.startup_rotation() - 3;
                        if let Ok(dims) = context.fb.set_rotation(n) {
                            raw_sender.send(display_rotate_event(n)).ok();
                            context.display.rotation = n;
                            let fb_rect = Rectangle::from(dims);
                            if context.display.dims != dims {
                                context.display.dims = dims;
                            }
                        }
                        view = Box::new(Gui::new(context.fb.rect(), &tx, &mut rq,
                                                 &host.as_deref(), &port, &username, &password,
                                                 encoding, scale, &mut context, panning, disable_touch, long_tap));
                    },
                    Event::Toggle(ViewId::Encoding) => {
                        // println!("{}", encoding);
                        encoding = !encoding;
                    },
                    Event::Toggle(ViewId::Scaling) => {
                        // println!("{}", scale);
                        scale = !scale;
                    },
                    Event::Toggle(ViewId::Wifi) => {
                        // std::process::Command::new("scripts/wifi-enable.sh").output().ok();
                        std::process::Command::new("scripts/wifi-enable.sh").spawn().ok();
                    },
                    Event::Toggle(ViewId::Long_Tap) => {
                        // println!("{}", long_tap);
                        long_tap = !long_tap;
                    },
                    Event::Select(EntryId::FastA2) => {
                        partial_update_mode = UpdateMode::Fast
                    },
                    Event::Select(EntryId::FastMonoA2) => {
                        partial_update_mode = UpdateMode::FastMono
                    },
                    Event::Select(EntryId::GuiDU) => {
                        partial_update_mode = UpdateMode::Gui
                    },
                    Event::Select(EntryId::PartialGL16) => {
                        partial_update_mode = UpdateMode::Partial
                    },
                    Event::Toggle(ViewId::Touch) => {
                        // println!("{}", disable_touch);
                        disable_touch = !disable_touch
                    },
                    Event::Toggle(ViewId::Panning) => {
                        // println!("{}", panning);
                        panning = !panning
                    },
                    Event::Back => {
                        break 'running
                    },
                    Event::Toggle(ViewId::VNC) => {
                        stream = match std::net::TcpStream::connect((host.as_deref().unwrap_or(""), port)) {
                            Ok(stream) => stream,
                            Err(error) => {
                                error!("cannot connect to {}:{}: {}", host.as_deref().unwrap_or(""), port, error);
                                tx.send(Event::Console(error.to_string())).ok();
                                continue;
                                // std::process::exit(1)
                            }
                        };

                        match Client::from_tcp_stream(stream, !exclusive, |methods| {
                            debug!("available authentication methods: {:?}", methods);
                            for method in methods {
                                match method {
                                    client::AuthMethod::None => return Some(client::AuthChoice::None),
                                    client::AuthMethod::Password => {
                                        return match password {
                                            None => None,
                                            Some(ref password) => {
                                                let mut key = [0; 8];
                                                for (i, byte) in password.bytes().enumerate() {
                                                    if i == 8 {
                                                        break;
                                                    }
                                                    key[i] = byte
                                                }
                                                Some(client::AuthChoice::Password(key))
                                            }
                                        }
                                    }
                                    client::AuthMethod::AppleRemoteDesktop => match (username.as_deref(), password.as_deref()) {
                                        (Some(username), Some(password)) => {
                                            return Some(client::AuthChoice::AppleRemoteDesktop(
                                                username.to_owned(),
                                                password.to_owned(),
                                            ))
                                        }
                                        _ => (),
                                    },
                                }
                            }
                            None
                        }) {
                            Ok(mut new_vnc) => {
                                (width, height) = new_vnc.size();
                                info!(
                                    "connected to \"{}\", {}x{} framebuffer",
                                   new_vnc.name(),
                                    width,
                                    height
                                );
                                vnc_format =new_vnc.format();
                                info!("received {:?}", vnc_format);
                               new_vnc.set_format(SD_COLOR_FORMAT).unwrap();
                                info!("request {:?}", SD_COLOR_FORMAT);
                                vnc_format =new_vnc.format();
                                info!("received {:?}", vnc_format);

                                if encoding {
                                   new_vnc.set_encodings(&[Encoding::RfbEncodingMono1bpp]).unwrap()
                                } else {
                                   new_vnc.set_encodings(&[Encoding::Zrle]).unwrap()
                                }

                                fb.set_rotation(context.display.rotation).ok();

                                if scale {
                                    if new_vnc
                                        .request_update(
                                            Rect {
                                                left: 0,
                                                top: 0,
                                                width,
                                                height,
                                            },
                                            false,
                                        )
                                        .is_err()
                                    {
                                        error!("server disconnected");
                                    }
                                } else {
                                    if new_vnc
                                        .request_update(
                                            Rect {
                                                left: 0,
                                                top: 0,
                                                width: if width < fb.width() as u16 {
                                                    width
                                                } else {
                                                    fb.width() as u16
                                                },
                                                height: if height < fb.height() as u16 {
                                                    height
                                                } else {
                                                    fb.height() as u16
                                                },
                                            },
                                            false,
                                        )
                                        .is_err()
                                    {
                                        error!("server disconnected");
                                    }
                                }
                                if scale {
                                    let scale_parameters = scale_parameters::new(true, width, height, fb.width(), fb.height() ,x_offset ,y_offset);
                                    scale_factor = scale_parameters.scale_factor;
                                    x_padding = scale_parameters.x_padding;
                                    y_padding = scale_parameters.y_padding;
                                    device_fb_rect = scale_parameters.device_fb_rect;
                                    _cropped_vnc_fb_rect = scale_parameters.cropped_vnc_fb_rect;
                                    _original_vnc_fb_rect = scale_parameters.original_vnc_fb_rect;
                                    scaled_fb_rect = scale_parameters.scaled_fb_rect;
                                } else {
                                    let scale_parameters = scale_parameters::new(false, width, height, fb.width(), fb.height() ,x_offset ,y_offset);
                                    scale_factor = scale_parameters.scale_factor;
                                    x_padding = scale_parameters.x_padding;
                                    y_padding = scale_parameters.y_padding;
                                    device_fb_rect = scale_parameters.device_fb_rect;
                                    _cropped_vnc_fb_rect = scale_parameters.cropped_vnc_fb_rect;
                                    _original_vnc_fb_rect = scale_parameters.original_vnc_fb_rect;
                                    scaled_fb_rect = scale_parameters.scaled_fb_rect;
                                }
                                gui_active = false;
                                // println!("gui");
                                vnc = Some(new_vnc);

                                fb.draw_rectangle(&device_fb_rect, WHITE);
                                fb.update(&device_fb_rect, UpdateMode::Full).ok();
                                break 'gui;
                            },
                            Err(error) => {
                                // println!("guifail");
                                error!("cannot initialize VNC session: {}", error);
                                // std::process::exit(1)
                                tx.send(Event::Console(error.to_string())).ok();
                                continue
                            }
                        };
                        // if vnc.is_some() {break 'gui} else {}
                    },
                    _ => {
                        handle_event(view.as_mut(), &evt, &tx, &mut bus, &mut rq, &mut context);
                    },
                }
                process_render_queue(view.as_ref(), &mut rq, &mut context, &mut updating);
                while let Some(ce) = bus.pop_front() {
                    tx.send(ce).ok();
                }
            }
        };
        // println!("mainloop");

        let event_params = event_handling::event_params::handle_events(&rx, scale_factor, width, height,
                                                                       fb.width(), fb.height(), x_padding, y_padding, x_offset, y_offset,
                                                                       &mut vnc, finger_down_count, finger_seconds, &mut fb,
                                                                       panning, has_drawn_once, scale, long_tap, gui_enabled, disable_touch,
                                                                       disable_exit_via_hold, exit_duration);
        has_drawn_once = event_params.has_drawn_once;
        finger_down_count =  event_params.finger_down_count;
        if event_params.exit_to_nickel {
            fb.set_rotation(CURRENT_DEVICE.startup_rotation()).ok();
            break 'running
        };
        if event_params.exit_to_gui {
            gui_active = true
        }

        x_offset = event_params.x_offset;
        y_offset = event_params.y_offset; //if its the same, it will be returned un changed, if changed return changed

        let mut frame_complete = false;
        let current_format = vnc.as_mut().unwrap().format();

        for event in vnc.as_mut().unwrap().poll_iter() {
            use client::Event;
            // dbg!(&event);
            match event {
                Event::Disconnected(None) => break 'running,
                Event::Disconnected(Some(error)) => {
                    error!("server disconnected: {:?}", error);
                    break 'running;
                }
                Event::PutPixels(vnc_rect, ref pixels) => {
                    let elapsed_ms = time_at_sol.elapsed().as_millis();
                    debug!("PutPixels, since loop {}, rect {}x{} at X{} Y{}"
                        , elapsed_ms, vnc_rect.width, vnc_rect.height,
                    vnc_rect.left, vnc_rect.top);

                    let mut counter_time = Instant::now();

                    let bpp = current_format.bits_per_pixel as usize / 8;

                    if scale {

                        let scaled_l = (vnc_rect.left as f32 * scale_factor).round() as u32;
                        let scaled_t = (vnc_rect.top as f32 * scale_factor).round() as u32;
                        let scaled_r =
                            ((vnc_rect.left + vnc_rect.width) as f32 * scale_factor).round() as u32;
                        let scaled_b =
                            ((vnc_rect.top + vnc_rect.height) as f32 * scale_factor).round() as u32;

                        let scaled_rect_width = scaled_r - scaled_l;
                        let scaled_rect_height = scaled_b - scaled_t;

                        if scaled_rect_width == 0 || scaled_rect_height == 0 {
                            continue;
                        }

                        let src_x: Vec<u32> = (0..scaled_rect_width)
                            .map(|x_out| ((x_out as f32 / scale_factor).round() as u32)
                                .clamp(0, vnc_rect.width as u32 - 1))
                            .collect();
                        let src_y: Vec<u32> = (0..scaled_rect_height)
                            .map(|y_out| ((y_out as f32 / scale_factor).round() as u32)
                                .clamp(0, vnc_rect.height as u32 - 1))
                            .collect();

                        // if false {
                        if encoding {

                            let row_bytes = (vnc_rect.width as usize + 7) / 8;
                            let mut row_buf = [0u8; 2048];

                            for y_out in 0..scaled_rect_height {
                                let local_y = src_y[y_out as usize];
                                // let original_y = ((y_out as f32) / scale_factor);
                                // let local_y = (original_y.round() as u32)
                                //     .clamp(0, (vnc_rect.height - 1) as u32);

                                for x_out in 0..scaled_rect_width as usize {
                                    let local_x = src_x[x_out] as usize;
                                    // let original_x = ((x_out as f32) / scale_factor);
                                    // let local_x = (original_x.round() as u32)
                                    //     .clamp(0, (vnc_rect.width - 1) as u32) as usize;

                                    let byte_idx = local_y as usize * row_bytes + local_x / 8;
                                    let bit_pos = 7 - (local_x % 8);
                                    let bit = (pixels[byte_idx] >> bit_pos) & 1;
                                    row_buf[x_out] = if bit == 1 { 0xFF } else { 0x00 };
                                }

                                fb.write_row_1bpp(
                                    scaled_l + x_padding,
                                    scaled_t + y_out + y_padding,
                                    &row_buf[..scaled_rect_width as usize],
                                );
                            }

                            // println!("Above continue");
                            // continue 'event
                        } else {
                            for y_out in 0..scaled_rect_height {
                                // let original_y = ((y_out as f32) / scale_factor);
                                // let local_y = (original_y.round() as u32)
                                //     .clamp(0, (vnc_rect.height - 1) as u32);

                                let local_y = src_y[y_out as usize];

                                for x_out in 0..scaled_rect_width {
                                    // let original_x = ((x_out as f32) / scale_factor);
                                    // let local_x = (original_x.round() as u32)
                                    //     .clamp(0, (vnc_rect.width - 1) as u32);

                                    let local_x = src_x[x_out as usize];
                                    // sample pixel...
                                    let src_idx = (local_y * vnc_rect.width as u32 + local_x) as usize;

                                    let mut luma = 0;
                                    let r;
                                    let g;
                                    let b;

                                    if !encoding && src_idx * bpp > pixels.len() {
                                        //dbg!(src_idx*bpp,pixels.len());
                                        //pixels is collection of bytes. u8. 4 bytes is 1 pixel.
                                        //oldest forced 8bits per pixel means 1 byte 1 is 1 pixel, if step by 4 then
                                        //only samplying every 4th pixel? if 8 bit forced would it still be vec of u8/ wouldnt it be vec of u2? 2 bits?
                                        //u8 used bc cpu is byte addressable, smallest unit
                                        dbg!(src_idx * bpp > pixels.len());
                                        continue
                                    } else {
                                        // if encoding {
                                        //     if src_idx * 1/8 > pixels.len() {
                                        //         continue
                                        //     }
                                        //     let row_bytes = (vnc_rect.width as usize + 7) / 8;
                                        //
                                        //     let byte_index =
                                        //         (local_y as usize * row_bytes) + (local_x as usize / 8);
                                        //
                                        //     let bit_pos = 7 - (local_x % 8);
                                        //
                                        //     let byte = pixels[byte_index];
                                        //     let bit = (byte >> bit_pos) & 1;
                                        //
                                        //     let luma = if bit == 1 { 255 } else { 0 };
                                        //     r = luma;
                                        //     g = luma;
                                        //     b = luma;
                                        // } else
                                        if bpp >= 3 {
                                            r = pixels[src_idx * bpp];
                                            g = pixels[src_idx * bpp + 1];
                                            b = pixels[src_idx * bpp + 2];
                                        } else if bpp == 2 && colour_format == 4 {
                                            // let bytes = pixels[src_idx*bpp] + pixels[src_idx*bpp+1];
                                            // let pixel = (bytes[0] as u16) << 8 | (bytes[1] as u16); big endian
                                            let bytes = (pixels[src_idx * bpp + 1] as u16) << 8
                                                | (pixels[src_idx * bpp] as u16); //little endian
                                            let r0 = (bytes >> 0) & 0b11111;
                                            let g0 = (bytes >> 5) & 0b111111;
                                            let b0 = (bytes >> 11) & 0b11111;
                                            //rgb565? rrrrrggggggbbbbb
                                            //bbbbbggggggrrrrr
                                            r = (r0 as f32 * 8.225806) as u8;
                                            g = (g0 as f32 * 4.047619) as u8;
                                            b = (b0 as f32 * 8.225806) as u8;
                                        } else if bpp == 1 && (colour_format == 1 || colour_format == 2) {
                                            let byte = pixels[src_idx];
                                            let r0 = (byte >> 2) & 0b11;
                                            let g0 = (byte >> 4) & 0b11;
                                            let b0 = (byte >> 6) & 0b11;

                                            r = r0*85;
                                            g = g0*85;
                                            b = b0*85;

                                            //rrggbbaa
                                            //aabbggrr
                                        } else if bpp == 1 && colour_format == 3 {
                                            let byte = pixels[src_idx];
                                            let r0 = (byte >> 0) & 0b111;
                                            let g0 = (byte >> 3) & 0b111;
                                            let b0 = (byte >> 6) & 0b11;

                                            r = (r0 as f32 * 36.42857) as u8;
                                            g = (g0 as f32 * 36.42857) as u8;
                                            b = b0 * 85;

                                            //rrrgggbb
                                            //bbgggrrr
                                        } else {
                                            let byte = pixels[src_idx];
                                            let r0 = (byte >> 0) & 0b11;
                                            let g0 = (byte >> 2) & 0b11;
                                            let b0 = (byte >> 4) & 0b11;

                                            r = r0*85;
                                            g = g0*85;
                                            b = b0*85;
                                            //rrggbb??
                                        };

                                        let r_luma = post_proc_bin.data[r as usize];
                                        let g_luma = post_proc_bin.data[g as usize];
                                        let b_luma = post_proc_bin.data[b as usize];

                                        let rgb = Color::Rgb(r_luma, g_luma, b_luma);
                                        if blue_noise {
                                            fb.set_pixel(
                                                scaled_l + x_out + x_padding,
                                                scaled_t + y_out + y_padding,
                                                transform_dither_g2(
                                                    scaled_l + x_out + x_padding,
                                                    scaled_t + y_out + y_padding,
                                                    rgb,
                                                ),
                                            );
                                        } else {
                                            fb.set_pixel(
                                                scaled_l + x_out + x_padding,
                                                scaled_t + y_out + y_padding,
                                                rgb,
                                            );
                                        };
                                    }
                                }
                            }
                        }
                        // println!("After continue");

                        let w = (vnc_rect.width as f32 * scale_factor).round();
                        let h = (vnc_rect.height as f32 * scale_factor).round();
                        let l = (vnc_rect.left as f32 * scale_factor).round();
                        let t = (vnc_rect.top as f32 * scale_factor).round();

                        let delta_rect = rect![
                            l as i32 + x_padding as i32,
                            t as i32 + y_padding as i32,
                            (l + /*w*/scaled_rect_width as f32+ x_padding as f32) as i32,
                            (t + /*h*/scaled_rect_height as f32+ y_padding as f32) as i32
                        ];

                        push_to_dirty_rect_list(&mut dirty_rects, delta_rect);

                        let elapsed_ms = time_at_sol.elapsed().as_millis();
                        debug!("End of PutPixels: {} MS elsaped since loop", elapsed_ms);

                        // counter += 1;
                        //
                        // cumulative_time += counter_time.elapsed().as_micros();
                        // cumulative_pixels += (scaled_rect_width*scaled_rect_height) as u128;
                        //
                        // println!("{}, {} pixels in {} micros {} pix/micro",
                        // counter, cumulative_pixels,cumulative_time,
                        //     cumulative_pixels/cumulative_time);
                    } else {

                        let w = vnc_rect.width as u32;
                        let h = vnc_rect.height as u32;
                        let l = vnc_rect.left as u32;
                        let t = vnc_rect.top as u32;

                        let mut left_x_truncate = 0;
                        let mut top_y_truncate = 0;
                        let mut right_x_truncate = 0;
                        let mut bottom_y_truncate = 0;

                        if height > fb.height() as u16 {
                            if t > fb.height() + y_offset {
                                continue;
                            }; //if top is greater than upper limit
                            if t + h < y_offset {
                                continue;
                            }; //if bottom is less than lower limit
                            //
                            // top_y_truncate = if y_offset as i32- t as i32 > 0  {y_offset - t} else {0};
                            // bottom_y_truncate = if (y_offset+fb.height()) as i32 - t as i32 > 0
                            //     && ((y_offset+fb.height()) as i32 - t as i32) < h as i32 {y_offset + fb.height() - t} else {0};
                            // println!("New{},{}", top_y_truncate, bottom_y_truncate);
                        };

                        if width > fb.width() as u16 {
                            // if l > fb.width()+x_offset || l < x_offset { continue };
                            if l > fb.width() + x_offset {
                                continue;
                            }; //if left is greater than upper limit
                            if l + w < x_offset {
                                continue;
                            }; //if right is less than lower limit

                            // left_x_truncate = if x_offset as i32- l as i32 > 0  {x_offset - l} else {0};
                            // right_x_truncate = if (x_offset+fb.height()) as i32 - l as i32 > 0
                            //     && ((x_offset+fb.height()) as i32 - l as i32) < w as i32 {x_offset+fb.width() - l} else {0};
                            // println!("New{},{}", left_x_truncate, right_x_truncate);
                        }; //left could be lower than limit and right could be more than upper, but doesnt mean whole rect is out of bounds

                        // if false {
                        if encoding {
                            let row_bytes = (vnc_rect.width as usize + 7) / 8;
                            let mut row_buf = [0u8; 2048];

                            for y_out in 0..h {
                                if height > fb.height() as u16 {
                                    if t + y_out < y_offset { continue; }
                                    if t + y_out == fb.height() + y_offset {
                                        bottom_y_truncate = y_out as u32;
                                        break;
                                    }
                                    if t + y_out == y_offset {
                                        top_y_truncate = y_out as u32;
                                    }
                                }

                                let mut col = 0usize;
                                for src_byte_idx in 0..row_bytes {
                                    if width > fb.width() as u16 {
                                        if l as usize + col < x_offset as usize { col += 8; continue; }
                                        if l as usize + col >= (fb.width() + x_offset) as usize {
                                            right_x_truncate = (col / 8) as u32;
                                            break;
                                        }
                                        if l as usize + col == x_offset as usize {
                                            left_x_truncate = (col / 8) as u32;
                                        }
                                    }
                                    let byte = pixels[y_out as usize * row_bytes + src_byte_idx];
                                    row_buf[col..col + 8].copy_from_slice(&EXPAND_1BPP[byte as usize]);
                                    col += 8;
                                }

                                let row_width = if right_x_truncate > 0 { right_x_truncate as usize } else { w as usize };

                                // println!("Enc{},{}", left_x_truncate, right_x_truncate);

                                fb.write_row_1bpp(
                                    (l as i32 + x_padding as i32 - x_offset as i32 + left_x_truncate as i32) as u32,
                                    (t as i32 + y_out as i32 + y_padding as i32 - y_offset as i32) as u32,
                                    &row_buf[left_x_truncate as usize..row_width],
                                );
                            }

                            // println!("Above continue");
                            // continue 'event
                        } else {
                            #[cfg(feature = "eink_device")]
                            {
                                'row: for row in 0..h {
                                    if height > fb.height() as u16 {
                                        if t + row < y_offset {
                                            //if y is less than lower limit, skip this row
                                            continue;
                                        };
                                        if t + row == fb.height() + y_offset {
                                            //if y is greater than upper limit, break row loop?
                                            bottom_y_truncate = row; //break column loop? because the rect is done, no more pixels will be in bounds
                                            // println!("Old{},{}", top_y_truncate, bottom_y_truncate);
                                            break 'row;
                                        };
                                        //we have filtered out rects that are entirely out of bounds
                                        //now filter partial in bounds or, entirely in bounds
                                        if t + row == y_offset {
                                            //if y is greater than lower limit?
                                            top_y_truncate = row; //if exactly on limit, make truncate this row
                                            // println!("Old{},{}", top_y_truncate, bottom_y_truncate);
                                        };
                                    };

                                    let row_idx = row * w;
                                    'col: for col in 0..w {
                                        if width > fb.width() as u16 {
                                            if l + col < x_offset {
                                                //if x below lower limit skip this pixel
                                                continue;
                                            }
                                            if l + col == fb.width() + x_offset { //if x is upper bound
                                                right_x_truncate = col; //since the limit will be the same for each row... no, we only want to break this one
                                                // println!("Old{},{}", left_x_truncate, right_x_truncate);
                                                break 'col; //because we must still process the remaining pixels and set them
                                            }
                                            if l + col == x_offset {
                                                left_x_truncate = col; //if x is lower bound
                                                // println!("Old{},{}", left_x_truncate, right_x_truncate);
                                            }
                                        };

                                        //let c = Color::Gray(gray_pixels[(row * w + col) as usize]);
                                        //pixels is vec of u8, 1 byte per vector element
                                        //4 elements make one pixel
                                        let src_idx = (row_idx + col) as usize;

                                        let mut luma = 0;
                                        let r;
                                        let g;
                                        let b;

                                        if !encoding && src_idx * bpp > pixels.len() {
                                            //dbg!(src_idx*bpp,pixels.len());
                                            //pixels is collection of bytes. u8. 4 bytes is 1 pixel.
                                            //oldest forced 8bits per pixel means 1 byte 1 is 1 pixel, if step by 4 then
                                            //only samplying every 4th pixel? if 8 bit forced would it still be vec of u8/ wouldnt it be vec of u2? 2 bits?
                                            //u8 used bc cpu is byte addressable, smallest unit
                                            dbg!(src_idx * bpp > pixels.len());
                                            continue
                                        } else {
                                            // if encoding {
                                            //     if src_idx * 1/8 > pixels.len() {
                                            //         continue
                                            //     }
                                            //
                                            //     let row_bytes = (vnc_rect.width as usize + 7) / 8;
                                            //
                                            //     let byte_index =
                                            //         (row as usize * row_bytes) + (col as usize / 8);
                                            //
                                            //     let bit_pos = 7 - (col % 8);
                                            //
                                            //     let byte = pixels[byte_index];
                                            //     let bit = (byte >> bit_pos) & 1;
                                            //
                                            //     let luma = if bit == 1 { 255 } else { 0 };
                                            //     r = luma;
                                            //     g = luma;
                                            //     b = luma;
                                            //
                                            // } else
                                            if bpp >= 3 {
                                                r = pixels[src_idx * bpp];
                                                g = pixels[src_idx * bpp + 1];
                                                b = pixels[src_idx * bpp + 2];
                                            } else if bpp == 2 && colour_format == 4 {
                                                // let bytes = pixels[src_idx*bpp] + pixels[src_idx*bpp+1];
                                                // let pixel = (bytes[0] as u16) << 8 | (bytes[1] as u16); big endian
                                                let bytes = (pixels[src_idx * bpp + 1] as u16) << 8
                                                    | (pixels[src_idx * bpp] as u16); //little endian
                                                let r0 = (bytes >> 0) & 0b11111;
                                                let g0 = (bytes >> 5) & 0b111111;
                                                let b0 = (bytes >> 11) & 0b11111;
                                                //rgb565? rrrrrggggggbbbbb
                                                //bbbbbggggggrrrrr
                                                r = (r0 as f32 * 8.225806) as u8;
                                                g = (g0 as f32 * 4.047619) as u8;
                                                b = (b0 as f32 * 8.225806) as u8;
                                            } else if bpp == 1 && (colour_format == 1 || colour_format == 2) {
                                                let byte = pixels[src_idx];
                                                let r0 = (byte >> 2) & 0b11;
                                                let g0 = (byte >> 4) & 0b11;
                                                let b0 = (byte >> 6) & 0b11;

                                                r = r0*85;
                                                g = g0*85;
                                                b = b0*85;

                                                //rrggbbaa
                                                //aabbggrr
                                            } else if bpp == 1 && colour_format == 3 {
                                                let byte = pixels[src_idx];
                                                let r0 = (byte >> 0) & 0b111;
                                                let g0 = (byte >> 3) & 0b111;
                                                let b0 = (byte >> 6) & 0b11;

                                                r = (r0 as f32 * 36.42857) as u8;
                                                g = (g0 as f32 * 36.42857) as u8;
                                                b = b0 * 85;

                                                //rrrgggbb
                                                //bbgggrrr
                                            } else {
                                                let byte = pixels[src_idx];
                                                let r0 = (byte >> 0) & 0b11;
                                                let g0 = (byte >> 2) & 0b11;
                                                let b0 = (byte >> 4) & 0b11;

                                                r = r0*85;
                                                g = g0*85;
                                                b = b0*85;
                                                //rrggbb??
                                            };

                                            let r_luma = post_proc_bin.data[r as usize];
                                            let g_luma = post_proc_bin.data[g as usize];
                                            let b_luma = post_proc_bin.data[b as usize];

                                            let rgb = Color::Rgb(r_luma, g_luma, b_luma);
                                            if blue_noise {
                                                fb.set_pixel(
                                                    l + col - x_offset + x_padding,
                                                    t + row - y_offset + y_padding,
                                                    transform_dither_g2(
                                                        l + col - x_offset + x_padding,
                                                        t + row - y_offset + y_padding,
                                                        rgb,
                                                    ),
                                                );
                                            } else {
                                                fb.set_pixel(
                                                    l + col - x_offset + x_padding,
                                                    t + row - y_offset + y_padding,
                                                    rgb,
                                                );
                                            };
                                        };
                                    }
                                }
                                //draw gray_tile merely creates grayscale pixel vec, does not do drawing?
                                //actual pixel updating happens in client.rs fb.update method
                            }
                        }
                        // println!("After continue");

                        //there is no coord to say, draw rect at location. instead each pixel is drawn one by one into fb...
                        //and then update called separately

                        let mut w = vnc_rect.width as i32;
                        let mut h = vnc_rect.height as i32;
                        let l = vnc_rect.left as i32;
                        let t = vnc_rect.top as i32;

                        if right_x_truncate > 0 {
                            w = right_x_truncate as i32
                        }
                        if bottom_y_truncate > 0 {
                            h = bottom_y_truncate as i32
                        }

                        let delta_rect = rect![
                            l + x_padding as i32 + left_x_truncate as i32 - x_offset as i32,
                            t + y_padding as i32 + top_y_truncate as i32 - y_offset as i32,
                            l + w + x_padding as i32 - x_offset as i32,
                            t + h + y_padding as i32 - y_offset as i32
                        ];
                        push_to_dirty_rect_list(&mut dirty_rects, delta_rect);

                        let elapsed_ms = time_at_sol.elapsed().as_millis();
                        debug!("End of PutPixels: {} MS elsaped since loop", elapsed_ms);

                        // counter += 1;
                        //
                        // cumulative_time += counter_time.elapsed().as_micros();
                        // cumulative_pixels += pixels.len() as u128;
                        //
                        // println!("{}, {} pixels in {} micros {} pix/micro",
                        // counter, cumulative_pixels,cumulative_time,
                        //     cumulative_pixels/cumulative_time);
                    };

                    // Single pass: convert to grayscale + apply post-processing LUT.
                    // Use the current negotiated format (may have changed via set_format).
                }

                Event::CopyPixels { src, dst } => {
                    let elapsed_ms = time_at_sol.elapsed().as_millis();
                    debug!("Copy pixels {} MS elsaped since loop", elapsed_ms);
                    //most of the time copyrect is never sent by server...

                    #[cfg(feature = "eink_device")]
                    {
                        if scale {
                            {
                                if (src.width as f32 * scale_factor).round() as u32 == 0
                                    || (src.height as f32 * scale_factor).round() as u32 == 0
                                {
                                    continue;
                                }

                                let src_left = (src.left as f32 * scale_factor);
                                let src_top = (src.top as f32 * scale_factor);

                                let dst_left = (dst.left as f32 * scale_factor);
                                let dst_top = (dst.top as f32 * scale_factor);

                                let mut intermediary_pixmap = Pixmap::new(
                                    (dst.width as f32 * scale_factor).round() as u32,
                                    (dst.height as f32 * scale_factor).round() as u32,
                                    CURRENT_DEVICE.color_samples(),
                                );

                                for y in 0..intermediary_pixmap.height {
                                    let y_total= (src_top + y as f32 + y_padding as f32).round() as u32;
                                    //copypixels merely copy whats on framebuffer, if putpixels blue noise dithered so will copied
                                    for x in 0..intermediary_pixmap.width {
                                        let color = fb.get_pixel(
                                            (src_left + x as f32 + x_padding as f32).round() as u32,
                                            y_total,
                                        );
                                        intermediary_pixmap.set_pixel(x, y, color);
                                    }
                                }

                                for y in 0..intermediary_pixmap.height {
                                    let y_total = (dst_top + y as f32 + y_padding as f32).round() as u32;
                                    for x in 0..intermediary_pixmap.width {
                                        let color = intermediary_pixmap.get_pixel(x, y);
                                        fb.set_pixel(
                                            (dst_left + x as f32 + x_padding as f32).round() as u32,
                                            y_total,
                                            color,
                                        );
                                    }
                                }
                            }

                            let delta_rect = rect![
                                (dst.left as f32 * scale_factor).round() as i32 + x_padding as i32,
                                (dst.top as f32 * scale_factor).round() as i32 + y_padding as i32,
                                ((dst.left as f32 * scale_factor) + dst.width as f32).round() as i32 + x_padding as i32,
                                ((dst.top as f32 * scale_factor) + dst.height as f32).round() as i32 + y_padding as i32
                            ];
                            push_to_dirty_rect_list(&mut dirty_rects, delta_rect);
                        //add to dirty rect list, merely copy rect to another place, no update call
                        } else {
                            let src_left = src.left as u32;
                            let src_top = src.top as u32;
                            let src_width = src.width as u32;
                            let src_height = src.height as u32;

                            let dst_left = dst.left as u32;
                            let dst_top = dst.top as u32;
                            let mut dst_width = dst.width as u32;
                            let mut dst_height = dst.height as u32;

                            let mut src_left_x_truncate = 0;
                            let mut src_top_y_truncate = 0;
                            let mut src_right_x_truncate = 0;
                            let mut src_bottom_y_truncate = 0;

                            let mut dst_left_x_truncate = 0;
                            let mut dst_top_y_truncate = 0;
                            let mut dst_right_x_truncate = 0;
                            let mut dst_bottom_y_truncate = 0;



                            {
                                if height > fb.height() as u16 {
                                    if dst_top > fb.height() + y_offset {
                                        println!("OOBCPchDT");
                                        continue;
                                    }; //if top is greater than upper
                                    if dst_top + dst_height < y_offset {
                                        println!("OOBCPchDT");
                                        continue;
                                    }; //if bot is less than lower
                                    if src_top > fb.height() + y_offset {
                                        println!("OOBCPchST");
                                        continue;
                                    }; //if top is greater than upper
                                    if src_top + dst_height < y_offset {
                                        println!("OOBCPchST");
                                        continue;
                                    }; //if bot is less than lower
                                }

                                if width > fb.width() as u16 {
                                    if dst_left > fb.width() + x_offset {
                                        println!("OOBCPcwDL");
                                        continue;
                                    }; //if left is greater than upper
                                    if dst_left + dst_width < x_offset {
                                        println!("OOBCPcwDL");
                                        continue;
                                    }; //if right is less than lower
                                    if src_left > fb.width() + x_offset {
                                        println!("OOBCPcwSL");
                                        continue;
                                    }; //if left is greater than upper
                                    if src_left + dst_width < x_offset {
                                        println!("OOBCPcwSL");
                                        continue;
                                    }; //if right is less than lower
                                }

                                let mut intermediary_pixmap = Pixmap::new(
                                    dst.width as u32,
                                    dst.height as u32,
                                    CURRENT_DEVICE.color_samples(),
                                );

                                'src_y: for y in 0..intermediary_pixmap.height {
                                    if height > fb.height() as u16 {
                                        if y + src_top == fb.height() + y_offset {
                                            src_bottom_y_truncate = y;
                                            break 'src_y;
                                        } //if y pixel is greater than upper

                                        if y + src_top < y_offset {
                                            // println!("OOBCPph");
                                            continue;
                                        } //do we want continue or break first? which saves cycles?

                                        if y + src_top == y_offset {
                                            src_top_y_truncate = y;
                                        } //if y less than lower, once hits lower limit
                                    };
                                    let y_total = src_top + y + y_padding - y_offset as u32;

                                    'src_x: for x in 0..intermediary_pixmap.width {

                                        if width > fb.width() as u16 {
                                            if x + src_left == fb.width() + x_offset {
                                                src_right_x_truncate = x;
                                                break 'src_x;
                                            }
                                            if x + src_left < x_offset {
                                                // println!("OOBCPpw");
                                                continue;
                                            }
                                            if x + src_left == x_offset {
                                                src_left_x_truncate = x;
                                            }
                                        };
                                        let color = fb.get_pixel(
                                            src_left + x + x_padding - x_offset as u32,
                                            y_total,
                                        );
                                        intermediary_pixmap.set_pixel(x, y, color);
                                    }
                                }

                                'dst_y: for y in 0..intermediary_pixmap.height {
                                    if height > fb.height() as u16 {
                                        if y + dst_top == fb.height() + y_offset {
                                            dst_bottom_y_truncate = y;
                                            break 'dst_y;
                                        } //if y pixel is greater than upper

                                        if y + dst_top < y_offset {
                                            // println!("OOBCPph");
                                            continue;
                                        } //do we want continue or break first? which saves cycles?

                                        if y + dst_top == y_offset {
                                            dst_top_y_truncate = y;
                                        } //if y less than lower, once hits lower limit
                                    };
                                    let y_total = dst_top + y - y_offset + y_padding as u32;

                                    'dst_x: for x in 0..intermediary_pixmap.width {
                                        let color = intermediary_pixmap.get_pixel(x, y);

                                        if width > fb.width() as u16 {
                                            if x + dst_left == fb.width() + x_offset {
                                                dst_right_x_truncate = x;
                                                break 'dst_x;
                                            }
                                            if x + dst_left < x_offset {
                                                // println!("OOBCPpw");
                                                continue;
                                            }
                                            if x + dst_left == x_offset {
                                                dst_left_x_truncate = x;
                                            }
                                        };
                                        // fb.set_pixel(dst_left + x, dst_top + y,  transform_dither_g2(dst_left + x, dst_top + y,color));
                                        fb.set_pixel(
                                            dst_left + x - x_offset + x_padding as u32,
                                            y_total,
                                            color,
                                        );
                                    }
                                }
                            }
                            if dst_right_x_truncate > 0 {
                                dst_width = dst_right_x_truncate
                            }
                            if dst_bottom_y_truncate > 0 {
                                dst_height = dst_bottom_y_truncate
                            }

                            let delta_rect = rect![
                                dst_left as i32 + x_padding as i32 + dst_left_x_truncate as i32
                                    - x_offset as i32,
                                dst_top as i32 + y_padding as i32 + dst_top_y_truncate as i32
                                    - y_offset as i32,
                                (dst_left + dst_width) as i32 + x_padding as i32 - x_offset as i32,
                                (dst_top + dst_height) as i32 + y_padding as i32 - y_offset as i32
                            ];
                            push_to_dirty_rect_list(&mut dirty_rects, delta_rect);
                        };
                    }
                }
                Event::EndOfFrame => {
                    let elapsed_ms = time_at_sol.elapsed().as_millis();
                    debug!("End of frame! {} MS elsaped since loop", elapsed_ms);
                    frame_complete = true;
                }
                x => info!("{:?}", x), /* ignore unsupported events */
                // _ => (),
            }
        }

        if frame_complete {
            if scale {
                if vnc.as_mut().unwrap()
                    .request_update(
                        Rect {
                            left: 0,
                            top: 0,
                            width,
                            height,
                        },
                        true,
                    )
                    .is_err()
                {
                    error!("server disconnected");
                    break;
                }
            } else {
                if vnc.as_mut().unwrap()
                    .request_update(
                        Rect {
                            left: 0 + x_offset as u16,
                            top: 0 + y_offset as u16,
                            width: if width < fb.width() as u16 {
                                width
                            } else {
                                fb.width() as u16
                            },
                            height: if height < fb.height() as u16 {
                                height
                            } else {
                                fb.height() as u16
                            },
                        },
                        true,
                    )
                    .is_err()
                {
                    error!("server disconnected");
                    break;
                }
            }
        }
        //only at end of frame request a new update

        if (time_at_last_draw.elapsed().as_millis() as u64) < FRAME_MS {
            let elapsed_ms = time_at_sol.elapsed().as_millis();
            debug!(
                    "Sleep for {} milliseconds, target {}, {} MS elsaped since loop",
                    time_at_last_draw.elapsed().as_millis(), FRAME_MS, elapsed_ms
                );
            thread::sleep(Duration::from_millis(
                FRAME_MS - time_at_last_draw.elapsed().as_millis() as u64,
                //time at sol is when running loop started for that pass
            ));
        } else if time_at_last_draw.elapsed().as_millis() as u64 >= FRAME_MS {
            let elapsed_ms = time_at_sol.elapsed().as_millis();
            if dirty_update_count > max_dirty_refreshes {

                let mut min_x = fb.width() as i32;
                let mut min_y = fb.height() as i32;
                let mut max_x = 0;
                let mut max_y = 0;
                for dr in &dirty_rects_since_refresh {
                    if dr.max.x > max_x {
                        max_x = dr.max.x
                    }
                    if dr.max.y > max_y {
                        max_y = dr.max.y
                    }
                    if dr.min.x < min_x {
                        min_x = dr.min.x
                    }
                    if dr.min.y < min_y {
                        min_y = dr.min.y
                    }
                }
                let mut union = rect![min_x,min_y,max_x,max_y];
                fb.update(&union, full_update_mode, /*UpdateMode::Partial GC16*/).ok();
                dirty_update_count = 0;

                debug!(
                    "Full update, since last {}, target {}, u_rect {}, late by {}, {} since loop, {} DRs",
                    time_at_last_draw.elapsed().as_millis(), FRAME_MS, union,
                    time_at_last_draw.elapsed().as_millis() as u64 - FRAME_MS, elapsed_ms, dirty_rects.len()
                );

                dirty_rects_since_refresh.clear();
                time_at_last_draw = Instant::now();
            } else {

                let mut min_x = fb.width() as i32;
                let mut min_y = fb.height() as i32;
                let mut max_x = 0;
                let mut max_y = 0;
                for dr in &dirty_rects {
                    if dr.max.x > max_x {
                        max_x = dr.max.x
                    }
                    if dr.max.y > max_y {
                        max_y = dr.max.y
                    }
                    if dr.min.x < min_x {
                        min_x = dr.min.x
                    }
                    if dr.min.y < min_y {
                        min_y = dr.min.y
                    }

                }
                let mut union = rect![min_x,min_y,max_x,max_y];
                fb.update(&union, partial_update_mode, /*UpdateMode::Partial GC16*/).ok();
                dirty_update_count += 1;

                debug!(
                    "Partial update, since last {}, target {}, u_rect {}, late by {}, {} since loop, {} DRs",
                    time_at_last_draw.elapsed().as_millis(), FRAME_MS, union,
                    time_at_last_draw.elapsed().as_millis() as u64 - FRAME_MS, elapsed_ms, dirty_rects.len()
                );

                dirty_rects.clear();
                time_at_last_draw = Instant::now();
            }
        }
    }
    Ok(())
}

fn push_to_dirty_rect_list(list: &mut Vec<Rectangle>, rect: Rectangle) {
    for dr in list.iter_mut() {
        if dr.contains(&rect) {
            return;
        }
        if rect.contains(&dr) {
            *dr = rect;
            return;
        }
        if rect.extends(&dr) {
            dr.absorb(&rect);
            return;
        }
    }
    list.push(rect);
}
