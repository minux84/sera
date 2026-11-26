// Silk: a warped fabric of broad folds, with a sheen in the accent
// colour running along the ridges.
void main() {
    vec2 p = vec2(v_uv.x * 1.7777778, v_uv.y);
    vec2 q = p;
    q.y += 0.13 * sin(q.x * 2.1 + u_phase.x) + 0.05 * sin(q.x * 4.3 - u_phase.y);
    q.x += 0.09 * sin(q.y * 2.8 + u_phase.z);
    float fold = 0.5 + 0.5 * sin(q.y * 7.5 + q.x * 1.6 + 1.6 * sin(q.x * 1.6 + u_phase.w));
    float ridge = 0.5 + 0.5 * sin(q.y * 15.0 + 2.2 * sin(q.x * 1.9 + u_phase.x));
    float r4 = ridge * ridge;
    float sheen = r4 * r4 * fold;
    // Squared fold: deep valleys between narrower lit folds.
    vec3 col = mix(u_base, ramp(fold * fold), 0.85);
    col += u_accent * 0.16 * sheen;
    gl_FragColor = finish(col);
}
