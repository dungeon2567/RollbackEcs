use crate::component::Component;
use crate::storage::block::Block;
use crate::storage::view::ViewMut;

pub struct Storage<T>
{
    pub root: Block<Box<Block<Box<Block<T>>>>>
}

impl<T> Storage<T> {
    pub fn new() -> Self {
        Storage {
            root: Block {
                presence_mask: 0,
                absence_mask: 0,
                change_mask: 0,
                data: std::array::from_fn(|_| std::mem::MaybeUninit::uninit())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::component::Component;
    use crate::storage::block::Block;

    struct Pos(u32);
    impl Component for Pos {}
    
}
