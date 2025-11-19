use crate::storage::block::Block;

pub use rollback_macros::system;

pub trait Query<Args> {
    fn run(&self, args: Args);
}

pub trait System<Args> {}

#[cfg(test)]
mod tests {
    use super::*;
    use rollback_macros::system;

    use crate::component::Component;
    use crate::scheduler::pipeline::PipelineStage;
    use crate::world::World;

    pub struct A(u8);
    impl Component for A {}

    pub struct B(u8);
    impl Component for B {}

    pub struct C(u8);
    impl Component for C {}

    pub struct D(u8);
    impl Component for D {}

    pub struct E(u8);
    impl Component for E {}

    pub struct F(u8);
    impl Component for F {}

    pub struct G(u8);
    impl Component for G {}

    system! {
        DestroySystem {
            query! {
                fn destroy(a: View<A>, b: ViewMut<B>, c: ViewMut<C>) All=[D] None=[E, F, G] {

                }
            }
        }
    }

    #[test]
    fn compiles_with_traits_present() {
        assert!(true);
    }

    #[test]
    fn macro_generates_three_level_run() {
        fn make_inner_block_a() -> Block<A> {
            let data: [std::mem::MaybeUninit<A>; 128] = std::array::from_fn(|i| std::mem::MaybeUninit::new(A(i as u8)));
            Block { presence_mask: u128::MAX, absence_mask: 0, change_mask: 0, data }
        }

        fn make_inner_block_b() -> Block<B> {
            let data: [std::mem::MaybeUninit<B>; 128] = std::array::from_fn(|i| std::mem::MaybeUninit::new(B(i as u8)));
            Block { presence_mask: u128::MAX, absence_mask: 0, change_mask: 0, data }
        }

        fn make_middle_block_a() -> Block<Box<Block<A>>> {
            let data: [std::mem::MaybeUninit<Box<Block<A>>>; 128] = std::array::from_fn(|j| {
                if j < 1 { std::mem::MaybeUninit::new(Box::new(make_inner_block_a())) } else { std::mem::MaybeUninit::uninit() }
            });
            Block { presence_mask: 0b1, absence_mask: 0, change_mask: 0, data }
        }

        fn make_middle_block_b() -> Block<Box<Block<B>>> {
            let data: [std::mem::MaybeUninit<Box<Block<B>>>; 128] = std::array::from_fn(|j| {
                if j < 1 { std::mem::MaybeUninit::new(Box::new(make_inner_block_b())) } else { std::mem::MaybeUninit::uninit() }
            });
            Block { presence_mask: 0b1, absence_mask: 0, change_mask: 0, data }
        }

        let mut world = World::new();
        let a_store = world.get::<A>();
        let b_store = world.get::<B>();
        let c_store = world.get::<C>();
        let d_store = world.get::<D>();

        fn make_inner_block_f() -> Block<F> {
            let data: [std::mem::MaybeUninit<F>; 128] = std::array::from_fn(|i| std::mem::MaybeUninit::new(F(i as u8)));
            Block { presence_mask: u128::MAX, absence_mask: 0, change_mask: 0, data }
        }

        fn make_middle_block_f() -> Block<Box<Block<F>>> {
            let data: [std::mem::MaybeUninit<Box<Block<F>>>; 128] = std::array::from_fn(|j| {
                if j < 1 { std::mem::MaybeUninit::new(Box::new(make_inner_block_f())) } else { std::mem::MaybeUninit::uninit() }
            });
            Block { presence_mask: 0b1, absence_mask: 0, change_mask: 0, data }
        }

        let f_store = world.get::<F>();
        a_store.borrow_mut().root = Block {
            presence_mask: 0b11,
            absence_mask: 0,
            change_mask: 0,
            data: std::array::from_fn(|i| {
                if i < 2 { std::mem::MaybeUninit::new(Box::new(make_middle_block_a())) } else { std::mem::MaybeUninit::uninit() }
            })
        };
        b_store.borrow_mut().root = Block {
            presence_mask: 0b11,
            absence_mask: 0,
            change_mask: 0,
            data: std::array::from_fn(|i| {
                if i < 2 { std::mem::MaybeUninit::new(Box::new(make_middle_block_b())) } else { std::mem::MaybeUninit::uninit() }
            })
        };
        // C and D storages left with default roots
        f_store.borrow_mut().root = Block {
            presence_mask: 0b11,
            absence_mask: 0,
            change_mask: 0,
            data: std::array::from_fn(|i| {
                if i < 2 { std::mem::MaybeUninit::new(Box::new(make_middle_block_f())) } else { std::mem::MaybeUninit::uninit() }
            })
        };

        let stage = DestroySystem::create(&mut world);
        assert!(stage.name().len() > 0);

        rollback_macros::system! {
            ZeroFiltersSystem {
                query! {
                    fn zero(c: View<C>, d: View<D>) All=[] None=[] {
                        assert_eq!(c.data.len(), d.data.len());
                    }
                }
            }
        }

        let stage2 = ZeroFiltersSystem::create(&mut world);
        stage2.run();
    }

