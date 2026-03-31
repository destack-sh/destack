declare function computeSomethingUnknown(): unknown;
declare const unbound: unknown;
declare const other: unknown;

let flag = computeSomethingUnknown();

capture((flag === 1234 ? "a" : "b", "c"));
capture((flag == 1234 ? "a" : "b", "c"));
capture((unbound ? "a" : "b", "c"));
capture((flag == 1234 ? "a" : unbound, "c"));
capture(([flag == 1234] ? unbound : other, "c"));
capture((new Date(), 123));

const funcWithNoSideEffects = () => 1;
capture((/* @__PURE__ */ funcWithNoSideEffects(), 456));
