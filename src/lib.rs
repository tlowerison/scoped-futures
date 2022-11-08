#![cfg_attr(not(feature = "std"), no_std)]

use core::{future::Future, marker::PhantomData, pin::Pin};

/// A [`Future`] with an implied upper bound on the lifetime of its provided lifetime.
pub trait ScopedFuture<'upper_bound, 'a, Bound: sealed::SealedBound = ImpliedLifetimeBound<'upper_bound, 'a>>: Future {}

/// A wrapper type which imposes an upper bound on a lifetime.
pub type ImpliedLifetimeBound<'upper_bound, 'a> = PhantomData<&'a &'upper_bound ()>;

impl<'b: 'a, 'a, Fut: Future + 'a> ScopedFuture<'b, 'a> for Fut {}

mod sealed {
    pub trait SealedBound {}
    impl<'upper_bound, 'a> SealedBound for super::ImpliedLifetimeBound<'upper_bound, 'a> {}
}

/// A boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedBoxFuture<'upper_bound, 'a, T> = Pin<Box<dyn ScopedFuture<'upper_bound, 'a, Output = T> + Send + 'a>>;

/// A non-Send boxed future whose lifetime is upper bounded.
#[cfg(feature = "std")]
pub type ScopedLocalBoxFuture<'upper_bound, 'a, T> = Pin<Box<dyn ScopedFuture<'upper_bound, 'a, Output = T> + 'a>>;

/// A [`Future`] wrapper type that imposes an upper bound on its lifetime's duration.
/// This is especially useful for callbacks that use higher-ranked lifetimes in their return type,
/// where it can prevent `'static` bounds from being placed on a returned `Future`.
///
/// # Example
/// ```
/// use core::pin::Pin;
/// use scoped_futures::{ScopedFuture, ScopedFutureExt};
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
///         F: for<'b /* where 'a: 'b */> FnOnce(&'b mut Self) -> Pin<Box<dyn ScopedFuture<'a, 'b, Output = Result<T, E>> + Send + 'b>> + Send + 'a,
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
///     }.boxed()).await?;
///
///     // note that `async` can be used instead of `async move` since the callback param is unused
///     db.transaction(|_| async {
///         if is_ok {
///             Ok(ok)
///         } else {
///             Err(err)
///         }
///     }.boxed()).await
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
pub struct ScopedFutureWrapper<'upper_bound, 'a, Fut> {
    future: Fut,
    scope: ImpliedLifetimeBound<'upper_bound, 'a>,
}

/// An extension trait for `Future` that provides methods for encoding lifetime upper bound information.
pub trait ScopedFutureExt: Sized {
    /// Boxes this `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn boxed<'upper_bound, 'a>(self) -> ScopedBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Send + Future + 'a;

    /// Boxes this `Future` and encodes the lifetimes of its captures.
    #[cfg(feature = "std")]
    fn boxed_local<'upper_bound, 'a>(self) -> ScopedLocalBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Future + 'a;

    /// Encodes the lifetimes of this `Future`'s captures.
    fn scoped<'upper_bound, 'a>(self) -> ScopedFutureWrapper<'upper_bound, 'a, Self>;
}

impl<'upper_bound, 'a, Fut> ScopedFutureWrapper<'upper_bound, 'a, Fut> {
    pin_utils::unsafe_pinned!(future: Fut);
}

impl<'upper_bound, 'a, Fut: Future> Future for ScopedFutureWrapper<'upper_bound, 'a, Fut> {
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        self.future().poll(cx)
    }
}

impl<Fut: Future> ScopedFutureExt for Fut {
    #[cfg(feature = "std")]
    fn boxed<'upper_bound, 'a>(self) -> ScopedBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Send + Future + 'a,
    {
        Box::pin(self)
    }

    #[cfg(feature = "std")]
    fn boxed_local<'upper_bound, 'a>(self) -> ScopedLocalBoxFuture<'upper_bound, 'a, <Self as Future>::Output>
    where
        Self: Future + 'a,
    {
        Box::pin(self)
    }

    fn scoped<'upper_bound, 'a>(self) -> ScopedFutureWrapper<'upper_bound, 'a, Self> {
        ScopedFutureWrapper { future: self, scope: PhantomData }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        impl<'upper_bound, 'a, T, Fut: Future<Output = T> + Send + 'a> From<Pin<Box<Fut>>> for ScopedBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Pin<Box<Fut>>) -> Self {
                future
            }
        }

        impl<'upper_bound, 'a, T, Fut: Future<Output = T> + 'a> From<Pin<Box<Fut>>> for ScopedLocalBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Pin<Box<Fut>>) -> Self {
                future
            }
        }

        impl<'upper_bound, 'a, T, Fut: Future<Output = T> + Send + 'a> From<Box<Fut>> for ScopedBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Box<Fut>) -> Self {
                Box::into_pin(future)
            }
        }

        impl<'upper_bound, 'a, T, Fut: Future<Output = T> + 'a> From<Box<Fut>> for ScopedLocalBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Box<Fut>) -> Self {
                Box::into_pin(future)
            }
        }

        impl<'upper_bound, 'a, T: 'a> From<Pin<Box<dyn Future<Output = T> + Send + 'a>>> for ScopedBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Pin<Box<dyn Future<Output = T> + Send + 'a>>) -> Self {
                Box::pin(future)
            }
        }

        impl<'upper_bound, 'a, T: 'a> From<Pin<Box<dyn Future<Output = T> + 'a>>> for ScopedLocalBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Pin<Box<dyn Future<Output = T> + 'a>>) -> Self {
                Box::pin(future)
            }
        }

        impl<'upper_bound, 'a, T: 'a> From<Box<dyn Future<Output = T> + Send + 'a>> for ScopedBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Box<dyn Future<Output = T> + Send + 'a>) -> Self {
                Box::into_pin(future).into()
            }
        }

        impl<'upper_bound, 'a, T: 'a> From<Box<dyn Future<Output = T> + 'a>> for ScopedLocalBoxFuture<'upper_bound, 'a, T> {
            fn from(future: Box<dyn Future<Output = T> + 'a>) -> Self {
                Box::into_pin(future).into()
            }
        }
    }
}
