use crate::Graphics;

pub enum Event<'a> {
    WindowResized(u32, u32),
    Draw(&'a mut Graphics),
}
