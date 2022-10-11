#![cfg_attr(not(feature = "std"), no_std)]

use ::core::{marker::PhantomData, pin::Pin, task::{Context, Poll}};
use ::core::future::Future;

#[cfg(feature = "std")]
use futures::future::{BoxFuture, FutureExt, LocalBoxFuture};

#[derive(Clone, Debug)]
pub struct ScopedFuture<'big, 'small, Fut> {
    future: Fut,
    scope: ImpliedLifetimeBound<'big, 'small>,
}

pub type ImpliedLifetimeBound<'big, 'small> = PhantomData<&'small &'big ()>;

#[cfg(feature = "std")]
pub type ScopedBoxFuture<'big, 'small, T> = ScopedFuture<'big, 'small, BoxFuture<'small, T>>;

#[cfg(feature = "std")]
pub type ScopedLocalBoxFuture<'big, 'small, T> = ScopedFuture<'big, 'small, LocalBoxFuture<'small, T>>;

pub trait ScopedFutureExt: Sized {
    fn scoped<'big, 'small>(self) -> ScopedFuture<'big, 'small, Self>;

    #[cfg(feature = "std")]
    fn scope_boxed<'big, 'small>(self) -> ScopedBoxFuture<'big, 'small, <Self as Future>::Output> where Self: Send + Future + 'small;

    #[cfg(feature = "std")]
    fn scope_boxed_local<'big, 'small>(self) -> ScopedLocalBoxFuture<'big, 'small, <Self as Future>::Output> where Self: Future + 'small;
}

impl<'big, 'small, Fut> ScopedFuture<'big, 'small, Fut> {
    pin_utils::unsafe_pinned!(future: Fut);
}

impl<'big, 'small, Fut: Future> Future for ScopedFuture<'big, 'small, Fut> {
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future().poll(cx)
    }
}

impl<Fut: Future> ScopedFutureExt for Fut {
    fn scoped<'big, 'small>(self) -> ScopedFuture<'big, 'small, Self> {
        ScopedFuture {
            future: self,
            scope: PhantomData,
        }
    }

    #[cfg(feature = "std")]
    fn scope_boxed<'big, 'small>(self) -> ScopedFuture<'big, 'small, BoxFuture<'small, <Self as Future>::Output>> where Self: Send + Future + 'small {
        ScopedFuture {
            future: self.boxed(),
            scope: PhantomData,
        }
    }

    #[cfg(feature = "std")]
    fn scope_boxed_local<'big, 'small>(self) -> ScopedFuture<'big, 'small, LocalBoxFuture<'small, <Self as Future>::Output>> where Self: Future + 'small {
        ScopedFuture {
            future: self.boxed_local(),
            scope: PhantomData,
        }
    }
}
