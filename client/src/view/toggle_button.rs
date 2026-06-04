use crate::device::CURRENT_DEVICE;
use crate::geom::{Rectangle, CornerSpec, BorderSpec};
use crate::font::{Fonts, font_from_style, NORMAL_STYLE};
use super::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData};
use super::{THICKNESS_MEDIUM, BORDER_RADIUS_LARGE};
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::input::{DeviceEvent, FingerStatus};
use crate::gesture::GestureEvent;
use crate::color::{TEXT_NORMAL, TEXT_INVERTED_HARD};
use crate::unit::scale_by_dpi;
use crate::context::Context;

pub struct Toggle_Button {
    id: Id,
    rect: Rectangle,
    children: Vec<Box<dyn View>>,
    event: Event,
    text: String,
    active: bool,
    pub disabled: bool,
}

impl Toggle_Button {
    pub fn new(rect: Rectangle, previous_state:bool ,event: Event, text: String) -> Toggle_Button {
        Toggle_Button {
            id: ID_FEEDER.next(),
            rect,
            children: Vec::new(),
            event,
            text,
            active: previous_state,
            disabled: false,
        }
    }

    pub fn disabled(mut self, value: bool) -> Toggle_Button {
        self.disabled = value;
        self
    }
}

impl View for Toggle_Button {
    fn handle_event(&mut self, evt: &Event, _hub: &Hub, bus: &mut Bus, rq: &mut RenderQueue, _context: &mut Context) -> bool {
        match *evt {
            Event::Device(DeviceEvent::Finger { status, position, .. }) if !self.disabled => {
                match status {
                    FingerStatus::Down if self.rect.includes(position) => {
                        self.active = !self.active;
                        rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                        true
                    },
                    FingerStatus::Up if self.active  && self.rect.includes(position)=> {
                        // self.active = !self.active;
                        rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                        true
                        //active field determines if pressed or not, but it does not flash eh?
                    },
                    FingerStatus::Up if self.active  && !self.rect.includes(position)=> {
                        // self.active = !self.active;
                        rq.add(RenderData::new(self.id, self.rect, UpdateMode::Gui));
                        false
                        //active field determines if pressed or not, but it does not flash eh?
                    },
                    _ => false,
                }
            },//i see use fingerevent instead of tap bc tap cant tell when lift or down, only sent once.
            Event::Gesture(GestureEvent::Tap(center)) if self.rect.includes(center) => {
                if !self.disabled {
                    bus.push_back(self.event.clone());
                }
                true
            },
            _ => false,
        }
    }

    fn render(&self, fb: &mut dyn Framebuffer, _rect: Rectangle, fonts: &mut Fonts) {
        let dpi = CURRENT_DEVICE.dpi;

        let scheme = if self.active {
            TEXT_INVERTED_HARD
        } else {
            TEXT_NORMAL
        };
        let foreground = if self.disabled { scheme[2] } else { scheme[1] };

        let border_radius = scale_by_dpi(BORDER_RADIUS_LARGE, dpi) as i32;
        let border_thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as u16;

        fb.draw_rounded_rectangle_with_border(&self.rect,
                                              &CornerSpec::Uniform(border_radius),
                                              &BorderSpec { thickness: border_thickness,
                                                            color: foreground },
                                              &scheme[0]);

        let font = font_from_style(fonts, &NORMAL_STYLE, dpi);
        let x_height = font.x_heights.0 as i32;
        let padding = font.em() as i32;
        let max_width = self.rect.width() as i32 - padding;

        let plan = font.plan(&self.text, Some(max_width), None);

        let dx = (self.rect.width() as i32 - plan.width) / 2;
        let dy = (self.rect.height() as i32 - x_height) / 2;
        let pt = pt!(self.rect.min.x + dx, self.rect.max.y - dy);

        font.render(fb, foreground, &plan, pt);
    }

    fn rect(&self) -> &Rectangle {
        &self.rect
    }

    fn rect_mut(&mut self) -> &mut Rectangle {
        &mut self.rect
    }

    fn children(&self) -> &Vec<Box<dyn View>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> {
        &mut self.children
    }

    fn id(&self) -> Id {
        self.id
    }
}
