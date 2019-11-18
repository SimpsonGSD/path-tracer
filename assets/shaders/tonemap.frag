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
    tex_color *= clear_colour_and_exposure.a;
    tex_color = tex_color / (1 + tex_color); // reinhard tonemap
    float vignette = 1.0 - distance(uv, vec2(0.5, 0.5));
    tex_color *= vignette;
    color = vec4(tex_color, 1.0);
    // cheap dithering
    // color += sin(gl_FragCoord.x*114.0)*sin(gl_FragCoord.y*211.1)/512.0;
    //vec3 gamma = vec3(2.2, 2.2, 2.2);
    //color = vec4(pow(tex_color, gamma), 1.0);
}
