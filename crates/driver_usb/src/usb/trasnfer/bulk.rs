#[derive(Debug, Clone)]
pub struct BulkTransfer {
    pub endpoint_id: usize,
    pub buffer_addr_len: (usize, usize),
}
