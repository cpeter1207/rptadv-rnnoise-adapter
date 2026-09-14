//! Private, minimal bindings for the RNNoise API owned by this adapter.
//!
//! No RNNoise type crosses the public descriptor. Tests replace the function
//! table below with deterministic fake functions.

use std::ffi::c_int;

/// Opaque upstream RNNoise denoiser state.
#[repr(C)]
pub(crate) struct DenoiseState {
    _private: [u8; 0],
}

/// Opaque upstream RNNoise model.
#[repr(C)]
pub(crate) struct RnnModel {
    _private: [u8; 0],
}

/// Adapter-owned bindings used by one persistent denoiser.
#[derive(Clone, Copy)]
pub(crate) struct FunctionTable {
    pub(crate) get_frame_size: unsafe extern "C" fn() -> c_int,
    pub(crate) create: unsafe extern "C" fn(*mut RnnModel) -> *mut DenoiseState,
    pub(crate) process_frame: unsafe extern "C" fn(*mut DenoiseState, *mut f32, *const f32) -> f32,
    pub(crate) destroy: unsafe extern "C" fn(*mut DenoiseState),
}

#[link(name = "rnnoise", kind = "dylib")]
unsafe extern "C" {
    /// Return the frame size required by the linked RNNoise implementation.
    #[link_name = "rnnoise_get_frame_size"]
    fn rnnoise_get_frame_size_native() -> c_int;
    /// Allocate an upstream denoiser using the default model.
    #[link_name = "rnnoise_create"]
    fn rnnoise_create_native(model: *mut RnnModel) -> *mut DenoiseState;
    /// Process one complete upstream RNNoise frame.
    #[link_name = "rnnoise_process_frame"]
    fn rnnoise_process_frame_native(
        state: *mut DenoiseState,
        output: *mut f32,
        input: *const f32,
    ) -> f32;
    /// Release an upstream denoiser.
    #[link_name = "rnnoise_destroy"]
    fn rnnoise_destroy_native(state: *mut DenoiseState);
}

/// Invoke the production RNNoise frame-size query.
unsafe extern "C" fn production_get_frame_size() -> c_int {
    unsafe { rnnoise_get_frame_size_native() }
}

/// Invoke the production RNNoise constructor.
unsafe extern "C" fn production_create(model: *mut RnnModel) -> *mut DenoiseState {
    unsafe { rnnoise_create_native(model) }
}

/// Invoke production RNNoise frame processing.
unsafe extern "C" fn production_process_frame(
    state: *mut DenoiseState,
    output: *mut f32,
    input: *const f32,
) -> f32 {
    unsafe { rnnoise_process_frame_native(state, output, input) }
}

/// Invoke the production RNNoise destructor.
unsafe extern "C" fn production_destroy(state: *mut DenoiseState) {
    unsafe { rnnoise_destroy_native(state) }
}

/// Function table used by the exported production descriptor.
pub(crate) static PRODUCTION_FUNCTIONS: FunctionTable = FunctionTable {
    get_frame_size: production_get_frame_size,
    create: production_create,
    process_frame: production_process_frame,
    destroy: production_destroy,
};
