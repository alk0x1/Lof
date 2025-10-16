pragma circom 2.0.0;

template ArrayAccess() {
    signal input arr[3];
    signal output first;
    signal output second;
    signal output third;

    first <== arr[0];
    second <== arr[1];
    third <== arr[2];
}

component main = ArrayAccess();
