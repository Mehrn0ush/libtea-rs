# Interoperability plan

Date: 2026-10-10. This file is the survey from that date. Current Oolong results are in [interoperability.md](interoperability.md). The paragraphs below were true on 2026-10-10: no TEA server had been driven by this SDK yet, and the notes are documents and README text, not the later client/server runs.

The pinned contract is OpenAPI `1.0.0` at commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`. A server that answers this client is not, by that fact, a conforming TEA server.

## What each named implementation says

Sources are the pinned `doc/tea-implementations.md` plus README files fetched on 2026-10-10. Where a README fetch failed, the version is unknown.

| Implementation | Role | Version claim found | SDK test |
| --- | --- | --- | --- |
| py-libtea (`sbomify/py-libtea`) | Python client | `main/README.md` returned HTTP 404. Version unknown | Not executed |
| pypi-tea (`sbomify/pypi-tea`) | Registry bridge | `main/README.md` returned HTTP 404. Version unknown | Not executed |
| ReARM CLI | Client | Spec list does not give a TEA version. `docs/tea.md` on `main` was empty in the fetch used here | Not executed |
| Vulnetix CLI | Client and producer | Pinned implementations doc says TEA v0.4.0 | Not executed. 0.4.0 is not this pin |
| Oolong (`relizaio/oolong`) | Server | README describes a lightweight TEA server and an OpenAPI generator command. It does not state `info.version`. Demo host `oolong-demo.rearmhq.com` did not resolve | Not executed |
| ReARM (`relizaio/rearm`) | Server | README says "TEA v 0.4.0" and "most of TEA v 0.4.0" | Not executed |
| sbomify (`sbomify/sbomify`) | Server | README says Transparency Exchange API v0.3.0-beta.2 | Not executed |
| Cybeats demonstration | Server | Pinned doc links `https://us.sbom.cybeats.com/.well-known/tea` | Document fetched. SDK not run |

## Cybeats well-known document

`GET https://us.sbom.cybeats.com/.well-known/tea` returned HTTP 200 and this body:

```json
{"endpoints":[{"url":"https://api-tea.us.services.cybeats.com","versions":["1"]}],"schemaVersion":1}
```

`schemaVersion` 1 matches the well-known schema. `versions: ["1"]` is not SemVer `MAJOR.MINOR.PATCH`. The discovery document at this pin requires full SemVer 2.0.0. `ApiVersion::parse` uses `semver::Version::parse`, which rejects `1`. `WellKnownDocument::parse` fails the document when any version string fails. This client would refuse the document. That refusal matches the pinned spec. It also means this demonstration endpoint is not a 1.0.0 interoperability target until it advertises a SemVer version such as `1.0.0`.

This prediction was not confirmed by calling `Client::discover`. Doing so would also require the default destination policy to allow that public HTTPS host, which it would, and would then fail at JSON version parsing before `/discovery`.

## Oolong demo

`https://oolong-demo.rearmhq.com/.well-known/tea` failed DNS resolution from this machine (`curl` exit 6). The Oolong README still shows TEI values in an older `urn:tei:` shape (`urn:tei:uuid:...`). This SDK accepts only `tei://`. Those README examples are not valid input for `Tei::parse`.

## Known client behaviors that will disagree with some peers

| Behavior | This client | Likely peer effect |
| --- | --- | --- |
| Query encoding | `/` in `tei` becomes `%2F` | A server that decodes the query once should accept it. A server written to the discovery prose example may still accept it. Not tested |
| API version path | Exact advertised string, `+` left unencoded | Peers that only route `/v1.0.0` will not see build metadata. That matches the spec |
| Version `1` or `0.4.0` | Rejected or not in `SupportedApi::tea_1_0_0` | ReARM 0.4.0, sbomify 0.3.0-beta.2, and the Cybeats `1` document are outside this client's default support set |
| `deflate` | On 2026-10-10 the decoder was raw DEFLATE (TEA-AUD-001). The client now uses the zlib wrapper for HTTP `deflate` | A peer that sends raw DEFLATE will fail decoding. A peer that sends zlib-wrapped `deflate` matches the current client |
| HTTP and private addresses | Refused unless that origin is excepted | Local demo servers on `127.0.0.1` need `DestinationPolicy::allow_origin` |
| Credentials | Not posted to a discovered host unless `allow_discovered_credentials` is set | A server that requires a token on the first well-known hop will fail closed |

## Tests to run later

Do not treat a skipped test as a pass. Gate each one on an environment variable so `cargo test` stays offline.

1. `TEA_INTEROP_WELL_KNOWN` — fetch that URL with the production policy, parse it, and record `schemaVersion`, endpoint URLs, and version strings. Fail only on transport errors. Print versions that `ApiVersion::parse` rejects. Do not call them conformant.
2. `TEA_INTEROP_TEI` — `Client::discover` against a TEI whose operator has published a 1.0.0 well-known document. Record the selected root and version. Do not send an API key unless `TEA_INTEROP_API_KEY_ID` and `TEA_INTEROP_API_KEY_SECRET` are both set.
3. Artifact — `download_format` for one media type and require `IntegrityOutcome::Verified`. HTTP `deflate` is now zlib (TEA-AUD-001, verified). A server that sends raw DEFLATE will fail decoding.
4. Negative — a TEA `404` with `OBJECT_UNKNOWN` must be `ClientError::Unresolved` and must not be described as an empty update set.

Suggested first targets, after their operators confirm a 1.0.0 deployment:

- A local Oolong or ReARM built from a tree that vendors this OpenAPI commit, with `allow_origin` for that local origin only.
- Cybeats only after `versions` contains a SemVer string this client supports.

No such confirmation was obtained in this audit.
