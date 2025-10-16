pragma circom 2.0.0;

template Divide() {
    signal input a;
    signal input b;
    signal output c;

    c <-- a / b;
    c * b === a;
}

component main = Divide();
