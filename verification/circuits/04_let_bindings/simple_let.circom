pragma circom 2.0.0;

template SimpleLet() {
    signal input a;
    signal input b;
    signal output result;
    
    signal temp;
    temp <== a + b;
    result <== temp * a;
}

component main = SimpleLet();