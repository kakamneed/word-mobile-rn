#!/bin/sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
IOS_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../../../.." && pwd)
CRATE_DIR="$REPO_ROOT/crates/platform-mobile"

PLATFORM_NAME_VALUE=${PLATFORM_NAME:-}
CURRENT_ARCH_VALUE=${CURRENT_ARCH:-}
if [ -z "$CURRENT_ARCH_VALUE" ] || [ "$CURRENT_ARCH_VALUE" = "undefined_arch" ]; then
  if [ -n "${NATIVE_ARCH_ACTUAL:-}" ]; then
    CURRENT_ARCH_VALUE=$NATIVE_ARCH_ACTUAL
  elif [ -n "${ARCHS:-}" ]; then
    CURRENT_ARCH_VALUE=${ARCHS%% *}
  fi
fi

if [ -z "$PLATFORM_NAME_VALUE" ]; then
  echo "PLATFORM_NAME is required" >&2
  exit 1
fi

if [ -z "$CURRENT_ARCH_VALUE" ]; then
  echo "CURRENT_ARCH could not be determined" >&2
  exit 1
fi

RUST_TARGET=
case "$PLATFORM_NAME_VALUE" in
  iphoneos)
    RUST_TARGET="aarch64-apple-ios"
    ;;
  iphonesimulator)
    case "$CURRENT_ARCH_VALUE" in
      arm64)
        RUST_TARGET="aarch64-apple-ios-sim"
        ;;
      x86_64)
        RUST_TARGET="x86_64-apple-ios"
        ;;
      *)
        echo "Unsupported iOS simulator arch: $CURRENT_ARCH_VALUE" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported PLATFORM_NAME: $PLATFORM_NAME_VALUE" >&2
    exit 1
    ;;
esac

PROFILE_DIR=debug
if [ "${CONFIGURATION:-Debug}" = "Release" ]; then
  PROFILE_DIR=release
fi

echo "Building Rust iOS library for $RUST_TARGET ($PROFILE_DIR)"

if [ "$PROFILE_DIR" = "release" ]; then
  cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --target "$RUST_TARGET" --release
else
  cargo build --manifest-path "$CRATE_DIR/Cargo.toml" --target "$RUST_TARGET"
fi

SOURCE_LIB="$REPO_ROOT/target/$RUST_TARGET/$PROFILE_DIR/libword_platform_mobile.a"
ARTIFACT_DIR="$IOS_DIR/rust-artifacts/$PLATFORM_NAME_VALUE-$CURRENT_ARCH_VALUE"
DEST_LIB="$ARTIFACT_DIR/libword_platform_mobile.a"

if [ ! -f "$SOURCE_LIB" ]; then
  echo "Expected Rust library not found: $SOURCE_LIB" >&2
  exit 1
fi

mkdir -p "$ARTIFACT_DIR"
cp "$SOURCE_LIB" "$DEST_LIB"

echo "Copied Rust iOS library to $DEST_LIB"
