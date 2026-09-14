//! Versioned, fixed-format RNNoise adapter for `rpt_advanced`.
//!
//! The public C descriptor contains no RNNoise types. One opaque handle owns
//! the upstream default-model state plus fixed scratch frames. Callers exchange
//! exactly 480 mono normalized F32 samples at 48 kHz; the adapter performs the
//! PCM-code scaling expected by RNNoise. After creation, `process` allocates
//! nothing, takes no lock, performs no I/O, emits no log, and has no intended
//! panic path.

#![deny(unsafe_op_in_unsafe_fn)]

mod ffi;

use std::ffi::{c_char, c_int};
use std::mem::size_of;
use std::ptr::{self, NonNull};
use std::slice;

/// ABI version exported by this adapter.
const ABI_VERSION: u32 = 1;
/// Stable name used to select this adapter capability.
const CAPABILITY_NAME: &[u8] = b"rptadv.rnnoise\0";
/// Only sample rate accepted by RNNoise ABI v1.
const SAMPLE_RATE_HZ: u32 = 48_000;
/// Only channel count accepted by RNNoise ABI v1.
const CHANNEL_COUNT: u32 = 1;
/// Exact sample count in every RNNoise ABI-v1 process call.
const FRAME_COUNT: usize = 480;
/// Conversion between canonical normalized PCM and RNNoise PCM-code units.
const PCM_CODE_SCALE: f32 = 32_768.0;
/// Silent frames processed during control-plane creation.
const WARMUP_FRAME_COUNT: usize = 2;

/// Successful adapter result.
const OK: c_int = 0;
/// Invalid public pointer or process frame count.
const INVALID_ARGUMENT: c_int = -1;
/// The linked RNNoise implementation failed or violated its contract.
const RNNOISE_ERROR: c_int = -2;
/// Requested PCM rate, layout, or setup frame size is unsupported.
const UNSUPPORTED: c_int = -3;

const _: () = assert!(OK == 0);
const _: () = assert!(INVALID_ARGUMENT == -1);
const _: () = assert!(RNNOISE_ERROR == -2);
const _: () = assert!(UNSUPPORTED == -3);

/// Persistent, opaque RNNoise denoiser represented by the public C ABI.
#[repr(C)]
pub struct Denoiser {
    state: NonNull<ffi::DenoiseState>,
    functions: &'static ffi::FunctionTable,
    input_codes: [f32; FRAME_COUNT],
    output_codes: [f32; FRAME_COUNT],
}

impl Drop for Denoiser {
    /// Release the persistent upstream state owned by this denoiser.
    fn drop(&mut self) {
        unsafe {
            (self.functions.destroy)(self.state.as_ptr());
        }
    }
}

/// Versioned descriptor exposed to C and adapter-neutral Rust consumers.
#[repr(C)]
pub struct AdapterDescriptor {
    struct_size: u32,
    abi_version: u32,
    capability_name: *const c_char,
    create: extern "C" fn(u32, u32, u32, *mut *mut Denoiser) -> c_int,
    process: extern "C" fn(*mut Denoiser, *const f32, u32, *mut f32, *mut f32) -> c_int,
    destroy: extern "C" fn(*mut Denoiser),
}

// The descriptor contains only immutable pointers to static code and storage.
unsafe impl Sync for AdapterDescriptor {}

/// Return whether an upstream speech probability is usable by the public API.
fn valid_probability(probability: f32) -> bool {
    probability.is_finite() && (0.0..=1.0).contains(&probability)
}

/// Process a frame through the supplied production or deterministic test bindings.
fn run_frame(denoiser: &mut Denoiser) -> Result<f32, ()> {
    let probability = unsafe {
        (denoiser.functions.process_frame)(
            denoiser.state.as_ptr(),
            denoiser.output_codes.as_mut_ptr(),
            denoiser.input_codes.as_ptr(),
        )
    };
    if valid_probability(probability) {
        Ok(probability)
    } else {
        Err(())
    }
}

