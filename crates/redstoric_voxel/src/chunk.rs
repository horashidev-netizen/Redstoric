use crate::block::BlockType;

pub const CHUNK_SIZE_X: usize = 16;
pub const CHUNK_SIZE_Y: usize = 64;
pub const CHUNK_SIZE_Z: usize = 16;

#[derive(Clone)]
pub struct ChunkData {
    pub blocks: [[[BlockType; CHUNK_SIZE_Z]; CHUNK_SIZE_Y]; CHUNK_SIZE_X],
}

impl Default for ChunkData {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkData {
    pub fn new() -> Self {
        Self {
            blocks: [[[BlockType::Air; CHUNK_SIZE_Z]; CHUNK_SIZE_Y]; CHUNK_SIZE_X],
        }
    }

    #[inline(always)]
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        self.blocks[x][y][z]
    }

    #[inline(always)]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockType) {
        self.blocks[x][y][z] = block;
    }
}