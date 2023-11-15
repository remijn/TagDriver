use embedded_graphics::{
    pixelcolor::{
        raw::{RawData, RawU2},
        BinaryColor, PixelColor, Rgb555, Rgb565, Rgb888,
    },
    prelude::RgbColor,
};

/// Binary color.
///
/// `BWR` is used for displays and images with 3 possible color states.
///
/// The interpretation of active and inactive states can be different for
/// different types of displays. A `BWRColor::On` might represent a black
/// pixel on an LCD and a white pixel on an OLED display.
///
/// To simplify the conversion from `BWRColor` to RGB and grayscale color
/// types the default conversions assume that `BWRColor::Off` is black and
/// `BWRColor::On` is white.
///
/// # Conversion between BWRColor and bool
///
/// ```
/// use embedded_graphics::pixelcolor::BWRColor;
///
/// // A BWRColor can be converted to a bool by using the is_on and is_off methods.
/// let color = BWRColor::On;
/// assert!(color.is_on());
/// assert!(color.invert().is_off());
///
/// // BWRColor implements From<bool>.
/// let bool_value = true;
/// let color: BWRColor = bool_value.into();
/// assert_eq!(color, BWRColor::On);
///
/// // this is equivalent to:
/// let bool_value = true;
/// let color = if bool_value {
///     BWRColor::On
/// } else {
///     BWRColor::Off
/// };
/// assert_eq!(color, BWRColor::On);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "defmt", derive(::defmt::Format))]
pub enum BWRColor {
    /// Inactive pixel.
    Off,

    /// Active pixel.
    On,

    /// Red pixel.
    Red,
}

impl Default for BWRColor {
    fn default() -> Self {
        Self::Off
    }
}

impl BWRColor {
    /// Inverts the color.
    ///
    /// # Examples
    ///
    /// ```
    /// use embedded_graphics::pixelcolor::BWRColor;
    ///
    /// assert_eq!(BWRColor::Off.invert(), BWRColor::On);
    /// assert_eq!(BWRColor::On.invert(), BWRColor::Off);
    /// ```
    #[inline]
    pub const fn invert(self) -> Self {
        match self {
            BWRColor::On => BWRColor::Off,
            BWRColor::Red => BWRColor::Off,
            BWRColor::Off => BWRColor::On,
        }
    }

    /// Returns `true` if this color is `On`.
    ///
    /// # Examples
    ///
    /// ```
    /// use embedded_graphics::pixelcolor::BWRColor;
    ///
    /// assert!(BWRColor::On.is_on());
    /// ```
    #[inline]
    pub const fn is_on(self) -> bool {
        matches!(self, BWRColor::On | BWRColor::Red)
    }

    /// Returns `true` if this color is `Off`.
    ///
    /// # Examples
    ///
    /// ```
    /// use embedded_graphics::pixelcolor::BWRColor;
    ///
    /// assert!(BWRColor::Off.is_off());
    /// ```
    #[inline]
    pub const fn is_off(self) -> bool {
        matches!(self, BWRColor::Off)
    }

    /// Maps active and inactive colors to a different type.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use embedded_graphics::pixelcolor::{BWRColor, Rgb565};
    /// let color = BWRColor::On;
    /// assert_eq!(color.map_color(Rgb565::RED, Rgb565::GREEN), Rgb565::GREEN)
    /// ```
    pub(crate) fn map_color<T>(self, value_off: T, value_on: T, value_red: T) -> T {
        match self {
            BWRColor::On => value_on,
            BWRColor::Off => value_off,
            BWRColor::Red => value_red,
        }
    }
}

impl PixelColor for BWRColor {
    type Raw = RawU2;
}

impl From<RawU2> for BWRColor {
    fn from(data: RawU2) -> Self {
        if data.into_inner() >= 2 {
            BWRColor::Red
        } else if data.into_inner() > 0 {
            BWRColor::On
        } else {
            BWRColor::Off
        }
    }
}

