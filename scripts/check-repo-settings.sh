#!/usr/bin/env bash
# Check that the repository settings the release process depends on exist.
#
#   check-repo-settings.sh [OWNER/REPO [BRANCH]]
#
# Reads two things through `gh api` (read-only, uses GH_TOKEN):
#   * the rules that apply to BRANCH (default main): a pull request is required,
#     the CI jobs below are required status checks, and force pushes and
#     deletion are blocked;
#   * the repository rulesets: an active tag ruleset covering `v*` that blocks
#     creation by others, updates, deletion and force pushes.
#
# A third read, repos/OWNER/REPO/actions/permissions, needs admin rights. When
# the token may not read it (HTTP 403 or 404, as with the workflow's
# GITHUB_TOKEN) the script says so and goes on: that setting is neither passed
# nor failed. If it can be read, "local_only" is a problem, because it makes every
# workflow fail with startup_failure before any job starts.
#
# It reports every problem it finds and exits 1 if there is any. It cannot
# protect anything itself: it only says whether the settings are there. Rules
# and rulesets are only available to private repositories on a paid plan; on a
# public repository they are free.
#
# The names in REQUIRED_CHECKS are the job names in .github/workflows/ci.yml.
# `cargo audit` is left out on purpose: that job is continue-on-error.

set -euo pipefail

die() {
  printf 'check-repo-settings: %s\n' "$*" >&2
  exit 2
}

repo="${1:-${GITHUB_REPOSITORY:-}}"
branch="${2:-main}"
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die "usage: check-repo-settings.sh OWNER/REPO [BRANCH]"
[[ "$branch" =~ ^[A-Za-z0-9._/-]+$ ]] || die "bad branch name"
command -v gh >/dev/null 2>&1 || die "gh is required"
command -v python3 >/dev/null 2>&1 || die "python3 is required"

REQUIRED_CHECKS="rust stable|rust 1.89.0|package|cargo deny|cargo semver-checks|non_exhaustive guards|tests on windows-2022|tests on macos-15"

work="$(mktemp -d "${TMPDIR:-/tmp}/libtea-settings.XXXXXX")"
trap 'rm -rf "$work"' EXIT

gh api "repos/$repo/rules/branches/$branch" >"$work/branch.json" ||
  die "cannot read the rules for $branch (is the repository public, and may the token read it?)"
gh api "repos/$repo/rulesets" >"$work/rulesets.json" ||
  die "cannot list the repository rulesets"

if gh api "repos/$repo/actions/permissions" >"$work/actions.json" 2>"$work/actions.err"; then
  :
elif grep -Eq 'HTTP (403|404)' "$work/actions.err"; then
  rm -f "$work/actions.json"
  printf 'note: cannot check the Actions permissions here (reading them needs admin rights); not verified\n' >&2
else
  die "cannot read the Actions permissions: $(head -c 200 "$work/actions.err")"
fi

python3 -I - "$work/rulesets.json" >"$work/tag-ids" <<'PY'
import json, sys
for ruleset in json.load(open(sys.argv[1], encoding="utf-8")):
    if ruleset.get("target") == "tag":
        print(int(ruleset["id"]))
PY
while IFS= read -r id; do
  [[ "$id" =~ ^[0-9]+$ ]] || continue
  gh api "repos/$repo/rulesets/$id" >"$work/tag-$id.json" ||
    die "cannot read ruleset $id"
done <"$work/tag-ids"

REQUIRED_CHECKS="$REQUIRED_CHECKS" python3 -I - "$work" "$branch" <<'PY'
import glob, json, os, sys

work, branch = sys.argv[1], sys.argv[2]
required = set(os.environ["REQUIRED_CHECKS"].split("|"))
problems = []

rules = json.load(open(os.path.join(work, "branch.json"), encoding="utf-8"))
types = {rule.get("type") for rule in rules}
if not rules:
    problems.append(f"no rules apply to {branch}")
if "pull_request" not in types:
    problems.append(f"{branch} does not require a pull request")
if "non_fast_forward" not in types:
    problems.append(f"force pushes to {branch} are not blocked")
if "deletion" not in types:
    problems.append(f"deleting {branch} is not blocked")
contexts = set()
for rule in rules:
    if rule.get("type") == "required_status_checks":
        for check in rule.get("parameters", {}).get("required_status_checks", []):
            contexts.add(check.get("context"))
if "required_status_checks" not in types:
    problems.append(f"{branch} does not require status checks")
missing = sorted(required - contexts)
if missing and "required_status_checks" in types:
    problems.append("required status checks are missing: " + ", ".join(missing))

actions_checked = os.path.exists(os.path.join(work, "actions.json"))
if actions_checked:
    allowed = json.load(open(os.path.join(work, "actions.json"), encoding="utf-8")).get(
        "allowed_actions"
    )
    if allowed == "local_only":
        problems.append(
            'Actions allow only "local_only" (this repository\'s own actions and workflows): '
            "every workflow fails with startup_failure before any job starts. "
            'Set Settings, Actions, General to "Allow all actions and reusable workflows"'
        )

need_tag_rules = {"creation", "update", "deletion", "non_fast_forward"}
good_tag_ruleset = False
for path in glob.glob(os.path.join(work, "tag-*.json")):
    ruleset = json.load(open(path, encoding="utf-8"))
    include = set(ruleset.get("conditions", {}).get("ref_name", {}).get("include", []))
    covers = bool(include & {"refs/tags/v*", "~ALL"})
    has = {rule.get("type") for rule in ruleset.get("rules", [])}
    if ruleset.get("enforcement") == "active" and covers and need_tag_rules <= has:
        good_tag_ruleset = True
if not good_tag_ruleset:
    problems.append(
        "no active tag ruleset covers v* and blocks creation, update, deletion and force pushes"
    )

if problems:
    for problem in problems:
        print(f"PROBLEM: {problem}", file=sys.stderr)
    sys.exit(1)
print("repository settings: ok" if actions_checked else "repository settings: ok (Actions permissions not verified)")
PY
