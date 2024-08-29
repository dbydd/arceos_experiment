#[derive(Debug, Clone)]
pub struct IsochTransfer {
    pub endpoint_id: usize,
    pub buffer_addr_len: (usize, usize),
    pub transfer_size_bytes: usize,
    pub max_packet: usize,
}
