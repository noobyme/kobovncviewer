use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use crate::device::CURRENT_DEVICE;
use crate::framebuffer::{Framebuffer, UpdateMode};
use crate::font::Fonts;
use crate::geom::{Rectangle, halves, Dir};
use crate::gesture::GestureEvent;
use crate::color::{BLACK, WHITE};
use crate::unit::scale_by_dpi;
use crate::context::Context;
use crate::input::{ButtonStatus, DeviceEvent};
use crate::view::{View, Event, Hub, Bus, Id, ID_FEEDER, RenderQueue, RenderData, ViewId, Align, SMALL_BAR_HEIGHT, BIG_BAR_HEIGHT, THICKNESS_MEDIUM, EntryId, EntryKind};

use crate::view::filler::Filler;
use crate::view::label::Label;
use crate::view::button::Button;
use crate::view::input_field::InputField;
use crate::view::keyboard::Keyboard;
use crate::view::common::locate;
use crate::view::menu::{Menu, MenuKind};
use crate::view::output_field::OutputField;
use crate::view::password_field::Password_Field;
use crate::view::toggle_button::Toggle_Button;

pub struct Gui {
    id: Id,
    rect: Rectangle,
    filled_rect: Rectangle,
    pub children: Vec<Box<dyn View>>,
    focus: Option<ViewId>,
    o_menu_rect: Rectangle,
    w_menu_rect: Rectangle,
}

