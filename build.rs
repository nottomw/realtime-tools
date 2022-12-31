use libbpf_cargo::SkeletonBuilder;
use std::fs::File;
use std::io::Write;
use std::{env, path::PathBuf};

fn main() {
    const EBPF_SOURCE_FILE: &str = "src/bpf/test.bpf.c";

    let mut out =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set in build script"));

    out.push("test.skel.rs");

    let bpftool_output = std::process::Command::new("sh")
        .arg("/sbin/bpftool")
        .arg("btf")
        .arg("dump")
        .arg("file")
        .arg("/sys/kernel/btf/vmlinux")
        .arg("format")
        .arg("c")
        .output()
        .expect("could not generate vmlinux.h");

    let mut vmlinux_header = File::create("src/bpf/vmlinux.h").unwrap();
    vmlinux_header
        .write_all(&bpftool_output.stdout)
        .expect("could not write vmlinux.h file");

    SkeletonBuilder::new()
        .source("src/bpf/test.bpf.c")
        .debug(true)
        .clang("/usr/bin/clang-13")
        .build_and_generate(&out)
        .unwrap();

    println!("cargo:rerun-if-changed={}", EBPF_SOURCE_FILE);
}
