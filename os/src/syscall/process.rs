//! Process management syscalls
use core::{mem, u64::MAX};

use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_byte_buffer, translated_str },
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );
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


/// 基于Lazy 策略的 mmap, 支持超过物理内存的虚拟内存
/// 设计思路：
///     - 单独使用一个容器存放mmap申请的虚拟内存, 与其他内存映射做区分，方便判断逻辑的实现;
///     - 容器结构为: `BTreeMap<VirtPageNum, (Option<FrameTracker>, PTEFlags)>`;
///     - 申请内存时，不会为虚拟内存映射物理内存，因此需要用 **空** 来表示当前的物理内存，因此使用 `Option` 包裹，初始为 `None`, 实际映射物理内存后才有实际的值;
///     - 同时，也不会将虚拟页码插入到 `page_table` 当中，因此 `PTEFlags` 也需要记录。
///     - 在`trap_handler` 的缺页异常处理中调用 `lazy_mmap` 来实际映射物理内存;
/// 需要注意：只有在 U 态可以正常触发缺页异常，因此为了在 S 态正常使用 mmap
/// 申请的内存，还需有额外实现函数。这里使用了`translate_with_mmap` 。
///
/// 申请一段空间，从 start 开始，长度位 len 
/// 区间内存在被申请过的地址，失败返回 -1
/// start 没有页对齐，失败返回 -1
/// port 不合法, 失败返回 -1
/// 成功返回 0
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if port == 0 {
        return -1;
    }

    // 除了第三位，其余必须为 0
    if (port & (MAX - 0b111) as usize) != 0 {
        return -1;
    }

    current_task().unwrap().mmap(start, len, port)
}

/// 释放 mmap 申请的空间, 从 start 开始，长度位 len
/// 区间内存存在未被 mmap 申请过的地址，失败返回 -1
/// 成功返回 0
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    current_task().unwrap().munmap(start, len)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        let new_task = task.spawn(data);

        let new_pid = new_task.getpid();
        // ! add task queue
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );

    current_task()
        .unwrap()
        .set_priority(prio)
}
