pragma circom 2.0.0;

template Equality() {
    signal input a;
    signal input b;
    signal output result;
    
    // Force equality constraint
    a === b;
    result <== 1; // Always output 1 if constraint is satisfied
}

component main = Equality();