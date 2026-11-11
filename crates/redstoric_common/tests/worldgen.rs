//! Test sinh thế giới: cùng seed phải cho cùng kết quả (server và mọi client tự sinh, không gửi chunk
//! qua mạng), và thế giới là một hòn đảo hữu hạn.

use redstoric_common::chunk::{ChunkData, CHUNK_SIZE_X, CHUNK_SIZE_Y, CHUNK_SIZE_Z};
use redstoric_common::{chunk_in_world, WorldGenerator, WORLD_CHUNKS_HALF, WORLD_HALF, WORLD_SEED};

fn same_chunk(a: &ChunkData, b: &ChunkData) -> bool {
    for x in 0..CHUNK_SIZE_X {
        for y in 0..CHUNK_SIZE_Y {
            for z in 0..CHUNK_SIZE_Z {
                if a.get_block(x, y, z) != b.get_block(x, y, z) {
                    return false;
                }
            }
        }
    }
    true
}

#[test]
fn same_seed_gives_identical_chunks() {
    let a = WorldGenerator::new(WORLD_SEED);
    let b = WorldGenerator::new(WORLD_SEED);
    for (cx, cz) in [(0, 0), (3, -2), (-7, 5)] {
        assert!(same_chunk(&a.generate_chunk(cx, cz), &b.generate_chunk(cx, cz)), "chunk ({cx}, {cz}) khác nhau");
    }
}

#[test]
fn different_seeds_give_different_terrain() {
    let a = WorldGenerator::new(WORLD_SEED);
    let b = WorldGenerator::new(WORLD_SEED.wrapping_add(1));
    let any_diff = [(0, 0), (4, 4), (-6, 2), (8, -8)]
        .iter()
        .any(|&(cx, cz)| !same_chunk(&a.generate_chunk(cx, cz), &b.generate_chunk(cx, cz)));
    assert!(any_diff, "hai seed khác nhau cho địa hình giống hệt");
}

#[test]
fn world_is_a_finite_island() {
    let g = WorldGenerator::new(WORLD_SEED);
    let half = WORLD_HALF as f64;
    assert!(g.island_value(0.0, 0.0) > 0.9, "tâm thế giới phải là đất liền");
    assert!(g.island_value(half - 1.0, 0.0) < 1e-9, "rìa thế giới phải là biển");
    assert!(g.island_value(0.0, -half + 1.0) < 1e-9, "rìa thế giới phải là biển");
    assert!(g.island_value(half, half) < 1e-9, "góc thế giới phải là biển");
}

#[test]
fn surface_height_stays_inside_chunk_column() {
    let g = WorldGenerator::new(WORLD_SEED);
    let half = WORLD_HALF;
    let mut x = -half;
    while x < half {
        let mut z = -half;
        while z < half {
            let h = g.get_surface_height(x as f64, z as f64);
            assert!(h < CHUNK_SIZE_Y, "độ cao {h} vượt cột chunk tại ({x}, {z})");
            z += 64;
        }
        x += 64;
    }
}

#[test]
fn chunk_bounds_match_world_size() {
    assert!(chunk_in_world(0, 0));
    assert!(chunk_in_world(-WORLD_CHUNKS_HALF, WORLD_CHUNKS_HALF - 1));
    assert!(!chunk_in_world(WORLD_CHUNKS_HALF, 0));
    assert!(!chunk_in_world(0, -WORLD_CHUNKS_HALF - 1));
    assert_eq!(WORLD_CHUNKS_HALF as usize * CHUNK_SIZE_X, WORLD_HALF as usize);
}
