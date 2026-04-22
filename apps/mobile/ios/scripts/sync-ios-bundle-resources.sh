#!/bin/sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
IOS_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
APP_ROOT=$(CDPATH= cd -- "$IOS_DIR/.." && pwd)
TARGET_RESOURCES_DIR="${TARGET_BUILD_DIR}/${UNLOCALIZED_RESOURCES_FOLDER_PATH}"
ANDROID_ASSETS_DIR="$APP_ROOT/android/app/src/main/assets"
SNAPSHOT_SRC="$APP_ROOT/resources/vocab-snapshot/vocab-snapshot.jsonl"

copy_dir_if_present() {
  SOURCE_DIR=$1
  DEST_DIR=$2
  if [ -d "$SOURCE_DIR" ]; then
    rm -rf "$DEST_DIR"
    mkdir -p "$(dirname "$DEST_DIR")"
    ditto "$SOURCE_DIR" "$DEST_DIR"
    echo "Copied resource directory: $SOURCE_DIR -> $DEST_DIR"
  fi
}

mkdir -p "$TARGET_RESOURCES_DIR"

copy_dir_if_present "$ANDROID_ASSETS_DIR/seed-vocab" "$TARGET_RESOURCES_DIR/seed-vocab"
copy_dir_if_present "$ANDROID_ASSETS_DIR/seed-medical" "$TARGET_RESOURCES_DIR/seed-medical"

if [ -f "$SNAPSHOT_SRC" ]; then
  mkdir -p "$TARGET_RESOURCES_DIR/vocab-snapshot"
  cp "$SNAPSHOT_SRC" "$TARGET_RESOURCES_DIR/vocab-snapshot/vocab-snapshot.jsonl"
  echo "Copied vocabulary snapshot: $SNAPSHOT_SRC"
else
  echo "warning: missing iOS snapshot source at $SNAPSHOT_SRC" >&2
fi
