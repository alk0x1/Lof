pragma circom 2.0.0;

template TupleOperations() {
    signal input pair1_0;
    signal input pair1_1;
    signal input pair2_0;
    signal input pair2_1;
    signal output sum;
    signal output product;

    sum <== pair1_0 + pair1_1 + pair2_0 + pair2_1;

    signal prod1;
    signal prod2;
    prod1 <== pair1_0 * pair1_1;
    prod2 <== pair2_0 * pair2_1;
    product <== prod1 + prod2;
}

component main = TupleOperations();
