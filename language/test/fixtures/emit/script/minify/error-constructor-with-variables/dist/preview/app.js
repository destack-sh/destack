function captureValue(value) {
    console.log(value);
    return value;
}
const e1 = new Error("test1"), e2 = new TypeError("test2"), e3 = new SyntaxError("test3");
captureValue(e1.message);
captureValue(e2.message);
captureValue(e3.message);
captureValue(e1 instanceof Error);
captureValue(e2 instanceof TypeError);
captureValue(e3 instanceof SyntaxError);
try {
    throw new RangeError("out of range");
} catch(e) {
    captureValue(e.message);
}
