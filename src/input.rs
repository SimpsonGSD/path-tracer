
use winit::event::{Event, VirtualKeyCode, DeviceEvent, KeyboardInput, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct AppUserInputState {
    pub keys_held: HashSet<VirtualKeyCode>,
    pub grabbed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UserInput {
    pub exit_requested: bool,
    pub new_frame_size: Option<(f32, f32)>,
    pub keys_held: HashSet<VirtualKeyCode>,
    pub keys_pressed: HashSet<VirtualKeyCode>,
    pub mouse_delta: (f32, f32)
}

impl UserInput {
    pub fn poll_events_loop<T: 'static>(
        events_loop: &mut EventLoop<T>,
        window: &mut Window, 
        app_user_input_state: &mut AppUserInputState
    ) -> Self {
        let mut output = UserInput::default();
        // we have to manually split the borrow here
        let keys_held = &mut app_user_input_state.keys_held;
        let keys_held_prev = keys_held.clone();
        let grabbed = &mut app_user_input_state.grabbed;

        use winit::platform::desktop::EventLoopExtDesktop;
        events_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Exit;
            
            match event {

                // Close when asked
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => output.exit_requested = true,

                // Track all keys, all the time. Note that because of key rollover details
                // it's possible to get key released events for keys we don't think are
                // pressed. This is a hardware limit, not something you can evade.
                Event::DeviceEvent {
                    event:
                        DeviceEvent::Key(KeyboardInput {
                            virtual_keycode: Some(code),
                            state,
                            ..
                        }),
                    ..
                } => drop(match state {
                    ElementState::Pressed => keys_held.insert(code),
                    ElementState::Released => keys_held.remove(&code),
                }),

                // We want to respond to some of the keys specially when they're also
                // window events too (meaning that the window was focused when the event
                // happened).
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state,
                                    virtual_keycode: Some(code),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    #[cfg(feature = "metal")]
                    {
                        match state {
                            ElementState::Pressed => keys_held.insert(code),
                            ElementState::Released => keys_held.remove(&code),
                        }
                    };
                    if state == ElementState::Pressed {
                        match code {
                            VirtualKeyCode::Escape => {
                                if *grabbed {
                                    log::debug!("Escape pressed while grabbed, releasing the mouse!");
                                    window
                                        .set_cursor_grab(false)
                                        .expect("Failed to release the mouse grab!");
                                    window.set_cursor_visible(true);
                                    *grabbed = false;
                                } else {
                                    output.exit_requested = true;
                                }
                            }
                            _ => (),
                        }
                    }
                }

                // Always track the mouse motion, but only update the orientation if
                // we're "grabbed".
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                    ..
                } => {
                    if *grabbed {
                        output.mouse_delta = (dx as f32, -dy as f32);
                    }   
                }

                // Left clicking in the window causes the mouse to get grabbed
                Event::WindowEvent {
                    event:
                        WindowEvent::MouseInput {
                            state: ElementState::Pressed,
                            button: MouseButton::Left,
                            ..
                        },
                    ..
                } => {
                    if *grabbed {
                        log::debug!("Click! We already have the mouse grabbed.");
                    } else {
                        log::debug!("Click! Grabbing the mouse.");
                        window.set_cursor_grab(true).expect("Failed to grab the mouse!");
                        window.set_cursor_visible(false);
                        *grabbed = true;
                    }
                }

                // Automatically release the mouse when focus is lost
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    if *grabbed {
                        log::debug!("Lost Focus, releasing the mouse grab...");
                        window
                            .set_cursor_grab(false)
                            .expect("Failed to release the mouse grab!");
                        window.set_cursor_visible(true);
                        *grabbed = false;
                    } else {
                        log::debug!("Lost Focus when mouse wasn't grabbed.");
                    }
                }

                // Update our size info if the window changes size.
                Event::WindowEvent {
                    event: WindowEvent::Resized(logical),
                    ..
                } => {
                    output.new_frame_size = Some((logical.width as f32, logical.height as f32));
                }
                _ => (),
            }
        });

        output.keys_held = if *grabbed {
            keys_held.clone()
        } else {
            HashSet::new()
        };

        // keys in prev key held set that are not in the current keys set means they were released
        let keys_pressed =  keys_held_prev.intersection(keys_held);
        for key in keys_pressed {
            output.keys_pressed.insert(*key);
        }

        output
    }
}