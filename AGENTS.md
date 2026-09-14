# rptadv-rnnoise-adapter development rules

## Shared rpt_advanced project baseline

This repository follows the shared rpt_advanced baseline. Run formatting,
lint, static analysis including Cppcheck, and Doxygen once where independent.
Run native Debian 13 amd64 and arm64 build, test, package, and staged-install
checks concurrently. Require 100% line and branch coverage of production code
only on Debian 13 amd64. Debian 12 support is aspirational and is not built or
tested automatically. Quality checks must not rewrite source files.

Before a push, run formatting, lint, and static analysis only. A full quality
gate is required for pull-request merge, not for every push. Treat warnings as
errors. Update Doxygen comments, tests, user documentation, examples, and
package artifacts with every affected interface. Consumers dynamically link
the released, versioned shared object; they must not vendor or static-link a
duplicate implementation.

Start and clean only project-owned, labeled test containers deterministically.
Pull and inspect the current `:latest` base image before a local container run.
Keep iteration evidence under ignored `/.work/`. Never deploy to a node or
alter node configuration without explicit approval.

## Adapter boundary

Follow `rpt_advanced` ADRs 0022, 0029, 0035, and 0040. This is a separately
versioned Rust `cdylib` with a narrow, stable C descriptor/function-table ABI.
It dynamically links RNNoise and exposes no RNNoise types. Public PCM is
48 kHz mono normalized IEEE-754 binary32, and each process operation contains
exactly 480 samples.

Creation may allocate and processes two silent warm-up frames. The persistent
process operation is allocation-free, lock-free, nonblocking, log-free, and
panic-free after setup. The caller serializes use of an individual denoiser
handle. Callback framing, delayed-output priming, counters, and bypass policy
belong to the consumer. Do not reset upstream RNNoise history on bypass: the
current required behavior retains that history.
