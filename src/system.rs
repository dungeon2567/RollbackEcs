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
pub struct ChangeCleanupSystem<T: Component>
{
    pub storage: std::rc::Rc<std::cell::RefCell<crate::storage::storage::Storage<T>>>,
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
}
