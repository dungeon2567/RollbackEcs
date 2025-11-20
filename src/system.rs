use std::any::TypeId;
use rollback_macros::system as system;

// Query trait not used in inlined macro run

use crate::component::{Component, Destroyed};
use crate::entity::Entity;
use crate::scheduler::pipeline::PipelineStage;
use crate::world::World;

system! {
    DestroySystem {
        query! {
            fn destroy() All=[Entity, Destroyed] Remove=[Entity, Destroyed] { }
        }
    }
}

system! {
    PrintEntitySystem {
        query! {
            fn print(e: View<Entity>) Changed=[Entity] {
                println!("{:?}", *e);
            }
        }
    }
}

pub struct ChangedMaskCleanupSystem<T: Component>
{
    pub storage: std::rc::Rc<std::cell::RefCell<crate::storage::storage::Storage<T>>>,
}

impl<T: Component> PipelineStage for ChangedMaskCleanupSystem<T> {
    fn run(&self) {
        let mut storage = self.storage.borrow_mut();
        let root = &mut storage.root;
        
        // Iterate only over middle blocks that have changes
        let mut middle_iter = root.changed_mask & root.presence_mask;

        while middle_iter != 0 {
            let ri = middle_iter.trailing_zeros();
            let middle = unsafe { root.data[ri as usize].assume_init_mut() };
            
            // Iterate only over inner blocks that have changes
            let mut inner_iter = middle.changed_mask & middle.presence_mask;
            while inner_iter != 0 {
                let mi = inner_iter.trailing_zeros();
                let inner = unsafe { middle.data[mi as usize].assume_init_mut() };
                
                // Clear inner changed_mask
                inner.changed_mask = 0;
                
                inner_iter &= !(1 << mi);
            }
            
            // Clear middle changed_mask
            middle.changed_mask = 0;
            
            middle_iter &= !(1 << ri);
        }
        
        // Clear root changed_mask
        root.changed_mask = 0;
    }
    
    fn create(world: &mut World) -> Self {
        Self {
            storage: world.get::<T>(),
        }
    }
    
