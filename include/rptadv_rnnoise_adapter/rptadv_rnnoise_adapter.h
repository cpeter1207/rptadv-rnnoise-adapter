/**
 * @file rptadv_rnnoise_adapter.h
 * @brief Stable C descriptor for the fixed-format RNNoise adapter.
 */

#ifndef RPTADV_RNNOISE_ADAPTER_H
#define RPTADV_RNNOISE_ADAPTER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** @brief ABI version implemented by this header and adapter. */
#define RPTADV_RNNOISE_ADAPTER_ABI_VERSION 1U
/** @brief Stable capability name reported by the descriptor. */
#define RPTADV_RNNOISE_ADAPTER_CAPABILITY "rptadv.rnnoise"
/** @brief Required RNNoise PCM sample rate. */
#define RPTADV_RNNOISE_SAMPLE_RATE_HZ 48000U
/** @brief Required RNNoise PCM channel count. */
#define RPTADV_RNNOISE_CHANNEL_COUNT 1U
/** @brief Required samples in one mono RNNoise process frame. */
#define RPTADV_RNNOISE_FRAME_COUNT 480U

/** @brief Results returned by RNNoise adapter operations. */
enum rptadv_rnnoise_adapter_result {
	/** Operation completed successfully. */
	RPTADV_RNNOISE_ADAPTER_OK = 0,
	/** A required pointer or process frame count was invalid. */
	RPTADV_RNNOISE_ADAPTER_INVALID_ARGUMENT = -1,
	/** The linked RNNoise implementation failed or violated its contract. */
	RPTADV_RNNOISE_ADAPTER_RNNOISE_ERROR = -2,
	/** The requested PCM rate, channel count, or setup frame size is unsupported. */
	RPTADV_RNNOISE_ADAPTER_UNSUPPORTED = -3,
};

/**
 * @brief Persistent RNNoise state owned by the adapter.
 *
 * The representation is private. A caller owns only the pointer returned by
 * @ref rptadv_rnnoise_adapter_descriptor::create and serializes all access to
 * that handle.
 */
struct rptadv_rnnoise_denoiser;

/**
 * @brief Versioned function table exported by the adapter shared object.
 *
 * Consumers verify the capability string, exact ABI version, minimum readable
 * descriptor prefix, and every required function pointer before using this
 * table. Creation and destruction are control-plane operations. @ref process
 * is the only real-time operation.
 */
struct rptadv_rnnoise_adapter_descriptor {
	/** Size of this descriptor, enabling compatible append-only extension. */
	uint32_t struct_size;
	/** ABI implemented by every function in this table. */
	uint32_t abi_version;
	/** Stable adapter capability name. */
	const char *capability_name;
	/**
	 * @brief Create one default-model fixed-format denoiser.
	 *
	 * @param sample_rate_hz Must equal @ref RPTADV_RNNOISE_SAMPLE_RATE_HZ.
	 * @param channel_count Must equal @ref RPTADV_RNNOISE_CHANNEL_COUNT.
	 * @param frame_count Must equal @ref RPTADV_RNNOISE_FRAME_COUNT.
	 * @param out_denoiser Destination for a newly owned handle on success.
	 * @return One @ref rptadv_rnnoise_adapter_result value.
	 *
	 * The destination is set to null before any fallible setup. Creation
	 * allocates the upstream state and adapter scratch storage, verifies the
	 * linked library's frame size, and processes two silent upstream frames on
	 * the control plane. This preserves the existing USBRadioPlus model setup
	 * without consuming caller audio or reporting warm-up voice activity.
	 */
	enum rptadv_rnnoise_adapter_result (*create)(
		uint32_t sample_rate_hz, uint32_t channel_count,
		uint32_t frame_count,
		struct rptadv_rnnoise_denoiser **out_denoiser);
	/**
	 * @brief Process exactly one prepared normalized mono F32 frame.
	 *
	 * @param denoiser Denoiser obtained from @ref create.
	 * @param input Exactly @ref RPTADV_RNNOISE_FRAME_COUNT readable samples.
	 * @param frame_count Must equal @ref RPTADV_RNNOISE_FRAME_COUNT.
	 * @param output Exactly @ref RPTADV_RNNOISE_FRAME_COUNT writable samples.
	 * @param out_vad_probability Required destination for speech probability.
	 * @return One @ref rptadv_rnnoise_adapter_result value.
	 *
	 * Input and output are canonical normalized F32 PCM with a full-scale
	 * reference of -1.0 through +1.0. The adapter privately converts to and from
	 * RNNoise's PCM-code scale. Input and output may overlap because the complete
	 * input frame is staged before any caller output is written. On success,
	 * @p out_vad_probability is finite and within zero through one. When that
	 * pointer is valid it is cleared to zero before later validation or failure.
	 *
	 * Setup preallocates all adapter and upstream state. This operation does not
	 * allocate, lock, wait, perform I/O, or emit adapter log messages. The caller
	 * serializes calls for one handle. Callback partition assembly, output
	 * priming, bypass policy, and counters remain consumer-owned behavior.
	 */
	enum rptadv_rnnoise_adapter_result (*process)(
		struct rptadv_rnnoise_denoiser *denoiser, const float *input,
		uint32_t frame_count, float *output,
		float *out_vad_probability);
	/**
	 * @brief Destroy a denoiser after all callers have stopped using it.
	 *
	 * @param denoiser Denoiser obtained from @ref create, or null.
	 */
	void (*destroy)(struct rptadv_rnnoise_denoiser *denoiser);
};

