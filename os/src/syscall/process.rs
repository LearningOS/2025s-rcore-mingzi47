//! Process management syscalls
use core::mem;

use crate::{mm::{translated_byte_and_check, translated_byte_buffer, PTEFlags}, task::{change_program_brk, current_user_token, exit_current_and_run_next, get_current_task_syscall_count, suspend_current_and_run_next}, timer::get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

const TRACE_READ: usize = 0;
const TRACE_WRITE: usize = 1;
const TRACE_COUNT: usize = 2;

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let token = current_user_token();
    let ptr = ts as *const u8;
    let len = mem::size_of::<TimeVal>();

    // 获取应用空间内存
    let buffers = translated_byte_buffer(token, ptr, len);
    let us = get_time_us();

    // 将要写入的数据转换为字节流，方便写入
    let tmp = TimeVal {
        sec : us / 1_000_000,
        usec : us % 1_000_000,
    };
    // https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
    let tmp_data = unsafe {
        core::slice::from_raw_parts(
            (&tmp as *const TimeVal) as *const u8,
            len,
        )
    };

    // 写入应用空间内存
    let mut tmp_i = 0;
    for buffer in buffers {
        for b in buffer {
            *b = tmp_data[tmp_i];
            tmp_i+=1;
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        TRACE_READ => {
            if let Some(byte) = translated_byte_and_check(
                current_user_token(),
                id as *const u8,
                PTEFlags::R | PTEFlags::U,
            ) {
                let v = *byte;
                v as isize
            } else {
                -1
            }
        },
        TRACE_WRITE => {
            if let Some(byte) = translated_byte_and_check(
                current_user_token(),
                id as *const u8,
                PTEFlags::W | PTEFlags::U,
            ) {
                *byte = data as u8;
                0
            } else {
                -1
            }
        },
        TRACE_COUNT => {
            get_current_task_syscall_count(id) as isize
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
