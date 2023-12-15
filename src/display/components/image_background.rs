use std::{
    io::{self},
    path::Path,
};

use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::{OriginDimensions, Point, Size},
    image::{Image, ImageDrawable},
    Drawable, Pixel,
};
use image::io::Reader as ImageReader;
use thiserror::Error;
use tinybmp::Bmp;

use crate::{
    display::bwr_color::BWRColor,
    log,
    state::{app::ApplicationState, value::StateValueType},
};

use super::{ApplicationStateConsumer, DisplayComponent};

pub struct StaticImageBackground<'a> {
    pub name: &'a str,
    pub display: u8,
    image: Box<Bmp<'a, BWRColor>>,
}

impl<'a> StaticImageBackground<'a> {
    pub fn new(name: &'a str, display: u8, image: Box<Bmp<'a, BWRColor>>) -> Self {
        Self {
            name,
            display,
            image,
        }
    }
}

#[derive(Error, Debug)]
pub enum LoadImageError {
    #[error("File error")]
    FileError(#[from] io::Error),
    #[error("Error loading image")]
    ImageError(#[from] image::ImageError),
    #[error("Unknown error")]
    Unknown,
}

impl<'a> DisplayComponent for StaticImageBackground<'a> {
    fn get_display(&self) -> u8 {
        self.display
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Fullscreen
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        _state: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Image::new(self.image.as_ref(), Point::new(0, 0)).draw(target)?;
        Ok(())
    }

    fn get_z_index(&self, _state: &ApplicationState) -> u32 {
        10
    }
}

pub struct LoadingImageBackground<'a> {
    pub name: &'a str,
    pub display: u8,
    pub size: Size,
    pub display_buffer: Vec<Pixel<BWRColor>>,
    loaded: String,
    pub image_property: &'a str,
    pub old_state: ApplicationState, // Values last drawn
    base_path: &'a Path,
}

impl<'a> LoadingImageBackground<'a> {
    pub fn new(
        name: &'a str,
        display: u8,
        size: Size,
        path_property: &'a str,
        initial_state: ApplicationState,
        base_path: &'a Path,
    ) -> Self {
        Self {
            name,
            display,
            size,
            display_buffer: vec![
                Pixel(Point::zero(), BWRColor::Off);
                (size.width * size.height) as usize
            ],
            loaded: "".to_string(),
            image_property: path_property,
            old_state: initial_state,
            base_path,
        }
    }

    pub fn load_image(&mut self, image: String) -> Result<(), LoadImageError> {
        println!("{} Loading image: {}", log::RENDER, image);

        let image_path = self.base_path.join(image.to_string() + ".png");

        let img = ImageReader::open(image_path)?.decode()?.resize_exact(
            self.size.width,
            self.size.height,
            image::imageops::FilterType::Nearest,
        );
        let img = img.into_rgba8();

        let img = img.enumerate_pixels().map(|pix| {
            Pixel(
                Point {
                    x: pix.0 as i32,
                    y: pix.1 as i32,
                },
                match pix.2 .0 {
                    [r, g, _b, _a] if (r > 128 && g < 128) => BWRColor::Red,
                    [_r, g, _b, _a] if g > 128 => BWRColor::On,
                    _ => BWRColor::Off,
                },
            )
        });
        self.loaded = image;

        self.display_buffer = img.collect();

        Ok(())
    }

    pub fn clear_image(&mut self) {
        self.display_buffer = vec![
            Pixel(Point::zero(), BWRColor::Off);
            (self.size.width * self.size.height) as usize
        ];
    }
}

impl<'a> DisplayComponent for LoadingImageBackground<'a> {
    fn get_display(&self) -> u8 {
        self.display
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::Fullscreen
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        _state: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let value = _state.get(self.image_property);

        if let Some(StateValueType::String(state_path)) = value {
            if *state_path != self.loaded {
                let path = state_path.clone();
                let res = self.load_image(path);
                if res.is_err() {
                    self.clear_image();
                }
            }
        }

        Image::new(self, Point::new(0, 0)).draw(target)?;

        self.old_state = _state.clone();
        Ok(())
    }

    fn get_z_index(&self, _state: &ApplicationState) -> u32 {
        10
    }
    fn state_consumer(&self) -> Option<&dyn ApplicationStateConsumer> {
        Some(self)
    }

    fn state_consumer_mut(&mut self) -> Option<&mut dyn ApplicationStateConsumer> {
        Some(self)
    }
}

impl<'a> ImageDrawable for LoadingImageBackground<'a> {
    type Color = BWRColor;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: embedded_graphics::prelude::DrawTarget<Color = Self::Color>,
    {
        target.draw_iter(self.display_buffer.clone().into_iter())?;
        Ok(())
    }

    fn draw_sub_image<D>(
        &self,
        _target: &mut D,
        _area: &embedded_graphics::primitives::Rectangle,
    ) -> Result<(), D::Error>
    where
        D: embedded_graphics::prelude::DrawTarget<Color = Self::Color>,
    {
        todo!()
    }
}

impl<'a> OriginDimensions for LoadingImageBackground<'a> {
    fn size(&self) -> Size {
        self.size
    }
}

impl<'a> ApplicationStateConsumer for LoadingImageBackground<'a> {
    fn needs_refresh(&self, new_state: &ApplicationState) -> bool {
        let property = self.image_property;

        let new_value = new_state
            .map
            .get(property)
            .expect("Property not found in app state");

        if let Some(new_value_type) = &new_value.get() {
            let old_value = self
                .old_state
                .map
                .get(property)
                .expect("Property not found in old app state")
                .get();

            match new_value_type {
                StateValueType::String(val) => {
                    if let Some(StateValueType::String(old)) = &old_value {
                        return *old != *val;
                    } else {
                        return true; // only new value, no old value, we should refresh
                    }
                }
                _ => return false,
            };
        }
        // our key was not found
        false
    }
}
