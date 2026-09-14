/**
 * @file descriptor_smoke.c
 * @brief Verify that a C consumer can use the public RNNoise descriptor.
 */

#include <assert.h>
#include <math.h>
#include <string.h>

#include "rptadv_rnnoise_adapter/rptadv_rnnoise_adapter.h"

int main(void)
{
	const struct rptadv_rnnoise_adapter_descriptor *descriptor;
	struct rptadv_rnnoise_denoiser *denoiser = NULL;
	float samples[RPTADV_RNNOISE_FRAME_COUNT] = { 0.0F };
	float vad_probability = -1.0F;

	descriptor = rptadv_rnnoise_adapter_descriptor();
	assert(descriptor != NULL);
	assert(descriptor->abi_version == RPTADV_RNNOISE_ADAPTER_ABI_VERSION);
	assert(descriptor->struct_size >=
	       RPTADV_RNNOISE_ADAPTER_DESCRIPTOR_V1_MIN_SIZE);
	assert(strcmp(descriptor->capability_name,
		      RPTADV_RNNOISE_ADAPTER_CAPABILITY) == 0);
	assert(descriptor->create != NULL);
	assert(descriptor->process != NULL);
	assert(descriptor->destroy != NULL);
	assert(descriptor->create(RPTADV_RNNOISE_SAMPLE_RATE_HZ,
				  RPTADV_RNNOISE_CHANNEL_COUNT,
				  RPTADV_RNNOISE_FRAME_COUNT, &denoiser) ==
	       RPTADV_RNNOISE_ADAPTER_OK);
	assert(denoiser != NULL);
	assert(descriptor->process(denoiser, samples,
				   RPTADV_RNNOISE_FRAME_COUNT, samples,
				   &vad_probability) == RPTADV_RNNOISE_ADAPTER_OK);
	assert(isfinite(vad_probability));
	assert(vad_probability >= 0.0F && vad_probability <= 1.0F);
	descriptor->destroy(denoiser);
	descriptor->destroy(NULL);
	return 0;
}
