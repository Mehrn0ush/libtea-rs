#!/usr/bin/env bash
# Read-only TEA interop probe. Does not mutate server state.
#
# Runs only the CLI subcommands `spec`, `discover`, `product-release`, and
# `download`. Those issue HTTP GET (and POST /token only when TEA_API_KEY_* are
# set and the server answers with a Bearer challenge). Never PUT/PATCH/DELETE.
#
# Env:
#   TEA_BASE_URL          API root for product-release/download phases
#   TEA_TEI               Optional TEI for discover
#   TEA_WELL_KNOWN_URL    Optional `.well-known/tea` URL override for discover
#   TEA_PRODUCT_RELEASE   Optional product release UUID
#   TEA_ARTIFACT_UUID     Optional artifact UUID
#   TEA_ARTIFACT_REV      Optional artifact revision (default 1)
#   TEA_MEDIA_TYPE        Optional media type for download (required when the
#                         artifact has more than one format)
#   TEA_ALLOW_WEAK_CHECKSUMS  Set to 1 to accept SHA-1-only checksums
#                         (passes --allow-weak-checksums; recorded in results)
#   TEA_API_VERSION       Default 1.0.0
#   TEA_API_KEY_ID / TEA_API_KEY_SECRET  Optional credentials (env only;
#                         never passed on argv; redacted from result files)
#   TEA_ALLOW_ORIGIN      Optional single origin allowed to use HTTP and
#                         non-public addresses (local fixtures only; passed to
#                         the CLI as --allow-origin)
#   TEA_OUT_DIR           Default ./interop-results (relative to the caller's
#                         working directory). Created with mode 700 when absent.
#                         An existing directory's mode is left unchanged.
#   TEA_CLI_BIN           Prebuilt `libtea` binary. Required whenever
#                         TEA_API_KEY_ID or TEA_API_KEY_SECRET is set: with
#                         credentials in the environment the script refuses to
#                         run cargo at all (cargo runs third-party build
#                         scripts). Without credentials and without it, the
#                         script builds `libtea-cli` and uses
#                         ${CARGO_TARGET_DIR:-<repo>/target}/debug/libtea.
#
# Exit status: 0 when at least one operation ran and none failed; 1 when any
# operation failed; 2 on a setup error or when nothing was probed.
#
# Example (Vulnetix-shaped):
#   TEA_TEI='tei://vulnetix.com/uuid/71286087-babf-5795-a6ce-4c350bc944b4' \
#   TEA_BASE_URL='https://www.vulnetix.com/tea' \
#   TEA_PRODUCT_RELEASE='71286087-babf-5795-a6ce-4c350bc944b4' \
#   TEA_ARTIFACT_UUID='fbe825ce-1fe6-58b2-bf8d-25fdd47b5f13' \
#   TEA_MEDIA_TYPE='application/vnd.cyclonedx+json' \
#   ./scripts/interop-probe.sh

set -euo pipefail
# Do not enable `set -x`: it would echo env values including secrets.
umask 077
set -o noclobber

die() {
  printf 'interop-probe: %s\n' "$*" >&2
  exit 2
}

command -v python3 >/dev/null 2>&1 || die "python3 is required"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

OUT_DIR="${TEA_OUT_DIR:-./interop-results}"
if [[ -L "$OUT_DIR" ]]; then
  die "TEA_OUT_DIR must not be a symlink"
elif [[ -e "$OUT_DIR" ]]; then
  [[ -d "$OUT_DIR" ]] || die "TEA_OUT_DIR exists and is not a directory"
else
  # umask 077 makes every directory created here mode 700. Nothing else is chmodded.
  mkdir -p -- "$OUT_DIR" || die "cannot create TEA_OUT_DIR"
fi

if [[ -n "${TEA_CLI_BIN:-}" ]]; then
  CLI_BIN="$TEA_CLI_BIN"
elif [[ -n "${TEA_API_KEY_ID:-}" || -n "${TEA_API_KEY_SECRET:-}" ]]; then
  die "TEA_API_KEY_ID/TEA_API_KEY_SECRET is set but TEA_CLI_BIN is not: refusing to run cargo with credentials in the environment (build scripts of dependencies could read them). Build libtea-cli first, without credentials, and set TEA_CLI_BIN to the binary."
else
  (cd "$ROOT" && cargo build -q -p libtea-cli --locked) || die "cargo build -p libtea-cli failed"
  CLI_BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/libtea"
fi
[[ -x "$CLI_BIN" ]] || die "CLI binary not found or not executable"

