# Third-party interop report template

Copy this file, fill it in, and open a PR or issue. Reports document **named**
deployments only. They are not CycloneDX certification and not a claim of
universal TEA compatibility.

## Environment

| Field | Value |
| --- | --- |
| Date (UTC) | |
| libtea-client version | |
| Spec commit (`libtea` `spec` / PINNED) | |
| Server product / name | |
| Server base URL | |
| TEI(s) exercised | |
| Auth mode | anonymous / API key |

## Operations probed

| Operation | Result (pass/fail) | Notes |
| --- | --- | --- |
| discover | | |
| get product release | | |
| list collections | | |
| download artifact | | |
| checksum outcome | Verified / other | |

## Limits of this report

- Evidence for the named deployment above only.
- Does not certify the server or the client.
- Checksum Verified ≠ publisher authenticity.
