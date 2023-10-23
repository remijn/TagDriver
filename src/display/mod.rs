#![allow(dead_code)]
pub mod bwr_color;
pub mod bwr_display;

use embedded_graphics::{mono_font::MonoTextStyle, prelude::*, primitives::*, Drawable};
use profont::PROFONT_24_POINT;
use std::convert::Infallible;

use self::{bwr_color::BWRColor, bwr_display::BWRDisplay};
const FILL_STYLE_RED: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(BWRColor::Red);
const FILL_STYLE_BLACK: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(BWRColor::On);
const FILL_STYLE_WHITE: PrimitiveStyle<BWRColor> = PrimitiveStyle::with_fill(BWRColor::Off);

const TEXT_STYLE_RED: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, BWRColor::Red);
const TEXT_STYLE_BLACK: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, BWRColor::On);
const TEXT_STYLE_WHITE: MonoTextStyle<'_, BWRColor> =
    MonoTextStyle::new(&PROFONT_24_POINT, BWRColor::Off);

pub fn generate_image(display: &mut BWRDisplay, i: u32) -> Result<u8, Infallible> {
    // let line_style = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    // let text_style = MonoTextStyle::new(&FONT, BinaryColor::On);

    display.clear(BWRColor::On)?;
    // debug_println!("display I:{}", i);

    Rectangle::new(Point::new(110, 122 - i as i32), Size::new(50, i))
        .into_styled(FILL_STYLE_WHITE)
        .draw(display)?;

    // let mut text: Vec<&str> = "Voor de grap even kijken hoe snel dit kan refreshen"
    //     .split(' ')
    //     .collect();
    // text.push("");

    // Text::new(text[i as usize], Point::new(10, 80), text_style_black).draw(display)?;

    // // Text::new("Werkt!", Point::new(10, 100), text_style_black).draw(display)?;

    // Rectangle::new(Point::new(190, 10), Size::new(50, 50))
    //     .into_styled(fill_style_black)
    //     .draw(display)?;

    // Rectangle::new(Point::new(1, 10), Size::new(100, 50))
    //     .into_styled(fill_style_red)
    //     .draw(display)?;

    // let circle_size = 5 * i;
    // for x in 0..WIDTH / circle_size {
    //     for y in 0..HEIGHT / circle_size {
    //         let mut style = line_style;
    //         if ((x + y) % 2 == 0) {
    //             style = fill_style;
    //         }
    //         Circle::new(
    //             Point::new((x * circle_size) as i32, (y * circle_size) as i32),
    //             circle_size,
    //         )
    //         .into_styled(style)
    //         .draw(display)?;
    //         // Rectangle::new(
    //         //     Point::new((x * circle_size) as i32, (y * circle_size) as i32),
    //         //     Size::new(10, 10),
    //         // )
    //         // .into_styled(fill_style)
    //         // .draw(display)?;
    //     }
    // }

    // Line::new(Point::new(5, 5), Point::new(120, 60))
    //     .into_styled(line_style)
    //     .draw(display)?;

    // Circle::new(Point::new(72, 8), 48)
    //     .into_styled(line_style)
    //     .draw(display)?;

    // Line::new(Point::new(48, 16), Point::new(8, 16))
    //     .into_styled(line_style)
    //     .draw(display)?;

    // Line::new(Point::new(48, 16), Point::new(64, 32))
    //     .into_styled(line_style)
    //     .draw(display)?;

    // Rectangle::new(Point::new(79, 15), Size::new(34, 34))
    //     .into_styled(line_style)
    //     .draw(display)?;

    // Text::new("Hello World!", Point::new(5, 5), text_style).draw(display)?;

    // let _text = Text::new(
    //     "Test a thinghythingasdfasdf",
    //     Point::new(60, 60),
    //     text_style,
    // )
    // .draw(display);

    Ok(0)
}
