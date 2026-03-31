new Error();
new Error("message");
new Error("message", { cause: "cause" });
new TypeError();
new TypeError("type error");
new SyntaxError();
new SyntaxError("syntax error");
new RangeError();
new RangeError("range error");
new ReferenceError();
new ReferenceError("ref error");
new EvalError();
new EvalError("eval error");
new URIError();
new URIError("uri error");
new AggregateError([], "aggregate error");
new AggregateError([new Error("e1")], "multiple");
const msg = "dynamic";
new Error(msg);
new TypeError(getErrorMessage());
new Date();
new Map();
new Set();
function getErrorMessage() {
    return "computed";
}
