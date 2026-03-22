/*---
description: Temporal.Duration throws a RangeError if any value is Infinity
esid: sec-temporal.duration
includes: [compareArray.js, temporalHelpers.js]
features: [Temporal]
---*/

// test.begin: source="Duration/infinity-throws-rangeerror.js" title="constructor positional infinity inputs" source_kind=statement-group source_key="assert.throws(RangeError, () => new Temporal.Duration(Infinity))" source_hash="fnv1a64:5cc5d72c125d47f5"
assert.throws(RangeError, () => new Temporal.Duration(Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, 0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, 0, 0, 0, 0, Infinity));
assert.throws(RangeError, () => new Temporal.Duration(0, 0, 0, 0, 0, 0, 0, 0, 0, Infinity));
// test.end

const O = (primitiveValue, propertyName) => (calls) =>
  TemporalHelpers.toPrimitiveObserver(calls, primitiveValue, propertyName);

// test.begin: source="Duration/infinity-throws-rangeerror.js" title="infinite years" source_kind=table-row source_key="infinite years" source_hash="fnv1a64:69a8268519efe92b"
{
  const actual: string[] = [];
  const args = [
    O(Infinity, "years"),
    O(0, "months"),
    O(0, "weeks"),
    O(0, "days"),
    O(0, "hours"),
    O(0, "minutes"),
    O(0, "seconds"),
    O(0, "milliseconds"),
    O(0, "microseconds"),
    O(0, "nanoseconds"),
  ];
  const expected = ["get years.valueOf", "call years.valueOf"];

  const args_ = args.map((observer) => observer(actual));
  assert.throws(RangeError, () => new Temporal.Duration(...args_), "infinite years");
  assert.compareArray(actual, expected, "infinite years order of operations");
}
// test.end

// test.begin: source="Duration/infinity-throws-rangeerror.js" title="infinite months" source_kind=table-row source_key="infinite months" source_hash="fnv1a64:7be33fe45fe89c11"
{
  const actual: string[] = [];
  const args = [
    O(0, "years"),
    O(Infinity, "months"),
    O(0, "weeks"),
    O(0, "days"),
    O(0, "hours"),
    O(0, "minutes"),
    O(0, "seconds"),
    O(0, "milliseconds"),
    O(0, "microseconds"),
    O(0, "nanoseconds"),
  ];
  const expected = [
    "get years.valueOf",
    "call years.valueOf",
    "get months.valueOf",
    "call months.valueOf",
  ];

  const args_ = args.map((observer) => observer(actual));
  assert.throws(RangeError, () => new Temporal.Duration(...args_), "infinite months");
  assert.compareArray(actual, expected, "infinite months order of operations");
}
// test.end
