pragma circom 2.0.0;

include "circomlib/circuits/comparators.circom";

template LessThanCircuit() {
    signal input a;
    signal input b;
    signal output result;

    component lt = LessThan(252); // 252-bit comparison
    lt.in[0] <== a;
    lt.in[1] <== b;
    result <== lt.out;
}

component main = LessThanCircuit();