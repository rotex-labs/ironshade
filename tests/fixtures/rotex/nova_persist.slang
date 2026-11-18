// Trail fade. Blends black over the loaded accumulation target. Alpha is
// 1 - exp(-dt * rate), so the trail length does not depend on the frame rate.
struct FadePush {
    float alpha;
    float pad0;
    float pad1;
    float pad2;
};

[[vk::push_constant]]
ConstantBuffer<FadePush> pc;

struct VsOut {
    float4 position : SV_Position;
};

[shader("vertex")]
VsOut vertexMain([[vk::location(0)]] float2 uv : TEXCOORD0) {
    VsOut output;
    output.position = float4(uv * 2.0 - 1.0, 0.0, 1.0);
    return output;
}

[shader("fragment")]
float4 fragmentMain(VsOut input) : SV_Target {
    return float4(0.0, 0.0, 0.0, pc.alpha);
}
