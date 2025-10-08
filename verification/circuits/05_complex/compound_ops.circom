pragma circom 2.0.0;

template CompoundOps() {
    signal input a;
    signal input b;
    signal input c;
    signal input d;
    signal output result;
    
    signal term1;
    signal term2;
    
    term1 <== a * b;
    term2 <== c * d;
    result <== term1 + term2;
}

component main = CompoundOps();