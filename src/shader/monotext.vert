#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec4 color;
layout (location = 1) in vec2 pos;
layout (location = 2) in vec2 uv;

layout (location = 0) out vec4 o_color;
layout (location = 1) out vec2 o_uv;
void main() {
	o_uv = uv;
	o_color = color;
	gl_Position = vec4(pos, 0.0, 1.0);
}
