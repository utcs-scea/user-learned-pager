pub mod counter {
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct PassAround {
        pub fd0: i64,
        pub ids: *mut u64,
        pub strings: *mut *mut u8,
    }
    pub const fn create_empty() -> PassAround {
        PassAround {
            fd0: -1,
            ids: std::ptr::null_mut(),
            strings: std::ptr::null_mut(),
        }
    }
}

#[link(name = "perf", kind = "static")]
extern "C" {
    pub fn size_counters() -> u8;
    pub fn create_counters() -> counter::PassAround;
    pub fn reset_counters(pa0: counter::PassAround);
    pub fn start_counters(pa0: counter::PassAround);
    pub fn stop_counters(pa0: counter::PassAround);
    pub fn print_counters(pa0: counter::PassAround, output_fd: i64, buffer: *mut u64);
}
