// Plasma: four interfering sine fields (one radial around points[0])
// mapped through the palette ramp.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    float v = sin(p.x * 2.6 + u_phase.x)
        + sin(p.y * 3.1 + u_phase.y)
        + sin((p.x + p.y) * 1.9 + u_phase.z)
        + sin(length(p - u_points[0].xy) * 4.2 - u_phase.w);
    float t = 0.5 + 0.5 * sin(v * 0.95);
    vec3 col = mix(u_base, ramp(t), 0.88);
    // A thin bright vein where the field crosses its midpoint.
    col += u_accent * 0.06 * exp(-pow(v * 2.2, 2.0));
    gl_FragColor = finish(col);
}
