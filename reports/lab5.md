## 功能实现总结

check： 安全性检查
alloc_res: 银行家算法进行资源分配的步骤
dealloc_res: 银行家算法进行资源回收的步骤

sys_lock 中的顺序:

    check -> mutex.lock() -> alloc_res

sys_unlock 中的顺序:

    dealloc_res -> mutex.unlock()

在拿到锁的情况下才会修改数据（银行家算法中的资源也是共享资源）。


## 问答

### 1 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

需要回收的资源：
1. 分配的文件描述符
2. 分配的内存
3. 子进程的父进程更改为init
4. Manager 就绪队列中的 TCB
5. set of timer condvars 中的 TCB

其他线程的 TaskControlBlock 可能在哪些位置被引用
1. set of timer condvars, 需要回收
2. Manager 就绪队列中的 TCB， 需要回收
3. PCB中的task队列， 与PCB一起回收
4. mutex，semaphore 的等待队列中，与PCB一起回收



### 2 对比以下两种 Mutex 中的实现，二者有什么区别？这些区别可能会导致什么问题？

```rust
impl Mutex for Mutex1 {
    fn lock(&self) {
        loop {
            let mut mutex_inner = self.inner.exclusive_access();
            if mutex_inner.locked {
                mutex_inner.wait_queue.push_back(current_task().unwrap());
                drop(mutex_inner);
                block_current_and_run_next();
            } else {
                mutex_inner.locked = true;
                break;
            }
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.locked = false;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        }
    }
}

impl Mutex for Mutex2 {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
```

Mutex1 在释放锁后，从等待队列中出来的任务需要重新与其他任务竞争锁；而 Mutex2 在释放锁时，如果等待队列不为空，唤醒一个任务并将锁直接交给他。

Mutex1 可能会导致某个线程每次竞争都失败，导致永远都拿不到锁，不满足 **有界等待** 的指标。



## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
