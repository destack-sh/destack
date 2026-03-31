let callCount = 0;
const originalSymbolFor = Symbol.for;

Symbol.for = function (key) {
    callCount++;
    return originalSymbolFor.call(Symbol, key);
};

Symbol.for("unused1");
Symbol.for("unused2");

const s1 = Symbol.for("used1");
const s2 = Symbol.for("used2");

Symbol.for = originalSymbolFor;

if (callCount !== 2) {
    throw new Error(`Expected 2 calls to Symbol.for, got ${callCount}`);
}

if (s1 !== Symbol.for("used1")) {
    throw new Error("Symbol s1 mismatch");
}
if (s2 !== Symbol.for("used2")) {
    throw new Error("Symbol s2 mismatch");
}

console.log("PASS");
