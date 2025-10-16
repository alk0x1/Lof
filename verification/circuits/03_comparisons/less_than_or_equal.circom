pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template LessThanOrEqualCircuit() {
    signal input a;
    signal input b;
    signal output result;

    component lte = LessEqThan(252);
    lte.in[0] <== a;
    lte.in[1] <== b;
    result <== lte.out;
}

component main = LessThanOrEqualCircuit();
