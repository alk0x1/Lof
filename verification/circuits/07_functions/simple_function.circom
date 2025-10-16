pragma circom 2.0.0;

template Square() {
    signal input x;
    signal output out;
    out <== x * x;
}

template SimpleFunction() {
    signal input a;
    signal output result;

    component sq = Square();
    sq.x <== a;
    result <== sq.out;
}

component main = SimpleFunction();
