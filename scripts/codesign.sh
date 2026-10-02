#!/usr/bin/env bash
#
# TransClip code-signing helper: one fixed, self-signed signing identity.
#
# Why this exists
#   macOS remembers Accessibility / Input Monitoring / Keychain grants by an app's code-signing
#   "designated requirement". The ad-hoc signature we used to ship makes that requirement the
#   hash of the binary (cdhash), so every release looked like a different app and every update
#   reset the user's permissions. Signing every release with ONE certificate pins the requirement
#
#       identifier "com.transclip" and certificate root = H"<sha1 of that certificate>"
#
#   to a value that is identical across versions, so grants survive updates and re-installs.
#
#   The certificate is self-signed: Gatekeeper does not trust it (first install still needs
#   `xattr -cr`) and end-user machines never need to trust it, because TCC only compares the
#   requirement string. Only the CI runner has to trust it, because `codesign` refuses
#   identities that are not trusted for code signing.
#
# Usage
#   scripts/codesign.sh setup [--out-dir DIR] [--force]   one-time: create the identity
#   scripts/codesign.sh import                            CI: import it into a temporary keychain
#   scripts/codesign.sh verify <TransClip.app | *.tar.gz> fail unless the requirement matches the pin
#
# Files
#   $DIR/transclip-codesign.p12        private key + certificate (secret, back it up)
#   $DIR/transclip-codesign.p12.pass   its password (secret, back it up)
#   .github/codesign-cert.sha1         SHA-1 of the certificate (public, committed; the "pin")
#
# NEVER change the bundle identifier or this certificate after a signed release has shipped:
# every user would have to re-grant Accessibility again.

set -euo pipefail

BUNDLE_ID="com.transclip"
IDENTITY_CN="TransClip Signing"
# LibreSSL: its PKCS#12 output imports into `security`; Homebrew's OpenSSL 3 needs -legacy.
OPENSSL="/usr/bin/openssl"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PIN_FILE="${CODESIGN_PIN_FILE:-$ROOT/.github/codesign-cert.sha1}"

WORK=""
cleanup() {
  if [[ -n "$WORK" && -d "$WORK" ]]; then
    rm -rf "$WORK"
  fi
}
trap cleanup EXIT

die() { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }

usage() {
  cat <<'EOF'
Usage:
  scripts/codesign.sh setup [--out-dir DIR] [--force]   create the signing identity (once)
  scripts/codesign.sh import                            CI: import $CODESIGN_P12_BASE64 / $CODESIGN_P12_PASSWORD
  scripts/codesign.sh verify <app | tar.gz>             check the signature against .github/codesign-cert.sha1
EOF
}

require_macos() {
  [[ "$(uname -s)" == "Darwin" ]] || die "this script only runs on macOS"
}

normalize_requirement() {
  printf '%s' "$1" | tr 'A-Z' 'a-z' | tr -d '"' | tr -s ' '
}

# Prints the pinned certificate SHA-1 (lowercase, 40 hex chars).
read_pin() {
  [[ -f "$PIN_FILE" ]] || die "missing $PIN_FILE (create it with: scripts/codesign.sh setup)"
  local sha
  sha="$(tr -d '[:space:]' < "$PIN_FILE" | tr 'A-F' 'a-f')"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || die "$PIN_FILE must contain the 40-hex SHA-1 of the signing certificate"
  printf '%s' "$sha"
}

# ---------------------------------------------------------------------------
# setup: create key + self-signed certificate, export PKCS#12, write the pin
# ---------------------------------------------------------------------------
cmd_setup() {
  local out_dir="$HOME/.tauri" force=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --out-dir) out_dir="${2:?--out-dir needs a value}"; shift 2 ;;
      --force) force=1; shift ;;
      *) die "unknown option: $1" ;;
    esac
  done
  require_macos
  [[ -x "$OPENSSL" ]] || die "$OPENSSL not found"

  local p12="$out_dir/transclip-codesign.p12"
  local pass_file="$out_dir/transclip-codesign.p12.pass"
  if [[ ( -e "$p12" || -e "$pass_file" ) && "$force" -eq 0 ]]; then
    die "$p12 already exists.
