use rollback_macros::system as system;

// Query trait not used in inlined macro run

use crate::component::Destroyed;
use crate::entity::Entity;


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
            fn print(e: View<Entity>) {
                println!("{:?}", e);
            }
        }
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

        // Entities should still be there
        assert_eq!(ents.borrow().len(), 3);
    }
}
