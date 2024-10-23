use clap::{Parser, ValueEnum};
use errno::errno;
use mmap_shim::counter;
use mmap_shim::{sigsegv, timer_sampler};
use std::fs::File;
use std::ops::{BitXorAssign, Shl, Shr};
use std::os::fd::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(ValueEnum, Clone, Debug)]
enum GupsFunction {
    ShiftXor,
    PhaseShifting,
}

/// Gups Variant to check overheads
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Size of buffer in bytes
    #[arg(short, long)]
    size_buffer: usize,

    /// Number of times to request buffer
    #[arg(short, long)]
    num_attempts: u64,

    /// Enable Timer Measurements
    #[arg(short, long)]
    timer: bool,

    /// Microseconds of Timer Signal
    #[arg(short, long)]
    usecs: i64,

    /// Disable Transparent Huge Pages
    #[arg(short, long)]
    disable_thp: bool,

    /// Function that should be used
    #[clap(value_enum, default_value_t=GupsFunction::ShiftXor)]
    function_type: GupsFunction,
}

#[derive(Clone)]
struct ShiftXor<T: Shl<u8, Output = T> + Shr<u8, Output = T> + BitXorAssign + Copy> {
    x: T,
    y: T,
    z: T,
    w: T,
}

impl<T: Shl<u8, Output = T> + Shr<u8, Output = T> + BitXorAssign + Copy> ShiftXor<T> {
    fn simplerand(&mut self) -> T {
        let mut t: T = self.x;
        t ^= t << 11;
        t ^= t >> 8;
        self.x = self.y;
        self.y = self.z;
        self.z = self.w;
        self.w ^= self.w >> 19;
        self.w ^= t;
        return self.w;
    }
}

static STREAM: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stats_fd = 3i64;

    let pa0 = unsafe { counter::create_counters() };

    if args.disable_thp {
        let res = unsafe { libc::prctl(libc::PR_SET_THP_DISABLE, 1, 0, 0, 0) };
        if res != 0 {
            let e = errno();
            eprintln!("res was {}", res);
            eprintln!("prctl had Error {}: {}", e.0, e);
            eprintln!("prctl called with: {}", libc::PR_SET_THP_DISABLE);
            panic!("Unable to disable THP");
        }
    }

    // Setup pointer sizes
    let pointer = unsafe { sigsegv::find_free_mem(args.size_buffer)? };
    let pointer_slice = pointer as *mut u8;
    let file = unsafe { File::from_raw_fd(std::io::stderr().as_raw_fd()) };
    sigsegv::initialize(pa0, pointer, args.size_buffer, Some(file))?;
    let slice: &mut [usize] = unsafe {
        std::slice::from_raw_parts_mut(
            pointer_slice as *mut usize,
            args.size_buffer / std::mem::size_of::<usize>(),
        )
    };

    // Initialize Timer
    if args.timer {
        timer_sampler::initialize(pa0, stats_fd, Some(args.usecs), None);
    } else {
        timer_sampler::initialize_no_timer(pa0, stats_fd);
    }
    let size = slice.len();

    let mut prand = ShiftXor {
        w: 1,
        x: 4,
        y: 7,
        z: 13,
    };

    match args.function_type {
        GupsFunction::ShiftXor => {
            for _ in 0..args.num_attempts {
                slice[prand.simplerand() % size] ^= prand.simplerand();
            }
        }
        GupsFunction::PhaseShifting => {
            let mut stream_offset: usize = 0;
            for _ in 0..args.num_attempts {
                match STREAM.load(Ordering::Relaxed) {
                    false => slice[prand.simplerand() % size] ^= prand.simplerand(),
                    true => {
                        stream_offset += 1usize << 12;
                        prand.simplerand();
                        slice[stream_offset % size] ^= prand.simplerand();
                    }
                }
            }
        }
    }

    timer_sampler::finalize();

    for i in 0..(size / (1 << 9)) {
        println!("{:?}", slice[i * (1usize << 9)]);
    }
    Ok(())
}
