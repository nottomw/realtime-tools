mod test {
    include!(concat!(env!("OUT_DIR"), "/test.skel.rs"));
}

use anyhow::{bail, Result};
use std::{thread, time};
use test::*;

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

fn main() {
    let mut skel_builder = TestSkelBuilder::default();

    skel_builder.obj_builder.debug(true);

    bump_memlock_rlimit().expect("couldn't bumb memlock rlimit");

    let mut open_skel = skel_builder.open().unwrap();

    let pid: i32 = std::process::id().try_into().unwrap();
    open_skel.rodata().my_pid = pid;

    let mut skel = open_skel.load().unwrap();
    skel.attach().unwrap();

    let sleep_time_seconds = 15;
    println!("Sleeping for {}s...", sleep_time_seconds);
    std::thread::sleep(time::Duration::from_secs(sleep_time_seconds));

    println!("Done...");
}
