pub struct Quirks {
    pub load_store: bool,
    pub shift: bool,
    pub jump: bool,
    pub vf_reset: bool,
    pub clip: bool 
}

impl Quirks {
    pub fn new(ld: bool, shift: bool, jump: bool, vf_reset: bool, clip: bool) -> Self {
        Quirks {
            load_store: ld,
            shift: shift,
            jump: jump,
            vf_reset: vf_reset,
            clip: clip 
        }
    }
}