The signing identity must stay the same for every release (a new one makes every user
re-grant permissions). Use --force only before the first signed release has shipped."
  fi

  umask 077
  WORK="$(mktemp -d)"

  info "Generating a self-signed code-signing certificate (\"$IDENTITY_CN\", RSA 2048, ~20 years)"
  cat > "$WORK/openssl.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = v3
prompt = no
[dn]
CN = $IDENTITY_CN
O = TransClip
[v3]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
subjectKeyIdentifier = hash
EOF
  local log
  if ! log="$("$OPENSSL" req -new -newkey rsa:2048 -nodes -x509 -sha256 -days 7300 \
      -config "$WORK/openssl.cnf" -keyout "$WORK/key.pem" -out "$WORK/cert.pem" 2>&1)"; then
    echo "$log" >&2
    die "openssl could not create the certificate"
  fi

  local sha1 pass
  sha1="$("$OPENSSL" x509 -in "$WORK/cert.pem" -noout -fingerprint -sha1 | sed 's/.*=//; s/://g' | tr 'A-F' 'a-f')"
  [[ "$sha1" =~ ^[0-9a-f]{40}$ ]] || die "could not read the certificate fingerprint"
  pass="$("$OPENSSL" rand -base64 24 | tr -d '\n')"
  P12_PASSWORD="$pass" "$OPENSSL" pkcs12 -export -inkey "$WORK/key.pem" -in "$WORK/cert.pem" \
    -name "$IDENTITY_CN" -out "$WORK/id.p12" -passout env:P12_PASSWORD

  info "Self-test: importing the PKCS#12 into a throwaway keychain (the same step CI runs)"
  local kc="$WORK/selftest.keychain-db" kcpass
  kcpass="$(uuidgen)"
  security create-keychain -p "$kcpass" "$kc"
  security unlock-keychain -p "$kcpass" "$kc"
  security import "$WORK/id.p12" -k "$kc" -P "$pass" -T /usr/bin/codesign > /dev/null \
    || die "'security import' rejected the PKCS#12"
  security find-identity -p codesigning "$kc" | grep -qi "$sha1" \
    || die "the identity was not found in the keychain after import"
  security delete-keychain "$kc" > /dev/null 2>&1 || true

  mkdir -p "$out_dir" "$(dirname "$PIN_FILE")"
  install -m 600 "$WORK/id.p12" "$p12"
  printf '%s' "$pass" > "$pass_file"
  chmod 600 "$pass_file"
  printf '%s\n' "$sha1" > "$PIN_FILE"
  chmod 644 "$PIN_FILE"

  local not_after
  not_after="$("$OPENSSL" x509 -in "$WORK/cert.pem" -noout -enddate | sed 's/notAfter=//')"
  cat <<EOF

