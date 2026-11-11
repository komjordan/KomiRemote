#!/usr/bin/env python3
"""Source bounds for the iOS controller branch. Not a compile or a device test."""
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[1]
errors = []


def text(rel):
    return (root / rel).read_text(encoding="utf-8")


def require(cond, message):
    if not cond:
        errors.append(message)


pubspec = text("flutter/pubspec.yaml")
require(pubspec.count("version: 1.5.1+92") == 1, "pubspec version is not 1.5.1+92")

info = text("flutter/ios/Runner/Info.plist")
require("<string>Komi Remote</string>" in info, "display name missing")
require("<string>KomiRemote</string>" in info, "technical bundle name missing")
require("NSMicrophoneUsageDescription" not in info, "microphone emission permission added")
require("NSCameraUsageDescription" in info, "QR camera permission missing")
require("n'émet pas la caméra" in info, "camera string does not refuse emission")
require("<string>komiremote</string>" in info, "komiremote URL scheme missing")

entitlements = text("flutter/ios/Runner/Runner.entitlements")
require("aps-environment" not in entitlements, "push entitlement still present")
require("wifi-info" not in entitlements, "wifi-info entitlement still present")

pbx = text("flutter/ios/Runner.xcodeproj/project.pbxproj")
require(pbx.count("PRODUCT_BUNDLE_IDENTIFIER = fr.komjordan.komiremote;") == 3, "bundle id count")
require("HZF9JMC8YN" not in pbx, "upstream Apple team id still present")
require(pbx.count("CODE_SIGNING_ALLOWED = NO;") == 3, "unsigned flag count")
require("liblibrustdesk.a" in pbx, "static lib link missing")

home = text("flutter/lib/mobile/pages/home_page.dart")
require("if (isAndroid && !bind.isOutgoingOnly())" in home, "host tab gate missing")
require("ServerPage()" in home, "upstream host page removed instead of gated")

lib = text("src/lib.rs")
require('#[cfg(not(any(target_os = "ios")))]\n/// cbindgen:ignore\nmod server;' in lib, "iOS still compiles the host server")
require("fn komi_ios_directory_context" in lib, "read-only name context missing")
require('#[cfg(target_os = "ios")]\n#[no_mangle]\npub extern "C" fn komi_ios_directory_context' in lib, "iOS context not ios-gated")

common = text("src/common.rs")
require('#[cfg(any(windows, target_os = "macos", target_os = "ios"))]' in common, "iOS defaults not initialized")

voice = text("src/client/io_loop.rs")
require("iOS does not have this server." in voice, "voice emission comment missing")
require('#[cfg(not(any(target_os = "ios")))]' in voice, "voice emission no longer excluded on iOS")

names = text("flutter/lib/desktop/komi_public_names.dart")
require("class KomiIosDirectoryContext" in names, "iOS directory adapter missing")
require("Windows et macOS uniquement" in names, "management gate removed")

connect = text("flutter/lib/common.dart")
require("KomiIosDirectoryContext.read()" in connect, "iOS connect path missing")
require("directory.storedPeers.contains" in connect, "iOS known-peer check is not file-backed")

banner = text("flutter/lib/mobile/pages/connection_page.dart")
require("Contrôleur uniquement" in banner, "controller banner missing")
require("ne sont pas émis" in banner, "emission refusal missing")

workflow = text(".github/workflows/komiremote-ios-test.yml")
require("workflow_dispatch:" in workflow, "manual dispatch missing")
require("pull_request:" not in workflow, "PR trigger present")
require("\n  push:" not in workflow and "\npush:" not in workflow, "push trigger present")
require("flutter-build.yml" not in workflow, "must not call the release workflow")
require("komiremote-macos-test" not in workflow, "must not dispatch the Mac workflow")
require("--no-codesign" in workflow, "unsigned build flag missing")
require("aarch64-apple-ios" in workflow, "device target missing")
require("1.5.1+92" in workflow, "workflow version gate missing")

ios_tree = "\n".join(p.read_text(encoding="utf-8", errors="ignore") for p in (root / "flutter/ios").rglob("*") if p.is_file() and p.stat().st_size < 2_000_000)
require("ReplayKit" not in ios_tree, "ReplayKit host extension present")
require("Broadcast" not in ios_tree, "broadcast upload extension present")

if errors:
    print("FAIL")
    for item in errors:
        print("-", item)
    sys.exit(1)
print("ok", len(errors) == 0)
