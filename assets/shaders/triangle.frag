#version 450

#extension GL_EXT_nonuniform_qualifier : require

#define PI 3.1415926535
#define MAX_FLOAT 99999.99
#define SAMPLES_PER_PIXEL 8
#define MAX_RECURSION 4
layout (location = 0) out vec4 ocolor;
layout (location = 0) in vec2 outUV;

struct Sphere {
    vec3 center;
    float radius;
    vec3 albedo;
    float material;
};


layout(std430, set = 0, binding = 1) buffer spheres {
    Sphere spheres[];
} scene;

layout(push_constant) uniform PushConstants {
    float time;
    uint numSpheres;
} pc;

// UTILS
// random number generator
vec2 randState;

float hash( const float n ) 
{
    return fract(sin(n)*43758.54554213);
}


float rand2D()
{
    randState.x = fract(sin(dot(randState.xy, vec2(12.9898, 78.233))) * 43758.5453);
    randState.y = fract(sin(dot(randState.xy, vec2(12.9898, 78.233))) * 43758.5453);;
    
    return randState.x;
}

// Jenkins hash function. TODO: check if we need something better.
uint hash1(uint x) {
    x += (x << 10u);
    x ^= (x >>  6u);
    x += (x <<  3u);
    x ^= (x >> 11u);
    x += (x << 15u);
    return x;
}

uint hash1_mut(inout uint h) {
    uint res = h;
    h = hash1(h);
    return res;
}
uint hash_combine2(uint x, uint y) {
    uint M = 1664525u, C = 1013904223u;
    uint seed = (x * M + y + C) * M;

    // Tempering (from Matsumoto)
    seed ^= (seed >> 11u);
    seed ^= (seed << 7u) & 0x9d2c5680u;
    seed ^= (seed << 15u) & 0xefc60000u;
    seed ^= (seed >> 18u);
    return seed;
}

uint hash2(uvec2 v) {
    return hash_combine2(v.x, hash1(v.y));
}

uint hash3(uvec3 v) {
    return hash_combine2(v.x, hash2(v.yz));
}

