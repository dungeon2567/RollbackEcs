use std::any::Any;
use std::cell::OnceCell;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

static COMPONENT_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub trait Component: Any where Self: Sized {
    fn type_index() -> usize {
        static ID: OnceLock<usize> = OnceLock::new();

        *ID.get_or_init(|| COMPONENT_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait Tag: Component + Any where Self: Sized{

}

#[derive(Tag)]
pub struct Destroyed {}

pub use rollback_macros::Component;
pub use rollback_macros::Tag;