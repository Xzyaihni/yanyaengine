use winit::event::{ElementState, VirtualKeyCode};


#[derive(Debug)]
pub enum Control
{
    Keyboard{keycode: VirtualKeyCode, state: ElementState},
    Mouse{button: u32, state: ElementState},
    Scroll{x: f64, y: f64}
}
