
use eventual::*;
use std::thread;

pub trait FutureHelpers {
    type Value: Send + 'static;
    type Error: Send + 'static;
    
    fn into_inner(self) -> Result<Self::Value,Self::Error>;
}

pub fn spawn<T, E, F>(f: F) -> Future<T, E>
where F: FnOnce() -> Result<T, E> + Send + 'static,
      T: Send + 'static,
      E: Send + 'static
{
    let (completer, future) = Future::pair();
    thread::spawn(move || match f() {
        Ok(good) => completer.complete(good),
        Err(bad) => completer.fail(bad)
    });

    future
}

impl<T: Send + 'static, E: Send + 'static> FutureHelpers for Future<T,E> {
    type Value = T;
    type Error = E;
    
    fn into_inner(self) -> Result<T,E> {
        match self.await() {
            Ok(r) => Ok(r),
            Err(AsyncError::Failed(e)) => Err(e),
            Err(AsyncError::Aborted) => panic!("shouldn't be aborting"),
        }
    }
}
