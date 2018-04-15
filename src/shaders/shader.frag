#version 120
#extension GL_EXT_gpu_shader4: enable

uniform sampler1D palette;
uniform usampler2D sprite;

varying in vec2 v_uv;
varying out vec4 Target0;

void main() {
	Target0 = texture1D(palette, float(texture2D(sprite, v_uv).r) / 64.0);
	if(Target0.a > 0.5) {
	    Target0 = texture1D(palette, float(texture2D(sprite, v_uv).r % uint(64)) / 64.0);
	}
	//Target0 = vec4(1.0, 1.0, 1.0, 1.0);
}
