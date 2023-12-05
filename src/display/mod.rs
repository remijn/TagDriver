#![allow(dead_code)]

pub mod bwr_color;
pub mod bwr_display;

pub mod components;

use self::bwr_color::BWRColor;

use embedded_graphics::{mono_font::MonoTextStyle, primitives::*};
use profont::PROFONT_24_POINT;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DisplayRotation {
    Zero,
    Rotate90,
    Rotate180,
    Rotate270,
}

pub const COLOR_BG: BWRColor = BWRColor::Off;
pub const COLOR_FG: BWRColor = BWRColor::On;

pub const STROKE_WIDTH: u32 = 2;

pub const FILL_STYLE_RED: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(BWRColor::Red);
pub const FILL_STYLE_BG: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(COLOR_BG);
pub const FILL_STYLE_FG: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(COLOR_FG);

pub const OUTLINE_STYLE_RED: PrimitiveStyle<BWRColor> =
    PrimitiveStyle::with_stroke(BWRColor::Red, STROKE_WIDTH);
pub const OUTLINE_STYLE_BG: PrimitiveStyle<BWRColor> =
    PrimitiveStyle::with_stroke(COLOR_BG, STROKE_WIDTH);
pub const OUTLINE_STYLE_FG: PrimitiveStyle<BWRColor> =
    PrimitiveStyle::with_stroke(COLOR_FG, STROKE_WIDTH);

pub const TEXT_STYLE_RED: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, BWRColor::Red);
pub const TEXT_STYLE_BG: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, COLOR_BG);
pub const TEXT_STYLE_FG: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, COLOR_FG);
