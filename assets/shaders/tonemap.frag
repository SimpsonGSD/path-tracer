#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 f_uv;

layout(set = 0, binding = 0) uniform texture2D colormap; 
layout(set = 0, binding = 1) uniform sampler colorsampler;

layout(std140, set = 0, binding = 2) uniform Args {
    vec4 exposure_numframes_xx;
};

layout(location = 0) out vec4 color;

// sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
const mat3x3 ACESInputMat_ColumnMajor = mat3(

    0.59719, 0.35458, 0.04823,
    0.07600, 0.90834, 0.01566,
    0.02840, 0.13383, 0.83777
);

const mat3x3 ACESInputMat = mat3(

    0.59719, 0.35458, 0.04823,
    0.07600, 0.90834, 0.01566,
    0.02840, 0.13383, 0.83777
);

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3x3 ACESOutputMat = mat3(
     1.60475, -0.53108, -0.07367,
    -0.10208,  1.10813, -0.00605,
    -0.00327, -0.07276,  1.07602
);

vec3 RRTAndODTFit(vec3 v)
{
    vec3 a = v * (v + 0.0245786f) - 0.000090537f;
    vec3 b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

vec3 ACESFitted(vec3 color)
{
    //color = ACESInputMat * color;
    color = color * ACESInputMat;

    // Apply RRT and ODT
    color = RRTAndODTFit(color);

   // color = ACESOutputMat * color;
    color = color * ACESOutputMat;

    // Clamp to [0, 1]
    color = clamp(color, 0, 1);

    return color;
}

void main() {
    vec2 uv = f_uv;
    uv.y = 1.0 - uv.y;
    vec3 tex_color = texture(sampler2D(colormap, colorsampler), uv).rgb;// / exposure_numframes_xx.g;
    tex_color *= exposure_numframes_xx.r;
    //tex_color = tex_color / (1 + tex_color); // reinhard tonemap
    tex_color = ACESFitted(tex_color);
    float vignette = 1.0 - distance(uv, vec2(0.5, 0.5));
    tex_color *= vignette;
    color = vec4(tex_color, 1.0);
    // cheap dithering
    // color += sin(gl_FragCoord.x*114.0)*sin(gl_FragCoord.y*211.1)/512.0;
    //vec3 gamma = vec3(2.2, 2.2, 2.2);
    //color = vec4(pow(tex_color, gamma), 1.0);
}
