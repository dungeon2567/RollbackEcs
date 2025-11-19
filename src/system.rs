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

    use crate::component::{Component, Destroyed};
    use crate::entity::Entity;
    use crate::scheduler::pipeline::PipelineStage;
    use crate::world::World;

    #[derive(Component)]
    pub struct A(u8);

    #[derive(Component)]
    pub struct B(u8);

    #[derive(Component)]
    pub struct C(u8);

    #[derive(Component)]
    pub struct D(u8);

    #[derive(Component)]
    pub struct E(u8);

    #[derive(Component)]
    pub struct F(u8);

    #[derive(Component)]
    pub struct G(u8);

    #[derive(Component)]
    pub struct H(u8);

    system! {
        DestroySystem {
            query! {
                fn destroy(e: ViewMut<Entity>) All=[Destroyed] {

                }
            }
        }
    }
}
