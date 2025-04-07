## 功能实现总结

功能较简单，固定了常数 MAX_SYSCALL_NUM 来表示系统调用的数量，并且在TCB中记录了当前任务系统调用计数的数组，通过数组下标访问。以上是改进后的思路，之前使用的代码手动填写的系统调用，不便于维护。改进之后的代码虽然比之前的可维护性更好，但是消耗的内存更大了。

## 问答题

### 1
RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0

#### ch2b_bad_address.rs

报错信息：[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.

访问 0x0 地址并修改其值，产生了硬件异常。

#### ch2b_bad_instructions.rs

报错信息：[kernel] IllegalInstruction in application, kernel killed it.

在 U 模式下使用了 S 模式返回 U 模式的特权指令 `sret`

#### ch2b_bad_register.rs

报错信息：[kernel] IllegalInstruction in application, kernel killed it.

在 U 模式下试图读取 S 模式的 `sstatus` 寄存器

### 2
#### 2.1

刚进入 `__restore` 时， `sp` 代表内核栈指针， 使用 `__restore` 的两种情况 : `__alltraps` 和  `__switch` 结束后。

#### 2.2

1. `sstatus` 中 `SPP` 等字段给出 Trap 发生之前 CPU 处在哪个特权级（S/U）等信息，执行 `sret` 指令时，CPU 会将当前的特权级按照 `sstatus` 的 `SPP` 字段设置为 U 或者 S ；

2. `sepc` 记录 Trap 发生之前执行的最后一条指令的地址， 执行 `sret` 指令时，CPU 会跳转到 `sepc` 寄存器指向的那条指令，然后继续执行；
3. `sscratch` 记录用户栈指针，用来恢复 `sp`。



#### 2.3

1. `x2`应该被保存的值是用户栈指针，在保存寄存器时，用户栈指针先保存在了 `sscratch`， 接着保存到了 `t2(x7)` 中，最后保存到内核栈上的 Trap 中的 `x[2]`；加载的时候先是从 Trap 中的 `x[2]` 加载到 `t2` , 在交换到 `sscratch`, 最后交换到 `sp`。

2. `x4` 从未被使用过。

#### 2.4

L60: `sp` 指向用户栈，`sscratch` 指向内核栈。

#### 2.5

最后一条指令 `sret`， 该指令表示 S 模式异常返回

#### 2.6

L13: `sp` 指向内核栈，`sscratch` 指向用户栈。

#### 2.7

`ecall` 指令


## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   >

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。


<!-- test -->
[RISC-V-book]: http://riscvbook.com/chinese/RISC-V-Reader-Chinese-v2p1.pdf