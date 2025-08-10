use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Debug)]
pub struct MutexBox<T> {
    inner: Arc<Mutex<Option<T>>>,
    taken: Arc<Mutex<bool>>,
    notify: Arc<Notify>,
}

impl<T> MutexBox<T>
{
    pub fn new() -> MutexBox<T> {
        Self {
            inner: Arc::new(Mutex::new(None)),
            taken: Arc::new(Mutex::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Nimmt den Wert temporär heraus, führt async-Funktion aus, setzt ihn wieder ein
    pub async fn take_with<F, Fut, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(T) -> Fut,
        Fut: std::future::Future<Output = (T, R)>,
    {
        let mut lock = self.inner.lock().await;
        let value = lock.take()?;
        let mut taken = self.taken.lock().await;
        *taken = true;
        drop(taken);
        drop(lock); // nicht während Await halten!

        let (new_value, result) = f(value).await;

        let mut lock = self.inner.lock().await;
        *lock = Some(new_value);
        let mut taken = self.taken.lock().await;
        *taken = false;
        drop(taken);
        drop(lock);
        self.notify.notify_one();
        Some(result)
    }

    pub async fn take_sync<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(T) -> (T, R),
    {
        let mut lock = self.inner.lock().await;
        let value = lock.take()?;
        let mut taken = self.taken.lock().await;
        *taken = true;
        let (new_value, result) = f(value);
        *lock = Some(new_value);
        *taken = false;
        drop(taken);
        drop(lock);
        self.notify.notify_one();
        Some(result)
    }

    pub async fn is_taken(&self) -> bool {
        let taken = self.taken.lock().await;
        taken.clone()
    }

    pub async fn open_async<F, Fut, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(T) -> Fut + Clone,
        Fut: std::future::Future<Output = (T, R)>,
    {
        loop {
            if !self.is_taken().await {
                return self.take_with(f.clone()).await;
            }
            self.notify.notified().await;
        }
    }

    pub async fn open<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(T) -> (T, R),
    {
        loop {
            if !self.is_taken().await {
                return self.take_sync(f).await;
            }
            self.notify.notified().await;
        }
    }

    pub async fn set(&self, value: Option<T>) {
        loop {
            if !self.is_taken().await {
                let mut lock = self.inner.lock().await;
                *lock = value;
                return ;
            }
            self.notify.notified().await;
        }
    }

    /// Gibt eine clonbare Referenz mit `'static`-Lifetime zurück.
    pub fn clone_handle(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            taken: Arc::clone(&self.taken),
            notify: Arc::clone(&self.notify),
        }
    }
}

impl<T> MutexBox<T>
where
    T: Clone,
{
    pub async fn clone_inner(&self) -> Option<T> {
        let res = self
            .open(|inner| {
                let output = inner.clone();
                (inner, output)
            })
            .await;
        res
    }
}