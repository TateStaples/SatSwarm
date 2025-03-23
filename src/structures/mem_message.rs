pub struct MemMessage {
    value: u8, // 8 bits
}

impl MemMessage {
    pub fn new(value: u8) -> Self {
        Self { value }
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}