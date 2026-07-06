//! A `requestAnimationFrame`-backed [`Future`].
//!
//! Awaiting [`next_animation_frame`] suspends until the browser is about to
//! repaint, then resolves with the frame's `DOMHighResTimeStamp` (milliseconds).
//! This is the WASM frame boundary that
//! [`CydFrameWasm::flush`](crate::CydFrameWasm) awaits.

use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::Window;

/// State shared between the future and its `requestAnimationFrame` callback.
///
/// Crucially this holds **no** reference to the [`Closure`]: the closure
/// captures an `Rc<Shared>`, but `Shared` never points back, so there is no
/// reference cycle and therefore no per-frame leak. The closure is owned
/// directly by [`NextAnimationFrame`] and dropped when the future completes or
/// is cancelled.
#[derive(Default)]
struct Shared {
    timestamp: Cell<Option<f64>>,
    waker: RefCell<Option<Waker>>,
}

/// Future returned by [`next_animation_frame`].
pub struct NextAnimationFrame {
    window: Window,
    shared: Rc<Shared>,
    // Owned here, not inside `shared`, so the callback's captured `Rc<Shared>`
    // forms no cycle. Kept alive until this future drops.
    _closure: Closure<dyn FnMut(f64)>,
    handle: i32,
}

/// Suspend until the next browser animation frame, resolving with its timestamp.
#[must_use]
pub fn next_animation_frame() -> NextAnimationFrame {
    let window = web_sys::window().expect("a browser window exists");
    let shared = Rc::new(Shared::default());

    let shared_for_callback = shared.clone();
    let closure = Closure::<dyn FnMut(f64)>::new(move |timestamp: f64| {
        shared_for_callback.timestamp.set(Some(timestamp));
        if let Some(waker) = shared_for_callback.waker.borrow_mut().take() {
            waker.wake();
        }
    });

    let handle = window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("request_animation_frame is available");

    NextAnimationFrame {
        window,
        shared,
        _closure: closure,
        handle,
    }
}

impl Future for NextAnimationFrame {
    type Output = f64;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<f64> {
        if let Some(timestamp) = self.shared.timestamp.get() {
            Poll::Ready(timestamp)
        } else {
            *self.shared.waker.borrow_mut() = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for NextAnimationFrame {
    fn drop(&mut self) {
        // If the future is dropped before the callback fires, cancel the pending
        // request so the browser never invokes a closure we are about to free.
        if self.shared.timestamp.get().is_none() {
            self.window
                .cancel_animation_frame(self.handle)
                .expect("cancel_animation_frame is available");
        }
    }
}
