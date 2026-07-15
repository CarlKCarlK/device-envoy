<!-- todo0 consider deleting this spec once the work below is implemented and released. -->

# DNS Resolver Boundary

## Purpose

Move the DNS **service boundary** to where it belongs. Today the platform DNS
closure resolves a hostname, but it also times itself, decides what counts as
success, and logs — none of which is platform-specific. Those three concerns
are copy-pasted, near-identically, across the ESP, RP, and WASM examples.

After this change, `Dns` is a plain resolver: given a hostname, return the
addresses or an error. The shared `dns_tester::run` loop owns timing, the
success verdict, the running tally, and any logging.

The resolver itself is not a per-platform closure. ESP and RP both receive the
same `embassy_net::Stack`, so one concrete struct — `DnsWithStack` — does the
query for both. The closure-injection adapter (`DnsRuntime`) is removed. The
`Dns` trait remains as the seam that lets the WASM mock and the host tests
supply a resolver with no real stack.

## Current problem

The `Dns` contract returns a pre-digested tester verdict rather than a DNS
result:

```rust
// device-envoy-core/src/dns.rs (today)
pub struct DnsResult {
    pub succeeded: bool,      // "Ok and non-empty" — a tester judgement
    pub latency_millis: u64,  // measured by the adapter
}

pub trait Dns {
    type Error;
    fn hostname(&self) -> &'static str;
    fn lookup(&mut self) -> impl Future<Output = Result<DnsResult, Self::Error>>;
}
```

Every platform closure therefore repeats the same non-platform work:

```rust
let query_start = Instant::now();
let dns_result = stack.dns_query(DNS_HOSTNAME, DnsQueryType::A).await; // <- the only per-platform line
let latency_millis = query_start.elapsed().as_millis();
let succeeded = match dns_result { /* Ok non-empty / Ok empty / Err */ };
// info!/warn! logging
Ok(DnsResult { succeeded, latency_millis })
```

Two smells confirm the boundary is misplaced:

- The WASM mock has no real query to time, so it **fabricates**
  `latency_millis: 12`.
- Both `device-envoy-core` and `device-envoy-examples-core` already depend on
  `embassy-time`, so `run` can call `Instant::now()` itself. Nothing forced
  timing into the adapter.

The hostname is also stated twice per example — stored in `DnsRuntime` *and*
captured directly by the closure (`TODO000 why listed twice`).

## Target design

### `Dns` becomes a resolver

The trait takes the hostname as an **input to `resolve`** and returns the
lookup outcome. It no longer knows about time, success, or the tally.

```rust
// device-envoy-core/src/dns.rs (target)

/// One resolved address (embassy-net's address type re-exported for callers).
pub use embassy_net::IpAddress;

/// Addresses returned by a single lookup. Capacity is small and fixed; extra
/// records are dropped.
pub type Addresses = heapless::Vec<IpAddress, 4>;

pub trait Dns {
    type Error;

    /// Resolve `hostname` to zero or more addresses.
    fn resolve(
        &mut self,
        hostname: &str,
    ) -> impl Future<Output = Result<Addresses, Self::Error>>;
}
```

Notes:

- `hostname` is a parameter, not stored state. `DnsRuntime` no longer holds a
  hostname, and the closure no longer captures one. The single source of truth
  is the caller (`dns_tester::run`), which already owns `DNS_HOSTNAME`.
- Return the addresses, not a count or a bool. The tester needs only
  emptiness, but a real resolver is reusable and keeps `Ok(empty)` distinct
  from `Err`.
- `hostname(&self)` is removed from the trait. The tester supplies the
  hostname for both the query and the on-screen label.

### `DnsWithStack` — the one real resolver

Both ESP and RP hand the app an `embassy_net::Stack<'static>` and both resolve
via `stack.dns_query(host, A)`. That is the only per-platform line today, and
it is identical, because the type is identical. So it collapses into one
concrete struct in core:

