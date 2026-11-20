# The 0.1.8 CLI commands against public servers, 2026-11-20

One run of the commands added in 0.1.8 (`inspect`, `search`, `discover --purl`,
`cle`) against the Vulnetix and ReARM demo deployments. Read-only and
anonymous. This is evidence for those two deployments on that day. It is not
evidence for those servers in general, and it is not a conformance result.

| Field | Value |
| --- | --- |
| Run | `Mehrn0ush/libtea-action-test` workflow run 44662015332 (`.github/workflows/new-commands.yml` at `dd9d9350866aea8095bee4b3f2bdc3f938a7c1cf`, pull request 1), 2026-11-20 02:36 UTC |
| Install | The Action `Mehrn0ush/libtea-rs@fce7a0150f9dd4f6eaeba28727651776e132258d` (`v0.1.8`), `version: 0.1.8`, `attestation: required`. Each job printed "attestation verified" and `libtea 0.1.8`; the commands then ran the installed binary directly, because the Action does not accept them as `command` |
| Runners | `ubuntu-24.04` (image 20261111.127.1 and 20261111.370.1), `macos-15` (arm64, 20261118.0550.1), `windows-2022` (20261112.366.7): six jobs, both servers on each |
| Servers | Vulnetix `https://www.vulnetix.com/tea`, TEI `tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4`; ReARM demo `https://demo.rearmhq.com/tea`, TEI `tei://demo.rearmhq.com/uuid/b9f115fb-6688-4a3f-a262-3acc04e1fc9e` |
| Spec commit | `962a0228af1bfcc1232ac3f79239317ff33fdb04` |
| Auth | Anonymous |

The subjects (product, release, first component and its release, and the
PURLs) were read from `inspect --json` and from the `search` output, not
chosen by hand. The workflow records every exit code and fails only on exit
2 (a bad argument); it passed on all six jobs. Results were the same on the
three runners for each server.

## Results

| Command | Vulnetix | ReARM demo |
| --- | --- | --- |
| `inspect <tei>`, `--json` and text | exit 0 | exit 0; 10 components, 2 without a pinned release |
| `search products`, `product-releases`, `components`, `component-releases`, `--page-size 2` | exit 0 for each | exit 0 for each |
| `search products --id-type TEI --id-value <tei>` | exit 0, no results | exit 0, no results |
| `search product-releases --id-type TEI --id-value <tei>` | exit 0, 1 result: the TEI's release | exit 0, 1 result: the TEI's release |
| `search products --id-type PURL --id-value <product PURL>` | exit 0, 1 result | exit 0, 1 result |
| `search product-releases --id-type PURL --id-value <release PURL>` | exit 0 | exit 0 |
| `discover --purl <product PURL>` (no version) | exit 0, one product release, `3021f133-9560-522e-b8c3-37719837fcd2`, not the TEI's release | **exit 5**, `OBJECT_UNKNOWN` |
| `discover --purl <release PURL>` (with version) | exit 0, the TEI's release `71286087-babf-5795-a6ce-4c350bc944b4` | exit 0, the TEI's release `b9f115fb-6688-4a3f-a262-3acc04e1fc9e` |
| `cle --product`, `--product-release`, `--component`, `--component-release` | exit 0, `implemented: true` for each; product and component 163 events each (82 `released`, 81 `supersededBy`), each release 2 (`released`, `supersededBy`) | exit 0, `implemented: true` for each; 1 `released` event in each document |
| `cle --product <uuid> --version <release version>` | not run: the version `image-ee234cacfad1` is not SemVer | exit 0 for `26.11.17`: `released` `2026-11-03T23:57:40Z`, no other milestone, nothing unevaluated |

The PURLs: Vulnetix product `pkg:generic/vulnetix/pix-sandbox`, release
`pkg:generic/vulnetix/pix-sandbox@image-ee234cacfad1`; ReARM product
`pkg:github/relizaio/rearm`, release `pkg:github/relizaio/rearm@26.11.17`.

## What this shows, and what it does not

- Both servers serve CLE documents for all four subjects, and the CLI parses
  them, including Vulnetix's 163 events. `Cle::status_for_semver` ran on one
  real document, ReARM's, which holds a single `released` event. Vulnetix's
  versions are not SemVer, so its `supersededBy` events were parsed but not
  evaluated. The other milestones (end of support, end of life and so on) are
  still tested only with hand-built documents.
- An unversioned PURL is resolved differently by the two servers. ReARM
  answers `OBJECT_UNKNOWN`. Vulnetix returns one release, not the TEI's. A
  release's own (versioned) PURL resolved to that release on both. The
  specification does not say which releases a server returns for a PURL
  without a version, so neither answer is a client or a server error.
- The filtered searches were checked for exit status and result count only.
  The returned objects were not compared field by field.
- Search by `CPE` and `COMPLIANCE_DOCUMENT`, paging with `--page-token`,
  `--sort-field`, `inspect --collection-version` and credentials were not
  exercised.
- The run's log on GitHub is the source, and it expires with the log
  retention of `Mehrn0ush/libtea-action-test`.
