mod test {
    include!(concat!(env!("OUT_DIR"), "/test.skel.rs"));
}

use anyhow::{bail, Result};
use libbpf_rs::RingBufferBuilder;
use plain::Plain;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
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

static RT_TEST_THREAD_STOP: AtomicBool = AtomicBool::new(false);

fn rt_thread_test(id: u32, prio: u8, sleep_time: u64) {
    use thread_priority::*;

    let prio_cs = ThreadPriorityValue::try_from(prio).unwrap();

    thread_priority::set_thread_priority_and_policy(
        thread_priority::thread_native_id(),
        ThreadPriority::Crossplatform(prio_cs),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin),
    )
    .unwrap();

    let mut i: u32 = 0;
    loop {
        let should_stop = RT_TEST_THREAD_STOP.load(Ordering::Acquire);
        if should_stop {
            break;
        }

        println!("[TID #{}] {} \tsleeping for 1s...", id, i);
        thread::sleep(Duration::from_secs(sleep_time));
        println!("[TID #{}] {} \twakeup", id, i);

        i += 1;
    }

    println!("[TID #{}] stopped", id);
}

fn handle_rb_data(data: &[u8]) -> i32 {
    let mut rt_event = test_bss_types::rt_event::default();

    unsafe impl Plain for test_bss_types::rt_event {}

    plain::copy_from_bytes(&mut rt_event, data).expect("data buffer was too short");

    let mut event_type_str = "switch";
    if rt_event.event_type == test_bss_types::rt_event_type::SCHED_WAKE {
        event_type_str = "wake";
    }

    println!(
        "TASK EVENT {}, \tPID prev: {}, \tPID next: {}, \tprio prev: {}, \tprio next: {}",
        event_type_str, //
        rt_event.pid_prev,
        rt_event.pid_next,
        rt_event.priority_prev,
        rt_event.priority_next
    );

    return 0;
}

fn ringbuf_poller(mut skel: TestSkel, poll_time_seconds: u64) {
    let mut ringbuf_builder = RingBufferBuilder::new();
    ringbuf_builder
        .add(skel.maps_mut().rb(), handle_rb_data)
        .expect("could not add map to ringbuf builder");

    let ringbuf = ringbuf_builder.build().unwrap();

    let start = Instant::now();

    loop {
        ringbuf.poll(Duration::from_secs(1)).expect("poll failed");

        let dur = start.elapsed().as_secs();
        if dur >= poll_time_seconds {
            break;
        }
    }

    println!("poller done");
}

fn self_test(skel: TestSkel) {
    let handle1 = thread::spawn(|| rt_thread_test(1, 10, 3));
    let handle2 = thread::spawn(|| rt_thread_test(2, 13, 2));
    let handle3 = thread::spawn(|| rt_thread_test(3, 16, 1));

    ringbuf_poller(skel, 10);

    println!("Stopping RT threads...");
    RT_TEST_THREAD_STOP.store(true, Ordering::Release);

    handle3.join().unwrap();
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

    bump_memlock_rlimit().expect("couldn't bump memlock rlimit");

    let mut skel_builder = TestSkelBuilder::default();
    skel_builder.obj_builder.debug(true);

    let mut open_skel = skel_builder.open().unwrap();
    open_skel.rodata().pid_to_trace = pid_to_trace;

    let mut skel = open_skel.load().unwrap();
    skel.attach().unwrap();

    if pid_to_trace == current_pid as i32 {
        self_test(skel);
    } else {
        println!("Sleeping...");
        thread::sleep(Duration::from_secs(20));
    }

    println!("Done...");
}
