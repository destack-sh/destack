// META: title=Headers structure
// META: global=window,worker

"use strict";

// test.begin: source="api/headers/headers-basic.any.js" title="Create headers from no parameter" source_kind=title source_key="Create headers from no parameter" source_hash="fnv1a64:b18f8001f2dc0ebc"
test(function() {
  new Headers();
}, "Create headers from no parameter");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Create headers from undefined parameter" source_kind=title source_key="Create headers from undefined parameter" source_hash="fnv1a64:e7c826d8d55a4989"
test(function() {
  new Headers(undefined);
}, "Create headers from undefined parameter");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Create headers from empty object" source_kind=title source_key="Create headers from empty object" source_hash="fnv1a64:502220e51b0d4bf4"
test(function() {
  new Headers({});
}, "Create headers from empty object");
// test.end

var headerEntriesDict: Record<string, string> = {
  "name1": "value1",
  "Name2": "value2",
  "name": "value3",
  "content-Type": "value4",
  "Content-Typ": "value5",
  "Content-Types": "value6",
};
var sortedHeaderDict: Record<string, string> = {};
var headerValues: string[] = [];
var sortedHeaderKeys = Object.keys(headerEntriesDict).map(function(value) {
  sortedHeaderDict[value.toLowerCase()] = headerEntriesDict[value];
  headerValues.push(headerEntriesDict[value]);
  return value.toLowerCase();
}).sort();

var iteratorPrototype = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));

function checkIteratorProperties(iterator: Iterator<unknown>) {
  var prototype = Object.getPrototypeOf(iterator);
  assert_equals(Object.getPrototypeOf(prototype), iteratorPrototype);

  var descriptor = Object.getOwnPropertyDescriptor(prototype, "next");
  assert_true(!!descriptor, "next descriptor exists");

  if (!descriptor) {
    return;
  }

  assert_true(descriptor.configurable, "configurable");
  assert_true(descriptor.enumerable, "enumerable");
  assert_true(descriptor.writable, "writable");
}

