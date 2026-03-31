declare const someGlobal: string;

Symbol.for("remove-in-prod");

const symbol = Symbol.for("keep-in-prod");

Symbol.for(someGlobal);

capture(symbol);
