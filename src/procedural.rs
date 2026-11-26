//! Animated wallpapers generated on the GPU. Each style is a small fragment
//! shader painted into a low-resolution texture (480x270, or 960x540 for
//! styles with defined edges) that is stretched to the screen like a photo:
//! the full-screen cost stays that of one textured quad. Frosted plates
//! sample the same texture, which is already blur-smooth.
//!
//! Motion is computed here, in f64, and handed to the shader as positions and
//! wrapped phases: mediump fragment floats on the TV never see a growing clock.

use std::f64::consts::TAU;

/// Draw path prefix: `procedural:<id>` animates, `procedural:<id>@still`
/// is one fixed frame (thumbnails, Reduce Motion).
pub const PREFIX: &str = "procedural:";
const STILL: &str = "@still";
pub const POINTS: usize = 8;
/// The still frame, and where live animation starts, so a wallpaper picked
/// from its thumbnail opens on the composition it showed.
const START: f64 = 41.;
const ASPECT: f64 = 16. / 9.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    /// Soft colour fields and a faint curtain.
    Aurora,
    /// Layered rolling hills of colour.
    Waves,
    /// Interfering sine fields through a colour ramp.
    Plasma,
    /// Rising and sinking metaballs.
    Lava,
    /// Folded, sheened fabric.
    Silk,
    /// Out-of-focus lights drifting.
    Bokeh,
    /// Four parallax layers of falling snow under a glow.
    Snow,
    /// Snow falling through a pine forest silhouette.
    SnowForest,
}

impl Style {
    pub const ALL: [Self; 8] = [
        Self::Aurora,
        Self::Waves,
        Self::Plasma,
        Self::Lava,
        Self::Silk,
        Self::Bokeh,
        Self::Snow,
        Self::SnowForest,
    ];
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
    /// Fragment shader body (after the shared header).
    pub fn shader(self) -> &'static str {
        match self {
            Self::Aurora => include_str!("shaders/procedural/aurora.frag"),
            Self::Waves => include_str!("shaders/procedural/waves.frag"),
            Self::Plasma => include_str!("shaders/procedural/plasma.frag"),
            Self::Lava => include_str!("shaders/procedural/lava.frag"),
            Self::Silk => include_str!("shaders/procedural/silk.frag"),
            Self::Bokeh => include_str!("shaders/procedural/bokeh.frag"),
            Self::Snow => include_str!("shaders/procedural/snow.frag"),
            Self::SnowForest => concat!(
                "#define TREES\n",
                include_str!("shaders/procedural/snow.frag")
            ),
        }
    }
    /// Render-target size: smooth styles need very few fragments; styles
    /// with edges get a quarter of 1080p so the stretch stays crisp.
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Waves | Self::Silk | Self::Lava | Self::Snow | Self::SnowForest => (960, 540),
            _ => (480, 270),
        }
    }
}

pub struct Palette {
    pub base: [f32; 3],
    pub accent: [f32; 3],
    pub colors: [[f32; 3]; 5],
}

pub struct Wallpaper {
    pub id: &'static str,
    /// English name (messages.json key in `i18n::wallpaper`).
    pub name: &'static str,
    pub style: Style,
    pub palette: Palette,
    /// Style tuning. Snow: fall speed, flake density, glow x, glow y.
    pub params: [f32; 4],
}

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.,
        ((hex >> 8) & 0xff) as f32 / 255.,
        (hex & 0xff) as f32 / 255.,
    ]
}

const fn wallpaper(
    id: &'static str,
    name: &'static str,
    style: Style,
    base: u32,
    accent: u32,
    colors: [u32; 5],
) -> Wallpaper {
    Wallpaper {
        id,
        name,
        style,
        palette: Palette {
            base: rgb(base),
            accent: rgb(accent),
            colors: [
                rgb(colors[0]),
                rgb(colors[1]),
                rgb(colors[2]),
                rgb(colors[3]),
                rgb(colors[4]),
            ],
        },
        params: [0.; 4],
    }
}

