/** @file reset_allocation_guard.c
 * @brief Native glibc allocation/free interposer and real-DSO reset regression.
 */
#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "rptadv_rnnoise_adapter/rptadv_rnnoise_adapter.h"

/* Direct glibc entry points avoid resolver allocation inside the armed region. */
extern void *__libc_malloc(size_t size);
extern void *__libc_calloc(size_t count, size_t size);
extern void *__libc_realloc(void *ptr, size_t size);
extern void __libc_free(void *ptr);
extern void *__libc_memalign(size_t alignment, size_t size);
static int armed;
static unsigned allocations;
static unsigned frees;

void *malloc(size_t size)
{
	if (armed) ++allocations;
	return __libc_malloc(size);
}
void *calloc(size_t count, size_t size)
{
	if (armed) ++allocations;
	return __libc_calloc(count, size);
}
void *realloc(void *ptr, size_t size)
{
	if (armed) { ++allocations; if (ptr != NULL) ++frees; }
	return __libc_realloc(ptr, size);
}
void free(void *ptr)
{
	if (armed) ++frees;
	__libc_free(ptr);
}
void *aligned_alloc(size_t alignment, size_t size)
{
	if (armed) ++allocations;
	return __libc_memalign(alignment, size);
}
int posix_memalign(void **ptr, size_t alignment, size_t size)
{
	void *result;
	if (alignment < sizeof(void *) || (alignment & (alignment - 1)) != 0)
		return 22;
	if (armed) ++allocations;
	result = __libc_memalign(alignment, size);
	if (result == NULL) return 12;
	*ptr = result;
	return 0;
}

int main(void)
{
	const struct rptadv_rnnoise_adapter_descriptor *d = rptadv_rnnoise_adapter_descriptor();
	struct rptadv_rnnoise_denoiser *dirty = NULL, *reference = NULL, *setup_control = NULL;
	float input[480], output[480], expected[480];
	float probability, expected_probability;
	void *control;
	assert(d != NULL && d->struct_size >= RPTADV_RNNOISE_ADAPTER_DESCRIPTOR_RESET_MIN_SIZE);
	assert(d->reset != NULL);
	assert(d->create(48000, 1, 480, &dirty) == 0);
	assert(d->create(48000, 1, 480, &reference) == 0);
	/* This nonzero caller PCM is test stimulus, never a reset-time observation. */
	for (unsigned i = 0; i < 480; ++i)
		input[i] = ((float)(i % 31) - 15.0F) / 32.0F;
	armed = 1;
	control = malloc(37);
	if (control != NULL) *(volatile unsigned char *)control = 0;
	free(control);
	armed = 0;
	assert(allocations > 0 && frees > 0);
	allocations = frees = 0;
	/* Prove allocation interception also reaches the actual linked DSOs. */
	armed = 1;
	int setup_result = d->create(48000, 1, 480, &setup_control);
	d->destroy(setup_control);
	armed = 0;
	assert(setup_result == 0 && allocations >= 2 && frees >= 2);
	allocations = frees = 0;
	for (unsigned iteration = 0; iteration < 256; ++iteration) {
		assert(d->process(dirty, input, 480, output, &probability) == 0);
		armed = 1;
		int reset_result = d->reset(dirty);
		int reference_reset = d->reset(reference);
		int result = d->process(dirty, input, 480, output, &probability);
		int reference_result = d->process(reference, input, 480, expected, &expected_probability);
		armed = 0;
		assert(reset_result == 0 && reference_reset == 0);
		assert(result == 0 && reference_result == 0);
		assert(allocations == 0 && frees == 0);
		assert(probability == expected_probability);
		assert(memcmp(output, expected, sizeof(output)) == 0);
	}
	d->destroy(dirty);
	d->destroy(reference);
	puts("reset guard: 256 dirty/reset/first-input equivalence cycles, zero allocation/free; positive control passed");
	return 0;
}