impl Gui {
    pub fn new(rect: Rectangle, hub: &Hub, rq: &mut RenderQueue,
               host: &Option<&str>, port:&u16, username:&Option<String>, password:&Option<String>,
               encoding_enabled:bool, scaling_enabled:bool,
               context: &mut Context, panning:bool, disable_touch:bool) -> Gui  {
        let id = ID_FEEDER.next();
        // let mut children: Vec<Box<dyn View>> = Vec::new();
        let mut children = Vec::new();

        let dpi = CURRENT_DEVICE.dpi;
        let small_height = scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32;//((121*212)/300).round().max(1)=85.5
        let big_height   = scale_by_dpi(BIG_BAR_HEIGHT,   dpi) as i32;//163*212/300.round().max(1)=115
        let thickness   = scale_by_dpi(THICKNESS_MEDIUM,  dpi) as i32;//23*212/300.round().max(1) /2 =0.1=>1
        let (small_thickness, big_thickness) = halves(thickness);//1,1

        let label_width   = rect.width() as i32 / 4; //758/4=189.5 =190
        let fb_third = rect.width() as i32 /3;

        //Background
        children.push(Box::new(Filler::new(
            rect![
                rect.min.x, //0
                rect.min.y, //0
                rect.max.x,
                rect.max.y ],
            WHITE,
        )) as Box<dyn View>);

        children.push(Box::new(
            InputField::new(
                rect![
                    rect.min.x, //0
                    rect.min.y, //0
                    rect.max.x/2,  //758
                    rect.min.y + small_height - small_thickness],//85
                ViewId::GuiInputField1,
            )
                .border(true)
                .placeholder("Enter host IP")
                .text(host.unwrap_or(""), context),
        ) as Box<dyn View>);
        children.push(Box::new(
            InputField::new(
                rect![
                    rect.max.x/2,
                    rect.min.y, //0
                    rect.max.x,  //758
                    rect.min.y + small_height - small_thickness],//85
                ViewId::GuiInputField2,
            )
                .border(true)
                .placeholder("Enter port")
                .text(&*port.to_string(), context),
        ) as Box<dyn View>);
        
        let mut row_height = rect.min.y + small_height + small_thickness;

        // ── 3  row separator ──────────────────────────────────────────
        children.push(Box::new(Filler::new(
            rect![
                rect.min.x,
                rect.min.y + small_height - small_thickness,
                rect.max.x,
                rect.min.y + small_height + small_thickness],
            BLACK,
        )) as Box<dyn View>);

        // ) as Box<dyn View>);
        children.push(Box::new(
            InputField::new(
                rect![
                    rect.min.x,
                    row_height,
                    rect.max.x/2,
                    row_height + small_height - small_thickness],
                ViewId::GuiInputField3,
            )
                .border(true)
                .placeholder("Enter username")
                .text(&*username.clone().unwrap_or("".to_string()), context),
        ) as Box<dyn View>);
        children.push(Box::new(
            Password_Field::new(
                rect![
                    rect.max.x/2,
                    row_height,
                    rect.max.x,
                    row_height + small_height - small_thickness],
                ViewId::GuiPasswordField1,
            )
                .border(true)
                .placeholder("Enter password")
                .text(&*password.clone().unwrap_or("".to_string()), context),
        ) as Box<dyn View>);

        row_height = row_height + small_height + small_thickness;

        // ── 7  separator below rows ───────────────────────────────────
        children.push(Box::new(Filler::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x,
                row_height + small_height - small_thickness],
            BLACK,
        )) as Box<dyn View>);

        children.push(Box::new(OutputField::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness],
            "Console Output".to_string(), Align::Center
        )) as Box<dyn View>);

        row_height = row_height + small_height + small_thickness;

        // ── 8  Toggle button ─────────────────────────────────────────
        children.push(Box::new(Button::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x/2,
                row_height + small_height - small_thickness],
            Event::Toggle(ViewId::VNC),
            "Start VNC".to_string(),
        )) as Box<dyn View>);

        children.push(Box::new(Button::new(
            rect![
                rect.max.x/2,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness],
            Event::Back,
            "Quit".to_string(),
        )) as Box<dyn View>);

        row_height = row_height + small_height + small_thickness;

        children.push(Box::new(Button::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x/2,
                row_height + small_height + small_thickness],
            Event::Toggle(ViewId::OrientationMenu),
            "Orientation".to_string()
        )) as Box<dyn View>);

        let o_menu_rect =rect![
                rect.min.x,
                row_height,
                rect.max.x/2,
                row_height + small_height + small_thickness];

        children.push(Box::new(Button::new(
            rect![
                rect.max.x/2,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness],
            Event::Toggle(ViewId::WaveformMenu),
            "Waveform Menu".to_string()
        )) as Box<dyn View>);

        let w_menu_rect =rect![
                rect.max.x/2,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness];

        // children.push(Box::new(Label::new(
        //     rect![
        //         rect.min.x,
        //         row_height,
        //         rect.max.x/2,
        //         row_height + small_height + small_thickness],
        //     "Orientation".to_string(),
        //     Align::Center
        // ).event(Some(Event::Toggle(ViewId::OrientationMenu)))
        // ) as Box<dyn View>);
        //
        // let o_menu_rect =rect![
        //         rect.min.x,
        //         row_height,
        //         rect.max.x/2,
        //         row_height + small_height + small_thickness];
        //
        // children.push(Box::new(Label::new(
        //     rect![
        //         rect.max.x/2,
        //         row_height,
        //         rect.max.x,
        //         row_height + small_height + small_thickness],
        //     "Waveform Menu".to_string(),
        //     Align::Center
        // ).event(Some(Event::Toggle(ViewId::WaveformMenu)))
        // ) as Box<dyn View>);
        //
        // let w_menu_rect =rect![
        //         rect.max.x/2,
        //         row_height,
        //         rect.max.x,
        //         row_height + small_height + small_thickness];

        row_height = row_height + small_height + small_thickness;

        children.push(Box::new(Toggle_Button::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x/2,
                row_height + small_height + small_thickness],
            scaling_enabled,
            Event::Toggle(ViewId::Scaling),
            "Enable scaling".to_string(),
        )) as Box<dyn View>);

        children.push(Box::new(Toggle_Button::new(
            rect![
                rect.max.x/2,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness],
            encoding_enabled,
            Event::Toggle(ViewId::Encoding),
            "Custom encoding".to_string(),
        )) as Box<dyn View>);

        row_height = row_height + small_height + small_thickness;

        children.push(Box::new(Toggle_Button::new(
            rect![
                rect.min.x,
                row_height,
                rect.max.x/2,
                row_height + small_height + small_thickness],
            disable_touch,
            Event::Toggle(ViewId::Touch),
            "Disable Touch".to_string(),
        )) as Box<dyn View>);

        children.push(Box::new(Toggle_Button::new(
            rect![
                rect.max.x/2,
                row_height,
                rect.max.x,
                row_height + small_height + small_thickness],
            panning,
            Event::Toggle(ViewId::Panning),
            "Enable Panning".to_string(),
        )) as Box<dyn View>);

        row_height = row_height + small_height + small_thickness;

        let filled_rect = rect![
                rect.min.x, //0
                rect.min.y + small_height + small_thickness + small_height + small_thickness, //just below the input fields
                rect.max.x,
                rect.max.y - 3*big_height + 2*big_thickness]; //above the keyboard?

        rq.add(RenderData::new(id, rect, UpdateMode::Full));

        Gui {
            id,
            rect,
            filled_rect,
            children,
            focus: None,
            o_menu_rect: o_menu_rect,
            w_menu_rect: w_menu_rect,
        }
    }

    fn toggle_keyboard(&mut self, enable: bool, _id: Option<ViewId>, hub: &Hub, rq: &mut RenderQueue, context: &mut Context) {
        if let Some(index) = locate::<Keyboard>(self) { //<Keyboard(Dictionary?)> supply dictionary to be searched,
            //and keyboard as the type to be searched?
            if enable {
                return;
            } //if can find keyboard in current views children?, if enable is true, exit function otherwise this block will turn off keyboard

            let mut rect = *self.child(index).rect(); //dictionary children, the keyboard index
            rect.absorb(self.child(index - 1).rect()); //if child rect is bigger than the parent rect, use parent? rect?
            //apparently it unions the rects
            // fn child(&self, index: usize) -> &dyn View {
            //     self.children()[index].as_ref()
            // }

            self.children.drain(index - 1..=index); //remove the keyboard view?

            // context.kb_rect = Rectangle::default(); //0,0,0,0 rect
            rq.add(RenderData::expose(rect, UpdateMode::Gui)); //expose sets view id=none
            hub.send(Event::Focus(None)).ok(); //unfocus?
        } else { //if cant find keyboard
            if !enable { //if passed in false as input paramter
                return;
            }
            //turn kb on
            let dpi = CURRENT_DEVICE.dpi;
            let (small_height, big_height) = (scale_by_dpi(SMALL_BAR_HEIGHT, dpi) as i32,
                                              scale_by_dpi(BIG_BAR_HEIGHT, dpi) as i32);
            let thickness = scale_by_dpi(THICKNESS_MEDIUM, dpi) as i32;
            let (small_thickness, big_thickness) = halves(thickness);

            let mut kb_rect = rect![self.rect.min.x,
                                        self.rect.max.y - 3*big_height+2*big_thickness/* - (small_height + 3 * big_height) as i32 + big_thickness*/,
                                        self.rect.max.x,
                                        self.rect.max.y /*- small_height - small_thickness*/];
            //every time, kb rect pre calculated b4 call new. not use fb rect

            // let number = id == Some(ViewId::GoToPageInput); //if id ==gotopageinput??
            // let index = locate::<BottomBar>(self).unwrap() + 1; //put it after bottom bar?
            // let index = locate::<BottomBar>(self).unwrap() + 1;

            let separator = Filler::new(rect![self.rect.min.x, kb_rect.min.y - thickness,
                                                  self.rect.max.x, kb_rect.min.y],
                                        BLACK);
            self.children.push(Box::new(separator) as Box<dyn View>);

            let keyboard = Keyboard::new(&mut kb_rect, false, context);
            self.children.push(Box::new(keyboard) as Box<dyn View>);

            //final black line? i see, no this is the 3rd black line, disappears with keyboard toggles

            for i in (self.children.len()-2)..self.children.len() {
                rq.add(RenderData::new(self.child(i).id(), *self.child(i).rect(), UpdateMode::Gui));
            }
        }
    }
}

