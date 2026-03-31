Symbol?.for("optional1");
Symbol?.for?.("optional2");
Symbol.for("conditional1");
Symbol.for("conditional2");
Symbol.for("ternary1");
Symbol.for("ternary2");
Symbol.for(Symbol.for("nested"));
export const symbolForEdgeCases = {
    spread: [...[Symbol.for("spread")]],
    keyedRecord: { [Symbol.for("key")]: "value" },
};