uint hash4(uvec4 v) {
    return hash_combine2(v.x, hash3(v.yzw));
}
float radical_inverse_vdc(uint bits) {
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return float(bits) * 2.3283064365386963e-10; // / 0x100000000
}
vec2 hammersley(uint i, uint n) {
    return vec2(float(i + 1) / n, radical_inverse_vdc(i + 1));
}
float hash12(vec2 p) {
    vec3 p3  = fract(vec3(p.xyx) * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
vec2 hash22(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
    p3 += dot(p3, p3.yzx+33.33);
    return fract((p3.xx+p3.yz)*p3.zy);
}
vec3 hash32(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
    p3 += dot(p3, p3.yxz+33.33);
    return fract((p3.xxy+p3.yzz)*p3.zyx);
}
float uint_to_u01_float(uint h) {
    uint mantissaMask = 0x007FFFFFu;
    uint one = 0x3F800000u;

    h &= mantissaMask;
    h |= one;

    float  r2 = float( h );
    return r2 - 1.0;
}
    float random(vec2 co)
{
    highp float a = 12.9898;
    highp float b = 78.233;
    highp float c = 43758.5453;
    highp float dt= dot(co.xy ,vec2(a,b));
    highp float sn= mod(dt,3.14);
    return fract(sin(sn) * c);
}
vec3 randomInUnitSphere(vec2 p) {
    float phi = 2.0 * PI * hash32(p).x;
    float cosTheta = 2.0 * hash32(p).y - 1.0;
    float u = hash32(p).z;

    float theta = acos(cosTheta);
    float r = pow(u, 1.0 / 3.0);

    float x = r * sin(theta) * cos(phi);
    float y = r * sin(theta) * sin(phi);
    float z = r * cos(theta);

    return vec3(x, y, z);
}

vec3 randomUnitVector(vec2 p) {
    return normalize(randomInUnitSphere(p));
}
// RAY

struct Ray {
    vec3 origin;
    vec3 direction;
};

vec3 rayAt(Ray ray, float t)
{
    return ray.origin + t*ray.direction;
}

// CAMERA

struct Camera {
    vec3 origin, lowerLeftCorner, horizontal, vertical;
};

Camera makeCamera()
{
    vec3 lookfrom = vec3(25.0, 4.0, 3.0);
    const vec3 lookat = vec3(0.0, 0.0, 0.0);
    vec3 vup = vec3(0, 1.0, 0);
    float aspect_ratio = 16/9;
    float theta = radians(20.0);
    
    float angle = pc.time / 2.0;
    mat4 rotationMatrix = mat4(cos(angle), 0.0, -sin(angle), 0.0,
                                    0.0, 1.0,        0.0, 0.0,
                            sin(angle),  0.0, cos(angle), 0.0,
                                    0.0,  0.0,        0.0, 1.0);

    lookfrom = vec3(rotationMatrix * vec4(lookfrom, 1.0));

    float h = tan(theta/2.0);
    float viewport_height = 2.0 * h;
    float viewport_width = aspect_ratio * viewport_height;
    vec3 w = normalize(lookfrom - lookat);
    vec3 u = normalize(cross(vup, w));
    vec3 v = cross(w, u);

    vec3 origin = lookfrom;
    
    vec3 horizontal = viewport_width * u;
    vec3 vertical = viewport_height * v;
    
    vec3 lowerLeftCorner = origin - horizontal/2.0 - vertical/2.0 - w;

    return Camera(origin, lowerLeftCorner, horizontal, vertical);
}

// MATERIAL
float material_diffuse = 0.0;
float material_metal = 1.0;

// INTERSECTIONS

struct Hit
{
    float t;
    vec3 point;
    vec3 normal;
    bool frontFace;
    float material;
    vec3 albedo;
};

bool hit(Ray r, int index, float t_min, float t_max, inout Hit rec)
{
    vec3 oc = r.origin - scene.spheres[index].center;

    float a = dot(r.direction, r.direction);
    float halfB = dot(oc, r.direction);
    float c = dot(oc, oc) - scene.spheres[index].radius*scene.spheres[index].radius;

    float discriminant = halfB*halfB - a * c;
    
    if (discriminant < 0.0)
    {
        return false;
    }

    float sqrtd = sqrt(discriminant);
    
    float t = (-halfB - sqrtd) / a;
    if (t < t_min || t > t_max)
    {
        t = (-halfB + sqrtd) / a;
        if (t < t_min || t > t_max)
        {
            return false;
        }
        return false;
    }

    vec3 p = rayAt(r, t);
        
    vec3 normal = p - scene.spheres[index].center;

    bool frontFace = dot(r.direction, normal) > 0.0;

    normal = frontFace ? -normal : normal;
    normal /= scene.spheres[index].radius;
    
    rec = Hit(t, p, normal, frontFace, scene.spheres[index].material, scene.spheres[index].albedo);
    
    return true;
}

bool raycast(const in Ray r, inout Hit h)
{
    bool didHit = false;
    float t_max = MAX_FLOAT;
    
    for (int index = 0; index < pc.numSpheres; index++)
    {        
        if (hit(r, index, 0.00001, t_max, h))
        {
            didHit   = true;
            t_max = h.t;
        }
    }
    return didHit;
}

vec3 rayColor(Ray r, vec2 seed)
{
    Hit rec;    
    vec3 col = vec3(1.0);

    for(int i=0; i < MAX_RECURSION; i++){
        bool didHit = raycast(r, rec);
        if (didHit)
        {
            if (rec.material == material_diffuse)
            {
                seed += float(i);
                vec3 rand = randomInUnitSphere(seed);
                vec3 target = rec.point + rec.normal + rand;
                r.origin = rec.point;
                r.direction = normalize(target - rec.point);
                col *= rec.albedo;
            } else if (rec.material == material_metal)
            {
                vec3 target = rec.point + rec.normal;
                r.origin = rec.point + rec.normal * 0.001;
                r.direction = reflect(normalize(r.direction), rec.normal);
                col *= rec.albedo;
            }
        }
        else
        {
            vec3 unitDirection = normalize(r.direction);
            float t = 0.5 * (unitDirection.y + 1.0);
            col *= mix(vec3(1.0), vec3(0.5,0.7,1.0), t);
            break;
        }
    }

    return col;
}
void main()
{
    // Normalized pixel coordinates (from 0 to 1)
    vec2 uv = outUV;

    Camera camera = makeCamera();

    vec3 col = vec3(0);

    for (float s = 0.0; s < SAMPLES_PER_PIXEL; ++s)
    {
        vec2 seed = hash22(outUV + s + pc.time);
    
        Ray r = Ray(camera.origin, normalize(camera.lowerLeftCorner + uv.x * camera.horizontal + uv.y * camera.vertical - camera.origin));
        col += rayColor(r, seed);
    }
    
    float scale = 1.0 / SAMPLES_PER_PIXEL;
    col = col * scale;
    
    ocolor = vec4(col, 1.0);
}