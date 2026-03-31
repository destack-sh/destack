Symbol?.for("optional1");
Symbol?.for?.("optional2");
true && Symbol.for("conditional1");
false || Symbol.for("conditional2");
true ? Symbol.for("ternary1") : null;
false ? null : Symbol.for("ternary2");
Symbol.for(Symbol.for("nested"));

export const symbolForEdgeCases = {
    spread: [...[Symbol.for("spread")]],
    keyedRecord: {
        [Symbol.for("key")]: "value",
    },
};