impl From<BWRColor> for RawU2 {
    fn from(color: BWRColor) -> Self {
        RawU2::new(color.map_color(0, 1, 2))
    }
}

impl From<bool> for BWRColor {
    fn from(value: bool) -> Self {
        if value {
            BWRColor::On
        } else {
            BWRColor::Off
        }
    }
}

impl From<BWRColor> for BinaryColor {
    fn from(color: BWRColor) -> Self {
        if color.is_on() {
            BinaryColor::On
        } else {
            BinaryColor::Off
        }
    }
}

impl From<BinaryColor> for BWRColor {
    fn from(value: BinaryColor) -> Self {
        if value.is_on() {
            BWRColor::On
        } else {
            BWRColor::Off
        }
    }
}

impl From<Rgb888> for BWRColor {
    fn from(color: Rgb888) -> Self {
        match color {
            Rgb888::RED => Self::Red,
            Rgb888::BLACK => Self::Off,
            Rgb888::WHITE => Self::On,
            _ => Self::Off,
        }
    }
}
impl From<Rgb555> for BWRColor {
    fn from(value: Rgb555) -> Self {
        Self::from(BinaryColor::from(value))
    }
}
impl From<Rgb565> for BWRColor {
    fn from(value: Rgb565) -> Self {
        Self::from(BinaryColor::from(value))
    }
}

impl From<BWRColor> for Rgb888 {
    fn from(value: BWRColor) -> Self {
        value.map_color(RgbColor::BLACK, RgbColor::WHITE, RgbColor::RED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::{IntoStorage, Rgb565, RgbColor};

    #[test]
    fn default_color_is_off() {
        assert_eq!(BWRColor::default(), BWRColor::Off);
    }

    #[test]
    fn invert_binary_color() {
        assert_eq!(BWRColor::Off.invert(), BWRColor::On);
        assert_eq!(BWRColor::On.invert(), BWRColor::Off);
    }

    #[test]
    fn map_binary_color() {
        assert_eq!(
            BWRColor::Off.map_color(Rgb565::BLUE, Rgb565::GREEN, Rgb565::RED),
            Rgb565::BLUE
        );
        assert_eq!(
            BWRColor::On.map_color(Rgb565::BLUE, Rgb565::GREEN, Rgb565::RED),
            Rgb565::GREEN
        );
        assert_eq!(
            BWRColor::Red.map_color(Rgb565::BLUE, Rgb565::GREEN, Rgb565::RED),
            Rgb565::RED
        );
    }

    #[test]
    fn from_data() {
        assert_eq!(BWRColor::from(RawU2::new(0)), BWRColor::Off);
        assert_eq!(BWRColor::from(RawU2::new(1)), BWRColor::On);
        assert_eq!(BWRColor::from(RawU2::new(2)), BWRColor::Red);
    }

    #[test]
    fn into_data() {
        assert_eq!(RawU2::from(BWRColor::Off).into_inner(), 0);
        assert_eq!(RawU2::from(BWRColor::On).into_inner(), 1);
        assert_eq!(RawU2::from(BWRColor::Red).into_inner(), 2);
    }

    #[test]
    fn from_bool() {
        assert_eq!(BWRColor::from(false), BWRColor::Off);
        assert_eq!(BWRColor::from(true), BWRColor::On);
    }

    #[test]
    fn is_on_off() {
        assert!(BWRColor::Off.is_off());
        assert!(!BWRColor::On.is_off());
        assert!(!BWRColor::Red.is_off());

        assert!(!BWRColor::Off.is_on());
        assert!(BWRColor::On.is_on());
        assert!(BWRColor::Red.is_on());
    }

    #[test]
    fn into_storage() {
        assert_eq!(BWRColor::Off.into_storage(), 0u8);
        assert_eq!(BWRColor::On.into_storage(), 1u8);
        assert_eq!(BWRColor::Red.into_storage(), 2u8);
    }
}
