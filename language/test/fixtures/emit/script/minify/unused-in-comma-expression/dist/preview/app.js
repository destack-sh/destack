// src/app.ts
var flag = computeSomethingUnknown();
capture("c");
capture((flag == 1234, "c"));
capture((unbound, "c"));
capture((flag == 1234 || unbound, "c"));
capture((flag == 1234, unbound, "c"));
capture(123);
var funcWithNoSideEffects = () => 1;
capture(456);
