pub mod counter;

pub mod timer_sampler {
    use crate::counter::counter::{create_empty, PassAround};
    use crate::counter::{
        create_counters, print_counters, reset_counters, size_counters, start_counters,
        stop_counters,
    };
    use libc::{itimerval, syscall, timeval, SYS_setitimer, ITIMER_PROF, SIGPROF};
    use std::process::exit;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct AlarmHandler {
        pa0: PassAround,
        output_fd: i64,
        buffer: Vec<u64>,
        func: Option<Box<dyn FnMut() -> ()>>,
    }

    static mut SINGLE_ALARM_HANDLER: AlarmHandler = AlarmHandler {
        pa0: create_empty(),
        output_fd: -1,
        buffer: Vec::<u64>::new(),
        func: None,
    };

    static mut HANDLER_ALLOWED: AtomicBool = AtomicBool::new(false);
    static mut HANDLER_FINALIZED: AtomicBool = AtomicBool::new(false);

    static mut TIMER: itimerval = itimerval {
        it_interval: timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        it_value: timeval {
            tv_sec: 0,
            tv_usec: 1000,
        },
    };

    fn call_setitimer() {
        unsafe {
            let times: *const itimerval = std::ptr::addr_of!(TIMER);
            let no_times: *const itimerval = std::ptr::null();
            let rc = syscall(SYS_setitimer, ITIMER_PROF, times, no_times);
            if rc != 0 {
                exit(-1);
            }
        }
    }

    pub fn signal_handler(signo: i32) {
        match signo {
            SIGPROF => {
                unsafe {
                    if !HANDLER_ALLOWED.load(Ordering::Relaxed) {
                        HANDLER_FINALIZED = true.into();
                        return;
                    }
                    stop_counters(SINGLE_ALARM_HANDLER.pa0);
                    print_counters(
                        SINGLE_ALARM_HANDLER.pa0,
                        SINGLE_ALARM_HANDLER.output_fd,
                        SINGLE_ALARM_HANDLER.buffer.as_mut_ptr(),
                    );
                    match &mut SINGLE_ALARM_HANDLER.func {
                        Some(boxed_func) => boxed_func(),
                        _ => {}
                    }
                }
                call_setitimer();
                unsafe {
                    start_counters(SINGLE_ALARM_HANDLER.pa0);
                }
            }
            _ => {
                panic!("Caught weird signal: {:?}", signo);
            }
        }
    }

    pub fn initialize(
        pa0: PassAround,
        output_fd: i64,
        useconds: Option<i64>,
        func: Option<Box<dyn FnMut() -> ()>>,
    ) -> PassAround {
        unsafe {
            let num_counters = size_counters();
            SINGLE_ALARM_HANDLER = AlarmHandler {
                pa0,
                output_fd,
                buffer: Vec::<u64>::with_capacity(num_counters as usize),
                func,
            };
            HANDLER_ALLOWED = true.into();
            HANDLER_FINALIZED = false.into();
            libc::signal(SIGPROF, signal_handler as usize);
        }
        if let Some(usec) = useconds {
            unsafe {
                TIMER.it_value.tv_usec = usec;
            }
        }
        call_setitimer();
        unsafe {
            start_counters(SINGLE_ALARM_HANDLER.pa0);
            SINGLE_ALARM_HANDLER.pa0
        }
    }

    pub fn initialize_pass_around(output_fd: i64) -> PassAround {
        unsafe {
            let num_counters = size_counters();
            SINGLE_ALARM_HANDLER = AlarmHandler {
                pa0: create_counters(),
                output_fd,
                buffer: Vec::<u64>::with_capacity(num_counters as usize),
                func: None,
            };
            HANDLER_ALLOWED = true.into();
            HANDLER_FINALIZED = false.into();
            libc::signal(SIGPROF, signal_handler as usize);
        }
        call_setitimer();
        unsafe {
            start_counters(SINGLE_ALARM_HANDLER.pa0);
            SINGLE_ALARM_HANDLER.pa0
        }
    }

