// One vertex is one particle. The buffer is vec2 position plus vec2 velocity,
// 16 bytes, vertex rate. The point size comes from the push constant.
struct PointPush {
    float aspect;
    float pointSize;
    float brightness;
    float pad;
};

[[vk::push_constant]]
ConstantBuffer<PointPush> pc;

struct VsOut {
    float4 position : SV_Position;
    // WGSL has no point-size builtin, and DXIL shader model 6 has no
    // SV_PointSize. Those targets draw a 1-pixel point. SPIR-V keeps the sprite.
#if !defined(ROTEX_WGSL) && !defined(ROTEX_DXIL)
    float pointSize : SV_PointSize;
#endif
    // Vertex inputs already occupy TEXCOORD0/1. A second TEXCOORD0 here is a
    // different float width, and D3D12 rejects the pixel-shader link.
    [[vk::location(0)]] float4 color : TEXCOORD2;
};

[shader("vertex")]
VsOut vertexMain(
    [[vk::location(0)]] float2 pos : TEXCOORD0,
    [[vk::location(1)]] float2 vel : TEXCOORD1
) {
    VsOut output;
    float2 p = pos;
    p.x /= pc.aspect;
    output.position = float4(p, 0.0, 1.0);
#if !defined(ROTEX_WGSL) && !defined(ROTEX_DXIL)
    output.pointSize = pc.pointSize;
#endif

    float t = clamp(length(vel) * 1.6, 0.0, 1.0);
    float3 cold = float3(0.05, 0.18, 0.85);
    float3 midc = float3(0.15, 0.75, 1.00);
    float3 hot = float3(1.00, 0.82, 0.55);
    float3 c = t < 0.5 ? lerp(cold, midc, t * 2.0) : lerp(midc, hot, t * 2.0 - 1.0);
    float h = frac(sin(dot(pos, float2(12.9898, 78.233))) * 43758.5453);
    c *= 0.8 + 0.4 * h;
    output.color = float4(c * pc.brightness, 1.0);
    return output;
}

[shader("fragment")]
#if defined(ROTEX_WGSL) || defined(ROTEX_DXIL)
// The pixel shader must declare SV_Position so TEXCOORD lands on the same
// hardware register the vertex shader writes. D3D12 rejects the link otherwise.
float4 fragmentMain(VsOut input) : SV_Target {
    return input.color;
}
#else
float4 fragmentMain(
    [[vk::location(0)]] float4 color : TEXCOORD2,
    float2 coord : SV_PointCoord
) : SV_Target {
    float2 d = coord - float2(0.5, 0.5);
    float r2 = dot(d, d);
    if (r2 > 0.25) {
        discard;
    }
    return float4(color.rgb * exp(-r2 * 12.0), 1.0);
}
#endif
