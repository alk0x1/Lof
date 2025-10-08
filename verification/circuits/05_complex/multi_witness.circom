pragma circom 2.0.0;

template MultiWitness() {
    signal input a;
    signal input b;
    signal output x;
    signal output y;
    signal output z;
    
    x <== a + b;
    y <== a * b;
    z <== a - b;
}

component main = MultiWitness();