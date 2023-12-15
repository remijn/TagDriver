use core::fmt;
use std::{error::Error, path::Path, time::Instant};

pub mod bar_dialog;
pub mod image_background;
pub mod simple_item;
pub mod state_item;

use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::{Angle, OriginDimensions, Size},
    image::Image,
    prelude::Point,
    primitives::{Arc, Circle, Primitive, PrimitiveStyle},
    Drawable,
};

use embedded_icon::mdi::{
    size32px::{
        Battery, Battery10, Battery20, Battery30, Battery40, Battery50, Battery60, Battery70,
        Battery80, Battery90, BatteryCharging10, BatteryCharging100, BatteryCharging20,
        BatteryCharging30, BatteryCharging40, BatteryCharging50, BatteryCharging60,
        BatteryCharging70, BatteryCharging80, BatteryCharging90, BatteryChargingOutline,
        BatteryOffOutline, BatteryOutline, PowerPlug, WifiStrength1, WifiStrength2, WifiStrength3,
        WifiStrength4, WifiStrengthAlertOutline, WifiStrengthOffOutline, WifiStrengthOutline,
    },
    size48px::{
        Arch, Brightness1, Brightness2, Brightness3, Brightness4, Brightness5, Brightness6,
        Brightness7, Cannabis, VolumeHigh, VolumeLow, VolumeMedium, VolumeVariantOff,
    },
};
use embedded_icon::NewIcon;
use tinybmp::Bmp;

use crate::{
    display::{
        components::{
            bar_dialog::BarDialog,
            image_background::{LoadingImageBackground, StaticImageBackground},
            simple_item::SimpleItem,
            state_item::StateItem,
        },
        COLOR_FG,
    },
    state::{
        app::ApplicationState,
        value::{NetworkState, StateValueType},
    },
};

use super::bwr_color::BWRColor;

pub trait ApplicationStateConsumer {
    fn needs_refresh(&self, new_values: &ApplicationState) -> bool;
}

pub type NextRefresh = (Instant, RefreshType);

pub enum RefreshType {
    Full,
    Partial,
}

#[derive(PartialEq, Eq)]
pub enum DisplayAreaType {
    Area(u32, u32), // item contained in area, i.e. icon for bluetooth
    Fullscreen,     // Fullscreen thing, large clock or image, stays open
    Dialog, // Fullscreen dialog that shows on change, (volume or brightness popup, notifications maybe), and dissapears after
}
pub trait DisplayComponent {
    fn get_display(&self) -> u8;
    fn get_type(&self) -> DisplayAreaType;
    fn get_name(&self) -> &str;
    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        values: &ApplicationState,
    ) -> Result<(), Box<dyn Error>>;
    fn get_z_index(&self, values: &ApplicationState) -> u32;
    fn get_refresh_at(&self) -> Option<Instant> {
        None
    }

    fn state_consumer(&self) -> Option<&dyn ApplicationStateConsumer> {
        None
    }
    fn state_consumer_mut(&mut self) -> Option<&mut dyn ApplicationStateConsumer> {
        None
    }
}

pub trait IconComponent {
    fn draw_icon(&self, target: &mut Canvas<BWRColor>, value: f64, center: Point);
}

