#[derive(Debug, Clone)]
pub struct IsochTransfer {
    pub endpoint_id: usize,
    pub buffer_addr_len: (usize, usize),
    pub request_times: usize,
    pub packet_size: usize,
}
