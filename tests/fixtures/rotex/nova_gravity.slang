// Nova gravity. One thread per particle, local size 256. Reads one device buffer
// and writes the other. Audio bands stay in the block so the layout matches Nova's
// 64-byte SimParams; this example leaves them at zero.
struct Particle {
    float2 pos;
    float2 vel;
};

struct Well {
    float2 pos;
    float strength;
    float radius;
};

struct SimPush {
    float dt;
    float gravityStrength;
    float damping;
    uint particleCount;
    uint wellCount;
    float bass;
    float mid;
    float treble;
    float boidSeparation;
    float boidAlignment;
    float boidCohesion;
    float boidRadius;
    float maxSpeed;
    float time;
    float gridDim;
    float boxHalf;
};

[[vk::push_constant]]
ConstantBuffer<SimPush> pc;

[[vk::binding(0, 0)]]
StructuredBuffer<Particle> inp;

[[vk::binding(1, 0)]]
RWStructuredBuffer<Particle> outp;

[[vk::binding(2, 0)]]
StructuredBuffer<Well> wells;

float hash(uint x) {
    x = (x ^ 61) ^ (x >> 16);
    x *= 9;
    x = x ^ (x >> 4);
    x *= 0x27d4eb2d;
    x = x ^ (x >> 15);
    return float(x) / 4294967295.0;
}

[shader("compute")]
[numthreads(256, 1, 1)]
void computeMain(uint3 dispatch_id : SV_DispatchThreadID) {
    uint i = dispatch_id.x;
    if (i >= pc.particleCount) {
        return;
    }

    float2 p = inp[i].pos;
    float2 v = inp[i].vel;
    float wellBoost = 1.0 + pc.bass * 1.2;
    float swirl = 0.15 + pc.mid * 0.1;
    float2 acc = float2(0.0, 0.0);

    for (uint w = 0; w < pc.wellCount; w++) {
        float2 d = wells[w].pos - p;
        float r2 = dot(d, d) + wells[w].radius;
        float invr = rsqrt(r2);
        float f = wells[w].strength * pc.gravityStrength * wellBoost * invr * invr;
        float2 dir = d * invr;
        acc += dir * f;
        acc += float2(-dir.y, dir.x) * f * swirl;
    }

    if (pc.treble > 0.001) {
        float a = hash(i) * 6.2831853;
        acc += float2(cos(a), sin(a)) * pc.treble * 0.08;
    }

    acc += -p * 0.01;
    v += acc * pc.dt;
    v *= pc.damping;

    float speed = length(v);
    if (speed > pc.maxSpeed) {
        v *= pc.maxSpeed / speed;
    }
    p += v * pc.dt;

    float h = pc.boxHalf;
    if (abs(p.x) > h) {
        v.x -= (p.x - sign(p.x) * h) * 4.0 * pc.dt;
    }
    if (abs(p.y) > h) {
        v.y -= (p.y - sign(p.y) * h) * 4.0 * pc.dt;
    }

    outp[i].pos = p;
    outp[i].vel = v;
}
