# Re-pinning the TEA specification

## When to re-pin

- Upstream OpenAPI or normative discovery/auth/collection/TEI files change in a
  way that affects consumer behavior.
- Security fixes in the pinned contract require adopting a newer commit.
- Scheduled drift CI reports a diff you decide to adopt.

Do **not** auto-follow `main`. Re-pin is an explicit version event.

## Steps

1. Choose upstream commit SHA from
   <https://github.com/CycloneDX/transparency-exchange-api>.
2. Update vendored files under `spec/vendor/` and refresh digests in
   `spec/PINNED.md` / `spec/manifest.json`.
3. Update `libtea_core::spec` constants (`COMMIT`, dates, version).
4. Adjust models / client behavior as required by the delta.
5. Update the "API operation matrix" in `docs/interoperability.md` and the "Specification baseline" in `docs/architecture.md`.
6. Run:
   - `cargo test --workspace --all-targets --locked`
   - `cargo run -p libtea-cli --locked -- conformance`
   - interop probes (`scripts/interop-probe.sh` or ignored live tests)
7. Bump crate versions if the public API or contract changes.
8. Update `CHANGELOG.md` with the new pin and breaking notes.

## Drift monitoring

`.github/workflows/spec-drift.yml` (scheduled / manual) fetches upstream tip,
diffs against the pinned OpenAPI copy, and uploads a diff artifact. It is
**not** required for merge.