# Read the version from the manifest: no cargo is needed (or allowed) here.
CLIENT_VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT/crates/tea-client/Cargo.toml" | head -n 1)"
[[ "$CLIENT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.+-]*)?$ ]] ||
  die "could not read the libtea-client version from crates/tea-client/Cargo.toml"

SPEC_JSON="$("$CLI_BIN" spec --json)" || die "\`libtea spec --json\` failed"
SPEC_COMMIT="$(printf '%s' "$SPEC_JSON" | python3 -I -c '
import json, re, sys
commit = json.load(sys.stdin).get("commit", "")
if not re.fullmatch(r"[0-9a-f]{40}", commit):
    sys.exit("spec --json did not report a 40-hex commit")
print(commit)
')" || die "could not read the pinned commit from \`libtea spec --json\`"

API_VERSION="${TEA_API_VERSION:-1.0.0}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
RESULT_JSON="$OUT_DIR/interop-${STAMP}.json"
RESULT_MD="$OUT_DIR/interop-${STAMP}.md"
[[ ! -e "$RESULT_JSON" && ! -e "$RESULT_MD" ]] || die "result files for $STAMP already exist"

# Redact secret substrings from captured CLI output before storage. The
# expansion is quoted so the secret is matched literally, not as a glob.
sanitize() {
  local text="$1"
  if [[ -n "${TEA_API_KEY_SECRET:-}" ]]; then
    text="${text//"$TEA_API_KEY_SECRET"/[redacted]}"
  fi
  if [[ -n "${TEA_API_KEY_ID:-}" ]]; then
    text="${text//"$TEA_API_KEY_ID"/[redacted-id]}"
  fi
  printf '%s' "$text"
}

ops=()
pass=0
fail=0

record() {
  local name="$1" status="$2" detail entry
  detail="$(sanitize "$3")"
  entry="$(printf '%s' "$detail" | python3 -I -c '
import json, sys
print(json.dumps({"operation": sys.argv[1], "status": sys.argv[2], "detail": sys.stdin.read()}))
' "$name" "$status")" || die "could not encode the result for $name"
  ops+=("$entry")
  if [[ "$status" == "pass" ]]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
  fi
}

# Credentials stay in the process environment for the CLI; never on argv.
origin_args=()
if [[ -n "${TEA_ALLOW_ORIGIN:-}" ]]; then
  origin_args=(--allow-origin "$TEA_ALLOW_ORIGIN")
fi

run_cli() {
  # `${arr[@]+...}` keeps bash 3.2 (macOS) happy under `set -u` with an empty array.
  "$CLI_BIN" "$@" ${origin_args[@]+"${origin_args[@]}"}
}

if [[ -n "${TEA_TEI:-}" ]]; then
  discover_args=(discover "$TEA_TEI")
  if [[ -n "${TEA_WELL_KNOWN_URL:-}" ]]; then
    discover_args+=(--well-known-url "$TEA_WELL_KNOWN_URL")
  fi
  if out="$(run_cli "${discover_args[@]}" 2>&1)"; then
    record "discover" "pass" "$out"
  else
    record "discover" "fail" "$out"
  fi
fi

if [[ -n "${TEA_BASE_URL:-}" && -n "${TEA_PRODUCT_RELEASE:-}" ]]; then
  if out="$(run_cli product-release --base "$TEA_BASE_URL" --api-version "$API_VERSION" --uuid "$TEA_PRODUCT_RELEASE" 2>&1)"; then
    record "product-release" "pass" "ok"
  else
    record "product-release" "fail" "$out"
  fi
fi

CHECKSUM="not_run"
ALLOW_WEAK="false"
if [[ "${TEA_ALLOW_WEAK_CHECKSUMS:-}" == "1" ]]; then
  ALLOW_WEAK="true"
fi
if [[ -n "${TEA_BASE_URL:-}" && -n "${TEA_ARTIFACT_UUID:-}" ]]; then
  # `download --output` refuses an existing path, so give it a fresh name
  # inside a private temporary directory.
  TMP_ART_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tea-probe.XXXXXX")"
  trap 'rm -rf "$TMP_ART_DIR"' EXIT
  TMP_ART="$TMP_ART_DIR/artifact"
  download_args=(download
    --base "$TEA_BASE_URL"
    --api-version "$API_VERSION"
    --artifact "$TEA_ARTIFACT_UUID"
    --revision "${TEA_ARTIFACT_REV:-1}"
    --require-checksums
    --output "$TMP_ART")
  if [[ -n "${TEA_MEDIA_TYPE:-}" ]]; then
    download_args+=(--media-type "$TEA_MEDIA_TYPE")
  fi
  if [[ "$ALLOW_WEAK" == "true" ]]; then
    download_args+=(--allow-weak-checksums)
  fi
  # Exit-status contract of `download --require-checksums` (no stderr prose
  # is parsed): 0 = checksums were declared and every one matched the decoded
  # bytes (with SHA-1-only matches counted only under --allow-weak-checksums);
  # 4 = every declared checksum matched but all are weak (SHA-1); anything
  # else = failure.
  status=0
  out="$(run_cli "${download_args[@]}" 2>&1)" || status=$?
  case "$status" in
    0)
      CHECKSUM="checksum_match_decoded_bytes"
      record "download" "pass" "declared checksums matched decoded bytes (not publisher authenticity)"
      ;;
    4)
      CHECKSUM="weak_algorithms_only"
      record "download" "fail" "only weak checksum algorithms (SHA-1) were declared; not Verified"
      ;;
    *)
      CHECKSUM="not_confirmed"
      record "download" "fail" "$out"
      ;;
  esac
