declare function capture(value: unknown): void;

const inlineArraySpreadSamples = [
    [1, 2, ...[3, 4], 5, 6, ...[7, ...[...[...[...[8, 9]]]]], 10, ...[...[...[...[...[...[...[11]]]]]]]],
    [1, 2, ...[3, 4], 5, 6, ...[7, [...[...[...[8, 9]]]]], 10, ...[...[...[...[...[...[...11]]]]]]],
];

switch (1) {
    case undefined: {
        capture("unreachable");
    }
}

const symbol = Symbol.for("inline-array-spread-cluster");
const symbolKey = Symbol.keyFor(symbol);

capture(inlineArraySpreadSamples);
capture(symbolKey);
