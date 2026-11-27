// Snow: four parallax layers of flakes, far to near, under a soft glow.
// Layer i: u_points[i] = (fall fraction, whole cells fallen, cells per
// screen height, flake radius in cells); u_points[i + 4] = (sway phase,
// density, brightness, drift fraction); whole cells drifted in u_phase.
// One flake at most per cell, centred well inside it: no neighbour lookups.
// With TREES, two mountain ranges and a row of pines stand between the
// far and near layers.
float drift_cells(int i) {
    return i == 0 ? u_phase.x : i == 1 ? u_phase.y : i == 2 ? u_phase.z : u_phase.w;
}
#ifdef TREES
// Triangle wave: 0 in the valleys, 1 at the peaks (x = 0.5 + n).
float tri(float x) {
    return 1.0 - abs(fract(x) - 0.5) * 2.0;
}
// A mountain range: three octaves of triangle waves give jagged peaks.
// Peaks above `capline` (screen height) carry snow; the foot fades into
// mist. Faces left of each main peak are lit, the others in shadow.
vec3 range(vec2 p, vec3 col, float base, float amp, float seed, float capline, vec3 rock, vec3 cap) {
    float ridge = base - amp * (0.62 * tri(p.x * 0.9 + seed)
        + 0.28 * tri(p.x * 2.3 + seed * 1.7)
        + 0.10 * tri(p.x * 5.9 + seed * 2.9));
    float inside = smoothstep(ridge - 0.0015, ridge + 0.0015, p.y);
    float lit = step(fract(p.x * 0.9 + seed + p.y * 0.55), 0.5);
    vec3 m = rock * mix(0.74, 1.0, lit);
    float snowy = 1.0 - smoothstep(capline - 0.006, capline + 0.006,
        p.y + 0.035 * tri(p.x * 11.0 + seed) + 0.015 * tri(p.x * 27.0 + seed));
    m = mix(m, cap * mix(0.72, 1.0, lit), snowy);
    m = mix(m, col, smoothstep(ridge + 0.04, base + 0.1, p.y) * 0.6);
    return mix(col, m, inside);
}
// Far and near ranges, between the far snow layers.
vec3 mountains(vec2 p, vec3 col) {
    col = range(p, col, 0.70, 0.30, 0.37, 0.53,
        mix(u_colors[4], u_colors[0], 0.35) * 0.85, mix(u_colors[2], u_colors[4], 0.3));
    return range(p, col, 0.81, 0.22, 1.71, 0.68,
        mix(u_base, u_colors[4], 0.55) * 0.7, u_colors[2] * 0.78);
}
// Two rows of small pines along the bottom, back row lighter and smaller.
vec3 forest(vec2 p, vec3 col) {
    for (int row = 0; row < 2; row++) {
        float spacing = row == 0 ? 0.07 : 0.11;
        float tallest = row == 0 ? 0.15 : 0.23;
        vec3 shade = row == 0 ? mix(u_colors[4], u_base, 0.45) * 0.7 : u_base * 0.4;
        float slot = floor(p.x / spacing);
        // A pine reaches at most ~0.78 of a slot from its centre (jittered
        // by up to 0.18): only the neighbour on this pixel's side can
        // overlap it, so two lookups per row instead of three.
        float side = p.x / spacing - slot < 0.5 ? -1.0 : 0.0;
        for (int j = 0; j < 2; j++) {
            float k = slot + side + float(j);
            vec4 n = texture2D(u_noise, (mod(vec2(k, 40.0 + float(row)), 64.0) + 0.5) / 64.0);
            float cx = (k + 0.5 + 0.35 * (n.x - 0.5)) * spacing;
            float h = tallest * (0.55 + 0.45 * n.y);
            float y = p.y - (0.985 - h);
            // Tiers: the half-width grows down each tier, then steps in.
            float tier = fract(y / h * 4.0);
            float reach = y * 0.34 * (0.62 + 0.38 * tier);
            float inside = step(0.0, y) * (1.0 - smoothstep(reach - 0.002, reach, abs(p.x - cx)));
            // Snow resting on the outer tips of each tier.
            float rest = inside * smoothstep(0.55, 1.0, tier) * smoothstep(0.4, 1.0, abs(p.x - cx) / max(reach, 0.001));
            col = mix(col, shade, inside);
            col = mix(col, u_colors[1] * 0.8, rest * 0.5);
        }
    }
    // A snow bank covering the trunks.
    float bank = smoothstep(0.955, 0.963, p.y + 0.01 * sin(p.x * 3.7) + 0.006 * sin(p.x * 9.1));
    // Lit from above: brighter at its crest.
    vec3 snowbank = u_colors[2] * mix(0.72, 0.5, smoothstep(0.96, 1.0, p.y));
    return mix(col, snowbank, bank);
}
#endif
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    vec3 col = mix(u_base, u_colors[4], v_uv.y);
    vec2 g = (p - u_params.zw) * vec2(1.0, 1.4);
    col += u_accent * 0.32 * exp(-dot(g, g) * 5.0);
    for (int i = 0; i < 4; i++) {
#ifdef TREES
        // Mountains and trees only occupy the lower part: the sky above
        // skips them (spatially coherent, so whole GPU tiles take one path).
        if (i == 1 && p.y > 0.38) {
            col = mountains(p, col);
        }
        if (i == 2 && p.y > 0.75) {
            col = forest(p, col);
        }
#endif
        vec4 layer = u_points[i];
        vec4 extra = u_points[i + 4];
        vec2 q = p * layer.z - vec2(extra.w, layer.x);
        vec2 cell = floor(q);
        vec2 f = q - cell;
        vec2 id = cell - vec2(drift_cells(i), layer.y);
        vec4 n = texture2D(u_noise, (mod(id + vec2(float(i) * 13.0, float(i) * 29.0), 64.0) + 0.5) / 64.0);
        float present = step(n.w, extra.y);
        vec2 c = vec2(0.5 + 0.26 * sin(extra.x + n.z * 6.2831), 0.2 + 0.6 * n.y);
        float r = layer.w * (0.55 + 0.45 * n.x);
        float d = length(f - c);
        float flake = present * (1.0 - smoothstep(r * 0.3, r, d));
        col = mix(col, u_colors[i], flake * extra.z);
    }
    gl_FragColor = finish(col);
}
