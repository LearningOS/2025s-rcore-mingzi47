//! Process management syscalls
use core::{mem, u64::MAX};

use crate::{mm::{translated_byte_and_check, translated_byte_buffer, PTEFlags}, task::{ change_program_brk, current_user_token, exit_current_and_run_next, get_current_task_syscall_count, mmap, munmap, suspend_current_and_run_next}, timer::get_time_us};

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
            debug!("sys_trace, request = {}, id = {}", trace_request, id);
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

/// 基于Lazy 策略的 mmap, 支持超过物理内存的虚拟内存
/// 设计思路：
///     - 单独使用一个容器存放mmap申请的虚拟内存, 与其他内存映射做区分，方便判断逻辑的实现;
///     - 容器结构为: `BTreeMap<VirtPageNum, (Option<FrameTracker>, PTEFlags)>`;
///     - 申请内存时，不会为虚拟内存映射物理内存，因此需要用 **空** 来表示当前的物理内存，因此使用 `Option` 包裹，初始为 `None`, 实际映射物理内存后才有实际的值;
///     - 同时，也不会将虚拟页码插入到 `page_table` 当中，因此 `PTEFlags` 也需要记录。
///     - 在`trap_handler` 的缺页异常处理中调用 `lazy_mmap` 来实际映射物理内存;
/// 需要注意：只有在 U 态可以正常触发缺页异常，因此为了在 S 态正常使用 mmap
/// 申请的内存，还需有额外实现函数。这里使用了`translate_with_mmap` 。
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if port == 0 {
        return -1;
    }

    // 除了第三位，其余必须为 0
    if (port & (MAX - 0b111) as usize) != 0 {
        return -1;
    }

    mmap(start, len, port)
}

/// 卸载 mmap 映射的内存
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap!");

    munmap(start, len)
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
