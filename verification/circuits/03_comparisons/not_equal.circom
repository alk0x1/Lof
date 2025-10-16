pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template NotEqual() {
    signal input a;
    signal input b;
    signal output result;

    component eq = IsEqual();
    eq.in[0] <== a;
    eq.in[1] <== b;
    result <== 1 - eq.out;
}

component main = NotEqual();
