# Release Workflow

This repository publishes native C ABI artifacts for `aegis-ffi`.

## Supported V1 Targets

Apple Silicon:

- `aarch64-apple-darwin`
- `aarch64-apple-ios`
- `aarch64-apple-ios-sim`

Windows:

- `x86_64-pc-windows-msvc`

Apple Intel, Linux, Android, Swift wrappers, C++ wrappers, Unity, Unreal, and JNI packages are not part of the V1 release package.

## Local Validation

Run:

```bash
./scripts/validate-rust.sh
./scripts/check-c-header.sh
```

## Apple Packaging

Run on Apple Silicon macOS or a macOS CI runner that supports Apple Silicon targets:

```bash
rustup target add aarch64-apple-darwin aarch64-apple-ios aarch64-apple-ios-sim
./scripts/package-apple-xcframework.sh
```

Output:

```text
dist/apple/AegisFFI.xcframework.zip
```

## Windows Packaging

Run on Windows with the MSVC toolchain:

```powershell
rustup target add x86_64-pc-windows-msvc
powershell -ExecutionPolicy Bypass -File scripts/package-windows-msvc.ps1
```

Output:

```text
dist/windows/aegis-windows-x86_64-msvc.zip
```

## Checksums

Run:

```bash
./scripts/package-checksums.sh
```

Output:

```text
dist/checksums/SHA256SUMS
```

## GitHub Release

Push a tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds Apple and Windows artifacts, merges checksums, and uploads them to the GitHub Release.
