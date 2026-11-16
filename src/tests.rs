//! Deterministic unit tests using a fake RNNoise function table.

use std::ffi::c_int;
use std::ptr;

use super::{
    AdapterDescriptor, Denoiser, FRAME_COUNT, INVALID_ARGUMENT, OK, RNNOISE_ERROR, SAMPLE_RATE_HZ,
    UNSUPPORTED, create, create_with_test_functions, destroy, process, reset, valid_probability,
};
use crate::ffi::{DenoiseState, FunctionTable, RnnModel};

#[repr(C)]
struct FakeState {
    process_calls: u32,
    init_calls: u32,
    init_result: c_int,
    response: u32,
    model_was_null: bool,
    last_input: [f32; FRAME_COUNT],
}

unsafe extern "C" fn fake_get_frame_size() -> c_int {
    FRAME_COUNT as c_int
}

unsafe extern "C" fn wrong_get_frame_size() -> c_int {
    160
}

unsafe extern "C" fn fake_create(model: *mut RnnModel) -> *mut DenoiseState {
    LIFECYCLE.with(|counts| {
        let (creates, destroys) = counts.get();
        counts.set((creates + 1, destroys));
    });
    Box::into_raw(Box::new(FakeState {
        process_calls: 0,
        init_calls: 0,
        init_result: 0,
        response: 0,
        model_was_null: model.is_null(),
        last_input: [0.0; FRAME_COUNT],
    }))
    .cast::<DenoiseState>()
}

unsafe extern "C" fn null_create(_model: *mut RnnModel) -> *mut DenoiseState {
    ptr::null_mut()
}

unsafe extern "C" fn fake_process_frame(
    state: *mut DenoiseState,
    output: *mut f32,
    input: *const f32,
) -> f32 {
    let Some(state) = (unsafe { state.cast::<FakeState>().as_mut() }) else {
        return f32::NAN;
    };
    if output.is_null() || input.is_null() {
        return f32::NAN;
    }
    state.process_calls += 1;
    for index in 0..FRAME_COUNT {
        let sample = unsafe { *input.add(index) };
        state.last_input[index] = sample;
        unsafe {
            *output.add(index) = sample * 0.5;
        }
    }
    match state.response {
        0 => 0.625,
        1 => f32::NAN,
        2 => -0.001,
        _ => 1.001,
    }
}

unsafe extern "C" fn invalid_warmup_process_frame(
    _state: *mut DenoiseState,
    _output: *mut f32,
    _input: *const f32,
) -> f32 {
    f32::INFINITY
}

unsafe extern "C" fn fake_destroy(state: *mut DenoiseState) {
    if !state.is_null() {
        LIFECYCLE.with(|counts| {
            let (creates, destroys) = counts.get();
            counts.set((creates, destroys + 1));
        });
        unsafe {
            drop(Box::from_raw(state.cast::<FakeState>()));
        }
    }
}

static FAKE_FUNCTIONS: FunctionTable = FunctionTable {
    get_frame_size: fake_get_frame_size,
    create: fake_create,
    process_frame: fake_process_frame,
    destroy: fake_destroy,
    init: fake_init,
};

static WRONG_FRAME_FUNCTIONS: FunctionTable = FunctionTable {
    get_frame_size: wrong_get_frame_size,
    create: fake_create,
    process_frame: fake_process_frame,
    destroy: fake_destroy,
    init: fake_init,
};

static NULL_CREATE_FUNCTIONS: FunctionTable = FunctionTable {
    get_frame_size: fake_get_frame_size,
    create: null_create,
    process_frame: fake_process_frame,
    destroy: fake_destroy,
    init: fake_init,
};

static INVALID_WARMUP_FUNCTIONS: FunctionTable = FunctionTable {
    get_frame_size: fake_get_frame_size,
    create: fake_create,
    process_frame: invalid_warmup_process_frame,
    destroy: fake_destroy,
    init: fake_init,
};

fn fake_denoiser() -> *mut Denoiser {
    let mut denoiser = ptr::null_mut();
    assert_eq!(
        create_with_test_functions(
            &FAKE_FUNCTIONS,
            SAMPLE_RATE_HZ,
            1,
            FRAME_COUNT as u32,
            &mut denoiser,
        ),
        OK
    );
    assert!(!denoiser.is_null());
    denoiser
}

