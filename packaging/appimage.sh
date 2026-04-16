#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Build an AppImage from the release binary.
# Expects: target/release/gvauchi exists, linuxdeploy available.

set -euo pipefail

VERSION="${1:-0.0.0}"
APPDIR="AppDir"

rm -rf "$APPDIR"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/metainfo"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

cp target/release/gvauchi "${APPDIR}/usr/bin/"
cp data/com.vauchi.desktop.desktop "${APPDIR}/usr/share/applications/"
cp data/com.vauchi.desktop.metainfo.xml "${APPDIR}/usr/share/metainfo/"

if [ -f data/icons/com.vauchi.desktop.png ]; then
  cp data/icons/com.vauchi.desktop.png \
    "${APPDIR}/usr/share/icons/hicolor/256x256/apps/com.vauchi.desktop.png"
fi

export DEPLOY_GTK_VERSION=4
linuxdeploy --appdir "$APPDIR" \
  --desktop-file "${APPDIR}/usr/share/applications/com.vauchi.desktop.desktop" \
  --plugin gtk \
  --output appimage

mv Vauchi*.AppImage "vauchi-gtk-${VERSION}-x86_64.AppImage" 2>/dev/null \
  || mv *.AppImage "vauchi-gtk-${VERSION}-x86_64.AppImage"
echo "Done: Built vauchi-gtk-${VERSION}-x86_64.AppImage"
