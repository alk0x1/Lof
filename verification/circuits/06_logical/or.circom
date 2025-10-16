pragma circom 2.0.0;

template Or() {
    signal input a;
    signal input b;
    signal output result;

    result <== a + b - a * b;
}

component main = Or();
