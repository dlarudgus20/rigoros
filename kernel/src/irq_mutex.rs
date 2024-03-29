use core::ops::{Deref, DerefMut};
use core::mem::ManuallyDrop;
use spin::{Mutex, MutexGuard};
use x86_64::instructions::interrupts::{are_enabled, enable, disable};

struct InterruptGuard {
    saved_intpt_flag: bool,
}

pub struct IrqMutex<T> {
    inner: Mutex<T>,
}

pub struct IrqMutexGuard<'a, T> {
    inner: ManuallyDrop<MutexGuard<'a, T>>,
    intr: ManuallyDrop<InterruptGuard>,
}

impl InterruptGuard {
    fn lock() -> Self {
        let saved_intpt_flag = are_enabled();
        if saved_intpt_flag {
            disable();
        }

        Self {
            saved_intpt_flag
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.saved_intpt_flag {
            enable();
        }
    }
}

impl<T> IrqMutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> IrqMutexGuard<T> {
        let intr = InterruptGuard::lock();
        let inner = self.inner.lock();
        IrqMutexGuard {
            inner: ManuallyDrop::new(inner),
            intr: ManuallyDrop::new(intr),
        }
    }

    pub fn try_lock(&self) -> Option<IrqMutexGuard<T>> {
        let intr = InterruptGuard::lock();

        if let Some(guard) = self.inner.try_lock() {
            Some(IrqMutexGuard {
                inner: ManuallyDrop::new(guard),
                intr: ManuallyDrop::new(intr),
            })
        }
        else {
            None
        }
    }
}

impl<T> Drop for IrqMutexGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.inner);
            ManuallyDrop::drop(&mut self.intr);
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
