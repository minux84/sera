// Winter day: snowy mountains, pines and falling snow under a sky that
// follows the local time of day (keyframes in procedural.rs `daylight`).
// u_base: sky top; u_colors[4]: horizon; u_accent: colour of the light;
// u_params: (sun or moon x, y, light level 0..1, star visibility 0..1);
// u_colors[0..3]: flake colours far to near, already lit for the hour.
// Snow layers use u_points / u_phase exactly as in snow.frag.
#ifdef WEATHER
// Live weather: (cloud cover 0..1, rain 0/1, fog 0..1, lightning flash).
uniform vec4 u_weather;
// Live weather season: (snow lying 0..1, greenery, flowers, autumn).
uniform vec4 u_season;
#endif
// How much snow lies on the ground, trees and peaks (always full in
// Winter day; the season and real snowfall decide it in Live weather).
float snow_cover() {
#ifdef WEATHER
    return u_season.x;
#else
    return 1.0;
#endif
}
#ifdef WEATHER
// Meadow grass, drying to gold-brown in autumn; dark at night.
vec3 grass_tone() {
    return mix(vec3(0.20, 0.42, 0.16), vec3(0.42, 0.34, 0.13), u_season.w * 0.8) * (0.3 + 0.7 * u_params.z);
}
#endif
float drift_cells(int i) {
    return i == 0 ? u_phase.x : i == 1 ? u_phase.y : i == 2 ? u_phase.z : u_phase.w;
}
// Triangle wave: 0 in the valleys, 1 at the peaks (x = 0.5 + n).
float tri(float x) {
    return 1.0 - abs(fract(x) - 0.5) * 2.0;
}
// Snow on the ground and peaks: white by day, sky-tinted and dim at night.
vec3 snow_tone() {
    float light = u_params.z;
    vec3 shadowed = mix(u_colors[4], u_accent, 0.25);
    return mix(shadowed, vec3(0.94, 0.96, 1.0), light) * (0.38 + 0.62 * light);
}
vec3 range(vec2 p, vec3 col, float base, float amp, float seed, float capline, vec3 rock) {
#ifdef WEATHER
    // Out of winter only the highest peaks keep their snow.
    capline -= (1.0 - u_season.x) * 0.075;
#endif
    vec3 phase = vec3(p.x * 0.9 + seed, p.x * 2.3 + seed * 1.7, p.x * 5.9 + seed * 2.9);
    float ridge = base - amp * (0.62 * tri(phase.x) + 0.28 * tri(phase.y) + 0.10 * tri(phase.z));
    float inside = smoothstep(ridge - 0.0015, ridge + 0.0015, p.y);
    // No faceted shading (it reads as stripes): an even body, lighter at
    // the top, with a soft rim of light along the ridge that is strongest
    // on the side towards the sun or moon and shifts with it gradually.
    float depth = max(p.y - ridge, 0.0);
    float towards = 0.5 + 0.5 * clamp((u_params.x - p.x) * 0.8, -1.0, 1.0);
    float lit = 0.55 + 0.25 * exp(-depth * 6.0) + 0.2 * exp(-depth * 40.0) * towards;
    float light = u_params.z;
    vec3 m = rock * mix(0.74, 1.0 + 0.1 * light, lit) + u_accent * 0.06 * lit * light;
    float snowy = 1.0 - smoothstep(capline - 0.006, capline + 0.006,
        p.y + 0.035 * tri(p.x * 11.0 + seed) + 0.015 * tri(p.x * 27.0 + seed));
    // Sunlit snow warms with the light's colour (pink at dawn, gold at dusk).
    vec3 cap = snow_tone() * mix(0.72, 1.0, lit) + u_accent * 0.12 * lit * light;
#ifdef WEATHER
    // Green lower slopes once the snow has gone.
    m = mix(m, mix(grass_tone(), m, 0.45), u_season.y * 0.5 * smoothstep(ridge + 0.06, base + 0.02, p.y));
#endif
    m = mix(m, cap, snowy);
    m = mix(m, col, smoothstep(ridge + 0.04, base + 0.1, p.y) * 0.6);
    return mix(col, m, inside);
}
vec3 mountains(vec2 p, vec3 col) {
    float light = u_params.z;
    vec3 far = mix(u_colors[4], u_base, 0.45) * (0.62 + 0.3 * light);
    vec3 near = mix(u_base, u_colors[4], 0.25) * (0.36 + 0.34 * light);
    col = range(p, col, 0.70, 0.30, 0.37, 0.53, far);
    return range(p, col, 0.81, 0.22, 1.71, 0.68, near);
}
vec3 forest(vec2 p, vec3 col) {
    float light = u_params.z;
    vec3 snow = snow_tone();
    for (int row = 0; row < 2; row++) {
        float spacing = row == 0 ? 0.07 : 0.11;
        float tallest = row == 0 ? 0.15 : 0.23;
        // Silhouettes at night, dark green needles by day.
        vec3 needles = mix(u_base * 0.35, vec3(0.06, 0.15, 0.11), light);
#ifdef WEATHER
        // Fresher green in the warm months.
        needles = mix(needles, vec3(0.07, 0.2, 0.1) * (0.35 + 0.65 * light), u_season.y * 0.6);
#endif
        vec3 shade = row == 0 ? mix(needles, u_colors[4], 0.25) : needles * 0.8;
        float slot = floor(p.x / spacing);
        float side = p.x / spacing - slot < 0.5 ? -1.0 : 0.0;
        for (int j = 0; j < 2; j++) {
            float k = slot + side + float(j);
            vec4 n = texture2D(u_noise, (mod(vec2(k, 40.0 + float(row)), 64.0) + 0.5) / 64.0);
            float cx = (k + 0.5 + 0.35 * (n.x - 0.5)) * spacing;
            float h = tallest * (0.55 + 0.45 * n.y);
            float y = p.y - (0.985 - h);
            float tier = fract(y / h * 4.0);
            float reach = y * 0.34 * (0.62 + 0.38 * tier);
            float inside = step(0.0, y) * (1.0 - smoothstep(reach - 0.002, reach, abs(p.x - cx)));
            float rest = inside * smoothstep(0.55, 1.0, tier) * smoothstep(0.4, 1.0, abs(p.x - cx) / max(reach, 0.001));
            col = mix(col, shade, inside);
            col = mix(col, snow * 0.9, rest * 0.55 * snow_cover());
        }
    }
    float bank = smoothstep(0.955, 0.963, p.y + 0.01 * sin(p.x * 3.7) + 0.006 * sin(p.x * 9.1));
    vec3 ground = snow * mix(0.9, 0.7, smoothstep(0.96, 1.0, p.y));
#ifdef WEATHER
    // Meadow out of winter, with flowers in spring and summer and fallen
    // leaves in autumn: one dot at most per small cell of the ground.
    ground = mix(grass_tone() * mix(1.0, 0.8, smoothstep(0.96, 1.0, p.y)), ground, u_season.x);
    float petals = max(u_season.z, u_season.w * 0.8);
    if (petals > 0.01 && bank > 0.0) {
        vec2 q = p * 90.0;
        vec2 cell = floor(q);
        vec4 n = texture2D(u_noise, (mod(cell + vec2(11.0, 17.0), 64.0) + 0.5) / 64.0);
        float dot_ = step(n.w + 0.002, 0.45 * petals)
            * (1.0 - smoothstep(0.12, 0.2, length(q - cell - (0.3 + 0.4 * n.yz))));
        // White, yellow, pink and violet flowers; orange, red, brown leaves.
        vec3 flower = n.x < 0.25 ? vec3(0.95) : n.x < 0.5 ? vec3(1.0, 0.85, 0.25)
            : n.x < 0.75 ? vec3(1.0, 0.55, 0.7) : vec3(0.7, 0.55, 1.0);
        vec3 leaf = n.x < 0.4 ? vec3(0.9, 0.45, 0.12) : n.x < 0.75 ? vec3(0.75, 0.2, 0.1) : vec3(0.5, 0.32, 0.12);
        vec3 tint = mix(flower, leaf, step(u_season.z, u_season.w));
        ground = mix(ground, tint * (0.35 + 0.65 * u_params.z), dot_);
    }
#endif
    return mix(col, ground, bank);
}
// u_drift.x: cloud drift in [0, 1); cloud i moves k_i times as fast.
// u_drift.y: overcast drift in screen heights, wrapped at 8 (seamless).
uniform vec4 u_drift;
// Cloud colours under the hour's light: sunlit tops and shaded undersides.
vec3 cloud_top() {
    float light = u_params.z;
    return mix(u_colors[4], vec3(1.0), 0.35 + 0.6 * light) * (0.45 + 0.55 * light) + u_accent * 0.12;
}
vec3 cloud_under() {
    return mix(cloud_top(), u_base, 0.35);
}
// The winter overcast: two layers of cloud cover the upper sky, their
// lower edges a row of rounded puffs, the nearer layer lower and darker.
// Returns the colour and, in .w, how much of the sky it covers.
vec4 overcast(vec2 p, vec3 col) {
    // Winter overcast is grey, never white: the clock and menus sit on it.
    float light = u_params.z;
    vec3 grey = mix(u_colors[4], vec3(0.80, 0.83, 0.88), 0.25 + 0.55 * light) * (0.42 + 0.4 * light)
        + u_accent * 0.08;
    float covered = 0.0;
    for (int l = 0; l < 2; l++) {
        float fl = float(l);
        // A row of touching half-circles hanging from a straight band, their
        // sizes swelling and shrinking in a rhythm (big, smaller, smallest,
        // bigger...). The row is u(x): every whole u is one half-circle, and
        // where u runs faster the half-circles are narrower. Everything
        // repeats every 4 screen heights, which the drift wraps at.
        float max_size = 0.10;
        float min_size = 0.1;
        float size = l == 0 ? 14.0 : 13.0;  // half-circles per screen height
        float base = l == 0 ? 0.015 : 0.06;
#ifdef WEATHER
        // A clear sky pulls the overcast off the top of the screen; a full
        // cover lowers it a little.
        base += (u_weather.x - 1.0) * 0.16 + max(u_weather.x - 0.9, 0.0) * 0.5;
#endif
        float x = p.x + u_drift.y * (l == 0 ? 1.0 : 1.5) + fl * 0.61;
        float f1 = 9.424778;                // 6 rhythm cycles per 4 heights
        float f2 = 3.141593;                // 2 slower swells per 4 heights
        float u = size * x + size * (max_size / f1 * sin(f1 * x) + min_size / f2 * sin(f2 * x + 1.3));
        float rate = size * (1.0 + max_size * cos(f1 * x) + min_size * cos(f2 * x + 1.3));
        float r = 0.5 / rate * 1.04;        // just touching its neighbours
        float dx = (fract(u) - 0.5) / rate;
        float edge = base + 0.6 * sqrt(max(0.0, r * r - dx * dx));
        float cover = 1.0 - smoothstep(edge - 0.003, edge + 0.003, p.y);
        // Darkest overhead, lighter towards the deck's lower edge, with a
        // faint shade under each puff.
        vec3 tone = grey * mix(0.72, 1.0, smoothstep(0.0, edge, p.y))
            * (1.0 - 0.08 * smoothstep(edge - 0.04, edge, p.y)) * (l == 0 ? 1.0 : 0.92);
        col = mix(col, tone, cover);
        covered = max(covered, cover);
    }
    return vec4(col, covered);
}
// One stylised cloud: three puffs over a flat base, centred at c, size s.
// v (0..1 each) varies the shape: puff sizes, the crown's height and how
// stretched the whole cloud is, so no two clouds look alike.
float cloud(vec2 p, vec2 c, float s, vec3 v) {
    vec2 q = (p - c) / s;
    q.x /= 0.8 + 0.6 * v.z;
    q.y /= 0.75 + 0.5 * v.y;
    float d = min(
        min(length(q - vec2(-0.55, 0.05)) - (0.26 + 0.2 * v.x),
            length(q - vec2(0.05 - 0.2 * v.z, -0.1 - 0.12 * v.y)) - (0.4 + 0.18 * v.y)),
        min(length(q - vec2(0.55, 0.02)) - (0.24 + 0.22 * fract(v.x + v.z)),
            length(q - vec2(0.95, 0.12)) - (0.12 + 0.16 * v.z)));
    float soft = 0.03 / s * 0.06 + 0.02;
    float body = 1.0 - smoothstep(-soft, soft, d);
    // Flat underside.
    return body * (1.0 - smoothstep(0.22, 0.3, q.y));
}
// Nine clouds drifting across the upper sky, lit by the hour's light:
// white by day, pink or gold at dawn and dusk, dim and thin at night.
vec3 clouds(vec2 p, vec3 col) {
    float light = u_params.z;
    vec3 top = cloud_top();
    vec3 under = cloud_under();
    for (int i = 0; i < 9; i++) {
        float k = float(i);
        float speed = i == 2 || i == 5 || i == 7 ? 2.0 : 1.0;
        float x = fract(k * 0.382 + u_drift.x * speed) * 2.6 - 0.45;
        // Loose clouds hang just under the overcast.
        vec2 c = vec2(x, 0.12 + 0.1 * fract(k * 0.618 + 0.1));
        float s = 0.042 + 0.018 * fract(k * 0.37 + 0.2);
        vec3 v = fract(vec3(0.37, 0.71, 0.53) * (k + 1.0) + vec3(0.13, 0.29, 0.61));
        float a = cloud(p, c, s, v);
        float shade = smoothstep(c.y - s * 0.4, c.y + s * 0.3, p.y);
        col = mix(col, mix(top, under, shade), a * (0.5 + 0.4 * light));
    }
    return col;
}
// Stars on a 70-per-height grid, fading towards the horizon.
vec3 stars(vec2 p, vec3 col) {
    vec2 sq = p * 70.0;
    vec2 cell = floor(sq);
    // Shift each 64-cell block so the noise tile never visibly repeats.
    vec2 id = vec2(cell.x, cell.y + floor(cell.x / 64.0) * 23.0);
    vec4 n = texture2D(u_noise, (mod(id + vec2(5.0, 33.0), 64.0) + 0.5) / 64.0);
    float d = length(sq - cell - (0.25 + 0.5 * n.yz));
    float star = step(n.w, 0.07) * (1.0 - smoothstep(0.04, 0.1 * (0.5 + n.x), d));
    return col + vec3(0.9, 0.93, 1.0) * star * u_params.w * (0.4 + 0.6 * n.x) * (1.0 - p.y / 0.75);
}
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    float night = step(0.5, u_params.w);
    vec3 col = mix(u_base, u_colors[4], smoothstep(0.0, 0.8, v_uv.y));
    if (u_params.w > 0.01 && p.y < 0.75) {
        col = stars(p, col);
    }
    // Light low on the horizon under the sun or moon (dawn, dusk).
    vec2 h = (p - vec2(u_params.x, 0.78)) * vec2(1.1, 3.2);
    col += u_accent * 0.22 * exp(-dot(h, h)) * (1.0 - 0.6 * u_params.z);
    // The sun or moon itself, with its halo.
    float dist = length(p - u_params.xy);
    float r = night > 0.5 ? 0.028 : 0.036;
    // Under a closed sky the sun or moon fades to a glow (Live weather).
    float veil = 0.0;