fn fake_state(denoiser: &Denoiser) -> &FakeState {
    unsafe { &*denoiser.state.as_ptr().cast::<FakeState>() }
}

fn fake_state_mut(denoiser: &mut Denoiser) -> &mut FakeState {
    unsafe { &mut *denoiser.state.as_ptr().cast::<FakeState>() }
}

#[test]
fn descriptor_identifies_abi_v1_and_complete_function_table() {
    let descriptor = unsafe { &*super::rptadv_rnnoise_adapter_descriptor() };
    assert_eq!(descriptor.abi_version, 1);
    assert!(descriptor.struct_size as usize >= std::mem::size_of::<AdapterDescriptor>());
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(descriptor.capability_name) }.to_bytes(),
        &super::CAPABILITY_NAME[..super::CAPABILITY_NAME.len() - 1]
    );
    let _ = descriptor.create;
    let _ = descriptor.process;
    let _ = descriptor.destroy;
}

#[test]
fn create_requires_the_fixed_public_pcm_contract_and_clears_output() {
    let sentinel = ptr::dangling_mut::<Denoiser>();
    assert_eq!(
        create_with_test_functions(&FAKE_FUNCTIONS, SAMPLE_RATE_HZ, 1, 480, ptr::null_mut()),
        INVALID_ARGUMENT
    );
    for (sample_rate, channels, frames) in [
        (0, 1, 480),
        (47_999, 1, 480),
        (48_001, 1, 480),
        (48_000, 0, 480),
        (48_000, 2, 480),
        (48_000, 1, 0),
        (48_000, 1, 479),
        (48_000, 1, 481),
    ] {
        let mut denoiser = sentinel;
        assert_eq!(
            create_with_test_functions(
                &FAKE_FUNCTIONS,
                sample_rate,
                channels,
                frames,
                &mut denoiser,
            ),
            UNSUPPORTED
        );
        assert!(denoiser.is_null());
    }
}

#[test]
fn create_checks_the_external_frame_contract_and_setup_failures() {
    let mut denoiser = ptr::dangling_mut::<Denoiser>();
    assert_eq!(
        create_with_test_functions(
            &WRONG_FRAME_FUNCTIONS,
            SAMPLE_RATE_HZ,
            1,
            FRAME_COUNT as u32,
            &mut denoiser,
        ),
        RNNOISE_ERROR
    );
    assert!(denoiser.is_null());
    denoiser = ptr::dangling_mut::<Denoiser>();
    assert_eq!(
        create_with_test_functions(
            &NULL_CREATE_FUNCTIONS,
            SAMPLE_RATE_HZ,
            1,
            FRAME_COUNT as u32,
            &mut denoiser,
        ),
        RNNOISE_ERROR
    );
    assert!(denoiser.is_null());
    denoiser = ptr::dangling_mut::<Denoiser>();
    assert_eq!(
        create_with_test_functions(
            &INVALID_WARMUP_FUNCTIONS,
            SAMPLE_RATE_HZ,
            1,
            FRAME_COUNT as u32,
            &mut denoiser,
        ),
        RNNOISE_ERROR
    );
    assert!(denoiser.is_null());
}

#[test]
fn create_warms_two_silent_frames_with_the_default_model() {
    let denoiser = fake_denoiser();
    let denoiser_ref = unsafe { &*denoiser };
    let state = fake_state(denoiser_ref);
    assert_eq!(state.process_calls, 2);
    assert!(state.model_was_null);
    assert!(state.last_input.iter().all(|sample| *sample == 0.0));
    assert!(
        denoiser_ref
            .output_codes
            .iter()
            .all(|sample| *sample == 0.0)
    );
    destroy(denoiser);
}

