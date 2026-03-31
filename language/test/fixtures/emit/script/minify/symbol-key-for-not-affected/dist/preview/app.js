Symbol.keyFor;
const symbol = Symbol.for("test"), key = Symbol.keyFor(symbol);
key;
