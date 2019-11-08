#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 f_uv;

layout(set = 0, binding = 0) uniform texture2D colormap; 
layout(set = 0, binding = 1) uniform sampler colorsampler;

layout(std140, set = 0, binding = 2) uniform Args {
    vec4 clear_colour_and_exposure;
};

layout(location = 0) out vec4 color;

void main() {
    vec2 uv = f_uv;
    uv.y = 1.0 - uv.y;
    vec3 tex_color = texture(sampler2D(colormap, colorsampler), uv).rgb;
    color = vec4(tex_color, 1.0);
    //color = vec4(1.0,0.0,0.0, 1.0);
}
