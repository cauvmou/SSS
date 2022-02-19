#![feature(drain_filter)]

use image::{ImageBuffer};
use mouse_rs::Mouse;
use scrap::{Display, Capturer};
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Window}, event::{KeyboardInput, ElementState}, dpi::PhysicalPosition, monitor::{MonitorHandle}};
use dirs::home_dir;
use std::{thread};
use std::process::Command;

static SUPER: u32 = 125;
static SHIFT: u32 = 42;
static S: u32 = 31;
static SSS: [u32; 3] = [SUPER, SHIFT, S];

fn main() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(false)
        .with_title("SSS Manager")
        .with_position(PhysicalPosition::new(0, 0))
        .build(&event_loop).unwrap();

    let mut pressed_keys: Vec<u32> = Vec::new();
    let mouse = Mouse::new();
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            winit::event::Event::DeviceEvent { event, .. } => {
                match event {
                    winit::event::DeviceEvent::Key(KeyboardInput { scancode, state, .. }) => {
                        if state == ElementState::Pressed && !pressed_keys.contains(&scancode) {
                            pressed_keys.push(scancode);
                        } else if state == ElementState::Released && pressed_keys.contains(&scancode) {
                            pressed_keys.drain_filter(|sc| sc == &scancode);
                        }
                        if pressed_keys.starts_with(&SSS) {
                            let pos =  mouse.get_position().unwrap();
                            let cursor_position = PhysicalPosition::new(pos.x as u32, pos.y as u32);
                            let (imgbuffer, monitor_handle) = screenshot(&window, cursor_position).unwrap();
                            imgbuffer.save_with_format(format!("{}/.sss/tmp.png", home_dir().unwrap().to_str().unwrap()), image::ImageFormat::Png).unwrap();
                            thread::spawn(move || {
                                // args: monitor_x monitor_y path
                                Command::new("./screenshot_frontend")
                                    .args(&[format!("{}", monitor_handle.position().x), format!("{}", monitor_handle.position().y), 
                                            format!("{}/.sss/tmp.png", home_dir().unwrap().to_str().unwrap())]).spawn().unwrap();
                            });
                        }
                    },
                    _ => {}
                }
            },
            winit::event::Event::MainEventsCleared => {
                
            },
            _ => {}
        }
    });
}

fn screenshot(window: &Window, cursor_position: PhysicalPosition<u32>) -> Option<(ImageBuffer<image::Rgba<u8>, Vec<u8>>, MonitorHandle)> {
    let monitors = window.available_monitors();
    let mut selected_monitor: Option<MonitorHandle> = None;
    let mut monitor_index: Option<usize> = None;
    for (index, monitor) in monitors.enumerate() {
        let pos = monitor.position();
        let size = monitor.size();
        if cursor_position.x >= pos.x as u32 && cursor_position.x <= pos.x as u32 + size.width && 
            cursor_position.y >= pos.y as u32 && cursor_position.y <= pos.y as u32 + size.height {
                selected_monitor = Some(monitor);
                monitor_index = Some(index);
        }
    }
    let displays = Display::all().unwrap();
    let mut index: usize = 0;
    let mut imgbuffer: Option<ImageBuffer<image::Rgba<u8>, Vec<u8>>> = None;
    for display in displays {
        if monitor_index.unwrap() == index {
            let (width, height) = (display.width() as usize, display.height() as usize);
            let mut capturer = Capturer::new(display).unwrap();
            let frame = capturer.frame().unwrap();
            let mut bitflipped = Vec::with_capacity(width * height * 4);
            let stride = frame.len() / height;

            for y in 0..height {
                for x in 0..width {
                    let i = stride * y + 4 * x;
                    bitflipped.extend_from_slice(&[
                        frame[i + 2],
                        frame[i + 1],
                        frame[i],
                        255,
                    ]);
                }
            }
            imgbuffer = image::ImageBuffer::from_raw(width as u32, height as u32, bitflipped);
        }
        index += 1;
    }
    Some((imgbuffer.unwrap(), selected_monitor.unwrap()))
}