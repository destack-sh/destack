Symbol.keyFor;
const symbol = Symbol.for(
    "test"
), key = Symbol.keyFor(symbol), otherKey = Symbol.keyFor(Symbol.for("other"));
key;
otherKey;
