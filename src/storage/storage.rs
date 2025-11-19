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
            root: Block::new()
        }
    }

    pub fn len(&self) -> usize {
        let root = &self.root;
        let mut count = 0;

        // 1. Count fully occupied middle blocks
        count += root.absence_mask.count_ones() as usize * 16384;

        // 2. Iterate partially occupied middle blocks
        let mut partial_root = root.presence_mask & !root.absence_mask;
        while partial_root != 0 {
            let ri = partial_root.trailing_zeros();
            let middle = unsafe { root.data[ri as usize].assume_init_ref() };

            // 2a. Count fully occupied inner blocks
            count += middle.absence_mask.count_ones() as usize * 128;

            // 2b. Iterate partially occupied inner blocks
            let mut partial_middle = middle.presence_mask & !middle.absence_mask;
            while partial_middle != 0 {
                let mi = partial_middle.trailing_zeros();
                let inner = unsafe { middle.data[mi as usize].assume_init_ref() };

                // 2c. Count items in inner block
                count += inner.absence_mask.count_ones() as usize;

                partial_middle &= !(1 << mi);
            }

            partial_root &= !(1 << ri);
        }

        count
    }

    pub fn set(&mut self, index: u32, value: &T) 
    where T: Clone
    {
        // Decode global index to ri, mi, ii
        // ri (0..128) * 16384 + mi (0..128) * 128 + ii (0..128)
        let ri = index / 16384;
        let mi = (index % 16384) / 128;
        let ii = index % 128;

        // Validate index is in bounds
        if ri >= 128 {
            panic!("Index out of bounds: {}", index);
        }

        let root = &mut self.root;

        // Ensure middle block exists
        root.ensure_child_exists(ri);

        let middle = unsafe { root.data[ri as usize].assume_init_mut() };

        // Ensure inner block exists
        if (middle.presence_mask >> mi) & 1 == 0 {
            // Create new inner block
            let new_inner = Block::new();
            middle.data[mi as usize].write(Box::new(new_inner));
            middle.presence_mask |= 1 << mi;
        }

        let inner = unsafe { middle.data[mi as usize].assume_init_mut() };

        // Set the value
        let slot = unsafe { inner.data[ii as usize].assume_init_mut() };
        *slot = value.clone();
        
        // Update presence and absence masks
        inner.presence_mask |= 1 << ii;
        inner.absence_mask |= 1 << ii;
    }
}

use crate::entity::Entity;

