#[macro_use]
extern crate build_cfg;

#[build_cfg_main]
fn main() {
    println!("cargo:rerun-if-changed=src/perf/perf.c");
    println!("cargo:rerun-if-changed=src/perf/perf.h");
    println!("cargo:rerun-if-changed=src/perf/counter.h");
    println!("cargo:rerun-if-changed=src/perf/counter.c");
    cc::Build::new()
        .file("src/perf/perf.c")
        .file("src/perf/counter.c")
        .compile("libperf.a");
}
