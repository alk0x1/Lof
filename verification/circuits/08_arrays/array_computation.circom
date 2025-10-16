pragma circom 2.0.0;

template ArrayComputation() {
    signal input arr[3];
    signal output sum;
    signal output product;

    sum <== arr[0] + arr[1] + arr[2];

    signal temp;
    temp <== arr[0] * arr[1];
    product <== temp * arr[2];
}

component main = ArrayComputation();
