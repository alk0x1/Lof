pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template GreaterThanOrEqualCircuit() {
    signal input a;
    signal input b;
    signal output result;

    component gte = GreaterEqThan(252);
    gte.in[0] <== a;
    gte.in[1] <== b;
    result <== gte.out;
}

component main = GreaterThanOrEqualCircuit();
