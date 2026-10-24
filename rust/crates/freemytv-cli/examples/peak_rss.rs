//! Peak memory of a command, for tests/bench/bench.py: `peak_rss CMD ARGS...` prints the child's peak RSS in KB.
//! Spawned from this small process, so the number isn't inflated by a large parent (as it is when Python spawns).
use std::process::{exit, Command, Stdio};

#[repr(C)]
struct Rusage {
    utime: [i64; 2],
    stime: [i64; 2],
    maxrss: i64,
    rest: [i64; 13],
}

extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn");
    if !status.success() {
        exit(1);
    }
    let mut u = Rusage {
        utime: [0; 2],
        stime: [0; 2],
        maxrss: 0,
        rest: [0; 13],
    };
    unsafe { getrusage(-1, &mut u) }; // RUSAGE_CHILDREN
    println!("{}", u.maxrss);
}