#[test]
fn process_scales_normalized_pcm_and_supports_in_place_buffers() {
    let denoiser = fake_denoiser();
    let mut samples = [0.0; FRAME_COUNT];
    samples[0] = -1.0;
    samples[1] = -0.25;
    samples[2] = 0.5;
    samples[3] = 1.0;
    let expected = samples;
    let mut probability = -1.0;

    assert_eq!(
        process(
            denoiser,
            samples.as_ptr(),
            FRAME_COUNT as u32,
            samples.as_mut_ptr(),
            &mut probability,
        ),
        OK
    );
    assert_eq!(probability, 0.625);
    for (actual, source) in samples.iter().zip(expected) {
        assert_eq!(*actual, source * 0.5);
    }
    let state = fake_state(unsafe { &*denoiser });
    assert_eq!(state.process_calls, 3);
    assert_eq!(state.last_input[0], -32_768.0);
    assert_eq!(state.last_input[1], -8_192.0);
    assert_eq!(state.last_input[2], 16_384.0);
    assert_eq!(state.last_input[3], 32_768.0);
    destroy(denoiser);
}

#[test]
fn process_rejects_incomplete_or_invalid_public_requests() {
    let denoiser = fake_denoiser();
    let input = [0.0; FRAME_COUNT];
    let mut output = [0.0; FRAME_COUNT];
    let mut probability = 9.0;

    assert_eq!(
        process(
            ptr::null_mut(),
            input.as_ptr(),
            FRAME_COUNT as u32,
            output.as_mut_ptr(),
            &mut probability,
        ),
        INVALID_ARGUMENT
    );
    assert_eq!(probability, 0.0);
    assert_eq!(
        process(
            denoiser,
            input.as_ptr(),
            FRAME_COUNT as u32,
            output.as_mut_ptr(),
            ptr::null_mut(),
        ),
        INVALID_ARGUMENT
    );
    for (input_pointer, frames, output_pointer) in [
        (ptr::null(), 480, output.as_mut_ptr()),
        (input.as_ptr(), 480, ptr::null_mut()),
        (input.as_ptr(), 0, output.as_mut_ptr()),
        (input.as_ptr(), 479, output.as_mut_ptr()),
        (input.as_ptr(), 481, output.as_mut_ptr()),
    ] {
        probability = 9.0;
        assert_eq!(
            process(
                denoiser,
                input_pointer,
                frames,
                output_pointer,
                &mut probability,
            ),
            INVALID_ARGUMENT
        );
        assert_eq!(probability, 0.0);
    }
    assert_eq!(fake_state(unsafe { &*denoiser }).process_calls, 2);
    destroy(denoiser);
}

#[test]
fn process_rejects_invalid_external_probabilities_without_publishing_output() {
    let denoiser = fake_denoiser();
    let input = [0.25; FRAME_COUNT];
    let mut output = [9.0; FRAME_COUNT];
    for response in [1, 2, 3] {
        fake_state_mut(unsafe { &mut *denoiser }).response = response;
        let mut probability = 9.0;
        assert_eq!(
            process(
                denoiser,
                input.as_ptr(),
                FRAME_COUNT as u32,
                output.as_mut_ptr(),
                &mut probability,
            ),
            RNNOISE_ERROR
        );
        assert_eq!(probability, 0.0);
        assert!(output.iter().all(|sample| *sample == 9.0));
    }
    destroy(denoiser);
}

#[test]
fn probability_validation_accepts_only_finite_unit_interval_values() {
    assert!(valid_probability(0.0));
    assert!(valid_probability(0.5));
    assert!(valid_probability(1.0));
    assert!(!valid_probability(-f32::EPSILON));
    assert!(!valid_probability(1.0 + f32::EPSILON));
    assert!(!valid_probability(f32::NEG_INFINITY));
    assert!(!valid_probability(f32::INFINITY));
    assert!(!valid_probability(f32::NAN));
}

#[test]
fn production_descriptor_processes_one_real_silent_frame() {
    let mut denoiser = ptr::null_mut();
    let input = [0.0; FRAME_COUNT];
    let mut output = [1.0; FRAME_COUNT];
    let mut probability = -1.0;
    assert_eq!(
        create(SAMPLE_RATE_HZ, 1, FRAME_COUNT as u32, &mut denoiser,),
        OK
    );
    assert_eq!(
        process(
            denoiser,
            input.as_ptr(),
            FRAME_COUNT as u32,
            output.as_mut_ptr(),
            &mut probability,
        ),
        OK
    );
    assert!(valid_probability(probability));
    assert!(output.iter().all(|sample| sample.is_finite()));
    destroy(denoiser);
    destroy(ptr::null_mut());
}

