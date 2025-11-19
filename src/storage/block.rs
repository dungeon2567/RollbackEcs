use std::mem::MaybeUninit;

pub struct Block<T> {
    pub presence_mask: u128,
    pub absence_mask: u128,
    pub change_mask: u128,
    pub data: [MaybeUninit<T>; 128]
}

impl<T> Drop for Block<T> {
    fn drop(&mut self) {
        let mut m = self.presence_mask;

        unsafe {
            let ptr = self.data.as_mut_ptr();

            while m != 0 {
                let start = m.trailing_zeros();

                let run = (m >> start).trailing_ones();

                for i in 0..run {
                    ptr.add((start + i) as usize).read().assume_init_drop();
                }

                let range_mask = if run == 128 { u128::MAX } else { ((1u128 << run) - 1) << start };

                m &= !range_mask;
            }
        }
    }
}
