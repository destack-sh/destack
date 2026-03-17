# CommonJS Interop

These tests define the static CommonJS interop surface for module export resolution.
The scope is intentionally static and top level only.

## default imports

### default imports read direct module exports assignment

> A top level `module.exports = value` assignment defines the default import value.

```js:cjs.js
function buildValue() {
    return 1;
}

module.exports = buildValue;
```

```ts:main.ts
import buildValue from "./cjs";

buildValue();
```

### default imports read bracket module exports assignment

> A top level `module["exports"] = value` assignment defines the default import value.

```js:cjs.js
function buildValue() {
    return 1;
}

module["exports"] = buildValue;
```

```ts:main.ts
import buildValue from "./cjs";

buildValue();
```

### default imports use the latest top level module exports assignment

> Later top level `module.exports` assignments override earlier ones.

```js:cjs.js
function first() {
    return 1;
}

function second() {
    return 2;
}

module.exports = first;
module.exports = second;
```

```ts:main.ts
import selected from "./cjs";

selected();
```

### exports reassignment does not replace module exports

> Reassigning `exports` alone does not replace `module.exports`.

```js:cjs.js
function fromModule() {
    return 1;
}

function fromExports() {
    return 2;
}

module.exports = fromModule;
exports = fromExports;
```

```ts:main.ts
import selected from "./cjs";

selected();
```

### typescript strict mode rejects synthetic default imports from named only commonjs exports

> Without interop options, TypeScript default imports require an explicit default export target.

```js:cjs.js
function buildValue() {
    return 1;
}

exports.buildValue = buildValue;
```

```ts:main.ts
import cjs from "./cjs";

cjs.buildValue();
```

- contains: does not exist on type unknown

### es module interop enables synthetic default imports from named only commonjs exports

> `compilerOptions.esModuleInterop` allows default imports to bind to the CommonJS namespace shape.

```json:destack.json
{ "compilerOptions": { "esModuleInterop": true } }
```

```js:cjs.js
function buildValue() {
    return 1;
}

exports.buildValue = buildValue;
```

```ts:main.ts
import cjs from "./cjs";

cjs.buildValue();
```

### allow synthetic default imports enables synthetic default imports from named only commonjs exports

> `compilerOptions.allowSyntheticDefaultImports` allows default imports to bind to the CommonJS namespace shape.

```json:destack.json
{ "compilerOptions": { "allowSyntheticDefaultImports": true } }
```

```js:cjs.js
function buildValue() {
    return 1;
}

exports.buildValue = buildValue;
```

```ts:main.ts
import cjs from "./cjs";

cjs.buildValue();
```

## named imports from static property writes

### named imports read exports dot property assignment

> A top level `exports.name = value` assignment defines a named export.

```js:cjs.js
function buildValue() {
    return 1;
}

exports.buildValue = buildValue;
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

### named imports read module exports dot property assignment

> A top level `module.exports.name = value` assignment defines a named export.

```js:cjs.js
function buildValue() {
    return 1;
}

module.exports.buildValue = buildValue;
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

### named imports read static bracket property assignment

> A top level static bracket key on `exports` defines a named export.

```js:cjs.js
function buildValue() {
    return 1;
}

exports["buildValue"] = buildValue;
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

### named imports read static module exports bracket property assignment

> A top level static bracket key on `module.exports` defines a named export.

```js:cjs.js
function buildValue() {
    return 1;
}

module.exports["buildValue"] = buildValue;
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

### named imports read object literal properties from module exports

> A top level `module.exports = { ... }` object literal defines static named exports.

```js:cjs.js
function first() {
    return 1;
}

const second = () => 2;

module.exports = {
    first,
    second,
};
```

```ts:main.ts
import { first, second } from "./cjs";

first();
second();
```

## mixed default and named interop

### mixed default and named imports read object literal exports

> Default and named imports can both read one top level object literal assignment.

```js:cjs.js
function buildValue() {
    return 1;
}

const version = "1.0.0";

module.exports = {
    buildValue,
    version,
};
```

```ts:main.ts
import cjs from "./cjs";
import { buildValue, version } from "./cjs";

cjs;
buildValue();
version;
```

### mixed default and named imports read function default with static helper

> Named imports can read static properties attached to a function assigned to `module.exports`.

```js:cjs.js
function main() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = main;
module.exports.helper = helper;
```

```ts:main.ts
import main from "./cjs";
import { helper } from "./cjs";

main();
helper();
```

### mixed exports and module exports property writes define named imports

> Static writes on both `exports` and `module.exports` participate in named export synthesis.

```js:cjs.js
function first() {
    return 1;
}

function second() {
    return 2;
}

exports.first = first;
module.exports.second = second;
```

```ts:main.ts
import { first, second } from "./cjs";

first();
second();
```

### export assignment suppresses commonjs named export synthesis

> A top level `export = value` assignment disables synthesized CommonJS named exports.

```ts:cjs.ts
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports.helper = helper;
export = selected;
```

```ts:main.ts
import { helper } from "./cjs";

helper();
```

- contains: missing symbol

### module exports replacement drops earlier named writes on the old object

> Named writes before a replacement assignment are not visible once `module.exports` is replaced.

```js:cjs.js
function first() {
    return 1;
}

function second() {
    return 2;
}

exports.first = first;
module.exports = second;
```

```ts:main.ts
import selected from "./cjs";
import { first } from "./cjs";

selected();
first();
```

- contains: missing symbol

### exports alias writes after replacement do not affect module exports

> Writes through `exports` after replacing `module.exports` do not define named exports.

```js:cjs.js
function selected() {
    return 1;
}

function leaked() {
    return 2;
}

module.exports = selected;
exports.leaked = leaked;
```

```ts:main.ts
import selected from "./cjs";
import { leaked } from "./cjs";

selected();
leaked();
```

- contains: missing symbol

## unsupported dynamic forms

### dynamic key assignment does not synthesize named exports

> Dynamic keys are not statically resolvable and do not define named exports.

```js:cjs.js
const key = "buildValue";

exports[key] = function buildValue() {
    return 1;
};
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

- contains: missing symbol

### aliased exports object assignment does not synthesize named exports

> Assignments through aliases of `exports` are not part of the static surface.

```js:cjs.js
const out = exports;

out.buildValue = function buildValue() {
    return 1;
};
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

- contains: missing symbol

### branch dependent writes do not synthesize named exports

> Control flow dependent export writes are not statically synthesized.

```js:cjs.js
if (Math.random() > 0.5) {
    exports.buildValue = function buildValue() {
        return 1;
    };
}
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

- contains: missing symbol

### defineproperty writes do not synthesize named exports

> Reflective property writes are outside the static CommonJS export surface.

```js:cjs.js
Object.defineProperty(exports, "buildValue", {
    value: function buildValue() {
        return 1;
    },
});
```

```ts:main.ts
import { buildValue } from "./cjs";

buildValue();
```

- contains: missing symbol
