attribute vec2 a_position;
varying PAINT_PRECISION vec2 v_uv;
void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    // Texture row 0 is drawn at the top of a quad, so uv.y = 0 is "top".
    v_uv = a_position * 0.5 + 0.5;
}
