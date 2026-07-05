#![cfg(feature = "tower")]

use adapta::edgeworth::EdgeworthBackoff;
use adapta::layer::{AdaptiveLayer, ErrorLatencyExtractor};
use adapta::AdaptationStrategy;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Debug, Clone, Default)]
struct MockService {
    calls: usize,
}

impl Service<&'static str> for MockService {
    type Response = &'static str;
    type Error = &'static str;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: &'static str) -> Self::Future {
        self.calls += 1;
        Box::pin(async move {
            if req == "fail" {
                Err("simulated failure")
            } else {
                Ok("success")
            }
        })
    }
}

#[tokio::test]
async fn test_tower_middleware_observation() {
    let strategy = EdgeworthBackoff::new(100.0, 0.1, 0.05);
    let layer = AdaptiveLayer::with_extractor(strategy, ErrorLatencyExtractor);
    let mut service = layer.layer(MockService::default());

    // Call with success -> ErrorLatencyExtractor should ignore it
    let res = service.call("ok").await;
    assert!(res.is_ok());
    assert_eq!(service.strategy_handle().lock().unwrap().sample_count(), 0);

    // Call 15 times with failure -> ErrorLatencyExtractor should observe 15 failure latencies
    for _ in 0..15 {
        let res = service.call("fail").await;
        assert!(res.is_err());
    }

    let handle = service.strategy_handle();
    let strat = handle.lock().unwrap();
    assert_eq!(strat.sample_count(), 15);
    // Because we have >= 15 samples, compute_skew_kurt should now be Some!
    assert!(strat.compute_skew_kurt().is_some());
}
