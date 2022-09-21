use std::cell::RefCell;
use std::rc::Rc;

use log::info;

use wayland_client::{
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
    Display, EventQueue, GlobalManager, Main,
};
use wayland_protocols::wlr::unstable::{
    layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1},
    virtual_pointer::v1::client::{zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1},
};

use xkbcommon::xkb;

use crate::config::{Config, KeynavAction};
use crate::render::RenderManager;

// Need to separate [App.DataData] and [App.Data] so that we can borrow the event queue
// mutably to dispatch events without simultaneously borrowing the rest of the
// app.data
struct App {
    config: Config,
    pointer_pos: (i32, i32),
    keyboard_state: Option<xkb::State>,
    should_exit: bool,
    renderer: Rc<RefCell<RenderManager>>,
    surface: Main<wl_surface::WlSurface>,
    pool: Main<wl_shm_pool::WlShmPool>,
    buffer: Main<wl_buffer::WlBuffer>,
}

impl App {
    pub fn init(config: Config, event_queue: &mut EventQueue) -> Result<Rc<RefCell<Self>>, String> {
        let attached_display = (event_queue.display()).clone().attach(event_queue.token());

        let globals = GlobalManager::new(&attached_display);

        // Make a synchronized roundtrip to the wayland server.
        //
        // When this returns it must be true that the server has already
        // sent us all available globals.
        info!("Recieving globals");
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .unwrap();

        let compositor = globals
            .instantiate_exact::<wl_compositor::WlCompositor>(4)
            .unwrap();
        let surface = compositor.create_surface();

        let layer_shell = globals
            .instantiate_exact::<zwlr_layer_shell_v1::ZwlrLayerShellV1>(4)
            .expect("Compositor does not support zwlr_layer_shell_v1");
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "keynav".to_string(),
        );
        layer_surface.set_size(0, 0);
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );

        layer_surface.set_exclusive_zone(-1);
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);

        info!("Initial commit of surface (trigger configure)");
        surface.commit();

        info!("Creating renderer");
        let renderer: Rc<RefCell<_>> = Rc::new(RefCell::new(
            RenderManager::init(cairo::Format::ARgb32, 100, 100).unwrap(),
        ));

        let shm = globals.instantiate_exact::<wl_shm::WlShm>(1).unwrap();
        let pool = shm.create_pool(
            renderer.borrow().get_shm_fd(),
            (renderer.borrow().get_buf_size()) as i32,
        );
        let buffer = pool.create_buffer(
            0,
            renderer.borrow().get_width() as i32,
            renderer.borrow().get_height() as i32,
            renderer.borrow().get_stride(),
            wl_shm::Format::Argb8888,
        );

        let app = Rc::new(RefCell::new(App {
            config,
            keyboard_state: None,
            pointer_pos: (0, 0),
            should_exit: false,
            renderer,
            surface,
            pool,
            buffer,
        }));

        {
            // Need to start listening to keyboard events as soon as we create the layer_surface otherwise we don't gain focus immediately
            let app = app.clone();
            let renderer = app.borrow().renderer.clone();
            layer_surface.quick_assign(move |layer_surface, event, _| match event {
                zwlr_layer_surface_v1::Event::Configure {
                    width,
                    height,
                    serial,
                } => {
                    info!("Configure: {}x{}", width, height);
                    renderer.borrow_mut().set_bounds(width, height).unwrap();
                    layer_surface.ack_configure(serial);
                    app.borrow_mut().rebind();
                }
                _ => (),
            });
        }

        event_queue
            .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .unwrap();

        {
            let virtual_pointer_manager = globals
                .instantiate_exact::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>(
                    2,
                )
                .expect("Compositor should support virtual pointer");
            let app = app.clone();
            let mut keyboard_created = false;
            let mut pointer_created = false;
            globals
                .instantiate_exact::<wl_seat::WlSeat>(1)
                .unwrap()
                .quick_assign(move |seat, event, _| {
                    // The capabilities of a seat are known at runtime and we retrieve
                    // them via an events. 3 capabilities exists: pointer, keyboard, and touch
                    // we are only interested in pointer & keyboard here
                    use wayland_client::protocol::wl_seat::{Capability, Event as SeatEvent};

                    let virtual_pointer =
                        virtual_pointer_manager.create_virtual_pointer(Some(&seat));
                    let region = compositor.create_region();
                    let app = app.clone();
                    if let SeatEvent::Capabilities { capabilities } = event {
                        if !pointer_created && capabilities.contains(Capability::Pointer) {
                            let app = app.clone();
                            pointer_created = true;
                            seat.get_pointer().quick_assign(
                                move |_pointer, event, _| match event {
                                    wl_pointer::Event::Enter {
                                        surface_x,
                                        surface_y,
                                        ..
                                    } => {
                                        info!("Pointer entered at {}, {}", surface_x, surface_y);
                                        let mut app = app.borrow_mut();
                                        app.surface.set_input_region(Some(&region));
                                        app.pointer_pos = (surface_x as i32, surface_y as i32);
                                    }
                                    _ => {}
                                },
                            );
                        };
                    }
                    if let SeatEvent::Capabilities { capabilities } = event {
                        if !keyboard_created && capabilities.contains(Capability::Keyboard) {
                            // create the keyboard only once
                            keyboard_created = true;
                            seat.get_keyboard()
                                .quick_assign(move |_keyboard, event, _| {
                                    app.borrow_mut()
                                        .handle_keyboard_event(&virtual_pointer, event);
                                });
                        };
                    }
                });
        }

        {
            event_queue
                .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
                .unwrap();
        }

        {
            let app = app.borrow();
            app.surface.attach(Some(&app.buffer), 0, 0);
            app.surface.commit();
        }

        {
            event_queue
                .sync_roundtrip(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
                .unwrap();
        }
        Ok(app)
    }
    pub fn rebind(&mut self) {
        info!("Rebinding");
        self.pool
            .resize(self.renderer.borrow().get_buf_size() as i32);
        self.buffer.destroy();
        self.buffer = self.pool.create_buffer(
            0,
            self.renderer.borrow().get_width() as i32,
            self.renderer.borrow().get_height() as i32,
            self.renderer.borrow().get_stride(),
            wl_shm::Format::Argb8888,
        );
    }

    pub fn commit(&self) {
        info!("Commiting");

        self.surface.attach(Some(&self.buffer), 0, 0);
        self.surface.damage_buffer(
            0,
            0,
            self.renderer.borrow().get_width() as i32,
            self.renderer.borrow().get_height() as i32,
        );
        self.surface.commit();
    }
    pub fn exit(&mut self) {
        self.should_exit = true;
    }
    pub fn click(&mut self, virtual_pointer: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1) {
        let rect = self.renderer.borrow().get_active_region();
        // We scale by extent because
        // motion_absolute takes ints, but
        // we have normalized scalar coords
        // so we are bsaically just
        // converting the float to a fixed
        // point with 4 decimal places
        let extent = 10000;
        virtual_pointer.motion_absolute(
            0,
            ((rect.x + rect.width / 2.0) * (extent as f64)) as u32,
            ((rect.y + rect.height / 2.0) * (extent as f64)) as u32,
            extent,
            extent,
        );
        virtual_pointer.frame();
        virtual_pointer.button(0, 272, wl_pointer::ButtonState::Pressed);
        virtual_pointer.frame();
        virtual_pointer.button(0, 272, wl_pointer::ButtonState::Released);
        virtual_pointer.frame();
    }
    pub fn center_cursor(&mut self) {
        let mut renderer = self.renderer.borrow_mut();
        let (pointer_surface_x, pointer_surface_y) = self.pointer_pos;
        let pointer_relative_x = (pointer_surface_x as f64) / (renderer.get_width() as f64);
        let pointer_relative_y = (pointer_surface_y as f64) / (renderer.get_height() as f64);

        let (width, height) = renderer.device_to_user(100.0, 100.0);
        renderer.update_active_region(cairo::Rectangle {
            x: pointer_relative_x - width / 2.0,
            y: pointer_relative_y - height / 2.0,
            height: height,
            width: width,
        });
    }
    pub fn narrow_left(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x,
                y: rect.y,
                height: rect.height,
                width: rect.width / 2.0,
            });
    }
    pub fn narrow_down(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x,
                y: rect.y + rect.height / 2.0,
                height: rect.height / 2.0,
                width: rect.width,
            });
    }

    pub fn narrow_up(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x,
                y: rect.y,
                height: rect.height / 2.0,
                width: rect.width,
            });
    }

    pub fn narrow_right(&mut self) {
        let rect = self.renderer.borrow_mut().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x + rect.width / 2.0,
                y: rect.y,
                height: rect.height,
                width: rect.width / 2.0,
            });
    }

    pub fn move_right(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x + rect.width,
                y: rect.y,
                height: rect.height,
                width: rect.width,
            });
    }
    pub fn move_left(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x - rect.width,
                y: rect.y,
                height: rect.height,
                width: rect.width,
            });
    }
    pub fn move_up(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x,
                y: rect.y - rect.height,
                height: rect.height,
                width: rect.width,
            });
    }
    pub fn move_down(&mut self) {
        let rect = self.renderer.borrow().get_active_region();
        self.renderer
            .borrow_mut()
            .update_active_region(cairo::Rectangle {
                x: rect.x,
                y: rect.y + rect.height,
                height: rect.height,
                width: rect.width,
            });
    }
    pub fn handle_keyboard_event(
        &mut self,
        virtual_pointer: &Main<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1>,
        event: wl_keyboard::Event,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                info!("Got keymap");
                match format {
                    wl_keyboard::KeymapFormat::XkbV1 => {
                        let maybe_keymap_or_err = unsafe {
                            xkb::Keymap::new_from_fd(
                                &xkb::Context::new(xkb::CONTEXT_NO_FLAGS),
                                fd,
                                size as usize,
                                xkb::KEYMAP_FORMAT_TEXT_V1,
                                xkb::COMPILE_NO_FLAGS,
                            )
                        };
                        match maybe_keymap_or_err {
                            Ok(Some(keymap)) => {
                                self.keyboard_state = Some(xkb::State::new(&keymap));
                            }
                            _ => {}
                        }
                    }

                    wl_keyboard::KeymapFormat::NoKeymap => {}
                    _ => {}
                }
            }
            wl_keyboard::Event::Enter { .. } => {
                info!("Gained keyboard focus.");
            }
            wl_keyboard::Event::Leave { .. } => {
                info!("Lost keyboard focus.");
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                // Lots of xkbcommon stuff sanity checked against wev
                match self.keyboard_state.clone() {
                    Some(mut keyboard_state) => {
                        keyboard_state.update_mask(
                            mods_depressed,
                            mods_latched,
                            mods_locked,
                            0,
                            0,
                            group,
                        );
                    }
                    None => {}
                };
            }
            wl_keyboard::Event::Key { key, state, .. } => {
                info!("Key with id {} was {:?}.", key, state);
                // TODO: Learn how xkbcommon actually works?
                let key = match self.keyboard_state.clone() {
                    Some(mut keyboard_state) => {
                        // Docs suggest getting key before updating
                        let key = keyboard_state.key_get_one_sym(key + 8);
                        info!("Key maps to {}", key);
                        keyboard_state.update_key(
                            key + 8, /* wayland docs told me to? */
                            match state {
                                wl_keyboard::KeyState::Pressed => xkb::KeyDirection::Down,
                                wl_keyboard::KeyState::Released => xkb::KeyDirection::Up,
                                _ => panic!("THIS SHOULD BE EXHAUSTIVE"),
                            },
                        );
                        key
                    }
                    None => key,
                };
                // TODO: Handle keymap
                // TODO: Maybe handle press vs relase
                if state == wl_keyboard::KeyState::Pressed {
                    let mappings = &self.config.mappings.clone();
                    match mappings.get(&key) {
                        Some(actions) => {
                            actions.iter().for_each(|action| match action {
                                KeynavAction::CenterCursor => {
                                    info!("Executing CenterCursor action");
                                    self.center_cursor();
                                }
                                KeynavAction::NarrowRight => {
                                    info!("Executing NarrowRight action");
                                    self.narrow_right();
                                }
                                KeynavAction::NarrowLeft => {
                                    info!("Executing NarrowLeft action");
                                    self.narrow_left();
                                }
                                KeynavAction::NarrowUp => {
                                    info!("Executing NarrowUp action");
                                    self.narrow_up();
                                }
                                KeynavAction::NarrowDown => {
                                    info!("Executing NarrowDown action");
                                    self.narrow_down();
                                }
                                KeynavAction::MoveRight => {
                                    info!("Executing MoveRight action");
                                    self.move_right();
                                }
                                KeynavAction::MoveLeft => {
                                    info!("Executing MoveLeft action");
                                    self.move_left();
                                }
                                KeynavAction::MoveUp => {
                                    info!("Executing MoveUp action");
                                    self.move_up();
                                }
                                KeynavAction::MoveDown => {
                                    info!("Executing MoveDown action");
                                    self.move_down();
                                }
                                KeynavAction::Click => {
                                    info!("Executing click action");
                                    self.click(&virtual_pointer);
                                }
                                KeynavAction::Exit => {
                                    info!("Executing exit action");
                                    self.exit();
                                }
                            });
                        }
                        None => {
                            info!("No actions associated with key")
                        }
                    }
                }
                self.renderer.borrow_mut().redraw().unwrap();
                self.commit();
            }
            _ => (),
        }
    }
}

pub struct AppRunner {
    app: Rc<RefCell<App>>,
    event_queue: EventQueue,
}
impl AppRunner {
    pub fn init(config: Config) -> Result<Self, String> {
        info!("Connecting to server");
        let display = Display::connect_to_env().unwrap();

        let mut event_queue = display.create_event_queue();

        let app = App::init(config, &mut event_queue)?;

        Ok(AppRunner {
            app: app,
            event_queue,
        })
    }
    pub fn pump(&mut self) -> bool {
        self.event_queue
            .dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
            .unwrap();
        if self.app.borrow().should_exit {
            self.event_queue
                .dispatch(&mut (), |_, _, _| { /* we ignore unfiltered messages */ })
                .unwrap();
            false
        } else {
            true
        }
    }
}
