// Reinhard tonemap of the HDR accumulation target onto the swapchain.
[[vk::binding(0, 0)]]
Texture2D<float4> accumTex;

[[vk::binding(1, 0)]]
SamplerState accumSamp;

struct VsOut {
    float4 position : SV_Position;
    [[vk::location(0)]] float2 uv : TEXCOORD0;
};

[shader("vertex")]
VsOut vertexMain([[vk::location(0)]] float2 uv : TEXCOORD0) {
    VsOut output;
    output.uv = uv;
    output.position = float4(uv * 2.0 - 1.0, 0.0, 1.0);
    return output;
}

[shader("fragment")]
float4 fragmentMain(VsOut input) : SV_Target {
    float3 c = accumTex.Sample(accumSamp, input.uv).rgb;
    c = max(c - 0.03, float3(0.0, 0.0, 0.0));
    c *= 1.3;
    c = c / (1.0 + c);
    float d = length(input.uv - 0.5);
    c *= 1.0 - 0.4 * smoothstep(0.45, 0.9, d);
    c = pow(c, float3(1.0 / 2.2, 1.0 / 2.2, 1.0 / 2.2));
    return float4(c, 1.0);
}
