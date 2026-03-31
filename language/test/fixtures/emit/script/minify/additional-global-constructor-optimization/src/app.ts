capture(new Array());
capture(new Array(3));
capture(new Array(1, 2, 3));
capture(new Array("string"));
capture(new Array(true));
capture(new Array(null));
capture(new Array(undefined));
capture(new Array({}));

capture(new Object());
capture(new Object(null));
capture(new Object({ a: 1 }));

capture(new Function("return 42"));
capture(new Function("a", "b", "return a + b"));

capture(new RegExp("test"));
capture(new RegExp("test", "gi"));
capture(new RegExp(/abc/));

const pattern = "\\d+";
capture(new RegExp(pattern));

capture(new Date());
capture(new Map());
capture(new Set());
