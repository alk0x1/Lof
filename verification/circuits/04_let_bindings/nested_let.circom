pragma circom 2.0.0;

template NestedLet() {
    signal input a;
    signal input b;
    signal input c;
    signal output result;
    
    signal x;
    signal y;
    
    x <== a + b;
    y <== x * c;
    result <== y - a;
}

component main = NestedLet();