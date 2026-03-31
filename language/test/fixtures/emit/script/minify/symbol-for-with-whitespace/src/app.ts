Symbol.for("remove-me-1");
Symbol.for("remove-me-2");

const symbol = Symbol.for("keep-me");

const ab = "a" + "b";
Symbol.for(ab);
Symbol.for(`template`);

capture(symbol, ab);
