use std::time::{Duration, Instant};

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

use embedded_graphics::{geometry::Point, image::Image, prelude::DrawTarget, Drawable};
use embedded_icon::mdi::size48px::{
    Arch, Brightness1, Brightness2, Brightness3, Brightness4, Brightness5, Brightness6,
    Brightness7, VolumeHigh, VolumeLow, VolumeMedium, VolumeVariantOff,
};
use embedded_icon::NewIcon;

// impl Into<IconObj<T> for Icon<C, T> {}

use itertools::Itertools;
use tinybmp::Bmp;
use tokio::{sync::mpsc, time::sleep};

use dbus::{BusType, DBusPropertyAdress, DBusProxyAdress};

use crate::display::components::{simple_item::SimpleItem, state_item::StateItem};

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
    let backlight_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.gnome.SettingsDaemon.Power",
        "/org/gnome/SettingsDaemon/Power",
    );

    // PROP display brightness
    let brightness_property: DBusPropertyAdress = DBusPropertyAdress::new(
        backlight_proxy,
        "org.gnome.SettingsDaemon.Power.Screen",
        "Brightness",
    );

    // PROXY playerctld Media player
    let player_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SESSION,
        "org.mpris.MediaPlayer2.playerctld",
        "/org/mpris/MediaPlayer2",
    );
    // PROP Volume
    let player_volume_property: DBusPropertyAdress =
        DBusPropertyAdress::new(player_proxy, "org.mpris.MediaPlayer2.Player", "Volume");

    // PROXY Battery status
    let battery_proxy: DBusProxyAdress = DBusProxyAdress::new(
        BusType::SYSTEM,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT1",
    );
    // PROP Volume
    let battery_level_property: DBusPropertyAdress =
        DBusPropertyAdress::new(battery_proxy, "org.freedesktop.UPower.Device", "Percentage");

    // ////////////
    // Configure the components to be displayed
    // ////////////

    let mut display_components: Vec<&mut dyn DisplayComponent> = Vec::new();

    const ICON_COLOR: BWRColor = COLOR_FG;

    // Dialog display brightness
    const BRIGHTNESS_ICON_COUNT: u32 = 6;
    let mut brightness_dialog = BarDialog::new(
        "brightness dialog",
        brightness_property,
        0,
        Box::new(|target: &mut BWRDisplay, val, center| {
            // const color = BWRColor::Off;
            match (val * BRIGHTNESS_ICON_COUNT as f64).floor() as u32 {
                6 => Image::with_center(&Brightness7::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                5 => Image::with_center(&Brightness6::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                4 => Image::with_center(&Brightness5::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                3 => Image::with_center(&Brightness4::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                2 => Image::with_center(&Brightness3::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                1 => Image::with_center(&Brightness2::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                0 | _ => Image::with_center(&Brightness1::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
            };
        }),
    );
    display_components.push(&mut brightness_dialog);

    // Dialog player volume
    const PLAYER_VOLUME_ICON_COUNT: u32 = 3;
    let mut player_volume_dialog = BarDialog::new(
        "player volume dialog",
        player_volume_property,
        1,
        Box::new(|target: &mut BWRDisplay, val, center| {
            match (val * PLAYER_VOLUME_ICON_COUNT as f64).ceil() as u16 {
                3 => Image::with_center(&VolumeHigh::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                2 => Image::with_center(&VolumeMedium::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                1 => Image::with_center(&VolumeLow::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                0 | _ => Image::with_center(&VolumeVariantOff::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
            };
        }),
    );
    display_components.push(&mut player_volume_dialog);

    let mut arch_icon = SimpleItem::new(
        "Arch Icon",
        Point::new(25, 25),
        0,
        Box::new(|target: &mut BWRDisplay, center| {
            Image::with_center(&Arch::new(ICON_COLOR), center)
                .draw(target)
                .ok();
        }),
    );
    display_components.push(&mut arch_icon);

    let mut battery_icon = StateItem::new(
        "Battery Icon",
        Point::new(60, 25),
        [battery_level_property].to_vec(),
        0,
        Box::new(|target: &mut BWRDisplay, val, center| {
            match ((val / 100.0) * PLAYER_VOLUME_ICON_COUNT as f64).ceil() as u16 {
                3 => Image::with_center(&VolumeHigh::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                2 => Image::with_center(&VolumeMedium::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                1 => Image::with_center(&VolumeLow::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
                0 | _ => Image::with_center(&VolumeVariantOff::new(ICON_COLOR), center)
                    .draw(target)
                    .ok(),
            };
        }),
    );
    display_components.push(&mut battery_icon);

    let background_small_bytes = include_bytes!("../resources/logo250.bmp");
    let background_small = Box::new(Bmp::<BWRColor>::from_slice(background_small_bytes).unwrap());

    //Background screen 0
    let mut background_0 = ImageBackground::new("Background 1", 0, background_small.clone());
    let mut background_1 = ImageBackground::new("Background 1", 1, background_small.clone());

    display_components.push(&mut background_0);
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
                .sorted_by(|a, b| Ord::cmp(&a.1, &b.1));

            // Draw the components to the screen's framebuffer
            for component in components {
                println!(
                    "{} Render Z:{} {}",
                    log::RENDER,
                    component.1,
                    component.0.get_name()
                );
                component.0.draw(display, values.clone())?;

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

                match component.0.get_type() {
                    DisplayAreaType::Dialog => {
                        screen_refresh_after[i] = component.0.get_refresh_at();
                        if let Some(time) = screen_refresh_after[i] {
                            println!(
                                "⏳️ Screen refresh after {}ms",
                                (time - Instant::now()).as_millis()
                            );
                        }
                        break;
                    }
                    // DisplayAreaType::Fullscreen => break,
                    _ => continue,
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
