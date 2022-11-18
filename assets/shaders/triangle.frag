#version 450

#define PI 3.1415926535
#define MAX_FLOAT 99999.99
#define SAMPLES_PER_PIXEL 8
#define MAX_RECURSION 4
layout (location = 0) out vec4 ocolor;
layout (location = 0) in vec2 outUV;

layout(push_constant) uniform PushConstants {
    float time;
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
    mat4 rotationMatrix = mat4(cos(angle), 0.0, sin(angle), 0.0,
                                    0.0, 1.0,        0.0, 0.0,
                            -sin(angle),  0.0, cos(angle), 0.0,
                                    0.0,  0.0,        0.0, 1.0);

    lookfrom = vec3(vec4(lookfrom, 1.0) * rotationMatrix);

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
int material_diffuse = 0;
int material_metal = 1;
// GEOMETRY

struct Sphere
{
    vec3 center;
    float radius;
    int material;
    vec3 albedo;
};

// INTERSECTIONS

struct Hit
{
    float t;
    vec3 point;
    vec3 normal;
    bool frontFace;
    int material;
    vec3 albedo;
};

// SCENE
Sphere scene[] = Sphere[80](
    Sphere(vec3( 0.000000, -1000.000000, 0.000000), 1000.000000, 0, vec3( 0.500000, 0.500000, 0.500000)),
    Sphere(vec3( -7.995381, 0.200000, -7.478668), 0.200000, 0, vec3( 0.380012, 0.506085, 0.762437)),
    Sphere(vec3( -7.696819, 0.200000, -5.468978), 0.200000, 0, vec3( 0.596282, 0.140784, 0.017972)),
    Sphere(vec3( -7.824804, 0.200000, -3.120637), 0.200000, 0, vec3( 0.288507, 0.465652, 0.665070)),
    Sphere(vec3( -7.132909, 0.200000, -1.701323), 0.200000, 0, vec3( 0.101047, 0.293493, 0.813446)),
    Sphere(vec3( -7.569523, 0.200000, 0.494554), 0.200000, 0, vec3( 0.365924, 0.221622, 0.058332)),
    Sphere(vec3( -7.730332, 0.200000, 2.358976), 0.200000, 0, vec3( 0.051231, 0.430547, 0.454086)),
    Sphere(vec3( -7.892865, 0.200000, 4.753728), 0.200000, 1, vec3( 0.826684, 0.820511, 0.908836)),
    Sphere(vec3( -7.656691, 0.200000, 6.888913), 0.200000, 0, vec3( 0.346542, 0.225385, 0.180132)),
    Sphere(vec3( -7.217835, 0.200000, 8.203466), 0.200000, 1, vec3( 0.600463, 0.582386, 0.608277)),
    Sphere(vec3( -5.115232, 0.200000, -7.980404), 0.200000, 0, vec3( 0.256969, 0.138639, 0.080293)),
    Sphere(vec3( -5.323222, 0.200000, -5.113037), 0.200000, 0, vec3( 0.193093, 0.510542, 0.613362)),
    Sphere(vec3( -5.410681, 0.200000, -3.527741), 0.200000, 0, vec3( 0.352200, 0.191551, 0.115972)),
    Sphere(vec3( -5.460670, 0.200000, -1.166543), 0.200000, 0, vec3( 0.029486, 0.249874, 0.077989)),
    Sphere(vec3( -5.457659, 0.200000, 0.363870), 0.200000, 0, vec3( 0.395713, 0.762043, 0.108515)),
    Sphere(vec3( -5.116586, 0.200000, 4.470188), 0.200000, 0, vec3( 0.059444, 0.404603, 0.171767)),
    Sphere(vec3( -5.273591, 0.200000, 6.795187), 0.200000, 0, vec3( 0.499454, 0.131330, 0.158348)),
    Sphere(vec3( -5.120286, 0.200000, 8.731398), 0.200000, 0, vec3( 0.267365, 0.136024, 0.300483)),
    Sphere(vec3( -3.601565, 0.200000, -7.895600), 0.200000, 0, vec3( 0.027752, 0.155209, 0.330428)),
    Sphere(vec3( -3.735860, 0.200000, -5.163056), 0.200000, 1, vec3( 0.576768, 0.884712, 0.993335)),
    Sphere(vec3( -3.481116, 0.200000, -3.794556), 0.200000, 0, vec3( 0.405104, 0.066436, 0.009339)),
    Sphere(vec3( -3.866858, 0.200000, -1.465965), 0.200000, 0, vec3( 0.027570, 0.021652, 0.252798)),
    Sphere(vec3( -3.168870, 0.200000, 0.553099), 0.200000, 0, vec3( 0.421992, 0.107577, 0.177504)),
    Sphere(vec3( -3.428552, 0.200000, 2.627547), 0.200000, 1, vec3( 0.974029, 0.653443, 0.571877)),
    Sphere(vec3( -3.771736, 0.200000, 4.324785), 0.200000, 0, vec3( 0.685957, 0.000043, 0.181270)),
    Sphere(vec3( -3.768522, 0.200000, 6.384588), 0.200000, 0, vec3( 0.025972, 0.082246, 0.138765)),
    Sphere(vec3( -3.286992, 0.200000, 8.441148), 0.200000, 0, vec3( 0.186577, 0.560376, 0.367045)),
    Sphere(vec3( -1.552127, 0.200000, -7.728200), 0.200000, 0, vec3( 0.202998, 0.002459, 0.015350)),
    Sphere(vec3( -1.360796, 0.200000, -5.346098), 0.200000, 0, vec3( 0.690820, 0.028470, 0.179907)),
    Sphere(vec3( -1.287209, 0.200000, -3.735321), 0.200000, 0, vec3( 0.345974, 0.672353, 0.450180)),
    Sphere(vec3( -1.344859, 0.200000, -1.726654), 0.200000, 0, vec3( 0.209209, 0.431116, 0.164732)),
    Sphere(vec3( -1.974774, 0.200000, 0.183260), 0.200000, 0, vec3( 0.006736, 0.675637, 0.622067)),
    Sphere(vec3( -1.542872, 0.200000, 2.067868), 0.200000, 0, vec3( 0.192247, 0.016661, 0.010109)),
    Sphere(vec3( -1.743856, 0.200000, 4.752810), 0.200000, 0, vec3( 0.295270, 0.108339, 0.276513)),
    Sphere(vec3( -1.955621, 0.200000, 6.493702), 0.200000, 0, vec3( 0.270527, 0.270494, 0.202029)),
    Sphere(vec3( -1.350449, 0.200000, 8.068503), 0.200000, 1, vec3( 0.646942, 0.501660, 0.573693)),
    Sphere(vec3( 0.706123, 0.200000, -7.116040), 0.200000, 0, vec3( 0.027695, 0.029917, 0.235781)),
    Sphere(vec3( 0.897766, 0.200000, -5.938681), 0.200000, 0, vec3( 0.114934, 0.046258, 0.039647)),
    Sphere(vec3( 0.744113, 0.200000, -3.402960), 0.200000, 0, vec3( 0.513631, 0.335578, 0.204787)),
    Sphere(vec3( 0.867750, 0.200000, -1.311908), 0.200000, 0, vec3( 0.400246, 0.000956, 0.040513)),
    Sphere(vec3( 0.082480, 0.200000, 0.838206), 0.200000, 0, vec3( 0.594141, 0.215068, 0.025718)),
    Sphere(vec3( 0.649692, 0.200000, 2.525103), 0.200000, 1, vec3( 0.602157, 0.797249, 0.614694)),
    Sphere(vec3( 0.378574, 0.200000, 4.055579), 0.200000, 0, vec3( 0.005086, 0.003349, 0.064403)),
    Sphere(vec3( 0.425844, 0.200000, 6.098526), 0.200000, 0, vec3( 0.266812, 0.016602, 0.000853)),
    Sphere(vec3( 0.261365, 0.200000, 8.661150), 0.200000, 0, vec3( 0.150201, 0.007353, 0.152506)),
    Sphere(vec3( 2.814218, 0.200000, -7.751227), 0.200000, 1, vec3( 0.570094, 0.610319, 0.584192)),
    Sphere(vec3( 2.050073, 0.200000, -5.731364), 0.200000, 0, vec3( 0.109886, 0.029498, 0.303265)),
    Sphere(vec3( 2.020130, 0.200000, -3.472627), 0.200000, 0, vec3( 0.216908, 0.216448, 0.221775)),
    Sphere(vec3( 2.884277, 0.200000, -1.232662), 0.200000, 0, vec3( 0.483428, 0.027275, 0.113898)),
    Sphere(vec3( 2.644454, 0.200000, 0.596324), 0.200000, 0, vec3( 0.005872, 0.860718, 0.561933)),
    Sphere(vec3( 2.194283, 0.200000, 2.880603), 0.200000, 0, vec3( 0.452710, 0.824152, 0.045179)),
    Sphere(vec3( 2.281000, 0.200000, 4.094307), 0.200000, 0, vec3( 0.002091, 0.145849, 0.032535)),
    Sphere(vec3( 2.080841, 0.200000, 6.716384), 0.200000, 0, vec3( 0.468539, 0.032772, 0.018071)),
    Sphere(vec3( 4.329136, 0.200000, -7.497218), 0.200000, 0, vec3( 0.030865, 0.071452, 0.016051)),
    Sphere(vec3( 4.750631, 0.200000, -3.836759), 0.200000, 0, vec3( 0.702578, 0.084798, 0.141374)),
    Sphere(vec3( 4.082084, 0.200000, -1.180746), 0.200000, 0, vec3( 0.043052, 0.793077, 0.018707)),
    Sphere(vec3( 4.429173, 0.200000, 2.069721), 0.200000, 0, vec3( 0.179009, 0.147750, 0.617371)),
    Sphere(vec3( 4.277152, 0.200000, 4.297482), 0.200000, 0, vec3( 0.422693, 0.011222, 0.211945)),
    Sphere(vec3( 4.012743, 0.200000, 6.225072), 0.200000, 0, vec3( 0.986275, 0.073358, 0.133628)),
    Sphere(vec3( 4.047066, 0.200000, 8.419360), 0.200000, 1, vec3( 0.878749, 0.677170, 0.684995)),
    Sphere(vec3( 6.441846, 0.200000, -7.700798), 0.200000, 0, vec3( 0.309255, 0.342524, 0.489512)),
    Sphere(vec3( 6.047810, 0.200000, -5.519369), 0.200000, 0, vec3( 0.532361, 0.008200, 0.077522)),
    Sphere(vec3( 6.779211, 0.200000, -3.740542), 0.200000, 0, vec3( 0.161234, 0.539314, 0.016667)),
    Sphere(vec3( 6.430776, 0.200000, -1.332107), 0.200000, 0, vec3( 0.641951, 0.661402, 0.326114)),
    Sphere(vec3( 6.476387, 0.200000, 0.329973), 0.200000, 0, vec3( 0.033000, 0.648388, 0.166911)),
    Sphere(vec3( 6.568686, 0.200000, 2.116949), 0.200000, 0, vec3( 0.590952, 0.072292, 0.125672)),
    Sphere(vec3( 6.371189, 0.200000, 4.609841), 0.200000, 1, vec3( 0.870345, 0.753830, 0.933118)),
    Sphere(vec3( 6.011877, 0.200000, 6.569579), 0.200000, 0, vec3( 0.044868, 0.651697, 0.086779)),
    Sphere(vec3( 6.096087, 0.200000, 8.892333), 0.200000, 0, vec3( 0.588587, 0.078723, 0.044928)),
    Sphere(vec3( 8.185763, 0.200000, -7.191109), 0.200000, 1, vec3( 0.989702, 0.886784, 0.540759)),
    Sphere(vec3( 8.411960, 0.200000, -5.285309), 0.200000, 0, vec3( 0.139604, 0.022029, 0.461688)),
    Sphere(vec3( 8.047109, 0.200000, -3.427552), 0.200000, 1, vec3( 0.815002, 0.631228, 0.806757)),
    Sphere(vec3( 8.119639, 0.200000, -1.652587), 0.200000, 0, vec3( 0.177852, 0.429797, 0.042251)),
    Sphere(vec3( 8.818120, 0.200000, 0.401292), 0.200000, 0, vec3( 0.065416, 0.087694, 0.040518)),
    Sphere(vec3( 8.754155, 0.200000, 2.152549), 0.200000, 0, vec3( 0.230659, 0.035665, 0.435895)),
    Sphere(vec3( 8.595298, 0.200000, 4.802001), 0.200000, 0, vec3( 0.188493, 0.184933, 0.040215)),
    Sphere(vec3( 8.036216, 0.200000, 6.739752), 0.200000, 0, vec3( 0.023192, 0.364636, 0.464844)),
    Sphere(vec3( 8.256561, 0.200000, 8.129115), 0.200000, 0, vec3( 0.002612, 0.598319, 0.435378)),
    Sphere(vec3( -4.000000, 1.000000, 0.000000), 1.000000, 0, vec3( 0.400000, 0.200000, 0.100000)),
    Sphere(vec3( 4.000000, 1.000000, 0.000000), 1.000000, 1, vec3( 0.700000, 0.600000, 0.500000))
    );

bool hit(Ray r, int index, float t_min, float t_max, inout Hit rec)
{
    Sphere sphere = scene[index];
    vec3 oc = r.origin - sphere.center;

    float a = dot(r.direction, r.direction);
    float halfB = dot(oc, r.direction);
    float c = dot(oc, oc) - sphere.radius*sphere.radius;

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
        
    vec3 normal = p - sphere.center;

    bool frontFace = dot(r.direction, normal) > 0.0;

    normal = frontFace ? -normal : normal;
    normal /= sphere.radius;
    
    rec = Hit(t, p, normal, frontFace, sphere.material, sphere.albedo);
    
    return true;
}

bool raycast(const in Ray r, inout Hit h)
{
    bool didHit = false;
    float t_max = MAX_FLOAT;
    
    for (int index = 0; index < scene.length(); index++)
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