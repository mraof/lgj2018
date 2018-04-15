#version 130

uniform int x;
uniform int y;
uniform float width;
uniform float height;
uniform float flip;

in vec2 pos;
in vec2 uv;
varying out vec2 v_uv;

void main() {
    v_uv = uv * vec2(flip, 1.0);
	gl_Position = vec4(float(pos.x + x) / width * 2.0 - 1.0, float(pos.y + y) / height * -2.0 + 1.0, 0.0, 1.0);
}
