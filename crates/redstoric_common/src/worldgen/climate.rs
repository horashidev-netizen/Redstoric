use noise::{NoiseFn, SuperSimplex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    Ocean,
    Desert,
    Savanna,
    Forest,
    SnowMountain,
}

pub struct ClimateGenerator {
    temp_noise: SuperSimplex,
    humid_noise: SuperSimplex,
}

impl ClimateGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            temp_noise: SuperSimplex::new(seed + 300),
            humid_noise: SuperSimplex::new(seed + 400),
        }
    }

    /// Trả về (Temperature, Humidity) trong khoảng [0.0, 1.0]
    pub fn sample_climate(&self, wx: f64, wy: f64, wz: f64) -> (f32, f32) {
        let scale = 0.0008;

        let raw_temp = self.temp_noise.get([wx * scale, wy * scale]) as f32;
        let raw_humid = self.humid_noise.get([wx * scale, wy * scale]) as f32;

        // Chuẩn hóa về [0.0, 1.0]
        let mut temp = (raw_temp + 1.0) * 0.5;
        let humid = (raw_humid + 1.0) * 0.5;

        // Càng lên cao nhiệt độ càng giảm (Lapse rate)
        if wz > 60.0 {
            temp -= ((wz - 60.0) as f32 * 0.01).clamp(0.0, 0.6);
        }

        (temp.clamp(0.0, 1.0), humid)
    }

    pub fn get_biome(&self, temp: f32, humid: f32, wz: f64, surface_z: f64, sea_level: f64) -> Biome {
        if surface_z < sea_level {
            return Biome::Ocean;
        }
        if wz > 85.0 || temp < 0.25 {
            return Biome::SnowMountain;
        }
        if temp > 0.65 {
            if humid < 0.35 {
                Biome::Desert
            } else {
                Biome::Savanna
            }
        } else {
            Biome::Forest
        }
    }
}