// Shared header of every animated wallpaper. Positions and phases arrive
// from the CPU already wrapped, so mediump is enough: no clock grows here.
// p: x in [0, 16/9], y in [0, 1], origin top-left.
uniform vec4 u_points[8];
uniform vec3 u_colors[5];
uniform vec3 u_base;
uniform vec3 u_accent;
uniform vec4 u_phase;
// Style tuning (per wallpaper) and a 64x64 random texture.
uniform vec4 u_params;
uniform sampler2D u_noise;
varying PAINT_PRECISION vec2 v_uv;
// The palette as a five-stop ramp, t in [0, 1].
vec3 ramp(float t) {
    t = clamp(t, 0.0, 1.0) * 4.0;
    vec3 c = mix(u_colors[0], u_colors[1], clamp(t, 0.0, 1.0));
    c = mix(c, u_colors[2], clamp(t - 1.0, 0.0, 1.0));
    c = mix(c, u_colors[3], clamp(t - 2.0, 0.0, 1.0));
    return mix(c, u_colors[4], clamp(t - 3.0, 0.0, 1.0));
}
// Darker edges keep the UI legible over the brightest areas.
vec4 finish(vec3 col) {
    vec2 e = v_uv - 0.5;
    return vec4(col * (1.0 - 0.55 * dot(e, e)), 1.0);
}
