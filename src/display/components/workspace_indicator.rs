use embedded_canvas::Canvas;
use embedded_graphics::{
    geometry::Size,
    primitives::{Circle, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle, StrokeAlignment},
    Drawable,
};

use crate::{
    display::{bwr_color::BWRColor, COLOR_FG},
    state::{app::ApplicationState, value::StateValueType},
};

use super::{ApplicationStateConsumer, DisplayComponent};

pub struct WorkspaceIndicator<'a> {
    pub name: &'a str,
    pub display: u8,
    pub area: Rectangle,
    pub properties: (&'a str, &'a str),
    pub old_state: ApplicationState, // Values last drawn
}

impl<'a> WorkspaceIndicator<'a> {
    pub fn new(
        name: &'a str,
        display: u8,
        area: Rectangle,
        properties: (&'a str, &'a str),
        initial_state: ApplicationState,
    ) -> Self {
        Self {
            name,
            display,
            area,
            old_state: initial_state,
            properties,
        }
    }
}

impl<'a> DisplayComponent for WorkspaceIndicator<'a> {
    fn get_display(&self) -> u8 {
        self.display
    }

    fn get_type(&self) -> super::DisplayAreaType {
        super::DisplayAreaType::DisplayArea(self.area)
    }

    fn get_name(&self) -> &str {
        self.name
    }

    fn draw(
        &mut self,
        target: &mut Canvas<BWRColor>,
        state: &ApplicationState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let active_prop = self.properties.0;
        let count_prop = self.properties.1;

        let StateValueType::U64(active) = state.get(active_prop).unwrap() else {
            panic!("Active Workspace is not a u64")
        };
        let StateValueType::U64(count) = state.get(count_prop).unwrap() else {
            panic!("Workspace Count is not a u64")
        };

        let active: u32 = *active as u32;
        let count: u32 = *count as u32;

        let mut inactive_style = PrimitiveStyle::with_stroke(COLOR_FG, 3);
        inactive_style.stroke_alignment = StrokeAlignment::Inside;
        let active_style = PrimitiveStyle::with_fill(COLOR_FG);

        let dot_size: Size = Size::new(20, 20);
        let dot_spacing = Size::new(10, 20);

        let active_width = dot_size.width + dot_spacing.width + dot_size.width;

        let total_width =
            active_width + ((count - 1) * dot_spacing.width) + ((count - 1) * dot_size.width);

        let size = Size::new(total_width, dot_size.height);

        let area = Rectangle::with_center(self.area.center() - self.area.top_left, size);
        let mut draw_point = area.top_left;

        for i in 0..count {
            if i == active {
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(draw_point, Size::new(active_width, dot_size.height)),
                    dot_size / 2,
                )
                .into_styled(active_style)
                .draw(target)?;

                draw_point.x += (active_width + dot_spacing.width) as i32;
            } else {
                Circle::new(draw_point, dot_size.width)
                    .into_styled(inactive_style)
                    .draw(target)?;

                draw_point.x += (dot_size.width + dot_spacing.width) as i32;
            }
        }

        self.old_state = state.clone();
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

impl<'a> ApplicationStateConsumer for WorkspaceIndicator<'a> {
    fn needs_refresh(&self, new_state: &ApplicationState) -> bool {
        let mut update = false;

        let active_prop = self.properties.0;
        let count_prop = self.properties.1;

        update |= self.old_state.get(active_prop) != new_state.get(active_prop);
        update |= self.old_state.get(count_prop) != new_state.get(count_prop);

        update
    }
}
