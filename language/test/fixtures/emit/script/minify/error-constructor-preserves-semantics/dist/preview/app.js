// src/app.ts
function captureValue(value) {
  return console.log(value), value;
}
var e1 = new Error("with new"), e2 = Error("without new");
captureValue(e1 instanceof Error);
captureValue(e2 instanceof Error);
captureValue(e1.message === "with new");
captureValue(e2.message === "without new");
captureValue(typeof e1.stack == "string");
captureValue(typeof e2.stack == "string");
var errors = [
  [new TypeError("t1"), TypeError("t2")],
  [new SyntaxError("s1"), SyntaxError("s2")],
  [new RangeError("r1"), RangeError("r2")]
];
for (let [withNew, withoutNew] of errors)
  captureValue(withNew.constructor === withoutNew.constructor);
