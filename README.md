# rptadv-rnnoise-adapter

`rptadv-rnnoise-adapter` is the versioned dynamic RNNoise dependency adapter
for `rpt_advanced` and USBRadioPlus. It implements the narrow descriptor
required by ADR 0022 so owned Rust audio code never imports an RNNoise type or
links RNNoise directly.

ABI v1 owns one persistent default-model RNNoise state and processes exactly
480 mono F32 samples at 48 kHz. Its public PCM is canonical normalized F32;
the adapter privately scales samples to and from the PCM-code units expected
by RNNoise. Creation verifies the linked library's frame size and processes
the same two silent warm-up frames used by current USBRadioPlus setup. RNNoise
remains a dynamic runtime dependency; this project neither vendors nor
static-links it.

Callback partition assembly and USBRadioPlus's two-live-frame output priming
remain consumer-owned because they are stream policy rather than RNNoise API.
ABI v1 deliberately has no reset entry. Current bypass behavior clears only
consumer framing state and retains upstream RNNoise history; reinitializing
the model would change that behavior.

The public descriptor is selected by
`RPTADV_RNNOISE_ADAPTER_CAPABILITY`. Consumers verify that name, the exact ABI
version, the descriptor's minimum readable prefix, and every required function
pointer. `process` validates a complete frame, supports overlapping input and
output buffers, and performs no allocation, locking, waiting, I/O, or adapter
logging.

Build a local shared object with `make`. Run fast source checks with
`make lint static-analysis`, focused tests with `make test`, or the complete
local gate with `make ci`. `make container-ci` builds the Debian 13 quality
image and runs that gate in a project-labeled disposable container. The image
build obtains the official RNNoise 0.2 source archive by pinned checksum and
installs locally built Debian packages so build dependencies and generated
runtime dependencies are verified rather than bypassed.

In that quality image, `make release-packages` builds and checks the adapter
packages and copies the retained RNNoise 0.2 runtime and development packages
into `build/debian-source/` for the same release. The source archive is built
by the existing package target.

`make install` installs the versioned shared object, public header, and
pkg-config metadata. The full public contract is documented in
[`include/rptadv_rnnoise_adapter/rptadv_rnnoise_adapter.h`](include/rptadv_rnnoise_adapter/rptadv_rnnoise_adapter.h).
`make docs` also checks every documented production Rust declaration and
publishes its complete implementation context through Doxygen.
