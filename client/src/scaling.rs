use crate::geom::{Rectangle};
use crate::rect;
pub struct scale_parameters {
    pub scale_factor:f32,
    pub x_padding: u32,
    pub y_padding: u32,
    pub device_fb_rect: Rectangle,
    pub cropped_vnc_fb_rect: Rectangle,
    pub original_vnc_fb_rect : Rectangle,
    pub scaled_fb_rect : Rectangle,
}

impl scale_parameters {
    pub fn new(scale:bool, width:u16, height:u16, fb_width:u32, fb_height:u32 ,x_offset:u32 ,y_offset:u32 ) -> scale_parameters {
        let mut scale_factor: f32 = 1.0;
        let mut x_padding = 0;
        let mut y_padding = 0;

        let mut device_fb_rect = rect![0, 0, fb_width as i32, fb_height as i32];
        let mut cropped_vnc_fb_rect = rect![
            0 + x_padding as i32,
            0 + y_padding as i32,
            fb_width as i32 + x_padding as i32,
            fb_height as i32 + y_padding as i32
        ];
        let mut original_vnc_fb_rect = rect![0, 0, width as i32, height as i32];
        let mut scaled_fb_rect = rect![
            0 + x_padding as i32,
            0 + y_padding as i32,
            width as i32 + x_padding as i32,
            height as i32 + y_padding as i32
        ];

        if scale {
            if width > height {
                //dbg!(fb_width,width,fb_height,height,(width as f32*scale_factor) as i32,(height as f32*scale_factor) as i32);
                if (height as f32 * (fb_width as f32 / width as f32)) > fb_height as f32 {
                    // fit_height = true;
                    scale_factor = fb_height as f32 / height as f32;
                    x_padding = ((fb_width - (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as u32) / 2) as u32; //when scale use height as scale factor,
                    //width is slightly smaller than expected, 758 to 768... if aspect ratio is exactly the same, at least for nia
                    y_padding = 0;
                    scaled_fb_rect = rect![
                        0 + x_padding as i32,
                        0 + y_padding as i32,
                        (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as i32 + x_padding as i32,
                        (height as f32 * scale_factor) as i32// + y_padding as i32
                    ];
                    scale_parameters {
                        scale_factor,
                        x_padding,
                        y_padding,
                        device_fb_rect,
                        cropped_vnc_fb_rect,
                        original_vnc_fb_rect,
                        scaled_fb_rect,
                    }
                } else {
                    // fit_width = true;
                    scale_factor = fb_width as f32 / width as f32;
                    y_padding = ((fb_height - (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as u32) / 2) as u32;
                    x_padding = 0;
                    scaled_fb_rect = rect![
                        0 + x_padding as i32,
                        0 + y_padding as i32,
                        (width as f32 * scale_factor) as i32,// + x_padding as i32,
                        (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as i32 + y_padding as i32
                    ];
                    scale_parameters {
                        scale_factor,
                        x_padding,
                        y_padding,
                        device_fb_rect,
                        cropped_vnc_fb_rect,
                        original_vnc_fb_rect,
                        scaled_fb_rect,
                    }
                }

            } else if height > width {
                //dbg!(fb_width,width,fb_height,height,(width as f32*scale_factor) as i32,(height as f32*scale_factor) as i32);
                //if 758x1024, true 3:4 will be 768x1024
                //758/768 = 0.98710865561*1024 = 1010.66666667
                //if 1072x1448, true 3:4 will be 1086x1448
                //1072/1086 = 0.98710865561*1448 = 1429.33333333
                //question is should we clip it or scale to fit the other axis instead?
                //scaled doesnt check for bounds thus leads to crash.
                if (width as f32 * (fb_height as f32 / height as f32)) > fb_width as f32 {
                    // fit_width = true;
                    scale_factor = fb_width as f32 / width as f32;
                    y_padding = ((fb_height - (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as u32) / 2) as u32;
                    x_padding = 0;
                    scaled_fb_rect = rect![
                        0 + x_padding as i32,
                        0 + y_padding as i32,
                        (width as f32 * scale_factor) as i32,// + x_padding as i32,
                        (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as i32 + y_padding as i32
                    ];
                    scale_parameters {
                        scale_factor,
                        x_padding,
                        y_padding,
                        device_fb_rect,
                        cropped_vnc_fb_rect,
                        original_vnc_fb_rect,
                        scaled_fb_rect,
                    }
                } else {
                    // fit_height = true;
                    scale_factor = fb_height as f32 / height as f32;
                    x_padding = ((fb_width - (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as u32) / 2) as u32; //when scale use height as scale factor,
                    //width is slightly smaller than expected, 758 to 768... if aspect ratio is exactly the same, at least for nia
                    y_padding = 0;
                    scaled_fb_rect = rect![
                        0 + x_padding as i32,
                        0 + y_padding as i32,
                        (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as i32 + x_padding as i32,
                        (height as f32 * scale_factor) as i32// + y_padding as i32
                    ];
                    scale_parameters {
                        scale_factor,
                        x_padding,
                        y_padding,
                        device_fb_rect,
                        cropped_vnc_fb_rect,
                        original_vnc_fb_rect,
                        scaled_fb_rect,
                    }
                }
            } else {
                if fb_height > fb_width {
                    //dbg!(fb_width,width,fb_height,height,(width as f32*scale_factor) as i32,(height as f32*scale_factor) as i32);
                    //want to fit to smallest fb axis instead.
                    if (width as f32 * (fb_height as f32 / height as f32)) > fb_width as f32 {
                        // fit_width = true;
                        scale_factor = fb_width as f32 / width as f32;
                        y_padding = ((fb_height - (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as u32) / 2) as u32;
                        x_padding = 0;
                        scaled_fb_rect = rect![
                            0 + x_padding as i32,
                            0 + y_padding as i32,
                            (width as f32 * scale_factor) as i32,// + x_padding as i32,
                            (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as i32 + y_padding as i32
                        ];
                        scale_parameters {
                            scale_factor,
                            x_padding,
                            y_padding,
                            device_fb_rect,
                            cropped_vnc_fb_rect,
                            original_vnc_fb_rect,
                            scaled_fb_rect,
                        }
                    } else {
                        // fit_height = true;
                        scale_factor = fb_height as f32 / height as f32;
                        x_padding = ((fb_width - (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as u32) / 2) as u32; //when scale use height as scale factor,
                        //width is slightly smaller than expected, 758 to 768... if aspect ratio is exactly the same, at least for nia
                        y_padding = 0;
                        scaled_fb_rect = rect![
                            0 + x_padding as i32,
                            0 + y_padding as i32,
                            (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as i32 + x_padding as i32,
                            (height as f32 * scale_factor) as i32// + y_padding as i32
                        ];
                        scale_parameters {
                            scale_factor,
                            x_padding,
                            y_padding,
                            device_fb_rect,
                            cropped_vnc_fb_rect,
                            original_vnc_fb_rect,
                            scaled_fb_rect,
                        }
                    }
                } else {
                    //dbg!(fb_width,width,fb_height,height,(width as f32*scale_factor) as i32,(height as f32*scale_factor) as i32);
                    if (height as f32 * (fb_width as f32 / width as f32)) > fb_height as f32 {
                        // fit_height = true;
                        scale_factor = fb_height as f32 / height as f32;
                        x_padding = ((fb_width - (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as u32) / 2) as u32; //when scale use height as scale factor,
                        //width is slightly smaller than expected, 758 to 768... if aspect ratio is exactly the same, at least for nia
                        y_padding = 0;
                        scaled_fb_rect = rect![
                            0 + x_padding as i32,
                            0 + y_padding as i32,
                            (width as f32 * scale_factor).clamp(0.0, fb_width as f32) as i32 + x_padding as i32,
                            (height as f32 * scale_factor) as i32// + y_padding as i32
                        ];
                        scale_parameters {
                            scale_factor,
                            x_padding,
                            y_padding,
                            device_fb_rect,
                            cropped_vnc_fb_rect,
                            original_vnc_fb_rect,
                            scaled_fb_rect,
                        }
                    } else {
                        // fit_width = true;
                        scale_factor = fb_width as f32 / width as f32;
                        y_padding = ((fb_height - (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as u32) / 2) as u32;
                        x_padding = 0;
                        scaled_fb_rect = rect![
                            0 + x_padding as i32,
                            0 + y_padding as i32,
                            (width as f32 * scale_factor) as i32,// + x_padding as i32,
                            (height as f32 * scale_factor).clamp(0.0, fb_height as f32) as i32 + y_padding as i32
                        ];
                        scale_parameters {
                            scale_factor,
                            x_padding,
                            y_padding,
                            device_fb_rect,
                            cropped_vnc_fb_rect,
                            original_vnc_fb_rect,
                            scaled_fb_rect,
                        }
                    }
                }
            }
        } else {
            if width < fb_width as u16 {
                x_padding = ((fb_width - width as u32) / 2) as u32
            }; //width should always be smaller than or equal to fb width
            if height < fb_height as u16 {
                y_padding = ((fb_height - height as u32) / 2) as u32; //if its bigger, it would fail anyway?
            };
            if width > fb_width as u16 {
                cropped_vnc_fb_rect = rect![
                    0 + x_padding as i32 + x_offset as i32,
                    0 + y_padding as i32 + y_offset as i32,
                    fb_width as i32 + x_padding as i32 + x_offset as i32,
                    fb_height as i32 + y_padding as i32 + y_offset as i32
                ];
            } else if height > fb_height as u16 {
                cropped_vnc_fb_rect = rect![
                    0 + x_padding as i32 + x_offset as i32,
                    0 + y_padding as i32 + y_offset as i32,
                    fb_width as i32 + x_padding as i32 + x_offset as i32,
                    fb_height as i32 + y_padding as i32 + y_offset as i32
                ];
            } else if width > fb_width as u16 && height > fb_height as u16 {
                cropped_vnc_fb_rect = rect![
                    0 + x_offset as i32,
                    0 + y_offset as i32,
                    fb_width as i32 + x_offset as i32,
                    fb_height as i32 + y_offset as i32
                ];
            }
            scale_parameters {
                scale_factor,
                x_padding,
                y_padding,
                device_fb_rect,
                cropped_vnc_fb_rect,
                original_vnc_fb_rect,
                scaled_fb_rect,
            }
        }
    }
}

