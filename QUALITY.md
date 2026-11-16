# Quality checks

[AGENTS.md](AGENTS.md) defines the project baseline. Fast checks are:

```sh
make lint static-analysis
```

The native Debian 13 pull-request gate runs formatting, lint, static analysis,
Cppcheck, and Doxygen once, then builds, tests, stages installation, builds
packages, runs autopkgtest, and checks the source archive on amd64 and arm64.
Production Rust requires 100% line and branch coverage on amd64 only;
`make ci` includes that coverage check.

Staged-install, package, and autopkgtest checks compile and run the public C
descriptor smoke test through installed pkg-config metadata. They verify the
adapter SONAME, dynamic RNNoise dependency, header, and absence of a static
adapter archive. Doxygen generation checks documented production Rust
declarations and embeds their complete source context in the reference pages.

The quality image builds verified RNNoise 0.2 Debian packages from the official
source archive. The container launcher removes only containers carrying this
project's exact project and workspace labels before and after a run. The
package target applies the same labeled cleanup to its nested autopkgtest
testbed. The project owns no persistent test containers.

## Explicit reset qualification

`make test` also runs `tests/reset_allocation_guard.c` against the real dynamic
adapter and RNNoise. The native glibc interposer counts malloc/calloc/realloc,
aligned allocation and free, verifies an allocating/freeing positive control,
then arms around 256 dirty-state resets plus first nonzero caller frames.
Samples and probabilities must equal an independently reset reference. Setup
and destruction are outside the guarded interval; reset does not warm up.
`LD_BIND_NOW=1` removes lazy dynamic-symbol binding from the measured interval.
Source audit of the pinned default-model init path separately covers bounded
work and absence of locks/I/O/logging; allocation instrumentation is not a
universal real-time proof. Never generalize to non-NULL custom models.

Both native Debian 13 amd64 and arm64 runs, package/staged-install checks and
100% production line/branch coverage remain required before merge. A local
amd64 guard pass alone does not qualify the full RT contract. The reset package
floor is not released alpha.2: assign and synchronize actual candidate metadata
only after its reviewed identity is established.