pub fn make_ui_components(state: ApplicationState) -> Vec<Box<dyn DisplayComponent>> {
    // ////////////
    // Configure the components to be displayed
    // ////////////

    let mut ui_components: Vec<Box<dyn DisplayComponent>> = Vec::new();

    const ICON_COLOR: BWRColor = COLOR_FG;

    // Dialog display brightness
    const BRIGHTNESS_ICON_COUNT: u32 = 6;
    let brightness_dialog = BarDialog::new(
        "brightness dialog",
        "backlight:brightness",
        0,
        state.clone(),
        Box::new(|target: &mut Canvas<BWRColor>, val, center| {
            // const color = BWRColor::Off;
            match (val * BRIGHTNESS_ICON_COUNT as f64).round() as u32 {
                6 => Image::with_center(&Brightness7::new(ICON_COLOR), center).draw(target),
                5 => Image::with_center(&Brightness6::new(ICON_COLOR), center).draw(target),
                4 => Image::with_center(&Brightness5::new(ICON_COLOR), center).draw(target),
                3 => Image::with_center(&Brightness4::new(ICON_COLOR), center).draw(target),
                2 => Image::with_center(&Brightness3::new(ICON_COLOR), center).draw(target),
                1 => Image::with_center(&Brightness2::new(ICON_COLOR), center).draw(target),
                _ => Image::with_center(&Brightness1::new(ICON_COLOR), center).draw(target),
            }
            .ok();
        }),
    );
    ui_components.push(Box::new(brightness_dialog));

    // Dialog player volume
    const PLAYER_VOLUME_ICON_COUNT: u32 = 3;
    let player_volume_dialog = BarDialog::new(
        "player volume dialog",
        "player:volume",
        1,
        state.clone(),
        Box::new(|target: &mut Canvas<BWRColor>, val, center| {
            match (val * PLAYER_VOLUME_ICON_COUNT as f64).ceil() as u16 {
                3 => Image::with_center(&VolumeHigh::new(ICON_COLOR), center).draw(target),
                2 => Image::with_center(&VolumeMedium::new(ICON_COLOR), center).draw(target),
                1 => Image::with_center(&VolumeLow::new(ICON_COLOR), center).draw(target),
                _ => Image::with_center(&VolumeVariantOff::new(ICON_COLOR), center).draw(target),
            }
            .ok();
        }),
    );
    ui_components.push(Box::new(player_volume_dialog));

    let arch_icon = SimpleItem::new("Arch Icon", 0, Arch::new(ICON_COLOR));
    ui_components.push(Box::new(arch_icon));

    let weed_icon = SimpleItem::new("Weed Icon", 0, Cannabis::new(ICON_COLOR));
    ui_components.push(Box::new(weed_icon));

    enum BatteryState {
        Unknown,
        Charging,
        Discharging,
        Empty,
        Full,
    }
    impl fmt::Debug for BatteryState {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    BatteryState::Unknown => "Unknown",
                    BatteryState::Charging => "Charging",
                    BatteryState::Discharging => "Discharging",
                    BatteryState::Empty => "Empty",
                    BatteryState::Full => "Full",
                }
            )
        }
    }

    const BATTERY_ICON_COUNT: u32 = 10;
    let battery_icon = StateItem::new(
        "Battery Icon",
        ["battery:level", "battery:state"].to_vec(),
        0,
        state.clone(),
        Box::new(
            |target: &mut Canvas<BWRColor>, values: &ApplicationState, center: Point| {
                let Some(StateValueType::F64(level)) = values.get("battery:level") else {
                    panic!("Value not found");
                };

                let bat_percentage = level / 100.0;

                let Some(StateValueType::U64(bat_state)) = values.get("battery:state") else {
                    panic!("Value not found");
                };
                let bat_state = match bat_state {
                    0 => BatteryState::Unknown,
                    1 => BatteryState::Charging,
                    2 => BatteryState::Discharging,
                    3 => BatteryState::Empty,
                    4 => BatteryState::Full,
                    _ => BatteryState::Unknown,
                };

                fn draw_arc(target: &mut Canvas<BWRColor>, value: f64, center: Point) {
                    let circle = Circle::with_center(
                        center,
                        target.size().width.min(target.size().height) - 7,
                    );
                    Arc::from_circle(
                        circle,
                        Angle::from_degrees(-90.0),
                        Angle::from_degrees((360.0 * value) as f32),
                    )
                    .into_styled(PrimitiveStyle::with_stroke(ICON_COLOR, 6))
                    .draw(target)
                    .ok();
                }

                match bat_state {
                    BatteryState::Unknown => {
                        Image::with_center(&BatteryOffOutline::new(ICON_COLOR), center)
                            .draw(target)
                            .ok();
                    }
                    BatteryState::Full => {
                        draw_arc(target, bat_percentage, center);
                        Image::with_center(&PowerPlug::new(ICON_COLOR), center)
                            .draw(target)
                            .ok();
                    }
                    BatteryState::Discharging | BatteryState::Empty => {
                        draw_arc(target, bat_percentage, center);
                        match (bat_percentage * BATTERY_ICON_COUNT as f64).round() as u16 {
                            10 => {
                                Image::with_center(&Battery::new(ICON_COLOR), center).draw(target)
                            }
                            9 => {
                                Image::with_center(&Battery90::new(ICON_COLOR), center).draw(target)
                            }
                            8 => {
                                Image::with_center(&Battery80::new(ICON_COLOR), center).draw(target)
                            }
                            7 => {
                                Image::with_center(&Battery70::new(ICON_COLOR), center).draw(target)
                            }
                            6 => {
                                Image::with_center(&Battery60::new(ICON_COLOR), center).draw(target)
                            }
                            5 => {
                                Image::with_center(&Battery50::new(ICON_COLOR), center).draw(target)
                            }
                            4 => {
                                Image::with_center(&Battery40::new(ICON_COLOR), center).draw(target)
                            }
                            3 => {
                                Image::with_center(&Battery30::new(ICON_COLOR), center).draw(target)
                            }
                            2 => {
                                Image::with_center(&Battery20::new(ICON_COLOR), center).draw(target)
                            }
                            1 => {
                                Image::with_center(&Battery10::new(ICON_COLOR), center).draw(target)
                            }
                            _ => Image::with_center(&BatteryOutline::new(ICON_COLOR), center)
                                .draw(target),
                        }
                        .ok();
                    }
                    BatteryState::Charging => {
                        draw_arc(target, bat_percentage, center);
                        let center = center + Size::new(1, 0);
                        match (bat_percentage * BATTERY_ICON_COUNT as f64).round() as u16 {
                            10 => Image::with_center(&BatteryCharging100::new(ICON_COLOR), center)
                                .draw(target),
                            9 => Image::with_center(&BatteryCharging90::new(ICON_COLOR), center)
                                .draw(target),
                            8 => Image::with_center(&BatteryCharging80::new(ICON_COLOR), center)
                                .draw(target),
                            7 => Image::with_center(&BatteryCharging70::new(ICON_COLOR), center)
                                .draw(target),
                            6 => Image::with_center(&BatteryCharging60::new(ICON_COLOR), center)
                                .draw(target),
                            5 => Image::with_center(&BatteryCharging50::new(ICON_COLOR), center)
                                .draw(target),
                            4 => Image::with_center(&BatteryCharging40::new(ICON_COLOR), center)
                                .draw(target),
                            3 => Image::with_center(&BatteryCharging30::new(ICON_COLOR), center)
                                .draw(target),
                            2 => Image::with_center(&BatteryCharging20::new(ICON_COLOR), center)
                                .draw(target),
                            1 => Image::with_center(&BatteryCharging10::new(ICON_COLOR), center)
                                .draw(target),
                            _ => {
                                Image::with_center(&BatteryChargingOutline::new(ICON_COLOR), center)
                                    .draw(target)
                            }
                        }
                        .ok();
                    }
                }
            },
        ),
    );
    ui_components.push(Box::new(battery_icon));

    let wifi_icon = StateItem::new(
        "Wifi Icon",
        ["wifi:state", "wifi:strength"].to_vec(),
        0,
        state.clone(),
        Box::new(
            |target: &mut Canvas<BWRColor>, values: &ApplicationState, center: Point| {
                let Some(StateValueType::NetworkState(state)) = values.get("wifi:state") else {
                    panic!("Value not found");
                };

                match state {
                    NetworkState::Connecting => {
                        Image::with_center(&WifiStrengthAlertOutline::new(ICON_COLOR), center)
                            .draw(target)
                            .ok();
                    }
                    NetworkState::Connected => {
                        let Some(StateValueType::F64(strength)) = values.get("wifi:strength")
                        else {
                            Image::with_center(&WifiStrengthAlertOutline::new(ICON_COLOR), center)
                                .draw(target)
                                .ok();
                            return;
                        };
                        match strength.round() as u32 {
                            0..=20 => {
                                Image::with_center(&WifiStrengthOutline::new(ICON_COLOR), center)
                                    .draw(target)
                            }
                            21..=40 => Image::with_center(&WifiStrength1::new(ICON_COLOR), center)
                                .draw(target),
                            41..=60 => Image::with_center(&WifiStrength2::new(ICON_COLOR), center)
                                .draw(target),
                            61..=80 => Image::with_center(&WifiStrength3::new(ICON_COLOR), center)
                                .draw(target),
                            81..=100 => Image::with_center(&WifiStrength4::new(ICON_COLOR), center)
                                .draw(target),
                            _ => Image::with_center(&WifiStrengthOutline::new(ICON_COLOR), center)
                                .draw(target),
                        }
                        .ok();
                    }
                    _ => {
                        Image::with_center(&WifiStrengthOffOutline::new(ICON_COLOR), center)
                            .draw(target)
                            .ok();
                    }
                }
            },
        ),
    );
    ui_components.push(Box::new(wifi_icon));

    let background_small_bytes = include_bytes!("../../../resources/logo250.bmp");
    let background_small = Box::new(Bmp::<BWRColor>::from_slice(background_small_bytes).unwrap());

    // let background_large_bytes = include_bytes!("../../../resources/logo400.bmp");
    // let background_large = Box::new(Bmp::<BWRColor>::from_slice(background_large_bytes).unwrap());

    // let mut background_0 = ImageBackground::new("Background 1", 0, background_small.clone());
    let background_1 = StaticImageBackground::new("Background 1", 1, background_small.clone());
    // let background_2 = StaticImageBackground::new("Background 2", 2, background_large.clone());

    let background_3 = LoadingImageBackground::new(
        "Background 2.5",
        2,
        Size::new(400, 300),
        "rear-image-path",
        state.clone(),
        Path::new("/home/nick/tags/img/400/"),
    );

    // background_3
    //     .load_image("/home/nick/tags/img/400/bobr.png".to_string())
    //     .unwrap();

    // display_components.push(&mut background_0);
    ui_components.push(Box::new(background_1));
    // ui_components.push(Box::new(background_2));

    ui_components.push(Box::new(background_3));

    // let arrow_icon = SimpleItem::new("Arrow test icon", 2, ArrowLeftThick::new(ICON_COLOR));
    // ui_components.push(Box::new(arrow_icon));

    ui_components
}