/**
 * @brief Minimum readable size of an ABI-v1 descriptor.
 *
 * Consumers require @c struct_size to be at least this value rather than
 * requiring equality, so a future provider may append fields while retaining
 * the ABI-v1 prefix.
 */
#define RPTADV_RNNOISE_ADAPTER_DESCRIPTOR_V1_MIN_SIZE \
	(offsetof(struct rptadv_rnnoise_adapter_descriptor, destroy) + \
	 sizeof(((struct rptadv_rnnoise_adapter_descriptor *)0)->destroy))

/*
 * Public enum parameters use the platform C-int ABI. Reject compilation modes
 * such as -fshort-enums that would be incompatible with Rust's c_int layout.
 */
#if defined(__cplusplus)
static_assert(sizeof(enum rptadv_rnnoise_adapter_result) == sizeof(int),
	      "adapter result enum must use the C int ABI");
static_assert(RPTADV_RNNOISE_ADAPTER_OK == 0,
	      "adapter result values are part of ABI v1");
static_assert(RPTADV_RNNOISE_ADAPTER_INVALID_ARGUMENT == -1,
	      "adapter result values are part of ABI v1");
static_assert(RPTADV_RNNOISE_ADAPTER_RNNOISE_ERROR == -2,
	      "adapter result values are part of ABI v1");
static_assert(RPTADV_RNNOISE_ADAPTER_UNSUPPORTED == -3,
	      "adapter result values are part of ABI v1");
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(enum rptadv_rnnoise_adapter_result) == sizeof(int),
	       "adapter result enum must use the C int ABI");
_Static_assert(RPTADV_RNNOISE_ADAPTER_OK == 0,
	       "adapter result values are part of ABI v1");
_Static_assert(RPTADV_RNNOISE_ADAPTER_INVALID_ARGUMENT == -1,
	       "adapter result values are part of ABI v1");
_Static_assert(RPTADV_RNNOISE_ADAPTER_RNNOISE_ERROR == -2,
	       "adapter result values are part of ABI v1");
_Static_assert(RPTADV_RNNOISE_ADAPTER_UNSUPPORTED == -3,
	       "adapter result values are part of ABI v1");
#endif

/**
 * @brief Return the immutable ABI-v1 RNNoise adapter descriptor.
 *
 * @return A process-lifetime descriptor; it must not be freed or modified.
 */
const struct rptadv_rnnoise_adapter_descriptor *
rptadv_rnnoise_adapter_descriptor(void);

#ifdef __cplusplus
}
#endif

#endif
