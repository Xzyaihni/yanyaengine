use winit::{
    event::{ElementState, MouseButton},
    keyboard::{Key, PhysicalKey}
};


#[derive(Debug, Clone)]
pub enum Control
{
    Keyboard{logical: Key, keycode: PhysicalKey, state: ElementState},
    Mouse{button: MouseButton, state: ElementState},
    Scroll{x: f64, y: f64}
}
