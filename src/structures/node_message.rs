pub struct NodeMessage {
    value: u8, // 8 bits, but only 3 bits are used
}

impl NodeMessage {
    pub fn new(value: u8) -> Self {
        assert!(value < 8, "NodeMessage must fit in 3 bits (0-7)");
        Self { value }
    }

    pub fn value(&self) -> u8 {
        self.value
    }
}