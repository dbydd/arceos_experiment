use alloc::vec::Vec;

#[derive(Debug)]
pub enum USBSystemEvent {
    MouseEvent(MouseEvent),
}

#[derive(Debug, Default)]
pub struct MouseEvent {
    pub dx: isize,
    pub dy: isize,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub wheel: isize,
}
