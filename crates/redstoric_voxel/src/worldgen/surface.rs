use noise::{Fbm, NoiseFn, Perlin, SuperSimplex};

pub struct SurfaceGenerator {
    fbm: Fbm<Perlin>,
    warp_x: SuperSimplex,
    warp_y: SuperSimplex,
    pub sea_level: f64,
}

impl SurfaceGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            fbm: Fbm::new(seed),
            warp_x: SuperSimplex::new(seed + 100),
            warp_y: SuperSimplex::new(seed + 200),
            sea_level: 32.0,
        }
    }

    /// Tính độ cao bề mặt tại tọa độ thế giới (wx, wy)
    pub fn sample_height(&self, wx: f64, wy: f64) -> f64 {
        let warp_scale = 0.0015;
        let warp_power = 60.0;

        // Domain warping: làm lệch tọa độ đầu vào
        let offset_x = self.warp_x.get([wx * warp_scale, wy * warp_scale]) * warp_power;
        let offset_y = self.warp_y.get([(wx + 123.4) * warp_scale, (wy + 567.8) * warp_scale]) * warp_power;

        let sample_x = (wx + offset_x) * 0.002;
        let sample_y = (wy + offset_y) * 0.002;

        // fBm tạo hình dáng đồi núi, nhân hệ số biên độ
        let raw_height = self.fbm.get([sample_x, sample_y]);

        // Đưa về dải độ cao mong muốn (mực nước biển ~32, núi cao ~120)
        self.sea_level + (raw_height * 45.0) + 15.0
    }
}