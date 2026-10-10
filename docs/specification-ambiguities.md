# Specification ambiguities

Pinned revision: `962a0228af1bfcc1232ac3f79239317ff33fdb04` (retrieved 2026-10-10).

## README status versus OpenAPI version

The repository README says the standard is "Beta 2". `spec/openapi.yaml` `info.version` is `1.0.0`, and the discovery examples advertise `"1.0.0"`.

This SDK implements OpenAPI `1.0.0` from the pinned commit. The README status line is treated as documentation drift, not as an instruction to implement an older beta.

## Error codes are closed in OpenAPI and open in the discovery prose

`error-response` lists a closed `unknown-error-type` enum and sets `additionalProperties: false`. The discovery document says a client shall not reject a `404` because it does not know the `error` string, so that a later revision can extend the enum without turning those responses into failover triggers.

Interpretation used: recognition follows the prose. A JSON object with a string `error` property and `Content-Type: application/json` is a TEA error even when the code is unknown or extra properties are present. Unknown codes are preserved. They do not cause failover on `/discovery`.

Consequence: a server that adds a field next to `error` is still treated as terminal for discovery. A strict schema validator would reject that body and might fail over. This client does not.

## Failover order

Endpoint selection must prefer the highest mutually supported SemVer version, then the higher `priority`. The failover section says the client should preserve priority order from highest to lowest while choosing the next untried compatible endpoint.

Interpretation used: the attempt list is sorted by version precedence descending, then priority descending, then document order. Failover walks that list. Priority does not override a higher mutually supported version.

## Query encoding of `tei` and `purl`

The discovery prose example leaves `/` unencoded in the `tei` query value (`tei%3A//products.example.com/...`). The OpenAPI description's example percent-encodes the whole identifier (`tei%3A%2F%2F...`) and says clients apply query serialization once. Servers must decode once.

Interpretation used: the client percent-encodes every octet that is not unreserved, once, which matches the OpenAPI example. A server that decodes the query component once accepts both spellings. A server that decodes twice would corrupt an OpenAPI-style value.

## Versioned base versus OpenAPI server URL

OpenAPI paths such as `/discovery` and `/token` are relative to a server URL that already includes `/v1.0.0` in the example server entry. The discovery document says to append `/v` and the advertised version string to `rootUrl`.

Interpretation used: `rootUrl` has no version prefix. Operation URLs are `{rootUrl}/v{advertised}/` + the OpenAPI path. `+` in build metadata is left unencoded in that path.

## Collection and artifact version minima

The prose says collection and artifact versions start at 1. The schemas type those fields as integers and do not declare `minimum: 1`.

Interpretation used: zero and negative values are rejected as non-conforming.

## HTTP in well-known documents

The JSON Schema pattern allows `http` so local documents still validate. The discovery prose says a client shall reject an `http` base unless that specific base was explicitly configured for local testing.

Interpretation used: documents may parse with `http` URLs. The default destination policy refuses to connect to them. `DestinationPolicy::allow_origin` permits one configured origin.

## Trailing dot on TEI domains

Normative examples use a DNS name without a trailing root dot. Comparison is exact, so `example.com` and `example.com.` would be different spellings if both were accepted.

Interpretation used: a trailing dot is rejected. This is recorded because the prose does not say so in those words.

## Signatures document

`signatures/signature.md` says it is work in progress and not part of the TEA standard. It mentions `/trust-anchors/`, which is absent from `spec/openapi.yaml`.

Interpretation used: signature download endpoints from the OpenAPI document are implemented. Trust-anchor distribution is not.
