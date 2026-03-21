// META: title=Blob Array Buffer
// META: script=../support/Blob.js
'use strict';

// test.begin: source="blob/Blob-array-buffer.any.js" title="Blob.arrayBuffer()" source_kind=title source_key="Blob.arrayBuffer()" source_hash="fnv1a64:445821c4e0fe3079"
promise_test(async () => {
  const input_arr = new TextEncoder().encode("PASS");
  const blob = new Blob([input_arr]);
  const array_buffer = await blob.arrayBuffer();
  assert_true(array_buffer instanceof ArrayBuffer);
  assert_equals_typed_array(new Uint8Array(array_buffer), input_arr);
}, "Blob.arrayBuffer()");
// test.end

// test.begin: source="blob/Blob-array-buffer.any.js" title="Blob.arrayBuffer() empty Blob data" source_kind=title source_key="Blob.arrayBuffer() empty Blob data" source_hash="fnv1a64:2f1ecb69db9b0ed2"
promise_test(async () => {
  const input_arr = new TextEncoder().encode("");
  const blob = new Blob([input_arr]);
  const array_buffer = await blob.arrayBuffer();
  assert_true(array_buffer instanceof ArrayBuffer);
  assert_equals_typed_array(new Uint8Array(array_buffer), input_arr);
}, "Blob.arrayBuffer() empty Blob data");
// test.end

// test.begin: source="blob/Blob-array-buffer.any.js" title="Blob.arrayBuffer() non-ascii input" source_kind=title source_key="Blob.arrayBuffer() non-ascii input" source_hash="fnv1a64:4388cd971eb7d7f8"
promise_test(async () => {
  const input_arr = new TextEncoder().encode("\u08B8\u000a");
  const blob = new Blob([input_arr]);
  const array_buffer = await blob.arrayBuffer();
  assert_equals_typed_array(new Uint8Array(array_buffer), input_arr);
}, "Blob.arrayBuffer() non-ascii input");
// test.end

// test.begin: source="blob/Blob-array-buffer.any.js" title="Blob.arrayBuffer() non-unicode input" source_kind=title source_key="Blob.arrayBuffer() non-unicode input" source_hash="fnv1a64:791d65c939821849"
promise_test(async () => {
  const input_arr = [8, 241, 48, 123, 151];
  const typed_arr = new Uint8Array(input_arr);
  const blob = new Blob([typed_arr]);
  const array_buffer = await blob.arrayBuffer();
  assert_equals_typed_array(new Uint8Array(array_buffer), typed_arr);
}, "Blob.arrayBuffer() non-unicode input");
// test.end

// test.begin: source="blob/Blob-array-buffer.any.js" title="Blob.arrayBuffer() concurrent reads" source_kind=title source_key="Blob.arrayBuffer() concurrent reads" source_hash="fnv1a64:f51031846b396c3c"
promise_test(async () => {
  const input_arr = new TextEncoder().encode("PASS");
  const blob = new Blob([input_arr]);
  const array_buffer_results = await Promise.all([
    blob.arrayBuffer(),
    blob.arrayBuffer(),
    blob.arrayBuffer(),
  ]);

  for (const array_buffer of array_buffer_results) {
    assert_true(array_buffer instanceof ArrayBuffer);
    assert_equals_typed_array(new Uint8Array(array_buffer), input_arr);
  }
}, "Blob.arrayBuffer() concurrent reads");
// test.end
