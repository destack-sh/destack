'use strict';

require('../common');
const assert = require('assert');
const { isatty } = require('tty');

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="stdin reported to not be a tty, but it is" source_kind=message source_key="stdin reported to not be a tty, but it is" source_hash="fnv1a64:e61e7573a42582e6"
assert.ok(isatty(0), 'stdin reported to not be a tty, but it is');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="stdout reported to not be a tty, but it is" source_kind=message source_key="stdout reported to not be a tty, but it is" source_hash="fnv1a64:e9e19c2e2e2a466a"
assert.ok(isatty(1), 'stdout reported to not be a tty, but it is');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="stderr reported to not be a tty, but it is" source_kind=message source_key="stderr reported to not be a tty, but it is" source_hash="fnv1a64:3877f1c9e44abb08"
assert.ok(isatty(2), 'stderr reported to not be a tty, but it is');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="-1 reported to be a tty, but it is not" source_kind=message source_key="-1 reported to be a tty, but it is not" source_hash="fnv1a64:5520a8f92bedd553"
assert.ok(!isatty(-1), '-1 reported to be a tty, but it is not');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="55555 reported to be a tty, but it is not" source_kind=message source_key="55555 reported to be a tty, but it is not" source_hash="fnv1a64:1d159d7a4448e985"
assert.ok(!isatty(55555), '55555 reported to be a tty, but it is not');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="2^31 reported to be a tty, but it is not" source_kind=message source_key="2^31 reported to be a tty, but it is not" source_hash="fnv1a64:e8990a2f8a2ebb69"
assert.ok(!isatty(2 ** 31), '2^31 reported to be a tty, but it is not');
// test.end

// test.begin: source="pseudo-tty/test-tty-isatty.js" title="1.1 reported to be a tty, but it is not" source_kind=message source_key="1.1 reported to be a tty, but it is not" source_hash="fnv1a64:981c38e5dc8322a3"
assert.ok(!isatty(1.1), '1.1 reported to be a tty, but it is not');
// test.end
