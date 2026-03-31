function captureValue(value) {
    console.log(value);
    return value;
}
const a1 = new Array(1, 2, 3), a2 = Array(1, 2, 3);
captureValue(JSON.stringify(a1) === JSON.stringify(a2));
captureValue(a1.constructor === a2.constructor);
const sparse = new Array(5);
captureValue(sparse.length === 5);
captureValue(0 in (sparse === !1));
captureValue(JSON.stringify(sparse) === "[null,null,null,null,null]");
const n = 3, a3 = new Array(n), a4 = Array(n);
captureValue(a3.length === a4.length && a3.length === 3 && a3[0] === void 0);
const o1 = new Object(), o2 = Object();
captureValue(typeof o1 === typeof o2);
captureValue(o1.constructor === o2.constructor);
const f1 = new Function("return 1"), f2 = Function("return 1");
captureValue(typeof f1 === typeof f2);
captureValue(f1() === f2());
const r1 = new RegExp("test", "g"), r2 = RegExp("test", "g");
captureValue(r1.source === r2.source);
captureValue(r1.flags === r2.flags);
