#![cfg_attr(not(feature = "std"), no_std)]

use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

#[cfg(feature = "std")]
use futures::future::{BoxFuture, FutureExt, LocalBoxFuture};

/// A [`Future`] wrapper that imposes an upper limit on the future's lifetime's duration.
/// This is especially useful in combination with higher-tranked bounds when a lifetime
/// bound is needed for the higher-ranked lifetime and a future is used in the bound.
///
/// # Example
/// ```
/// use futures::future::{ScopedBoxFuture, ScopedFutureExt};
///
/// pub struct Db {
///     count: u8,
/// }
///
/// impl Db {
///     async fn transaction<'a, T: 'a, E: 'a, F: 'a>(&mut self, callback: F) -> Result<T, E>
///     where
///         // ScopedBoxFuture imposes a lifetime bound on 'b which prevents the hrtb below needing to be satisfied
///         // for all lifetimes (including 'static) and instead only lifetimes which live at most as long as 'a
///         F: for<'b /* where 'a: 'b */> FnOnce(&'b mut Self) -> ScopedBoxFuture<'a, 'b, Result<T, E>> + Send,
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
///     db.transaction(|db| async move {
///         db.count += 1;
///         if is_ok {
///             Ok(ok)
///         } else {
///             Err(err)
///         }
///     }.scope_boxed()).await?;
///
///     // note that `async` is used instead of `async move`
///     // since the callback parameter is unused
///     db.transaction(|_| async {
///         if is_ok {
///             Ok(ok)
///         } else {
///             Err(err)
///         }
///     }.scope_boxed()).await
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ScopedFuture<'upper_bound, 'a, Fut> {
    future: Fut,
    scope: ImpliedLifetimeBound<'upper_bound, 'a>,
}

/// A wrapper type which imposes an upper bound on the provided lifetime.
pub type ImpliedLifetimeBound<'upper_bound, 'a> = PhantomData<&'a &'upper_bound ()>;

/// A boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedBoxFuture<'upper_bound, 'a, T> = ScopedFuture<'upper_bound, 'a, BoxFuture<'a, T>>;

/// A non-Send boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedLocalBoxFuture<'upper_bound, 'a, T> =
    ScopedFuture<'upper_bound, 'a, LocalBoxFuture<'a, T>>;

/// An extension trait for `Future`s that provides methods for encoding lifetime information of captures.
pub trait ScopedFutureExt: Sized {
    /// Encodes the lifetimes of this `Future`'s captures.
    fn scoped<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, Self>;

    /// Boxes this `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn scope_boxed<'upper_bound, 'a>(
        self,
    ) -> ScopedBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Send + Future + 'a;

    /// Boxes this non-Send `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn scope_boxed_local<'upper_bound, 'a>(
        self,
    ) -> ScopedLocalBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Future + 'a;
}

impl<'upper_bound, 'a, Fut> ScopedFuture<'upper_bound, 'a, Fut> {
    pin_utils::unsafe_pinned!(future: Fut);
}

impl<'upper_bound, 'a, Fut: Future> Future for ScopedFuture<'upper_bound, 'a, Fut> {
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future().poll(cx)
    }
}

impl<Fut: Future> ScopedFutureExt for Fut {
    fn scoped<'upper_bound, 'a>(self) -> ScopedFuture<'upper_bound, 'a, Self> {
        ScopedFuture {
            future: self,
            scope: PhantomData,
        }
    }

    #[cfg(feature = "std")]
    fn scope_boxed<'upper_bound, 'a>(
        self,
    ) -> ScopedFuture<'upper_bound, 'a, BoxFuture<'a, <Self as Future>::Output>>
    where
        Self: Send + Future + 'a,
    {
        ScopedFuture {
            future: self.boxed(),
            scope: PhantomData,
        }
    }

    #[cfg(feature = "std")]
    fn scope_boxed_local<'upper_bound, 'a>(
        self,
    ) -> ScopedFuture<'upper_bound, 'a, LocalBoxFuture<'a, <Self as Future>::Output>>
    where
        Self: Future + 'a,
    {
        ScopedFuture {
            future: self.boxed_local(),
            scope: PhantomData,
        }
    }
}
