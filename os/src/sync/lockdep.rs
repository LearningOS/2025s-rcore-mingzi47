use alloc::vec::Vec;

use super::UPSafeCell;

const DEADLOCK: isize = -0xdead;

///
pub trait LockDep: Send + Sync {
    /// 初始化一个资源
    fn init(&self, id: usize, num: usize);
    /// 安全性检查，失败返回 -0xdead， 否则返回0
    /// 检查要在调用lock前
    fn check(&self, tid: usize, id: usize, num: usize) -> isize;
    /// 分配资源给 tid
    /// WARN: 分配资源要在调用lock后，拿到锁后才要分配资源!!!
    fn res_alloc(&self, tid: usize, id: usize, num: usize);
    /// 回收资源 tid
    fn res_dealloc(&self, tid: usize, id: usize, num: usize);
} 


/// 银行家算法
pub struct LockDepBanker{
    /// 
    inner: UPSafeCell<LockDepBankerInner>,
}

pub struct LockDepBankerInner {
    // 可用资源的数量
    available: Vec<usize>,
    //
    tasks: Vec<LockDepBankerTask>,
}

///
#[derive(Debug)]
pub struct LockDepBankerTask {
    allocation: Vec<usize>,
    max: Vec<usize>,
}


impl LockDepBanker {
    /// create a Banker algo
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(LockDepBankerInner{
                    available: Vec::new(),
                    tasks: Vec::new(),
                })
            }
        }
    }

    fn resize_available(&self, new_len: usize, value: usize) {
        let mut inner = self.inner.exclusive_access();
        if new_len <= inner.available.len() {
            return;
        }

        inner.available.resize(new_len, value);
    }

    fn resize_tasks(&self, new_len: usize) {
        let mut inner = self.inner.exclusive_access();
        if new_len > inner.tasks.len() {
            inner.tasks.resize_with(new_len, || {
                LockDepBankerTask{
                    allocation: Vec::new(),
                    max: Vec::new(),
                }
            });
        }

        let res_num = inner.available.len();
        inner.tasks
            .iter_mut()
            .for_each(|t| {
                if res_num > t.allocation.len() {
                    t.allocation.resize(res_num, 0);
                    t.max.resize(res_num, 0);
                }
            });

    }
}

impl LockDep for LockDepBanker  {
    /// 
    fn init(&self, id: usize, num: usize) {
        self.resize_available(id + 1, num);

        let mut inner = self.inner.exclusive_access();
        inner.available[id] = num;
    }
    /// 
    fn check(&self, tid: usize, id: usize, num: usize) -> isize {
        self.resize_tasks(tid + 1);
        let mut inner = self.inner.exclusive_access();

        // 最大需求增加
        inner.tasks[tid].max[id] += num;

        let mut work = inner.available.clone();
        let task_num = inner.tasks.len();
        let mut finish: Vec<bool> = Vec::new();
        finish.resize(task_num, false);

        let mut count = 0;

        while count < task_num {
            let mut found = false;
            for (i, t) in inner.tasks.iter().enumerate() {
                if finish[i] {
                    continue;
                }

                if work
                    .iter()
                    .enumerate()
                    .all(|(j, w)| {
                        *w >= (t.max[j] - t.allocation[j])
                    })
                {
                    work
                        .iter_mut()
                        .enumerate()
                        .for_each(|(j, w)| {
                            *w += t.allocation[j];
                        });
                    finish[i] = true;
                    found = true;
                    count+=1;
                }
            }

            if !found {
                return DEADLOCK;
            }
        }

        0
    }
    fn res_alloc(&self, tid: usize, id: usize, num: usize) {
        let mut inner = self.inner.exclusive_access();

        inner.tasks[tid].allocation[id] += num;
        inner.available[id] -= num;
    }
    /// 
    fn res_dealloc(&self, tid: usize, id: usize, num: usize) {
        let mut inner = self.inner.exclusive_access();

        inner.tasks[tid].max[id] -= num;
        inner.tasks[tid].allocation[id] -= num;
        inner.available[id] += num;
    }
}
