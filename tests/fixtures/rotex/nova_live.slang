// Additive copy of the live particle target onto the swapchain.
// The points themselves are drawn into a float target, where the blend is cheaper.
[[vk::binding(0, 0)]]
Texture2D<float4> liveTex;

[[vk::binding(1, 0)]]
SamplerState liveSamp;

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
    return liveTex.Sample(liveSamp, input.uv);
}
