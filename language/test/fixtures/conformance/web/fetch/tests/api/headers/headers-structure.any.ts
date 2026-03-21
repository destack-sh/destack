// META: title=Headers basic
// META: global=window,worker

"use strict";

var headers = new Headers();

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has append method" source_kind=template-item source_key="append" source_hash="fnv1a64:513ab0f84b789a36"
test(function() {
  assert_true("append" in headers, "headers has append method");
}, "Headers has append method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has delete method" source_kind=template-item source_key="delete" source_hash="fnv1a64:a216b8d5e6a34bad"
test(function() {
  assert_true("delete" in headers, "headers has delete method");
}, "Headers has delete method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has get method" source_kind=template-item source_key="get" source_hash="fnv1a64:88ca69c269e1931a"
test(function() {
  assert_true("get" in headers, "headers has get method");
}, "Headers has get method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has has method" source_kind=template-item source_key="has" source_hash="fnv1a64:7ba65e310b3b7e76"
test(function() {
  assert_true("has" in headers, "headers has has method");
}, "Headers has has method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has set method" source_kind=template-item source_key="set" source_hash="fnv1a64:dbdf6c3dcb0d285e"
test(function() {
  assert_true("set" in headers, "headers has set method");
}, "Headers has set method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has entries method" source_kind=template-item source_key="entries" source_hash="fnv1a64:cb261c2c01249136"
test(function() {
  assert_true("entries" in headers, "headers has entries method");
}, "Headers has entries method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has keys method" source_kind=template-item source_key="keys" source_hash="fnv1a64:3f1c946b0016e0a2"
test(function() {
  assert_true("keys" in headers, "headers has keys method");
}, "Headers has keys method");
// test.end

// test.begin: source="api/headers/headers-structure.any.js" title="Headers has values method" source_kind=template-item source_key="values" source_hash="fnv1a64:a91e6a6c91ccb590"
test(function() {
  assert_true("values" in headers, "headers has values method");
}, "Headers has values method");
// test.end