Created the TransClip signing identity.

  PKCS#12   $p12   (private key + certificate, chmod 600)
  Password  $pass_file   (chmod 600)
  Pin       ${PIN_FILE#"$ROOT"/}   (public, commit it)
  SHA-1     $sha1
  Expires   $not_after

Every release will carry this designated requirement:
  identifier "$BUNDLE_ID" and certificate root = H"$sha1"

Next steps
  1. BACK UP both files in $out_dir (password manager or encrypted storage).
     If they are lost the identity cannot be recreated and every user has to re-grant
     Accessibility once more.
  2. Register the CI secrets (run from the repository):

       base64 < "$p12" | tr -d '\n' | gh secret set CODESIGN_P12_BASE64
       gh secret set CODESIGN_P12_PASSWORD < "$pass_file"

  3. Commit ${PIN_FILE#"$ROOT"/}, then run the release workflow manually once
     (Actions > release > Run workflow, or: gh workflow run release.yml) to dry-run signing.
EOF
}

# ---------------------------------------------------------------------------
# import (CI): decode the PKCS#12 secret into a temporary keychain
# ---------------------------------------------------------------------------
cmd_import() {
  require_macos
  : "${CODESIGN_P12_BASE64:?CODESIGN_P12_BASE64 is empty}"
  : "${CODESIGN_P12_PASSWORD:?CODESIGN_P12_PASSWORD is empty}"

  local tmp="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
  local kc="${CODESIGN_KEYCHAIN_PATH:-$tmp/transclip-signing.keychain-db}"
  local p12="$tmp/transclip-signing.p12" kcpass sha1
  sha1="$(read_pin)"
  kcpass="$(uuidgen)"

  ( umask 077; printf '%s' "$CODESIGN_P12_BASE64" | base64 --decode > "$p12" )
  security create-keychain -p "$kcpass" "$kc"
  security set-keychain-settings -lut 21600 "$kc"
  security unlock-keychain -p "$kcpass" "$kc"
  security import "$p12" -k "$kc" -P "$CODESIGN_P12_PASSWORD" -T /usr/bin/codesign > /dev/null
  rm -f "$p12"
  # Let codesign use the private key without an interactive keychain prompt.
  security set-key-partition-list -S apple-tool:,apple: -s -k "$kcpass" "$kc" > /dev/null

  security find-identity -p codesigning "$kc" | grep -qi "$sha1" \
    || die "the imported certificate does not match the pin ($sha1); is CODESIGN_P12_BASE64 the right identity?"
  info "Imported signing identity $sha1 into $kc"

  # Public certificate, for the CI step that marks it trusted for code signing.
  security find-certificate -c "$IDENTITY_CN" -p "$kc" > "$tmp/transclip-signing.pem"

  if [[ -n "${GITHUB_ENV:-}" ]]; then
    {
      echo "CODESIGN_KEYCHAIN_PATH=$kc"
      # Selecting the identity by hash is unambiguous. Tauri passes it straight to `codesign -s`.
      echo "APPLE_SIGNING_IDENTITY=$(printf '%s' "$sha1" | tr 'a-f' 'A-F')"
    } >> "$GITHUB_ENV"
  fi
}

# ---------------------------------------------------------------------------
# verify: the signature must be valid and carry exactly the pinned requirement
# ---------------------------------------------------------------------------
cmd_verify() {
  local target="${1:-}"
  [[ -n "$target" ]] || { usage >&2; exit 2; }
  require_macos

  local sha1 expected app="$target"
  sha1="$(read_pin)"
  expected="identifier \"$BUNDLE_ID\" and certificate root = H\"$sha1\""

  if [[ "$target" == *.tar.gz ]]; then
    [[ -f "$target" ]] || die "not found: $target"
    WORK="$(mktemp -d)"
    tar -xzf "$target" -C "$WORK"
    app="$(find "$WORK" -maxdepth 1 -name '*.app' -type d | head -n 1)"
  fi
  [[ -d "$app" ]] || die "app bundle not found: $target"

  info "Verifying $target"
  codesign --verify --deep --strict --verbose=2 "$app" \
    || die "codesign --verify failed: the bundle is unsigned, ad-hoc signed or was modified after signing"

  local out actual
  out="$(codesign -d -r- "$app" 2>&1 || true)"
  actual="$(awk -F'designated => ' '/designated => /{print $2; exit}' <<< "$out")"
  [[ -n "$actual" ]] || die "could not read the designated requirement of $app"
  echo "    designated => $actual"

  # Ignore cosmetic differences (case, quoting); the identifier and the hash must still match exactly.
  if [[ "$(normalize_requirement "$actual")" != "$(normalize_requirement "$expected")" ]]; then
    die "designated requirement mismatch.
  expected: $expected
  actual:   $actual
Shipping this build would make macOS treat it as a different app and reset every user's
Accessibility / Keychain grants. Fix the signing identity instead of releasing."
  fi

  info "OK: signature is valid and matches the pinned requirement"
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    {
      echo "### Code signing"
      echo "- \`$(basename "$target")\`: \`$actual\`"
    } >> "$GITHUB_STEP_SUMMARY"
  fi
}

case "${1:-}" in
  setup) shift; cmd_setup "$@" ;;
  import) shift; cmd_import "$@" ;;
  verify) shift; cmd_verify "$@" ;;
  ""|-h|--help|help) usage ;;
  *) usage >&2; exit 2 ;;
esac