```rust
use embassy_net::{Stack, dns::DnsQueryType};

/// A `Dns` resolver backed by an embassy-net stack. `Stack` is `Copy`, so this
/// holds it by value — no lifetime or `&mut` friction at the call site.
pub struct DnsWithStack<'a> {
    stack: Stack<'a>,
}

impl<'a> DnsWithStack<'a> {
    pub const fn new(stack: Stack<'a>) -> Self {
        Self { stack }
    }
}

impl Dns for DnsWithStack<'_> {
    type Error = embassy_net::dns::Error;

    async fn resolve(&mut self, hostname: &str) -> Result<Addresses, Self::Error> {
        self.stack.dns_query(hostname, DnsQueryType::A).await
    }
}
```

`dns_query` already returns `heapless::Vec<IpAddress, N>`; pick `Addresses`'
capacity to match embassy-net's so the value passes through with no conversion.
The `A` query type is fixed here (out of scope to parameterize).

The closure adapter `DnsRuntime` is deleted; nothing injects `stack.dns_query`
per platform anymore.

### `dns_tester::run` owns timing + verdict

`run` holds the hostname, times each lookup, derives the verdict, updates the
tally, and does any logging — once, not per platform.

```rust
const DNS_HOSTNAME: &str = "example.com";

// on TouchAction::StartDns:
let start = Instant::now();
let outcome = dns.resolve(DNS_HOSTNAME).await.map_err(Error::Dns)?;
let latency_millis = start.elapsed().as_millis();

queries = queries.saturating_add(1);
last_latency_millis = Some(latency_millis);
if outcome.is_empty() {
    failures = failures.saturating_add(1);
    status = Status::Fail;
} else {
    successes = successes.saturating_add(1);
    if matches!(status, Status::Tap) { status = Status::Ok; }
}
```

`DNS_HOSTNAME` moves into `dns_tester` (core), so the hostname label on screen
and the query use the same constant.

### Platform call sites collapse

ESP / RP — no closure, no timing, no verdict:

```rust
let mut dns = DnsWithStack::new(stack);
```

WASM — a mock trait impl, no stack, no fabricated latency (real elapsed time is
measured by `run`):

```rust
struct MockDns;

impl Dns for MockDns {
    type Error = core::convert::Infallible;
    async fn resolve(&mut self, _hostname: &str) -> Result<Addresses, Self::Error> {
        Ok(Addresses::from_slice(&[MOCK_ADDR]).unwrap())
    }
}
```

The host tests (`SuccessfulDns`, `CountingDns`) keep their own `Dns` impls,
updated to the new `resolve(&mut self, hostname)` signature and returning
`Addresses` instead of `DnsResult`.

## `DnsResult`

`DnsResult` is deleted. Timing and success are `run`'s local variables, not a
type crossing the boundary.

## Scope

In scope:

- `device-envoy-core/src/dns.rs`: reshape the `Dns` trait; delete `DnsRuntime`;
  add `DnsWithStack`; remove `DnsResult`; add `Addresses`/`IpAddress`.
- `device-envoy-examples-core/src/dns_tester.rs`: own `DNS_HOSTNAME`, timing,
  verdict, tally; drop the `hostname()`/`DnsResult` usage.
- ESP and RP call sites (examples + demos): replace the `DnsRuntime::new(...)`
  closure with `DnsWithStack::new(stack)`.
- WASM and `CydMemory` tests: update their `Dns` impls (or a small mock) to the
  new `resolve(&mut self, hostname)` signature returning `Addresses`.

Out of scope:

- The touch/orientation/UI loop and layout.
- Wi-Fi setup and the `Exit` handling.
- Query types beyond `A` (`DnsWithStack` hardcodes `A`; only the hostname is
  parameterized).

## Acceptance

- No platform closure measures time, computes a success bool, or builds a
  result struct.
- `DnsResult` no longer exists.
- The hostname is written once per role (the query and the label share
  `dns_tester::DNS_HOSTNAME`); no example states it twice.
- The WASM mock returns addresses only; its on-screen latency is real elapsed
  time from `run`.
- Existing `dns_tester` memory/behavior tests pass unchanged in intent.