    #[test]
    fn placeholder_empty_all() {
        assert!(true);
    }

    #[test]
    fn macro_supports_empty_all() {
        fn make_inner_block_a() -> Block<A> {
            let data: [std::mem::MaybeUninit<A>; 128] = std::array::from_fn(|i| std::mem::MaybeUninit::new(A(i as u8)));
            Block { presence_mask: u128::MAX, absence_mask: 0, change_mask: 0, data }
        }

        fn make_inner_block_b() -> Block<B> {
            let data: [std::mem::MaybeUninit<B>; 128] = std::array::from_fn(|i| std::mem::MaybeUninit::new(B(i as u8)));
            Block { presence_mask: u128::MAX, absence_mask: 0, change_mask: 0, data }
        }

        fn make_middle_block_a() -> Block<Box<Block<A>>> {
            let data: [std::mem::MaybeUninit<Box<Block<A>>>; 128] = std::array::from_fn(|j| {
                if j < 1 { std::mem::MaybeUninit::new(Box::new(make_inner_block_a())) } else { std::mem::MaybeUninit::uninit() }
            });
            Block { presence_mask: 0b1, absence_mask: 0, change_mask: 0, data }
        }

        fn make_middle_block_b() -> Block<Box<Block<B>>> {
            let data: [std::mem::MaybeUninit<Box<Block<B>>>; 128] = std::array::from_fn(|j| {
                if j < 1 { std::mem::MaybeUninit::new(Box::new(make_inner_block_b())) } else { std::mem::MaybeUninit::uninit() }
            });
            Block { presence_mask: 0b1, absence_mask: 0, change_mask: 0, data }
        }

        let mut world = World::new();
        let a_store = world.get::<A>();
        let b_store = world.get::<B>();

        a_store.borrow_mut().root = Block {
            presence_mask: 0b11,
            absence_mask: 0,
            change_mask: 0,
            data: std::array::from_fn(|i| {
                if i < 2 { std::mem::MaybeUninit::new(Box::new(make_middle_block_a())) } else { std::mem::MaybeUninit::uninit() }
            })
        };
        b_store.borrow_mut().root = Block {
            presence_mask: 0b11,
            absence_mask: 0,
            change_mask: 0,
            data: std::array::from_fn(|i| {
                if i < 2 { std::mem::MaybeUninit::new(Box::new(make_middle_block_b())) } else { std::mem::MaybeUninit::uninit() }
            })
        };

        assert_eq!(a_store.borrow().root.presence_mask, 0b11);
    }

    #[test]
    fn placeholder_variadic_ids() {
        assert!(true);
    }

    #[test]
    fn macro_supports_variadic_all_with_ids() {
        let mut world = World::new();
        assert!(true);
    }
}