fi

if [[ $((pass + fail)) -eq 0 ]]; then
  die "nothing was probed: set TEA_TEI, or TEA_BASE_URL with TEA_PRODUCT_RELEASE and/or TEA_ARTIFACT_UUID"
fi

if [[ -n "${TEA_API_KEY_ID:-}" && -n "${TEA_API_KEY_SECRET:-}" ]]; then
  AUTH_MODE="api_key_env"
else
  AUTH_MODE="anonymous"
fi

# Values reach python through the environment and stdin, never argv. Base URL
# userinfo, query, and fragment are dropped before they are written.
printf '%s\n' "${ops[@]}" | \
  PROBE_STAMP="$STAMP" \
  PROBE_CLIENT_VERSION="$CLIENT_VERSION" \
  PROBE_SPEC_COMMIT="$SPEC_COMMIT" \
  PROBE_BASE_URL="$(sanitize "${TEA_BASE_URL:-}")" \
  PROBE_TEI="$(sanitize "${TEA_TEI:-}")" \
  PROBE_AUTH_MODE="$AUTH_MODE" \
  PROBE_CHECKSUM="$CHECKSUM" \
  PROBE_ALLOW_WEAK="$ALLOW_WEAK" \
  PROBE_PASS="$pass" \
  PROBE_FAIL="$fail" \
  PROBE_JSON="$RESULT_JSON" \
  PROBE_MD="$RESULT_MD" \
  python3 -I -c '
import json, os, sys
from urllib.parse import urlsplit, urlunsplit

def clean_url(text):
    if not text:
        return ""
    try:
        parts = urlsplit(text)
        host = parts.hostname or ""
        if ":" in host:
            host = "[" + host + "]"
        port = parts.port
    except ValueError:
        return "<unparseable>"
    netloc = host + (":" + str(port) if port else "")
    return urlunsplit((parts.scheme, netloc, parts.path, "", ""))

env = os.environ
ops = [json.loads(line) for line in sys.stdin if line.strip()]
result = {
    "timestamp_utc": env["PROBE_STAMP"],
    "client_version": env["PROBE_CLIENT_VERSION"],
    "spec_commit": env["PROBE_SPEC_COMMIT"],
    "server_base_url": clean_url(env["PROBE_BASE_URL"]),
    "tei": env["PROBE_TEI"],
    "auth_mode": env["PROBE_AUTH_MODE"],
    "checksum_result": env["PROBE_CHECKSUM"],
    "allow_weak_checksums": env["PROBE_ALLOW_WEAK"] == "true",
    "pass": int(env["PROBE_PASS"]),
    "fail": int(env["PROBE_FAIL"]),
    "operations": ops,
}
with open(env["PROBE_JSON"], "x", encoding="utf-8") as out:
    json.dump(result, out, indent=2)
    out.write("\n")
lines = [
    "# Interop probe " + result["timestamp_utc"],
    "",
    "- client_version: " + result["client_version"],
    "- spec_commit: " + result["spec_commit"],
    "- server_base_url: " + result["server_base_url"],
    "- auth_mode: " + result["auth_mode"],
    "- checksum_result: " + result["checksum_result"]
    + " (decoded-byte digests only; not authenticity)",
    "- allow_weak_checksums: " + str(result["allow_weak_checksums"]).lower(),
    "- pass/fail: %d / %d" % (result["pass"], result["fail"]),
    "",
    "Results are evidence for the probed deployment only, not ecosystem-wide",
    "TEA compatibility.",
]
with open(env["PROBE_MD"], "x", encoding="utf-8") as out:
    out.write("\n".join(lines) + "\n")
' || die "could not write result files"

echo "Wrote $RESULT_JSON"
echo "Wrote $RESULT_MD"
[[ "$fail" -eq 0 ]]