// test.begin: source="api/headers/headers-basic.any.js" title="Check keys method" source_kind=title source_key="Check keys method" source_hash="fnv1a64:3d2ecd9515c687b2"
test(function() {
  var headers = new Headers(headerEntriesDict);
  var actual = headers.keys();
  checkIteratorProperties(actual);

  sortedHeaderKeys.forEach(function(key) {
    const entry = actual.next();
    assert_false(entry.done);

    if (entry.done) {
      return;
    }

    assert_equals(entry.value, key);
  });

  assert_true(actual.next().done);
  assert_true(actual.next().done);

  for (const key of headers.keys()) {
    assert_true(sortedHeaderKeys.indexOf(key) != -1);
  }
}, "Check keys method");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Check values method" source_kind=title source_key="Check values method" source_hash="fnv1a64:a7a9f83da245f5fd"
test(function() {
  var headers = new Headers(headerEntriesDict);
  var actual = headers.values();
  checkIteratorProperties(actual);

  sortedHeaderKeys.forEach(function(key) {
    const entry = actual.next();
    assert_false(entry.done);

    if (entry.done) {
      return;
    }

    assert_equals(entry.value, sortedHeaderDict[key]);
  });

  assert_true(actual.next().done);
  assert_true(actual.next().done);

  for (const value of headers.values()) {
    assert_true(headerValues.indexOf(value) != -1);
  }
}, "Check values method");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Check entries method" source_kind=title source_key="Check entries method" source_hash="fnv1a64:06d7b11331b3d305"
test(function() {
  var headers = new Headers(headerEntriesDict);
  var actual = headers.entries();
  checkIteratorProperties(actual);

  sortedHeaderKeys.forEach(function(key) {
    const entry = actual.next();
    assert_false(entry.done);

    if (entry.done) {
      return;
    }

    assert_equals(entry.value[0], key);
    assert_equals(entry.value[1], sortedHeaderDict[key]);
  });

  assert_true(actual.next().done);
  assert_true(actual.next().done);

  for (const entry of headers.entries()) {
    assert_equals(entry[1], sortedHeaderDict[entry[0]]);
  }
}, "Check entries method");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Check Symbol.iterator method" source_kind=title source_key="Check Symbol.iterator method" source_hash="fnv1a64:7a59ff415a6ae70a"
test(function() {
  var headers = new Headers(headerEntriesDict);
  var actual = headers[Symbol.iterator]();

  sortedHeaderKeys.forEach(function(key) {
    const entry = actual.next();
    assert_false(entry.done);

    if (entry.done) {
      return;
    }

    assert_equals(entry.value[0], key);
    assert_equals(entry.value[1], sortedHeaderDict[key]);
  });

  assert_true(actual.next().done);
  assert_true(actual.next().done);
}, "Check Symbol.iterator method");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Check forEach method" source_kind=title source_key="Check forEach method" source_hash="fnv1a64:6f0b9a910a9467a8"
test(function() {
  var headers = new Headers(headerEntriesDict);
  var reference = sortedHeaderKeys[Symbol.iterator]();

  headers.forEach(function(value, key, container) {
    assert_equals(headers, container);

    const entry = reference.next();
    assert_false(entry.done);

    if (entry.done) {
      return;
    }

    assert_equals(key, entry.value);
    assert_equals(value, sortedHeaderDict[entry.value]);
  });

  assert_true(reference.next().done);
}, "Check forEach method");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Iteration skips elements removed while iterating" source_kind=title source_key="Iteration skips elements removed while iterating" source_hash="fnv1a64:4475de4622ee2a92"
test(() => {
  const headers = new Headers({"foo": "2", "baz": "1", "BAR": "0"});
  const actualKeys: string[] = [];
  const actualValues: string[] = [];

  for (const [header, value] of headers) {
    actualKeys.push(header);
    actualValues.push(value);
    headers.delete("foo");
  }

  assert_array_equals(actualKeys, ["bar", "baz"]);
  assert_array_equals(actualValues, ["0", "1"]);
}, "Iteration skips elements removed while iterating");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Removing elements already iterated over causes an element to be skipped during iteration" source_kind=title source_key="Removing elements already iterated over causes an element to be skipped during iteration" source_hash="fnv1a64:0cb3f44de1ea9d7b"
test(() => {
  const headers = new Headers({"foo": "2", "baz": "1", "BAR": "0", "quux": "3"});
  const actualKeys: string[] = [];
  const actualValues: string[] = [];

  for (const [header, value] of headers) {
    actualKeys.push(header);
    actualValues.push(value);

    if (header === "baz") {
      headers.delete("bar");
    }
  }

  assert_array_equals(actualKeys, ["bar", "baz", "quux"]);
  assert_array_equals(actualValues, ["0", "1", "3"]);
}, "Removing elements already iterated over causes an element to be skipped during iteration");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Appending a value pair during iteration causes it to be reached during iteration" source_kind=title source_key="Appending a value pair during iteration causes it to be reached during iteration" source_hash="fnv1a64:d66f34c005fb5c49"
test(() => {
  const headers = new Headers({"foo": "2", "baz": "1", "BAR": "0", "quux": "3"});
  const actualKeys: string[] = [];
  const actualValues: string[] = [];

  for (const [header, value] of headers) {
    actualKeys.push(header);
    actualValues.push(value);

    if (header === "baz") {
      headers.append("X-yZ", "4");
    }
  }

  assert_array_equals(actualKeys, ["bar", "baz", "foo", "quux", "x-yz"]);
  assert_array_equals(actualValues, ["0", "1", "2", "3", "4"]);
}, "Appending a value pair during iteration causes it to be reached during iteration");
// test.end

// test.begin: source="api/headers/headers-basic.any.js" title="Prepending a value pair before the current element position causes it to be skipped during iteration and adds the current element a second time" source_kind=title source_key="Prepending a value pair before the current element position causes it to be skipped during iteration and adds the current element a second time" source_hash="fnv1a64:ce29931b86a0b7be"
test(() => {
  const headers = new Headers({"foo": "2", "baz": "1", "BAR": "0", "quux": "3"});
  const actualKeys: string[] = [];
  const actualValues: string[] = [];

  for (const [header, value] of headers) {
    actualKeys.push(header);
    actualValues.push(value);

    if (header === "baz") {
      headers.append("abc", "-1");
    }
  }

  assert_array_equals(actualKeys, ["bar", "baz", "baz", "foo", "quux"]);
  assert_array_equals(actualValues, ["0", "1", "1", "2", "3"]);
}, "Prepending a value pair before the current element position causes it to be skipped during iteration and adds the current element a second time");
// test.end
