declare function computeSomethingUnknown();
declare const unbound, other;
let flag = computeSomethingUnknown();
flag === 1234?"a":"b", "c";
flag == 1234?"a":"b", "c";
unbound?"a":"b", "c";
flag == 1234?"a":unbound, "c";
[flag == 1234]?unbound:other, "c";
new Date(), 123;
const funcWithNoSideEffects = function() {
    return 1;
};
funcWithNoSideEffects(), 456;
