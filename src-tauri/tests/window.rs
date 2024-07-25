use std::thread;
use std::time::Duration;
use rdev::{Event, EventType, listen};

fn callback(event: Event) {
    println!("Event: {:?}", event);
    match event.event_type {
        EventType::KeyPress(key) => {
            println!("KeyPress: {:?}", key);
        }
        EventType::KeyRelease(key) => {
            println!("KeyRelease: {:?}", key);
        }
        EventType::ButtonPress(button) => {
            println!("ButtonPress: {:?}", button);
        }
        EventType::ButtonRelease(button) => {
            println!("ButtonRelease: {:?}", button);
        }
        EventType::MouseMove { x, y } => {
            println!("MouseMove: x={} y={}", x, y);
        }
        EventType::Wheel { delta_x, delta_y } => {
            println!("Wheel: delta_x={} delta_y={}", delta_x, delta_y);
        }
        _ => {}
    }
}

#[test]
pub fn get_current_window() -> anyhow::Result<()> {
    use winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::macos::MonitorHandleExtMacOS,
        window::WindowBuilder,
    };
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_os = "macos")]
    {
        if let Some(monitor_handle) = window.current_monitor() {
            let ns_screen = monitor_handle.ns_screen();
            println!("NSScreen: {:?}", ns_screen);
        } else {
            println!("Failed to get current monitor");
        }
    }
    // let Some(t) = en.ns_screen();
    // println!("{:?}",t);
    println!("Window:");

    Ok(())
}
