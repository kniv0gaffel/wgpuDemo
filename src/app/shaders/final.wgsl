struct VertexInput 
{
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct VertexOutput 
{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}



@vertex
fn vs_main( model: VertexInput ) -> VertexOutput 
{
    var out: VertexOutput;    
    out.uv = model.uv;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}



struct Color
{
    color: vec4<f32>
}

@group(0) @binding(0)
var<uniform> c_diffuse: Color;
@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0)@binding(2)
var s_diffuse: sampler;


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv );
    //return textureSample(t_diffuse, s_diffuse, in.uv ) + textureSample(t_diffuse1, s_diffuse1, in.uv );
    //return textureSample(t_diffuse1, s_diffuse1, in.uv );
    //return c_diffuse.color;
}