#ifdef WEATHER
    veil = smoothstep(0.6, 1.0, u_weather.x);
#endif
    vec3 halo = night > 0.5 ? u_accent : mix(u_accent, vec3(1.0, 0.82, 0.4), 0.45);
    col += halo * exp(-dist * dist * (night > 0.5 ? 70.0 : 22.0)) * (night > 0.5 ? 0.22 : 0.45)
        * (1.0 - 0.85 * veil);
    // A warm yellow sun (deeper orange as its light reddens at dawn and
    // dusk); a pale moon at night.
    vec3 sun = mix(vec3(1, 0.84, 0.72), u_accent, 0.3);
    vec3 disc = night > 0.5 ? mix(u_accent, vec3(1.0), 0.65) : sun;
    col = mix(col, disc, (1.0 - smoothstep(r * 0.85, r, dist)) * (1.0 - 0.9 * veil));
    // In front of the sun and moon, only in the upper sky. Behind the
    // overcast the sun or moon is only a diffuse brightening.
    if (p.y < 0.42) {
        vec4 deck = overcast(p, col);
        float glow = exp(-dist * dist * 9.0) * deck.w * (night > 0.5 ? 0.12 : 0.28);
        col = deck.rgb + u_accent * glow;
        // Loose clouds under the overcast are hidden for now (clouds()).
#ifdef WEATHER
        // Partly cloudy: loose clouds, fading as the sky clears or closes.
        float loose = clamp(u_weather.x * (1.0 - u_weather.x) * 4.0, 0.0, 1.0);
        if (loose > 0.01) {
            col = mix(col, clouds(p, col), loose);
        }
#endif
    }
