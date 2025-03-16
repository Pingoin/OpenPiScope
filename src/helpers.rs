use std::cell::RefCell;

pub struct MutexBox<T>(critical_section::Mutex<RefCell<T>>);

impl<T> MutexBox<T> {
    pub const fn new(value: T) -> MutexBox<T> {
        MutexBox(critical_section::Mutex::new(RefCell::new(value)))
    }

    pub fn open<F, R>(&self, f: F) -> R
    where
        F: Fn(&mut T) -> R,
    {
        critical_section::with(|cs| {
            let mut data = self.0.borrow_ref_mut(cs);
            f(&mut data)
        })
    }
    pub fn clone_inner(&self) -> T
    where
        T: Clone,
    {
        self.open(|c| c.clone())
    }
}
