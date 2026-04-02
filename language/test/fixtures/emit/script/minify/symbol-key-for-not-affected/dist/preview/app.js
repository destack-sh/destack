Symbol.keyFor;var symbol=Symbol.for("test"),key=Symbol.keyFor(symbol),otherKey=Symbol.keyFor(Symbol.for("other"));capture(key);capture(otherKey);
