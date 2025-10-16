pragma circom 2.0.0;

include "../../node_modules/circomlib/circuits/comparators.circom";

template SimpleMatch() {
    signal input x;
    signal output result;

    component is_zero = IsZero();
    component is_one = IsEqual();

    is_zero.in <== x;
    is_one.in[0] <== x;
    is_one.in[1] <== 1;

    signal case0;
    signal case1;
    signal case_default;

    case0 <== is_zero.out * 100;
    case1 <== is_one.out * 200;
    case_default <== (1 - is_zero.out) * (1 - is_one.out) * 300;

    result <== case0 + case1 + case_default;
}

component main = SimpleMatch();