thread_local! {
    static LIFECYCLE: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
}

unsafe extern "C" fn fake_init(state: *mut DenoiseState, model: *mut RnnModel) -> c_int {
    let state = unsafe { &mut *state.cast::<FakeState>() };
    state.init_calls += 1;
    state.model_was_null = model.is_null();
    state.last_input.fill(0.0);
    state.init_result
}

#[test]
fn reset_preserves_handle_without_processing() {
    let denoiser = fake_denoiser();
    let upstream = unsafe { (*denoiser).state };
    let lifecycle = LIFECYCLE.with(|counts| counts.get());
    for expected in 1..=3 {
        assert_eq!(reset(denoiser), OK);
        let current = unsafe { &*denoiser };
        assert_eq!(current.state, upstream);
        assert_eq!(fake_state(current).init_calls, expected);
        assert_eq!(fake_state(current).process_calls, 2);
        assert!(fake_state(current).model_was_null);
        assert_eq!(LIFECYCLE.with(|counts| counts.get()), lifecycle);
    }
    destroy(denoiser);
}

#[test]
fn reset_clears_scratch_and_default_model_history() {
    let denoiser = fake_denoiser();
    let input = [0.25; FRAME_COUNT];
    let mut output = [0.0; FRAME_COUNT];
    let mut probability = 0.0;
    assert_eq!(
        process(
            denoiser,
            input.as_ptr(),
            FRAME_COUNT as u32,
            output.as_mut_ptr(),
            &mut probability
        ),
        OK
    );
    assert_eq!(reset(denoiser), OK);
    let current = unsafe { &*denoiser };
    assert!(current.input_codes.iter().all(|v| *v == 0.0));
    assert!(current.output_codes.iter().all(|v| *v == 0.0));
    assert!(fake_state(current).last_input.iter().all(|v| *v == 0.0));
    assert!(fake_state(current).model_was_null);
    assert_eq!(fake_state(current).process_calls, 3);
    destroy(denoiser);
}

#[test]
fn reset_rejects_null_and_reports_init_failure() {
    assert_eq!(reset(ptr::null_mut()), INVALID_ARGUMENT);
    let denoiser = fake_denoiser();
    fake_state_mut(unsafe { &mut *denoiser }).init_result = -1;
    assert_eq!(reset(denoiser), RNNOISE_ERROR);
    fake_state_mut(unsafe { &mut *denoiser }).init_result = 1;
    assert_eq!(reset(denoiser), RNNOISE_ERROR);
    destroy(denoiser);
}

#[test]
fn reset_failure_blocks_process_until_success() {
    let denoiser = fake_denoiser();
    fake_state_mut(unsafe { &mut *denoiser }).init_result = -1;
    let lifecycle = LIFECYCLE.with(|counts| counts.get());
    assert_eq!(reset(denoiser), RNNOISE_ERROR);
    let input = [0.25; FRAME_COUNT];
    let mut output = [9.0; FRAME_COUNT];
    let mut probability = 9.0;
    for _ in 0..2 {
        assert_eq!(
            process(
                denoiser,
                input.as_ptr(),
                FRAME_COUNT as u32,
                output.as_mut_ptr(),
                &mut probability
            ),
            RNNOISE_ERROR
        );
        assert_eq!(probability, 0.0);
        assert_eq!(output, [9.0; FRAME_COUNT]);
        assert_eq!(fake_state(unsafe { &*denoiser }).process_calls, 2);
    }
    fake_state_mut(unsafe { &mut *denoiser }).init_result = 0;
    assert_eq!(reset(denoiser), OK);
    assert_eq!(
        process(
            denoiser,
            input.as_ptr(),
            FRAME_COUNT as u32,
            output.as_mut_ptr(),
            &mut probability
        ),
        OK
    );
    assert_eq!(probability, 0.625);
    assert_eq!(output, [0.125; FRAME_COUNT]);
    assert_eq!(LIFECYCLE.with(|counts| counts.get()), lifecycle);
    destroy(denoiser);
}
