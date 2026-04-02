// src/app.ts
capture(new Error());
capture(new Error("message"));
capture(new Error("message", { cause: "cause" }));
capture(new TypeError());
capture(new TypeError("type error"));
capture(new SyntaxError());
capture(new SyntaxError("syntax error"));
capture(new RangeError());
capture(new RangeError("range error"));
capture(new ReferenceError());
capture(new ReferenceError("ref error"));
capture(new EvalError());
capture(new EvalError("eval error"));
capture(new URIError());
capture(new URIError("uri error"));
capture(new AggregateError([], "aggregate error"));
capture(new AggregateError([new Error("e1")], "multiple"));
var msg = "dynamic";
capture(new Error(msg));
capture(new TypeError(getErrorMessage()));
capture(/* @__PURE__ */ new Date());
capture(/* @__PURE__ */ new Map());
capture(/* @__PURE__ */ new Set());
function getErrorMessage() {
  return "computed";
}