impl View for Gui {
    fn handle_event(&mut self, evt: &Event, hub: &Hub, bus: &mut Bus, rq: &mut RenderQueue, context: &mut Context) -> bool {
        match *evt {
            // Track which field is active so keyboard re-focuses it on re-open
            //focus none is only sent when keyboard is untoggled...
            //only input fields send focus events... so if keep focus field none by default
            Event::Gesture(GestureEvent::Tap(center)) if self.filled_rect.includes(center)=> {
                self.focus == None;
                self.toggle_keyboard(false, self.focus, hub, rq, context);
                true
            },
            Event::Toggle(ViewId::OrientationMenu) => {
                let entries = vec![
                    EntryKind::Command(
                        "Portrait".to_string(),
                        EntryId::Portrait, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "Landscape".to_string(),
                        EntryId::Landscape, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "Inverted Portrait".to_string(),
                        EntryId::IPortrait, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "Inverted Landscape".to_string(),
                        EntryId::ILandscape, // any EntryId variant
                    ),];

                self.children.push(Box::new(Menu::new(
                    self.o_menu_rect,                    // rectangle to anchor the menu to
                    ViewId::OrientationMenu, // unique id for this menu
                    MenuKind::DropDown,
                    entries,                 // Vec<EntryKind>
                    context,
                )) as Box<dyn View>);
                rq.add(RenderData::new(self.id, self.o_menu_rect, UpdateMode::Gui));
                true
            },
            Event::Select(ref id ) => {
                match id {
                    EntryId::Portrait => {
                        // hub.send(Event::Portrait).ok();
                        hub.send(Event::Select(EntryId::Portrait)).ok();
                    },
                    EntryId::IPortrait => {
                        // hub.send(Event::IPortrait).ok();
                        hub.send(Event::Select(EntryId::IPortrait)).ok();
                    },
                    EntryId::Landscape => {
                        // hub.send(Event::Landscape).ok();
                        hub.send(Event::Select(EntryId::Landscape)).ok();
                    },
                    EntryId::ILandscape => {
                        // hub.send(Event::ILandscape).ok();
                        hub.send(Event::Select(EntryId::ILandscape)).ok();
                    },
                    _ => {},
                }
                true
            },
            Event::Toggle(ViewId::WaveformMenu) => {
                let entries = vec![
                    EntryKind::Command(
                        "FastA2".to_string(),
                        EntryId::FastA2, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "FastMonoA2".to_string(),
                        EntryId::FastMonoA2, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "GuiDU".to_string(),
                        EntryId::GuiDU, // any EntryId variant
                    ),
                    EntryKind::Command(
                        "PartialGL16".to_string(),
                        EntryId::PartialGL16, // any EntryId variant
                    ),];

                self.children.push(Box::new(Menu::new(
                    self.w_menu_rect,                    // rectangle to anchor the menu to
                    ViewId::WaveformMenu, // unique id for this menu
                    MenuKind::DropDown,
                    entries,                 // Vec<EntryKind>
                    context,
                )) as Box<dyn View>);
                rq.add(RenderData::new(self.id, self.w_menu_rect, UpdateMode::Gui));
                true
            },
            Event::Select(ref id ) => {
                match id {
                    EntryId::FastA2 => {
                        hub.send(Event::Select(EntryId::FastA2)).ok();
                    },
                    EntryId::FastMonoA2 => {
                        hub.send(Event::Select(EntryId::FastMonoA2)).ok();
                    },
                    EntryId::GuiDU => {
                        hub.send(Event::Select(EntryId::GuiDU)).ok();    
                    },
                    EntryId::PartialGL16 => {
                        hub.send(Event::Select(EntryId::PartialGL16)).ok();       
                    },
                    _ => {},
                }
                true
            },
            Event::Gesture(GestureEvent::Swipe { dir, start, .. }) if self.rect.includes(start) => {
                match dir {
                    Dir::South => self.toggle_keyboard(false, self.focus, hub, rq, context),
                    _ => (),
                }
                true
            },
            Event::Device(DeviceEvent::Button { code, status: ButtonStatus::Pressed, .. }) => {
                hub.send(Event::Back).ok();
                true
            },
            Event::Submit(ViewId::GuiInputField1, ref text) => {
                if !text.is_empty() {
                    // self.toggle_keyboard(false, None, hub, rq, context);
                    hub.send(Event::Submit(ViewId::GuiInputField1, text.clone())).ok();
                }
                true
            },
            Event::Submit(ViewId::GuiInputField2, ref text) => {
                if !text.is_empty() {
                    // self.toggle_keyboard(false, None, hub, rq, context);;
                    hub.send(Event::Submit(ViewId::GuiInputField2, text.clone())).ok();
                }
                true
            },
            Event::Submit(ViewId::GuiInputField3, ref text) => {
                if !text.is_empty() {
                    // self.toggle_keyboard(false, None, hub, rq, context);
                    hub.send(Event::Submit(ViewId::GuiInputField3, text.clone())).ok();
                }
                true
            },
            Event::Submit(ViewId::GuiPasswordField1, ref text) => {
                if !text.is_empty() {
                    // self.toggle_keyboard(false, None, hub, rq, context);
                    hub.send(Event::Submit(ViewId::GuiPasswordField1, text.clone())).ok();
                }
                true
            },
            Event::Focus(v) => {
                self.focus = v;
                if v.is_some() {
                    self.toggle_keyboard(true, v, hub, rq, context);
                }
                true
            },
            Event::Gesture(GestureEvent::Cross(_)) => {
                hub.send(Event::Back).ok();
                true
            },
            _ => false,
        }
    }

    fn render(&self, _fb: &mut dyn Framebuffer, _rect: Rectangle, _fonts: &mut Fonts) {
    }


    fn is_background(&self) -> bool { true }

    fn rect(&self)         -> &Rectangle              { &self.rect }
    fn rect_mut(&mut self) -> &mut Rectangle          { &mut self.rect }
    fn children(&self)     -> &Vec<Box<dyn View>>     { &self.children }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn View>> { &mut self.children }
    fn id(&self)           -> Id                      { self.id }
}