    pub fn initialize_no_timer(pa0: PassAround, output_fd: i64) {
        unsafe {
            let num_counters = size_counters();
            SINGLE_ALARM_HANDLER = AlarmHandler {
                pa0,
                output_fd,
                buffer: Vec::<u64>::with_capacity(num_counters as usize),
                func: None,
            };
            HANDLER_ALLOWED = false.into();
            HANDLER_FINALIZED = true.into();
        }
        unsafe {
            start_counters(SINGLE_ALARM_HANDLER.pa0);
        }
    }

    pub fn finalize() {
        unsafe {
            HANDLER_ALLOWED = false.into();
            while !HANDLER_FINALIZED.load(Ordering::Relaxed) {}
            stop_counters(SINGLE_ALARM_HANDLER.pa0);
            print_counters(
                SINGLE_ALARM_HANDLER.pa0,
                SINGLE_ALARM_HANDLER.output_fd,
                SINGLE_ALARM_HANDLER.buffer.as_mut_ptr(),
            );
            reset_counters(SINGLE_ALARM_HANDLER.pa0);
        }
    }
}

pub mod sigsegv {
    use crate::counter::counter::{create_empty, PassAround};
    use crate::counter::{start_counters, stop_counters};
    use errno::errno;
    use libc::{
        mmap, munmap, sigaction, sigaddset, sigemptyset, sighandler_t, siginfo_t, sigset_t,
        MAP_ANON, MAP_FAILED, MAP_FIXED_NOREPLACE, MAP_POPULATE, MAP_PRIVATE, PROT_READ,
        PROT_WRITE, SA_SIGINFO, SIGPROF, SIGSEGV,
    };
    use std::fs::File;
    use std::io::Write;
    use std::mem::MaybeUninit;
    use std::os::raw::{c_int, c_void};
    use std::process::exit;

