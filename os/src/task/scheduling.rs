/// Stride Scheduling
/// 
const BIG_STRIDE: usize = 1000;
const DEFAULT_STRIDE: usize = 16;


#[derive(Debug)]
pub struct Stride {
    stride: usize,
    pass: usize,
}

impl Stride {
    pub fn new() -> Self {
        Self {
            stride: calc_stride(DEFAULT_STRIDE),
            pass: 0,
        } 
    }

    /// 设置步长
    /// 成功返回步长 stride
    /// 优先级 prio < 2， 设置失败，返回 -1
    pub fn set_stride(&mut self, prio: isize) -> isize {
        if prio < 2 {
            -1
        } else {
            self.stride = calc_stride(prio as usize);

            self.stride as isize
        }
    }

    /// 增加行程
    pub fn add_pass(&mut self) -> usize {
        let stride = self.stride;
        self.pass += stride;

        self.pass
    }
}

impl Ord for Stride {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.pass.cmp(&other.pass)
    }
}

impl Eq for Stride {
}

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.pass == other.pass
    }
}

/// 根据优先级计算步长
fn calc_stride(prio: usize) -> usize {
    BIG_STRIDE / prio
}
