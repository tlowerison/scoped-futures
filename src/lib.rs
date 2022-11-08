#![cfg_attr(not(feature = "std"), no_std)]

use core::future::Future;
use core::marker::PhantomData;

#[cfg(feature = "std")]
use futures::future::{BoxFuture, FutureExt, LocalBoxFuture};

/// A [`Future`] wrapper type that imposes an upper bound on its lifetime's duration.
/// This is especially useful for callbacks that use higher-ranked lifetimes in their return type,
/// where it can prevent `'static` bounds from being placed on a returned `Future`.
///
/// # Example
/// ```
/// use scoped_futures::{ScopedBoxFuture, ScopedFutureExt};
///
/// pub struct Db {
///     count: u8,
/// }
///
/// impl Db {
///     async fn transaction<'a, F, T, E>(&mut self, callback: F) -> Result<T, E>
///     where
///         // ScopedBoxFuture imposes a lifetime bound on 'b which prevents the hrtb below needing
///         // to be satisfied for all lifetimes (including 'static) and instead only lifetimes
///         // which live at most as long as 'a
///         F: for<'b /* where 'a: 'b */> FnOnce(&'b mut Self) -> ScopedBoxFuture<'a, 'b, Result<T, E>> + Send + 'a,
///         T: 'a,
///         E: 'a,
///     {
///         callback(self).await
///     }
/// }
///
/// pub async fn test_transaction<'a, 'b>(
///     db: &mut Db,
///     ok: &'a str,
///     err: &'b str,
///     is_ok: bool,
/// ) -> Result<&'a str, &'b str> {
///     // note the lack of `move` or any cloning in front of the closure
///     db.transaction(|db| async move {
///         db.count += 1;
///         if is_ok {
///             Ok(ok)
///         } else {
///             Err(err)
///         }
///     }.scope_boxed()).await?;
///
///     // note that `async` can be used instead of `async move` since the callback param is unused
///     db.transaction(|_| async {
///         if is_ok {
///             Ok(ok)
///         } else {
///             Err(err)
///         }
///     }.scope_boxed()).await
/// }
///
/// #[test]
/// fn test_transaction_works() {
///     futures::executor::block_on(async {
///         let mut db = Db { count: 0 };
///         let ok = String::from("ok");
///         let err = String::from("err");
///         let result = test_transaction(&mut db, &ok, &err, true).await;
///         assert_eq!(ok, result.unwrap());
///         assert_eq!(1, db.count);
///     })
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ScopedFuture<'upper_bound, 'a, Fut> {
    future: Fut,
    scope: ImpliedLifetimeBound<'upper_bound, 'a>,
}

/// A wrapper type which imposes an upper bound on a lifetime.
pub type ImpliedLifetimeBound<'upper_bound, 'a> = PhantomData<&'a &'upper_bound ()>;

/// A boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedBoxFuture<'upper_bound, 'a, T> = ScopedFuture<'upper_bound, 'a, BoxFuture<'a, T>>;

/// A non-Send boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedLocalBoxFuture<'upper_bound, 'a, T> = ScopedFuture<'upper_bound, 'a, LocalBoxFuture<'a, T>>;

/// An extension trait for `Future` that provides methods for encoding lifetime upper bound information.
pub trait ScopedFutureExt: Sized {
    /// Encodes the lifetimes of this `Future`'s captures.
    fn scoped<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, Self>;

    /// Boxes this `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn scope_boxed<'upper_bound, 'a>(self) -> ScopedBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Send + Future + 'a;

    /// Boxes this non-Send `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn scope_boxed_local<'upper_bound, 'a>(self) -> ScopedLocalBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Future + 'a;
}

impl<'upper_bound, 'a, Fut> ScopedFuture<'upper_bound, 'a, Fut> {
    pin_utils::unsafe_pinned!(future: Fut);
}

impl<'upper_bound, 'a, Fut: Future> Future for ScopedFuture<'upper_bound, 'a, Fut> {
    type Output = Fut::Output;
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        self.future().poll(cx)
    }
}

impl<Fut: Future> ScopedFutureExt for Fut {
    fn scoped<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, Self> {
        ScopedFuture { future: self, scope: PhantomData }
    }

    #[cfg(feature = "std")]
    fn scope_boxed<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, BoxFuture<'a, <Self as Future>::Output>>
    where
        Self: Send + Future + 'a,
    {
        ScopedFuture { future: self.boxed(), scope: PhantomData }
    }

    #[cfg(feature = "std")]
    fn scope_boxed_local<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, LocalBoxFuture<'a, <Self as Future>::Output>>
    where
        Self: Future + 'a,
    {
        ScopedFuture { future: self.boxed_local(), scope: PhantomData }
    }
}
