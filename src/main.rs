use core::fmt;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

mod dbus;
mod display;
mod eink;
mod log;

use display::{
    bwr_color::BWRColor,
    bwr_display::BWRDisplay,
    components::{
        bar_dialog::BarDialog, image_background::ImageBackground, DisplayAreaType, DisplayComponent,
    },
    COLOR_BG, COLOR_FG,
};
use eink::{thread::start_eink_thread, EInkInterface};

use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::{Angle, OriginDimensions, Point, Size},
    image::Image,
    prelude::DrawTarget,
    primitives::{Arc, Circle, Primitive, PrimitiveStyle, Rectangle},
    Drawable,
};

use embedded_icon::mdi::{
    size32px::{
        Battery, Battery10, Battery20, Battery30, Battery40, Battery50, Battery60, Battery70,
        Battery80, Battery90, BatteryCharging10, BatteryCharging100, BatteryCharging20,
        BatteryCharging30, BatteryCharging40, BatteryCharging50, BatteryCharging60,
        BatteryCharging70, BatteryCharging80, BatteryCharging90, BatteryOffOutline, BatteryOutline,
        PowerPlug,
    },
    size48px::{
        Arch, Brightness1, Brightness2, Brightness3, Brightness4, Brightness5, Brightness6,
        Brightness7, Cannabis, VolumeHigh, VolumeLow, VolumeMedium, VolumeVariantOff,
    },
};
use embedded_icon::NewIcon;

// impl Into<IconObj<T> for Icon<C, T> {}

use itertools::Itertools;
use tinybmp::Bmp;
use tokio::{sync::mpsc, time::sleep};

use dbus::{BusType, DBusPropertyAdress, DBusProxyAdress};

use crate::{
    dbus::DBusValue,
    display::{
        components::{simple_item::SimpleItem, state_item::StateItem},
        OUTLINE_STYLE_FG,
    },
};

