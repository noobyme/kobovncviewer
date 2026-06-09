use std::sync::mpsc::{self, Sender, Receiver};
use std::time::{Duration, Instant};
use crate::device::CURRENT_DEVICE;
use crate::framebuffer::Framebuffer;
use crate::geom::Dir;
use crate::gesture::{Event, GestureEvent};
use crate::input::{ButtonCode, ButtonStatus, DeviceEvent, FingerStatus};
use crate::vnc::{Client, Rect};

pub struct event_params {
    pub has_drawn_once: bool,
    pub finger_down_count: Instant,
    pub exit_to_nickel:bool,
    pub exit_to_gui:bool,
    pub x_offset:u32,
    pub y_offset:u32,
}

impl event_params {
    pub fn handle_events(rx: &Receiver<Event>, scale_factor: f32, width: u16, height: u16,
                         fb_width: u32, fb_height: u32, x_padding: u32, y_padding: u32, mut x_offset: u32, mut y_offset: u32,
                         vnc: &mut Option<Client>, mut finger_down_count: Instant, finger_seconds: Duration, fb: &mut Box<dyn Framebuffer>,
                         panning: bool, mut has_drawn_once: bool, scale: bool, long_tap: bool, gui_enabled:bool, disable_touch:bool,
                         disable_exit_via_hold:bool, exit_duration:u32,
                         ) -> event_params {
        if let Ok(evt) = rx.try_recv() {
            match evt {
                Event::Device(de) => {
                    match de {
                        DeviceEvent::Finger {
                            id,
                            time,
                            status,
                            position,
                        } => {
                            match id {
                                0 | 1 | 2 => {
                                    match status {
                                        FingerStatus::Up => {
                                            //we only want send right click once we release long_tap
                                            if scale && !disable_touch {
                                                if long_tap {
                                                    if finger_down_count.elapsed() > finger_seconds
                                                    {
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x04,
                                                            (((position.x as f32
                                                                - x_padding as f32)
                                                                / scale_factor)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            (((position.y as f32
                                                                - y_padding as f32)
                                                                / scale_factor)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x00,
                                                            (((position.x as f32
                                                                - x_padding as f32)
                                                                / scale_factor)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            (((position.y as f32
                                                                - y_padding as f32)
                                                                / scale_factor)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                        //dbg!(((position.x as f32-x_padding as f32) / scale_factor) as u16, ((position.y as f32-y_padding as f32) / scale_factor) as u16);
                                                    }
                                                } else {
                                                     vnc.as_mut().unwrap().send_pointer_event(
                                                        0x00,
                                                        (((position.x as f32 - x_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, width as u16),
                                                        (((position.y as f32 - y_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, height as u16),
                                                    )
                                                        .unwrap();
                                                    //dbg!(((position.x as f32 - x_padding as f32) / scale_factor) as u16, ((position.y as f32 - y_padding as f32) / scale_factor) as u16);
                                                }
                                            } else if !disable_touch {
                                                if long_tap {
                                                    if finger_down_count.elapsed() > finger_seconds {
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x04,
                                                            ((position.x as i16 - x_padding as i16
                                                                + x_offset as i16)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            ((position.y as i16 - y_padding as i16
                                                                + y_offset as i16)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x00,
                                                            ((position.x as i16 - x_padding as i16
                                                                + x_offset as i16)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            ((position.y as i16 - y_padding as i16
                                                                + y_offset as i16)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                        //dbg!(position.x as u16-x_padding as u16, position.y as u16-y_padding as u16);

                                                    }
                                                } else {
                                                    if panning {
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x01,
                                                            ((position.x as i16 - x_padding as i16
                                                                + x_offset as i16)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            ((position.y as i16 - y_padding as i16
                                                                + y_offset as i16)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x00,
                                                            ((position.x as i16 - x_padding as i16
                                                                + x_offset as i16)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            ((position.y as i16 - y_padding as i16
                                                                + y_offset as i16)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                        //dbg!(position.x as u16-x_padding as u16, position.y as u16-y_padding as u16);
                                                    } else {
                                                         vnc.as_mut().unwrap().send_pointer_event(
                                                            0x00,
                                                            ((position.x as i16 - x_padding as i16
                                                                + x_offset as i16)
                                                                as u16)
                                                                .clamp(0, width as u16),
                                                            ((position.y as i16 - y_padding as i16
                                                                + y_offset as i16)
                                                                as u16)
                                                                .clamp(0, height as u16),
                                                        )
                                                            .unwrap();
                                                        //dbg!(position.x as u16-x_padding as u16, position.y as u16-y_padding as u16);
                                                    }
                                                }
                                            };
                                            if finger_down_count.elapsed() > Duration::from_secs(exit_duration as u64) && !disable_exit_via_hold {
                                                if gui_enabled {
                                                    return event_params {
                                                        has_drawn_once,
                                                        finger_down_count,
                                                        exit_to_nickel: if gui_enabled {false} else {true},
                                                        exit_to_gui: if gui_enabled {true} else {false},
                                                        x_offset,
                                                        y_offset,
                                                    }
                                                } else {
                                                    return event_params {
                                                        has_drawn_once,
                                                        finger_down_count,
                                                        exit_to_nickel: if gui_enabled {false} else {true},
                                                        exit_to_gui: if gui_enabled {true} else {false},
                                                        x_offset,
                                                        y_offset,
                                                    }
                                                }
                                            };
                                        }
                                        FingerStatus::Down => {
                                            if scale && !disable_touch {
                                                if panning {
                                                    finger_down_count = Instant::now();
                                                } else {
                                                     vnc.as_mut().unwrap().send_pointer_event(
                                                        0x01,
                                                        (((position.x as f32 - x_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, width as u16),
                                                        (((position.y as f32 - y_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, height as u16),
                                                    )
                                                        .unwrap();
                                                    finger_down_count = Instant::now();
                                                    //dbg!((((position.x as f32 - x_padding as f32)/ scale_factor) as u16).clamp(0,width as u16),
                                                    // (((position.y as f32 - y_padding as f32)/ scale_factor) as u16).clamp(0,height as u16));
                                                }

                                            } else if !disable_touch {
                                                if panning {
                                                    finger_down_count = Instant::now();
                                                } else {
                                                     vnc.as_mut().unwrap().send_pointer_event(
                                                        0x01,
                                                        ((position.x as i16 - x_padding as i16
                                                            + x_offset as i16)
                                                            as u16)
                                                            .clamp(0, width as u16),
                                                        ((position.y as i16 - y_padding as i16
                                                            + y_offset as i16)
                                                            as u16)
                                                            .clamp(0, height as u16),
                                                    )
                                                        .unwrap();
                                                    finger_down_count = Instant::now();
                                                    //dbg!(position.x as u16-x_padding as u16,position.y as u16-y_padding as u16);
                                                }
                                            }
                                            finger_down_count = Instant::now();
                                        }
                                        FingerStatus::Motion => {
                                            if scale && !disable_touch {
                                                if panning {

                                                } else {
                                                     vnc.as_mut().unwrap().send_pointer_event(
                                                        0x01,
                                                        (((position.x as f32 - x_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, width as u16),
                                                        (((position.y as f32 - y_padding as f32)
                                                            / scale_factor)
                                                            as u16)
                                                            .clamp(0, height as u16),
                                                    )
                                                        .unwrap();
                                                    //dbg!((((position.x as f32 - x_padding as f32) / scale_factor) as u16).clamp(0,width as u16), (((position.y as f32 - y_padding as f32) / scale_factor) as u16).clamp(0,height as u16));
                                                    //100-10/2 45 100/2-10=40
                                                    //from physical framebuffer means must minus padding before scale, scale is so original
                                                }
                                            } else if !disable_touch {
                                                if panning {

                                                } else {
                                                     vnc.as_mut().unwrap().send_pointer_event(
                                                        0x01,
                                                        ((position.x as i16 - x_padding as i16
                                                            + x_offset as i16)
                                                            as u16)
                                                            .clamp(0, width as u16),
                                                        ((position.y as i16 - y_padding as i16
                                                            + y_offset as i16)
                                                            as u16)
                                                            .clamp(0, height as u16),
                                                    )
                                                        .unwrap();
                                                    //dbg!(position.x as u16-x_padding as u16, position.y as u16-y_padding as u16)
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    println!("Unknown finger ID")
                                }
                            }
                        }
                        DeviceEvent::Button {
                            code: ButtonCode::Power,
                            status: ButtonStatus::Pressed,
                            ..
                        } => {
                            // println!("BUTTON");
                            return event_params {
                                has_drawn_once,
                                finger_down_count,
                                exit_to_nickel: if gui_enabled {false} else {true},
                                exit_to_gui: if gui_enabled {true} else {false},
                                x_offset,
                                y_offset,
                            }
                        }
                        DeviceEvent::CoverOn => {
                            // println!("COVER");
                            return event_params {
                                has_drawn_once,
                                finger_down_count,
                                exit_to_nickel: if gui_enabled {false} else {true},
                                exit_to_gui: if gui_enabled {true} else {false},
                                x_offset,
                                y_offset,
                            }
                        }
                        // DeviceEvent::Button { code: ButtonCode::Light, status: ButtonStatus::Pressed, .. } => {
                        //     tx.send(Event::ToggleFrontlight).ok();
                        // },
                        // DeviceEvent::RotateScreen(i8) => {
                        // },
                        _ => {}
                    }
                }
                Event::Gesture(ge) => {
                    if panning {
                        match ge {
                            GestureEvent::Swipe { dir, .. } => {
                                // dbg!(x_offset, y_offset, dir);
                                match dir {
                                    Dir::North => {
                                        if height > fb_height as u16 {
                                            has_drawn_once = false;
                                            if y_offset + fb_height + fb_height / 2 < height as u32
                                            {
                                                y_offset += fb_height / 2;
                                            } else {
                                                y_offset += height as u32 - fb_height - y_offset;
                                            }
                                        }
                                    }
                                    Dir::East => {
                                        if width > fb_width as u16 {
                                            has_drawn_once = false;
                                            //0-1920-379 >379
                                            if x_offset > fb_width {
                                                x_offset -= fb_width / 2;
                                            } else {
                                                x_offset = 0;
                                            }
                                        }
                                    }
                                    Dir::South => {
                                        if height > fb_height as u16 {
                                            has_drawn_once = false;
                                            // 0 -1080/2 >1080/2
                                            if y_offset > fb_height {
                                                y_offset -= fb_height / 2;
                                            } else {
                                                y_offset = 0;
                                            }
                                        }
                                    }
                                    Dir::West => {
                                        if width > fb_width as u16 {
                                            has_drawn_once = false;
                                            //0+758+758/2 < 1920
                                            if x_offset + fb_width + fb_width / 2 < width as u32 {
                                                x_offset += fb_width / 2;
                                            } else {
                                                // =1000+1920-758-1000
                                                x_offset += width as u32 - fb_width - x_offset;
                                            }
                                        }
                                    }
                                }
                                if  vnc.as_mut().unwrap()
                                    .request_update(
                                        Rect {
                                            left: 0 + x_offset as u16,
                                            top: 0 + y_offset as u16,
                                            width: fb_width as u16,
                                            height: fb_height as u16,
                                        },
                                        false,
                                    )
                                    .is_err()
                                {
                                    error!("server disconnected");
                                    // break;
                                }
                                // fb.update(&device_fb_rect, full_update_mode).ok();
                                // dbg!(x_offset,y_offset);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {},
            };
        }
        event_params {
            has_drawn_once,
            finger_down_count,
            exit_to_nickel: false,
            exit_to_gui: false,
            x_offset,
            y_offset,
        }
    }
}
