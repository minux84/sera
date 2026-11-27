//! Animated wallpapers generated on the GPU. Each style is a small fragment
//! shader painted into a low-resolution texture (480x270, or 960x540 for
//! styles with defined edges) that is stretched to the screen like a photo:
//! the full-screen cost stays that of one textured quad. Frosted plates
//! sample the same texture, which is already blur-smooth.
//!
//! Motion is computed here, in f64, and handed to the shader as positions and
//! wrapped phases: mediump fragment floats on the TV never see a growing clock.

use std::f64::consts::TAU;

/// Where the shader sources live in the checkout, for `--live-shaders`.
pub fn source_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders/procedural")
}

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
    /// Snowy mountains and pines whose sky, light, sun and moon follow the
    /// local time of day.
    SnowDay,
    /// The Winter day scene reacting to the weather widget's conditions.
    WeatherDay,
}

impl Style {
    pub const ALL: [Self; 10] = [
        Self::Aurora,
        Self::Waves,
        Self::Plasma,
        Self::Lava,
        Self::Silk,
        Self::Bokeh,
        Self::Snow,
        Self::SnowForest,
        Self::SnowDay,
        Self::WeatherDay,
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
            Self::SnowDay => include_str!("shaders/procedural/snowday.frag"),
            Self::WeatherDay => concat!(
                "#define WEATHER\n",
                include_str!("shaders/procedural/snowday.frag")
            ),
            Self::SnowForest => concat!(
                "#define TREES\n",
                include_str!("shaders/procedural/snow.frag")
            ),
        }
    }
    /// Source file of the style in `shaders/procedural/`.
    pub fn file(self) -> &'static str {
        match self {
            Self::Aurora => "aurora.frag",
            Self::Waves => "waves.frag",
            Self::Plasma => "plasma.frag",
            Self::Lava => "lava.frag",
            Self::Silk => "silk.frag",
            Self::Bokeh => "bokeh.frag",
            Self::Snow | Self::SnowForest => "snow.frag",
            Self::SnowDay | Self::WeatherDay => "snowday.frag",
        }
    }
    /// Preprocessor lines the style prepends to its file.
    fn defines(self) -> &'static str {
        match self {
            Self::SnowForest => "#define TREES\n",
            Self::WeatherDay => "#define WEATHER\n",
            _ => "",
        }
    }
    /// Fragment body read from `dir` (live shader preview), defines included.
    pub fn read_shader(self, dir: &std::path::Path) -> std::io::Result<String> {
        Ok(self.defines().to_owned() + &std::fs::read_to_string(dir.join(self.file()))?)
    }
    /// Render-target size: smooth styles need very few fragments; styles
    /// with edges get a quarter of 1080p so the stretch stays crisp.
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Waves
            | Self::Silk
            | Self::Lava
            | Self::Snow
            | Self::SnowForest
            | Self::SnowDay
            | Self::WeatherDay => (960, 540),
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
pub const ANIMATED: [Wallpaper; 23] = [
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
    // Sky, light and colours come from the time of day (see `daylight`).
    snow(
        "winter-day",
        "Winter day",
        Style::SnowDay,
        0x050a1c,
        0x16243f,
        (0x9db8ff, 0., 0.),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        0.8,
        0.36,
    ),
    // The same scene driven by the weather widget (see `Conditions`).
    snow(
        "live-weather",
        "Live weather",
        Style::WeatherDay,
        0x050a1c,
        0x16243f,
        (0x9db8ff, 0., 0.),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        0.8,
        0.36,
    ),
    // The other seasons of the same scene, fixed: (day of year, cloud cover)
    // in place of the glow position. No snow falls.
    snow(
        "spring-day",
        "Spring day",
        Style::WeatherDay,
        0x050a1c,
        0x16243f,
        (0x9db8ff, 140., 0.45),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        0.8,
        0.,
    ),
    snow(
        "summer-day",
        "Summer day",
        Style::WeatherDay,
        0x050a1c,
        0x16243f,
        (0x9db8ff, 196., 0.),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        0.8,
        0.,
    ),
    snow(
        "autumn-day",
        "Autumn day",
        Style::WeatherDay,
        0x050a1c,
        0x16243f,
        (0x9db8ff, 298., 0.45),
        [0xa8bcd8, 0xc8d8ee, 0xe4eefa, 0xffffff],
        0.8,
        0.,
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
    /// Live weather: (cloud cover, rain 0/1, fog, lightning flash).
    pub weather: [f32; 4],
    /// Live weather: (snow on the ground, greenery, flowers, autumn leaves).
    pub season: [f32; 4],
    pub points: [[f32; 4]; POINTS],
    pub phase: [f32; 4],
    /// Colours and tuning that change over time (time of day), replacing
    /// the wallpaper's own.
    pub palette: Option<Palette>,
    pub params: Option<[f32; 4]>,
    /// Slow drift in [0, 1) (Winter day's clouds; integer multiples of it
    /// stay seamless where it wraps).
    pub drift: [f32; 4],
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
        Style::Snow | Style::SnowForest | Style::SnowDay | Style::WeatherDay => {}
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
    // Seasonal days pin a day of the year (params[2]) and a cloud cover
    // (params[3]) instead of following the widget; Live weather has 0 there.
    let fixed_day = (style == Style::WeatherDay && wallpaper.params[2] > 0.)
        .then_some((wallpaper.params[2], wallpaper.params[3]));
    let weather = (style == Style::WeatherDay).then(|| match fixed_day {
        Some((_, cover)) => Conditions {
            cover,
            ..Conditions::CLEAR
        },
        None => conditions(),
    });
    // The season sets the hours of daylight, mapped onto the winter
    // keyframes so dawn and dusk land at the real times.
    // The landscape's season: fixed for Spring/Summer/Autumn day, today's
    // for Live weather.
    let season = weather.as_ref().map(|_| match fixed_day {
        Some((day, _)) => Season::on(day, latitude()),
        None => Season::now(latitude()),
    });
    // The sun always follows today's real sunrise and sunset, whatever
    // season the landscape shows (Winter day included).
    let daylight_now = matches!(style, Style::SnowDay | Style::WeatherDay).then(|| {
        let hour = day_hour();
        daylight(Season::now(latitude()).winter_hour(hour))
    });
    if matches!(
        style,
        Style::Snow | Style::SnowForest | Style::SnowDay | Style::WeatherDay
    ) {
        // Per layer, far to near: cells per screen height, fall speed in
        // screen heights per second, flake radius in cells, brightness.
        const LAYERS: [(f64, f64, f32, f32); 4] = [
            (34., 0.022, 0.13, 0.42),
            (24., 0.034, 0.14, 0.6),
            (16., 0.05, 0.15, 0.82),
            (10., 0.075, 0.16, 1.),
        ];
        let [speed, density, ..] = wallpaper.params;
        // Winter day snows less by day than by night; Live weather only
        // when and as hard as it really snows or rains.
        let density = match &weather {
            Some(w) => density * 1.6 * w.precipitation,
            None => density * daylight_now.as_ref().map_or(1., |d| d.2),
        };
        // Rain falls fast; wind slants whatever falls.
        let speed = f64::from(speed);
        let (fall_speed, wind) = match &weather {
            Some(w) => (
                speed * if w.rain { 7. } else { 1. },
                0.08 + f64::from(w.wind) / 40.,
            ),
            None => (speed, 0.3 * speed),
        };
        for (i, (cells, fall, radius, bright)) in LAYERS.into_iter().enumerate() {
            let k = i as f64;
            // The noise texture repeats every 64 cells: offsets wrap there
            // with no seam, and split into whole cells and a fraction so the
            // shader's coordinates stay small enough for mediump.
            let down = (t * fall * fall_speed * cells).rem_euclid(64.);
            let across = (t * fall * fall_speed * wind * cells).rem_euclid(64.);
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
    let (palette, params) = match daylight_now {
        Some((mut palette, mut params, _)) => {
            if let Some(s) = &season {
                // Autumn light is warm and golden; summer skies a deeper blue.
                let warm = [1.0, 0.72, 0.4];
                palette.accent = mix(palette.accent, warm, s.autumn * 0.5);
                palette.colors[4] = mix(
                    palette.colors[4],
                    mix(palette.colors[4], warm, 0.5),
                    s.autumn * 0.35,
                );
                let summer = (1. - s.snow) * (1. - s.autumn) * (1. - s.flowers * 0.5);
                let deep = |c: [f32; 3]| {
                    let l = (c[0] + c[1] + c[2]) / 3.;
                    [0, 1, 2].map(|i| (l + (c[i] - l) * 1.35).clamp(0., 1.))
                };
                palette.base = mix(palette.base, deep(palette.base), summer * 0.6);
                palette.colors[4] = mix(palette.colors[4], deep(palette.colors[4]), summer * 0.6);
            }
            if let Some(w) = &weather {
                overcast_light(&mut palette, &mut params, w);
            }
            (Some(palette), Some(params))
        }
        None => (None, None),
    };
    // Live weather: cloud cover, rain instead of snow, fog, lightning.
    let weather_uniform = weather.map_or([0.; 4], |w| {
        [
            w.cover,
            if w.rain { 1. } else { 0. },
            w.fog,
            if w.storm { lightning(t) } else { 0. },
        ]
    });
    // Clouds cross the sky in roughly three minutes at the base speed. The
    // overcast row drifts in screen heights and repeats every 4 (twice that
    // for its back layer, which moves 1.5x as fast): wrapping at 8 is seamless.
    let drift = [
        (t * 0.006).fract() as f32,
        (t * 0.004).rem_euclid(8.) as f32,
        0.,
        0.,
    ];
    // Ground and trees: (snow cover, greenery, flowers, autumn).
    let season_uniform = match (season, &weather) {
        (Some(s), Some(w)) => {
            // Real snowfall whitens the ground whatever the date.
            let snow = if w.rain {
                s.snow
            } else {
                s.snow.max(w.precipitation.min(0.85))
            };
            [
                snow,
                (1. - snow).max(0.),
                s.flowers * (1. - snow),
                s.autumn * (1. - snow),
            ]
        }
        _ => [1., 0., 0., 0.],
    };
    Frame {
        season: season_uniform,
        weather: weather_uniform,
        points,
        phase,
        palette,
        params,
        drift,
    }
}

/// Weather the Live weather wallpaper shows, from the widget's conditions.
#[derive(Clone, Copy)]
pub struct Conditions {
    /// 0 clear sky .. 1 fully overcast.
    pub cover: f32,
    /// 0 none .. 1 heavy snow or rain.
    pub precipitation: f32,
    pub rain: bool,
    pub fog: f32,
    /// km/h.
    pub wind: f32,
    pub storm: bool,
}

impl Conditions {
    const CLEAR: Self = Self {
        cover: 0.,
        precipitation: 0.,
        rain: false,
        fog: 0.,
        wind: 0.,
        storm: false,
    };

    /// From the widget's mood and, when there is one, its reading (the WMO
    /// code sets light / moderate / heavy precipitation).
    pub fn from_weather(
        mood: crate::weather_sky::Mood,
        reading: Option<crate::weather::Reading>,
    ) -> Self {
        use crate::weather_sky::Mood;
        let code = reading.map_or(0, |r| r.code);
        let intensity = match code {
            51 | 56 | 61 | 66 | 71 | 77 | 80 | 85 => 0.45,
            55 | 57 | 65 | 67 | 75 | 82 | 86 | 99 => 1.,
            _ => 0.75,
        };
        let mut c = Self::CLEAR;
        c.wind = reading.map_or(0., |r| r.wind_speed as f32);
        match mood {
            Mood::Clear | Mood::Night => {}
            Mood::Partly => c.cover = 0.45,
            Mood::Wind => {
                c.cover = 0.5;
                c.wind = c.wind.max(40.);
            }
            Mood::Cloudy | Mood::NightCloudy => c.cover = 0.9,
            Mood::Fog => {
                c.cover = 0.9;
                c.fog = 1.;
            }
            Mood::Rain => {
                c.cover = 1.;
                c.rain = true;
                c.precipitation = intensity;
            }
            Mood::Storm => {
                c.cover = 1.;
                c.rain = true;
                c.storm = true;
                c.precipitation = 1.;
            }
            Mood::Snow => {
                c.cover = 1.;
                c.precipitation = intensity;
            }
        }
        c
    }
}

static WEATHER: std::sync::Mutex<Conditions> = std::sync::Mutex::new(Conditions::CLEAR);

/// The widget's current weather, for the frame being built.
pub fn set_weather(conditions: Conditions) {
    if let Ok(mut current) = WEATHER.lock() {
        *current = conditions;
    }
}

fn conditions() -> Conditions {
    WEATHER.lock().map_or(Conditions::CLEAR, |c| *c)
}

/// A grey, dimmer sky under cloud: colours lose saturation and light, the
/// sun or moon's light greys, stars hide.
fn overcast_light(palette: &mut Palette, params: &mut [f32; 4], w: &Conditions) {
    // Overcast skies lose almost all their colour; rain and storms darken
    // the whole scene on top of that.
    let dark = w.cover * 0.22
        + if w.rain { 0.15 * w.precipitation } else { 0. }
        + if w.storm { 0.25 } else { 0. }
        + w.fog * 0.05;
    let grey = |c: [f32; 3]| {
        let l = 0.3 * c[0] + 0.55 * c[1] + 0.15 * c[2];
        mix(c, [l, l * 1.01, l * 1.05], w.cover * 0.9).map(|v| v * (1. - dark))
    };
    palette.base = grey(palette.base);
    palette.colors[4] = grey(palette.colors[4]);
    for flake in palette.colors.iter_mut().take(4) {
        *flake = mix(*flake, grey(*flake), 0.5);
    }
    palette.accent = mix(palette.accent, [0.75, 0.78, 0.82], w.cover * 0.8);
    params[2] *= 1. - dark * 0.7;
    params[3] *= 1. - w.cover;
}

/// Brief lightning flashes at irregular moments (one chance every 6 s).
fn lightning(t: f64) -> f32 {
    let slot = (t / 6.).floor();
    let pick = ((slot * 12.9898).sin() * 43758.5453).fract().abs();
    if pick < 0.45 {
        return 0.;
    }
    let at = (t - slot * 6.) - pick * 4.;
    if at < 0. {
        return 0.;
    }
    // A bright flash and a fainter second one just after.
    ((-at * 14.).exp() + 0.5 * (-(at - 0.18).abs() * 20.).exp()).min(1.) as f32
}

/// Hours since local midnight of the frame being drawn (fractional).
static DAY_HOUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Latitude of the weather location (f32 bits); 45°N until one is set.
static LATITUDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x4234_0000);

/// The weather location's latitude (south flips the seasons), if any.
pub fn set_latitude(latitude: Option<f64>) {
    let value = latitude.map_or(45., |l| l as f32);
    LATITUDE.store(value.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

fn latitude() -> f32 {
    f32::from_bits(LATITUDE.load(std::sync::atomic::Ordering::Relaxed))
}
/// Day of the year (1..=366) of the frame being drawn.
static DAY_OF_YEAR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(355);

/// The season at the frame's date, for Live weather. Values are smooth
/// through the year (no jump on the first of a month).
#[derive(Clone, Copy)]
struct Season {
    /// Snow lying on the ground and trees, 0..1.
    snow: f32,
    /// Spring and early summer meadow flowers, 0..1.
    flowers: f32,
    /// Autumn colours and fallen leaves, 0..1.
    autumn: f32,
    /// Local sunrise and sunset (hours, with summer time).
    sunrise: f32,
    sunset: f32,
}

impl Season {
    fn now(latitude: f32) -> Self {
        Self::on(
            DAY_OF_YEAR.load(std::sync::atomic::Ordering::Relaxed) as f32,
            latitude,
        )
    }

    /// The season on day `day` of the year.
    fn on(mut day: f32, latitude: f32) -> Self {
        if latitude < 0. {
            day = (day + 182.5) % 365.;
        }
        let turn = std::f32::consts::TAU / 365.;
        let summer = 0.5 + 0.5 * ((day - 172.) * turn).cos();
        // Distance in days around the year, for seasonal bumps.
        let near = |centre: f32, width: f32| {
            let d = (day - centre).rem_euclid(365.);
            let d = d.min(365. - d);
            (-(d / width).powi(2)).exp()
        };
        let ramp = |from: f32, to: f32| {
            let t = ((day - from) / (to - from)).clamp(0., 1.);
            t * t * (3. - 2. * t)
        };
        // Snow settles in the second and third week of December and melts
        // through March; autumn holds from early October until it comes.
        let snow = if day > 200. {
            ramp(340., 356.)
        } else {
            1. - ramp(60., 85.)
        };
        let autumn = ramp(250., 275.) * (1. - ramp(340., 356.));
        // Day length grows with latitude (about 8.7 to 15.5 hours at 45°).
        let swing = (latitude.abs() / 45. * 3.4).min(6.);
        let length = 12. + swing * (2. * summer - 1.);
        let summer_time = latitude >= 0. && (84..=300).contains(&(day as u32));
        let noon = 12.4 + if summer_time { 1. } else { 0. };
        Self {
            snow,
            flowers: near(145., 45.),
            autumn,
            sunrise: noon - length / 2.,
            sunset: noon + length / 2.,
        }
    }

    /// `hour` mapped onto the winter keyframes: sunrise to sunrise, sunset
    /// to sunset, day and night stretched linearly in between.
    fn winter_hour(&self, hour: f32) -> f32 {
        let (rise, set) = (self.sunrise, self.sunset);
        let h = hour.rem_euclid(24.);
        let mapped = if (rise..set).contains(&h) {
            SUNRISE + (h - rise) / (set - rise) * (SUNSET - SUNRISE)
        } else {
            let night = 24. - set + rise;
            let since = (h - set).rem_euclid(24.);
            SUNSET + since / night * (24. - SUNSET + SUNRISE)
        };
        mapped.rem_euclid(24.)
    }
}

/// The wall clock of the frame being built (the preview's `--time` too).
pub fn set_day(now: time::OffsetDateTime) {
    let hour =
        f32::from(now.hour()) + f32::from(now.minute()) / 60. + f32::from(now.second()) / 3600.;
    DAY_HOUR.store(hour.to_bits(), std::sync::atomic::Ordering::Relaxed);
    DAY_OF_YEAR.store(
        u32::from(now.ordinal()),
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// The hour Winter day shows. `SERA_DAY_CYCLE=<seconds>` runs a whole day
/// in that many seconds instead (to preview the transitions).
fn day_hour() -> f32 {
    static CYCLE: std::sync::OnceLock<Option<(f64, std::time::Instant)>> =
        std::sync::OnceLock::new();
    let cycle = CYCLE.get_or_init(|| {
        std::env::var("SERA_DAY_CYCLE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|s| *s > 0.)
            .map(|s| (s, std::time::Instant::now()))
    });
    match cycle {
        Some((seconds, start)) => ((start.elapsed().as_secs_f64() / seconds).fract() * 24.) as f32,
        None => f32::from_bits(DAY_HOUR.load(std::sync::atomic::Ordering::Relaxed)),
    }
}

/// One moment of a winter day: sky top and horizon, the light's colour,
/// how lit the scene is (0..1), how visible the stars are, and how much
/// it snows (1 = the full night snowfall).
struct Sky {
    hour: f32,
    top: u32,
    horizon: u32,
    glow: u32,
    light: f32,
    stars: f32,
    snow: f32,
}

/// Winter keyframes (short days: sunrise ~7:30, sunset ~16:45).
const DAY: [Sky; 10] = [
    Sky {
        hour: 0.,
        top: 0x050a1c,
        horizon: 0x16243f,
        glow: 0x9db8ff,
        light: 0.24,
        stars: 1.,
        snow: 1.0,
    },
    Sky {
        hour: 5.75,
        top: 0x070d24,
        horizon: 0x1c2a4c,
        glow: 0x9db8ff,
        light: 0.26,
        stars: 0.9,
        snow: 1.0,
    },
    Sky {
        hour: 7.0,
        top: 0x1c2858,
        horizon: 0x8a6a8e,
        glow: 0xffa890,
        light: 0.45,
        stars: 0.3,
        snow: 0.85,
    },
    Sky {
        hour: 7.75,
        top: 0x34509a,
        horizon: 0xf0a078,
        glow: 0xffc090,
        light: 0.7,
        stars: 0.,
        snow: 0.6,
    },
    Sky {
        hour: 9.5,
        top: 0x3462aa,
        horizon: 0x9cc0e4,
        glow: 0xfff2dc,
        light: 0.95,
        stars: 0.,
        snow: 0.35,
    },
    Sky {
        hour: 14.5,
        top: 0x2f5ea8,
        horizon: 0xa6c4e2,
        glow: 0xfff0d4,
        light: 1.,
        stars: 0.,
        snow: 0.35,
    },
    Sky {
        hour: 16.25,
        top: 0x2c4486,
        horizon: 0xf08a56,
        glow: 0xffa050,
        light: 0.72,
        stars: 0.,
        snow: 0.85,
    },
    Sky {
        hour: 17.0,
        top: 0x1a2054,
        horizon: 0x8a4a6a,
        glow: 0xe07a80,
        light: 0.45,
        stars: 0.3,
        snow: 1.0,
    },
    Sky {
        hour: 18.0,
        top: 0x0a1030,
        horizon: 0x24305a,
        glow: 0x9db8ff,
        light: 0.3,
        stars: 0.85,
        snow: 1.0,
    },
    Sky {
        hour: 24.,
        top: 0x050a1c,
        horizon: 0x16243f,
        glow: 0x9db8ff,
        light: 0.24,
        stars: 1.,
        snow: 1.0,
    },
];
const SUNRISE: f32 = 7.4;
const SUNSET: f32 = 16.7;

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Sky palette and shader tuning at `hour`: u_params is (light x, light y,
/// light level, stars). The sun crosses between sunrise and sunset, the
/// moon through the night; both set behind the mountains.
fn daylight(hour: f32) -> (Palette, [f32; 4], f32) {
    let hour = hour.rem_euclid(24.);
    let next = DAY
        .iter()
        .position(|k| k.hour > hour)
        .unwrap_or(DAY.len() - 1);
    let (a, b) = (&DAY[next.saturating_sub(1)], &DAY[next]);
    let span = (b.hour - a.hour).max(0.001);
    let t = ((hour - a.hour) / span).clamp(0., 1.);
    let t = t * t * (3. - 2. * t);
    let top = mix(rgb(a.top), rgb(b.top), t);
    let horizon = mix(rgb(a.horizon), rgb(b.horizon), t);
    let glow = mix(rgb(a.glow), rgb(b.glow), t);
    let light = a.light + (b.light - a.light) * t;
    let stars = a.stars + (b.stars - a.stars) * t;
    let snow = a.snow + (b.snow - a.snow) * t;
    // Arc progress of the sun by day, of the moon by night.
    let (start, length) = if (SUNRISE..SUNSET).contains(&hour) {
        (SUNRISE, SUNSET - SUNRISE)
    } else {
        (SUNSET, 24. - SUNSET + SUNRISE)
    };
    let arc = (hour - start).rem_euclid(24.) / length;
    let x = 0.12 + 1.54 * arc;
    let y = 0.78 - 0.6 * (arc * std::f32::consts::PI).sin();
    // Flakes catch the light: white by day, blue-grey at night, warm at dusk.
    let white = [1., 1., 1.];
    let catch = mix(mix(horizon, glow, 0.5), white, 0.35 + 0.5 * light);
    let flakes = [0.55, 0.68, 0.82, 0.95].map(|k| mix(top, catch, k));
    (
        Palette {
            base: top,
            accent: glow,
            colors: [flakes[0], flakes[1], flakes[2], flakes[3], horizon],
        },
        [x, y, light, stars],
        snow,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GLSL ES 1.00 (the TV) has no implicit int-to-float conversion,
    /// unlike desktop GL (the Mac preview): `cond ? 14.0 : 13` or
    /// `float x = 2;` fail to compile there and the wallpaper turns black.
    #[test]
    fn shaders_never_mix_int_literals_into_floats() {
        let is_int = |t: &str| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit());
        let is_float =
            |t: &str| t.contains('.') && t.trim_start_matches('-').parse::<f32>().is_ok();
        let sources = Style::ALL
            .map(|s| (s.file(), s.shader()))
            .into_iter()
            .chain([(
                "common.frag",
                include_str!("shaders/procedural/common.frag"),
            )]);
        for (file, source) in sources {
            for (number, line) in source.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                // `? a : b` with one float and one int literal.
                if let Some((_, rest)) = code.split_once('?')
                    && let Some((a, b)) = rest.split_once(':')
                {
                    let a = a.trim();
                    let b = b.trim().trim_end_matches(';').trim_end_matches(')').trim();
                    assert!(
                        !((is_int(a) && is_float(b)) || (is_float(a) && is_int(b))),
                        "{file}:{}: int literal in a float ternary: {line}",
                        number + 1
                    );
                }
                // `float x = 2;`
                if let Some(rest) = code.trim().strip_prefix("float ")
                    && let Some((_, value)) = rest.split_once('=')
                {
                    let value = value.trim().trim_end_matches(';').trim();
                    assert!(
                        !is_int(value.trim_start_matches('-')),
                        "{file}:{}: int literal assigned to a float: {line}",
                        number + 1
                    );
                }
            }
        }
    }

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
