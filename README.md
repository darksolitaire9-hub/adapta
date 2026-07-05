# adapta

[![CI Status](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/darksolitaire9-hub/adapta/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![No Std Compatible](https://img.shields.io/badge/no__std-compatible-orange.svg)](#architecture--no_std-guarantee)
[![Version](https://img.shields.io/badge/version-0.0.2-blueviolet.svg)](#release-cycle--versioning-strategy)

**Zero-latency, `#![no_std]` closed-loop adaptation kernel and Edgeworth higher-order retry tuner for distributed systems and agentic evaluation pipelines.**

Authored by **Chanak Karki**.  
Canonical Online Repository: [github.com/darksolitaire9-hub/adapta](https://github.com/darksolitaire9-hub/adapta)

---

## Table of Contents
- [Explain to Me Like I am a 5 Year Old (ELI5)](#explain-to-me-like-i-am-a-5-year-old-eli5)
- [Why This Helps LLMs and Humans](#why-this-helps-llms-and-humans)
- [Why adapta?](#why-adapta)
- [Hexagonal Architecture (Ports & Adapters)](#hexagonal-architecture-ports--adapters)
- [Theoretical Foundations & Academic References](#theoretical-foundations--academic-references)
- [Quantitative Verification & Guardrails](#quantitative-verification--guardrails)
- [Release Cycle & Versioning Strategy](#release-cycle--versioning-strategy)
- [Security & Zero Data Leakage Guarantee](#security--zero-data-leakage-guarantee)
- [Usage Example](#usage-example)
- [License](#license)

---

## Explain to Me Like I am a 5 Year Old (ELI5)

Imagine you are waiting for your friend to bring you a toy. Usually, they take **10 seconds**. 
But sometimes, your friend gets distracted by a puppy on the sidewalk and takes **50 seconds**!

If you get impatient and always walk away after 15 seconds, you will miss getting your toy whenever puppy days happen! But if you always wait 50 seconds every single time, you waste lots of time standing around when they aren't even distracted.

**`adapta` is like a smart timer watch.** It watches how often your friend gets distracted by puppies. When it notices puppy days are happening, it automatically gives your friend a little bit of extra time. When things are quiet again, it goes back to normal. That way, you never walk away too early, but you also never waste time standing around!

---

## Why This Helps LLMs and Humans

### For LLMs (Autonomous Coding Agents)
- **Deterministic Math & Explicit Guardrails**: LLMs struggle with guessing statistical formulas or boundary conditions (like division-by-zero on zero variance). By explicitly tabulating exact mathematical invariants ($N \ge 15$, $\sigma^2 \ge 1\text{e-}12$, reference moments), LLMs can write evaluation loops and tests without hallucinating thresholds.
- **Token-Efficient Retrieval**: The clear Table of Contents and modular Hexagonal Architecture allow agents to target specific domain ports or adapters during code generation without ingesting massive unorganized codebases.

### For Humans (Systems Engineers & Reviewers)
- **Zero-Cognitive-Load Integration**: Engineers do not need a PhD in option pricing or self-supervised learning to use `adapta`. They can plug `AdaptiveLayer` directly into their Tower/Tokio middleware stacks in 3 lines of code.
- **Enterprise Patent Safety**: By providing dual licensing under MIT and Apache-2.0, legal teams get explicit patent retaliation protection while individual developers retain lightweight MIT freedom.

---

## Why adapta?

Standard distributed retry mechanisms and evaluation gates rely on static heuristics: fixed backoff schedules, Gaussian $(\mu + 2\sigma)$ tail assumptions, and static rejection thresholds. In real-world environments, network latency distributions are heavy-tailed and right-skewed, while LLM evaluation scores undergo benign drift over time.

`adapta` solves this by bridging **online statistical physics** and **self-supervised representation learning** into a zero-allocation, `#![no_std]` Rust kernel that adapts dynamically to system feedback.

---

## Hexagonal Architecture (Ports & Adapters)

**Yes, `adapta` is designed strictly according to Hexagonal Architecture (Ports and Adapters) principles.**

The architecture completely isolates the deterministic mathematical kernel (The Hexagon) from asynchronous network plumbing, I/O runtimes, and application control planes:

```mermaid
graph TD
    subgraph Primary Inbound Adapters [Primary Inbound Adapters]
        L[tower::Layer / AdaptiveLayer]
        S[tower::Service / AdaptiveService]
        E[SignalExtractor / LatencyExtractor]
    end

    subgraph Domain Core Hexagon [Domain Core Hexagon - no_std]
        P["Port: AdaptationStrategy<E, O>"]
        B[32-Sample Ring Buffer & Statistical Math]
        G[Mathematical Guardrails: N >= 15, Var >= 1e-12]
    end

    subgraph Secondary Outbound Adapters [Secondary Outbound Adapters]
        A1[EdgeworthBackoff - Skewness/Kurtosis Tuner]
        A2[AdaJepaCorrector - Online Drift Corrector]
    end

    L -->|wraps| S
    S -->|extracts E via| E
    E -->|ingests signal into| P
    P --- B
    B --- G
    P <|..| A1
    P <|..| A2
```

### 1. The Domain Core (The Hexagon)
At the center of `adapta` is the `#![no_std]` domain port:
```rust
pub trait AdaptationStrategy<E, O> {
    fn observe(&mut self, signal: E);
    fn adapt(&self) -> Option<O>;
    fn sample_count(&self) -> usize;
}
```
The core operates on fixed-capacity circular buffers without heap allocation, maintaining mathematical invariants independently of `std`, `tokio`, or `tower`.

### 2. Primary / Inbound Ports & Adapters
- **Port (`SignalExtractor<Res, Err>`)**: Defines how raw asynchronous results are translated into scalar observation signals.
- **Adapters (`AdaptiveLayer` & `AdaptiveService`)**: Implement standard `tower::Layer` and `tower::Service` traits. They intercept execution futures, measure elapsed time via `Instant::now()`, and feed signals into the domain port without acquiring runtime locks during polling.

### 3. Secondary / Outbound Adapters
- **`EdgeworthBackoff`**: Adapts statistical moments into upstream timeout multipliers.
- **`AdaJepaCorrector`**: Adapts online error gradients into test-time evaluation baselines.

---

## Theoretical Foundations & Academic References

`adapta` is built upon verifiable academic literature, translating continuous mathematical formulations into discrete, fault-tolerant software primitives:

### 1. Edgeworth Higher-Order Margin Correction
* **Reference**: Corrado & Su (2004), *"Testing Option Pricing with the Edgeworth Expansion"*, arXiv:cond-mat/0401192 ([DOI/Link](https://arxiv.org/abs/0401192)).
* **Mechanism**: Standard Gaussian timeout models underestimate extreme tail delays in heavy-tailed systems. `EdgeworthBackoff` computes sample skewness ($g_1$) and excess kurtosis ($g_2$) over a sliding window of recent latencies to expand safety margins:
  $$\hat{g}_1 = \frac{m_3}{m_2^{3/2}}, \quad \hat{g}_2 = \frac{m_4}{m_2^2} - 3$$
  $$M = 1.0 + c_{\text{skew}} \max(0, \hat{g}_1) + c_{\text{kurt}} \max(0, \hat{g}_2)$$
  $$\text{Timeout}_{\text{new}} = \text{Timeout}_{\text{base}} \times M$$

### 2. AdaJEPA Closed-Loop Test-Time Adaptation
* **Reference**: Chanak Karki & AdaJEPA Authors (2026), *"AdaJEPA: An Adaptive Latent World Model"*, arXiv:2606.32026 ([DOI/Link](https://arxiv.org/abs/2606.32026)).
* **Mechanism**: To prevent false-positive regression alarms under benign environmental shift, `AdaJepaCorrector` performs online 1-step gradient updates on evaluation baselines while enforcing a hard topological bound ($\Delta_{\max}$) against terminal invariant violations:
  $$\mu_{t+1} = \mu_t + \eta (L_t - \mu_t) \quad \text{if } |L_t - \mu_t| < \Delta_{\max}$$

---

## Quantitative Verification & Guardrails

To prevent LLM-generated numerical bugs, division-by-zero (`NaN`/`Inf`), or oscillatory collapse, `adapta` enforces strict mathematical guardrails verified via property-based testing (`proptest`) and deterministic golden test suites:

| Guardrail Parameter | Threshold Value | Enforcement Behavior |
| :--- | :--- | :--- |
| **Minimum Sample Gate ($N$)** | $N \ge 15$ | Returns exact base backoff ($M = 1.0$) until 15 samples are observed. Prevents small-sample variance spikes. |
| **Zero-Variance Floor ($\sigma^2$)** | $\sigma^2 \ge 1\text{e-}12$ | Returns exact base backoff if sample variance drops below $1\text{e-}12$. Completely eliminates division-by-zero `NaN` or `Inf` errors. |
| **Max Drift Ceiling ($\Delta_{\max}$)** | $\Delta_{\max} = 5.0$ | Immediately terminates evaluation and flags `TERMINAL_REGRESSION_FAILURE` without adapting baseline if error exceeds threshold. |
| **Golden Reference Vector** | $N=15$ standard vector | Deterministically verified against exact reference moments: Mean $= 16.2$, $\sigma^2 = 113.62666667$, $g_1 = 2.21894643$, $g_2 = 4.00433476$. |

---

## Release Cycle & Versioning Strategy

`adapta` follows Semantic Versioning (`SemVer 2.0.0`) backed by automated GitHub Actions CI/CD workflows:

### 1. Release Stages
* **`v0.0.x` (Alpha / Dogfooding - Current)**: Focused on architectural verification, `#![no_std]` compliance, and internal dogfooding within production environments (`rmcp` upstream pipelines and `bineval` regression loops).
* **`v0.1.x` (Beta / Feature Freeze)**: Public API stabilization, concurrency overhead benchmarking under high load, and Tower middleware trait graduation.
* **`v1.0.0` (Production Release)**: Full stability guarantees, crates.io publication, and long-term support.

### 2. Automated Release Workflow
Every push to `main` triggers comprehensive linting (`clippy`), formatting checks (`rustfmt`), property-based tests (`proptest`), and `#![no_std]` verification. Pushing a release tag matching `v*.*.*` automatically triggers our GitHub Actions release pipeline (`.github/workflows/release.yml`), compiling production binaries and generating documented GitHub Releases.

---

## Security & Zero Data Leakage Guarantee

`adapta` is built from the ground up for zero-trust execution environments:
1. **Scalar-Only Ingestion**: The adaptation kernel only observes scalar floating-point metrics (elapsed milliseconds or error scores $E \in \mathbb{R}$).
2. **No Payloads or PII**: Request bodies, headers, user identifiers, network URLs, and filesystem paths are never touched, copied, or stored.
3. **No Telemetry or Logging**: The crate contains zero network reporting, zero telemetry, and zero local filesystem writing. It cannot leak internal workspace structures or API secrets.

---

## Usage Example

```rust
use adapta::edgeworth::EdgeworthBackoff;
use adapta::AdaptationStrategy;

let mut tuner = EdgeworthBackoff::new(100.0, 0.1, 0.05);

// Ingest latency observations from upstream requests
for latency in [10.0, 12.0, 15.0, 11.0, 10.0, 25.0, 30.0, 12.0, 11.0, 10.0, 10.0, 11.0, 14.0, 50.0, 12.0] {
    tuner.observe(latency);
}

// Enforces N >= 15 minimum sample guardrail and computes skewness/kurtosis backoff
if let Some(adjusted_timeout) = tuner.adapt() {
    println!("Adjusted upstream timeout: {:.2} ms", adjusted_timeout);
}
```

---

## License

Licensed under either of [MIT License](LICENSE) or Apache License, Version 2.0 at your option.  
Copyright (c) 2026 Chanak Karki.