impl Storage<Entity> {
    pub fn create(&mut self) -> &Entity {
        let root = &mut self.root;
        
        // 1. Find free slot in root
        let free_root = !root.absence_mask;
        if free_root == 0 {
            panic!("Storage is full");
        }
        let ri = free_root.trailing_zeros();
        
        root.ensure_child_exists(ri);
        
        let mut inner_full = false;
        let mut middle_full = false;
        let mi;
        let ii;

        {
            let middle = unsafe { root.data[ri as usize].assume_init_mut() };

            // 2. Find free slot in middle
            let free_middle = !middle.absence_mask;
            if free_middle == 0 {
                 panic!("Storage inconsistency: Root said free, Middle is full");
            }
            mi = free_middle.trailing_zeros();

            // Ensure inner block exists
            if (middle.presence_mask >> mi) & 1 == 0 {
                let mut new_inner = Block::new();
                
                // Pre-initialize all entities in this new chunk
                for i in 0..128 {
                    let global_index = ri * 16384 + mi * 128 + i;
                    new_inner.data[i as usize].write(Entity::new(global_index, 0));
                }
                // All slots are present (initialized)
                new_inner.presence_mask = u128::MAX;
                // All slots are free (absent from "occupied" set)
                new_inner.absence_mask = 0;
                
                middle.data[mi as usize].write(Box::new(new_inner));
                middle.presence_mask |= 1 << mi;
            }
            
            {
                let inner = unsafe { middle.data[mi as usize].assume_init_mut() };

                // 3. Find free slot in inner
                let free_inner = !inner.absence_mask;
                if free_inner == 0 {
                     panic!("Storage inconsistency: Middle said free, Inner is full");
                }
                ii = free_inner.trailing_zeros();

                // Always increment generation for the allocated entity
                let entity = unsafe { inner.data[ii as usize].assume_init_mut() };
                entity.increment_generation();
                
                // Mark as occupied
                inner.absence_mask |= 1 << ii;
                
                if inner.absence_mask == u128::MAX {
                    inner_full = true;
                }
            }
            
            if inner_full {
                middle.absence_mask |= 1 << mi;
                if middle.absence_mask == u128::MAX {
                    middle_full = true;
                }
            }
        }
        
        if middle_full {
            root.absence_mask |= 1 << ri;
        }

        // Re-traverse to return the reference.
        unsafe {
            let middle = root.data[ri as usize].assume_init_mut();
            let inner = middle.data[mi as usize].assume_init_mut();
            inner.data[ii as usize].assume_init_ref()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::storage::block::Block;

    #[test]
    fn test_create() {
        let mut storage = Storage::<Entity>::new();

        // Create 128 items (fill one inner block)
        for i in 0..128 {
            let e = storage.create();
            assert_eq!(e.index(), i);
            assert_eq!(e.generation(), 1);
        }

        {
            // Verify root masks
            let root = &storage.root;
            // First bit of presence should be 1 (first middle block exists)
            assert_eq!(root.presence_mask, 1);
            // Absence mask should be 0 (first middle block is not full yet)
            assert_eq!(root.absence_mask, 0);

            // Verify middle block
            let middle = unsafe { root.data[0].assume_init_ref() };
            // First bit of presence should be 1 (first inner block exists)
            assert_eq!(middle.presence_mask, 1);
            // First bit of absence should be 1 (first inner block is full)
            assert_eq!(middle.absence_mask, 1);

            // Verify inner block
            let inner = unsafe { middle.data[0].assume_init_ref() };
            assert_eq!(inner.presence_mask, u128::MAX);
            assert_eq!(inner.absence_mask, u128::MAX);
        }

        // Fill the rest of the first middle block (128 * 128 = 16384 items total)
        // We already inserted 128 items (0..128).
        // We need to insert 127 more inner blocks.
        for i in 128..16384 {
            let e = storage.create();
            assert_eq!(e.index(), i);
            assert_eq!(e.generation(), 1);
        }

        {
            let root = &storage.root;
            // Verify root masks again
            // First bit of presence should be 1
            assert_eq!(root.presence_mask, 1);
            // First bit of absence should be 1 (first middle block is now full)
            assert_eq!(root.absence_mask, 1);

            // Verify middle block
            let middle = unsafe { root.data[0].assume_init_ref() };
            assert_eq!(middle.presence_mask, u128::MAX); // All inner blocks present
            assert_eq!(middle.absence_mask, u128::MAX); // All inner blocks full
        }
    }

    #[test]
    fn test_len() {
        let mut storage = Storage::<Entity>::new();

        assert_eq!(storage.len(), 0);

        for _ in 0..10 {
            storage.create();
        }
        assert_eq!(storage.len(), 10);

        // Fill one inner block (128 items)
        for _ in 10..128 {
            storage.create();
        }
        assert_eq!(storage.len(), 128);

        // Add one more to start next inner block
        storage.create();
        assert_eq!(storage.len(), 129);

        // Fill a whole middle block (128 * 128 = 16384 items)
        // We already have 129 items.
        for _ in 129..16384 {
            storage.create();
        }
        assert_eq!(storage.len(), 16384);

        // Add one more to start next middle block
        storage.create();
        assert_eq!(storage.len(), 16385);
    }

    #[test]
    fn test_entity_generation() {
        let mut storage = Storage::<Entity>::new();

        // Create first entity
        let e1 = *storage.create();
        assert_eq!(e1.index(), 0);
        assert_eq!(e1.generation(), 1); // Initialized to 0, incremented to 1

        // Create second entity
        let e2 = *storage.create();
        assert_eq!(e2.index(), 1);
        assert_eq!(e2.generation(), 1); // Initialized to 0, incremented to 1

        // "Delete" first entity by clearing absence mask bit
        // We need to manually access the inner block to do this for testing
        {
            let root = &mut storage.root;
            let middle = unsafe { root.data[0].assume_init_mut() };
            let inner = unsafe { middle.data[0].assume_init_mut() };
            inner.absence_mask &= !1; // Clear bit 0
        }

        // Create again, should reuse slot 0 and increment generation
        let e3 = *storage.create();
        assert_eq!(e3.index(), 0);
        assert_eq!(e3.generation(), 2); // 1 -> 2
    }
}
