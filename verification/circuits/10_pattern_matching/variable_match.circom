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

    // Break down into quadratic constraints
    signal is_zero_branch;
    signal not_zero_branch;
    signal one_minus_is_zero;

    one_minus_is_zero <== 1 - is_zero.out;
    is_zero_branch <== is_zero.out * y;
    not_zero_branch <== one_minus_is_zero * doubled;
    result <== is_zero_branch + not_zero_branch;
}

component main = VariableMatch();
