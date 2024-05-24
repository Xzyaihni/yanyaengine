use winit::{
    event::{ElementState, MouseButton},
    keyboard::PhysicalKey
};


#[derive(Debug, Clone)]
pub enum Control
{
    Keyboard{keycode: PhysicalKey, state: ElementState},
    Mouse{button: MouseButton, state: ElementState},
    Scroll{x: f64, y: f64}
}
