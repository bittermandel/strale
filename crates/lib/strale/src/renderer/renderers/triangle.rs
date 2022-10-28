use std::{
    sync::Arc,
    time::{self, SystemTime},
};

use vulkano::{
    buffer::BufferContents,
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            render_pass::PipelineRenderingCreateInfo,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline,
    },
    shader::ShaderModule,
};

use crate::renderer::{backend::Backend, vertex::Vertex};

pub fn render_triangle(backend: &Backend) -> (Arc<GraphicsPipeline>, impl BufferContents) {
    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
				#version 450
				layout(location = 0) in vec2 position;
				void main() {
					gl_Position = vec4(position, 0.0, 1.0);
				}
			"
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
            #version 450
            
            #define PI 3.14159
            #define MAX_FLOAT 1e5
            #define SAMPLES_PER_PIXEL 100.
            #define MAX_RECURSION 16

            layout(location = 0) out vec4 ocolor;

            vec2 iResolution = vec2(1920., 1080.);
            
            layout(push_constant) uniform PushConstantData {
                float iTime;
              } pc;
            
            // UTILS
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
            
            vec3 randomInUnitSphere(vec2 p) {
                vec3 rand = hash32(p);
                float phi = 2.0 * PI * rand.x;
                float cosTheta = 2.0 * rand.y - 1.0;
                float u = rand.z;
            
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
                vec3 lookfrom = vec3(13.0, 2.0, 3.0);
                vec3 lookat = vec3(0, 1.0, -1.0);
                vec3 vup = vec3(0, -1.0, 0);
                float aspect_ratio = iResolution.x / iResolution.y;

                float theta = radians(20.0);
                
                float h = tan(theta/2.0);
                float viewport_height = 2.0 * h;
                float viewport_width = aspect_ratio * viewport_height;

                vec3 w = normalize(lookfrom - lookat);
                vec3 u = normalize(cross(vup, w));
                vec3 v = cross(w, u);
            
                vec3 origin = vec3(0);    
                
                vec3 horizontal = viewport_width * u;
                vec3 vertical = viewport_height * v;
                
                vec3 lowerLeftCorner = origin - horizontal/2.0 - vertical/2.0 - w;
            
                return Camera(origin, lowerLeftCorner, horizontal, vertical);
            }
            
            // GEOMETRY
            
            struct Sphere
            {
                vec3 center;
                float radius;
                vec3 color;
            };
            
            // INTERSECTIONS
            
            struct Hit
            {
                float t;
                vec3 point;
                vec3 normal;
                bool frontFace;
                vec3 color;
            };
            
            bool hit(Ray r, Sphere sphere, float t_min, float t_max, inout Hit rec)
            {
                vec3 oc = r.origin - sphere.center;
            
                float halfB = dot(oc, r.direction);
                float c = dot(oc, oc) - sphere.radius*sphere.radius;
            
                float discriminant = halfB*halfB - c;
                
                if (discriminant < 0.0)
                {
                    return false;
                }
            
                float sqrtd = sqrt(discriminant);
                
                float t1 = -halfB - sqrtd;
                float t2 = -halfB + sqrtd;
                
                float t = t1 < 0.001 ? t2 : t1;
                if (t < t_min || t > t_max)
                {
                    return false;
                }
            
                vec3 p = rayAt(r, t);
                    
                vec3 normal = p - sphere.center;
            
                bool frontFace = dot(r.direction, normal) > 0.0;
            
                normal = frontFace ? -normal : normal;
                normal /= sphere.radius;
                
                rec = Hit(t, p, normal, frontFace, sphere.color);
                
                return true;
            }
            
            
            bool raycast(const in Ray r, inout Hit h, inout Sphere[2] spheres)
            {
                bool didHit = false;
                float t_max = MAX_FLOAT;
                
                didHit = hit(r, spheres[1], 0.001, t_max, h) || didHit;
                if (didHit)
                {
                    float t_max = h.t;
                }
                didHit = hit(r, spheres[0], 0.001, t_max, h) || didHit;
                
                return didHit;
            }
            
            vec3 rayColor(Ray r, inout Sphere[2] spheres, vec2 seed)
            {
                Hit rec;    
                vec3 col = vec3(1.0);
                vec3 rand = randomInUnitSphere(seed);
            
                for(int i=0; i < MAX_RECURSION; i++){
                    bool didHit = raycast(r, rec, spheres);
                    seed += float(i);
                    if (didHit)
                    {
                        vec3 target = rec.point + rec.normal + rand;
                        r.origin = rec.point;
                        r.direction = normalize(target - rec.point);
                        col *= 0.5;
                    }
                    else
                    {
                        vec3 unitDirection = normalize(r.direction);
                        float t = 0.5 * (unitDirection.y + 1.0);
                        col *= mix(vec3(1.0), vec3(0.5,0.7,1.0), t);
                        return col;
                    }
                }
            
                return col;
            }
            
            bool raycastIndex(const in Ray r, out int index, inout Hit h, inout Sphere[2] spheres)
            {
                bool didHit = false;
                float t_max = MAX_FLOAT;
                
                bool sphere2Hit = hit(r, spheres[1], 0.00001, t_max, h) || didHit;
                if (sphere2Hit)
                {
                    float t_max = h.t;
                    index = 1;
                    didHit = true;
                }
                bool sphere1Hit = hit(r, spheres[0], 0.00001, t_max, h);
                if (sphere1Hit)
                {
                    float t_max = h.t;
                    index = 0;
                    didHit = true;
                }
                
                return didHit;
            }
            
            void main()
            {
                 // Normalized pixel coordinates (from 0 to 1)
                vec2 uv = gl_FragCoord.xy/iResolution.xy;
            
                Camera camera = makeCamera();
            
                vec3 col;
                
                Sphere spheres[] = Sphere[](
                    Sphere(vec3(0.0, 1.0, -1.0), 0.5, vec3(1.0, 0.0, 0.0)),
                    Sphere(vec3(0.0, -100.5, -1.0), 100.0, vec3(0.0, 1.0, 0.0))
                );
            
                for (float s = 0.0; s < SAMPLES_PER_PIXEL; ++s)
                {
                    vec2 seed = hash22(gl_FragCoord.xy + s * (mod(pc.iTime, 100.)));
                
                    Ray r = Ray(camera.origin, normalize(camera.lowerLeftCorner + uv.x * camera.horizontal + uv.y * camera.vertical - camera.origin));
                    col += rayColor(r, spheres, seed);
                }
                
                float scale = 1.0 / SAMPLES_PER_PIXEL;
                col = col * scale;
                
                ocolor = vec4(col, 1.0);
            }
            
			",
            types_meta: {
                use bytemuck::{Pod, Zeroable};

                #[derive(Clone, Copy, Zeroable, Pod)]
            },
        }
    }

    let vs = vs::load(backend.device.raw.clone()).unwrap();
    let fs = fs::load(backend.device.raw.clone()).unwrap();

    let push_constant = fs::ty::PushConstantData {
        iTime: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32(),
    };

    let pipeline = GraphicsPipeline::start()
        // We describe the formats of attachment images where the colors, depth and/or stencil
        // information will be written. The pipeline will only be usable with this particular
        // configuration of the attachment images.
        .render_pass(PipelineRenderingCreateInfo {
            // We specify a single color attachment that will be rendered to. When we begin
            // rendering, we will specify a swapchain image to be used as this attachment, so here
            // we set its format to be the same format as the swapchain.
            color_attachment_formats: vec![Some(backend.swapchain.raw.image_format())],
            ..Default::default()
        })
        // We need to indicate the layout of the vertices.
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        // The content of the vertex buffer describes a list of triangles.
        .input_assembly_state(InputAssemblyState::new())
        // A Vulkan shader can in theory contain multiple entry points, so we have to specify
        // which one.
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        // Use a resizable viewport set to draw over the entire window
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        // See `vertex_shader`.
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
        .build(backend.device.raw.clone())
        .unwrap();

    return (pipeline, push_constant);
}
