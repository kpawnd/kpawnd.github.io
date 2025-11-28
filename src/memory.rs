pub struct Memory { pub total: u32, pub free: u32 }
impl Memory { pub fn new(total: u32) -> Self { Memory { total, free: total } } pub fn alloc(&mut self, size: u32) -> Option<u32> { if size > self.free { return None; } let offset = self.total - self.free; self.free -= size; Some(offset) } pub fn usage(&self) -> (u32,u32) { (self.total - self.free, self.total) } }
