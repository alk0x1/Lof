pragma circom 2.0.0;

template Not() {
    signal input a;
    signal output result;

    result <== 1 - a;
}

component main = Not();
