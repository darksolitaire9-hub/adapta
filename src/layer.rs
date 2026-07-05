use crate::AdaptationStrategy;
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Instant;
use tower_layer::Layer;
use tower_service::Service;

/// Trait for extracting an adaptation signal (e.g., latency or error metric) from request/response results.
pub trait SignalExtractor<Res, Err>: Send + Sync + 'static {
    fn extract(&self, result: &Result<Res, Err>, elapsed_ms: f64) -> Option<f64>;
}

/// Default extractor that observes request latency in milliseconds for all requests.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencyExtractor;

impl<Res, Err> SignalExtractor<Res, Err> for LatencyExtractor
where
    Res: Send + 'static,
    Err: Send + 'static,
{
    fn extract(&self, _result: &Result<Res, Err>, elapsed_ms: f64) -> Option<f64> {
        Some(elapsed_ms)
    }
}

/// Extractor that only observes latency when the underlying service returns an Err.
#[derive(Debug, Clone, Copy, Default)]
pub struct ErrorLatencyExtractor;

impl<Res, Err> SignalExtractor<Res, Err> for ErrorLatencyExtractor
where
    Res: Send + 'static,
    Err: Send + 'static,
{
    fn extract(&self, result: &Result<Res, Err>, elapsed_ms: f64) -> Option<f64> {
        match result {
            Err(_) => Some(elapsed_ms),
            Ok(_) => None,
        }
    }
}

/// A Tower layer that wraps services with closed-loop error observation and adaptation.
#[derive(Debug, Clone)]
pub struct AdaptiveLayer<S, X> {
    strategy: Arc<Mutex<S>>,
    extractor: Arc<X>,
}

impl<S> AdaptiveLayer<S, LatencyExtractor> {
    /// Creates a new AdaptiveLayer observing latency across all requests.
    pub fn new(strategy: S) -> Self {
        Self {
            strategy: Arc::new(Mutex::new(strategy)),
            extractor: Arc::new(LatencyExtractor),
        }
    }
}

impl<S, X> AdaptiveLayer<S, X> {
    /// Creates a new AdaptiveLayer with a custom signal extractor.
    pub fn with_extractor(strategy: S, extractor: X) -> Self {
        Self {
            strategy: Arc::new(Mutex::new(strategy)),
            extractor: Arc::new(extractor),
        }
    }

    /// Returns a thread-safe handle to the adaptation strategy.
    pub fn strategy_handle(&self) -> Arc<Mutex<S>> {
        Arc::clone(&self.strategy)
    }
}

impl<S, X, Inner> Layer<Inner> for AdaptiveLayer<S, X>
where
    S: Send + 'static,
    X: Clone,
{
    type Service = AdaptiveService<Inner, S, X>;

    fn layer(&self, inner: Inner) -> Self::Service {
        AdaptiveService {
            inner,
            strategy: Arc::clone(&self.strategy),
            extractor: Arc::clone(&self.extractor),
        }
    }
}

/// A Tower service wrapper that observes request signals and feeds them into an AdaptationStrategy.
#[derive(Debug, Clone)]
pub struct AdaptiveService<Inner, S, X> {
    inner: Inner,
    strategy: Arc<Mutex<S>>,
    extractor: Arc<X>,
}

impl<Inner, S, X> AdaptiveService<Inner, S, X> {
    /// Returns a thread-safe handle to the underlying adaptation strategy.
    pub fn strategy_handle(&self) -> Arc<Mutex<S>> {
        Arc::clone(&self.strategy)
    }
}

impl<Inner, S, X, Req> Service<Req> for AdaptiveService<Inner, S, X>
where
    Inner: Service<Req>,
    S: AdaptationStrategy<f64, f64> + Send + 'static,
    X: SignalExtractor<Inner::Response, Inner::Error>,
{
    type Response = Inner::Response;
    type Error = Inner::Error;
    type Future = AdaptiveFuture<Inner::Future, S, X>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let start = Instant::now();
        let fut = self.inner.call(req);
        AdaptiveFuture {
            inner: fut,
            strategy: Arc::clone(&self.strategy),
            extractor: Arc::clone(&self.extractor),
            start,
        }
    }
}

pin_project! {
    /// Future wrapper that measures elapsed execution time and records adaptation signals upon completion.
    pub struct AdaptiveFuture<F, S, X> {
        #[pin]
        inner: F,
        strategy: Arc<Mutex<S>>,
        extractor: Arc<X>,
        start: Instant,
    }
}

impl<F, S, X, Res, Err> Future for AdaptiveFuture<F, S, X>
where
    F: Future<Output = Result<Res, Err>>,
    S: AdaptationStrategy<f64, f64> + Send + 'static,
    X: SignalExtractor<Res, Err>,
{
    type Output = Result<Res, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(cx) {
            Poll::Ready(result) => {
                let elapsed_ms = this.start.elapsed().as_secs_f64() * 1000.0;
                if let Some(signal) = this.extractor.extract(&result, elapsed_ms) {
                    if let Ok(mut strat) = this.strategy.lock() {
                        strat.observe(signal);
                    }
                }
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
