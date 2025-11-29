use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockState {
    Free,
    Allocated,
}

pub struct MemoryBlock {
    pub offset: u32,
    pub size: u32,
    pub state: BlockState,
}

pub struct Memory {
    pub total: u32,
    pub free: u32,
    blocks: BTreeMap<u32, MemoryBlock>,
}

impl Default for Memory {
    fn default() -> Self {
        Memory::new(0)
    }
}

impl Memory {
    pub fn new(total: u32) -> Self {
        let mut blocks = BTreeMap::new();
        if total > 0 {
            blocks.insert(
                0,
                MemoryBlock {
                    offset: 0,
                    size: total,
                    state: BlockState::Free,
                },
            );
        }
        Memory {
            total,
            free: total,
            blocks,
        }
    }

    // First-fit allocation strategy
    pub fn alloc(&mut self, size: u32) -> Option<u32> {
        if size > self.free || size == 0 {
            return None;
        }

        // Find first free block that fits
        let mut found_offset = None;
        for (offset, block) in self.blocks.iter() {
            if block.state == BlockState::Free && block.size >= size {
                found_offset = Some(*offset);
                break;
            }
        }

        if let Some(offset) = found_offset {
            let block_size = self.blocks.get(&offset).unwrap().size;

            // Split block if larger than needed
            if block_size > size {
                self.blocks.insert(
                    offset,
                    MemoryBlock {
                        offset,
                        size,
                        state: BlockState::Allocated,
                    },
                );
                self.blocks.insert(
                    offset + size,
                    MemoryBlock {
                        offset: offset + size,
                        size: block_size - size,
                        state: BlockState::Free,
                    },
                );
            } else {
                self.blocks.insert(
                    offset,
                    MemoryBlock {
                        offset,
                        size,
                        state: BlockState::Allocated,
                    },
                );
            }

            self.free -= size;
            return Some(offset);
        }

        None
    }

    pub fn free(&mut self, offset: u32) -> bool {
        if let Some(block) = self.blocks.get_mut(&offset) {
            if block.state == BlockState::Allocated {
                block.state = BlockState::Free;
                self.free += block.size;
                self.coalesce();
                return true;
            }
        }
        false
    }

    // Merge adjacent free blocks
    fn coalesce(&mut self) {
        let mut to_remove = Vec::new();
        let mut to_add = Vec::new();

        let offsets: Vec<u32> = self.blocks.keys().copied().collect();
        for i in 0..offsets.len() {
            if i + 1 >= offsets.len() {
                break;
            }

            let curr_offset = offsets[i];
            let next_offset = offsets[i + 1];

            if let (Some(curr), Some(next)) =
                (self.blocks.get(&curr_offset), self.blocks.get(&next_offset))
            {
                if curr.state == BlockState::Free
                    && next.state == BlockState::Free
                    && curr.offset + curr.size == next.offset
                {
                    to_remove.push(curr_offset);
                    to_remove.push(next_offset);
                    to_add.push(MemoryBlock {
                        offset: curr.offset,
                        size: curr.size + next.size,
                        state: BlockState::Free,
                    });
                }
            }
        }

        for offset in to_remove {
            self.blocks.remove(&offset);
        }
        for block in to_add {
            self.blocks.insert(block.offset, block);
        }
    }

    pub fn usage(&self) -> (u32, u32) {
        (self.total - self.free, self.total)
    }

    pub fn fragmentation(&self) -> f32 {
        let free_blocks = self
            .blocks
            .values()
            .filter(|b| b.state == BlockState::Free)
            .count();
        if self.free > 0 && free_blocks > 0 {
            1.0 - (self.free as f32 / (free_blocks as f32 * self.total as f32))
        } else {
            0.0
        }
    }
}
