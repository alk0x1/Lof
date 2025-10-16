pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template GreaterThanCircuit() {
    signal input a;
    signal input b;
    signal output result;

    component gt = GreaterThan(252);
    gt.in[0] <== a;
    gt.in[1] <== b;
    result <== gt.out;
}

component main = GreaterThanCircuit();