/// Create a denoiser through the supplied production or deterministic test bindings.
fn create_with_functions(
    functions: &'static ffi::FunctionTable,
    sample_rate_hz: u32,
    channel_count: u32,
    frame_count: u32,
    out_denoiser: *mut *mut Denoiser,
) -> c_int {
    let Some(out_denoiser) = NonNull::new(out_denoiser) else {
        return INVALID_ARGUMENT;
    };
    unsafe {
        *out_denoiser.as_ptr() = ptr::null_mut();
    }
    if sample_rate_hz != SAMPLE_RATE_HZ
        || channel_count != CHANNEL_COUNT
        || frame_count != FRAME_COUNT as u32
    {
        return UNSUPPORTED;
    }
    if unsafe { (functions.get_frame_size)() } != FRAME_COUNT as c_int {
        return RNNOISE_ERROR;
    }

    let Some(state) = NonNull::new(unsafe { (functions.create)(ptr::null_mut()) }) else {
        return RNNOISE_ERROR;
    };
    let mut denoiser = Box::new(Denoiser {
        state,
        functions,
        input_codes: [0.0; FRAME_COUNT],
        output_codes: [0.0; FRAME_COUNT],
    });
    for _ in 0..WARMUP_FRAME_COUNT {
        if run_frame(&mut denoiser).is_err() {
            return RNNOISE_ERROR;
        }
    }
    denoiser.output_codes.fill(0.0);
    unsafe {
        *out_denoiser.as_ptr() = Box::into_raw(denoiser);
    }
    OK
}

/// Create one fixed-format denoiser for the exported production descriptor.
extern "C" fn create(
    sample_rate_hz: u32,
    channel_count: u32,
    frame_count: u32,
    out_denoiser: *mut *mut Denoiser,
) -> c_int {
    create_with_functions(
        &ffi::PRODUCTION_FUNCTIONS,
        sample_rate_hz,
        channel_count,
        frame_count,
        out_denoiser,
    )
}

/// Process exactly one normalized F32 RNNoise frame without control-plane work.
extern "C" fn process(
    denoiser: *mut Denoiser,
    input: *const f32,
    frame_count: u32,
    output: *mut f32,
    out_vad_probability: *mut f32,
) -> c_int {
    let Some(out_vad_probability) = NonNull::new(out_vad_probability) else {
        return INVALID_ARGUMENT;
    };
    unsafe {
        *out_vad_probability.as_ptr() = 0.0;
    }
    let Some(mut denoiser) = NonNull::new(denoiser) else {
        return INVALID_ARGUMENT;
    };
    if input.is_null() || output.is_null() || frame_count != FRAME_COUNT as u32 {
        return INVALID_ARGUMENT;
    }
    let denoiser = unsafe { denoiser.as_mut() };
    {
        let input = unsafe { slice::from_raw_parts(input, FRAME_COUNT) };
        for (codes, sample) in denoiser.input_codes.iter_mut().zip(input) {
            *codes = *sample * PCM_CODE_SCALE;
        }
    }
    let Ok(probability) = run_frame(denoiser) else {
        return RNNOISE_ERROR;
    };
    let output = unsafe { slice::from_raw_parts_mut(output, FRAME_COUNT) };
    for (sample, codes) in output.iter_mut().zip(&denoiser.output_codes) {
        *sample = *codes / PCM_CODE_SCALE;
    }
    unsafe {
        *out_vad_probability.as_ptr() = probability;
    }
    OK
}

/// Destroy one denoiser after all callers have stopped using it.
extern "C" fn destroy(denoiser: *mut Denoiser) {
    let Some(denoiser) = NonNull::new(denoiser) else {
        return;
    };
    unsafe {
        drop(Box::from_raw(denoiser.as_ptr()));
    }
}

/// Immutable ABI-v1 production descriptor retained for the process lifetime.
static DESCRIPTOR: AdapterDescriptor = AdapterDescriptor {
    struct_size: size_of::<AdapterDescriptor>() as u32,
    abi_version: ABI_VERSION,
    capability_name: CAPABILITY_NAME.as_ptr().cast::<c_char>(),
    create,
    process,
    destroy,
};

/// Return the immutable function table for ABI version one.
#[unsafe(no_mangle)]
pub extern "C" fn rptadv_rnnoise_adapter_descriptor() -> *const AdapterDescriptor {
    &DESCRIPTOR
}

#[cfg(test)]
/// Create a denoiser through deterministic test bindings.
pub(crate) fn create_with_test_functions(
    functions: &'static ffi::FunctionTable,
    sample_rate_hz: u32,
    channel_count: u32,
    frame_count: u32,
    out_denoiser: *mut *mut Denoiser,
) -> c_int {
    create_with_functions(
        functions,
        sample_rate_hz,
        channel_count,
        frame_count,
        out_denoiser,
    )
}

#[cfg(test)]
mod tests;
