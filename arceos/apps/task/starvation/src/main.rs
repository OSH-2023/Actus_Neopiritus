#![no_std]
#![no_main]

#[macro_use]
extern crate libax;
extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use libax::thread::{self, yield_now};

struct TaskParam {
    data_len: usize,
    value: u64,
    nice: isize,
}

const TASK_PARAMS: &[TaskParam] = &[
    // four short tasks
    TaskParam {
        data_len: 2000_000,
        value: 1000,
        nice: -12,
    },
    TaskParam {
        data_len: 2000_000,
        value: 1000,
        nice: -11,
    },
    TaskParam {
        data_len: 2000_000,
        value: 1000,
        nice: -11,
    },
    TaskParam {
        data_len: 2000_000,
        value: 1000,
        nice: -11,
    },
    // one long task
    TaskParam {
        data_len: 1,
        value: 1000_000_000,
        nice: -20,
    },
];

const PAYLOAD_KIND: usize = 5;

fn load(n: &u64) -> u64 {
    // time consuming is linear with *n
    let mut sum: u64 = *n;
    for i in 0..*n {
        sum += ((i ^ (i * 3)) ^ (i + *n)) / (i + 1);
    }
    yield_now();
    sum
}

#[no_mangle]
fn main() {
    thread::set_priority(-20);
    let data = (0..PAYLOAD_KIND)
        .map(|i| Arc::new(vec![TASK_PARAMS[i].value; TASK_PARAMS[i].data_len]))
        .collect::<Vec<_>>();
    let mut expect: u64 = 0;
    for data_inner in &data {
        expect += data_inner.iter().map(load).sum::<u64>();
    }

    let mut tasks = Vec::with_capacity(PAYLOAD_KIND);
    let start_time = libax::time::Instant::now();
    for i in 0..PAYLOAD_KIND {
        let vec = data[i].clone();
        let data_len = TASK_PARAMS[i].data_len;
        let nice = TASK_PARAMS[i].nice;
        tasks.push(thread::spawn(move || {
            let left = 0;
            let right = data_len;
            thread::set_priority(nice);
            println!(
                "part {}: {:?} [{}, {})",
                i,
                thread::current().id(),
                left,
                right
            );

            let partial_sum: u64 = vec[left..right].iter().map(load).sum();
            let leave_time = start_time.elapsed().as_millis() as u64;

            println!("part {}: {:?} finished", i, thread::current().id());
            (partial_sum, leave_time)
        }));
    }

    let (results, leave_times): (Vec<_>, Vec<_>) =
        tasks.into_iter().map(|t| t.join().unwrap()).unzip();
    let actual = results.iter().sum();

    println!("sum = {}", actual);
    println!("leave time:");
    for (i, time) in leave_times.iter().enumerate() {
        println!("task {} = {}ms", i, time);
    }

    assert_eq!(expect, actual);

    println!("Realtime tests run OK!");
}