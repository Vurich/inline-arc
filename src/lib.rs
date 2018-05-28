#![cfg_attr(test, feature(test))]

use std::cell::UnsafeCell;
use std::mem;
use std::ops::Deref;
use std::sync::{Arc as StdArc, Weak};

pub struct Arc<T> {
    inner: UnsafeCell<ArcData<T>>,
}

unsafe impl<T: Send> Send for Arc<T> {}

enum ArcData<T> {
    Inline(T),
    Shared(StdArc<T>),
    Poisoned,
}

impl<T> Arc<T> {
    pub fn new(val: T) -> Self {
        Arc {
            inner: ArcData::Inline(val).into(),
        }
    }
}

impl<T> Arc<T>
where
    T: Clone,
{
    pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> {
        use ArcData::*;

        let inner = unsafe { &mut *this.inner.get() };

        match inner {
            Inline(val) => Some(val),
            Shared(_) => None,
            Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }

    pub fn try_unwrap(this: Arc<T>) -> Result<T, Arc<T>> {
        use ArcData::*;
        use std::ptr;

        let inner = unsafe { ptr::read(this.inner.get()) };

        match inner {
            Inline(val) => Ok(val),
            Shared(_) => Err(Arc {
                inner: inner.into()
            }),
            Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }

    pub fn strong_count(this: &Arc<T>) -> usize {
        use ArcData::*;

        let inner = unsafe { &mut *this.inner.get() };

        match inner {
            Inline(_) => 1,
            Shared(val) => StdArc::strong_count(val),
            Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }

    pub fn weak_count(this: &Arc<T>) -> usize {
        use ArcData::*;

        let inner = unsafe { &mut *this.inner.get() };

        match inner {
            Inline(_) => 0,
            Shared(val) => StdArc::weak_count(val),
            Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }

    pub fn downgrade(this: &Arc<T>) -> Weak<T> {
        use ArcData::*;

        let inner = mem::replace(unsafe { &mut *this.inner.get() }, Poisoned);

        match inner {
            Inline(val) => {
                let shared = StdArc::new(val);
                let out = StdArc::downgrade(&shared);

                mem::replace(unsafe { &mut *this.inner.get() }, Shared(shared));

                out
            }
            Shared(val) => StdArc::downgrade(&val),
            Poisoned => panic!(
                "`Arc::clone` or `Arc::new` panicked and poisoned `Arc`! This should never happen."
            ),
        }
    }

    pub unsafe fn from_raw(ptr: *const T) -> Arc<T> {
        Arc {
            inner: ArcData::Shared(StdArc::from_raw(ptr)).into(),
        }
    }

    pub fn make_mut(this: &mut Arc<T>) -> &mut T {
        use ArcData::*;

        let inner = unsafe { &mut *this.inner.get() };

        match inner {
            Inline(val) => val,
            Shared(val) => {
                mem::replace(unsafe { &mut *this.inner.get() }, Inline((&**val).clone()));

                match unsafe { &mut *this.inner.get() } {
                    Inline(val) => val,
                    _ => panic!()
                }
            },
            Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        match unsafe { &*self.inner.get() } {
            ArcData::Inline(val) => val,
            ArcData::Shared(val) => &*val,
            ArcData::Poisoned => panic!("`Arc::clone` or `Arc::new` panicked and poisoned `inline_arc::Arc`! This should never happen."),
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        use ArcData::*;

        let inner = mem::replace(unsafe { &mut *self.inner.get() }, Poisoned);

        match inner {
            Inline(val) => {
                let shared = StdArc::new(val);
                mem::replace(unsafe { &mut *self.inner.get() }, Shared(shared.clone()));
                Arc {
                    inner: Shared(shared).into(),
                }
            }
            Shared(val) => Arc {
                inner: Shared(val.clone()).into(),
            },
            Poisoned => panic!(
                "`Arc::clone` or `Arc::new` panicked and poisoned `Arc`! This should never happen."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use self::test::Bencher;

    #[test]
    fn can_make_mut() {
        use super::Arc;

        let mut data = Arc::new(5);

        *Arc::make_mut(&mut data) += 1; // Won't clone anything
        let mut other_data = Arc::clone(&data); // Won't clone inner data
        *Arc::make_mut(&mut data) += 1; // Clones inner data
        *Arc::make_mut(&mut data) += 1; // Won't clone anything
        *Arc::make_mut(&mut other_data) *= 2; // Won't clone anything

        // Now `data` and `other_data` point to different values.
        assert_eq!(*data, 8);
        assert_eq!(*other_data, 12);
    }

    #[bench]
    fn make_mut(b: &mut Bencher) {
        use super::Arc;

        b.iter(|| {
            let mut data = Arc::new(5);

            *Arc::make_mut(&mut data) += 1; // Won't clone anything
            let mut other_data = Arc::clone(&data); // Won't clone inner data
            *Arc::make_mut(&mut data) += 1; // Clones inner data
            *Arc::make_mut(&mut data) += 1; // Won't clone anything
            *Arc::make_mut(&mut other_data) *= 2; // Won't clone anything

            // Now `data` and `other_data` point to different values.
            assert_eq!(*data, 8);
            assert_eq!(*other_data, 12);
        });
    }

    #[bench]
    fn make_mut_std(b: &mut Bencher) {
        use std::sync::Arc;

        b.iter(|| {
            let mut data = Arc::new(5);

            *Arc::make_mut(&mut data) += 1; // Won't clone anything
            let mut other_data = Arc::clone(&data); // Won't clone inner data
            *Arc::make_mut(&mut data) += 1; // Clones inner data
            *Arc::make_mut(&mut data) += 1; // Won't clone anything
            *Arc::make_mut(&mut other_data) *= 2; // Won't clone anything

            // Now `data` and `other_data` point to different values.
            assert_eq!(*data, 8);
            assert_eq!(*other_data, 12);
        });
    }
}