const SCREEN_COUNT: u8 = 2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ////////////
    // Setup the EInk interface threads, these handle the uart
    // ////////////

    let interface1: EInkInterface = start_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if00",
        912600,
        250,
        128,
        false,
    )?;

    let interface2: EInkInterface = start_eink_thread(
        "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if04",
        912600,
        250,
        128,
        true,
    )?;
    // let interface3: EInkInterface = start_eink_thread(
    //     "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if02",
    //     912600,
    //     300,
    //     400,
    //     false,
    // )?;

    // ////////////
    // Initialise the Display and rendering code for each interface
    // ////////////
    let mut screens: [(BWRDisplay, EInkInterface); SCREEN_COUNT as usize] =
        [interface1, interface2].map(|interface| {
            (
                BWRDisplay::new(interface.width, interface.height, interface.flip),
                interface,
            )
        });

    // ////////////
    // Setup the dbus Proxies and Properties we want to listen to
    // ////////////

    let (dbus_tx, mut dbus_rx) = mpsc::channel::<bool>(20);
    let mut dbus_interface =
        dbus::dbus_interface::DBusInterface::new(dbus_tx).expect("Could not connect to DBus");

    // PROXY Backlight power settings
    static BACKLIGHT_PROXY: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.gnome.SettingsDaemon.Power",
        "/org/gnome/SettingsDaemon/Power",
    );

    // PROP display brightness
    static BRIGHTNESS_PROPERTY: DBusPropertyAdress = DBusPropertyAdress::new(
        &BACKLIGHT_PROXY,
        "org.gnome.SettingsDaemon.Power.Screen",
        "Brightness",
    );

    // PROXY playerctld Media player
    static PLAYER_PROXY: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.mpris.MediaPlayer2.playerctld",
        "/org/mpris/MediaPlayer2",
    );
    // PROP Volume
    static PLAYER_VOLUME_PROPERTY: DBusPropertyAdress =
        DBusPropertyAdress::new(&PLAYER_PROXY, "org.mpris.MediaPlayer2.Player", "Volume");

    // PROXY Battery status
    static BATTERY_PROXY: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SYSTEM,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT1",
    );
    // PROP Battery Percentage
    static BATTERY_LEVEL_PROPERTY: DBusPropertyAdress = DBusPropertyAdress::new(
        &BATTERY_PROXY,
        "org.freedesktop.UPower.Device",
        "Percentage",
    );

    // PROP Battery State
    static BATTERY_STATE_PROPERTY: DBusPropertyAdress =
        DBusPropertyAdress::new(&BATTERY_PROXY, "org.freedesktop.UPower.Device", "State");

    // ////////////
    // Configure the components to be displayed
    // ////////////

    let mut display_components: Vec<&mut dyn DisplayComponent> = Vec::new();

    const ICON_COLOR: BWRColor = COLOR_FG;

    // Dialog display brightness
    const BRIGHTNESS_ICON_COUNT: u32 = 6;
    let mut brightness_dialog = BarDialog::new(
        "brightness dialog",
        &BRIGHTNESS_PROPERTY,
        0,
        Box::new(|target: &mut Canvas<BWRColor>, val, center| {
            // const color = BWRColor::Off;
            match (val * BRIGHTNESS_ICON_COUNT as f64).floor() as u32 {
                6 => Image::with_center(&Brightness7::new(ICON_COLOR), center).draw(target),
                5 => Image::with_center(&Brightness6::new(ICON_COLOR), center).draw(target),
                4 => Image::with_center(&Brightness5::new(ICON_COLOR), center).draw(target),
                3 => Image::with_center(&Brightness4::new(ICON_COLOR), center).draw(target),
                2 => Image::with_center(&Brightness3::new(ICON_COLOR), center).draw(target),
                1 => Image::with_center(&Brightness2::new(ICON_COLOR), center).draw(target),
                0 | _ => Image::with_center(&Brightness1::new(ICON_COLOR), center).draw(target),
            }
            .ok();
        }),
    );
    display_components.push(&mut brightness_dialog);

    // Dialog player volume
    const PLAYER_VOLUME_ICON_COUNT: u32 = 3;
    let mut player_volume_dialog = BarDialog::new(
        "player volume dialog",
        &PLAYER_VOLUME_PROPERTY,
        1,
        Box::new(|target: &mut Canvas<BWRColor>, val, center| {
            match (val * PLAYER_VOLUME_ICON_COUNT as f64).ceil() as u16 {
                3 => Image::with_center(&VolumeHigh::new(ICON_COLOR), center).draw(target),
                2 => Image::with_center(&VolumeMedium::new(ICON_COLOR), center).draw(target),
                1 => Image::with_center(&VolumeLow::new(ICON_COLOR), center).draw(target),
                0 | _ => {
                    Image::with_center(&VolumeVariantOff::new(ICON_COLOR), center).draw(target)
                }
            }
            .ok();
        }),
    );
    display_components.push(&mut player_volume_dialog);

    let mut arch_icon = SimpleItem::new(
        "Arch Icon",
        0,
        Box::new(|target: &mut Canvas<BWRColor>, center| {
            Image::with_center(&Arch::new(ICON_COLOR), center)
                .draw(target)
                .ok();
        }),
    );
    display_components.push(&mut arch_icon);

    let mut weed_icon = SimpleItem::new(
        "Weed Icon",
        0,
        Box::new(|target: &mut Canvas<BWRColor>, center| {
            Image::with_center(&Cannabis::new(ICON_COLOR), center)
                .draw(target)
                .ok();
        }),
    );
    display_components.push(&mut weed_icon);

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
    let mut battery_icon = StateItem::new(
        "Battery Icon",
        [&BATTERY_LEVEL_PROPERTY, &BATTERY_STATE_PROPERTY].to_vec(),
        0,
        Box::new(
            |target: &mut Canvas<BWRColor>,
             values: HashMap<DBusPropertyAdress, DBusValue>,
             center: Point| {
                let level = values.get(&BATTERY_LEVEL_PROPERTY).expect("no level");

                let bat_percentage = match level {
                    DBusValue::F64(val) => *val,
                    DBusValue::U64(val) => *val as f64 / 100.0,
                    DBusValue::I64(val) => *val as f64 / 100.0,
                    _ => 0.0,
                } / 100.0;

                let state = values.get(&BATTERY_STATE_PROPERTY).expect("no state");
                let bat_state = match state {
                    DBusValue::U64(val) if *val == 0 => BatteryState::Unknown,
                    DBusValue::U64(val) if *val == 1 => BatteryState::Charging,
                    DBusValue::U64(val) if *val == 2 => BatteryState::Discharging,
                    DBusValue::U64(val) if *val == 3 => BatteryState::Empty,
                    DBusValue::U64(val) if *val == 4 => BatteryState::Full,
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
                    .into_styled(PrimitiveStyle::with_stroke(COLOR_FG, 6))
                    .draw(target)
                    .ok();
                }

                match bat_state {
                    BatteryState::Unknown => {
                        Image::with_center(&BatteryOffOutline::new(ICON_COLOR), center)
                            .draw(target)
                            .ok();
                        return;
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
                            0 | _ => Image::with_center(&BatteryOutline::new(ICON_COLOR), center)
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
                            0 | _ => Image::with_center(&BatteryOutline::new(ICON_COLOR), center)
                                .draw(target),
                        }
                        .ok();
                    }
                }
            },
        ),
    );
    display_components.push(&mut battery_icon);

    let background_small_bytes = include_bytes!("../resources/logo250.bmp");
    let background_small = Box::new(Bmp::<BWRColor>::from_slice(background_small_bytes).unwrap());

    //Background screen 0
    // let mut background_0 = ImageBackground::new("Background 1", 0, background_small.clone());
    let mut background_1 = ImageBackground::new("Background 1", 1, background_small.clone());

    // display_components.push(&mut background_0);
    display_components.push(&mut background_1);
    // ////////////
    // Get all the wanted dbus properties and their initial values
    // ////////////

    let mut properties: Vec<DBusPropertyAdress> = Vec::new();
    // Get all the wanted properties
    let iter = display_components.iter();
    for component in iter {
        if let Some(dbus) = component.dbus() {
            properties.append(
                &mut dbus
                    .wanted_dbus_values()
                    .iter()
                    .map(|v| (*v).clone())
                    .collect_vec(),
            );
        }
    }

    let dbus_values = dbus_interface.values.clone();

    let initial = dbus_interface.init(properties).await?; //.expect("Could not init DBus");

    // Set the initial values for the components
    for component in display_components.iter_mut() {
        if let Some(dbus) = component.dbus_mut() {
            dbus.set_initial(&initial);
        }
    }
    // ////////////
    // Start the dbus thread
    // ////////////

    println!("{}", log::WELCOME);

    tokio::spawn(async move {
        dbus_interface.run().await;
    });

    // ////////////
    // Run the main loop
    // ////////////

    let mut screen_refresh_after: [Option<Instant>; SCREEN_COUNT as usize] =
        [Some(Instant::now()); SCREEN_COUNT as usize];

    loop {
        // Proccess dmesg updates
        let mut screen_needs_refresh: Vec<bool> = [false; SCREEN_COUNT as usize].to_vec().clone();

        while let Ok(_has_new) = dbus_rx.try_recv() {
            // We have new values, check with each component if this new state requires a refresh
            let values = &dbus_values.lock().await;

            for component in display_components.iter() {
                let mut component_needs_refresh = false;

                if let Some(dbus) = component.dbus() {
                    // Is updated needed by dbus?
                    component_needs_refresh = dbus.needs_refresh(&values);
                }

                if component_needs_refresh {
                    println!(
                        "{} Component \"{}\" requests update on screen {}",
                        log::SCREEN,
                        component.get_name(),
                        component.get_screen()
                    )
                }
                screen_needs_refresh[component.get_screen() as usize] |= component_needs_refresh;
            }
        }

        for i in 0..SCREEN_COUNT {
            if let Some(time) = screen_refresh_after[i as usize] {
                if time < Instant::now() {
                    screen_needs_refresh[i as usize] = true;
                    screen_refresh_after[i as usize] = None;
                    println!("{} Refresh After on screen {}", log::SCREEN, i)
                }
            }
        }

        //Loop through each screen, and check if it needs updating
        for (i, (display, interface)) in screens.iter_mut().enumerate() {
            if !screen_needs_refresh[i] {
                continue;
            }

            // Screen i needs an update, lets wrender
            println!("{} Rendering screen {}", log::RENDER, i);

            // clear the screen
            display.clear(COLOR_BG)?;

            let values = Box::new(dbus_values.lock().await.clone());

            // list of components filtered by the current screen, mapped to zindex, and then sorted
            let components = display_components
                .iter_mut()
                .filter(|component| component.get_screen() == i as u8)
                .map(|component| {
                    let index = component.get_z_index(&values);
                    (component, index)
                })
                .filter(|component| component.1 != 0)
                .sorted_by(|a, b| Ord::cmp(&b.1, &a.1));

            // Draw the components to a list of canvases
            let mut canvases: Vec<(Canvas<BWRColor>, DisplayAreaType)> = Vec::new();
            for component in components {
                println!(
                    "{} Render {} Z:{}",
                    log::RENDER,
                    component.0.get_name(),
                    component.1,
                );

                let mut size = display.size();

                match component.0.get_type() {
                    DisplayAreaType::Area(width, height) => {
                        size = Size::new(width, height);
                    }
                    _ => {}
                }

                let mut canvas = {
                    let canvas = Canvas::<BWRColor>::new(size);

                    // draw a rectangle smaller than the canvas (with 1px)
                    // let canvas_rectangle = Rectangle::new(Point::zero(), size);

                    // let canvas_outline = canvas_rectangle.into_styled(OUTLINE_STYLE_FG);
                    // draw the canvas rectangle for debugging
                    // canvas_outline.draw(&mut canvas)?;

                    canvas
                };

                component.0.draw(&mut canvas, values.clone())?;

                let refresh = component.0.get_refresh_at();
                if refresh.is_some()
                    && (screen_refresh_after[i].is_none()
                        || refresh.expect("") > screen_refresh_after[i].expect(""))
                {
                    screen_refresh_after[i] = component.0.get_refresh_at();
                    println!(
                        "⏳️ Screen refresh after {}ms",
                        (screen_refresh_after[i].expect("") - Instant::now()).as_millis()
                    );
                }
                canvases.push((canvas, component.0.get_type()));

                match component.0.get_type() {
                    DisplayAreaType::Dialog => {
                        break;
                    }
                    // DisplayAreaType::Fullscreen => break,
                    _ => continue,
                }
            }

            canvases.reverse();

            let mut pos = Point::new(10, 10);

            for canvas in canvases {
                match canvas.1 {
                    DisplayAreaType::Fullscreen | DisplayAreaType::Dialog => canvas
                        .0
                        .place_at(Point::zero())
                        .draw(display)
                        .expect("Could not draw canvas to display"),
                    DisplayAreaType::Area(width, _height) => {
                        canvas
                            .0
                            .place_at(pos)
                            .draw(display)
                            .expect("Could not draw canvas to display");

                        pos += Size::new(width, 0);
                    }
                }
            }

            drop(values);

            let (black, _red4) = display.get_fixed_buffer();

            // Stupid hack to force full-refresh the right screen
            if interface.flip {
                interface
                    .full(black)
                    .await
                    .expect("Error sending to main thread");
            } else {
                interface
                    .fast(black)
                    .await
                    .expect("Error sending to main thread");
            }
        }
        sleep(Duration::from_millis(10)).await;
    }
}
