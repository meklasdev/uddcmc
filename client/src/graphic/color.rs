use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Yellow,
    Green,
    Red,
    Blue,
    White,
    Purple,
    Cyan,
    Black,
}

impl Color {
    pub fn to_rgb(&self) -> Rgb {
        match self {
            Color::Yellow => Rgb::new(1.0, 1.0, 0.0),
            Color::Green => Rgb::new(0.0, 1.0, 0.0),
            Color::Red => Rgb::new(1.0, 0.0, 0.0),
            Color::Blue => Rgb::new(0.0, 0.0, 1.0),
            Color::White => Rgb::new(1.0, 1.0, 1.0),
            Color::Purple => Rgb::new(1.0, 0.0, 1.0),
            Color::Cyan => Rgb::new(0.0, 1.0, 1.0),
            Color::Black => Rgb::new(0.0, 0.0, 0.0),
        }
    }

    pub fn to_rgba(&self, a: f32) -> Rgba {
        Rgba::new(self.to_rgb(), a)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn with_alpha(self, a: f32) -> Rgba {
        Rgba::new(self, a)
    }
}

impl From<(f32, f32, f32)> for Rgb {
    fn from(value: (f32, f32, f32)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl From<Rgb> for (f32, f32, f32) {
    fn from(value: Rgb) -> Self {
        (value.r, value.g, value.b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub rgb: Rgb,
    pub a: f32,
}

impl Rgba {
    pub fn new(rgb: Rgb, a: f32) -> Self {
        Self { rgb, a }
    }

    pub fn new_rgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            rgb: Rgb { r, g, b },
            a,
        }
    }
}

impl From<(f32, f32, f32, f32)> for Rgba {
    fn from(value: (f32, f32, f32, f32)) -> Self {
        Self::new_rgb(value.0, value.1, value.2, value.3)
    }
}

impl From<Rgba> for (f32, f32, f32, f32) {
    fn from(value: Rgba) -> Self {
        (value.r, value.g, value.b, value.a)
    }
}

impl Deref for Rgba {
    type Target = Rgb;

    fn deref(&self) -> &Self::Target {
        &self.rgb
    }
}

impl DerefMut for Rgba {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rgb
    }
}
