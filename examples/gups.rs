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
    MatrixMultiplication
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum MapType {
    Normal,
    HugePagesOnly,
    BasePagesOnly,
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

    /// Function that should be used
    #[clap(short, long, value_enum, default_value_t=GupsFunction::ShiftXor)]
    function_type: GupsFunction,

    /// Type of mapping that should be used
    #[clap(short, long, value_enum, default_value_t=MapType::Normal)]
    map_type: MapType,
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

fn matrix_multiply(size: usize) -> Vec<Vec<f64>> {
    let a = vec![vec![1.0; size]; size];
    let b = vec![vec![1.0; size]; size];
    let mut c = vec![vec![0.0; size]; size];

    for i in 0..size {
        for j in 0..size {
            for k in 0..size {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    c
}
static STREAM: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let stats_fd = 3i64;

    let pa0 = unsafe { counter::create_counters() };

    let map_func: Option<Box<dyn FnMut() -> bool>> = match args.map_type {
        MapType::Normal => None,
        MapType::HugePagesOnly => Some(Box::new(|| true)),
        MapType::BasePagesOnly => Some(Box::new(|| false)),
    };

    if args.map_type == MapType::HugePagesOnly || args.map_type == MapType::BasePagesOnly {
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
    sigsegv::initialize(pa0, pointer, args.size_buffer, Some(file), map_func)?;
    let slice: &mut [usize] = unsafe {
        std::slice::from_raw_parts_mut(
            pointer_slice as *mut usize,
            args.size_buffer / std::mem::size_of::<usize>(),
        )
    };

    // Initialize Timer
    if args.timer {
        match args.function_type {
            GupsFunction::PhaseShifting => {
                let count: Box<u64> = Box::new(0);
                let count_ref: &'static mut u64 = Box::leak(count);
                let f = Box::new(|| {
                    if *count_ref % 10 == 0 {
                        STREAM.store(!STREAM.load(Ordering::Relaxed), Ordering::Relaxed)
                    }
                    *count_ref += 1;
                });
                timer_sampler::initialize(pa0, stats_fd, Some(args.usecs), Some(f));
            }
            _ => {
                timer_sampler::initialize(pa0, stats_fd, Some(args.usecs), None);
            }
        }
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
        GupsFunction::MatrixMultiplication => {
            let matrix_size = 100;
            let _result = matrix_multiply(matrix_size);
            // println!("")
        }
    }

    timer_sampler::finalize();

    for i in 0..(size / (1 << 9)) {
        println!("{:?}", slice[i * (1usize << 9)]);
    }
    Ok(())
}
