use std::any::Any;
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::cell::RefCell;
use crate::component::Component;
use crate::scheduler::pipeline::PipelineStage;
use crate::storage::storage::{Storage};

pub struct World {
    pub storages: [MaybeUninit<Box<dyn Any>>; 128],
    pub mask: u128
}

impl World {
    pub fn new() -> Self {
        World {
            storages: std::array::from_fn(|_| MaybeUninit::uninit()),
            mask: 0,
        }
    }
    
    pub fn get<T: Component>(&mut self) -> Rc<RefCell<Storage<T>>> {
        let id = T::type_index();

        if id >= 128 { panic!("invalid component type index") }

        let bit = 1u128 << id;

        if (self.mask & bit) == 0 {
            let rc = Rc::new(RefCell::new(Storage::<T>::new()));
            self.storages[id] = MaybeUninit::new(Box::new(rc.clone()) as Box<dyn Any>);
            self.mask |= bit;
            return rc;
        }

        let any = unsafe { self.storages[id].assume_init_ref() };

        unsafe {
            let raw = any.as_ref() as *const dyn Any as *const Rc<RefCell<Storage<T>>>;

            (*raw).clone()
        }
    }

    pub fn run<T: PipelineStage>(&mut self) {
        T::create(self).run();
    }

    pub fn schedule<T: PipelineStage>(&mut self) {
        T::create(self).run();
    }
}