#ifdef WEATHER
    // Lightning lights the sky from above.
    col += vec3(0.8, 0.84, 1.0) * u_weather.w * 0.55 * (1.0 - smoothstep(0.2, 0.85, p.y));
    vec3 fog = mix(u_colors[4], vec3(0.74, 0.77, 0.82), 0.5) * (0.45 + 0.55 * u_params.z);
#endif
    for (int i = 0; i < 4; i++) {
        // Mountains and trees only occupy the lower part: the sky above
        // skips them (spatially coherent, so whole GPU tiles take one path).
        if (i == 1 && p.y > 0.38) {
            col = mountains(p, col);
        }
#ifdef WEATHER
        // Fog thickens over the mountains, before the nearer trees.
        if (i == 2) {
            col = mix(col, fog, u_weather.z * (0.25 + 0.45 * smoothstep(0.2, 0.9, p.y)));
        }
#endif
        if (i == 2 && p.y > 0.75) {
            col = forest(p, col);
        }
        vec4 layer = u_points[i];
        vec4 extra = u_points[i + 4];
        vec2 q = p * layer.z - vec2(extra.w, layer.x);
        vec2 cell = floor(q);
        vec2 f = q - cell;
        vec2 id = cell - vec2(drift_cells(i), layer.y);
        vec4 n = texture2D(u_noise, (mod(id + vec2(float(i) * 13.0, float(i) * 29.0), 64.0) + 0.5) / 64.0);
        // The offset keeps a zero density at zero flakes: step(0.0, 0.0) is
        // 1, and some noise texels are exactly 0 (Live weather, no snow).
        float present = step(n.w + 0.002, extra.y);
        vec2 c = vec2(0.5 + 0.26 * sin(extra.x + n.z * 6.2831), 0.2 + 0.6 * n.y);
        float rad = layer.w * (0.55 + 0.45 * n.x);
        float flake = present * (1.0 - smoothstep(rad * 0.3, rad, length(f - c)));
        float strength = extra.z;
#ifdef WEATHER
        if (u_weather.y > 0.5) {
            // Rain: a thin slanted streak instead of a round flake.
            vec2 g = f - c;
            g.x += g.y * 0.22;
            flake = present * smoothstep(0.42, 0.25, abs(g.y))
                * (1.0 - smoothstep(rad * 0.1, rad * 0.28, abs(g.x)));
            strength *= 0.55;
        }
#endif
        col = mix(col, u_colors[i], flake * strength);
    }
#ifdef WEATHER
    col = mix(col, fog, u_weather.z * 0.15);
    col += vec3(0.8, 0.84, 1.0) * u_weather.w * 0.12;
#endif
    gl_FragColor = finish(col);
}
