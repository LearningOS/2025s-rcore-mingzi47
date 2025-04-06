//! Process management syscalls
use crate::{
    syscall::*,
    config::MAX_APP_NUM,
    sync::UPSafeCell,
    task::{
        exit_current_and_run_next, get_current_task_id, mod_current_task_mem_by_address, suspend_current_and_run_next, watch_current_task_mem_by_address
    },
    timer::get_time_us
};

use lazy_static::*;

const TRACE_REQUEST_WATCH_MEM: usize = 0;
const TRACE_REQUEST_MOD_MEM: usize = 1;
const TRACE_REQUEST_COUNT: usize = 2;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub struct TraceCountManager {
    inner: UPSafeCell<[AppTraceCount; MAX_APP_NUM]>,
}

#[derive(Clone, Copy)]
pub struct AppTraceCount {
    count: [(usize, usize); 5]
}

lazy_static! {
    /// Global variable: TRACE_COUNT_MANAGER
    pub static ref TRACE_COUNT_MANAGER: TraceCountManager = {
        let inner = [AppTraceCount{
            count: [
                (SYSCALL_WRITE, 0),
                (SYSCALL_EXIT, 0),
                (SYSCALL_YIELD, 0),
                (SYSCALL_GET_TIME, 0),
                (SYSCALL_TRACE, 0),
            ]
        }; MAX_APP_NUM];

        TraceCountManager {
            inner: unsafe {
                UPSafeCell::new(inner)
            },
        }
    };

}

impl TraceCountManager {
    // 为当前任务统计系统调用
    pub fn trace_count(&self, syscall_id: usize) {
        let current_id = get_current_task_id();
        let mut inner = self.inner.exclusive_access();

        if let Some(trace) = inner.get_mut(current_id) {
            if let Some((_, count)) = trace.count
                .iter_mut()
                .find(|e| e.0 == syscall_id)
            {
                *count += 1;
            }
        }
    }

    // 查看当前任务统计系统调用
    pub fn get_count(&self, syscall_id: usize) -> usize {
        let current_id = get_current_task_id();
        let inner = self.inner.exclusive_access();

        if let Some(trace) = inner.get(current_id) {
            trace.count
                .iter()
                .find(|e| e.0 == syscall_id)
                .unwrap()
                .1
        } else {
            0
        }
    }
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        TRACE_REQUEST_WATCH_MEM =>  {
            watch_current_task_mem_by_address(id) as isize
        },
        TRACE_REQUEST_MOD_MEM =>  {
            let data = data as u8;
            mod_current_task_mem_by_address(id, data);

            0
        },
        TRACE_REQUEST_COUNT =>  {
            TRACE_COUNT_MANAGER.get_count(id) as isize
        },
        _ => -1,
    }
}
