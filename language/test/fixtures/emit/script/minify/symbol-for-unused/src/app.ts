Symbol.for("test1");
Symbol.for("test2");
Symbol.for(`test3`);
Symbol.for("test" + 4);

const sideEffect = "test" + 4;
Symbol.for(sideEffect);

export const usedSymbols = {
    s1: Symbol.for("used1"),
    s2: Symbol.for("used2"),
    s3: Symbol.for("used3"),
    argument: Symbol.for("argument"),
    property: { prop: Symbol.for("property") },
    returned: (() => Symbol.for("return"))(),
    sideEffect,
};
