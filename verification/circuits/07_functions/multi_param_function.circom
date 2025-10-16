pragma circom 2.0.0;

template MultiplyAndAdd() {
    signal input a;
    signal input b;
    signal input c;
    signal output out;

    signal product;
    product <== a * b;
    out <== product + c;
}

template MultiParamFunction() {
    signal input x;
    signal input y;
    signal input z;
    signal output result;

    component calc = MultiplyAndAdd();
    calc.a <== x;
    calc.b <== y;
    calc.c <== z;
    result <== calc.out;
}

component main = MultiParamFunction();
