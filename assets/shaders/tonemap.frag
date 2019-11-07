#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 f_uv;

//layout(set = 0, binding = 0) uniform sampler tex_sampler;
//layout(set = 0, binding = 1) uniform texture2D hdr_tex;

layout(std140, set = 0, binding = 2) uniform Args {
    float exposure;
    vec3 clear_colour;
};

layout(location = 0) out vec4 color;

void main() {
    vec2 uv = f_uv;
    uv.y = 1.0 - uv.y;

    //color = vec4(clear_colour, 1.0);
    color = vec4(1.0,0.0,0.0, 1.0);
}
