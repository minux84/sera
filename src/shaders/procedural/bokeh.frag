// Bokeh: eight out-of-focus lights (points: x, y, radius, brightness)
// drifting over a dark gradient; their colours walk the palette ramp.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    vec3 col = mix(u_base * 1.8, u_base, v_uv.y);
    for (int i = 0; i < 8; i++) {
        float d = length(p - u_points[i].xy);
        float r = u_points[i].z;
        // A soft disc with a slightly brighter rim, like a real lens.
        float disc = 1.0 - smoothstep(r * 0.82, r, d);
        float rim = smoothstep(r * 0.6, r * 0.95, d) * disc;
        float glow = exp(-d * d / (r * r) * 1.5) * 0.25;
        col += ramp(float(i) / 7.0) * u_points[i].w * (0.42 * disc + 0.22 * rim + glow);
    }
    gl_FragColor = finish(col);
}
