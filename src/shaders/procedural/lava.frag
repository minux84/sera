// Lava lamp: six metaballs (points: x, y, r²) merging as they rise and
// sink, warm at the bottom, glowing into a dark backdrop.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    float field = 0.0;
    for (int i = 0; i < 6; i++) {
        vec2 d = p - u_points[i].xy;
        field += u_points[i].z / (dot(d, d) + 0.0008);
    }
    vec3 back = mix(u_base * 1.6, u_base, v_uv.y);
    // Light the lamp throws around the wax.
    vec3 col = back + u_accent * 0.16 * clamp(field * 0.5, 0.0, 1.0);
    float inside = smoothstep(0.96, 1.04, field);
    // Wax colour follows height; denser cores are brighter.
    vec3 wax = ramp(1.0 - v_uv.y) * (0.72 + 0.18 * clamp(field - 1.0, 0.0, 1.0));
    col = mix(col, wax, inside);
    gl_FragColor = finish(col);
}
