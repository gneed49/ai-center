#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
android_sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Android/Sdk}}"
build_tools="$(find "${android_sdk}/build-tools" -mindepth 1 -maxdepth 1 -type d -print | sort -V | tail -n 1)"

aapt="${build_tools}/aapt"
apksigner="${build_tools}/apksigner"
outputs="${repo_dir}/apps/desktop/src-tauri/gen/android/app/build/outputs"
arm64_apk="${outputs}/apk/arm64/debug/app-arm64-debug.apk"
x86_64_apk="${outputs}/apk/x86_64/debug/app-x86_64-debug.apk"
release_apk="${outputs}/apk/universal/release/app-universal-release-unsigned.apk"
release_aab="${outputs}/bundle/universalRelease/app-universal-release.aab"
generated_android="${repo_dir}/apps/desktop/src-tauri/gen/android"

for executable in "${aapt}" "${apksigner}"; do
  if [[ ! -x "${executable}" ]]; then
    echo "Outil Android introuvable : ${executable}" >&2
    exit 1
  fi
done

for artifact in "${arm64_apk}" "${x86_64_apk}" "${release_apk}" "${release_aab}"; do
  if [[ ! -f "${artifact}" ]]; then
    echo "Artefact Android manquant : ${artifact}" >&2
    exit 1
  fi
done

"${apksigner}" verify "${arm64_apk}"
"${apksigner}" verify "${x86_64_apk}"

badging="$("${aapt}" dump badging "${release_apk}")"
grep -Fq "package: name='com.aicenter.client'" <<<"${badging}"
grep -Fq "sdkVersion:'24'" <<<"${badging}"
grep -Fq "targetSdkVersion:'36'" <<<"${badging}"
grep -Fq "uses-permission: name='android.permission.INTERNET'" <<<"${badging}"
grep -Fq "native-code: 'arm64-v8a' 'x86_64'" <<<"${badging}"

aab_listing="$(unzip -Z1 "${release_aab}")"
grep -Fq 'base/lib/arm64-v8a/' <<<"${aab_listing}"
grep -Fq 'base/lib/x86_64/' <<<"${aab_listing}"

debug_manifest="$("${aapt}" dump xmltree "${arm64_apk}" AndroidManifest.xml)"
release_manifest="$("${aapt}" dump xmltree "${release_apk}" AndroidManifest.xml)"
grep -Fq 'android:usesCleartextTraffic' <<<"${debug_manifest}"
grep -Fq '(type 0x12)0xffffffff' <<<"${debug_manifest}"
grep -Fq 'android:usesCleartextTraffic' <<<"${release_manifest}"
grep -Fq '(type 0x12)0x0' <<<"${release_manifest}"

audit_dir="$(mktemp -d /tmp/ai-center-android-audit-XXXXXX)"
cleanup() {
  rm -r -- "${audit_dir}"
}
trap cleanup EXIT

mkdir "${audit_dir}/arm64" "${audit_dir}/release-apk" "${audit_dir}/release-aab"
unzip -q "${arm64_apk}" -d "${audit_dir}/arm64"
unzip -q "${release_apk}" -d "${audit_dir}/release-apk"
unzip -q "${release_aab}" -d "${audit_dir}/release-aab"

if rg -a -l 'OPENAI_API_KEY|DATABASE_URL|sk-proj-[A-Za-z0-9_-]{20,}' "${audit_dir}" >/dev/null; then
  echo "Un secret serveur potentiel a été détecté dans un artefact Android." >&2
  exit 1
fi

if rg -a -l --glob '!app/build/**' \
  'OPENAI_API_KEY|DATABASE_URL|sk-proj-[A-Za-z0-9_-]{20,}' \
  "${generated_android}" >/dev/null; then
  echo "Un secret serveur potentiel a été détecté dans les sources Android générées." >&2
  exit 1
fi

echo "Artefacts Android valides : manifeste, signatures debug et audit de secrets réussis."
sha256sum "${arm64_apk}" "${x86_64_apk}" "${release_apk}" "${release_aab}"
