extern "C" __global__
void simulated_task(float* data, int iterations, int n)
{
    int idx = blockIdx.x * blockDim.x + threadIdx.x;

    if (idx < n) {
        float x = data[idx];

        for (int i = 0; i < iterations; i++) {
            x = x * 1.000001f + 0.000001f;
        }

        data[idx] = x;
    }
}