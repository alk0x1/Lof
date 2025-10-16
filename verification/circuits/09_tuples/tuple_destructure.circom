pragma circom 2.0.0;

template TupleDestructure() {
    signal input pair_0;
    signal input pair_1;
    signal output first;
    signal output second;

    first <== pair_0;
    second <== pair_1;
}

component main = TupleDestructure();
