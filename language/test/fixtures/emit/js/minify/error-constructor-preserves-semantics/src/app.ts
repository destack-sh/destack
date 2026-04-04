function captureValue(value) {
    console.log(value);
    return value;
}

const e1 = new Error("with new");
const e2 = Error("without new");

captureValue(e1 instanceof Error);
captureValue(e2 instanceof Error);
captureValue(e1.message === "with new");
captureValue(e2.message === "without new");
captureValue(typeof e1.stack === "string");
captureValue(typeof e2.stack === "string");

const errors = [
    [new TypeError("t1"), TypeError("t2")],
    [new SyntaxError("s1"), SyntaxError("s2")],
    [new RangeError("r1"), RangeError("r2")]
];

for (const [withNew, withoutNew] of errors) {
    captureValue(withNew.constructor === withoutNew.constructor);
}
