Symbol.keyFor;

const symbol = Symbol.for("test");
const key = Symbol.keyFor(symbol);
const otherKey = Symbol.keyFor(Symbol.for("other"));

capture(key);
capture(otherKey);