/// A snow wallpaper: `flakes` are the four layers' colours far to near,
/// `ground` the bottom of the sky gradient, `glow` its light (colour and
/// position), `speed` and `density` (0..1) the snowfall.
#[allow(clippy::too_many_arguments)]
const fn snow(
    id: &'static str,
    name: &'static str,
    style: Style,
    sky: u32,
    ground: u32,
    glow: (u32, f32, f32),
    flakes: [u32; 4],
    speed: f32,
    density: f32,
) -> Wallpaper {
    let mut w = wallpaper(
        id,
        name,
        style,
        sky,
        glow.0,
        [flakes[0], flakes[1], flakes[2], flakes[3], ground],
    );
    w.params = [speed, density, glow.1, glow.2];
    w
}

/// Settings › Wallpaper › Animated, in gallery order.
pub const ANIMATED: [Wallpaper; 18] = [
    wallpaper(
        "aurora",
        "Aurora",
        Style::Aurora,
        0x040812,
        0x3df2b0,
        [0x0f8f7a, 0x2a3fb8, 0x5b2ea6, 0x16a36b, 0x123a7a],
    ),
    wallpaper(
        "aurora-ember",
        "Ember",
        Style::Aurora,
        0x0d0408,
        0xff8a4c,
        [0xb8323f, 0x5e1f8a, 0xd0602a, 0x7a1446, 0x2b1660],
    ),
    wallpaper(
        "aurora-lagoon",
        "Lagoon",
        Style::Aurora,
        0x020b12,
        0x5ee7ff,
        [0x0a6c9c, 0x0f8f8a, 0x1b3f9e, 0x1aa6b8, 0x2a2478],
    ),
    wallpaper(
        "waves-dusk",
        "Dusk waves",
        Style::Waves,
        0x1a0f2e,
        0xffb37a,
        [0x6b3a7a, 0x8a3f6e, 0x5a2a5e, 0x3a1d4a, 0x1d1030],
    ),
    wallpaper(
        "waves-ocean",
        "Ocean waves",
        Style::Waves,
        0x061a2e,
        0x9fe3ff,
        [0x1f6f9a, 0x195a86, 0x124670, 0x0c3358, 0x061e38],
    ),
    wallpaper(
        "plasma-neon",
        "Neon plasma",
        Style::Plasma,
        0x05030c,
        0xffffff,
        [0x1a0a4a, 0x6a1bb0, 0xd0338a, 0x2a6ad8, 0x0c1a40],
    ),
    wallpaper(
        "plasma-pastel",
        "Pastel plasma",
        Style::Plasma,
        0x0e0c16,
        0xffffff,
        [0x3a4a78, 0x6a5a8c, 0x8c6078, 0x4a7a8a, 0x2e3a62],
    ),
    wallpaper(
        "lava",
        "Lava lamp",
        Style::Lava,
        0x14061e,
        0xff6a2a,
        [0xe0942c, 0xd8661e, 0xb83a1e, 0x8c1c36, 0x520e36],
    ),
    wallpaper(
        "lava-mint",
        "Mint lamp",
        Style::Lava,
        0x041418,
        0x4affc8,
        [0x86e0c0, 0x46c8a2, 0x1c9e88, 0x0e7072, 0x08465a],
    ),
    wallpaper(
        "silk",
        "Silk",
        Style::Silk,
        0x0a0806,
        0xffe2b0,
        [0x140e08, 0x2e2214, 0x5a4226, 0x8a6a3e, 0xb89058],
    ),
    wallpaper(
        "silk-rose",
        "Rose silk",
        Style::Silk,
        0x0e0609,
        0xffc8d8,
        [0x16080c, 0x34121e, 0x5e2236, 0x8a3e56, 0xb46a84],
    ),
    wallpaper(
        "bokeh",
        "City lights",
        Style::Bokeh,
        0x07060c,
        0x000000,
        [0xffb04a, 0xff6a3a, 0xffd88a, 0xe84a6a, 0x8a6aff],
    ),
    wallpaper(
        "bokeh-blue",
        "Blue hour",
        Style::Bokeh,
        0x030814,
        0x000000,
        [0x4aa8ff, 0x6ae0ff, 0x8a7aff, 0x2a6aff, 0xc0e8ff],
    ),
    // Christmas: snowfall in a few moods.
    snow(
        "snowfall",
        "Snowfall",
        Style::Snow,
        0x07122a,
        0x1c3658,
        (0x8fb4ff, 1.42, 0.14),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        1.,
        0.42,
    ),
    snow(
        "snow-forest",
        "Snowy forest",
        Style::SnowForest,
        0x0a1630,
        0x2a4468,
        (0xa8c4ff, 0.42, 0.12),
        [0x9fb2cc, 0xc4d4ea, 0xe6eef8, 0xffffff],
        0.85,
        0.38,
    ),
    snow(
        "christmas-eve",
        "Christmas Eve",
        Style::Snow,
        0x22040a,
        0x5a1018,
        (0xffa040, 0.89, 1.08),
        [0xc8a890, 0xe4c8aa, 0xf6e2c8, 0xfff4e4],
        0.75,
        0.36,
    ),
    snow(
        "christmas-forest",
        "Christmas forest",
        Style::SnowForest,
        0x03140f,
        0x0e3426,
        (0xffb85a, 1.3, 0.78),
        [0x9cb8a8, 0xc4dccc, 0xe4f2e8, 0xfffaf0],
        0.7,
        0.36,
    ),
    snow(
        "blizzard",
        "Blizzard",
        Style::Snow,
        0x1a222c,
        0x56667a,
        (0xa0b0c4, 0.4, 0.2),
        [0x9aa6b4, 0xbcc6d2, 0xdde4ec, 0xffffff],
        2.4,
        0.72,
    ),
];

