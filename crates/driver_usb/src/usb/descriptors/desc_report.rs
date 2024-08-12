

#[derive(Debug)]
pub struct ReportEvent {
    pub usage_page: u32,
    pub usage: u32,
    pub value: i32,
    pub relative: bool,
}

#[derive(Debug)]
pub struct ReportInput {
    pub bit_length: usize,
    pub bit_offset: usize,
    pub global_state: GlobalItemsState,
    pub local_state: LocalItemsState,
    pub flags: MainItemFlags,
}

pub struct ReportHandler {
    pub inputs: Vec<ReportInput>,
    pub total_byte_length: usize,
    pub absolutes: HashMap<(u32, u32), i32>,
    pub arrays: HashSet<(u32, u32)>,
}

impl ReportHandler{
    pub fn new() -> Result<self,Error>{
        
    }
}