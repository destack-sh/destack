let assignment = /** @type {string} */ getValue();
var newArray = /** @type {array} */ numberOrString.map((x) => x);
/* 2 */ /** @type {{bar: string[]}} */ ({}).bar.forEach(doStuff);
/** @type {{bar: string[]}} */ /* 2 */ ({}).bar.forEach(doStuff);

(
    @deco
    class Foo {}
).name;
(
    @deco
    class {}
).name;
