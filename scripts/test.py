#!/usr/bin/env python3
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[1]
scratch = root / ".build"
environment = dict(os.environ)
environment["CLANG_MODULE_CACHE_PATH"] = str(scratch / "clang-cache")
environment["SWIFTPM_MODULECACHE_OVERRIDE"] = str(scratch / "module-cache")
toolchain = Path(subprocess.check_output(["xcrun", "--find", "swift"], text=True).strip()).parent.parent
subprocess.run(["swift", "test", "--disable-xctest", "--enable-swift-testing", "--disable-sandbox",
    "-Xswiftc", "-plugin-path", "-Xswiftc", str(toolchain / "lib/swift/host/plugins/testing"),
    "--scratch-path", str(scratch), "--cache-path", str(scratch / "package-cache"),
    "--config-path", str(scratch / "config"), "--security-path", str(scratch / "security")],
    cwd=root, env=environment, check=True)
