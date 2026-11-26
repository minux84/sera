// Waves: five layered hills of colour rolling past each other, back to
// front (points[i].xy: the two phases of layer i), under a glowing sky.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    // Sky: accent glow low on the horizon fading into the base.
    float glow = exp(-pow((p.y - 0.42) * 3.2, 2.0)) * exp(-pow((p.x - 1.15) * 1.1, 2.0));
    vec3 col = mix(u_base, u_accent, 0.35 * glow);
    for (int i = 0; i < 5; i++) {
        float k = float(i);
        float crest = 0.36 + 0.13 * k
            + 0.065 * sin(p.x * (1.3 + 0.35 * k) + u_points[i].x)
            + 0.03 * sin(p.x * (3.1 + 0.6 * k) + u_points[i].y);
        float below = p.y - crest;
        float cover = smoothstep(-0.0025, 0.0025, below);
        // Each hill is lit at its crest and darkens into its own body.
        // (max: exp of a large positive value overflows mediump, and inf * 0 is NaN.)
        vec3 hill = u_colors[i] * (1.0 + 0.35 * exp(-max(below, 0.0) * 26.0) - 0.25 * clamp(below * 3.0, 0.0, 1.0));
        // Soft shadow the hill casts on what lies behind it.
        col *= 1.0 - 0.22 * smoothstep(0.06, 0.0, -below) * (1.0 - cover);
        col = mix(col, hill, cover);
    }
    gl_FragColor = finish(col);
}