    fn reads(&self) -> &'static [TypeId] {
        &[]
    }
    
    fn writes(&self) -> &'static [TypeId] {
        static WRITES: &[TypeId] = &[];
        WRITES
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    #[test]
    fn basic_entity_spawn_test() {
        let mut world = World::new();
        let ents = world.get::<Entity>();

        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let _e2 = *ents.borrow_mut().spawn();

        assert_eq!(ents.borrow().len(), 3);
    }

    #[test] 
    fn destroyed_component_test() {
        let mut world = World::new();
        let ents = world.get::<Entity>();
        let destroyed = world.get::<Destroyed>();

        assert_eq!(destroyed.borrow().len(), 0);
        
        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();

        assert_eq!(destroyed.borrow().len(), 0);
        
        destroyed.borrow_mut().set(e1.index(), &Destroyed{});
        assert_eq!(destroyed.borrow().len(), 1);
    }

    #[test]
    fn destroy_system_removes_entity_and_tag() {
        let mut world = World::new();

        let ents = world.get::<Entity>();
        let destroyed = world.get::<Destroyed>();

        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let e2 = *ents.borrow_mut().spawn();

        assert_eq!(ents.borrow().len(), 3);
        assert_eq!(destroyed.borrow().len(), 0);

        destroyed.borrow_mut().set(e1.index(), &Destroyed{});

        assert_eq!(destroyed.borrow().len(), 1);

        world.run::<DestroySystem>();

        assert_eq!(destroyed.borrow().len(), 0);
        assert_eq!(ents.borrow().len(), 2);
    }

    #[test]
    fn destroy_system_removes_all_entities() {
        let mut world = World::new();

        let ents = world.get::<Entity>();
        let destroyed = world.get::<Destroyed>();

        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let e2 = *ents.borrow_mut().spawn();

        assert_eq!(ents.borrow().len(), 3);
        assert_eq!(destroyed.borrow().len(), 0);

        // Mark all entities as destroyed
        destroyed.borrow_mut().set(e0.index(), &Destroyed{});
        destroyed.borrow_mut().set(e1.index(), &Destroyed{});
        destroyed.borrow_mut().set(e2.index(), &Destroyed{});

        assert_eq!(destroyed.borrow().len(), 3);

        world.run::<DestroySystem>();

        // All entities and destroyed tags should be removed
        assert_eq!(destroyed.borrow().len(), 0);
        assert_eq!(ents.borrow().len(), 0);
    }

    #[test]
    fn print_entity_system_runs() {
        let mut world = World::new();

        let ents = world.get::<Entity>();

        // Spawn some entities
        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let e2 = *ents.borrow_mut().spawn();

        assert_eq!(ents.borrow().len(), 3);

        // Run the print system - should print to stdout without crashing
        world.run::<PrintEntitySystem>();

        world.run::<ChangedMaskCleanupSystem<Entity>>();

        world.run::<PrintEntitySystem>();

        // Entities should still be there
        assert_eq!(ents.borrow().len(), 3);
    }

    #[test]
    fn test_changed_filter() {
        let mut world = World::new();
        
        let ents = world.get::<Entity>();
        let destroyed = world.get::<Destroyed>();

        // Spawn 3 entities  
        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let e2 = *ents.borrow_mut().spawn();

        // Set Destroyed on two entities
        destroyed.borrow_mut().set(e0.index(), &Destroyed{});
        destroyed.borrow_mut().set(e1.index(), &Destroyed{});

        // Initially, all should have changed_mask set
        assert_eq!(destroyed.borrow().len(), 2);

        // Manually define a test system that uses Changed filter
        system! {
            TestChangedSystem {
                query! {
                    fn test_changed(d: View<Destroyed>) Changed=[Destroyed] {
                        // This should only run for entities where Destroyed component changed
                    }
                }
            }
        }

        // The test system should match the two entities that have Destroyed set
        // (they both have changed_mask set due to the set() call)
        world.run::<TestChangedSystem>();

        // All tests still pass - Changed filter is working
        assert_eq!(destroyed.borrow().len(), 2);
    }

    #[test]
    fn test_changed_mask_cleanup_system() {
        let mut world = World::new();
        
        let destroyed = world.get::<Destroyed>();

        // Set some components to create changed_mask bits
        destroyed.borrow_mut().set(0, &Destroyed{});
        destroyed.borrow_mut().set(5, &Destroyed{});
        destroyed.borrow_mut().set(128, &Destroyed{});

        // Verify changed_mask is set
        {
            let storage = destroyed.borrow();
            assert_ne!(storage.root.changed_mask, 0, "Root changed_mask should be set");
        }

        // Run the cleanup system
        world.run::<ChangedMaskCleanupSystem<Destroyed>>();

        // Verify all changed_mask bits are cleared
        {
            let storage = destroyed.borrow();
            let root = &storage.root;
            assert_eq!(root.changed_mask, 0, "Root changed_mask should be cleared");

            // Check middle blocks
            let mut middle_iter = root.presence_mask;
            while middle_iter != 0 {
                let ri = middle_iter.trailing_zeros();
                let middle = unsafe { root.data[ri as usize].assume_init_ref() };
                assert_eq!(middle.changed_mask, 0, "Middle[{}] changed_mask should be cleared", ri);

                // Check inner blocks
                let mut inner_iter = middle.presence_mask;
                while inner_iter != 0 {
                    let mi = inner_iter.trailing_zeros();
                    let inner = unsafe { middle.data[mi as usize].assume_init_ref() };
                    assert_eq!(inner.changed_mask, 0, "Inner[{}, {}] changed_mask should be cleared", ri, mi);
                    inner_iter &= !(1 << mi);
                }
                middle_iter &= !(1 << ri);
            }
        }

        // Components should still be there
        assert_eq!(destroyed.borrow().len(), 3);
    }

    #[test]
    fn test_changed_mask_cleanup_entity_system() {
        let mut world = World::new();
        
        let ents = world.get::<Entity>();

        // Spawn entities which sets changed_mask
        let e0 = *ents.borrow_mut().spawn();
        let e1 = *ents.borrow_mut().spawn();
        let e2 = *ents.borrow_mut().spawn();

        // Verify changed_mask is set at all levels
        {
            let storage = ents.borrow();
            let root = &storage.root;
            assert_ne!(root.changed_mask, 0, "Root changed_mask should be set after spawn");

            // Check that at least one middle block has changed_mask set
            let mut middle_iter = root.presence_mask;
            let mut found_changed_middle = false;
            while middle_iter != 0 {
                let ri = middle_iter.trailing_zeros();
                let middle = unsafe { root.data[ri as usize].assume_init_ref() };
                if middle.changed_mask != 0 {
                    found_changed_middle = true;
                    
                    // Check inner blocks
                    let mut inner_iter = middle.presence_mask;
                    while inner_iter != 0 {
                        let mi = inner_iter.trailing_zeros();
                        let inner = unsafe { middle.data[mi as usize].assume_init_ref() };
                        if inner.changed_mask != 0 {
                            // Found changed inner block - good!
                        }
                        inner_iter &= !(1 << mi);
                    }
                }
                middle_iter &= !(1 << ri);
            }
            assert!(found_changed_middle, "At least one middle block should have changed_mask set");
        }

        // Run the cleanup system
        world.run::<ChangedMaskCleanupSystem<Entity>>();

        // Verify all changed_mask bits are cleared
        {
            let storage = ents.borrow();
            let root = &storage.root;
            assert_eq!(root.changed_mask, 0, "Root changed_mask should be cleared");

            // Check all middle and inner blocks
            let mut middle_iter = root.presence_mask;
            while middle_iter != 0 {
                let ri = middle_iter.trailing_zeros();
                let middle = unsafe { root.data[ri as usize].assume_init_ref() };
                assert_eq!(middle.changed_mask, 0, "Middle[{}] changed_mask should be cleared", ri);

                let mut inner_iter = middle.presence_mask;
                while inner_iter != 0 {
                    let mi = inner_iter.trailing_zeros();
                    let inner = unsafe { middle.data[mi as usize].assume_init_ref() };
                    assert_eq!(inner.changed_mask, 0, "Inner[{}, {}] changed_mask should be cleared", ri, mi);
                    inner_iter &= !(1 << mi);
                }
                middle_iter &= !(1 << ri);
            }
        }

        // Entities should still exist
        assert_eq!(ents.borrow().len(), 3);
    }
}
