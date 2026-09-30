#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "${script_dir}/.." && pwd)"
android_sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Android/Sdk}}"
android_java="${JAVA_HOME:-${HOME}/Android/jdk-21}"

if [[ ! -d "${android_sdk}" ]]; then
  echo "Android SDK introuvable : ${android_sdk}" >&2
  echo "Définissez ANDROID_HOME ou ANDROID_SDK_ROOT." >&2
  exit 1
fi

if [[ ! -x "${android_java}/bin/java" ]]; then
  java_binary="$(command -v java || true)"
  if [[ -z "${java_binary}" ]]; then
    echo "Java introuvable. Définissez JAVA_HOME vers un JDK 17 ou 21." >&2
    exit 1
  fi
  java_binary="$(readlink -f "${java_binary}")"
  android_java="${java_binary%/bin/java}"
fi

android_ndk="${NDK_HOME:-}"
if [[ -z "${android_ndk}" ]]; then
  android_ndk="$(find "${android_sdk}/ndk" -mindepth 1 -maxdepth 1 -type d -print 2>/dev/null | sort -V | tail -n 1)"
fi
if [[ -z "${android_ndk}" || ! -d "${android_ndk}" ]]; then
  echo "Android NDK introuvable sous ${android_sdk}/ndk." >&2
  exit 1
fi

export ANDROID_HOME="${android_sdk}"
export ANDROID_SDK_ROOT="${android_sdk}"
export JAVA_HOME="${android_java}"
export NDK_HOME="${android_ndk}"
export PATH="${JAVA_HOME}/bin:${ANDROID_HOME}/platform-tools:${ANDROID_HOME}/cmdline-tools/latest/bin:${PATH}"

cd "${repo_dir}"
exec npx --no-install tauri android "$@" --config apps/desktop/src-tauri/tauri.conf.json
