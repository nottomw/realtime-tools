mod test {
    include!(concat!(env!("OUT_DIR"), "/test.skel.rs"));
}

use anyhow::{bail, Result};
use std::env;
use std::time::Duration;
use std::{thread, time};
use test::*;
use thread_priority;

fn bump_memlock_rlimit() -> Result<()> {
    let rlimit = libc::rlimit {
        rlim_cur: 128 << 20,
        rlim_max: 128 << 20,
    };

    if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
        bail!("Failed to increase rlimit");
    }

    Ok(())
}

fn rt_thread_test() {
    use thread_priority::*;

    thread_priority::set_thread_priority_and_policy(
        thread_priority::thread_native_id(),
        ThreadPriority::Crossplatform(ThreadPriorityValue::try_from(10).unwrap()),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin),
    )
    .unwrap();

    for _ in 1..5 {
        println!("RT thread sleeping for 1s...");
        thread::sleep(Duration::from_secs(2));
    }
}

fn self_test() {
    let handle1 = thread::spawn(rt_thread_test);
    let handle2 = thread::spawn(rt_thread_test);

    handle2.join().unwrap();
    handle1.join().unwrap();
}

fn main() {
    let current_pid: i32 = std::process::id() as i32;
    let mut pid_to_trace: i32 = current_pid;
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("missing pid to track, tracing {}", pid_to_trace);
    } else {
        pid_to_trace = args[1].parse::<i32>().unwrap();
    }

    println!("Tracking PID: {}", pid_to_trace);

    bump_memlock_rlimit().expect("couldn't bumb memlock rlimit");

    let mut skel_builder = TestSkelBuilder::default();
    skel_builder.obj_builder.debug(true);

    let mut open_skel = skel_builder.open().unwrap();
    open_skel.rodata().pid_to_trace = pid_to_trace;

    let mut skel = open_skel.load().unwrap();
    skel.attach().unwrap();

    if pid_to_trace == current_pid as i32 {
        self_test();
    }

    println!("Sleeping...");
    thread::sleep(Duration::from_secs(20));

    println!("Done...");
}
