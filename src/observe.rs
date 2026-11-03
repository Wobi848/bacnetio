//! What a reading reports while it runs: each request with its duration and outcome (the request
//! log), and how far it is, with the points read so far.

use std::fmt::Display;
use std::future::Future;
use std::time::{Duration, Instant};

use crate::points::Point;

/// One request to a device, as the request log shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// "ReadPropertyMultiple, 16 objects"
    pub what: String,
    pub took: Duration,
    /// `None` when the device answered; else why not, in its words or as a timeout.
    pub failed: Option<String>,
}

pub enum Event<'a> {
    Request(Request),
    /// Entries of the object list known so far, of all; `total` is `None` until the length is known.
    Listed {
        read: usize,
        total: Option<usize>,
    },
    /// Points whose values were read so far, and all points, read or not.
    Values {
        read: usize,
        points: &'a [Point],
    },
}

/// Receives the events of a reading; `&|_| {}` when nobody listens.
pub type Observer<'a> = &'a (dyn Fn(Event<'_>) + Sync);

/// Runs a request, measures it and reports it.
pub async fn timed<T, E: Display>(
    observe: Observer<'_>,
    what: impl FnOnce() -> String,
    request: impl Future<Output = Result<T, E>>,
) -> Result<T, E> {
    let started = Instant::now();
    let result = request.await;
    observe(Event::Request(Request {
        what: what(),
        took: started.elapsed(),
        failed: result.as_ref().err().map(ToString::to_string),
    }));
    result
}
