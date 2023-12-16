use std::{
    io::{self},
    sync::Arc,
    time::{Duration, Instant},
};
#[macro_use]
extern crate enum_primitive;
mod dbus;
mod display;
mod eink;
mod log;
mod state;

use colored::Colorize;
use display::{
    bwr_color::BWRColor, bwr_display::BWRDisplay, components::DisplayAreaType, COLOR_BG,
};
use eink::thread::start_eink_thread;

use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::{OriginDimensions, Point, Size},
    prelude::DrawTarget,
    Drawable,
};

// impl Into<IconObj<T> for Icon<C, T> {}

use itertools::Itertools;
use tokio::{
    sync::{mpsc, Mutex},
    time::sleep,
};

use crate::{
    dbus::dbus_interface::run_dbus_thread,
    display::{components::make_ui_components, DisplayRotation},
    state::{build_state_map, value::StateValueType},
};

const DISPLAY_COUNT: u8 = 3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", log::WELCOME.blue());

    // ////////////
    // Setup the EInk interface threads, these handle the uart
    // ////////////

    let mut displays = [
        (
            BWRDisplay::new(250, 122, DisplayRotation::Zero, display::DisplayFlip::None),
            start_eink_thread(
                "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if00",
                912600,
                250,
                122,
            )?,
        ),
        (
            BWRDisplay::new(
                250,
                122,
                DisplayRotation::Rotate180,
                display::DisplayFlip::None,
            ),
            start_eink_thread(
                "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if04",
                912600,
                250,
                122,
            )?,
        ),
        (
            BWRDisplay::new(
                400,
                300,
                DisplayRotation::Rotate270,
                display::DisplayFlip::Horizontal,
            ),
            start_eink_thread(
                "/dev/serial/by-id/usb-RemijnPi_Eink_Driver_DE6270431F67292B-if02",
                912600,
                300,
                400,
            )?,
        ),
    ];

    // Setup the global app state

    let state = Arc::new(Mutex::new(build_state_map()));

    let (state_update_tx, mut state_update_rx) = mpsc::channel::<()>(20);

    // Star the stdin thread
    let stdin_state = state.clone();
    let stdin_update_tx = state_update_tx.clone();
    tokio::spawn(async move {
        loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();

            let input = buffer.trim();

            let parts: Vec<&str> = input.split(' ').collect();

            if parts.is_empty() {
                continue;
            }

            let first = *parts.first().unwrap();

            match first {
                "state" => {
                    let lock = stdin_state.lock().await;
                    println!(
                        "{}{}\n{}",
                        log::STATE,
                        "Application State: ".green(),
                        serde_json::to_string_pretty(&*lock).expect("cant get json")
                    );
                    drop(lock);
                }
                "show" if parts.len() > 1 => {
                    let mut lock = stdin_state.lock().await;

                    let updated = lock
                        .update(
                            "rear-image-path",
                            Some(StateValueType::String(parts.get(1).unwrap().to_string())),
                        )
                        .expect("Could not set rear display image");
                    drop(lock);
                    if updated {
                        stdin_update_tx.send(()).await.unwrap();
                    }
                }
                "show" if parts.len() == 1 => {
                    let updated = stdin_state
                        .lock()
                        .await
                        .update("rear-image-path", None)
                        .expect("Count not reset rear display image");
                    if updated {
                        stdin_update_tx.send(()).await.unwrap();
                    }
                }
                _ => println!("{} Unknown command {}", log::WARN, buffer.trim().red()),
            }
        }
    });

    // Start the dbus thread
    let dbus_state = state.clone();
    let dbus_update_tx = state_update_tx.clone();
    tokio::spawn(async move {
        run_dbus_thread(dbus_update_tx, dbus_state)
            .await
            .expect("DBus thread crashed");
    });

    // let dbus get the default values before we lock the state
    sleep(Duration::from_millis(10)).await;

    let state_lock = state.lock().await;

    let mut ui_components = make_ui_components(state_lock.clone());

    drop(state_lock);

    let mut display_refresh_after: [Option<Instant>; DISPLAY_COUNT as usize] =
        [Some(Instant::now()); DISPLAY_COUNT as usize];

    // ////////////
    // Run the main loop
    // ////////////
    loop {
        let mut display_needs_refresh: Vec<bool> = [false; DISPLAY_COUNT as usize].to_vec().clone();

        // Proccess state updates for each component and
        // set refresh for the displays with the components that need it
        while state_update_rx.try_recv().is_ok() {
            // We have new values, check with each component if this new state requires a refresh
            let state_lock = state.lock().await;

            for component in ui_components.iter() {
                let mut component_needs_refresh = false;

                if let Some(state_consumer) = component.state_consumer() {
                    // Is updated needed by state consumer?
                    component_needs_refresh = state_consumer.needs_refresh(&state_lock);
                }

                if component_needs_refresh {
                    println!(
                        "{} Component \"{}\" requests update on display {}",
                        log::DISPLAY,
                        component.get_name(),
                        component.get_display()
                    )
                }
                display_needs_refresh[component.get_display() as usize] |= component_needs_refresh;
            }
        }

        // Set refresh if the timeout has been hit
        for i in 0..DISPLAY_COUNT {
            if let Some(time) = display_refresh_after[i as usize] {
                if time < Instant::now() {
                    display_needs_refresh[i as usize] = true;
                    display_refresh_after[i as usize] = None;
                    println!("{} Refresh After on display {}", log::DISPLAY, i)
                }
            }
        }

        //Loop through the displays that need a refresh
        for (i, (display, interface)) in displays
            .iter_mut()
            .enumerate()
            .filter(|v| display_needs_refresh[v.0])
        {
            // Display i needs an update, lets wrender
            println!("{} Rendering display {}", log::RENDER, i);

            // clear the display
            display.clear(COLOR_BG)?;
            let values = Box::new(state.lock().await.clone());

            // list of components filtered by the current display, mapped to zindex, and then sorted
            let components = ui_components
                .iter_mut()
                .filter(|component| component.get_display() == i as u8)
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

                let size: Size = match component.0.get_type() {
                    DisplayAreaType::Icon(icon_size) => icon_size,
                    DisplayAreaType::Fullscreen => display.size(),
                    DisplayAreaType::Dialog => display.size(),
                    DisplayAreaType::DisplayArea(area) => area.size,
                };

                let mut canvas = {
                    // draw a rectangle smaller than the canvas (with 1px)
                    // let canvas_rectangle = Rectangle::new(Point::zero(), size);

                    // let canvas_outline = canvas_rectangle.into_styled(OUTLINE_STYLE_FG);
                    // draw the canvas rectangle for debugging
                    // canvas_outline.draw(&mut canvas)?;

                    Canvas::<BWRColor>::new(size)
                };

                component.0.draw(&mut canvas, &values)?;

                let refresh = component.0.get_refresh_at();
                if refresh.is_some()
                    && (display_refresh_after[i].is_none()
                        || refresh.expect("") > display_refresh_after[i].expect(""))
                {
                    display_refresh_after[i] = component.0.get_refresh_at();
                    println!(
                        "⏳️ Display refresh after {}ms",
                        (display_refresh_after[i].expect("") - Instant::now()).as_millis()
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

            let total_icons_width = canvases
                .iter()
                .filter_map(|(_canvas, area)| {
                    if let DisplayAreaType::Icon(size) = area {
                        Some(size.width)
                    } else {
                        None
                    }
                })
                .reduce(|v, a| v + a)
                .unwrap_or(0);

            let mut pos = Point::new(((display.size().width - total_icons_width) / 2) as i32, 10);

            for canvas in canvases {
                match canvas.1 {
                    DisplayAreaType::Fullscreen | DisplayAreaType::Dialog => canvas
                        .0
                        .place_at(Point::zero())
                        .draw(display)
                        .expect("Could not draw canvas to display"),

                    DisplayAreaType::Icon(size) => {
                        canvas
                            .0
                            .place_at(pos)
                            .draw(display)
                            .expect("Could not draw canvas to display");
                        pos += Size::new(size.width, 0);
                    }
                    DisplayAreaType::DisplayArea(area) => {
                        canvas
                            .0
                            .place_at(area.top_left)
                            .draw(display)
                            .expect("Could not draw canvas to display");
                    }
                }
            }

            drop(values);

            let (black, _red) = display.get_fixed_buffer();

            interface.black_border = true;

            // Stupid hack to force full-refresh the right display
            if !interface._port.ends_with("if00") {
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
