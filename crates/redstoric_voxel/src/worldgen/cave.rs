use noise::{NoiseFn, SuperSimplex};

pub struct CaveGenerator {
    cave_noise: SuperSimplex,
}

impl CaveGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            cave_noise: SuperSimplex::new(seed + 500),
        }
    }

    /// Trả về true nếu vị trí (wx, wy, wz) bị khoét rỗng thành hang động
    pub fn is_cave(&self, wx: f64, wy: f64, wz: f64, surface_z: f64) -> bool {
        // Không đục hang ở sát bề mặt trên cùng (tránh lở đất)
        if wz >= surface_z - 4.0 || wz <= 4.0 {
            return false;
        }

        let scale = 0.025;
        let val = self.cave_noise.get([wx * scale, wy * scale, wz * scale]).abs();

        // Giá trị càng gần 0 -> tâm ống hang động rỗng
        val < 0.075
    }
}