pub fn find(id: &str) -> Option<&'static Wallpaper> {
    ANIMATED.iter().find(|w| w.id == id)
}

pub fn is_procedural(id: &str) -> bool {
    find(id).is_some()
}

/// The draw path of wallpaper `id`.
pub fn path(id: &str, live: bool) -> String {
    format!("{PREFIX}{id}{}", if live { "" } else { STILL })
}

/// The wallpaper of a draw path, and whether it animates.
pub fn parse(path: &str) -> Option<(&'static Wallpaper, bool)> {
    let rest = path.strip_prefix(PREFIX)?;
    match rest.strip_suffix(STILL) {
        Some(id) => find(id).map(|w| (w, false)),
        None => find(rest).map(|w| (w, true)),
    }
}

/// Shader inputs of one moment. Coordinates are in screen heights with the
/// origin top-left, so x runs to 16/9. What each point means is per style.
pub struct Frame {
    pub points: [[f32; 4]; POINTS],
    pub phase: [f32; 4],
}

fn wrap(v: f64) -> f32 {
    v.rem_euclid(TAU) as f32
}

/// The composition of `style` `seconds` into the animation (`None`: still).
pub fn frame(wallpaper: &Wallpaper, seconds: Option<f64>) -> Frame {
    let style = wallpaper.style;
    let t = START + seconds.unwrap_or(0.);
    let mut points = [[0.; 4]; POINTS];
    let mut phase = [0.; 4];
    match style {
        Style::Aurora => {
            // Centre x, centre y, 1/r², strength of five colour fields.
            for (i, p) in points.iter_mut().take(5).enumerate() {
                let k = i as f64;
                // Incommensurate slow speeds: the layout never visibly repeats.
                let sx = 0.031 + 0.011 * k;
                let sy = 0.023 + 0.009 * ((k * 1.7) % 3.);
                let radius = 0.42 + 0.08 * (t * 0.05 + k).sin() + 0.04 * k;
                *p = [
                    (ASPECT * (0.5 + 0.42 * (t * sx + k * 1.9).sin())) as f32,
                    (0.5 + 0.36 * (t * sy + k * 2.6).cos()) as f32,
                    (1. / (radius * radius)) as f32,
                    (0.82 - 0.06 * k) as f32,
                ];
            }
            phase = [
                wrap(t * 0.13),
                wrap(t * 0.11 + 1.),
                wrap(t * 0.19),
                wrap(t * 0.27 + 2.),
            ];
        }
        Style::Waves => {
            // Two travelling phases per layer, back to front.
            for (i, p) in points.iter_mut().take(5).enumerate() {
                let k = i as f64;
                *p = [
                    wrap(t * (0.07 + 0.025 * k) + k * 1.3),
                    wrap(-t * (0.05 + 0.02 * k) + k * 2.1),
                    0.,
                    0.,
                ];
            }
        }
        Style::Plasma => {
            points[0] = [
                (ASPECT * (0.5 + 0.3 * (t * 0.037).sin())) as f32,
                (0.5 + 0.3 * (t * 0.029 + 1.).cos()) as f32,
                0.,
                0.,
            ];
            phase = [
                wrap(t * 0.11),
                wrap(t * 0.083 + 1.),
                wrap(t * 0.061 + 2.),
                wrap(t * 0.17),
            ];
        }
        Style::Lava => {
            // Centre x, centre y, r² of six blobs rising and sinking.
            for (i, p) in points.iter_mut().take(6).enumerate() {
                let k = i as f64;
                let r = 0.115 + 0.02 * ((k * 2.3).sin() + 1.);
                *p = [
                    (ASPECT * (0.18 + 0.13 * k + 0.05 * (t * 0.05 + k).sin())) as f32,
                    (0.5 + 0.48 * (t * (0.045 + 0.012 * k) + k * 1.7).sin()) as f32,
                    (r * r) as f32,
                    0.,
                ];
            }
        }
        Style::Silk => {
            phase = [
                wrap(t * 0.09),
                wrap(t * 0.07 + 1.),
                wrap(t * 0.05 + 2.),
                wrap(t * 0.12 + 3.),
            ];
        }
        // Filled in below: snow needs the wallpaper's own tuning.
        Style::Snow | Style::SnowForest => {}
        Style::Bokeh => {
            // Centre x, centre y, radius, brightness of eight lights.
            for (i, p) in points.iter_mut().enumerate() {
                let k = i as f64;
                *p = [
                    (ASPECT * (0.5 + 0.55 * (t * (0.017 + 0.004 * k) + k * 2.4).sin())) as f32,
                    (0.5 + 0.5 * (t * (0.013 + 0.003 * k) + k * 1.1).cos()) as f32,
                    (0.07 + 0.012 * ((k * 3.7) % 7.)) as f32,
                    (0.45 + 0.25 * (t * 0.21 + k * 1.9).sin()) as f32,
                ];
            }
        }
    }
    if matches!(style, Style::Snow | Style::SnowForest) {
        // Per layer, far to near: cells per screen height, fall speed in
        // screen heights per second, flake radius in cells, brightness.
        const LAYERS: [(f64, f64, f32, f32); 4] = [
            (34., 0.022, 0.13, 0.42),
            (24., 0.034, 0.14, 0.6),
            (16., 0.05, 0.15, 0.82),
            (10., 0.075, 0.16, 1.),
        ];
        let [speed, density, ..] = wallpaper.params;
        let speed = f64::from(speed);
        for (i, (cells, fall, radius, bright)) in LAYERS.into_iter().enumerate() {
            let k = i as f64;
            // The noise texture repeats every 64 cells: offsets wrap there
            // with no seam, and split into whole cells and a fraction so the
            // shader's coordinates stay small enough for mediump.
            let down = (t * fall * speed * cells).rem_euclid(64.);
            let across = (t * fall * speed * 0.3 * speed * cells).rem_euclid(64.);
            points[i] = [
                down.fract() as f32,
                down.floor() as f32,
                cells as f32,
                radius,
            ];
            points[4 + i] = [
                wrap(t * (0.5 + 0.12 * k) + k),
                density,
                bright,
                across.fract() as f32,
            ];
            phase[i] = across.floor() as f32;
        }
    }
    Frame { points, phase }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_round_trip() {
        assert!(parse(&path("aurora", true)).is_some_and(|(_, live)| live));
        assert!(parse(&path("silk-rose", false)).is_some_and(|(_, live)| !live));
        assert!(parse("procedural:nope").is_none());
        assert!(parse("wallpapers/dunes.jpg").is_none());
    }

    #[test]
    fn catalogue_ids_are_unique_and_cover_every_style() {
        let mut ids: Vec<_> = ANIMATED.iter().map(|w| w.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ANIMATED.len());
        for style in Style::ALL {
            assert!(ANIMATED.iter().any(|w| w.style == style));
        }
    }

    #[test]
    fn inputs_stay_small_for_mediump() {
        // Phases are wrapped to a turn; snow's whole-cell offsets to 64.
        for wallpaper in &ANIMATED {
            let f = frame(wallpaper, Some(86_400. * 30.));
            assert!(f.phase.iter().all(|p| (0. ..64.).contains(p)));
            assert!(f.points.iter().flatten().all(|v| v.abs() < 64.));
        }
    }
}
