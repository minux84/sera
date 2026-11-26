// Aurora: five soft colour fields (points: x, y, 1/r², strength) and a
// faint curtain in the accent colour.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    // Gentle domain warp: the fields read as fluid rather than circles.
    p.x += 0.07 * sin(p.y * 3.1 + u_phase.x);
    p.y += 0.05 * sin(p.x * 2.3 + u_phase.y);
    vec3 col = u_base;
    for (int i = 0; i < 5; i++) {
        vec2 d = p - u_points[i].xy;
        float w = u_points[i].w * exp(-dot(d, d) * u_points[i].z);
        col = mix(col, u_colors[i], w);
    }
    float line = 0.34 + 0.07 * sin(p.x * 2.1 + u_phase.z) + 0.035 * sin(p.x * 4.9 + u_phase.w);
    float dy = (p.y - line) * 7.0;
    float ribbon = exp(-dy * dy) * (0.55 + 0.45 * sin(p.x * 1.3 + u_phase.w));
    col += u_accent * ribbon * 0.22;
    gl_FragColor = finish(col);
}
