# Authentication and token refresh

## Baseline

`libtea-client` (sources under `crates/tea-client/`) implements the TEA API-key
baseline:

1. `POST /token` with HTTP Basic (form-encoded id and secret) and
   `grant_type=client_credentials`.
2. Resource calls use `Authorization: Bearer <access_token>`.
3. The access token is not inspected beyond storage and presentation.

Other OAuth profiles (SAML, JWT bearer, token exchange, mutual TLS) are
**not** implemented and not planned until an interoperability need appears.

## Single-flight refresh

Each `Client` keeps **one async lock per API origin** (`scheme://host:port`)
around `POST /token`. Refreshes for the same origin serialize on that lock.
Refreshes for different origins on the same `Client` do **not** wait for each
other.

After acquiring the lock, a waiter:

1. re-checks the bearer cache. If a usable bearer is stored and differs from
   the value that just received `401` (or no bearer was presented), it reuses
   that bearer and skips `POST /token`;
2. otherwise, if an exchange for this origin **finished while the waiter was
   queued** and that exchange failed, it returns the same failure (same
   `ClientError` variant; `Api` loses its free-text `message`) without sending
   another `POST /token`;
3. otherwise it performs its own exchange.

### Asserted bound

When all callers share one `Client` and challenge the **same** API origin with
a resource `401` while the bearer cache for that origin holds no usable token,
the number of `POST /token` requests for the burst is **1** if either:

- the first exchange **succeeds** and the stored token is not already expired
  when each waiter re-checks the cache; or
- the first exchange **fails** and every caller in the burst reached the
  refresh lock before that exchange finished.

Covered by:

- `concurrent_n32_challenges_one_token_exchange` (`N = 32`, barrier, 200 ms
  token delay, success);
- `concurrent_challenges_share_one_token_exchange` (success);
- `hanging_token_origin_does_not_stall_waiters_or_other_origins` (4 waiters,
  hanging `/token`, exactly 1 POST, every waiter gets `ClientError::Timeout`).

This is a per-burst bound, not an "exactly one exchange for the process
lifetime" guarantee.

### When additional exchanges happen

- A waiter that finds a just-stored token already expired (for example a very
  short `expires_in`, or `expires_in: 0`) exchanges again.
- The only cached bearer is the value that just received `401`; it must be
  replaced.
- A call that starts **after** a failed exchange has finished retries; the
  shared failure only covers waiters that queued during that exchange
  (`a_later_call_retries_after_a_shared_token_failure`).
- The task performing an exchange is cancelled (dropped) before the exchange
  finishes. Nothing is recorded, and the next waiter performs its own
  exchange (`a_cancelled_refresh_does_not_wedge_the_next_caller`).
- A different origin needs its own token.
- A later resource `401` after a successful refresh starts a new refresh cycle.

### Lock lifetime

An origin's lock entry is created by the first caller that needs a refresh and
removed by the last caller that holds it, including a caller that is
cancelled. The map therefore holds only origins with a refresh running or
queued, however many origins a `Client` talks to
(`refresh_slots_do_not_grow_with_origin_churn`,
`a_cancelled_refresh_leaves_no_slot`). A stored failure lives in that entry,
so it is never handed to a later burst
(`shared_failure_does_not_outlive_its_burst`).

## Timeouts and stalls

`POST /token` uses the same HTTP client limits as every other request:
`Limits::request_timeout` (default 30 s, total per request) and
`Limits::connect_timeout` (default 10 s). Token requests are not retried.
A hanging token endpoint therefore fails with `ClientError::Timeout` after
about `request_timeout` (`hanging_token_endpoint_times_out_without_the_secret`).

- Waiters for the **same** origin that queued during that exchange fail at the
  same time with `Timeout`; they do not each wait a further `request_timeout`.
- Other origins on the same `Client` are not blocked by it.

The lock is per origin, not client-wide, so a hanging origin does not stall
requests to a healthy one (`hanging_token_origin_does_not_stall_waiters_or_other_origins`).

## Secrets

- Prefer `TEA_API_KEY_ID` / `TEA_API_KEY_SECRET` (CLI) or `ApiKey` constructed in
  process memory.
- Do not log secrets. `Debug` for `ApiKey` redacts the secret. Token error paths
  are tested not to echo the secret (`token_error_text_omits_the_secret`,
  `hanging_token_endpoint_times_out_without_the_secret`). A server that puts
  the secret into its own `error` field is echoed verbatim in
  `ClientError::Token`; the server already holds the secret in that case.
