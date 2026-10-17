#!/usr/bin/env python3
"""Build and bundle the local macOS app with the installed Swift toolchain."""
from pathlib import Path
import os
import plistlib
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
configuration = "debug" if "--debug" in sys.argv else "release"
scratch = root / ".build"
environment = dict(os.environ)
environment["CLANG_MODULE_CACHE_PATH"] = str(scratch / "clang-cache")
environment["SWIFTPM_MODULECACHE_OVERRIDE"] = str(scratch / "module-cache")
flags = ["--disable-sandbox", "--scratch-path", str(scratch), "--cache-path", str(scratch / "package-cache"),
         "--config-path", str(scratch / "config"), "--security-path", str(scratch / "security"), "-c", configuration]
subprocess.run(["swift", "build", *flags], cwd=root, env=environment, check=True)
products = Path(subprocess.check_output(["swift", "build", *flags, "--show-bin-path"], cwd=root, env=environment, text=True).strip())
app = root / "build/text-md.app"
macos = app / "Contents/MacOS"
resources = app / "Contents/Resources"
macos.mkdir(parents=True, exist_ok=True)
resources.mkdir(parents=True, exist_ok=True)
shutil.copy2(products / "text-md", macos / "text-md")
for notice in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
    shutil.copy2(root / notice, resources / notice)
if (resources / "Licenses").exists():
    shutil.rmtree(resources / "Licenses")
shutil.copytree(root / "Licenses", resources / "Licenses")
for bundle in products.glob("*.bundle"):
    if bundle.name.endswith("Tests.bundle"):
        continue
    shutil.copytree(bundle, resources / bundle.name, dirs_exist_ok=True)
iconset = root / "build/AppIcon.iconset"
iconset.mkdir(parents=True, exist_ok=True)
png = root / "build/AppIcon.png"
subprocess.run(["swift", str(root / "scripts/icon.swift"), str(png)], env=environment, check=True)
for points in (16, 32, 128, 256, 512):
    for scale in (1, 2):
        pixels = points * scale
        name = f"icon_{points}x{points}" + ("@2x" if scale == 2 else "") + ".png"
        subprocess.run(["sips", "-z", str(pixels), str(pixels), str(png), "--out", str(iconset / name)],
                       stdout=subprocess.DEVNULL, check=True)
subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(resources / "AppIcon.icns")], check=True)
frameworks = app / "Contents/Frameworks"
for binary in products.glob("*.dylib"):
    frameworks.mkdir(exist_ok=True)
    shutil.copy2(binary, frameworks / binary.name)
for binary in products.glob("*.framework"):
    frameworks.mkdir(exist_ok=True)
    shutil.copytree(binary, frameworks / binary.name, dirs_exist_ok=True)
info = {
    "CFBundleName": "text-md", "CFBundleDisplayName": "text-md", "CFBundleExecutable": "text-md",
    "CFBundleIdentifier": "com.cassiano.text-md", "CFBundlePackageType": "APPL",
    "CFBundleIconFile": "AppIcon",
    "CFBundleShortVersionString": "0.3.2", "CFBundleVersion": "7", "LSMinimumSystemVersion": "14.0",
    "NSHighResolutionCapable": True, "NSPrincipalClass": "NSApplication", "CFBundleDevelopmentRegion": "pt_BR",
    "CFBundleLocalizations": ["pt_BR"],
    "CFBundleDocumentTypes": [{"CFBundleTypeName": "Markdown", "CFBundleTypeRole": "Editor",
        "LSHandlerRank": "Alternate", "LSItemContentTypes": ["net.daringfireball.markdown", "public.plain-text"]}],
    "UTImportedTypeDeclarations": [{"UTTypeIdentifier": "net.daringfireball.markdown", "UTTypeDescription": "Markdown",
        "UTTypeConformsTo": ["public.plain-text"], "UTTypeTagSpecification": {"public.filename-extension": ["md", "markdown"]}}],
}
with (app / "Contents/Info.plist").open("wb") as output:
    plistlib.dump(info, output)
subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(app)], check=True)
print(app)
