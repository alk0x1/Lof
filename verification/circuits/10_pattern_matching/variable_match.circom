pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template VariableMatch() {
    signal input x;
    signal input y;
    signal output result;

    component is_zero = IsZero();
    is_zero.in <== x;

    signal doubled;
    doubled <== x * 2;

    result <== is_zero.out * y + (1 - is_zero.out) * doubled;
}

component main = VariableMatch();
