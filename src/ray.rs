use nalgebra::Vector3;

struct Ray {
    orig: Vector3<f32>,
    dir: Vector3<f32>,
}

impl Ray {
    pub fn new(origin: Vector3<f32>, direction: Vector3<f32>) -> Self {
        return Ray {
            orig: origin,
            dir: direction,
        };
    }

    fn at(&self, t: f32) -> Vector3<f32> {
        return self.orig + t * self.dir;
    }
}
