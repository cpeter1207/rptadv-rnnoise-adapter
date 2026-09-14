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