    pub unsafe fn find_free_mem(size: usize) -> Result<*mut c_void, &'static str> {
        let pointer = mmap(
            std::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            default_mmap_flags(),
            -1,
            0,
        );
        match pointer {
            MAP_FAILED => Err("mmap failed looking for free mem"),
            ptr => match munmap(ptr as *mut c_void, size) {
                0 => Ok(ptr),
                _ => Err("munmap failed looking for free mem"),
            },
        }
    }

    struct SigSegvHandler {
        pa0: PassAround,
        pointer: *mut c_void,
        size: usize,
        output_file: Option<File>,
    }

    static mut SIGSEGV_HANDLER: SigSegvHandler = SigSegvHandler {
        pa0: create_empty(),
        pointer: std::ptr::null_mut(),
        size: 0,
        output_file: None,
    };

    pub const fn default_mmap_flags() -> c_int {
        MAP_PRIVATE | MAP_ANON
    }

    pub fn call_mmap(ptr: *mut c_void, len: usize, _map_huge: bool) -> Result<(), &'static str> {
        unsafe {
            if MAP_FAILED
                == mmap(
                    ptr,
                    len,
                    PROT_READ | PROT_WRITE,
                    default_mmap_flags() | MAP_FIXED_NOREPLACE | MAP_POPULATE,
                    -1,
                    0,
                )
            {
                let e = errno();
                eprintln!("mmap had Error {}: {}", e.0, e);
                let mut rlim: libc::rlimit = MaybeUninit::zeroed().assume_init();
                let _rc = libc::getrlimit(libc::RLIMIT_AS, &mut rlim);
                eprintln!(
                    "getrlimit RLIMIT_AS soft:\t{:#x};\thard:\t{:#x}",
                    rlim.rlim_cur, rlim.rlim_max
                );
                let _rc = libc::getrlimit(libc::RLIMIT_RSS, &mut rlim);
                eprintln!(
                    "getrlimit RLIMIT_RSS soft:\t{:#x};\thard:\t{:#x}",
                    rlim.rlim_cur, rlim.rlim_max
                );
                let _rc = libc::getrlimit(libc::RLIMIT_MEMLOCK, &mut rlim);
                eprintln!(
                    "getrlimit RLIMIT_MEMLOCK soft:\t{:#x};\thard:\t{:#x}",
                    rlim.rlim_cur, rlim.rlim_max
                );
                return Err("Mmap Error");
            }
            Ok(())
        }
    }

    pub unsafe fn signal_handler(signo: c_int, si: *mut siginfo_t, _: *mut c_void) {
        match signo {
            SIGSEGV => {
                let vfa: *mut c_void;
                stop_counters(SIGSEGV_HANDLER.pa0);
                if si == std::ptr::null_mut() {
                    panic!("siginfo_t is null");
                }
                vfa = (*si).si_addr();

                // Print vfa
                if let Some(out_file) = &mut SIGSEGV_HANDLER.output_file {
                    let res = out_file.write_fmt(format_args!("{:#x}\tvfa\n", vfa as usize));
                    match res {
                        Err(_) => {
                            panic!("Failed to print vfa");
                        }
                        _ => {}
                    }
                }

                // Align address
                let ufa = vfa as usize;
                let afa = ((ufa >> 12) << 12) as *mut c_void;

                // Check address
                let attempt = afa as usize;
                let lower_bound = SIGSEGV_HANDLER.pointer as usize;
                let size = SIGSEGV_HANDLER.size;
                if lower_bound > attempt || (attempt - lower_bound) >= size {
                    eprintln!(
                        "afa: {:?} - lower_bound: {:?} - size: {:#x}",
                        afa, SIGSEGV_HANDLER.pointer, size,
                    );
                    panic!("TRUE SIGSEGV");
                }

                // mmap at that address
                if let Err(err) = call_mmap(afa, 1usize << 12, false) {
                    panic!("{:?} - {:?}", err, vfa);
                }
                start_counters(SIGSEGV_HANDLER.pa0);
            }
            _ => {
                exit(-1);
            }
        }
    }

    pub fn initialize(
        pa0: PassAround,
        pointer: *mut c_void,
        size: usize,
        output_file: Option<File>,
    ) -> Result<(), &'static str> {
        unsafe {
            SIGSEGV_HANDLER = SigSegvHandler {
                pa0,
                pointer,
                size,
                output_file,
            };
            // Make sigset
            let mut sa_mask: sigset_t = MaybeUninit::zeroed().assume_init();
            let mask: *mut libc::sigset_t = &mut sa_mask;
            if 0 != sigemptyset(mask) || 0 != sigaddset(mask, SIGPROF) {
                panic!("sigset did not work");
            }
            let new_action = sigaction {
                sa_sigaction: signal_handler as sighandler_t,
                sa_mask,
                sa_flags: SA_SIGINFO,
                sa_restorer: None,
            };
            let rc = sigaction(SIGSEGV, &new_action, std::ptr::null_mut());
            match rc {
                0 => Ok(()),
                _ => Err("Sigaction Failed"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;
    use tempfile::tempfile;

    #[test]
    fn check_some_output_timer() -> Result<(), Box<dyn std::error::Error>> {
        let file = tempfile()?;
        assert_eq!(file.metadata()?.len(), 0);
        let fd = file.as_raw_fd() as i64;
        let pa = timer_sampler::initialize_pass_around(fd);
        let sigsegv_file = tempfile()?;
        let sigsegv_clone = sigsegv_file.try_clone()?;
        assert_eq!(sigsegv_file.metadata()?.len(), 0);
        let size = 1usize << 12;
        let pointer_void = unsafe { sigsegv::find_free_mem(size)? };

        sigsegv::initialize(pa, pointer_void, size, Some(sigsegv_file))?;

        unsafe {
            let pointer = pointer_void as *mut u8;
            *pointer = 5u8;
            assert_eq!(*pointer, 5u8);
        }

        timer_sampler::finalize();
        assert!(file.metadata()?.len() > 0);
        assert!(sigsegv_clone.metadata()?.len() > 0);
        Ok(())
    }
}
