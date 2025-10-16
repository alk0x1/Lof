pragma circom 2.0.0;

template And() {
    signal input a;
    signal input b;
    signal output result;

    result <== a * b;
}

component main = And();
