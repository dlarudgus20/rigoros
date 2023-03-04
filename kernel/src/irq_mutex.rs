use core::ops::{Deref, DerefMut};
use spin::{Mutex, MutexGuard};
use x86_64::instructions::interrupts::{are_enabled, enable, disable};

pub struct IrqMutex<T> {
    inner: Mutex<T>,
}

pub struct IrqMutexGuard<'a, T> {
    inner: MutexGuard<'a, T>,
    saved_intpt_flag: bool,
}

impl<T> IrqMutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> IrqMutexGuard<T> {
        let saved_intpt_flag = are_enabled();
        if saved_intpt_flag {
            disable();
        }

        IrqMutexGuard {
            inner: self.inner.lock(),
            saved_intpt_flag,
        }
    }

    pub fn try_lock(&self) -> Option<IrqMutexGuard<T>> {
        let saved_intpt_flag = are_enabled();
        if saved_intpt_flag {
            disable();
        }

        if let Some(guard) = self.inner.try_lock() {
            Some(IrqMutexGuard {
                inner: guard,
                saved_intpt_flag,
            })
        }
        else {
            if saved_intpt_flag {
                enable();
            }
            None
        }
    }
}

impl<T> Drop for IrqMutexGuard<'_, T> {
    fn drop(&mut self) {
        if self.saved_intpt_flag {
            enable();
        }
    }
}

impl<T> Deref for IrqMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.deref()
    }
}

impl<T> DerefMut for IrqMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.inner.deref_mut()
    }
}

unsafe impl<T: Send> Send for IrqMutex<T> {}
unsafe impl<T: Send> Sync for IrqMutex<T> {}
