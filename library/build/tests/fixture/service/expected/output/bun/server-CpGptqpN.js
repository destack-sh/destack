function getEnumValues(entries) {
	const numericValues = Object.values(entries).filter((v) => typeof v === "number");
	return Object.entries(entries).filter(([k, _]) => numericValues.indexOf(+k) === -1).map(([_, v]) => v);
}
function joinValues(array, separator = "|") {
	return array.map((val) => stringifyPrimitive(val)).join(separator);
}
function jsonStringifyReplacer(_, value) {
	if (typeof value === "bigint") return value.toString();
	return value;
}
var Cached = class {
	constructor(getter) {
		this._getter = getter;
		this._value = void 0;
	}
	get value() {
		const getter = this._getter;
		if (getter !== void 0) {
			this._value = getter();
			this._getter = void 0;
		}
		return this._value;
	}
};
function cached(getter) {
	return new Cached(getter);
}
function nullish(input) {
	return input === null || input === void 0;
}
function cleanRegex(source) {
	const start = source.startsWith("^") ? 1 : 0;
	const end = source.endsWith("$") ? source.length - 1 : source.length;
	return source.slice(start, end);
}
function floatSafeRemainder(val, step) {
	const ratio = val / step;
	const roundedRatio = Math.round(ratio);
	const tolerance = 4 * Number.EPSILON * Math.max(Math.abs(ratio), 1);
	if (Math.abs(ratio - roundedRatio) < tolerance) return 0;
	return ratio - roundedRatio;
}
var EVALUATING = /* @__PURE__*/ Symbol("evaluating");
function defineLazy(object, key, getter) {
	let value = void 0;
	Object.defineProperty(object, key, {
		get() {
			if (value === EVALUATING) return;
			if (value === void 0) {
				value = EVALUATING;
				value = getter();
			}
			return value;
		},
		set(v) {
			Object.defineProperty(object, key, { value: v });
		},
		configurable: true
	});
}
function assignProp(target, prop, value) {
	Object.defineProperty(target, prop, {
		value,
		writable: true,
		enumerable: true,
		configurable: true
	});
}
/**
* Whichever object a def's `shape` currently answers from: the one the caller passed until the first read, the frozen copy after it.
*
* Its keys and descriptors read without invoking anything, which is what lets a discriminated union check its discriminator, and the cycle walk read a shape, without resolving a getter that references the schema being constructed. A def that answers `shape` from an accessor of its own has none.
*/
function rawShape(def) {
	const desc = Object.getOwnPropertyDescriptor(def, "shape");
	return desc?.get ? desc.get.raw : desc?.value;
}
function sourceShape(schema) {
	return rawShape(schema._zod.def) ?? schema._zod.def.shape;
}
function deferProp(target, key, getter) {
	Object.defineProperty(target, key, {
		get() {
			const value = getter();
			assignProp(this, key, value);
			return value;
		},
		enumerable: true,
		configurable: true
	});
}
function putProp(target, key, value) {
	if (key in target) assignProp(target, key, value);
	else target[key] = value;
}
/**
* Copies `keys` of `source`'s shape onto `target`, each value passed through `wrap`.
*
* A key the source has resolved is copied through now, so the derived shape states it outright and nothing has to resolve it to learn what it holds. A key the source still defers stays deferred, and reads back through the source's own `shape`, so it resolves once and both shapes get that one schema.
*/
function mirrorShape(target, source, keys, wrap) {
	const raw = sourceShape(source);
	for (const key of keys) {
		const desc = Object.getOwnPropertyDescriptor(raw, key);
		if (!desc.enumerable) continue;
		if (desc.get) deferProp(target, key, () => {
			const value = source._zod.def.shape[key];
			return wrap ? wrap(value, key) : value;
		});
		else putProp(target, key, wrap ? wrap(desc.value, key) : desc.value);
	}
}
function mirrorProps(target, source) {
	for (const key of Reflect.ownKeys(source)) {
		const desc = Object.getOwnPropertyDescriptor(source, key);
		if (!desc.enumerable) continue;
		if (desc.get) deferProp(target, key, () => source[key]);
		else putProp(target, key, desc.value);
	}
}
function mergeDefs(...defs) {
	const mergedDescriptors = {};
	for (const def of defs) {
		const descriptors = Object.getOwnPropertyDescriptors(def);
		Object.assign(mergedDescriptors, descriptors);
	}
	return Object.defineProperties({}, mergedDescriptors);
}
function esc(str) {
	return JSON.stringify(str);
}
function slugify(input) {
	return input.toLowerCase().trim().replace(/[^\w\s-]/g, "").replace(/[\s_-]+/g, "-").replace(/^-+|-+$/g, "");
}
var captureStackTrace = "captureStackTrace" in Error ? Error.captureStackTrace : (..._args) => {};
function isObject$1(data) {
	return typeof data === "object" && data !== null && !Array.isArray(data);
}
var allowsEval = /* @__PURE__*/ cached(() => {
	if (globalConfig.jitless) return false;
	if (typeof navigator !== "undefined" && navigator?.userAgent?.includes("Cloudflare")) return false;
	try {
		new Function("");
		return true;
	} catch (_) {
		return false;
	}
});
function isPlainObject(o) {
	if (isObject$1(o) === false) return false;
	const ctor = o.constructor;
	if (ctor === void 0) return true;
	if (typeof ctor !== "function") return true;
	const prot = ctor.prototype;
	if (isObject$1(prot) === false) return false;
	if (Object.prototype.hasOwnProperty.call(prot, "isPrototypeOf") === false) return false;
	return true;
}
function shallowClone(o) {
	if (isPlainObject(o)) return { ...o };
	if (Array.isArray(o)) return [...o];
	if (o instanceof Map) return new Map(o);
	if (o instanceof Set) return new Set(o);
	return o;
}
var propertyKeyTypes = /* @__PURE__*/ new Set([
	"string",
	"number",
	"symbol"
]);
function escapeRegex(str) {
	return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
function clone(inst, def, params) {
	const cl = new inst._zod.constr(def ?? inst._zod.def);
	if (!def || params?.parent) cl._zod.parent = inst;
	return cl;
}
function normalizeParams(_params) {
	const params = _params;
	if (!params) return {};
	if (typeof params === "string") return { error: () => params };
	if (params?.message !== void 0) {
		if (params?.error !== void 0) throw new Error("Cannot specify both `message` and `error` params");
		params.error = params.message;
	}
	delete params.message;
	if (typeof params.error === "string") return {
		...params,
		error: () => params.error
	};
	return params;
}
function stringifyPrimitive(value) {
	if (typeof value === "bigint") return value.toString() + "n";
	if (typeof value === "string") return `"${value}"`;
	return `${value}`;
}
function optionalKeys(shape) {
	return Object.keys(shape).filter((k) => {
		return shape[k]._zod.optin !== void 0 && shape[k]._zod.optout === "optional";
	});
}
var NUMBER_FORMAT_RANGES = /*@__PURE__*/ (() => ({
	safeint: [Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER],
	int32: [-2147483648, 2147483647],
	uint32: [0, 4294967295],
	float32: [-34028234663852886e22, 34028234663852886e22],
	float64: [-Number.MAX_VALUE, Number.MAX_VALUE]
}))();
var BIGINT_FORMAT_RANGES = {
	int64: [/* @__PURE__*/ BigInt("-9223372036854775808"), /* @__PURE__*/ BigInt("9223372036854775807")],
	uint64: [/* @__PURE__*/ BigInt(0), /* @__PURE__*/ BigInt("18446744073709551615")]
};
function pick(schema, mask) {
	const currDef = schema._zod.def;
	const checks = currDef.checks;
	if (checks && checks.length > 0) throw new Error(".pick() cannot be used on object schemas containing refinements");
	const newShape = {};
	mirrorShape(newShape, schema, maskedKeys(schema, mask));
	return clone(schema, mergeDefs(currDef, {
		shape: newShape,
		checks: []
	}));
}
function maskedKeys(schema, mask) {
	const raw = sourceShape(schema);
	const keys = [];
	for (const key of Reflect.ownKeys(mask)) {
		if (!Object.getOwnPropertyDescriptor(raw, key)?.enumerable) throw new Error(`Unrecognized key: "${String(key)}"`);
		if (mask[key]) keys.push(key);
	}
	return keys;
}
function omit(schema, mask) {
	const currDef = schema._zod.def;
	const checks = currDef.checks;
	if (checks && checks.length > 0) throw new Error(".omit() cannot be used on object schemas containing refinements");
	const omitted = new Set(maskedKeys(schema, mask));
	const newShape = {};
	mirrorShape(newShape, schema, Reflect.ownKeys(sourceShape(schema)).filter((key) => !omitted.has(key)));
	return clone(schema, mergeDefs(currDef, {
		shape: newShape,
		checks: []
	}));
}
function extend(schema, shape) {
	if (!isPlainObject(shape)) throw new Error("Invalid input to extend: expected a plain object");
	const checks = schema._zod.def.checks;
	if (checks && checks.length > 0) {
		const existingShape = sourceShape(schema);
		for (const key of Reflect.ownKeys(shape)) if (Object.getOwnPropertyDescriptor(existingShape, key) !== void 0) throw new Error("Cannot overwrite keys on object schemas containing refinements. Use `.safeExtend()` instead.");
	}
	return clone(schema, mergeDefs(schema._zod.def, { shape: extended(schema, shape) }));
}
function extended(schema, shape) {
	const newShape = {};
	mirrorShape(newShape, schema, Reflect.ownKeys(sourceShape(schema)));
	mirrorProps(newShape, shape);
	return newShape;
}
function safeExtend(schema, shape) {
	if (!isPlainObject(shape)) throw new Error("Invalid input to safeExtend: expected a plain object");
	return clone(schema, mergeDefs(schema._zod.def, { shape: extended(schema, shape) }));
}
function merge(a, b) {
	if (!b?._zod?.def) throw new Error("Invalid input to merge: expected an object schema. To merge a plain shape, use `.extend()`.");
	if (a._zod.def.checks?.length) throw new Error(".merge() cannot be used on object schemas containing refinements. Use .safeExtend() instead.");
	const newShape = {};
	mirrorShape(newShape, a, Reflect.ownKeys(sourceShape(a)));
	mirrorShape(newShape, b, Reflect.ownKeys(sourceShape(b)));
	return clone(a, mergeDefs(a._zod.def, {
		shape: newShape,
		get catchall() {
			return b._zod.def.catchall;
		},
		checks: b._zod.def.checks ?? []
	}));
}
function partial(Class, schema, mask, name = "partial") {
	const checks = schema._zod.def.checks;
	if (checks && checks.length > 0) throw new Error(`.${name}() cannot be used on object schemas containing refinements`);
	const selected = mask ? new Set(maskedKeys(schema, mask)) : void 0;
	const newShape = {};
	mirrorShape(newShape, schema, Reflect.ownKeys(sourceShape(schema)), Class && ((value, key) => selected && !selected.has(key) ? value : new Class({
		type: "optional",
		innerType: value
	})));
	return clone(schema, mergeDefs(schema._zod.def, {
		shape: newShape,
		checks: []
	}));
}
function required(Class, schema, mask) {
	const selected = mask ? new Set(maskedKeys(schema, mask)) : void 0;
	const newShape = {};
	mirrorShape(newShape, schema, Reflect.ownKeys(sourceShape(schema)), (value, key) => selected && !selected.has(key) ? value : new Class({
		type: "nonoptional",
		innerType: value
	}));
	return clone(schema, mergeDefs(schema._zod.def, { shape: newShape }));
}
function aborted(x, startIndex = 0) {
	if (x.aborted === true) return true;
	for (let i = startIndex; i < x.issues.length; i++) if (x.issues[i]?.continue !== true) return true;
	return false;
}
function explicitlyAborted(x, startIndex = 0) {
	if (x.aborted === true) return true;
	for (let i = startIndex; i < x.issues.length; i++) if (x.issues[i]?.continue === false) return true;
	return false;
}
function prefixIssues(path, issues) {
	return issues.map((iss) => {
		var _a;
		(_a = iss).path ?? (_a.path = []);
		iss.path.unshift(path);
		return iss;
	});
}
function unwrapMessage(message) {
	return typeof message === "string" ? message : message?.message;
}
function attachSchema(issues, start, inst) {
	var _a;
	for (let i = start; i < issues.length; i++) (_a = issues[i]).schema ?? (_a.schema = inst);
}
function finalizeIssue(iss, ctx, config) {
	var _a;
	const traits = iss.inst?._zod?.traits;
	if (traits?.has("$ZodType")) {
		if (traits.has("$ZodCheck")) (_a = iss).schema ?? (_a.schema = iss.inst);
		else iss.schema = iss.inst;
	}
	const schemaError = iss.schema !== iss.inst ? iss.schema?._zod.def?.error : void 0;
	const message = iss.message ? iss.message : unwrapMessage(iss.inst?._zod.def?.error?.(iss)) ?? unwrapMessage(schemaError?.(iss)) ?? unwrapMessage(ctx?.error?.(iss)) ?? unwrapMessage(config.customError?.(iss)) ?? unwrapMessage(config.localeError?.(iss)) ?? "Invalid input";
	const full = {};
	for (const k of Object.keys(iss)) {
		if (k === "inst" || k === "schema" || k === "continue" || k === "input" || k === "__proto__") continue;
		full[k] = iss[k];
	}
	full.path ?? (full.path = []);
	full.message = message;
	if (ctx?.reportInput) full.input = iss.input;
	return full;
}
var highSurrogate = /[\uD800-\uDBFF]/;
function codePointLength(str) {
	const units = str.length;
	if (!highSurrogate.test(str)) return units;
	let count = units;
	for (let i = 0; i < units - 1; i++) if ((str.charCodeAt(i) & 64512) === 55296 && (str.charCodeAt(i + 1) & 64512) === 56320) {
		count--;
		i++;
	}
	return count;
}
function getLengthableOrigin(input) {
	if (Array.isArray(input)) return "array";
	if (typeof input === "string") return "string";
	return "unknown";
}
function parsedType(data) {
	const t = typeof data;
	switch (t) {
		case "number": return Number.isNaN(data) ? "nan" : "number";
		case "object": {
			if (data === null) return "null";
			if (Array.isArray(data)) return "array";
			const obj = data;
			if (obj && Object.getPrototypeOf(obj) !== Object.prototype && "constructor" in obj && obj.constructor) return obj.constructor.name;
		}
	}
	return t;
}
function issue(...args) {
	const [iss, input, inst] = args;
	if (typeof iss === "string") return {
		message: iss,
		code: "custom",
		input,
		inst
	};
	return { ...iss };
}
/**
* Installs a trait's members on its prototype. Each value builds that member for the instance on first read; the built value shadows the accessor as an own property, so a detached `const { parse } = schema` keeps working.
*
* Call this from a `proto` initializer, which runs once per prototype — never per instance.
*/
function members(proto, table) {
	for (const key in table) {
		const desc = Object.getOwnPropertyDescriptor(table, key);
		if (desc.get) Object.defineProperty(proto, key, {
			...desc,
			enumerable: false
		});
		else defineBound(proto, key, desc.value);
	}
}
/** Shadows a prototype member with an own value, so a getter that builds from the instance runs once. */
function own(inst, key, value, enumerable = true) {
	Object.defineProperty(inst, key, {
		configurable: true,
		writable: true,
		enumerable,
		value
	});
	return value;
}
/** Like {@link own}, for a member that was never an own data property and has to stay out of `Object.keys`. */
function hide(inst, key, value) {
	return own(inst, key, value, false);
}
/** Adds members a table derives from the instance: each builds on first read and shadows as own data, and assignment shadows the same way, as when these were own properties. */
function derived(computes, table) {
	for (const key in computes) {
		const compute = computes[key];
		Object.defineProperty(table, key, {
			configurable: true,
			enumerable: true,
			get() {
				return own(this, key, compute(this));
			},
			set(value) {
				own(this, key, value);
			}
		});
	}
	return table;
}
function defineBound(proto, key, fn) {
	Object.defineProperty(proto, key, {
		configurable: true,
		get() {
			return this == null ? fn : own(this, key, fn.bind(this));
		},
		set(value) {
			own(this, key, value);
		}
	});
}
/** Returns the prototype to install on, or `undefined` if this group is already installed on it. */
function claim(inst, sentinel) {
	const proto = Object.getPrototypeOf(inst);
	return sentinel in proto ? void 0 : proto;
}
var installing;
var broke = false;
var breaker = {
	configurable: true,
	get() {
		broke = true;
	}
};
/**
* Installs a lazily-derived internal on the `_zod` prototype of `inst`'s
* constructor, computed from the internals object itself and cached there on
* first read. One accessor per constructor rather than one per instance.
*/
function defineLazyInternal(inst, key, compute) {
	const proto = Object.getPrototypeOf(inst._zod);
	if (key in proto && installing !== inst._zod) {
		installing = void 0;
		return;
	}
	installing = inst._zod;
	Object.defineProperty(proto, key, {
		configurable: true,
		get() {
			Object.defineProperty(this, key, breaker);
			const outer = broke;
			broke = false;
			try {
				const value = compute(this);
				if (broke) delete this[key];
				else Object.defineProperty(this, key, {
					configurable: true,
					writable: true,
					value
				});
				broke = broke || outer;
				return value;
			} catch (err) {
				delete this[key];
				broke = broke || outer;
				throw err;
			}
		},
		set(value) {
			Object.defineProperty(this, key, {
				configurable: true,
				writable: true,
				value
			});
		}
	});
}
/**
* Installs `key` on `inst`'s prototype, computed by `make` on first read and cached there as an own
* data property. One accessor per constructor rather than one per instance, because an own accessor
* puts every instance after the first into v8 dictionary mode. The key doubles as the sentinel.
*/
function installLazyProp(inst, key, make, enumerable) {
	const proto = claim(inst, key);
	if (!proto) return;
	Object.defineProperty(proto, key, {
		configurable: true,
		get() {
			const desc = {
				configurable: true,
				writable: true,
				enumerable,
				value: void 0
			};
			Object.defineProperty(this, key, desc);
			desc.value = make(this);
			Object.defineProperty(this, key, desc);
			return desc.value;
		},
		set(value) {
			Object.defineProperty(this, key, {
				configurable: true,
				writable: true,
				enumerable,
				value
			});
		}
	});
}
/** Marks the thunk `_catch` synthesises for a constant catch value. `Function.length` cannot tell that thunk from a user callback — rest and defaulted parameters both report arity 0 — and a user callback reads `ctx.error`, whose issues only finalize correctly against the caller's per-parse error map. Provenance can say what arity cannot. A plain string key rather than `Symbol.for`, whose call at module scope no bundler can prove pure — the same shape that anchored `urlCanParse` into every build. */
var CONSTANT_CATCH = "~constantCatch";
/** Wraps a constant catch value in a thunk tagged with {@link CONSTANT_CATCH}. */
function constantCatch(value) {
	const fn = () => value;
	fn[CONSTANT_CATCH] = true;
	return fn;
}
var _a$1;
var _zodDesc = {
	value: void 0,
	enumerable: false
};
var _E = "captureStackTrace" in Error ? Error : null;
function newError(Definition) {
	const E = _E;
	if (E) {
		const saved = E.stackTraceLimit;
		if (typeof saved === "number") {
			try {
				E.stackTraceLimit = 0;
			} catch {
				_E = null;
				return new Definition();
			}
			try {
				return new Definition();
			} finally {
				E.stackTraceLimit = saved;
			}
		}
	}
	return new Definition();
}
function $constructor(name, initializer, proto, params) {
	const zodProto = {};
	function Internals(def) {
		this.def = def;
		this.constr = _;
		this.traits = /* @__PURE__ */ new Set();
	}
	Internals.prototype = zodProto;
	const protoMembers = proto;
	const initialized = protoMembers && /* @__PURE__ */ new WeakSet();
	function init(inst, def) {
		if (!inst._zod) {
			_zodDesc.value = new Internals(def);
			try {
				Object.defineProperty(inst, "_zod", _zodDesc);
			} finally {
				_zodDesc.value = void 0;
			}
		} else if (inst._zod.traits.has(name)) return;
		inst._zod.traits.add(name);
		initializer(inst, def);
		if (initialized) {
			const own = Object.getPrototypeOf(inst);
			const ctorProto = inst._zod.constr.prototype;
			let up = own;
			while (up && up !== ctorProto) up = Object.getPrototypeOf(up);
			const target = up ?? own;
			if (!initialized.has(target)) {
				initialized.add(target);
				members(target, protoMembers);
			}
		}
		const proto = _.prototype;
		for (const k in proto) {
			if (!Object.prototype.hasOwnProperty.call(proto, k)) continue;
			if (!(k in inst)) inst[k] = proto[k].bind(inst);
		}
	}
	const Parent = params?.Parent ?? Object;
	class Definition extends Parent {}
	Object.defineProperty(Definition, "name", { value: name });
	function _(def) {
		const inst = params?.Parent ? newError(Definition) : this;
		init(inst, def);
		const deferred = inst._zod.deferred;
		if (deferred) {
			for (const fn of deferred) fn();
			inst._zod.deferred = void 0;
		}
		const pp = globalThis.__zod_globalConfig?.postProcessor;
		if (pp) pp(inst);
		return inst;
	}
	Object.defineProperty(_, "init", { value: init });
	Object.defineProperty(_, Symbol.hasInstance, { value: (inst) => {
		if (params?.Parent && inst instanceof params.Parent) return true;
		return inst?._zod?.traits?.has(name);
	} });
	Object.defineProperty(_, "name", { value: name });
	return _;
}
var $ZodAsyncError = class extends Error {
	constructor() {
		super(`Encountered Promise during synchronous parse. Use .parseAsync() instead.`);
	}
};
var $ZodEncodeError = class extends Error {
	constructor(name) {
		super(`Encountered unidirectional transform during encode: ${name}`);
		this.name = "ZodEncodeError";
	}
};
(_a$1 = globalThis).__zod_globalConfig ?? (_a$1.__zod_globalConfig = {});
var globalConfig = globalThis.__zod_globalConfig;
function config(newConfig) {
	if (newConfig) Object.assign(globalConfig, newConfig);
	return globalConfig;
}
function _getMessage() {
	const internals = this._zod;
	internals.message ?? (internals.message = JSON.stringify(internals.def, jsonStringifyReplacer, 2));
	return internals.message;
}
function _setMessage(value) {
	this._zod.message = value;
}
var _messageDesc = {
	get: _getMessage,
	set: _setMessage,
	enumerable: true,
	configurable: true
};
var _issuesDesc = {
	value: void 0,
	enumerable: false
};
var _installedToString = /* @__PURE__ */ new WeakSet([Object.prototype, Error.prototype]);
var initializer$1 = (inst, def) => {
	inst.name = "$ZodError";
	_issuesDesc.value = def;
	Object.defineProperty(inst, "issues", _issuesDesc);
	_issuesDesc.value = void 0;
	Object.defineProperty(inst, "message", _messageDesc);
	const proto = Object.getPrototypeOf(inst);
	if (!_installedToString.has(proto)) {
		_installedToString.add(proto);
		Object.defineProperty(proto, "toString", {
			configurable: true,
			enumerable: false,
			get() {
				const value = () => this.message;
				Object.defineProperty(this, "toString", {
					value,
					configurable: true,
					writable: true
				});
				return value;
			},
			set(value) {
				Object.defineProperty(this, "toString", {
					value,
					configurable: true,
					writable: true
				});
			}
		});
	}
};
var $ZodError = $constructor("$ZodError", initializer$1);
$constructor("$ZodError", initializer$1, void 0, { Parent: Error });
/** Get-or-create `obj[key]` as an own data property. A path segment naming an inherited member
* ("toString", "constructor") would otherwise read through to the prototype, and assigning
* "__proto__" would hit the setter instead of creating a key. */
function node(obj, key, make) {
	if (!Object.prototype.hasOwnProperty.call(obj, key)) {
		if (key === "__proto__") Object.defineProperty(obj, key, {
			value: make(),
			writable: true,
			enumerable: true,
			configurable: true
		});
		else obj[key] = make();
	}
	return obj[key];
}
function flattenError(error, mapper = (issue) => issue.message) {
	const fieldErrors = {};
	const formErrors = [];
	for (const sub of error.issues) if (sub.path.length > 0) node(fieldErrors, sub.path[0], () => []).push(mapper(sub));
	else formErrors.push(mapper(sub));
	return {
		formErrors,
		fieldErrors
	};
}
function formatError(error, mapper = (issue) => issue.message) {
	const fieldErrors = { _errors: [] };
	const processError = (error, path = []) => {
		for (const issue of error.issues) if (issue.code === "invalid_union" && issue.errors.length) issue.errors.map((issues) => processError({ issues }, [...path, ...issue.path]));
		else if (issue.code === "invalid_key") processError({ issues: issue.issues }, [...path, ...issue.path]);
		else if (issue.code === "invalid_element") processError({ issues: issue.issues }, [...path, ...issue.path]);
		else {
			const fullpath = [...path, ...issue.path];
			if (fullpath.length === 0) fieldErrors._errors.push(mapper(issue));
			else {
				let curr = fieldErrors;
				let i = 0;
				while (i < fullpath.length) {
					const el = fullpath[i];
					const terminal = i === fullpath.length - 1;
					if (el === "_errors") {
						if (terminal) curr._errors.push(mapper(issue));
						i++;
						continue;
					}
					if (!Object.prototype.hasOwnProperty.call(curr, el)) Object.defineProperty(curr, el, {
						value: { _errors: [] },
						enumerable: true,
						writable: true,
						configurable: true
					});
					const node = curr[el];
					if (terminal) node._errors.push(mapper(issue));
					curr = node;
					i++;
				}
			}
		}
	};
	processError(error);
	return fieldErrors;
}
function finalizeParams(callee, params) {
	return {
		callee: params?.callee ?? callee,
		Err: params?.Err
	};
}
var _parse = (_Err) => {
	const fn = (schema, value, _ctx, _params) => {
		const ctx = _ctx ? {
			..._ctx,
			async: false
		} : { async: false };
		const result = schema._zod.run({
			value,
			issues: []
		}, ctx);
		if (result instanceof Promise) throw new $ZodAsyncError();
		if (result.issues.length) {
			const e = new ((_params?.Err) ?? _Err)(result.issues.map((iss) => finalizeIssue(iss, ctx, config())));
			captureStackTrace(e, _params?.callee ?? fn);
			throw e;
		}
		return result.value;
	};
	return fn;
};
var _parseAsync = (_Err) => {
	const fn = async (schema, value, _ctx, params) => {
		const ctx = _ctx ? {
			..._ctx,
			async: true
		} : { async: true };
		let result = schema._zod.run({
			value,
			issues: []
		}, ctx);
		if (result instanceof Promise) result = await result;
		if (result.issues.length) {
			const e = new ((params?.Err) ?? _Err)(result.issues.map((iss) => finalizeIssue(iss, ctx, config())));
			captureStackTrace(e, params?.callee ?? fn);
			throw e;
		}
		return result.value;
	};
	return fn;
};
var _safeParse = (_Err) => (schema, value, _ctx) => {
	const ctx = _ctx ? {
		..._ctx,
		async: false
	} : { async: false };
	const result = schema._zod.run({
		value,
		issues: []
	}, ctx);
	if (result instanceof Promise) throw new $ZodAsyncError();
	return result.issues.length ? failure(_Err, result.issues, ctx) : {
		success: true,
		data: result.value
	};
};
function failure(Err, issues, ctx) {
	let error;
	return {
		success: false,
		get error() {
			if (!error) {
				error = new Err(issues.map((iss) => finalizeIssue(iss, ctx, config())));
				issues = void 0;
				ctx = void 0;
			}
			return error;
		},
		set error(e) {
			error = e;
			issues = void 0;
			ctx = void 0;
		}
	};
}
var _safeParseAsync = (_Err) => async (schema, value, _ctx) => {
	const ctx = _ctx ? {
		..._ctx,
		async: true
	} : { async: true };
	let result = schema._zod.run({
		value,
		issues: []
	}, ctx);
	if (result instanceof Promise) result = await result;
	return result.issues.length ? failure(_Err, result.issues, ctx) : {
		success: true,
		data: result.value
	};
};
var COMPILE_INVALID = /* @__PURE__ */ Symbol.for("zod.compile.invalid");
var COMPILE_FALLBACK = /* @__PURE__ */ Symbol.for("zod.compile.fallback");
var validate$1 = ((schema, value, _ctx) => {
	const validator = schema._zod.bag.validator;
	if (validator !== void 0) {
		if (validator(value) !== COMPILE_INVALID) return true;
		if (validator.definite === true && _ctx === void 0) return false;
	}
	return validateFallback(schema, value, _ctx);
});
function validateFallback(schema, value, _ctx) {
	const ctx = _ctx ? {
		..._ctx,
		async: false,
		abortEarly: true
	} : {
		async: false,
		abortEarly: true
	};
	const fallbackRun = schema._zod.bag.fallbackRun;
	let result;
	if (fallbackRun) {
		ctx[COMPILE_FALLBACK] = true;
		result = fallbackRun({
			value,
			issues: []
		}, ctx);
	} else result = schema._zod.run({
		value,
		issues: []
	}, ctx);
	if (result instanceof Promise) throw new $ZodAsyncError();
	return result.issues.length === 0;
}
var validateAsync$1 = async (schema, value, _ctx) => {
	const ctx = _ctx ? {
		..._ctx,
		async: true,
		abortEarly: true
	} : {
		async: true,
		abortEarly: true
	};
	let result = schema._zod.run({
		value,
		issues: []
	}, ctx);
	if (result instanceof Promise) result = await result;
	return result.issues.length === 0;
};
var _encode = (_Err) => {
	const parse = _parse(_Err);
	const fn = (schema, value, _ctx, _params) => {
		const ctx = _ctx ? {
			..._ctx,
			direction: "backward"
		} : { direction: "backward" };
		return parse(schema, value, ctx, finalizeParams(fn, _params));
	};
	return fn;
};
var _decode = (_Err) => {
	const parse = _parse(_Err);
	const fn = (schema, value, _ctx, _params) => {
		return parse(schema, value, _ctx, finalizeParams(fn, _params));
	};
	return fn;
};
var _encodeAsync = (_Err) => {
	const parseAsync = _parseAsync(_Err);
	const fn = async (schema, value, _ctx, _params) => {
		const ctx = _ctx ? {
			..._ctx,
			direction: "backward"
		} : { direction: "backward" };
		return await parseAsync(schema, value, ctx, finalizeParams(fn, _params));
	};
	return fn;
};
var _decodeAsync = (_Err) => {
	const parseAsync = _parseAsync(_Err);
	const fn = async (schema, value, _ctx, _params) => {
		return await parseAsync(schema, value, _ctx, finalizeParams(fn, _params));
	};
	return fn;
};
var _safeEncode = (_Err) => (schema, value, _ctx) => {
	const ctx = _ctx ? {
		..._ctx,
		direction: "backward"
	} : { direction: "backward" };
	return _safeParse(_Err)(schema, value, ctx);
};
var _safeDecode = (_Err) => (schema, value, _ctx) => {
	return _safeParse(_Err)(schema, value, _ctx);
};
var _safeEncodeAsync = (_Err) => async (schema, value, _ctx) => {
	const ctx = _ctx ? {
		..._ctx,
		direction: "backward"
	} : { direction: "backward" };
	return _safeParseAsync(_Err)(schema, value, ctx);
};
var _safeDecodeAsync = (_Err) => async (schema, value, _ctx) => {
	return _safeParseAsync(_Err)(schema, value, _ctx);
};
/**
* @deprecated CUID v1 is deprecated by its authors due to information leakage
* (timestamps embedded in the id). Use {@link cuid2} instead.
* See https://github.com/paralleldrive/cuid.
*/
var cuid = /^[cC][0-9a-z]{6,}$/;
var cuid2 = /^[0-9a-z]+$/;
var ulid = /^[0-7][0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{25}$/;
var xid = /^[0-9a-vA-V]{20}$/;
var ksuid = /^[A-Za-z0-9]{27}$/;
var nanoid = /^[a-zA-Z0-9_-]{21}$/;
function nanoidOfLength(length) {
	return new RegExp(`^[a-zA-Z0-9_-]{${length}}$`);
}
/** ISO 8601-1 duration regex. Does not support the 8601-2 extensions like negative durations or fractional/negative components. */
var duration = /^P(?:(\d+W)|(?!.*W)(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+([.,]\d+)?S)?)?)$/;
/** A regex for any UUID-like identifier: 8-4-4-4-12 hex pattern */
var guid = /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/;
/** Returns a regex for validating an RFC 9562/4122 UUID.
*
* @param version Optionally specify a version 1-8. If no version is specified, all versions are supported. */
var uuid = (version) => {
	if (!version) return /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$/;
	return new RegExp(`^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-${version}[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$`);
};
/** Practical email validation */
var email = /^(?:[A-Za-z0-9_'+\-]+\.)*[A-Za-z0-9_'+\-]*[A-Za-z0-9_+-]@(?:[A-Za-z0-9][A-Za-z0-9\-]*\.)+[A-Za-z]{2,}$/;
var _emoji$1 = `^(?=[\\s\\S]*[\\p{Extended_Pictographic}\\p{Regional_Indicator}\\u20E3])[\\p{Extended_Pictographic}\\p{Emoji_Component}]+$`;
function emoji() {
	return new RegExp(_emoji$1, "u");
}
var ipv4 = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/;
var ipv6 = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:))$/;
var cidrv4 = /^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/([0-9]|[1-2][0-9]|3[0-2])$/;
var cidrv6 = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:))\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/;
var base64 = /^$|^(?:[0-9a-zA-Z+/]{4})*(?:(?:[0-9a-zA-Z+/]{2}==)|(?:[0-9a-zA-Z+/]{3}=))?$/;
var base64url = /^(?:[A-Za-z0-9_-]{4})*(?:[A-Za-z0-9_-]{2,3})?$/;
var httpProtocol = /^https?$/;
var e164 = /^\+[1-9]\d{6,14}$/;
var dateSource = `(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))`;
/** Anchors a pattern source. The interpolation lives here rather than at the call site because
* esbuild will not drop a `@__PURE__` call whose own argument interpolates a variable, but it
* will drop `anchor(dateSource)`. Keeping it inline pinned `date` into every bundle. */
function anchor(source) {
	return new RegExp(`^${source}$`);
}
var date = /*@__PURE__*/ anchor(dateSource);
function timeSource(args) {
	const hhmm = `(?:[01]\\d|2[0-3]):[0-5]\\d`;
	return typeof args.precision === "number" ? args.precision === -1 ? `${hhmm}` : args.precision === 0 ? `${hhmm}:[0-5]\\d` : `${hhmm}:[0-5]\\d\\.\\d{${args.precision}}` : args.seconds ? `${hhmm}:[0-5]\\d(?:\\.\\d+)?` : `${hhmm}(?::[0-5]\\d(?:\\.\\d+)?)?`;
}
function time(args) {
	return new RegExp(`^${timeSource(args)}$`);
}
function datetime(args) {
	const opts = ["Z"];
	if (args.offset) opts.push(`([+-](?:[01]\\d|2[0-3]):[0-5]\\d)`);
	const qualified = `${timeSource({
		precision: args.precision,
		seconds: true
	})}(?:${opts.join("|")})`;
	const timeRegex = args.local ? `${qualified}|${timeSource({ precision: args.precision })}` : qualified;
	return new RegExp(`^${dateSource}T(?:${timeRegex})$`);
}
var anyString = /^[\s\S]{0,}$/;
var integer = /^-?\d+$/;
var number$1 = /^-?\d+(?:\.\d+)?$/;
var boolean$1 = /^(?:true|false)$/i;
var _null$2 = /^null$/i;
var lowercase = /^[^A-Z]*$/;
var uppercase = /^[^a-z]*$/;
var $ZodCheck = /*@__PURE__*/ $constructor("$ZodCheck", (inst, def) => {
	var _a;
	inst._zod ?? (inst._zod = {});
	inst._zod.def = def;
	(_a = inst._zod).onattach ?? (_a.onattach = []);
});
/** Default `when` for length-based checks: run only on non-nullish values with a `length`. */
var _whenHasLength = (payload) => {
	const val = payload.value;
	return !nullish(val) && val.length !== void 0;
};
var numericOriginMap = {
	number: "number",
	bigint: "bigint",
	object: "date"
};
var $ZodCheckLessThan = /*@__PURE__*/ $constructor("$ZodCheckLessThan", (inst, def) => {
	$ZodCheck.init(inst, def);
	const origin = numericOriginMap[typeof def.value];
	inst._zod.check = (payload) => {
		if (def.inclusive ? payload.value <= def.value : payload.value < def.value) return;
		payload.issues.push({
			origin: numericOriginMap[typeof payload.value] ?? origin,
			code: "too_big",
			maximum: typeof def.value === "object" ? def.value.getTime() : def.value,
			input: payload.value,
			inclusive: def.inclusive,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckGreaterThan = /*@__PURE__*/ $constructor("$ZodCheckGreaterThan", (inst, def) => {
	$ZodCheck.init(inst, def);
	const origin = numericOriginMap[typeof def.value];
	inst._zod.check = (payload) => {
		if (def.inclusive ? payload.value >= def.value : payload.value > def.value) return;
		payload.issues.push({
			origin: numericOriginMap[typeof payload.value] ?? origin,
			code: "too_small",
			minimum: typeof def.value === "object" ? def.value.getTime() : def.value,
			input: payload.value,
			inclusive: def.inclusive,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckMultipleOf = /*@__PURE__*/ $constructor("$ZodCheckMultipleOf", (inst, def) => {
	$ZodCheck.init(inst, def);
	inst._zod.check = (payload) => {
		if (typeof payload.value !== typeof def.value) throw new Error("Cannot mix number and bigint in multiple_of check.");
		if (typeof payload.value === "bigint" ? def.value !== BigInt(0) && payload.value % def.value === BigInt(0) : floatSafeRemainder(payload.value, def.value) === 0) return;
		payload.issues.push({
			origin: typeof payload.value,
			code: "not_multiple_of",
			divisor: def.value,
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckNumberFormat = /*@__PURE__*/ $constructor("$ZodCheckNumberFormat", (inst, def) => {
	$ZodCheck.init(inst, def);
	def.format = def.format || "float64";
	const isInt = def.format?.includes("int");
	const origin = isInt ? "int" : "number";
	const [minimum, maximum] = NUMBER_FORMAT_RANGES[def.format];
	inst._zod.check = (payload) => {
		const input = payload.value;
		if (isInt) {
			if (!Number.isInteger(input)) {
				payload.issues.push({
					expected: origin,
					format: def.format,
					code: "invalid_type",
					continue: false,
					input,
					inst
				});
				return;
			}
			if (!Number.isSafeInteger(input)) {
				if (input > 0) payload.issues.push({
					input,
					code: "too_big",
					maximum: Number.MAX_SAFE_INTEGER,
					note: "Integers must be within the safe integer range.",
					inst,
					origin,
					inclusive: true,
					continue: !def.abort
				});
				else payload.issues.push({
					input,
					code: "too_small",
					minimum: Number.MIN_SAFE_INTEGER,
					note: "Integers must be within the safe integer range.",
					inst,
					origin,
					inclusive: true,
					continue: !def.abort
				});
				return;
			}
		}
		if (input < minimum) payload.issues.push({
			origin: "number",
			input,
			code: "too_small",
			minimum,
			inclusive: true,
			inst,
			continue: !def.abort
		});
		if (input > maximum) payload.issues.push({
			origin: "number",
			input,
			code: "too_big",
			maximum,
			inclusive: true,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckMaxLength = /*@__PURE__*/ $constructor("$ZodCheckMaxLength", (inst, def) => {
	var _a;
	$ZodCheck.init(inst, def);
	(_a = inst._zod.def).when ?? (_a.when = _whenHasLength);
	inst._zod.check = (payload) => {
		const input = payload.value;
		const units = input.length;
		if ((typeof input === "string" && units > def.maximum ? codePointLength(input) : units) <= def.maximum) return;
		const origin = getLengthableOrigin(input);
		payload.issues.push({
			origin,
			code: "too_big",
			maximum: def.maximum,
			inclusive: true,
			input,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckMinLength = /*@__PURE__*/ $constructor("$ZodCheckMinLength", (inst, def) => {
	var _a;
	$ZodCheck.init(inst, def);
	(_a = inst._zod.def).when ?? (_a.when = _whenHasLength);
	inst._zod.check = (payload) => {
		const input = payload.value;
		const units = input.length;
		if ((typeof input === "string" && units >= def.minimum && units < def.minimum * 2 ? codePointLength(input) : units) >= def.minimum) return;
		const origin = getLengthableOrigin(input);
		payload.issues.push({
			origin,
			code: "too_small",
			minimum: def.minimum,
			inclusive: true,
			input,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckLengthEquals = /*@__PURE__*/ $constructor("$ZodCheckLengthEquals", (inst, def) => {
	var _a;
	$ZodCheck.init(inst, def);
	(_a = inst._zod.def).when ?? (_a.when = _whenHasLength);
	inst._zod.check = (payload) => {
		const input = payload.value;
		const units = input.length;
		const length = typeof input === "string" && units >= def.length && units <= def.length * 2 ? codePointLength(input) : units;
		if (length === def.length) return;
		const origin = getLengthableOrigin(input);
		const tooBig = length > def.length;
		payload.issues.push({
			origin,
			...tooBig ? {
				code: "too_big",
				maximum: def.length
			} : {
				code: "too_small",
				minimum: def.length
			},
			inclusive: true,
			exact: true,
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckStringFormat = /*@__PURE__*/ $constructor("$ZodCheckStringFormat", (inst, def) => {
	var _a, _b;
	$ZodCheck.init(inst, def);
	if (def.pattern) (_a = inst._zod).check ?? (_a.check = (payload) => {
		def.pattern.lastIndex = 0;
		if (def.pattern.test(payload.value)) return;
		payload.issues.push({
			origin: "string",
			code: "invalid_format",
			format: def.format,
			input: payload.value,
			...def.pattern ? { pattern: def.pattern.toString() } : {},
			inst,
			continue: !def.abort
		});
	});
	else (_b = inst._zod).check ?? (_b.check = () => {});
});
var $ZodCheckRegex = /*@__PURE__*/ $constructor("$ZodCheckRegex", (inst, def) => {
	$ZodCheckStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		def.pattern.lastIndex = 0;
		if (def.pattern.test(payload.value)) return;
		payload.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "regex",
			input: payload.value,
			pattern: def.pattern.toString(),
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckLowerCase = /*@__PURE__*/ $constructor("$ZodCheckLowerCase", (inst, def) => {
	def.pattern ?? (def.pattern = lowercase);
	$ZodCheckStringFormat.init(inst, def);
});
var $ZodCheckUpperCase = /*@__PURE__*/ $constructor("$ZodCheckUpperCase", (inst, def) => {
	def.pattern ?? (def.pattern = uppercase);
	$ZodCheckStringFormat.init(inst, def);
});
var $ZodCheckIncludes = /*@__PURE__*/ $constructor("$ZodCheckIncludes", (inst, def) => {
	$ZodCheck.init(inst, def);
	const escapedRegex = escapeRegex(def.includes);
	def.pattern = new RegExp(typeof def.position === "number" ? `^.{${def.position},}${escapedRegex}` : escapedRegex);
	inst._zod.check = (payload) => {
		if (payload.value.includes(def.includes, def.position)) return;
		payload.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "includes",
			includes: def.includes,
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckStartsWith = /*@__PURE__*/ $constructor("$ZodCheckStartsWith", (inst, def) => {
	$ZodCheck.init(inst, def);
	const pattern = new RegExp(`^${escapeRegex(def.prefix)}.*`);
	def.pattern ?? (def.pattern = pattern);
	inst._zod.check = (payload) => {
		if (payload.value.startsWith(def.prefix)) return;
		payload.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "starts_with",
			prefix: def.prefix,
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckEndsWith = /*@__PURE__*/ $constructor("$ZodCheckEndsWith", (inst, def) => {
	$ZodCheck.init(inst, def);
	const pattern = new RegExp(`.*${escapeRegex(def.suffix)}$`);
	def.pattern ?? (def.pattern = pattern);
	inst._zod.check = (payload) => {
		if (payload.value.endsWith(def.suffix)) return;
		payload.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "ends_with",
			suffix: def.suffix,
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCheckOverwrite = /*@__PURE__*/ $constructor("$ZodCheckOverwrite", (inst, def) => {
	$ZodCheck.init(inst, def);
	inst._zod.check = (payload) => {
		payload.value = def.tx(payload.value);
	};
});
var Doc = class {
	constructor(args = [], closed = {}) {
		this.content = [];
		this.indent = 0;
		this.args = args;
		this.closed = closed;
	}
	indented(fn) {
		this.indent += 1;
		try {
			fn(this);
		} finally {
			this.indent -= 1;
		}
	}
	write(arg) {
		if (typeof arg === "function") {
			arg(this, { execution: "sync" });
			arg(this, { execution: "async" });
			return;
		}
		const lines = arg.split("\n").filter((x) => x);
		const minIndent = Math.min(...lines.map((x) => x.length - x.trimStart().length));
		const dedented = lines.map((x) => x.slice(minIndent)).map((x) => " ".repeat(this.indent * 2) + x);
		for (const line of dedented) this.content.push(line);
	}
	compile() {
		const F = Function;
		const content = this?.content ?? [``];
		return new F(...Object.keys(this.closed), `return function (${this.args.join(", ")}) {\n${content.join("\n")}\n};`)(...Object.values(this.closed));
	}
};
var version$1 = {
	major: 4,
	minor: 6,
	patch: 5
};
var $ZodType = /*@__PURE__*/ $constructor("$ZodType", (inst, def) => {
	var _a;
	inst ?? (inst = {});
	inst._zod.def = def;
	inst._zod.bag = inst._zod.bag || {};
	inst._zod.version = version$1;
	const defChecks = inst._zod.def.checks;
	const checks = inst._zod.traits.has("$ZodCheck") ? [inst, ...defChecks ?? []] : defChecks?.length ? [...defChecks] : [];
	for (const ch of checks) for (const fn of ch._zod.onattach) fn(inst);
	if (checks.length === 0) {
		(_a = inst._zod).deferred ?? (_a.deferred = []);
		inst._zod.deferred?.push(() => {
			inst._zod.run = inst._zod.parse;
		});
	} else {
		const runChecks = (payload, checks, ctx) => {
			if (payload.memo) return payload;
			let isAborted = aborted(payload);
			let asyncResult;
			for (const ch of checks) {
				if (ch._zod.def.when) {
					if (explicitlyAborted(payload)) continue;
					if (!ch._zod.def.when(payload)) continue;
				} else if (isAborted) continue;
				const currLen = payload.issues.length;
				const _ = ch._zod.check(payload);
				if (_ instanceof Promise && ctx?.async === false) throw new $ZodAsyncError();
				if (asyncResult || _ instanceof Promise) asyncResult = (asyncResult ?? Promise.resolve()).then(async () => {
					await _;
					if (payload.issues.length === currLen) return;
					attachSchema(payload.issues, currLen, inst);
					if (!isAborted) isAborted = aborted(payload, currLen);
				});
				else {
					if (payload.issues.length === currLen) continue;
					attachSchema(payload.issues, currLen, inst);
					if (!isAborted) isAborted = aborted(payload, currLen);
				}
			}
			if (asyncResult) return asyncResult.then(() => {
				return payload;
			});
			return payload;
		};
		const handleCanaryResult = (canary, payload, ctx) => {
			if (aborted(canary)) {
				canary.aborted = true;
				return canary;
			}
			const checkResult = runChecks(payload, checks, ctx);
			if (checkResult instanceof Promise) {
				if (ctx.async === false) throw new $ZodAsyncError();
				return checkResult.then((checkResult) => inst._zod.parse(checkResult, ctx));
			}
			return inst._zod.parse(checkResult, ctx);
		};
		inst._zod.run = (payload, ctx) => {
			if (ctx.skipChecks) return inst._zod.parse(payload, ctx);
			if (ctx.direction === "backward") {
				const canary = inst._zod.parse({
					value: payload.value,
					issues: []
				}, {
					...ctx,
					skipChecks: true
				});
				if (canary instanceof Promise) return canary.then((canary) => {
					return handleCanaryResult(canary, payload, ctx);
				});
				return handleCanaryResult(canary, payload, ctx);
			}
			const result = inst._zod.parse(payload, ctx);
			if (result instanceof Promise) {
				if (ctx.async === false) throw new $ZodAsyncError();
				return result.then((result) => runChecks(result, checks, ctx));
			}
			return runChecks(result, checks, ctx);
		};
	}
}, {
	get "~standard"() {
		return hide(this, "~standard", standardProps(this));
	},
	set "~standard"(value) {
		own(this, "~standard", value);
	}
});
/** The Standard Schema surface for `inst`. Shared so wrappers can extend it without forcing it. */
var toStandardResult = (r, ctx) => r.issues.length ? { issues: r.issues.map((iss) => finalizeIssue(iss, ctx, config())) } : { value: r.value };
async function validateAsync(inst, value) {
	const ctx = { async: true };
	return toStandardResult(await inst._zod.run({
		value,
		issues: []
	}, ctx), ctx);
}
function standardProps(inst) {
	return {
		validate: (value) => {
			const ctx = { async: false };
			try {
				const r = inst._zod.run({
					value,
					issues: []
				}, ctx);
				if (!(r instanceof Promise)) return toStandardResult(r, ctx);
			} catch (_) {}
			return validateAsync(inst, value);
		},
		vendor: "zod",
		version: 1
	};
}
var $ZodString = /*@__PURE__*/ $constructor("$ZodString", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.pattern = def.pattern ?? anyString;
	inst._zod.parse = (payload, _) => {
		if (def.coerce) try {
			payload.value = String(payload.value);
		} catch (_) {}
		if (typeof payload.value === "string") return payload;
		payload.issues.push({
			expected: "string",
			code: "invalid_type",
			input: payload.value,
			inst
		});
		return payload;
	};
});
var $ZodStringFormat = /*@__PURE__*/ $constructor("$ZodStringFormat", (inst, def) => {
	$ZodCheckStringFormat.init(inst, def);
	$ZodString.init(inst, def);
});
var $ZodGUID = /*@__PURE__*/ $constructor("$ZodGUID", (inst, def) => {
	def.pattern ?? (def.pattern = guid);
	$ZodStringFormat.init(inst, def);
});
var $ZodUUID = /*@__PURE__*/ $constructor("$ZodUUID", (inst, def) => {
	if (def.version) {
		const v = {
			v1: 1,
			v2: 2,
			v3: 3,
			v4: 4,
			v5: 5,
			v6: 6,
			v7: 7,
			v8: 8
		}[def.version];
		if (v === void 0) throw new Error(`Invalid UUID version: "${def.version}"`);
		def.pattern ?? (def.pattern = uuid(v));
	} else def.pattern ?? (def.pattern = uuid());
	$ZodStringFormat.init(inst, def);
});
var $ZodEmail = /*@__PURE__*/ $constructor("$ZodEmail", (inst, def) => {
	def.pattern ?? (def.pattern = email);
	$ZodStringFormat.init(inst, def);
});
function canParseURL(input) {
	try {
		if (typeof URL !== "undefined" && typeof URL.canParse === "function") return URL.canParse(input);
		new URL(input);
		return true;
	} catch {
		return false;
	}
}
function validateURL(trimmed, def) {
	if (!("normalize" in def) && !("hostname" in def) && !("protocol" in def)) return canParseURL(trimmed) || 2;
	return parseURLObject(trimmed, def);
}
/** Parses a URL while preserving the non-normalizing HTTP guard. */
function parseURLObject(trimmed, def) {
	if (!def.normalize && def.protocol?.source === httpProtocol.source && !/^https?:\/\//i.test(trimmed)) return 1;
	try {
		if (typeof URL !== "undefined") {
			const URLStatic = URL;
			if (typeof URLStatic.parse === "function") return URLStatic.parse(trimmed) ?? 2;
		}
		return new URL(trimmed);
	} catch {
		return 2;
	}
}
var asciiTabOrNewline = /[\t\n\r]/g;
/** The URL parser deletes every ASCII tab, LF and CR from its input before it parses, so `new URL("https://exa\nmple.com")` reports on `example.com`. Applying the same deletion to the returned value closes the half of that divergence which can move the host; the parser's other rewrite, stripping C0 controls at the edges, cannot. */
function stripTabAndNewline(value) {
	return value.replace(asciiTabOrNewline, "");
}
function urlHostnameOk(url, hostname) {
	hostname.lastIndex = 0;
	return hostname.test(url.hostname);
}
function urlProtocolOk(url, protocol) {
	protocol.lastIndex = 0;
	return protocol.test(url.protocol.endsWith(":") ? url.protocol.slice(0, -1) : url.protocol);
}
var $ZodURL = /*@__PURE__*/ $constructor("$ZodURL", (inst, def) => {
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		try {
			const trimmed = payload.value.trim();
			const url = validateURL(trimmed, def);
			if (url === 1) {
				payload.issues.push({
					code: "invalid_format",
					format: "url",
					note: "Invalid URL format",
					input: payload.value,
					inst,
					continue: !def.abort
				});
				return;
			}
			if (url === 2) {
				payload.issues.push({
					code: "invalid_format",
					format: "url",
					input: payload.value,
					inst,
					continue: !def.abort
				});
				return;
			}
			if (url === true) {
				payload.value = stripTabAndNewline(trimmed);
				return;
			}
			if (def.hostname && !urlHostnameOk(url, def.hostname)) payload.issues.push({
				code: "invalid_format",
				format: "url",
				note: "Invalid hostname",
				pattern: def.hostname.source,
				input: payload.value,
				inst,
				continue: !def.abort
			});
			if (def.protocol && !urlProtocolOk(url, def.protocol)) payload.issues.push({
				code: "invalid_format",
				format: "url",
				note: "Invalid protocol",
				pattern: def.protocol.source,
				input: payload.value,
				inst,
				continue: !def.abort
			});
			payload.value = def.normalize ? url.href : stripTabAndNewline(trimmed);
			return;
		} catch (_) {
			payload.issues.push({
				code: "invalid_format",
				format: "url",
				input: payload.value,
				inst,
				continue: !def.abort
			});
		}
	};
});
var $ZodEmoji = /*@__PURE__*/ $constructor("$ZodEmoji", (inst, def) => {
	def.pattern ?? (def.pattern = emoji());
	$ZodStringFormat.init(inst, def);
});
var $ZodNanoID = /*@__PURE__*/ $constructor("$ZodNanoID", (inst, def) => {
	if (def.length !== void 0 && (!Number.isInteger(def.length) || def.length < 1)) throw new Error(`Invalid nanoid length: ${def.length}`);
	def.pattern ?? (def.pattern = def.length === void 0 ? nanoid : nanoidOfLength(def.length));
	$ZodStringFormat.init(inst, def);
});
/**
* @deprecated CUID v1 is deprecated by its authors due to information leakage
* (timestamps embedded in the id). Use {@link $ZodCUID2} instead.
* See https://github.com/paralleldrive/cuid.
*/
var $ZodCUID = /*@__PURE__*/ $constructor("$ZodCUID", (inst, def) => {
	def.pattern ?? (def.pattern = cuid);
	$ZodStringFormat.init(inst, def);
});
var $ZodCUID2 = /*@__PURE__*/ $constructor("$ZodCUID2", (inst, def) => {
	def.pattern ?? (def.pattern = cuid2);
	$ZodStringFormat.init(inst, def);
});
var $ZodULID = /*@__PURE__*/ $constructor("$ZodULID", (inst, def) => {
	def.pattern ?? (def.pattern = ulid);
	$ZodStringFormat.init(inst, def);
});
var $ZodXID = /*@__PURE__*/ $constructor("$ZodXID", (inst, def) => {
	def.pattern ?? (def.pattern = xid);
	$ZodStringFormat.init(inst, def);
});
var $ZodKSUID = /*@__PURE__*/ $constructor("$ZodKSUID", (inst, def) => {
	def.pattern ?? (def.pattern = ksuid);
	$ZodStringFormat.init(inst, def);
});
var $ZodISODateTime = /*@__PURE__*/ $constructor("$ZodISODateTime", (inst, def) => {
	def.pattern ?? (def.pattern = datetime(def));
	$ZodStringFormat.init(inst, def);
});
var $ZodISODate = /*@__PURE__*/ $constructor("$ZodISODate", (inst, def) => {
	def.pattern ?? (def.pattern = date);
	$ZodStringFormat.init(inst, def);
});
var $ZodISOTime = /*@__PURE__*/ $constructor("$ZodISOTime", (inst, def) => {
	def.pattern ?? (def.pattern = time(def));
	$ZodStringFormat.init(inst, def);
});
var $ZodISODuration = /*@__PURE__*/ $constructor("$ZodISODuration", (inst, def) => {
	def.pattern ?? (def.pattern = duration);
	$ZodStringFormat.init(inst, def);
});
var $ZodIPv4 = /*@__PURE__*/ $constructor("$ZodIPv4", (inst, def) => {
	def.pattern ?? (def.pattern = ipv4);
	$ZodStringFormat.init(inst, def);
});
/** An IPv6 address is written with hex digits, colons and dots, and nothing else. The guard is what makes the check below an IPv6 check: `new URL("http://[...]")` parses an authority, not an address, so `@` and `\` re-delimit it and `"::@1\\"` validates against the host `0.0.0.1`. The URL parser also deletes ASCII tab, LF and CR rather than failing, which is how `"::1\n"` validated as `::1`. */
var ipv6Alphabet = /^[0-9a-fA-F:.]+$/;
function isValidIPv6(value) {
	if (!ipv6Alphabet.test(value)) return false;
	return canParseURL(`http://[${value}]`);
}
var $ZodIPv6 = /*@__PURE__*/ $constructor("$ZodIPv6", (inst, def) => {
	def.pattern ?? (def.pattern = ipv6);
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		if (!isValidIPv6(payload.value)) payload.issues.push({
			code: "invalid_format",
			format: "ipv6",
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodCIDRv4 = /*@__PURE__*/ $constructor("$ZodCIDRv4", (inst, def) => {
	def.pattern ?? (def.pattern = cidrv4);
	$ZodStringFormat.init(inst, def);
});
function isValidCIDRv6(value) {
	const parts = value.split("/");
	if (parts.length !== 2) return false;
	const [address, prefix] = parts;
	if (!prefix) return false;
	const prefixNum = Number(prefix);
	if (`${prefixNum}` !== prefix) return false;
	if (prefixNum < 0 || prefixNum > 128) return false;
	return isValidIPv6(address);
}
var $ZodCIDRv6 = /*@__PURE__*/ $constructor("$ZodCIDRv6", (inst, def) => {
	def.pattern ?? (def.pattern = cidrv6);
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		if (!isValidCIDRv6(payload.value)) payload.issues.push({
			code: "invalid_format",
			format: "cidrv6",
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
function isValidBase64(data) {
	if (data === "") return true;
	if (/\s/.test(data)) return false;
	if (data.length % 4 !== 0) return false;
	try {
		atob(data);
		return true;
	} catch {
		return false;
	}
}
var base64Charset = /^[0-9a-zA-Z+/]*={0,2}$/;
var $ZodBase64 = /*@__PURE__*/ $constructor("$ZodBase64", (inst, def) => {
	def.pattern ?? (def.pattern = base64Charset);
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		if (isValidBase64(payload.value)) return;
		payload.issues.push({
			code: "invalid_format",
			format: "base64",
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var base64urlCharset = /^[A-Za-z0-9_-]*$/;
function isValidBase64URL(data) {
	if (!base64urlCharset.test(data)) return false;
	const base64 = data.replace(/[-_]/g, (c) => c === "-" ? "+" : "/");
	return isValidBase64(base64.padEnd(Math.ceil(base64.length / 4) * 4, "="));
}
var $ZodBase64URL = /*@__PURE__*/ $constructor("$ZodBase64URL", (inst, def) => {
	def.pattern ?? (def.pattern = base64urlCharset);
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		if (isValidBase64URL(payload.value)) return;
		payload.issues.push({
			code: "invalid_format",
			format: "base64url",
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodE164 = /*@__PURE__*/ $constructor("$ZodE164", (inst, def) => {
	def.pattern ?? (def.pattern = e164);
	$ZodStringFormat.init(inst, def);
});
function isValidJWT(token, algorithm = null) {
	try {
		const tokensParts = token.split(".");
		if (tokensParts.length !== 3) return false;
		const [header] = tokensParts;
		if (!header) return false;
		const parsedHeader = JSON.parse(atob(header));
		if ("typ" in parsedHeader && parsedHeader?.typ !== "JWT") return false;
		if (!parsedHeader.alg) return false;
		if (algorithm && (!("alg" in parsedHeader) || parsedHeader.alg !== algorithm)) return false;
		return true;
	} catch {
		return false;
	}
}
var $ZodJWT = /*@__PURE__*/ $constructor("$ZodJWT", (inst, def) => {
	$ZodStringFormat.init(inst, def);
	inst._zod.check = (payload) => {
		if (isValidJWT(payload.value, def.alg)) return;
		payload.issues.push({
			code: "invalid_format",
			format: "jwt",
			input: payload.value,
			inst,
			continue: !def.abort
		});
	};
});
var $ZodNumber = /*@__PURE__*/ $constructor("$ZodNumber", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.pattern = number$1;
	inst._zod.parse = (payload, _ctx) => {
		if (def.coerce) try {
			payload.value = Number(payload.value);
		} catch (_) {}
		const input = payload.value;
		if (typeof input === "number" && !Number.isNaN(input) && Number.isFinite(input)) return payload;
		const received = typeof input === "number" ? Number.isNaN(input) ? "NaN" : !Number.isFinite(input) ? String(input) : void 0 : void 0;
		payload.issues.push({
			expected: "number",
			code: "invalid_type",
			input,
			inst,
			...received ? { received } : {}
		});
		return payload;
	};
});
var $ZodNumberFormat = /*@__PURE__*/ $constructor("$ZodNumberFormat", (inst, def) => {
	$ZodCheckNumberFormat.init(inst, def);
	$ZodNumber.init(inst, def);
});
var $ZodBoolean = /*@__PURE__*/ $constructor("$ZodBoolean", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.pattern = boolean$1;
	inst._zod.parse = (payload, _ctx) => {
		if (def.coerce) try {
			payload.value = Boolean(payload.value);
		} catch (_) {}
		const input = payload.value;
		if (typeof input === "boolean") return payload;
		payload.issues.push({
			expected: "boolean",
			code: "invalid_type",
			input,
			inst
		});
		return payload;
	};
});
var $ZodNull = /*@__PURE__*/ $constructor("$ZodNull", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.pattern = _null$2;
	inst._zod.values = /* @__PURE__ */ new Set([null]);
	inst._zod.parse = (payload, _ctx) => {
		const input = payload.value;
		if (input === null) return payload;
		payload.issues.push({
			expected: "null",
			code: "invalid_type",
			input,
			inst
		});
		return payload;
	};
});
var $ZodUnknown = /*@__PURE__*/ $constructor("$ZodUnknown", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.parse = (payload) => payload;
});
var $ZodNever = /*@__PURE__*/ $constructor("$ZodNever", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.parse = (payload, _ctx) => {
		payload.issues.push({
			expected: "never",
			code: "invalid_type",
			input: payload.value,
			inst
		});
		return payload;
	};
});
function handleArrayResult(result, final, index) {
	if (result.issues.length) final.issues.push(...prefixIssues(index, result.issues));
	final.value[index] = result.value;
}
var $ZodArray = /*@__PURE__*/ $constructor("$ZodArray", (inst, def) => {
	$ZodType.init(inst, def);
	const memo = globalConfig.memoizer;
	memo?.attach(inst);
	inst._zod.parse = (payload, ctx) => {
		const input = payload.value;
		if (!Array.isArray(input)) {
			payload.issues.push({
				expected: "array",
				code: "invalid_type",
				input,
				inst
			});
			return payload;
		}
		payload.value = memo ? memo.alloc(inst, payload, Array(input.length), ctx) : Array(input.length);
		const proms = [];
		const abortEarly = ctx?.abortEarly;
		for (let i = 0; i < input.length; i++) {
			const item = input[i];
			const result = def.element._zod.run({
				value: item,
				issues: []
			}, ctx);
			if (result instanceof Promise) proms.push(result.then((result) => handleArrayResult(result, payload, i)));
			else {
				handleArrayResult(result, payload, i);
				if (abortEarly && result.issues.length !== 0 && aborted(result)) break;
			}
		}
		if (proms.length) return Promise.all(proms).then(() => payload);
		return payload;
	};
});
function handlePropertyResult(result, final, key, input, optin, optout) {
	const isPresent = key in input;
	const isOptionalOut = optout === "optional";
	if (!isPresent && isOptionalOut && optin === "optional") return;
	if (result.issues.length) {
		if (optin !== void 0 && isOptionalOut && !isPresent) return;
		final.issues.push(...prefixIssues(key, result.issues));
	}
	if (!isPresent && optin === void 0) {
		if (!result.issues.length) final.issues.push({
			code: "invalid_type",
			expected: "nonoptional",
			input: void 0,
			path: [key]
		});
		return;
	}
	if (result.value === void 0) {
		if (isPresent || optin === "defaulted" && !isOptionalOut) final.value[key] = void 0;
	} else final.value[key] = result.value;
}
var NO_SYMBOL_KEYS = [];
function normalizeDef(def) {
	const keys = Object.keys(def.shape);
	const ownSymbols = Object.getOwnPropertySymbols(def.shape);
	const symbolKeys = ownSymbols.length ? ownSymbols : NO_SYMBOL_KEYS;
	const allKeys = symbolKeys.length ? [...keys, ...symbolKeys] : keys;
	for (const k of allKeys) if (!def.shape?.[k]?._zod?.traits?.has("$ZodType")) throw new Error(`Invalid element at key "${String(k)}": expected a Zod schema`);
	const okeys = optionalKeys(def.shape);
	return {
		...def,
		allKeys,
		symbolKeys,
		keySet: new Set(keys),
		numKeys: keys.length,
		optionalKeys: new Set(okeys)
	};
}
function handleCatchall(proms, input, payload, ctx, def, inst, abortEarly) {
	const unrecognized = [];
	const keySet = def.keySet;
	const _catchall = def.catchall._zod;
	const t = _catchall.def.type;
	const optin = _catchall.optin;
	const optout = _catchall.optout;
	let seen = 0;
	for (const key in input) {
		if (abortEarly && payload.issues.length !== seen) {
			if (aborted(payload, seen)) break;
			seen = payload.issues.length;
		}
		if (keySet.has(key)) continue;
		if (key === "__proto__") {
			if (t === "never") unrecognized.push(key);
			continue;
		}
		if (t === "never") {
			unrecognized.push(key);
			continue;
		}
		const r = _catchall.run({
			value: input[key],
			issues: []
		}, ctx);
		if (r instanceof Promise) proms.push(r.then((r) => handlePropertyResult(r, payload, key, input, optin, optout)));
		else handlePropertyResult(r, payload, key, input, optin, optout);
	}
	if (unrecognized.length) payload.issues.push({
		code: "unrecognized_keys",
		keys: unrecognized,
		input,
		inst,
		continue: true
	});
	if (!proms.length) return payload;
	return Promise.all(proms).then(() => {
		return payload;
	});
}
var $ZodObject = /*@__PURE__*/ $constructor("$ZodObject", (inst, def) => {
	$ZodType.init(inst, def);
	const desc = Object.getOwnPropertyDescriptor(def, "shape");
	const sh = desc?.get ? desc.get.raw : def.shape ?? {};
	if (sh) {
		const get = () => {
			const newSh = { ...sh };
			Object.defineProperty(def, "shape", { value: newSh });
			get.raw = newSh;
			return newSh;
		};
		get.raw = sh;
		Object.defineProperty(def, "shape", { get });
	}
	const _normalized = cached(() => normalizeDef(def));
	defineLazyInternal(inst, "propValues", (zod) => {
		const shape = zod.def.shape;
		const propValues = {};
		for (const key in shape) {
			const field = shape[key]._zod;
			if (field.values) {
				if (!Object.prototype.hasOwnProperty.call(propValues, key)) assignProp(propValues, key, /* @__PURE__ */ new Set());
				for (const v of field.values) propValues[key].add(v);
				if (field.optin !== void 0) propValues[key].add(void 0);
			}
		}
		return propValues;
	});
	const isObject = isObject$1;
	const catchall = def.catchall;
	let value;
	const memo = globalConfig.memoizer;
	memo?.attach(inst);
	inst._zod.parse = (payload, ctx) => {
		value ?? (value = _normalized.value);
		const input = payload.value;
		if (!isObject(input)) {
			payload.issues.push({
				expected: "object",
				code: "invalid_type",
				input,
				inst
			});
			return payload;
		}
		payload.value = memo ? memo.alloc(inst, payload, {}, ctx) : {};
		const proms = [];
		const shape = value.shape;
		const abortEarly = ctx?.abortEarly;
		let seen = payload.issues.length;
		for (const key of value.allKeys) {
			if (abortEarly && payload.issues.length !== seen) {
				if (aborted(payload, seen)) break;
				seen = payload.issues.length;
			}
			if (key === "__proto__") continue;
			const el = shape[key];
			const optin = el._zod.optin;
			const optout = el._zod.optout;
			const r = el._zod.run({
				value: input[key],
				issues: []
			}, ctx);
			if (r instanceof Promise) proms.push(r.then((r) => handlePropertyResult(r, payload, key, input, optin, optout)));
			else handlePropertyResult(r, payload, key, input, optin, optout);
		}
		if (!catchall) return proms.length ? Promise.all(proms).then(() => payload) : payload;
		return handleCatchall(proms, input, payload, ctx, _normalized.value, inst, abortEarly === true);
	};
});
var $ZodObjectJIT = /*@__PURE__*/ $constructor("$ZodObjectJIT", (inst, def) => {
	$ZodObject.init(inst, def);
	const superParse = inst._zod.parse;
	const _normalized = cached(() => normalizeDef(def));
	const memo = globalConfig.memoizer;
	const generateFastpass = (shape) => {
		const normalized = _normalized.value;
		const syms = normalized.symbolKeys;
		const doc = new Doc(["payload", "ctx"], {
			shape,
			inst,
			memo,
			syms
		});
		const parseStr = (k) => `shape[${k}]._zod.run({ value: input[${k}], issues: [] }, ctx)`;
		const prefixStr = (id, k) => `
          let ${id}_ab = false;
          for (let i = 0; i < ${id}.issues.length; i++) {
            const iss = ${id}.issues[i];
            iss.path = iss.path ? [${k}, ...iss.path] : [${k}];
            payload.issues.push(iss);
            if (iss.continue !== true) ${id}_ab = true;
          }
          if (${id}_ab && ctx && ctx.abortEarly) {
            payload.value = newResult;
            return payload;
          }`;
		doc.write(`const input = payload.value;`);
		const ids = Object.create(null);
		let counter = 0;
		for (const key of normalized.allKeys) ids[key] = `key_${counter++}`;
		doc.write(memo ? `const newResult = memo.alloc(inst, payload, {}, ctx);` : `const newResult = {};`);
		for (const key of normalized.allKeys) {
			if (key === "__proto__") continue;
			const id = ids[key];
			const k = typeof key === "symbol" ? `syms[${syms.indexOf(key)}]` : esc(key);
			const isPresent = `${k} in input`;
			const schema = shape[key];
			const optin = schema?._zod?.optin;
			const isOptionalIn = optin !== void 0;
			const isOptionalOut = schema?._zod?.optout === "optional";
			doc.write(`const ${id} = ${parseStr(k)};`);
			if (isOptionalIn && isOptionalOut) {
				const assign = optin === "optional" ? `${id}_present` : `${id}.value !== undefined || ${id}_present`;
				doc.write(`
        const ${id}_present = ${isPresent};
        if (!${id}.issues.length || ${id}_present) {
          if (${id}.issues.length) {${prefixStr(id, k)}
          }

          if (${assign}) {
            newResult[${k}] = ${id}.value;
          }
        }

      `);
			} else if (!isOptionalIn) doc.write(`
        const ${id}_present = ${isPresent};
        if (${id}.issues.length) {${prefixStr(id, k)}
        }
        if (!${id}_present && !${id}.issues.length) {
          payload.issues.push({
            code: "invalid_type",
            expected: "nonoptional",
            input: undefined,
            path: [${k}]
          });
          if (ctx && ctx.abortEarly) {
            payload.value = newResult;
            return payload;
          }
        }

        if (${id}_present) {
          newResult[${k}] = ${id}.value;
        }

      `);
			else {
				doc.write(`
        if (${id}.issues.length) {${prefixStr(id, k)}
        }
      `);
				if (optin === "defaulted") doc.write(`newResult[${k}] = ${id}.value;`);
				else doc.write(`
        if (${id}.value !== undefined || ${isPresent}) {
          newResult[${k}] = ${id}.value;
        }
      `);
			}
		}
		doc.write(`payload.value = newResult;`);
		doc.write(`return payload;`);
		return doc.compile();
	};
	let fastpass;
	const isObject = isObject$1;
	const jit = !globalConfig.jitless;
	const fastEnabled = jit && allowsEval.value;
	const catchall = def.catchall;
	let value;
	inst._zod.parse = (payload, ctx) => {
		value ?? (value = _normalized.value);
		const input = payload.value;
		if (!isObject(input)) {
			payload.issues.push({
				expected: "object",
				code: "invalid_type",
				input,
				inst
			});
			return payload;
		}
		if (jit && fastEnabled && ctx?.async === false && ctx.jitless !== true) {
			if (!fastpass) fastpass = generateFastpass(def.shape);
			payload = fastpass(payload, ctx);
			if (!catchall) return payload;
			return handleCatchall([], input, payload, ctx, value, inst, ctx?.abortEarly === true);
		}
		return superParse(payload, ctx);
	};
});
function handleUnionResults(results, final, inst, ctx) {
	for (const result of results) if (result.issues.length === 0) {
		final.value = result.value;
		return final;
	}
	const nonaborted = results.filter((r) => !aborted(r));
	if (nonaborted.length === 1) {
		final.value = nonaborted[0].value;
		return nonaborted[0];
	}
	final.issues.push({
		code: "invalid_union",
		input: final.value,
		inst,
		errors: results.map((result) => result.issues.map((iss) => finalizeIssue(iss, ctx, config())))
	});
	return final;
}
var $ZodUnion = /*@__PURE__*/ $constructor("$ZodUnion", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "optin", (zod) => zod.def.options.some((o) => o._zod.optin === "defaulted") ? "defaulted" : zod.def.options.some((o) => o._zod.optin !== void 0) ? "optional" : void 0);
	defineLazyInternal(inst, "optout", (zod) => zod.def.options.some((o) => o._zod.optout === "optional") ? "optional" : void 0);
	defineLazyInternal(inst, "values", (zod) => {
		if (zod.def.options.every((o) => o._zod.values)) return new Set(zod.def.options.flatMap((option) => Array.from(option._zod.values)));
	});
	defineLazyInternal(inst, "pattern", (zod) => {
		if (zod.def.options.every((o) => o._zod.pattern)) {
			const patterns = zod.def.options.map((o) => o._zod.pattern);
			return new RegExp(`^(${patterns.map((p) => cleanRegex(p.source)).join("|")})$`);
		}
	});
	const first = def.options.length === 1 ? def.options[0]._zod.run : null;
	inst._zod.parse = (payload, ctx) => {
		if (first) return first(payload, ctx);
		let async = false;
		const results = [];
		for (const option of def.options) {
			const result = option._zod.run({
				value: payload.value,
				issues: []
			}, ctx);
			if (result instanceof Promise) {
				results.push(result);
				async = true;
			} else {
				if (result.issues.length === 0) return result;
				results.push(result);
			}
		}
		if (!async) return handleUnionResults(results, payload, inst, ctx);
		return Promise.all(results).then((results) => {
			return handleUnionResults(results, payload, inst, ctx);
		});
	};
});
var $ZodIntersection = /*@__PURE__*/ $constructor("$ZodIntersection", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.parse = (payload, ctx) => {
		const input = payload.value;
		const left = def.left._zod.run({
			value: input,
			issues: []
		}, ctx);
		const right = def.right._zod.run({
			value: input,
			issues: []
		}, ctx);
		if (left instanceof Promise || right instanceof Promise) return Promise.all([left, right]).then(([left, right]) => {
			return handleIntersectionResults(payload, left, right);
		});
		return handleIntersectionResults(payload, left, right);
	};
});
function mergeValues(a, b) {
	if (a === b) return {
		valid: true,
		data: a
	};
	if (a instanceof Date && b instanceof Date && +a === +b) return {
		valid: true,
		data: a
	};
	if (isPlainObject(a) && isPlainObject(b)) {
		const bKeys = Object.keys(b);
		const sharedKeys = Object.keys(a).filter((key) => bKeys.indexOf(key) !== -1);
		const newObj = {
			...a,
			...b
		};
		if (Object.prototype.hasOwnProperty.call(newObj, "__proto__")) delete newObj.__proto__;
		for (const key of sharedKeys) {
			if (key === "__proto__") continue;
			const sharedValue = mergeValues(a[key], b[key]);
			if (!sharedValue.valid) return {
				valid: false,
				mergeErrorPath: [key, ...sharedValue.mergeErrorPath]
			};
			newObj[key] = sharedValue.data;
		}
		return {
			valid: true,
			data: newObj
		};
	}
	if (Array.isArray(a) && Array.isArray(b)) {
		if (a.length !== b.length) return {
			valid: false,
			mergeErrorPath: []
		};
		const newArray = [];
		for (let index = 0; index < a.length; index++) {
			const itemA = a[index];
			const itemB = b[index];
			const sharedValue = mergeValues(itemA, itemB);
			if (!sharedValue.valid) return {
				valid: false,
				mergeErrorPath: [index, ...sharedValue.mergeErrorPath]
			};
			newArray.push(sharedValue.data);
		}
		return {
			valid: true,
			data: newArray
		};
	}
	return {
		valid: false,
		mergeErrorPath: []
	};
}
function handleIntersectionResults(result, left, right) {
	const unrecKeys = /* @__PURE__ */ new Map();
	let unrecIssue;
	const keyIssues = /* @__PURE__ */ new Map();
	const collect = (iss, side) => {
		let keys;
		if (iss.code === "unrecognized_keys" && !iss.path?.length) {
			unrecIssue ?? (unrecIssue = iss);
			keys = iss.keys;
		} else if (iss.code === "invalid_key" && iss.origin === "record" && iss.path?.length === 1) {
			const k = String(iss.path[0]);
			if (!keyIssues.has(k)) keyIssues.set(k, iss);
			keys = [k];
		} else return false;
		for (const k of keys) {
			if (!unrecKeys.has(k)) unrecKeys.set(k, {});
			unrecKeys.get(k)[side] = true;
		}
		return true;
	};
	for (const iss of left.issues) if (!collect(iss, "l")) result.issues.push(iss);
	for (const iss of right.issues) if (!collect(iss, "r")) result.issues.push(iss);
	const bothKeys = [...unrecKeys].filter(([, f]) => f.l && f.r).map(([k]) => k);
	if (bothKeys.length) {
		const aggregated = unrecIssue ? bothKeys.filter((k) => unrecIssue.keys.includes(k)) : [];
		if (aggregated.length) result.issues.push({
			...unrecIssue,
			keys: aggregated
		});
		for (const k of bothKeys) if (!aggregated.includes(k) && keyIssues.has(k)) result.issues.push(keyIssues.get(k));
	}
	const merged = mergeValues(left.value, right.value);
	if (!merged.valid) {
		if (aborted(result)) return result;
		throw new Error(`Unmergable intersection. Error path: ${JSON.stringify(merged.mergeErrorPath)}`);
	}
	result.value = merged.data;
	return result;
}
var $ZodRecord = /*@__PURE__*/ $constructor("$ZodRecord", (inst, def) => {
	$ZodType.init(inst, def);
	const memo = globalConfig.memoizer;
	memo?.attach(inst);
	inst._zod.parse = (payload, ctx) => {
		const input = payload.value;
		if (!isPlainObject(input)) {
			payload.issues.push({
				expected: "record",
				code: "invalid_type",
				input,
				inst
			});
			return payload;
		}
		const proms = [];
		const values = def.keyType._zod.values;
		if (values && !def.partial) {
			payload.value = memo ? memo.alloc(inst, payload, {}, ctx) : {};
			const recordKeys = /* @__PURE__ */ new Set();
			for (const key of values) if (typeof key === "string" || typeof key === "number" || typeof key === "symbol") {
				recordKeys.add(typeof key === "number" ? key.toString() : key);
				if (key === "__proto__") continue;
				const keyResult = def.keyType._zod.run({
					value: key,
					issues: []
				}, ctx);
				if (keyResult instanceof Promise) throw new Error("Async schemas not supported in object keys currently");
				if (keyResult.issues.length) {
					payload.issues.push({
						code: "invalid_key",
						origin: "record",
						issues: keyResult.issues.map((iss) => finalizeIssue(iss, ctx, config())),
						input: key,
						path: [key],
						inst
					});
					continue;
				}
				const outKey = keyResult.value;
				if (outKey === "__proto__") continue;
				const result = def.valueType._zod.run({
					value: input[key],
					issues: []
				}, ctx);
				if (result instanceof Promise) proms.push(result.then((result) => {
					if (result.issues.length) payload.issues.push(...prefixIssues(key, result.issues));
					payload.value[outKey] = result.value;
				}));
				else {
					if (result.issues.length) payload.issues.push(...prefixIssues(key, result.issues));
					payload.value[outKey] = result.value;
				}
			}
			let unrecognized;
			for (const key in input) if (!recordKeys.has(key)) {
				if (def.mode === "loose") {
					if (key === "__proto__") continue;
					payload.value[key] = input[key];
				} else {
					unrecognized = unrecognized ?? [];
					unrecognized.push(key);
				}
			}
			if (unrecognized && unrecognized.length > 0) payload.issues.push({
				code: "unrecognized_keys",
				input,
				inst,
				keys: unrecognized,
				continue: true
			});
		} else {
			payload.value = memo ? memo.alloc(inst, payload, {}, ctx) : {};
			let unrecognized;
			for (const key of Reflect.ownKeys(input)) {
				if (key === "__proto__") continue;
				if (!Object.prototype.propertyIsEnumerable.call(input, key)) continue;
				let keyResult = def.keyType._zod.run({
					value: key,
					issues: []
				}, ctx);
				if (keyResult instanceof Promise) throw new Error("Async schemas not supported in object keys currently");
				if (typeof key === "string" && number$1.test(key) && keyResult.issues.length) {
					const retryResult = def.keyType._zod.run({
						value: Number(key),
						issues: []
					}, ctx);
					if (retryResult instanceof Promise) throw new Error("Async schemas not supported in object keys currently");
					if (retryResult.issues.length === 0) keyResult = retryResult;
				}
				if (keyResult.issues.length) {
					if (def.mode === "loose") payload.value[key] = input[key];
					else if (values) {
						unrecognized = unrecognized ?? [];
						unrecognized.push(key);
					} else payload.issues.push({
						code: "invalid_key",
						origin: "record",
						issues: keyResult.issues.map((iss) => finalizeIssue(iss, ctx, config())),
						input: key,
						path: [key],
						inst
					});
					continue;
				}
				const outKey = keyResult.value;
				if (outKey === "__proto__") continue;
				const result = def.valueType._zod.run({
					value: input[key],
					issues: []
				}, ctx);
				if (result instanceof Promise) proms.push(result.then((result) => {
					if (result.issues.length) payload.issues.push(...prefixIssues(key, result.issues));
					payload.value[outKey] = result.value;
				}));
				else {
					if (result.issues.length) payload.issues.push(...prefixIssues(key, result.issues));
					payload.value[outKey] = result.value;
				}
			}
			if (unrecognized && unrecognized.length > 0) payload.issues.push({
				code: "unrecognized_keys",
				input,
				inst,
				keys: unrecognized,
				continue: true
			});
		}
		if (proms.length) return Promise.all(proms).then(() => payload);
		return payload;
	};
});
var $ZodEnum = /*@__PURE__*/ $constructor("$ZodEnum", (inst, def) => {
	$ZodType.init(inst, def);
	const values = getEnumValues(def.entries);
	const valuesSet = new Set(values);
	inst._zod.values = valuesSet;
	defineLazyInternal(inst, "pattern", (zod) => {
		const patternValues = getEnumValues(zod.def.entries).filter((k) => propertyKeyTypes.has(typeof k));
		return new RegExp(patternValues.length ? `^(${patternValues.map((o) => escapeRegex(o.toString())).join("|")})$` : "^[^\\s\\S]$");
	});
	inst._zod.parse = (payload, _ctx) => {
		const input = payload.value;
		if (valuesSet.has(input)) return payload;
		payload.issues.push({
			code: "invalid_value",
			values,
			input,
			inst
		});
		return payload;
	};
});
var $ZodLiteral = /*@__PURE__*/ $constructor("$ZodLiteral", (inst, def) => {
	$ZodType.init(inst, def);
	const values = new Set(def.values);
	inst._zod.values = values;
	defineLazyInternal(inst, "pattern", (zod) => {
		const vals = zod.def.values;
		return new RegExp(vals.length ? `^(${vals.map((o) => typeof o === "string" ? escapeRegex(o) : o ? escapeRegex(o.toString()) : String(o)).join("|")})$` : "^[^\\s\\S]$");
	});
	inst._zod.parse = (payload, _ctx) => {
		const input = payload.value;
		if (values.has(input)) return payload;
		payload.issues.push({
			code: "invalid_value",
			values: def.values,
			input,
			inst
		});
		return payload;
	};
});
var $ZodTransform = /*@__PURE__*/ $constructor("$ZodTransform", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.optin = "optional";
	globalConfig.memoizer?.guard(inst);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") throw new $ZodEncodeError(inst.constructor.name);
		const _out = def.transform(payload.value, payload);
		if (ctx.async) return (_out instanceof Promise ? _out : Promise.resolve(_out)).then((output) => {
			payload.value = output;
			return payload;
		});
		if (_out instanceof Promise) throw new $ZodAsyncError();
		payload.value = _out;
		return payload;
	};
});
function handleOptionalResult(payload, result) {
	payload.value = result.issues.length ? void 0 : result.value;
	return payload;
}
var $ZodOptional = /*@__PURE__*/ $constructor("$ZodOptional", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "optin", (zod) => zod.def.innerType._zod.optin === "defaulted" ? "defaulted" : "optional");
	inst._zod.optout = "optional";
	defineLazyInternal(inst, "values", (zod) => {
		const values = zod.def.innerType._zod.values;
		return values ? /* @__PURE__ */ new Set([...values, void 0]) : void 0;
	});
	defineLazyInternal(inst, "pattern", (zod) => {
		const pattern = zod.def.innerType._zod.pattern;
		return pattern ? new RegExp(`^(${cleanRegex(pattern.source)})?$`) : void 0;
	});
	inst._zod.parse = (payload, ctx) => {
		if (payload.value === void 0) {
			if (def.innerType._zod.optin !== "defaulted") return payload;
			const result = def.innerType._zod.run({
				value: payload.value,
				issues: []
			}, ctx);
			if (result instanceof Promise) return result.then((result) => handleOptionalResult(payload, result));
			return handleOptionalResult(payload, result);
		}
		return def.innerType._zod.run(payload, ctx);
	};
});
var $ZodExactOptional = /*@__PURE__*/ $constructor("$ZodExactOptional", (inst, def) => {
	$ZodOptional.init(inst, def);
	defineLazyInternal(inst, "values", (zod) => zod.def.innerType._zod.values);
	defineLazyInternal(inst, "pattern", (zod) => zod.def.innerType._zod.pattern);
	inst._zod.parse = (payload, ctx) => {
		return def.innerType._zod.run(payload, ctx);
	};
});
var $ZodNullable = /*@__PURE__*/ $constructor("$ZodNullable", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "optin", (zod) => zod.def.innerType._zod.optin);
	defineLazyInternal(inst, "optout", (zod) => zod.def.innerType._zod.optout);
	defineLazyInternal(inst, "pattern", (zod) => {
		const pattern = zod.def.innerType._zod.pattern;
		return pattern ? new RegExp(`^(${cleanRegex(pattern.source)}|null)$`) : void 0;
	});
	defineLazyInternal(inst, "values", (zod) => {
		return zod.def.innerType._zod.values ? /* @__PURE__ */ new Set([...zod.def.innerType._zod.values, null]) : void 0;
	});
	inst._zod.parse = (payload, ctx) => {
		if (payload.value === null) return payload;
		return def.innerType._zod.run(payload, ctx);
	};
});
var $ZodDefault = /*@__PURE__*/ $constructor("$ZodDefault", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.optin = "defaulted";
	defineLazyInternal(inst, "values", (zod) => zod.def.innerType._zod.values);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") return def.innerType._zod.run(payload, ctx);
		if (payload.value === void 0) {
			payload.value = def.defaultValue;
			/**
			* $ZodDefault returns the default value immediately in forward direction.
			* It doesn't pass the default value into the validator ("prefault"). There's no reason to pass the default value through validation. The validity of the default is enforced by TypeScript statically. Otherwise, it's the responsibility of the user to ensure the default is valid. In the case of pipes with divergent in/out types, you can specify the default on the `in` schema of your ZodPipe to set a "prefault" for the pipe.   */
			return payload;
		}
		const result = def.innerType._zod.run(payload, ctx);
		if (result instanceof Promise) return result.then((result) => handleDefaultResult(result, def));
		return handleDefaultResult(result, def);
	};
});
function handleDefaultResult(payload, def) {
	if (payload.value === void 0) payload.value = def.defaultValue;
	return payload;
}
var $ZodPrefault = /*@__PURE__*/ $constructor("$ZodPrefault", (inst, def) => {
	$ZodType.init(inst, def);
	inst._zod.optin = "defaulted";
	defineLazyInternal(inst, "values", (zod) => zod.def.innerType._zod.values);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") return def.innerType._zod.run(payload, ctx);
		if (payload.value === void 0) payload.value = def.defaultValue;
		return def.innerType._zod.run(payload, ctx);
	};
});
var $ZodNonOptional = /*@__PURE__*/ $constructor("$ZodNonOptional", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "values", (zod) => {
		const v = zod.def.innerType._zod.values;
		return v ? new Set([...v].filter((x) => x !== void 0)) : void 0;
	});
	inst._zod.parse = (payload, ctx) => {
		const result = def.innerType._zod.run(payload, ctx);
		if (result instanceof Promise) return result.then((result) => handleNonOptionalResult(result, inst));
		return handleNonOptionalResult(result, inst);
	};
});
function handleNonOptionalResult(payload, inst) {
	if (!payload.issues.length && payload.value === void 0) payload.issues.push({
		code: "invalid_type",
		expected: "nonoptional",
		input: payload.value,
		inst
	});
	return payload;
}
function handleCatchResult(payload, result, def, ctx) {
	if (!result.issues.length) {
		payload.value = result.value;
		if (result.memo) payload.memo = true;
		return payload;
	}
	payload.value = def.catchValue({
		...result,
		value: payload.value,
		error: { issues: result.issues.map((iss) => finalizeIssue(iss, ctx, config())) },
		input: payload.value
	});
	return payload;
}
var $ZodCatch = /*@__PURE__*/ $constructor("$ZodCatch", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "optin", (zod) => zod.def.innerType._zod.optin === "defaulted" ? "defaulted" : "optional");
	defineLazyInternal(inst, "optout", (zod) => zod.def.innerType._zod.optout);
	defineLazyInternal(inst, "values", (zod) => zod.def.innerType._zod.values);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") return def.innerType._zod.run(payload, ctx);
		const result = def.innerType._zod.run({
			value: payload.value,
			issues: []
		}, ctx);
		if (result instanceof Promise) return result.then((result) => handleCatchResult(payload, result, def, ctx));
		return handleCatchResult(payload, result, def, ctx);
	};
});
var $ZodPipe = /*@__PURE__*/ $constructor("$ZodPipe", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "values", (zod) => zod.def.in._zod.values);
	defineLazyInternal(inst, "optin", (zod) => zod.def.in._zod.optin);
	defineLazyInternal(inst, "optout", (zod) => zod.def.out._zod.optout);
	defineLazyInternal(inst, "propValues", (zod) => zod.def.in._zod.propValues);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") {
			const right = def.out._zod.run(payload, ctx);
			if (right instanceof Promise) return right.then((right) => handlePipeResult(right, def.in, ctx));
			return handlePipeResult(right, def.in, ctx);
		}
		const left = def.in._zod.run(payload, ctx);
		if (left instanceof Promise) return left.then((left) => handlePipeResult(left, def.out, ctx));
		return handlePipeResult(left, def.out, ctx);
	};
});
function handlePipeResult(left, next, ctx) {
	if (left.issues.some((iss) => iss.code !== "unrecognized_keys")) {
		left.aborted = true;
		return left;
	}
	return next._zod.run({
		value: left.value,
		issues: left.issues
	}, ctx);
}
var $ZodReadonly = /*@__PURE__*/ $constructor("$ZodReadonly", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazyInternal(inst, "propValues", (zod) => zod.def.innerType._zod.propValues);
	defineLazyInternal(inst, "values", (zod) => zod.def.innerType._zod.values);
	defineLazyInternal(inst, "optin", (zod) => zod.def.innerType?._zod?.optin);
	defineLazyInternal(inst, "optout", (zod) => zod.def.innerType?._zod?.optout);
	inst._zod.parse = (payload, ctx) => {
		if (ctx.direction === "backward") return def.innerType._zod.run(payload, ctx);
		const result = def.innerType._zod.run(payload, ctx);
		if (result instanceof Promise) return result.then(handleReadonlyResult);
		return handleReadonlyResult(result);
	};
});
function handleReadonlyResult(payload) {
	if (!payload.memo) payload.value = Object.freeze(payload.value);
	return payload;
}
var $ZodLazy = /*@__PURE__*/ $constructor("$ZodLazy", (inst, def) => {
	$ZodType.init(inst, def);
	defineLazy(inst._zod, "innerType", () => {
		const d = def;
		if (!d._cachedInner) d._cachedInner = def.getter();
		return d._cachedInner;
	});
	defineLazyInternal(inst, "pattern", (zod) => zod.innerType?._zod?.pattern);
	defineLazyInternal(inst, "propValues", (zod) => zod.innerType?._zod?.propValues);
	defineLazyInternal(inst, "optin", (zod) => zod.innerType?._zod?.optin ?? void 0);
	defineLazyInternal(inst, "optout", (zod) => zod.innerType?._zod?.optout ?? void 0);
	inst._zod.parse = (payload, ctx) => {
		return inst._zod.innerType._zod.run(payload, ctx);
	};
});
var $ZodCustom = /*@__PURE__*/ $constructor("$ZodCustom", (inst, def) => {
	$ZodCheck.init(inst, def);
	$ZodType.init(inst, def);
	inst._zod.parse = (payload, _) => {
		return payload;
	};
	inst._zod.check = (payload) => {
		const input = payload.value;
		const r = def.fn(input);
		if (r instanceof Promise) return r.then((r) => handleRefineResult(r, payload, input, inst));
		handleRefineResult(r, payload, input, inst);
	};
});
function handleRefineResult(result, payload, input, inst) {
	if (!result) {
		const _iss = {
			code: "custom",
			input,
			inst,
			path: [...inst._zod.def.path ?? []],
			continue: !inst._zod.def.abort
		};
		if (inst._zod.def.params) _iss.params = inst._zod.def.params;
		payload.issues.push(issue(_iss));
	}
}
var $ZodCyclicError = class extends Error {
	constructor() {
		super(`Cannot parse a reference cycle that closes through a transform`);
		this.name = "ZodCyclicError";
	}
};
/** Keyed off the context object every schema in one parse call already shares. */
var STATE = "~memo";
var NO_ISSUES = [];
function isRef(value) {
	return value !== null && typeof value === "object";
}
function cloneIssues(issues) {
	return issues.map((iss) => iss.path ? {
		...iss,
		path: iss.path.slice()
	} : { ...iss });
}
var recursive = /*@__PURE__*/ new WeakMap();
/** What the walk established, in order of certainty: ordered so the strongest answer among children wins. */
var NONE = 0;
var ASSUMED = 1;
var PROVEN = 2;
/** Whether this schema's subtree contains a cycle, so one parse can re-enter it. */
function isRecursive(inst, stack, resolve) {
	const cached = recursive.get(inst);
	if (cached !== void 0) return cached ? PROVEN : NONE;
	if (stack.has(inst)) return PROVEN;
	stack.add(inst);
	let result = NONE;
	const check = (child) => {
		if (result !== PROVEN && child?._zod) {
			const answer = isRecursive(child, stack, resolve);
			if (answer > result) result = answer;
		}
	};
	const shape = (sh, spread) => {
		let answer = NONE;
		for (const key of Reflect.ownKeys(sh)) {
			const desc = Object.getOwnPropertyDescriptor(sh, key);
			if (spread && !desc.enumerable) continue;
			const child = desc.get ? ASSUMED : desc.value?._zod ? isRecursive(desc.value, stack, resolve) : NONE;
			if (child > answer) answer = child;
		}
		return answer;
	};
	const merge = (answer) => {
		if (answer > result) result = answer;
	};
	const def = inst._zod.def;
	switch (def.type) {
		case "object": {
			const raw = rawShape(def);
			merge(raw ? shape(raw, true) : ASSUMED);
			check(def.catchall);
			break;
		}
		case "array":
			check(def.element);
			break;
		case "tuple":
			for (const el of def.items) check(el);
			check(def.rest);
			break;
		case "record":
		case "map":
			check(def.keyType);
			check(def.valueType);
			break;
		case "set":
			check(def.valueType);
			break;
		case "union":
			for (const el of def.options) check(el);
			break;
		case "intersection":
			check(def.left);
			check(def.right);
			break;
		case "optional":
		case "nullable":
		case "default":
		case "prefault":
		case "catch":
		case "readonly":
		case "nonoptional":
		case "promise":
		case "success":
			check(def.innerType);
			break;
		case "pipe":
			check(def.in);
			check(def.out);
			break;
		case "function":
			check(def.input);
			check(def.output);
			break;
		case "lazy": {
			const inner = def._cachedInner ?? (resolve ? inst._zod.innerType : void 0);
			merge(inner ? isRecursive(inner, stack, false) : ASSUMED);
			break;
		}
		case "template_literal":
		case "string":
		case "number":
		case "int":
		case "boolean":
		case "bigint":
		case "symbol":
		case "undefined":
		case "null":
		case "void":
		case "never":
		case "any":
		case "unknown":
		case "date":
		case "nan":
		case "enum":
		case "literal":
		case "file":
		case "transform":
		case "custom": break;
		default: for (const key in def) {
			const desc = Object.getOwnPropertyDescriptor(def, key);
			if (!desc || desc.get) continue;
			const value = desc.value;
			if (!value || typeof value !== "object") continue;
			if (value._zod) check(value);
			else if (Array.isArray(value)) for (const el of value) check(el);
		}
	}
	stack.delete(inst);
	return settle(inst, result);
}
/** An assumed answer must not outlive the resolution that settles it, so only a certain one is cached. */
function settle(inst, answer) {
	if (answer !== ASSUMED) recursive.set(inst, answer === PROVEN);
	return answer;
}
function bucketFor(state, inst) {
	let bucket = state.buckets.get(inst);
	if (!bucket) {
		bucket = /* @__PURE__ */ new WeakMap();
		state.buckets.set(inst, bucket);
	}
	return bucket;
}
var handoff;
var open = [];
var memo = {
	alloc(_inst, payload, empty) {
		const bucket = handoff;
		if (!bucket) return empty;
		handoff = void 0;
		const entry = {
			value: empty,
			issues: null
		};
		bucket.set(payload.value, entry);
		open.push(entry);
		return empty;
	},
	guard(inst) {
		var _a;
		(_a = inst._zod).deferred ?? (_a.deferred = []);
		inst._zod.deferred.push(() => {
			const base = inst._zod.parse;
			const wrapped = (payload, ctx) => {
				if (ctx.direction !== "backward" && isBackEdge(ctx, payload.value)) throw new $ZodCyclicError();
				return base(payload, ctx);
			};
			inst._zod.parse = wrapped;
			if (inst._zod.run === base) inst._zod.run = wrapped;
		});
	},
	attach(inst) {
		var _a;
		let isRecursiveInst;
		let rechecked = false;
		let lastCtx;
		let lastBucket;
		(_a = inst._zod).deferred ?? (_a.deferred = []);
		inst._zod.deferred.push(() => {
			const base = inst._zod.parse;
			const wrapped = (payload, ctx) => {
				if (isRecursiveInst === void 0) {
					const walked = isRecursive(inst, /* @__PURE__ */ new Set(), false);
					if (walked === NONE) {
						inst._zod.parse = base;
						if (inst._zod.run === wrapped) inst._zod.run = base;
						return base(payload, ctx);
					}
					if (walked === PROVEN || rechecked) isRecursiveInst = true;
					else rechecked = true;
				}
				const input = payload.value;
				if (!isRef(input)) return base(payload, ctx);
				let state = ctx[STATE];
				if (!state) {
					state = {
						buckets: /* @__PURE__ */ new WeakMap(),
						backEdges: void 0
					};
					ctx[STATE] = state;
				}
				let bucket;
				if (lastCtx === ctx) bucket = lastBucket;
				else {
					bucket = bucketFor(state, inst);
					lastCtx = ctx;
					lastBucket = bucket;
				}
				const hit = bucket.get(input);
				if (hit) {
					payload.value = hit.value;
					if (hit.issues) {
						if (hit.issues.length) payload.issues.push(...cloneIssues(hit.issues));
					} else {
						payload.memo = true;
						state.backEdges ?? (state.backEdges = /* @__PURE__ */ new WeakSet());
						state.backEdges.add(hit.value);
					}
					return payload;
				}
				handoff = bucket;
				const depth = open.length;
				const result = base(payload, ctx);
				handoff = void 0;
				const entry = open.length > depth ? open.pop() : void 0;
				if (result instanceof Promise) return result.then((r) => {
					if (entry) entry.issues = r.issues.length ? cloneIssues(r.issues) : NO_ISSUES;
					return r;
				});
				if (entry) entry.issues = result.issues.length ? cloneIssues(result.issues) : NO_ISSUES;
				return result;
			};
			inst._zod.parse = wrapped;
			if (inst._zod.run === base) inst._zod.run = wrapped;
		});
	}
};
/** The memoizer that gives containers cycle support. `zod` installs it by default; `zod/mini` opts in with `config({ memoizer: memoizer() })`. */
function memoizer() {
	return memo;
}
/** Whether this value is a node a back-edge resolved to before it finished. */
function isBackEdge(ctx, value) {
	const backEdges = ctx[STATE]?.backEdges;
	return backEdges !== void 0 && isRef(value) && backEdges.has(value);
}
var error = () => {
	const Sizable = {
		string: {
			unit: "characters",
			verb: "to have"
		},
		file: {
			unit: "bytes",
			verb: "to have"
		},
		array: {
			unit: "items",
			verb: "to have"
		},
		set: {
			unit: "items",
			verb: "to have"
		},
		map: {
			unit: "entries",
			verb: "to have"
		}
	};
	function getSizing(origin) {
		return Sizable[origin] ?? null;
	}
	const FormatDictionary = {
		regex: "input",
		email: "email address",
		url: "URL",
		emoji: "emoji",
		uuid: "UUID",
		uuidv4: "UUIDv4",
		uuidv6: "UUIDv6",
		nanoid: "nanoid",
		guid: "GUID",
		cuid: "cuid",
		cuid2: "cuid2",
		ulid: "ULID",
		xid: "XID",
		ksuid: "KSUID",
		datetime: "ISO datetime",
		date: "ISO date",
		time: "ISO time",
		duration: "ISO duration",
		ipv4: "IPv4 address",
		ipv6: "IPv6 address",
		mac: "MAC address",
		cidrv4: "IPv4 range",
		cidrv6: "IPv6 range",
		base64: "base64-encoded string",
		base64url: "base64url-encoded string",
		json_string: "JSON string",
		e164: "E.164 number",
		currency_code: "currency code",
		credit_card: "credit card number",
		iban: "IBAN",
		jwt: "JWT",
		template_literal: "input"
	};
	const TypeDictionary = { nan: "NaN" };
	function getTypeName(type, input) {
		if (type === "number" && typeof input === "number" && !Number.isFinite(input)) return String(input);
		return TypeDictionary[type] ?? type;
	}
	return (issue) => {
		switch (issue.code) {
			case "invalid_type": return `Invalid input: expected ${getTypeName(issue.expected)}, received ${getTypeName(parsedType(issue.input), issue.input)}`;
			case "invalid_value":
				if (issue.values.length === 1) return `Invalid input: expected ${stringifyPrimitive(issue.values[0])}`;
				return `Invalid option: expected one of ${joinValues(issue.values, "|")}`;
			case "too_big": {
				const adj = issue.exact ? "exactly " : issue.inclusive ? "<=" : "<";
				const sizing = getSizing(issue.origin);
				if (sizing) return `Too big: expected ${issue.origin ?? "value"} to have ${adj}${issue.maximum.toString()} ${sizing.unit ?? "elements"}`;
				return `Too big: expected ${issue.origin ?? "value"} to be ${adj}${issue.maximum.toString()}`;
			}
			case "too_small": {
				const adj = issue.exact ? "exactly " : issue.inclusive ? ">=" : ">";
				const sizing = getSizing(issue.origin);
				if (sizing) return `Too small: expected ${issue.origin} to have ${adj}${issue.minimum.toString()} ${sizing.unit}`;
				return `Too small: expected ${issue.origin} to be ${adj}${issue.minimum.toString()}`;
			}
			case "invalid_format": {
				const _issue = issue;
				if (_issue.format === "starts_with") return `Invalid string: must start with "${_issue.prefix}"`;
				if (_issue.format === "ends_with") return `Invalid string: must end with "${_issue.suffix}"`;
				if (_issue.format === "includes") return `Invalid string: must include "${_issue.includes}"`;
				if (_issue.format === "regex") return `Invalid string: must match pattern ${_issue.pattern}`;
				return `Invalid ${FormatDictionary[_issue.format] ?? issue.format}`;
			}
			case "not_multiple_of": return `Invalid number: must be a multiple of ${issue.divisor}`;
			case "unrecognized_keys": return `Unrecognized key${issue.keys.length > 1 ? "s" : ""}: ${joinValues(issue.keys, ", ")}`;
			case "invalid_key": return `Invalid key in ${issue.origin}`;
			case "invalid_union":
				if (issue.options && Array.isArray(issue.options) && issue.options.length > 0) return `Invalid discriminator value. Expected ${issue.options.map((o) => `'${o}'`).join(" | ")}`;
				if (issue.inclusive === false) return "Invalid input: more than one option matched";
				return "Invalid input";
			case "invalid_element": return `Invalid value in ${issue.origin}`;
			default: return `Invalid input`;
		}
	};
};
function en_default() {
	return { localeError: error() };
}
var _a;
var $ZodRegistry = class {
	constructor() {
		this._map = /* @__PURE__ */ new WeakMap();
		this._idmap = /* @__PURE__ */ new Map();
	}
	add(schema, ..._meta) {
		const meta = _meta[0];
		this._map.set(schema, meta);
		if (meta && typeof meta === "object" && "id" in meta) this._idmap.set(meta.id, schema);
		return this;
	}
	clear() {
		this._map = /* @__PURE__ */ new WeakMap();
		this._idmap = /* @__PURE__ */ new Map();
		return this;
	}
	remove(schema) {
		const meta = this._map.get(schema);
		if (meta && typeof meta === "object" && "id" in meta) this._idmap.delete(meta.id);
		this._map.delete(schema);
		return this;
	}
	get(schema) {
		const p = schema._zod.parent;
		if (p) {
			const pm = { ...this.get(p) ?? {} };
			delete pm.id;
			const f = {
				...pm,
				...this._map.get(schema)
			};
			return Object.keys(f).length ? f : void 0;
		}
		return this._map.get(schema);
	}
	has(schema) {
		return this._map.has(schema);
	}
};
function registry() {
	return new $ZodRegistry();
}
(_a = globalThis).__zod_globalRegistry ?? (_a.__zod_globalRegistry = registry());
var globalRegistry = globalThis.__zod_globalRegistry;
function snapshotChecks(def) {
	if (def.checks) def.checks = [...def.checks];
	return def;
}
// @__NO_SIDE_EFFECTS__
function _string(Class, params) {
	return new Class(snapshotChecks({
		type: "string",
		...normalizeParams(params)
	}));
}
// @__NO_SIDE_EFFECTS__
function _email(Class, params) {
	return new Class({
		type: "string",
		format: "email",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _guid(Class, params) {
	return new Class({
		type: "string",
		format: "guid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _uuid(Class, params) {
	return new Class({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _uuidv4(Class, params) {
	return new Class({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: false,
		version: "v4",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _uuidv6(Class, params) {
	return new Class({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: false,
		version: "v6",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _uuidv7(Class, params) {
	return new Class({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: false,
		version: "v7",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _url(Class, params) {
	return new Class({
		type: "string",
		format: "url",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _emoji(Class, params) {
	return new Class({
		type: "string",
		format: "emoji",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _nanoid(Class, params) {
	return new Class({
		type: "string",
		format: "nanoid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
/**
* @deprecated CUID v1 is deprecated by its authors due to information leakage
* (timestamps embedded in the id). Use {@link _cuid2} instead.
* See https://github.com/paralleldrive/cuid.
*/
// @__NO_SIDE_EFFECTS__
function _cuid(Class, params) {
	return new Class({
		type: "string",
		format: "cuid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _cuid2(Class, params) {
	return new Class({
		type: "string",
		format: "cuid2",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _ulid(Class, params) {
	return new Class({
		type: "string",
		format: "ulid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _xid(Class, params) {
	return new Class({
		type: "string",
		format: "xid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _ksuid(Class, params) {
	return new Class({
		type: "string",
		format: "ksuid",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _ipv4(Class, params) {
	return new Class({
		type: "string",
		format: "ipv4",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _ipv6(Class, params) {
	return new Class({
		type: "string",
		format: "ipv6",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _cidrv4(Class, params) {
	return new Class({
		type: "string",
		format: "cidrv4",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _cidrv6(Class, params) {
	return new Class({
		type: "string",
		format: "cidrv6",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _base64(Class, params) {
	return new Class({
		type: "string",
		format: "base64",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _base64url(Class, params) {
	return new Class({
		type: "string",
		format: "base64url",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _e164(Class, params) {
	return new Class({
		type: "string",
		format: "e164",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _jwt(Class, params) {
	return new Class({
		type: "string",
		format: "jwt",
		check: "string_format",
		abort: false,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _isoDateTime(Class, params) {
	return new Class({
		type: "string",
		format: "datetime",
		check: "string_format",
		offset: false,
		local: false,
		precision: null,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _isoDate(Class, params) {
	return new Class({
		type: "string",
		format: "date",
		check: "string_format",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _isoTime(Class, params) {
	return new Class({
		type: "string",
		format: "time",
		check: "string_format",
		precision: null,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _isoDuration(Class, params) {
	return new Class({
		type: "string",
		format: "duration",
		check: "string_format",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _number(Class, params) {
	return new Class(snapshotChecks({
		type: "number",
		checks: [],
		...normalizeParams(params)
	}));
}
// @__NO_SIDE_EFFECTS__
function _int(Class, params) {
	return new Class({
		type: "number",
		check: "number_format",
		abort: false,
		format: "safeint",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _boolean(Class, params) {
	return new Class({
		type: "boolean",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _null$1(Class, params) {
	return new Class({
		type: "null",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _unknown(Class) {
	return new Class({ type: "unknown" });
}
// @__NO_SIDE_EFFECTS__
function _never(Class, params) {
	return new Class({
		type: "never",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _lt(value, params) {
	return new $ZodCheckLessThan({
		check: "less_than",
		...normalizeParams(params),
		value,
		inclusive: false
	});
}
// @__NO_SIDE_EFFECTS__
function _lte(value, params) {
	return new $ZodCheckLessThan({
		check: "less_than",
		...normalizeParams(params),
		value,
		inclusive: true
	});
}
// @__NO_SIDE_EFFECTS__
function _gt(value, params) {
	return new $ZodCheckGreaterThan({
		check: "greater_than",
		...normalizeParams(params),
		value,
		inclusive: false
	});
}
// @__NO_SIDE_EFFECTS__
function _gte(value, params) {
	return new $ZodCheckGreaterThan({
		check: "greater_than",
		...normalizeParams(params),
		value,
		inclusive: true
	});
}
// @__NO_SIDE_EFFECTS__
function _multipleOf(value, params) {
	return new $ZodCheckMultipleOf({
		check: "multiple_of",
		...normalizeParams(params),
		value
	});
}
// @__NO_SIDE_EFFECTS__
function _maxLength(maximum, params) {
	return new $ZodCheckMaxLength({
		check: "max_length",
		...normalizeParams(params),
		maximum
	});
}
// @__NO_SIDE_EFFECTS__
function _minLength(minimum, params) {
	return new $ZodCheckMinLength({
		check: "min_length",
		...normalizeParams(params),
		minimum
	});
}
// @__NO_SIDE_EFFECTS__
function _length(length, params) {
	return new $ZodCheckLengthEquals({
		check: "length_equals",
		...normalizeParams(params),
		length
	});
}
// @__NO_SIDE_EFFECTS__
function _regex(pattern, params) {
	return new $ZodCheckRegex({
		check: "string_format",
		format: "regex",
		...normalizeParams(params),
		pattern
	});
}
// @__NO_SIDE_EFFECTS__
function _lowercase(params) {
	return new $ZodCheckLowerCase({
		check: "string_format",
		format: "lowercase",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _uppercase(params) {
	return new $ZodCheckUpperCase({
		check: "string_format",
		format: "uppercase",
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _includes(includes, params) {
	return new $ZodCheckIncludes({
		check: "string_format",
		format: "includes",
		...normalizeParams(params),
		includes
	});
}
// @__NO_SIDE_EFFECTS__
function _startsWith(prefix, params) {
	return new $ZodCheckStartsWith({
		check: "string_format",
		format: "starts_with",
		...normalizeParams(params),
		prefix
	});
}
// @__NO_SIDE_EFFECTS__
function _endsWith(suffix, params) {
	return new $ZodCheckEndsWith({
		check: "string_format",
		format: "ends_with",
		...normalizeParams(params),
		suffix
	});
}
// @__NO_SIDE_EFFECTS__
function _overwrite(tx) {
	return new $ZodCheckOverwrite({
		check: "overwrite",
		tx
	});
}
// @__NO_SIDE_EFFECTS__
function _normalize(form) {
	return /* @__PURE__ */ _overwrite((input) => input.normalize(form));
}
// @__NO_SIDE_EFFECTS__
function _trim() {
	return /* @__PURE__ */ _overwrite((input) => input.trim());
}
// @__NO_SIDE_EFFECTS__
function _toLowerCase() {
	return /* @__PURE__ */ _overwrite((input) => input.toLowerCase());
}
// @__NO_SIDE_EFFECTS__
function _toUpperCase() {
	return /* @__PURE__ */ _overwrite((input) => input.toUpperCase());
}
// @__NO_SIDE_EFFECTS__
function _slugify() {
	return /* @__PURE__ */ _overwrite((input) => slugify(input));
}
// @__NO_SIDE_EFFECTS__
function _array(Class, element, params) {
	return new Class({
		type: "array",
		element,
		...normalizeParams(params)
	});
}
// @__NO_SIDE_EFFECTS__
function _refine(Class, fn, _params) {
	return new Class({
		type: "custom",
		check: "custom",
		fn,
		...normalizeParams(_params)
	});
}
// @__NO_SIDE_EFFECTS__
function _superRefine(fn, params) {
	const ch = /* @__PURE__ */ _check((payload) => {
		payload.addIssue = (issue$2) => {
			if (typeof issue$2 === "string") payload.issues.push(issue(issue$2, payload.value, ch._zod.def));
			else {
				const _issue = issue$2;
				if (_issue.fatal) _issue.continue = false;
				_issue.code ?? (_issue.code = "custom");
				if (!("input" in _issue)) _issue.input = payload.value;
				_issue.inst ?? (_issue.inst = ch);
				_issue.continue ?? (_issue.continue = !ch._zod.def.abort);
				payload.issues.push(issue(_issue));
			}
		};
		return fn(payload.value, payload);
	}, params);
	return ch;
}
// @__NO_SIDE_EFFECTS__
function _check(fn, params) {
	const ch = new $ZodCheck({
		check: "custom",
		...normalizeParams(params)
	});
	ch._zod.check = fn;
	return ch;
}
function assignProps(target, ...sources) {
	for (const source of sources) for (const key of Reflect.ownKeys(source)) if (Object.prototype.propertyIsEnumerable.call(source, key)) assignProp(target, key, source[key]);
	return target;
}
function initializeContext(params) {
	let target = params?.target ?? "draft-2020-12";
	if (target === "draft-4") target = "draft-04";
	if (target === "draft-7") target = "draft-07";
	return {
		processors: params.processors ?? {},
		metadataRegistry: params?.metadata ?? globalRegistry,
		target,
		unrepresentable: params?.unrepresentable ?? "throw",
		override: params?.override ?? (() => {}),
		io: params?.io ?? "output",
		counter: 0,
		seen: /* @__PURE__ */ new Map(),
		sharedDefsExtractedFor: void 0,
		sharedEmitDoneFor: void 0,
		cycles: params?.cycles ?? "ref",
		reused: params?.reused ?? "inline",
		intersections: [],
		deferred: [],
		external: params?.external ?? void 0
	};
}
/**
* Applies the `unrepresentable` setting at a site that has no JSON Schema equivalent. Throws
* `message` unless the setting (or the handler's return value) says otherwise. Returns `true` if a
* custom JSON Schema was written into `json`, in which case the caller must not write its own.
*/
function handleUnrepresentable(schema, ctx, json, params, message) {
	const result = typeof ctx.unrepresentable === "function" ? ctx.unrepresentable({
		zodSchema: schema,
		path: params.path,
		message
	}) : ctx.unrepresentable;
	if (result === "any") return false;
	if (result === void 0 || result === "throw") throw new Error(message);
	Object.assign(json, result);
	return true;
}
function processSchema(schema, ctx, _params = {
	path: [],
	schemaPath: []
}) {
	var _a;
	const def = schema._zod.def;
	const seen = ctx.seen.get(schema);
	if (seen) {
		seen.count++;
		if (_params.schemaPath.includes(schema)) seen.cycle = _params.path;
		return seen.schema;
	}
	const result = {
		schema: {},
		count: 1,
		cycle: void 0,
		path: _params.path
	};
	ctx.seen.set(schema, result);
	ctx.sharedDefsExtractedFor = void 0;
	ctx.sharedEmitDoneFor = void 0;
	const overrideSchema = schema._zod.toJSONSchema?.();
	if (overrideSchema) result.schema = overrideSchema;
	else {
		const params = {
			..._params,
			schemaPath: [..._params.schemaPath, schema],
			path: _params.path
		};
		if (schema._zod.processJSONSchema) schema._zod.processJSONSchema(ctx, result.schema, params);
		else {
			const _json = result.schema;
			const processor = ctx.processors[def.type];
			if (!processor) throw new Error(`[toJSONSchema]: Non-representable type encountered: ${def.type}`);
			processor(schema, ctx, _json, params);
		}
		const parent = schema._zod.parent;
		if (parent) {
			if (!result.ref) result.ref = parent;
			processSchema(parent, ctx, params);
			ctx.seen.get(parent).isParent = true;
		}
	}
	const meta = ctx.metadataRegistry.get(schema);
	if (meta) assignProps(result.schema, meta);
	if (ctx.io === "input" && isTransforming(schema)) {
		delete result.schema.examples;
		delete result.schema.default;
	}
	if (ctx.io === "input" && "_prefault" in result.schema) (_a = result.schema).default ?? (_a.default = result.schema._prefault);
	delete result.schema._prefault;
	return ctx.seen.get(schema).schema;
}
function encodeJSONPointerSegment(segment) {
	return segment.replace(/~/g, "~0").replace(/\//g, "~1");
}
function extractDefs(ctx, schema) {
	const root = ctx.seen.get(schema);
	if (!root) throw new Error("Unprocessed schema. This is a bug in Zod.");
	if (ctx.external && ctx.sharedDefsExtractedFor === ctx.external) return;
	const idToSchema = /* @__PURE__ */ new Map();
	for (const entry of ctx.seen.entries()) {
		const id = ctx.metadataRegistry.get(entry[0])?.id;
		if (id) {
			const existing = idToSchema.get(id);
			if (existing && existing !== entry[0]) throw new Error(`Duplicate schema id "${id}" detected during JSON Schema conversion. Two different schemas cannot share the same id when converted together.`);
			idToSchema.set(id, entry[0]);
		}
	}
	const makeURI = (entry) => {
		const defsSegment = ctx.target === "draft-2020-12" ? "$defs" : "definitions";
		if (ctx.external) {
			const externalId = ctx.external.registry.get(entry[0])?.id;
			const uriGenerator = ctx.external.uri ?? ((id) => id);
			if (externalId) return { ref: uriGenerator(externalId) };
			const id = entry[1].defId ?? entry[1].schema.id ?? `schema${ctx.counter++}`;
			entry[1].defId = id;
			return {
				defId: id,
				ref: `${uriGenerator("__shared")}#/${defsSegment}/${encodeJSONPointerSegment(id)}`
			};
		}
		const uriPrefix = `#`;
		const defUriPrefix = `${uriPrefix}/${defsSegment}/`;
		if (entry[1] === root && !entry[1].schema.id) return { ref: uriPrefix };
		const defId = entry[1].schema.id ?? `__schema${ctx.counter++}`;
		return {
			defId,
			ref: defUriPrefix + encodeJSONPointerSegment(defId)
		};
	};
	const extractToDef = (entry) => {
		if (entry[1].schema.$ref) return;
		const seen = entry[1];
		const { ref, defId } = makeURI(entry);
		seen.def = { ...seen.schema };
		if (defId) seen.defId = defId;
		const schema = seen.schema;
		for (const key in schema) delete schema[key];
		schema.$ref = ref;
	};
	if (ctx.cycles === "throw") for (const entry of ctx.seen.entries()) {
		const seen = entry[1];
		if (seen.cycle) throw new Error(`Cycle detected: #/${seen.cycle?.join("/")}/<root>

Set the \`cycles\` parameter to \`"ref"\` to resolve cyclical schemas with defs.`);
	}
	for (const entry of ctx.seen.entries()) {
		const seen = entry[1];
		if (schema === entry[0]) {
			extractToDef(entry);
			continue;
		}
		if (ctx.external) {
			const ext = ctx.external.registry.get(entry[0])?.id;
			if (schema !== entry[0] && ext) {
				extractToDef(entry);
				continue;
			}
		}
		if (ctx.metadataRegistry.get(entry[0])?.id) {
			extractToDef(entry);
			continue;
		}
		if (seen.cycle) {
			extractToDef(entry);
			continue;
		}
		if (seen.count > 1) {
			if (ctx.reused === "ref") extractToDef(entry);
		}
	}
	if (ctx.external) ctx.sharedDefsExtractedFor = ctx.external;
}
/** Rewrites `anyOf: [{type: "a"}, {type: "b"}]` to `type: ["a", "b"]`, which every JSON Schema draft treats as equivalent and most consumers render far better for the nullable case. Only branches that are a bare type assertion qualify — anything carrying a constraint, `$ref`, `const` or metadata is left alone. Runs after `flattenRef`, so a branch an override decorated or `$defs` extraction turned into a `$ref` is no longer bare and correctly stays in `anyOf`. `oneOf` is excluded: `integer` and `number` overlap, so "exactly one" and "at least one" are not the same there. OpenAPI 3.0 is excluded: its `type` must be a single string. */
function compactTypeUnion(schema) {
	const options = schema.anyOf;
	if (!Array.isArray(options) || options.length === 0 || schema.type !== void 0) return;
	const types = [];
	for (const option of options) {
		if (!option || typeof option !== "object") return;
		compactTypeUnion(option);
		const keys = Object.keys(option);
		if (keys.length !== 1 || keys[0] !== "type") return;
		const type = option.type;
		for (const member of Array.isArray(type) ? type : [type]) {
			if (typeof member !== "string") return;
			if (!types.includes(member)) types.push(member);
		}
	}
	delete schema.anyOf;
	schema.type = types.length === 1 ? types[0] : types;
}
/** Keywords `foldIntersection` knows how to combine. Anything else — `$ref`, `patternProperties`,
* an annotation like `description` — makes a member unfoldable, so a constraint this does not
* understand leaves the `allOf` alone instead of being silently dropped or misattributed. */
var FOLDABLE_KEYS = /* @__PURE__ */ new Set([
	"type",
	"properties",
	"required",
	"additionalProperties"
]);
var UNION_KEYS = ["oneOf", "anyOf"];
/** A member's constraint on a key it does not declare itself. A `catchall` states one; `false`, an absent `additionalProperties`, and the empty schema a loose object emits state nothing. */
function undeclaredConstraint(member) {
	const extra = member.additionalProperties;
	if (extra === void 0 || extra === false || typeof extra !== "object" || extra === null) return null;
	return Object.keys(extra).length ? extra : null;
}
/** Combines object members into the single object they describe together, or returns `null` if any of them carries a keyword outside {@link FOLDABLE_KEYS}. */
function foldObjects(members) {
	const objects = [];
	for (const member of members) {
		if (typeof member !== "object" || member.type !== "object") return null;
		for (const key in member) if (!FOLDABLE_KEYS.has(key)) return null;
		objects.push(member);
	}
	const properties = {};
	const required = /* @__PURE__ */ new Set();
	for (const object of objects) {
		for (const key in object.properties) {
			if (Object.prototype.hasOwnProperty.call(properties, key)) continue;
			const parts = [];
			for (const other of objects) {
				const part = other.properties?.[key] ?? undeclaredConstraint(other);
				if (part === null || part === void 0) continue;
				if (!parts.some((seen) => JSON.stringify(seen) === JSON.stringify(part))) parts.push(part);
			}
			assignProp(properties, key, parts.length === 1 ? parts[0] : foldObjects(parts) ?? { allOf: parts });
		}
		for (const key of object.required ?? []) required.add(key);
	}
	const folded = {
		type: "object",
		properties
	};
	if (required.size) folded.required = [...required];
	if (objects.every((object) => object.additionalProperties === false)) folded.additionalProperties = false;
	else {
		const constraints = [];
		for (const object of objects) {
			const constraint = undeclaredConstraint(object);
			if (constraint && !constraints.some((seen) => JSON.stringify(seen) === JSON.stringify(constraint))) constraints.push(constraint);
		}
		if (constraints.length === 1) folded.additionalProperties = constraints[0];
		else if (constraints.length > 1) folded.additionalProperties = { allOf: constraints };
	}
	return folded;
}
/** `additionalProperties` in an `allOf` member sees only that member's own `properties`, so two
* closed object members reject each other's keys and the schema validates nothing. Zod's parser
* pools the key sets instead — `handleIntersectionResults` reports a key as unrecognized only when
* *every* side rejects it — so the emitted schema has to pool them too, and folding the members
* into one object is the encoding that says so on every target.
*
* This runs from `finalize`, after `extractDefs`, which is what keeps it clear of the `$ref`
* machinery: a member extracted into `$defs` is already a `$ref` by now and declines to fold, so it
* keeps its reference and its own closedness rather than being inlined as a stale copy. */
function foldIntersection(json) {
	const allOf = json.allOf;
	if (!Array.isArray(allOf) || allOf.length < 2) return;
	for (const key of FOLDABLE_KEYS) if (key in json) return;
	const unions = allOf.filter((m) => UNION_KEYS.some((k) => Array.isArray(m[k])));
	let folded = null;
	if (!unions.length) folded = foldObjects(allOf);
	else {
		const union = unions[0];
		const keyword = UNION_KEYS.find((k) => Array.isArray(union[k]));
		if (Object.keys(union).length !== 1) return;
		const rest = allOf.filter((m) => m !== union);
		const branches = union[keyword].map((branch) => foldObjects([...rest, branch]));
		if (branches.some((b) => !b)) return;
		folded = { [keyword]: branches };
	}
	if (!folded) return;
	delete json.allOf;
	assignProps(json, folded);
}
function finalize(ctx, schema) {
	const root = ctx.seen.get(schema);
	if (!root) throw new Error("Unprocessed schema. This is a bug in Zod.");
	const flattenRef = (zodSchema) => {
		const seen = ctx.seen.get(zodSchema);
		if (seen.ref === null) return;
		const schema = seen.def ?? seen.schema;
		const _cached = { ...schema };
		const ref = seen.ref;
		seen.ref = null;
		if (ref) {
			flattenRef(ref);
			const refSeen = ctx.seen.get(ref);
			const refSchema = refSeen.schema;
			if (refSchema.$ref && (ctx.target === "draft-07" || ctx.target === "draft-04" || ctx.target === "openapi-3.0")) {
				schema.allOf = schema.allOf ?? [];
				schema.allOf.push(refSchema);
			} else assignProps(schema, refSchema);
			assignProps(schema, _cached);
			if (zodSchema._zod.parent === ref) for (const key in schema) {
				if (key === "$ref" || key === "allOf") continue;
				if (!(key in _cached)) delete schema[key];
			}
			if (refSchema.$ref && refSeen.def) for (const key in schema) {
				if (key === "$ref" || key === "allOf") continue;
				if (key in refSeen.def && JSON.stringify(schema[key]) === JSON.stringify(refSeen.def[key])) delete schema[key];
			}
		}
		const parent = zodSchema._zod.parent;
		if (parent && parent !== ref) {
			flattenRef(parent);
			const parentSeen = ctx.seen.get(parent);
			if (parentSeen?.schema.$ref) {
				schema.$ref = parentSeen.schema.$ref;
				if (parentSeen.def) for (const key in schema) {
					if (key === "$ref" || key === "allOf") continue;
					if (key in parentSeen.def && JSON.stringify(schema[key]) === JSON.stringify(parentSeen.def[key])) delete schema[key];
				}
			}
		}
		ctx.override({
			zodSchema,
			jsonSchema: schema,
			path: seen.path ?? []
		});
	};
	if (!ctx.external || ctx.sharedEmitDoneFor !== ctx.external) {
		for (const entry of [...ctx.seen.entries()].reverse()) flattenRef(entry[0]);
		if (ctx.target !== "openapi-3.0") for (const entry of ctx.seen.entries()) compactTypeUnion(entry[1].def ?? entry[1].schema);
		for (const rewrite of ctx.deferred) rewrite();
		if (ctx.intersections.length) {
			const carriers = /* @__PURE__ */ new Map();
			for (const seen of ctx.seen.values()) for (const json of [seen.schema, seen.def]) {
				const allOf = json?.allOf;
				if (!Array.isArray(allOf)) continue;
				const existing = carriers.get(allOf);
				if (existing) existing.push(json);
				else carriers.set(allOf, [json]);
			}
			for (const allOf of ctx.intersections) for (const json of carriers.get(allOf) ?? []) foldIntersection(json);
		}
	}
	const result = {};
	if (ctx.target === "draft-2020-12") result.$schema = "https://json-schema.org/draft/2020-12/schema";
	else if (ctx.target === "draft-07") result.$schema = "http://json-schema.org/draft-07/schema#";
	else if (ctx.target === "draft-04") result.$schema = "http://json-schema.org/draft-04/schema#";
	else if (ctx.target === "openapi-3.0") {}
	if (ctx.external?.uri) {
		const id = ctx.external.registry.get(schema)?.id;
		if (!id) throw new Error("Schema is missing an `id` property");
		result.$id = ctx.external.uri(id);
	}
	assignProps(result, root.defId ? root.schema : root.def ?? root.schema);
	const rootMetaId = ctx.metadataRegistry.get(schema)?.id;
	if (rootMetaId !== void 0 && result.id === rootMetaId) delete result.id;
	const defs = ctx.external?.defs ?? {};
	if (!ctx.external || ctx.sharedEmitDoneFor !== ctx.external) for (const entry of ctx.seen.entries()) {
		const seen = entry[1];
		if (seen.def && seen.defId) {
			if (seen.def.id === seen.defId) delete seen.def.id;
			assignProp(defs, seen.defId, seen.def);
		}
	}
	if (ctx.external) ctx.sharedEmitDoneFor = ctx.external;
	if (ctx.external) {} else if (Object.keys(defs).length > 0) {
		if (ctx.target === "draft-2020-12") result.$defs = defs;
		else result.definitions = defs;
	}
	try {
		const finalized = JSON.parse(JSON.stringify(result));
		Object.defineProperty(finalized, "~standard", {
			value: {
				...schema["~standard"],
				jsonSchema: {
					input: createStandardJSONSchemaMethod(schema, "input", ctx.processors),
					output: createStandardJSONSchemaMethod(schema, "output", ctx.processors)
				}
			},
			enumerable: false,
			writable: false
		});
		return finalized;
	} catch (_err) {
		throw new Error("Error converting schema to JSON.");
	}
}
function isTransforming(_schema, _ctx) {
	const ctx = _ctx ?? { seen: /* @__PURE__ */ new Set() };
	if (ctx.seen.has(_schema)) return false;
	ctx.seen.add(_schema);
	const def = _schema._zod.def;
	if (def.type === "transform") return true;
	if (def.type === "array") return isTransforming(def.element, ctx);
	if (def.type === "set") return isTransforming(def.valueType, ctx);
	if (def.type === "lazy") return isTransforming(def.getter(), ctx);
	if (def.type === "promise" || def.type === "optional" || def.type === "nonoptional" || def.type === "nullable" || def.type === "readonly" || def.type === "default" || def.type === "prefault" || def.type === "catch") return isTransforming(def.innerType, ctx);
	if (def.type === "intersection") return isTransforming(def.left, ctx) || isTransforming(def.right, ctx);
	if (def.type === "record" || def.type === "map") return isTransforming(def.keyType, ctx) || isTransforming(def.valueType, ctx);
	if (def.type === "pipe") {
		if (_schema._zod.traits.has("$ZodCodec")) return true;
		return isTransforming(def.in, ctx) || isTransforming(def.out, ctx);
	}
	if (def.type === "object") {
		for (const key in def.shape) if (isTransforming(def.shape[key], ctx)) return true;
		return false;
	}
	if (def.type === "union") {
		for (const option of def.options) if (isTransforming(option, ctx)) return true;
		return false;
	}
	if (def.type === "tuple") {
		for (const item of def.items) if (isTransforming(item, ctx)) return true;
		if (def.rest && isTransforming(def.rest, ctx)) return true;
		return false;
	}
	return false;
}
/**
* Creates a toJSONSchema method for a schema instance.
* This encapsulates the logic of initializing context, processing, extracting defs, and finalizing.
*/
var createToJSONSchemaMethod = (schema, processors = {}) => (params) => {
	const ctx = initializeContext({
		...params,
		processors
	});
	processSchema(schema, ctx);
	extractDefs(ctx, schema);
	return finalize(ctx, schema);
};
var createStandardJSONSchemaMethod = (schema, io, processors = {}) => (params) => {
	const { libraryOptions, target } = params ?? {};
	const ctx = initializeContext({
		...libraryOptions ?? {},
		target,
		io,
		processors
	});
	processSchema(schema, ctx);
	extractDefs(ctx, schema);
	return finalize(ctx, schema);
};
var narrowMin = (agg, key, value) => {
	if (agg[key] === void 0 || value > agg[key]) agg[key] = value;
};
var narrowMax = (agg, key, value) => {
	if (agg[key] === void 0 || value < agg[key]) agg[key] = value;
};
var narrowBoth = (agg, value) => {
	narrowMin(agg, "minimum", value);
	narrowMax(agg, "maximum", value);
};
var addDivisor = (agg, value) => {
	agg.multipleOf ?? (agg.multipleOf = []);
	if (!agg.multipleOf.includes(value)) agg.multipleOf.push(value);
};
var addPattern = (agg, pattern) => {
	agg.patterns ?? (agg.patterns = /* @__PURE__ */ new Set());
	agg.patterns.add(pattern);
};
var intersectMime = (agg, mime) => {
	agg.mime = agg.mime ? agg.mime.filter((m) => mime.includes(m)) : [...mime];
};
var setFormat = (agg, format) => {
	agg.format = format;
	if (format.includes("int")) agg.isInt = true;
};
var minContributor = (agg, def) => narrowMin(agg, "minimum", def.minimum);
var maxContributor = (agg, def) => narrowMax(agg, "maximum", def.maximum);
var formatContributor = (ranges) => (agg, def) => {
	setFormat(agg, def.format);
	const [minimum, maximum] = ranges[def.format];
	narrowMin(agg, "minimum", minimum);
	narrowMax(agg, "maximum", maximum);
};
var contributors = {
	greater_than: (agg, def) => narrowMin(agg, def.inclusive ? "minimum" : "exclusiveMinimum", def.value),
	less_than: (agg, def) => narrowMax(agg, def.inclusive ? "maximum" : "exclusiveMaximum", def.value),
	multiple_of: (agg, def) => addDivisor(agg, def.value),
	number_format: formatContributor(NUMBER_FORMAT_RANGES),
	bigint_format: formatContributor(BIGINT_FORMAT_RANGES),
	min_length: minContributor,
	max_length: maxContributor,
	length_equals: (agg, def) => narrowBoth(agg, def.length),
	min_size: minContributor,
	max_size: maxContributor,
	size_equals: (agg, def) => narrowBoth(agg, def.size),
	string_format: (agg, def) => {
		setFormat(agg, def.format);
		if (def.pattern) addPattern(agg, def.pattern);
		if (def.format === "base64" || def.format === "base64url") agg.contentEncoding = def.format;
		if (def.local || def.precision === -1) agg.laxFormat = true;
	},
	mime_type: (agg, def) => intersectMime(agg, def.mime)
};
function aggregateChecks(schema) {
	const agg = {};
	const def = schema._zod.def;
	const list = schema._zod.traits.has("$ZodCheck") ? [schema, ...def.checks ?? []] : def.checks ?? [];
	for (const ch of list) contributors[ch._zod.def.check]?.(agg, ch._zod.def);
	const bag = schema._zod.bag;
	if (bag.minimum !== void 0) narrowMin(agg, "minimum", bag.minimum);
	if (bag.exclusiveMinimum !== void 0) narrowMin(agg, "exclusiveMinimum", bag.exclusiveMinimum);
	if (bag.maximum !== void 0) narrowMax(agg, "maximum", bag.maximum);
	if (bag.exclusiveMaximum !== void 0) narrowMax(agg, "exclusiveMaximum", bag.exclusiveMaximum);
	if (bag.multipleOf !== void 0) addDivisor(agg, bag.multipleOf);
	if (bag.format !== void 0) {
		agg.format ?? (agg.format = bag.format);
		if (bag.format.includes("int")) agg.isInt = true;
	}
	if (bag.mime) intersectMime(agg, bag.mime);
	for (const pattern of bag.patterns ?? []) addPattern(agg, pattern);
	return agg;
}
var formatMap = {
	guid: "uuid",
	url: "uri",
	datetime: "date-time",
	json_string: "json-string",
	regex: ""
};
var exactPatterns = /* @__PURE__ */ new Map([[base64Charset, base64], [base64urlCharset, base64url]]);
var exactPattern = (p) => exactPatterns.get(p) ?? p;
var stringProcessor = (schema, ctx, _json, _params) => {
	const json = _json;
	json.type = "string";
	const { minimum, maximum, format, patterns, contentEncoding, laxFormat } = aggregateChecks(schema);
	if (typeof minimum === "number") json.minLength = minimum;
	if (typeof maximum === "number") json.maxLength = maximum;
	if (format) {
		json.format = formatMap[format] ?? format;
		if (json.format === "") delete json.format;
		if (format === "time" || laxFormat) delete json.format;
	}
	if (contentEncoding) json.contentEncoding = contentEncoding;
	if (patterns && patterns.size > 0) {
		const patternList = [...patterns].map(exactPattern);
		if (patternList.length === 1) json.pattern = patternList[0].source;
		else if (patternList.length > 1) json.allOf = [...patternList.map((regex) => ({
			...ctx.target === "draft-07" || ctx.target === "draft-04" || ctx.target === "openapi-3.0" ? { type: "string" } : {},
			pattern: regex.source
		}))];
	}
};
var numberProcessor = (schema, ctx, _json, params) => {
	const json = _json;
	const { minimum, maximum, multipleOf, exclusiveMaximum, exclusiveMinimum, isInt } = aggregateChecks(schema);
	json.type = isInt ? "integer" : "number";
	const exMin = typeof exclusiveMinimum === "number" && exclusiveMinimum >= (minimum ?? Number.NEGATIVE_INFINITY);
	const exMax = typeof exclusiveMaximum === "number" && exclusiveMaximum <= (maximum ?? Number.POSITIVE_INFINITY);
	const legacy = ctx.target === "draft-04" || ctx.target === "openapi-3.0";
	if (exMin) {
		if (legacy) {
			json.minimum = exclusiveMinimum;
			json.exclusiveMinimum = true;
		} else json.exclusiveMinimum = exclusiveMinimum;
	} else if (typeof minimum === "number") json.minimum = minimum;
	if (exMax) {
		if (legacy) {
			json.maximum = exclusiveMaximum;
			json.exclusiveMaximum = true;
		} else json.exclusiveMaximum = exclusiveMaximum;
	} else if (typeof maximum === "number") json.maximum = maximum;
	if (multipleOf) {
		const divisors = /* @__PURE__ */ new Set();
		for (const divisor of multipleOf) if (Number.isFinite(divisor) && divisor !== 0) divisors.add(Math.abs(divisor));
		else handleUnrepresentable(schema, ctx, json, params, `A multipleOf divisor of ${divisor} cannot be represented in JSON Schema`);
		const [first, ...rest] = divisors;
		if (first !== void 0) json.multipleOf = first;
		if (rest.length) json.allOf = [...json.allOf ?? [], ...rest.map((m) => ({ multipleOf: m }))];
	}
};
var booleanProcessor = (_schema, _ctx, json, _params) => {
	json.type = "boolean";
};
var nullProcessor = (_schema, ctx, json, _params) => {
	if (ctx.target === "openapi-3.0") {
		json.type = "string";
		json.nullable = true;
		json.enum = [null];
	} else json.type = "null";
};
var neverProcessor = (_schema, _ctx, json, _params) => {
	json.not = {};
};
var enumProcessor = (schema, _ctx, json, _params) => {
	const def = schema._zod.def;
	const values = getEnumValues(def.entries);
	if (values.length === 0) {
		json.not = {};
		return;
	}
	if (values.every((v) => typeof v === "number")) json.type = "number";
	if (values.every((v) => typeof v === "string")) json.type = "string";
	json.enum = values;
};
var literalProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	if (def.values.length === 0) {
		json.not = {};
		return;
	}
	const vals = [];
	for (const val of def.values) if (val === void 0) {
		if (handleUnrepresentable(schema, ctx, json, params, "Literal `undefined` cannot be represented in JSON Schema")) return;
	} else if (typeof val === "bigint") {
		if (handleUnrepresentable(schema, ctx, json, params, "BigInt literals cannot be represented in JSON Schema")) return;
		vals.push(Number(val));
	} else vals.push(val);
	if (vals.length === 0) {} else if (vals.length === 1) {
		const val = vals[0];
		json.type = val === null ? "null" : typeof val;
		if (ctx.target === "draft-04" || ctx.target === "openapi-3.0") json.enum = [val];
		else json.const = val;
	} else {
		if (vals.every((v) => typeof v === "number")) json.type = "number";
		if (vals.every((v) => typeof v === "string")) json.type = "string";
		if (vals.every((v) => typeof v === "boolean")) json.type = "boolean";
		if (vals.every((v) => v === null)) json.type = "null";
		json.enum = vals;
	}
};
var customProcessor = (schema, ctx, json, params) => {
	handleUnrepresentable(schema, ctx, json, params, "Custom types cannot be represented in JSON Schema");
};
var transformProcessor = (schema, ctx, json, params) => {
	handleUnrepresentable(schema, ctx, json, params, "Transforms cannot be represented in JSON Schema");
};
var arrayProcessor = (schema, ctx, _json, params) => {
	const json = _json;
	const def = schema._zod.def;
	const { minimum, maximum } = aggregateChecks(schema);
	if (typeof minimum === "number") json.minItems = minimum;
	if (typeof maximum === "number") json.maxItems = maximum;
	json.type = "array";
	json.items = processSchema(def.element, ctx, {
		...params,
		path: [...params.path, "items"]
	});
};
function inputOptin(schema) {
	const def = schema._zod.def;
	if (def.type === "pipe" && def.in._zod.traits.has("$ZodTransform")) return inputOptin(def.out);
	if (def.type === "catch") return inputOptin(def.innerType);
	return schema._zod.optin;
}
var objectProcessor = (schema, ctx, _json, params) => {
	const json = _json;
	const def = schema._zod.def;
	const shape = def.shape;
	if (Object.getOwnPropertySymbols(shape).length && handleUnrepresentable(schema, ctx, json, params, "Symbol keys cannot be represented in JSON Schema")) return;
	json.type = "object";
	json.properties = {};
	for (const key in shape) assignProp(json.properties, key, processSchema(shape[key], ctx, {
		...params,
		path: [
			...params.path,
			"properties",
			key
		]
	}));
	const requiredKeys = [];
	for (const key of Object.keys(shape)) {
		const field = def.shape[key];
		if (ctx.io === "input" ? inputOptin(field) === void 0 : field._zod.optout === void 0) requiredKeys.push(key);
	}
	if (requiredKeys.length > 0) json.required = requiredKeys;
	if (def.catchall?._zod.def.type === "never") json.additionalProperties = false;
	else if (!def.catchall) {
		if (ctx.io === "output") json.additionalProperties = false;
	} else if (def.catchall) json.additionalProperties = processSchema(def.catchall, ctx, {
		...params,
		path: [...params.path, "additionalProperties"]
	});
};
var unionProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	const isExclusive = def.inclusive === false;
	const options = def.options.map((x, i) => processSchema(x, ctx, {
		...params,
		path: [
			...params.path,
			isExclusive ? "oneOf" : "anyOf",
			i
		]
	}));
	if (isExclusive) json.oneOf = options;
	else json.anyOf = options;
};
var intersectionProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	const a = processSchema(def.left, ctx, {
		...params,
		path: [
			...params.path,
			"allOf",
			0
		]
	});
	const b = processSchema(def.right, ctx, {
		...params,
		path: [
			...params.path,
			"allOf",
			1
		]
	});
	const isSimpleIntersection = (val) => "allOf" in val && Object.keys(val).length === 1;
	const allOf = [...isSimpleIntersection(a) ? a.allOf : [a], ...isSimpleIntersection(b) ? b.allOf : [b]];
	json.allOf = allOf;
	ctx.intersections.push(allOf);
};
/** JSON object keys are always strings, so a numeric record key schema is re-expressed over the
* numeric-string form the record parser matches. Deferred to `finalize`, after the flatten: a key
* behind a wrapper only carries its own `type` before then, and a union key only has its branches.
*
* A numeric bound cannot apply to a property name, so `minimum` and its siblings are dropped rather
* than carried over: keeping them beside `type: "string"` reproduces the match-nothing schema this
* exists to fix. A key that carries one therefore emits wider than the record parses — `z.record(z.number().min(5), V)`
* accepts `"3"` — which is the deliberate trade, since throwing on it would reject an ordinary schema
* outright. */
function stringifyKeyNames(bySchema, json, visited) {
	if (json.$ref) {
		if (visited.has(json)) return json;
		visited.add(json);
		const def = bySchema.get(json)?.def;
		if (!def) return json;
		const inlined = stringifyKeyNames(bySchema, def, visited);
		return inlined === def ? json : inlined;
	}
	for (const keyword of ["anyOf", "oneOf"]) {
		const branches = json[keyword];
		if (!Array.isArray(branches)) continue;
		const mapped = branches.map((branch) => stringifyKeyNames(bySchema, branch, visited));
		if (mapped.some((branch, i) => branch !== branches[i])) json = {
			...json,
			[keyword]: mapped
		};
	}
	const types = Array.isArray(json.type) ? json.type : [json.type];
	const numericType = !types.includes("string") && types.some((t) => t === "number" || t === "integer");
	const values = json.enum ?? (json.const !== void 0 ? [json.const] : void 0);
	if (!numericType && !values?.some((v) => typeof v === "number")) return json;
	const { minimum, maximum, exclusiveMinimum, exclusiveMaximum, multipleOf, format, id, ...rest } = json;
	if (rest.enum) rest.enum = rest.enum.map((v) => typeof v === "number" ? String(v) : v);
	else if (typeof rest.const === "number") rest.const = String(rest.const);
	if (!numericType) return rest;
	rest.type = "string";
	if (!values) rest.pattern = (types.includes("number") ? number$1 : integer).source;
	return rest;
}
/** Every record of one conversion, so the carriers are found in a single pass rather than once per record. */
var pendingRecords = /* @__PURE__ */ new WeakMap();
function rewriteKeyNames(ctx) {
	const bySchema = /* @__PURE__ */ new Map();
	for (const entry of ctx.seen.values()) if (entry.def && !bySchema.has(entry.schema)) bySchema.set(entry.schema, entry);
	const rewrites = /* @__PURE__ */ new Map();
	for (const record of pendingRecords.get(ctx) ?? []) {
		const seen = ctx.seen.get(record);
		const names = (seen?.def ?? seen?.schema)?.propertyNames;
		if (!names || names === true || rewrites.has(names)) continue;
		const rewritten = stringifyKeyNames(bySchema, names, /* @__PURE__ */ new Set());
		if (rewritten !== names) rewrites.set(names, rewritten);
	}
	if (!rewrites.size) return;
	for (const entry of ctx.seen.values()) for (const carrier of [entry.schema, entry.def]) {
		const rewritten = carrier && rewrites.get(carrier.propertyNames);
		if (rewritten) carrier.propertyNames = rewritten;
	}
}
var recordProcessor = (schema, ctx, _json, params) => {
	const json = _json;
	const def = schema._zod.def;
	json.type = "object";
	const keyType = def.keyType;
	const patterns = aggregateChecks(keyType).patterns;
	if (def.mode === "loose" && patterns && patterns.size > 0) {
		const valueSchema = processSchema(def.valueType, ctx, {
			...params,
			path: [
				...params.path,
				"patternProperties",
				"*"
			]
		});
		json.patternProperties = {};
		for (const pattern of patterns) assignProp(json.patternProperties, exactPattern(pattern).source, valueSchema);
	} else {
		if (ctx.target === "draft-07" || ctx.target === "draft-2020-12") {
			json.propertyNames = processSchema(def.keyType, ctx, {
				...params,
				path: [...params.path, "propertyNames"]
			});
			let pending = pendingRecords.get(ctx);
			if (!pending) {
				pending = [];
				pendingRecords.set(ctx, pending);
				ctx.deferred.push(() => rewriteKeyNames(ctx));
			}
			pending.push(schema);
		}
		json.additionalProperties = processSchema(def.valueType, ctx, {
			...params,
			path: [...params.path, "additionalProperties"]
		});
	}
	const keyValues = keyType._zod.values;
	const omittableOnInput = ctx.io === "input" && inputOptin(def.valueType) !== void 0;
	if (keyValues && !def.partial && !omittableOnInput) {
		const validKeyValues = [...keyValues].filter((v) => typeof v === "string" || typeof v === "number");
		if (validKeyValues.length > 0) json.required = validKeyValues.map(String);
	}
};
var nullableProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	const inner = processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	if (ctx.target === "openapi-3.0") {
		seen.ref = def.innerType;
		json.nullable = true;
	} else json.anyOf = [inner, { type: "null" }];
};
var nonoptionalProcessor = (schema, ctx, _json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
};
/** Round-trips a default value through JSON so the emitted schema is guaranteed to be valid JSON.
* A BigInt has no reliable encoding, so it goes through `unrepresentable` like any other
* unrepresentable value. Returns a sentinel when the caller must not write a default of its own. */
var UNREPRESENTABLE_DEFAULT = Symbol();
function serializeDefaultValue(value, schema, ctx, json, params) {
	let unrepresentable = false;
	const serialized = JSON.stringify(value, (_, val) => {
		if (typeof val !== "bigint") return val;
		unrepresentable = true;
		return null;
	});
	if (!unrepresentable) return JSON.parse(serialized);
	handleUnrepresentable(schema, ctx, json, params, "BigInt defaults cannot be represented in JSON Schema");
	return UNREPRESENTABLE_DEFAULT;
}
var defaultProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
	const value = serializeDefaultValue(def.defaultValue, schema, ctx, json, params);
	if (value !== UNREPRESENTABLE_DEFAULT) json.default = value;
};
var prefaultProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
	if (ctx.io !== "input") return;
	const value = serializeDefaultValue(def.defaultValue, schema, ctx, json, params);
	if (value !== UNREPRESENTABLE_DEFAULT) json._prefault = value;
};
var catchProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
	let catchValue;
	try {
		catchValue = def.catchValue(void 0);
	} catch {
		handleUnrepresentable(schema, ctx, json, params, "Dynamic catch values are not supported in JSON Schema");
		return;
	}
	json.default = catchValue;
};
var pipeProcessor = (schema, ctx, _json, params) => {
	const def = schema._zod.def;
	const inIsTransform = def.in._zod.traits.has("$ZodTransform");
	const innerType = ctx.io === "input" ? inIsTransform ? def.out : def.in : def.out;
	processSchema(innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = innerType;
};
var readonlyProcessor = (schema, ctx, json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
	json.readOnly = true;
};
var optionalProcessor = (schema, ctx, _json, params) => {
	const def = schema._zod.def;
	processSchema(def.innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = def.innerType;
};
var lazyProcessor = (schema, ctx, _json, params) => {
	const innerType = schema._zod.innerType;
	processSchema(innerType, ctx, params);
	const seen = ctx.seen.get(schema);
	seen.ref = innerType;
};
var _installedErrorProtos = /* @__PURE__ */ new WeakSet([Object.prototype, Error.prototype]);
function _lazyMethod(proto, key, make) {
	Object.defineProperty(proto, key, {
		configurable: true,
		enumerable: false,
		get() {
			const value = make(this);
			Object.defineProperty(this, key, {
				value,
				configurable: true,
				writable: true
			});
			return value;
		},
		set(value) {
			Object.defineProperty(this, key, {
				value,
				configurable: true,
				writable: true
			});
		}
	});
}
var initializer = (inst, issues) => {
	$ZodError.init(inst, issues);
	inst.name = "ZodError";
	const proto = Object.getPrototypeOf(inst);
	if (_installedErrorProtos.has(proto)) return;
	_installedErrorProtos.add(proto);
	_lazyMethod(proto, "format", (self) => (mapper) => formatError(self, mapper));
	_lazyMethod(proto, "flatten", (self) => (mapper) => flattenError(self, mapper));
	_lazyMethod(proto, "addIssue", (self) => (issue) => {
		self.issues.push(issue);
		self.message = JSON.stringify(self.issues, jsonStringifyReplacer, 2);
	});
	_lazyMethod(proto, "addIssues", (self) => (issues) => {
		self.issues.push(...issues);
		self.message = JSON.stringify(self.issues, jsonStringifyReplacer, 2);
	});
	Object.defineProperty(proto, "isEmpty", {
		configurable: true,
		enumerable: false,
		get() {
			return this.issues.length === 0;
		}
	});
};
var ZodRealError = /*@__PURE__*/ $constructor("ZodError", initializer, void 0, { Parent: Error });
var parse = /* @__PURE__ */ _parse(ZodRealError);
var parseAsync = /* @__PURE__ */ _parseAsync(ZodRealError);
var safeParse = /* @__PURE__ */ _safeParse(ZodRealError);
var safeParseAsync = /* @__PURE__ */ _safeParseAsync(ZodRealError);
var encode = /* @__PURE__ */ _encode(ZodRealError);
var decode = /* @__PURE__ */ _decode(ZodRealError);
var encodeAsync = /* @__PURE__ */ _encodeAsync(ZodRealError);
var decodeAsync = /* @__PURE__ */ _decodeAsync(ZodRealError);
var safeEncode = /* @__PURE__ */ _safeEncode(ZodRealError);
var safeDecode = /* @__PURE__ */ _safeDecode(ZodRealError);
var safeEncodeAsync = /* @__PURE__ */ _safeEncodeAsync(ZodRealError);
var safeDecodeAsync = /* @__PURE__ */ _safeDecodeAsync(ZodRealError);
function _ensureDefaultLocale() {
	if (!globalConfig.localeError) config(en_default());
}
function _ensureDefaultMemoizer() {
	if (!globalConfig.memoizer) config({ memoizer: memoizer() });
}
var ZodType = /*@__PURE__*/ $constructor("ZodType", (inst, def) => {
	_ensureDefaultLocale();
	$ZodType.init(inst, def);
	inst.def = def;
	inst.type = def.type;
	return inst;
}, {
	check(...chks) {
		const def = this.def;
		return this.clone(mergeDefs(def, { checks: [...def.checks ?? [], ...chks.map((ch) => typeof ch === "function" ? { _zod: {
			check: ch,
			def: { check: "custom" },
			onattach: []
		} } : ch)] }), { parent: true });
	},
	with(...chks) {
		return this.check(...chks);
	},
	clone(def, params) {
		return clone(this, def, params);
	},
	brand() {
		return this;
	},
	register(reg, meta) {
		reg.add(this, meta);
		return this;
	},
	refine(check, params) {
		return this.check(refine(check, params));
	},
	superRefine(refinement, params) {
		return this.check(superRefine(refinement, params));
	},
	overwrite(fn) {
		return this.check(/* @__PURE__ */ _overwrite(fn));
	},
	optional() {
		return optional(this);
	},
	exactOptional() {
		return exactOptional(this);
	},
	nullable() {
		return nullable(this);
	},
	nullish() {
		return optional(nullable(this));
	},
	nonoptional(params) {
		return nonoptional(this, params);
	},
	array() {
		return array(this);
	},
	or(arg) {
		return union([this, arg]);
	},
	and(arg) {
		return intersection(this, arg);
	},
	transform(tx) {
		return pipe(this, transform(tx));
	},
	default(d) {
		return _default(this, d);
	},
	prefault(d) {
		return prefault(this, d);
	},
	catch(params) {
		return _catch(this, params);
	},
	pipe(target) {
		return pipe(this, target);
	},
	readonly() {
		return readonly(this);
	},
	describe(description) {
		const cl = this.clone();
		globalRegistry.add(cl, { description });
		return cl;
	},
	meta(...args) {
		if (args.length === 0) return globalRegistry.get(this);
		const cl = this.clone();
		globalRegistry.add(cl, args[0]);
		return cl;
	},
	isOptional() {
		return this.safeParse(void 0).success;
	},
	isNullable() {
		return this.safeParse(null).success;
	},
	apply(fn, ...args) {
		return args.length === 0 ? fn(this) : fn(this, ...args);
	},
	get "~standard"() {
		return hide(this, "~standard", {
			...standardProps(this),
			jsonSchema: {
				input: createStandardJSONSchemaMethod(this, "input"),
				output: createStandardJSONSchemaMethod(this, "output")
			}
		});
	},
	set "~standard"(value) {
		own(this, "~standard", value);
	},
	parse: function _parse(data, params) {
		return parse(this, data, params, { callee: _parse });
	},
	parseAsync: async function _parseAsync(data, params) {
		return await parseAsync(this, data, params, { callee: _parseAsync });
	},
	safeParse(data, params) {
		return safeParse(this, data, params);
	},
	async safeParseAsync(data, params) {
		return safeParseAsync(this, data, params);
	},
	get spa() {
		return this?.safeParseAsync;
	},
	set spa(value) {
		own(this, "spa", value);
	},
	validate(data, params) {
		return validate$1(this, data, params);
	},
	validateAsync(data, params) {
		return validateAsync$1(this, data, params);
	},
	encode: function _encode(data, params) {
		return encode(this, data, params, { callee: _encode });
	},
	decode: function _decode(data, params) {
		return decode(this, data, params, { callee: _decode });
	},
	encodeAsync: async function _encodeAsync(data, params) {
		return await encodeAsync(this, data, params, { callee: _encodeAsync });
	},
	decodeAsync: async function _decodeAsync(data, params) {
		return await decodeAsync(this, data, params, { callee: _decodeAsync });
	},
	safeEncode(data, params) {
		return safeEncode(this, data, params);
	},
	safeDecode(data, params) {
		return safeDecode(this, data, params);
	},
	async safeEncodeAsync(data, params) {
		return safeEncodeAsync(this, data, params);
	},
	async safeDecodeAsync(data, params) {
		return safeDecodeAsync(this, data, params);
	},
	toJSONSchema(params) {
		return createToJSONSchemaMethod(this, {})(params);
	},
	get description() {
		return globalRegistry.get(this)?.description;
	},
	get _def() {
		return this._zod.def;
	}
});
/** @internal */
var _ZodString = /*@__PURE__*/ $constructor("_ZodString", (inst, def) => {
	$ZodString.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => stringProcessor(inst, ctx, json, params);
}, /*@__PURE__*/ derived({
	format: (inst) => aggregateChecks(inst).format ?? null,
	minLength: (inst) => aggregateChecks(inst).minimum ?? null,
	maxLength: (inst) => aggregateChecks(inst).maximum ?? null
}, {
	regex(...args) {
		return this.check(/* @__PURE__ */ _regex(...args));
	},
	includes(...args) {
		return this.check(/* @__PURE__ */ _includes(...args));
	},
	startsWith(...args) {
		return this.check(/* @__PURE__ */ _startsWith(...args));
	},
	endsWith(...args) {
		return this.check(/* @__PURE__ */ _endsWith(...args));
	},
	min(...args) {
		return this.check(/* @__PURE__ */ _minLength(...args));
	},
	max(...args) {
		return this.check(/* @__PURE__ */ _maxLength(...args));
	},
	length(...args) {
		return this.check(/* @__PURE__ */ _length(...args));
	},
	nonempty(...args) {
		return this.check(/* @__PURE__ */ _minLength(1, ...args));
	},
	lowercase(params) {
		return this.check(/* @__PURE__ */ _lowercase(params));
	},
	uppercase(params) {
		return this.check(/* @__PURE__ */ _uppercase(params));
	},
	trim() {
		return this.check(/* @__PURE__ */ _trim());
	},
	normalize(...args) {
		return this.check(/* @__PURE__ */ _normalize(...args));
	},
	toLowerCase() {
		return this.check(/* @__PURE__ */ _toLowerCase());
	},
	toUpperCase() {
		return this.check(/* @__PURE__ */ _toUpperCase());
	},
	slugify() {
		return this.check(/* @__PURE__ */ _slugify());
	}
}));
var ZodString = /*@__PURE__*/ $constructor("ZodString", (inst, def) => {
	$ZodString.init(inst, def);
	_ZodString.init(inst, def);
}, {
	email(params) {
		return this.check(/* @__PURE__ */ _email(ZodEmail, params));
	},
	url(params) {
		return this.check(/* @__PURE__ */ _url(ZodURL, params));
	},
	jwt(params) {
		return this.check(/* @__PURE__ */ _jwt(ZodJWT, params));
	},
	emoji(params) {
		return this.check(/* @__PURE__ */ _emoji(ZodEmoji, params));
	},
	guid(params) {
		return this.check(/* @__PURE__ */ _guid(ZodGUID, params));
	},
	uuid(params) {
		return this.check(/* @__PURE__ */ _uuid(ZodUUID, params));
	},
	uuidv4(params) {
		return this.check(/* @__PURE__ */ _uuidv4(ZodUUID, params));
	},
	uuidv6(params) {
		return this.check(/* @__PURE__ */ _uuidv6(ZodUUID, params));
	},
	uuidv7(params) {
		return this.check(/* @__PURE__ */ _uuidv7(ZodUUID, params));
	},
	nanoid(params) {
		return this.check(/* @__PURE__ */ _nanoid(ZodNanoID, params));
	},
	cuid(params) {
		return this.check(/* @__PURE__ */ _cuid(ZodCUID, params));
	},
	cuid2(params) {
		return this.check(/* @__PURE__ */ _cuid2(ZodCUID2, params));
	},
	ulid(params) {
		return this.check(/* @__PURE__ */ _ulid(ZodULID, params));
	},
	base64(params) {
		return this.check(/* @__PURE__ */ _base64(ZodBase64, params));
	},
	base64url(params) {
		return this.check(/* @__PURE__ */ _base64url(ZodBase64URL, params));
	},
	xid(params) {
		return this.check(/* @__PURE__ */ _xid(ZodXID, params));
	},
	ksuid(params) {
		return this.check(/* @__PURE__ */ _ksuid(ZodKSUID, params));
	},
	ipv4(params) {
		return this.check(/* @__PURE__ */ _ipv4(ZodIPv4, params));
	},
	ipv6(params) {
		return this.check(/* @__PURE__ */ _ipv6(ZodIPv6, params));
	},
	cidrv4(params) {
		return this.check(/* @__PURE__ */ _cidrv4(ZodCIDRv4, params));
	},
	cidrv6(params) {
		return this.check(/* @__PURE__ */ _cidrv6(ZodCIDRv6, params));
	},
	e164(params) {
		return this.check(/* @__PURE__ */ _e164(ZodE164, params));
	},
	datetime(params) {
		return this.check(/* @__PURE__ */ _isoDateTime(ZodISODateTime, params));
	},
	date(params) {
		return this.check(/* @__PURE__ */ _isoDate(ZodISODate, params));
	},
	time(params) {
		return this.check(/* @__PURE__ */ _isoTime(ZodISOTime, params));
	},
	duration(params) {
		return this.check(/* @__PURE__ */ _isoDuration(ZodISODuration, params));
	}
});
function string(params) {
	return /* @__PURE__ */ _string(ZodString, params);
}
var ZodStringFormat = /*@__PURE__*/ $constructor("ZodStringFormat", (inst, def) => {
	$ZodStringFormat.init(inst, def);
	_ZodString.init(inst, def);
});
var ZodISODateTime = /*@__PURE__*/ $constructor("ZodISODateTime", (inst, def) => {
	$ZodISODateTime.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodISODate = /*@__PURE__*/ $constructor("ZodISODate", (inst, def) => {
	$ZodISODate.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodISOTime = /*@__PURE__*/ $constructor("ZodISOTime", (inst, def) => {
	$ZodISOTime.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodISODuration = /*@__PURE__*/ $constructor("ZodISODuration", (inst, def) => {
	$ZodISODuration.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodEmail = /*@__PURE__*/ $constructor("ZodEmail", (inst, def) => {
	$ZodEmail.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodGUID = /*@__PURE__*/ $constructor("ZodGUID", (inst, def) => {
	$ZodGUID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodUUID = /*@__PURE__*/ $constructor("ZodUUID", (inst, def) => {
	$ZodUUID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodURL = /*@__PURE__*/ $constructor("ZodURL", (inst, def) => {
	$ZodURL.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodEmoji = /*@__PURE__*/ $constructor("ZodEmoji", (inst, def) => {
	$ZodEmoji.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodNanoID = /*@__PURE__*/ $constructor("ZodNanoID", (inst, def) => {
	$ZodNanoID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
/**
* @deprecated CUID v1 is deprecated by its authors due to information leakage
* (timestamps embedded in the id). Use {@link ZodCUID2} instead.
* See https://github.com/paralleldrive/cuid.
*/
var ZodCUID = /*@__PURE__*/ $constructor("ZodCUID", (inst, def) => {
	$ZodCUID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodCUID2 = /*@__PURE__*/ $constructor("ZodCUID2", (inst, def) => {
	$ZodCUID2.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodULID = /*@__PURE__*/ $constructor("ZodULID", (inst, def) => {
	$ZodULID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodXID = /*@__PURE__*/ $constructor("ZodXID", (inst, def) => {
	$ZodXID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodKSUID = /*@__PURE__*/ $constructor("ZodKSUID", (inst, def) => {
	$ZodKSUID.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodIPv4 = /*@__PURE__*/ $constructor("ZodIPv4", (inst, def) => {
	$ZodIPv4.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodIPv6 = /*@__PURE__*/ $constructor("ZodIPv6", (inst, def) => {
	$ZodIPv6.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodCIDRv4 = /*@__PURE__*/ $constructor("ZodCIDRv4", (inst, def) => {
	$ZodCIDRv4.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodCIDRv6 = /*@__PURE__*/ $constructor("ZodCIDRv6", (inst, def) => {
	$ZodCIDRv6.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodBase64 = /*@__PURE__*/ $constructor("ZodBase64", (inst, def) => {
	$ZodBase64.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodBase64URL = /*@__PURE__*/ $constructor("ZodBase64URL", (inst, def) => {
	$ZodBase64URL.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodE164 = /*@__PURE__*/ $constructor("ZodE164", (inst, def) => {
	$ZodE164.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodJWT = /*@__PURE__*/ $constructor("ZodJWT", (inst, def) => {
	$ZodJWT.init(inst, def);
	ZodStringFormat.init(inst, def);
});
var ZodNumber = /*@__PURE__*/ $constructor("ZodNumber", (inst, def) => {
	$ZodNumber.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => numberProcessor(inst, ctx, json, params);
	inst.isFinite = true;
}, /*@__PURE__*/ derived({
	minValue: (inst) => {
		const { minimum, exclusiveMinimum } = aggregateChecks(inst);
		return Math.max(minimum ?? Number.NEGATIVE_INFINITY, exclusiveMinimum ?? Number.NEGATIVE_INFINITY);
	},
	maxValue: (inst) => {
		const { maximum, exclusiveMaximum } = aggregateChecks(inst);
		return Math.min(maximum ?? Number.POSITIVE_INFINITY, exclusiveMaximum ?? Number.POSITIVE_INFINITY);
	},
	isInt: (inst) => {
		const { isInt, multipleOf } = aggregateChecks(inst);
		return !!isInt || !!multipleOf?.some(Number.isSafeInteger);
	},
	format: (inst) => aggregateChecks(inst).format ?? null
}, {
	gt(value, params) {
		return this.check(/* @__PURE__ */ _gt(value, params));
	},
	gte(value, params) {
		return this.check(/* @__PURE__ */ _gte(value, params));
	},
	min(value, params) {
		return this.check(/* @__PURE__ */ _gte(value, params));
	},
	lt(value, params) {
		return this.check(/* @__PURE__ */ _lt(value, params));
	},
	lte(value, params) {
		return this.check(/* @__PURE__ */ _lte(value, params));
	},
	max(value, params) {
		return this.check(/* @__PURE__ */ _lte(value, params));
	},
	int(params) {
		return this.check(int(params));
	},
	safe(params) {
		return this.check(int(params));
	},
	positive(params) {
		return this.check(/* @__PURE__ */ _gt(0, params));
	},
	nonnegative(params) {
		return this.check(/* @__PURE__ */ _gte(0, params));
	},
	negative(params) {
		return this.check(/* @__PURE__ */ _lt(0, params));
	},
	nonpositive(params) {
		return this.check(/* @__PURE__ */ _lte(0, params));
	},
	multipleOf(value, params) {
		return this.check(/* @__PURE__ */ _multipleOf(value, params));
	},
	step(value, params) {
		return this.check(/* @__PURE__ */ _multipleOf(value, params));
	},
	finite() {
		return this;
	}
}));
function number(params) {
	return /* @__PURE__ */ _number(ZodNumber, params);
}
var ZodNumberFormat = /*@__PURE__*/ $constructor("ZodNumberFormat", (inst, def) => {
	$ZodNumberFormat.init(inst, def);
	ZodNumber.init(inst, def);
});
function int(params) {
	return /* @__PURE__ */ _int(ZodNumberFormat, params);
}
var ZodBoolean = /*@__PURE__*/ $constructor("ZodBoolean", (inst, def) => {
	$ZodBoolean.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => booleanProcessor(inst, ctx, json, params);
});
function boolean(params) {
	return /* @__PURE__ */ _boolean(ZodBoolean, params);
}
var ZodNull = /*@__PURE__*/ $constructor("ZodNull", (inst, def) => {
	$ZodNull.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => nullProcessor(inst, ctx, json, params);
});
function _null(params) {
	return /* @__PURE__ */ _null$1(ZodNull, params);
}
var ZodUnknown = /*@__PURE__*/ $constructor("ZodUnknown", (inst, def) => {
	$ZodUnknown.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => void 0;
});
function unknown() {
	return /* @__PURE__ */ _unknown(ZodUnknown);
}
var ZodNever = /*@__PURE__*/ $constructor("ZodNever", (inst, def) => {
	$ZodNever.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => neverProcessor(inst, ctx, json, params);
});
function never(params) {
	return /* @__PURE__ */ _never(ZodNever, params);
}
var ZodArray = /*@__PURE__*/ $constructor("ZodArray", (inst, def) => {
	_ensureDefaultMemoizer();
	$ZodArray.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => arrayProcessor(inst, ctx, json, params);
	inst.element = def.element;
}, {
	min(n, params) {
		return this.check(/* @__PURE__ */ _minLength(n, params));
	},
	nonempty(params) {
		return this.check(/* @__PURE__ */ _minLength(1, params));
	},
	max(n, params) {
		return this.check(/* @__PURE__ */ _maxLength(n, params));
	},
	length(n, params) {
		return this.check(/* @__PURE__ */ _length(n, params));
	},
	unwrap() {
		return this.element;
	}
});
function array(element, params) {
	return /* @__PURE__ */ _array(ZodArray, element, params);
}
var ZodObject = /*@__PURE__*/ $constructor("ZodObject", (inst, def) => {
	_ensureDefaultMemoizer();
	$ZodObjectJIT.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => objectProcessor(inst, ctx, json, params);
	installLazyProp(inst, "shape", (self) => self._zod.def.shape, false);
}, {
	keyof() {
		return _enum(Object.keys(this._zod.def.shape));
	},
	catchall(catchall) {
		return this.clone(mergeDefs(this._zod.def, { catchall }));
	},
	passthrough() {
		return this.clone(mergeDefs(this._zod.def, { catchall: unknown() }));
	},
	loose() {
		return this.clone(mergeDefs(this._zod.def, { catchall: unknown() }));
	},
	strict() {
		return this.clone(mergeDefs(this._zod.def, { catchall: never() }));
	},
	strip() {
		return this.clone(mergeDefs(this._zod.def, { catchall: void 0 }));
	},
	extend(incoming) {
		return extend(this, incoming);
	},
	safeExtend(incoming) {
		return safeExtend(this, incoming);
	},
	merge(other) {
		return merge(this, other);
	},
	pick(mask) {
		return pick(this, mask);
	},
	omit(mask) {
		return omit(this, mask);
	},
	partial(...args) {
		return partial(ZodOptional, this, args[0]);
	},
	exactPartial(...args) {
		return partial(ZodExactOptional, this, args[0], "exactPartial");
	},
	required(...args) {
		return required(ZodNonOptional, this, args[0]);
	}
});
function strictObject(shape, params) {
	return new ZodObject({
		type: "object",
		shape,
		catchall: never(),
		...normalizeParams(params)
	});
}
var ZodUnion = /*@__PURE__*/ $constructor("ZodUnion", (inst, def) => {
	$ZodUnion.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => unionProcessor(inst, ctx, json, params);
	inst.options = def.options;
});
function union(options, params) {
	return new ZodUnion({
		type: "union",
		options,
		...normalizeParams(params)
	});
}
var ZodIntersection = /*@__PURE__*/ $constructor("ZodIntersection", (inst, def) => {
	$ZodIntersection.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => intersectionProcessor(inst, ctx, json, params);
});
function intersection(left, right) {
	return new ZodIntersection({
		type: "intersection",
		left,
		right
	});
}
var ZodRecord = /*@__PURE__*/ $constructor("ZodRecord", (inst, def) => {
	_ensureDefaultMemoizer();
	$ZodRecord.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => recordProcessor(inst, ctx, json, params);
	inst.keyType = def.keyType;
	inst.valueType = def.valueType;
});
function record(keyType, valueType, params) {
	if (!valueType || !valueType._zod) return new ZodRecord({
		type: "record",
		keyType: string(),
		valueType: keyType,
		...normalizeParams(valueType)
	});
	return new ZodRecord({
		type: "record",
		keyType,
		valueType,
		...normalizeParams(params)
	});
}
var ZodEnum = /*@__PURE__*/ $constructor("ZodEnum", (inst, def) => {
	$ZodEnum.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => enumProcessor(inst, ctx, json, params);
	inst.enum = def.entries;
	inst.options = [...inst._zod.values];
	const keys = new Set(Object.keys(def.entries));
	inst.extract = (values, params) => {
		const newEntries = {};
		for (const value of values) if (keys.has(value)) newEntries[value] = def.entries[value];
		else throw new Error(`Key ${value} not found in enum`);
		return new ZodEnum({
			...def,
			checks: [],
			...normalizeParams(params),
			entries: newEntries
		});
	};
	inst.exclude = (values, params) => {
		const newEntries = { ...def.entries };
		for (const value of values) if (keys.has(value)) delete newEntries[value];
		else throw new Error(`Key ${value} not found in enum`);
		return new ZodEnum({
			...def,
			checks: [],
			...normalizeParams(params),
			entries: newEntries
		});
	};
});
function _enum(values, params) {
	return new ZodEnum({
		type: "enum",
		entries: Array.isArray(values) ? Object.fromEntries(values.map((v) => [v, v])) : values,
		...normalizeParams(params)
	});
}
var ZodLiteral = /*@__PURE__*/ $constructor("ZodLiteral", (inst, def) => {
	$ZodLiteral.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => literalProcessor(inst, ctx, json, params);
	inst.values = new Set(def.values);
	Object.defineProperty(inst, "value", { get() {
		if (def.values.length > 1) throw new Error("This schema contains multiple valid literal values. Use `.values` instead.");
		return def.values[0];
	} });
});
function literal(value, params) {
	return new ZodLiteral({
		type: "literal",
		values: Array.isArray(value) ? value : [value],
		...normalizeParams(params)
	});
}
var ZodTransform = /*@__PURE__*/ $constructor("ZodTransform", (inst, def) => {
	_ensureDefaultMemoizer();
	$ZodTransform.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => transformProcessor(inst, ctx, json, params);
	inst._zod.parse = (payload, _ctx) => {
		if (_ctx.direction === "backward") throw new $ZodEncodeError(inst.constructor.name);
		payload.addIssue = (issue$1) => {
			if (typeof issue$1 === "string") payload.issues.push(issue(issue$1, payload.value, def));
			else {
				const _issue = issue$1;
				if (_issue.fatal) _issue.continue = false;
				_issue.code ?? (_issue.code = "custom");
				if (!("input" in _issue)) _issue.input = payload.value;
				_issue.inst ?? (_issue.inst = inst);
				payload.issues.push(issue(_issue));
			}
		};
		const output = def.transform(payload.value, payload);
		if (output instanceof Promise) return output.then((output) => {
			payload.value = output;
			return payload;
		});
		payload.value = output;
		return payload;
	};
});
function transform(fn) {
	return new ZodTransform({
		type: "transform",
		transform: fn
	});
}
var ZodOptional = /*@__PURE__*/ $constructor("ZodOptional", (inst, def) => {
	$ZodOptional.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => optionalProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function optional(innerType) {
	return new ZodOptional({
		type: "optional",
		innerType
	});
}
var ZodExactOptional = /*@__PURE__*/ $constructor("ZodExactOptional", (inst, def) => {
	$ZodExactOptional.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => optionalProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function exactOptional(innerType) {
	return new ZodExactOptional({
		type: "optional",
		innerType
	});
}
var ZodNullable = /*@__PURE__*/ $constructor("ZodNullable", (inst, def) => {
	$ZodNullable.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => nullableProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function nullable(innerType) {
	return new ZodNullable({
		type: "nullable",
		innerType
	});
}
var ZodDefault = /*@__PURE__*/ $constructor("ZodDefault", (inst, def) => {
	$ZodDefault.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => defaultProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
	inst.removeDefault = inst.unwrap;
});
function _default(innerType, defaultValue) {
	return new ZodDefault({
		type: "default",
		innerType,
		get defaultValue() {
			return typeof defaultValue === "function" ? defaultValue() : shallowClone(defaultValue);
		}
	});
}
var ZodPrefault = /*@__PURE__*/ $constructor("ZodPrefault", (inst, def) => {
	$ZodPrefault.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => prefaultProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function prefault(innerType, defaultValue) {
	return new ZodPrefault({
		type: "prefault",
		innerType,
		get defaultValue() {
			return typeof defaultValue === "function" ? defaultValue() : shallowClone(defaultValue);
		}
	});
}
var ZodNonOptional = /*@__PURE__*/ $constructor("ZodNonOptional", (inst, def) => {
	$ZodNonOptional.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => nonoptionalProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function nonoptional(innerType, params) {
	return new ZodNonOptional({
		type: "nonoptional",
		innerType,
		...normalizeParams(params)
	});
}
var ZodCatch = /*@__PURE__*/ $constructor("ZodCatch", (inst, def) => {
	$ZodCatch.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => catchProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
	inst.removeCatch = inst.unwrap;
});
function _catch(innerType, catchValue) {
	return new ZodCatch({
		type: "catch",
		innerType,
		catchValue: typeof catchValue === "function" ? catchValue : constantCatch(catchValue)
	});
}
var ZodPipe = /*@__PURE__*/ $constructor("ZodPipe", (inst, def) => {
	$ZodPipe.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => pipeProcessor(inst, ctx, json, params);
	inst.in = def.in;
	inst.out = def.out;
});
function pipe(in_, out) {
	return new ZodPipe({
		type: "pipe",
		in: in_,
		out
	});
}
var ZodReadonly = /*@__PURE__*/ $constructor("ZodReadonly", (inst, def) => {
	$ZodReadonly.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => readonlyProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.innerType;
});
function readonly(innerType) {
	return new ZodReadonly({
		type: "readonly",
		innerType
	});
}
var ZodLazy = /*@__PURE__*/ $constructor("ZodLazy", (inst, def) => {
	$ZodLazy.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => lazyProcessor(inst, ctx, json, params);
	inst.unwrap = () => inst._zod.def.getter();
});
function lazy$1(getter) {
	return new ZodLazy({
		type: "lazy",
		getter
	});
}
var ZodCustom = /*@__PURE__*/ $constructor("ZodCustom", (inst, def) => {
	$ZodCustom.init(inst, def);
	ZodType.init(inst, def);
	inst._zod.processJSONSchema = (ctx, json, params) => customProcessor(inst, ctx, json, params);
});
function refine(fn, _params = {}) {
	return /* @__PURE__ */ _refine(ZodCustom, fn, _params);
}
function superRefine(fn, params) {
	return /* @__PURE__ */ _superRefine(fn, params);
}
function json(params) {
	const jsonSchema = lazy$1(() => {
		return union([
			string(params),
			number(),
			boolean(),
			_null(),
			array(jsonSchema),
			record(string(), jsonSchema)
		]);
	});
	return jsonSchema;
}
/** Zod's built-in guard for string and array length checks. */
var LENGTH_CHECK = (/* @__PURE__ */ _minLength(0))._zod.def.when;
/** Descriptive metadata that cannot replace exported validation rules. */
var METADATA_KEYS = /* @__PURE__ */ new Set([
	"id",
	"title",
	"description",
	"deprecated",
	"examples"
]);
/** Require declarative schemas for JSON-compatible values. */
function validate(schema, visited, isProperty) {
	const definition = schema._zod.def;
	if (definition.type === "optional" && !isProperty) throw new TypeError("optional schemas are only supported as object properties");
	const contexts = visited.get(schema);
	if (contexts?.has(isProperty)) return;
	if (contexts) contexts.add(isProperty);
	else visited.set(schema, /* @__PURE__ */ new Set([isProperty]));
	const metadata = globalRegistry.get(schema);
	for (const key of Object.keys(metadata ?? {})) if (!METADATA_KEYS.has(key)) throw new TypeError(`unsupported schema metadata: ${key}`);
	if (metadata !== void 0) json().parse(metadata);
	if ("coerce" in definition && definition.coerce) throw new TypeError("declared schemas cannot coerce values");
	const checks = [...definition.checks ?? []];
	if ("check" in definition) checks.push(schema);
	for (const check of checks) {
		const rule = check._zod.def;
		if (rule.when !== void 0 && rule.when !== LENGTH_CHECK) throw new TypeError("declared schemas cannot use conditional checks");
		switch (rule.check) {
			case "less_than":
			case "greater_than":
			case "multiple_of":
			case "min_length":
			case "max_length":
			case "length_equals":
			case "number_format":
			case "string_format": break;
			default: throw new TypeError(`unsupported schema check: ${rule.check}`);
		}
		if ("pattern" in rule && rule.pattern instanceof RegExp && (rule.pattern.global || rule.pattern.sticky)) throw new TypeError("declared regular expressions cannot use global or sticky flags");
		if (rule.check === "string_format") {
			const format = rule.format;
			if (![
				"regex",
				"uuid",
				"guid",
				"nanoid",
				"cuid2",
				"ulid",
				"xid",
				"ksuid",
				"email",
				"url",
				"emoji",
				"hostname",
				"hex",
				"currency_code",
				"jwt",
				"credit_card",
				"iban",
				"ipv4",
				"ipv6",
				"mac",
				"base64",
				"base64url",
				"e164",
				"cidrv4",
				"cidrv6",
				"datetime",
				"date",
				"time",
				"duration"
			].includes(format) && !/^(?:md5|sha1|sha256|sha384|sha512)_(?:hex|base64|base64url)$/.test(format)) throw new TypeError(`unsupported string format: ${format}`);
		}
		if ("normalize" in rule && rule.normalize) throw new TypeError("declared schemas cannot request URL normalization");
		if ("fn" in rule && !(rule.check === "string_format" && "pattern" in rule)) throw new TypeError("declared schemas cannot use custom validation functions");
	}
	switch (definition.type) {
		case "string":
		case "number":
		case "boolean":
		case "null":
		case "enum":
		case "never": break;
		case "literal":
			for (const value of definition.values) json().parse(value);
			break;
		case "template_literal":
			for (const part of definition.parts) if (typeof part === "object" && part !== null) validate(part, visited, false);
			break;
		case "object":
			if (definition.catchall?._zod.def.type !== "never") throw new TypeError("declared object schemas must reject unknown properties");
			for (const property of Object.values(definition.shape)) validate(property, visited, true);
			break;
		case "array":
			validate(definition.element, visited, false);
			break;
		case "tuple":
			for (const item of definition.items) validate(item, visited, false);
			if (definition.rest) validate(definition.rest, visited, false);
			break;
		case "record":
			validate(definition.keyType, visited, false);
			validate(definition.valueType, visited, false);
			break;
		case "intersection":
			validate(definition.left, visited, isProperty);
			validate(definition.right, visited, isProperty);
			break;
		case "union":
			for (const option of definition.options) validate(option, visited, isProperty);
			break;
		case "nullable":
			validate(definition.innerType, visited, isProperty);
			break;
		case "optional":
			validate(definition.innerType, visited, isProperty);
			break;
		case "lazy":
			validate(definition.getter(), visited, isProperty);
			break;
		default: throw new TypeError(`unsupported schema type: ${definition.type}`);
	}
}
/** Check the supported declaration and retain native validation and inference. */
function defineSchema(schema) {
	validate(schema, /* @__PURE__ */ new Map(), false);
	return schema;
}
/** The canonical lowercase UUIDv7 representation from RFC 9562. */
var UUID_V7 = "[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}";
/** Define a typed entity identifier with a lowercase prefix and a UUIDv7 suffix. */
function identifier(prefix) {
	if (!/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/.test(prefix)) throw new TypeError(`Invalid identifier prefix: ${prefix}`);
	const pattern = new RegExp(`^${prefix}-${UUID_V7}$(?![\\s\\S])`);
	const validator = string().regex(pattern).brand();
	defineSchema(validator);
	return validator;
}
/** A declaration name within a package. */
var ResourceName = defineSchema(string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/));
/** A named infrastructure dependency declared by a package. */
var ResourceDeclaration = defineSchema(strictObject({
	/** The package-local resource name. */
	name: ResourceName,
	/** The resource kind defined by its domain library. */
	kind: ResourceName,
	/** The declaration format version. */
	version: number().int().positive(),
	/** The specification validated by the domain library. */
	spec: record(string(), json())
}));
/** Define a resource declaration with a concrete specification. */
function defineResourceSchema(kind, version, spec) {
	ResourceDeclaration.pick({
		kind: true,
		version: true
	}).parse({
		kind,
		version
	});
	return defineSchema(ResourceDeclaration.extend({
		kind: literal(kind),
		version: literal(version),
		spec
	}));
}
/** An inert declaration with access to a host-bound client. */
var Resource = class {
	/** The declaration name. */
	name;
	/** The resource kind. */
	kind;
	/** The declaration format version. */
	version;
	/** The domain specification. */
	spec;
	/** Retain validated metadata without opening a resource. */
	constructor(declaration) {
		this.name = declaration.name;
		this.kind = declaration.kind;
		this.version = declaration.version;
		this.spec = declaration.spec;
	}
	/** Get the resource client bound to the current operation. */
	get(context) {
		return context.get(this);
	}
};
/** A named database dependency. */
var DatabaseDeclaration = defineResourceSchema("database", 1, defineSchema(strictObject({ 
/** The dialect used by queries and migrations. */
dialect: defineSchema(_enum(["sqlite", "postgresql"])) })));
/** An inert database declaration with invocation-scoped connection access. */
var Database = class extends Resource {
	/** Retrieve the authorized connection with source-inferred schema queries. */
	get(context, schema) {
		const connection = context.get(this);
		if (connection.connection.native.dialect !== this.spec.dialect) throw new TypeError(`Database ${this.name} requires ${this.spec.dialect}, received ${connection.connection.native.dialect}.`);
		return schema ? connection.bind(schema) : connection;
	}
};
/** Declare a database dependency. */
function defineDatabase(declaration) {
	return new Database(DatabaseDeclaration.parse({
		...declaration,
		kind: "database",
		version: 1
	}));
}
/** A vault resource dependency. */
var VaultDeclaration = defineResourceSchema("vault", 1, defineSchema(strictObject({})));
/** Declare a vault resource. */
function defineVault(declaration) {
	return VaultDeclaration.parse({
		...declaration,
		kind: "vault",
		version: 1
	});
}
/** A secret selected when installing a package. */
var SecretDeclaration = defineSchema(strictObject({
	/** The package-local secret name. */
	name: ResourceName,
	/** The declaration format version. */
	version: literal(1)
}));
defineSchema(strictObject({
	/** The space administering the vault. */
	space: identifier("space"),
	/** The secret identifier. */
	secret: identifier("secret"),
	/** An exact version; omit to select the current version at access time. */
	version: number().int().positive().optional()
}));
/** Declare a secret without embedding its value. */
function defineSecret(declaration) {
	return SecretDeclaration.parse({
		...declaration,
		version: 1
	});
}
/** Fields shared by calendar, interval, and one-off schedules. */
var SCHEDULE = strictObject({
	/** The package-local schedule name. */
	name: ResourceName,
	/** The declaration format version. */
	version: literal(1),
	/** The exported handler receiving the scheduled occurrence. */
	handler: string().min(1),
	/** Whether occurrences may overlap. */
	concurrency: _enum([
		"allow",
		"forbid",
		"replace"
	]),
	/** How late an occurrence may start, in milliseconds. */
	deadline: number().int().nonnegative()
});
/** A controller-managed schedule invoking an exported workload handler. */
var ScheduleDeclaration = defineSchema(union([
	SCHEDULE.extend({
		/** Evaluate calendar occurrences in the selected time zone. */
		timing: literal("cron"),
		/** A five-field cron expression. */
		cron: string().regex(/^\S+\s+\S+\s+\S+\s+\S+\s+\S+$/),
		/** The IANA time zone used to evaluate occurrences. */
		timezone: string().min(1),
		/** The earliest occurrence time in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative().optional(),
		/** The exclusive end time in UTC epoch milliseconds. */
		endsAt: number().int().nonnegative().optional()
	}),
	SCHEDULE.extend({
		/** Repeat at a fixed interval from the first occurrence. */
		timing: literal("interval"),
		/** The interval in milliseconds. */
		interval: number().int().positive(),
		/** The first occurrence time in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative(),
		/** The exclusive end time in UTC epoch milliseconds. */
		endsAt: number().int().nonnegative().optional()
	}),
	SCHEDULE.extend({
		/** Invoke the handler once at the selected time. */
		timing: literal("once"),
		/** The occurrence time in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative()
	})
]));
/** Declare a schedule for build-time validation. */
function defineSchedule(value) {
	return ScheduleDeclaration.parse(value);
}
function resolveMaybeOptionalOptions(rest) {
	return rest[0] ?? {};
}
function toArray(value) {
	return Array.isArray(value) ? value : value === void 0 || value === null ? [] : [value];
}
var ORPC_NAME = "orpc";
var ORPC_SHARED_PACKAGE_NAME = "@orpc/shared";
var ORPC_SHARED_PACKAGE_VERSION = "1.15.1";
var AbortError = class extends Error {
	constructor(...rest) {
		super(...rest);
		this.name = "AbortError";
	}
};
function once(fn) {
	let cached;
	return () => {
		if (cached) return cached.result;
		const result = fn();
		cached = { result };
		return result;
	};
}
function sequential(fn) {
	let lastOperationPromise = Promise.resolve();
	return (...args) => {
		return lastOperationPromise = lastOperationPromise.catch(() => {}).then(() => {
			return fn(...args);
		});
	};
}
var SPAN_ERROR_STATUS = 2;
var GLOBAL_OTEL_CONFIG_KEY = `__${ORPC_SHARED_PACKAGE_NAME}@${ORPC_SHARED_PACKAGE_VERSION}/otel/config__`;
function setGlobalOtelConfig(config) {
	globalThis[GLOBAL_OTEL_CONFIG_KEY] = config;
}
function getGlobalOtelConfig() {
	return globalThis[GLOBAL_OTEL_CONFIG_KEY];
}
function startSpan(name, options = {}, context) {
	return (getGlobalOtelConfig()?.tracer)?.startSpan(name, options, context);
}
function setSpanError(span, error, options = {}) {
	if (!span) return;
	const exception = toOtelException(error);
	span.recordException(exception);
	if (!options.signal?.aborted || options.signal.reason !== error) span.setStatus({
		code: SPAN_ERROR_STATUS,
		message: exception.message
	});
}
function toOtelException(error) {
	if (error instanceof Error) {
		const exception = {
			message: error.message,
			name: error.name,
			stack: error.stack
		};
		if ("code" in error && (typeof error.code === "string" || typeof error.code === "number")) exception.code = error.code;
		return exception;
	}
	return { message: String(error) };
}
async function runWithSpan({ name, context, ...options }, fn) {
	const tracer = getGlobalOtelConfig()?.tracer;
	if (!tracer) return fn();
	const callback = async (span) => {
		try {
			return await fn(span);
		} catch (e) {
			setSpanError(span, e, options);
			throw e;
		} finally {
			span.end();
		}
	};
	if (context) return tracer.startActiveSpan(name, options, context, callback);
	else return tracer.startActiveSpan(name, options, callback);
}
async function runInSpanContext(span, fn) {
	const otelConfig = getGlobalOtelConfig();
	if (!span || !otelConfig) return fn();
	const ctx = otelConfig.trace.setSpan(otelConfig.context.active(), span);
	return otelConfig.context.with(ctx, fn);
}
function isAsyncIteratorObject(maybe) {
	if (!maybe || typeof maybe !== "object") return false;
	return "next" in maybe && typeof maybe.next === "function" && Symbol.asyncIterator in maybe && typeof maybe[Symbol.asyncIterator] === "function";
}
var asyncDisposeSymbol = Symbol.asyncDispose ?? Symbol.for("asyncDispose");
var AsyncIteratorClass = class {
	#isDone = false;
	#isExecuteComplete = false;
	#cleanup;
	#next;
	constructor(next, cleanup) {
		this.#cleanup = cleanup;
		this.#next = sequential(async () => {
			if (this.#isDone) return {
				done: true,
				value: void 0
			};
			try {
				const result = await next();
				if (result.done) this.#isDone = true;
				return result;
			} catch (err) {
				this.#isDone = true;
				throw err;
			} finally {
				if (this.#isDone && !this.#isExecuteComplete) {
					this.#isExecuteComplete = true;
					await this.#cleanup("next");
				}
			}
		});
	}
	next() {
		return this.#next();
	}
	async return(value) {
		this.#isDone = true;
		if (!this.#isExecuteComplete) {
			this.#isExecuteComplete = true;
			await this.#cleanup("return");
		}
		return {
			done: true,
			value
		};
	}
	async throw(err) {
		this.#isDone = true;
		if (!this.#isExecuteComplete) {
			this.#isExecuteComplete = true;
			await this.#cleanup("throw");
		}
		throw err;
	}
	/**
	* asyncDispose symbol only available in esnext, we should fallback to Symbol.for('asyncDispose')
	*/
	async [asyncDisposeSymbol]() {
		this.#isDone = true;
		if (!this.#isExecuteComplete) {
			this.#isExecuteComplete = true;
			await this.#cleanup("dispose");
		}
	}
	[Symbol.asyncIterator]() {
		return this;
	}
};
function asyncIteratorWithSpan({ name, ...options }, iterator) {
	let span;
	return new AsyncIteratorClass(async () => {
		span ??= startSpan(name);
		try {
			const result = await runInSpanContext(span, () => iterator.next());
			span?.addEvent(result.done ? "completed" : "yielded");
			return result;
		} catch (err) {
			setSpanError(span, err, options);
			throw err;
		}
	}, async (reason) => {
		try {
			if (reason !== "next") await runInSpanContext(span, () => iterator.return?.());
		} catch (err) {
			setSpanError(span, err, options);
			throw err;
		} finally {
			span?.end();
		}
	});
}
function intercept(interceptors, options, main) {
	const next = (options2, index) => {
		const interceptor = interceptors[index];
		if (!interceptor) return main(options2);
		return interceptor({
			...options2,
			next: (newOptions = options2) => next(newOptions, index + 1)
		});
	};
	return next(options, 0);
}
function parseEmptyableJSON(text) {
	if (!text) return;
	return JSON.parse(text);
}
function stringifyJSON(value) {
	return JSON.stringify(value);
}
function getConstructor(value) {
	if (!isTypescriptObject(value)) return null;
	return Object.getPrototypeOf(value)?.constructor;
}
function isObject(value) {
	if (!value || typeof value !== "object") return false;
	const proto = Object.getPrototypeOf(value);
	return proto === Object.prototype || !proto || !proto.constructor;
}
function isTypescriptObject(value) {
	return !!value && (typeof value === "object" || typeof value === "function");
}
var NullProtoObj$1 = /* @__PURE__ */ (() => {
	const e = function() {};
	e.prototype = /* @__PURE__ */ Object.create(null);
	Object.freeze(e.prototype);
	return e;
})();
function value(value2, ...args) {
	if (typeof value2 === "function") return value2(...args);
	return value2;
}
function overlayProxy(target, partial) {
	return new Proxy(typeof target === "function" ? partial : target, {
		get(_, prop) {
			const targetValue = prop in partial ? partial : value(target);
			const v = Reflect.get(targetValue, prop);
			return typeof v === "function" ? v.bind(targetValue) : v;
		},
		has(_, prop) {
			return Reflect.has(partial, prop) || Reflect.has(value(target), prop);
		}
	});
}
function tryDecodeURIComponent(value) {
	try {
		return decodeURIComponent(value);
	} catch {
		return value;
	}
}
var ORPC_CLIENT_PACKAGE_NAME = "@orpc/client";
var ORPC_CLIENT_PACKAGE_VERSION = "1.15.1";
var COMMON_ORPC_ERROR_DEFS = {
	BAD_REQUEST: {
		status: 400,
		message: "Bad Request"
	},
	UNAUTHORIZED: {
		status: 401,
		message: "Unauthorized"
	},
	FORBIDDEN: {
		status: 403,
		message: "Forbidden"
	},
	NOT_FOUND: {
		status: 404,
		message: "Not Found"
	},
	METHOD_NOT_SUPPORTED: {
		status: 405,
		message: "Method Not Supported"
	},
	NOT_ACCEPTABLE: {
		status: 406,
		message: "Not Acceptable"
	},
	TIMEOUT: {
		status: 408,
		message: "Request Timeout"
	},
	CONFLICT: {
		status: 409,
		message: "Conflict"
	},
	PRECONDITION_FAILED: {
		status: 412,
		message: "Precondition Failed"
	},
	PAYLOAD_TOO_LARGE: {
		status: 413,
		message: "Payload Too Large"
	},
	UNSUPPORTED_MEDIA_TYPE: {
		status: 415,
		message: "Unsupported Media Type"
	},
	UNPROCESSABLE_CONTENT: {
		status: 422,
		message: "Unprocessable Content"
	},
	TOO_MANY_REQUESTS: {
		status: 429,
		message: "Too Many Requests"
	},
	CLIENT_CLOSED_REQUEST: {
		status: 499,
		message: "Client Closed Request"
	},
	INTERNAL_SERVER_ERROR: {
		status: 500,
		message: "Internal Server Error"
	},
	NOT_IMPLEMENTED: {
		status: 501,
		message: "Not Implemented"
	},
	BAD_GATEWAY: {
		status: 502,
		message: "Bad Gateway"
	},
	SERVICE_UNAVAILABLE: {
		status: 503,
		message: "Service Unavailable"
	},
	GATEWAY_TIMEOUT: {
		status: 504,
		message: "Gateway Timeout"
	}
};
function fallbackORPCErrorStatus(code, status) {
	return status ?? COMMON_ORPC_ERROR_DEFS[code]?.status ?? 500;
}
function fallbackORPCErrorMessage(code, message) {
	return message || COMMON_ORPC_ERROR_DEFS[code]?.message || code;
}
var globalORPCErrorConstructors;
var ORPCError = class ORPCError extends Error {
	defined;
	code;
	status;
	data;
	static {
		const GLOBAL_ORPC_ERROR_CONSTRUCTORS_SYMBOL = Symbol.for(`__${ORPC_CLIENT_PACKAGE_NAME}@${ORPC_CLIENT_PACKAGE_VERSION}/error/ORPC_ERROR_CONSTRUCTORS__`);
		globalThis[GLOBAL_ORPC_ERROR_CONSTRUCTORS_SYMBOL] ??= /* @__PURE__ */ new WeakSet();
		globalORPCErrorConstructors = globalThis[GLOBAL_ORPC_ERROR_CONSTRUCTORS_SYMBOL];
		globalORPCErrorConstructors.add(ORPCError);
	}
	constructor(code, ...rest) {
		const options = resolveMaybeOptionalOptions(rest);
		if (options.status !== void 0 && !isORPCErrorStatus(options.status)) throw new Error("[ORPCError] Invalid error status code.");
		const message = fallbackORPCErrorMessage(code, options.message);
		super(message, options);
		this.code = code;
		this.status = fallbackORPCErrorStatus(code, options.status);
		this.defined = options.defined ?? false;
		this.data = options.data;
	}
	toJSON() {
		return {
			defined: this.defined,
			code: this.code,
			status: this.status,
			message: this.message,
			data: this.data
		};
	}
	/**
	* Workaround for Next.js where different contexts use separate
	* dependency graphs, causing multiple ORPCError constructors existing and breaking
	* `instanceof` checks across contexts.
	*
	* This is particularly problematic with "Optimized SSR", where orpc-client
	* executes in one context but is invoked from another. When an error is thrown
	* in the execution context, `instanceof ORPCError` checks fail in the
	* invocation context due to separate class constructors.
	*
	* @todo Remove this and related code if Next.js resolves the multiple dependency graph issue.
	*/
	static [Symbol.hasInstance](instance) {
		if (globalORPCErrorConstructors.has(this)) {
			const constructor = getConstructor(instance);
			if (constructor && globalORPCErrorConstructors.has(constructor)) return true;
		}
		return super[Symbol.hasInstance](instance);
	}
};
function toORPCError(error) {
	return error instanceof ORPCError ? error : new ORPCError("INTERNAL_SERVER_ERROR", {
		message: "Internal server error",
		cause: error
	});
}
function isORPCErrorStatus(status) {
	return status < 200 || status >= 400;
}
function isORPCErrorJson(json) {
	if (!isObject(json)) return false;
	const validKeys = [
		"defined",
		"code",
		"status",
		"message",
		"data"
	];
	if (Object.keys(json).some((k) => !validKeys.includes(k))) return false;
	return "defined" in json && typeof json.defined === "boolean" && "code" in json && typeof json.code === "string" && "status" in json && typeof json.status === "number" && isORPCErrorStatus(json.status) && "message" in json && typeof json.message === "string";
}
function createORPCErrorFromJson(json, options = {}) {
	return new ORPCError(json.code, {
		...options,
		...json
	});
}
var EventEncoderError = class extends TypeError {};
var EventDecoderError = class extends TypeError {};
var ErrorEvent = class extends Error {
	data;
	constructor(options) {
		super(options?.message ?? "An error event was received", options);
		this.data = options?.data;
	}
};
var LINE_ENDING_REGEX$1 = /\r\n|\r(?!\n)|\n/;
var MESSAGE_DELIMITER_REGEX = /(?:\r\n|\r(?!\n)|\n){2}/;
var MESSAGE_DELIMITER_GLOBAL_REGEX = /(?:\r\n|\r(?!\n)|\n){2}/g;
var CR = 13;
var LF = 10;
var SPACE = 32;
function decodeEventMessage(encoded) {
	const message = {
		data: void 0,
		event: void 0,
		id: void 0,
		retry: void 0,
		comments: []
	};
	for (const line of encoded.split(LINE_ENDING_REGEX$1)) {
		if (line === "") continue;
		const index = line.indexOf(":");
		const value = index === -1 ? "" : line.slice(line.charCodeAt(index + 1) === SPACE ? index + 2 : index + 1);
		if (index === 0) {
			message.comments.push(value);
			continue;
		}
		switch (index === -1 ? line : line.slice(0, index)) {
			case "data":
				message.data = message.data === void 0 ? value : `${message.data}
${value}`;
				break;
			case "event":
				message.event = value;
				break;
			case "id":
				message.id = value;
				break;
			case "retry": {
				const maybeInteger = Number.parseInt(value, 10);
				if (maybeInteger >= 0 && maybeInteger.toString() === value) message.retry = maybeInteger;
				break;
			}
		}
	}
	return message;
}
var EventDecoder = class {
	constructor(options = {}) {
		this.options = options;
	}
	pending = [];
	tail = "";
	discardLeadingLF = false;
	feed(chunk) {
		if (chunk === "") return;
		if (this.discardLeadingLF) {
			this.discardLeadingLF = false;
			if (chunk.charCodeAt(0) === LF) {
				chunk = chunk.slice(1);
				if (chunk === "") return;
			}
		}
		const scan = this.tail + chunk;
		if (!MESSAGE_DELIMITER_REGEX.test(scan)) {
			this.pending.push(chunk);
			this.tail = scan.slice(-3);
			return;
		}
		this.pending.push(chunk);
		const buffered = this.pending.length === 1 ? chunk : this.pending.join("");
		const offset = buffered.length - scan.length;
		const parts = [];
		let start = 0;
		for (const match of scan.matchAll(MESSAGE_DELIMITER_GLOBAL_REGEX)) {
			parts.push(buffered.slice(start, offset + match.index));
			start = offset + match.index + match[0].length;
		}
		const incomplete = buffered.slice(start);
		this.pending.length = 0;
		this.tail = incomplete.slice(-3);
		if (incomplete === "") this.discardLeadingLF = chunk.charCodeAt(chunk.length - 1) === CR;
		else this.pending.push(incomplete);
		for (const encoded of parts) {
			const message = decodeEventMessage(encoded);
			if (this.options.onEvent) this.options.onEvent(message);
		}
	}
	end() {
		if (this.pending.length !== 0) throw new EventDecoderError("Event Iterator ended before complete");
	}
};
var EventDecoderStream = class extends TransformStream {
	constructor() {
		let decoder;
		super({
			start(controller) {
				decoder = new EventDecoder({ onEvent: (event) => {
					controller.enqueue(event);
				} });
			},
			transform(chunk) {
				decoder.feed(chunk);
			},
			flush() {
				decoder.end();
			}
		});
	}
};
var LINE_ENDING_REGEX = /\r\n|[\n\r]/;
var LINE_ENDING_GLOBAL_REGEX = /\r\n|[\n\r]/g;
function containsLineBreak(value) {
	return LINE_ENDING_REGEX.test(value);
}
function assertEventId(id) {
	if (containsLineBreak(id)) throw new EventEncoderError("Event's id must not contain a carriage return or newline character");
}
function assertEventName(event) {
	if (containsLineBreak(event)) throw new EventEncoderError("Event's event must not contain a carriage return or newline character");
}
function assertEventRetry(retry) {
	if (!Number.isInteger(retry) || retry < 0) throw new EventEncoderError("Event's retry must be a integer and >= 0");
}
function assertEventComment(comment) {
	if (containsLineBreak(comment)) throw new EventEncoderError("Event's comment must not contain a carriage return or newline character");
}
function encodeEventData(data) {
	if (data === void 0) return "";
	return `data: ${data.replace(LINE_ENDING_GLOBAL_REGEX, "\ndata: ")}
`;
}
function encodeEventComments(comments) {
	let output = "";
	for (const comment of comments ?? []) {
		assertEventComment(comment);
		output += `: ${comment}
`;
	}
	return output;
}
function encodeEventMessage(message) {
	let output = "";
	output += encodeEventComments(message.comments);
	if (message.event !== void 0) {
		assertEventName(message.event);
		output += `event: ${message.event}
`;
	}
	if (message.retry !== void 0) {
		assertEventRetry(message.retry);
		output += `retry: ${message.retry}
`;
	}
	if (message.id !== void 0) {
		assertEventId(message.id);
		output += `id: ${message.id}
`;
	}
	output += encodeEventData(message.data);
	output += "\n";
	return output;
}
var EVENT_SOURCE_META_SYMBOL = Symbol("ORPC_EVENT_SOURCE_META");
function withEventMeta(container, meta) {
	if (meta.id === void 0 && meta.retry === void 0 && !meta.comments?.length) return container;
	if (meta.id !== void 0) assertEventId(meta.id);
	if (meta.retry !== void 0) assertEventRetry(meta.retry);
	if (meta.comments !== void 0) for (const comment of meta.comments) assertEventComment(comment);
	return new Proxy(container, { get(target, prop, receiver) {
		if (prop === EVENT_SOURCE_META_SYMBOL) return meta;
		return Reflect.get(target, prop, receiver);
	} });
}
function getEventMeta(container) {
	return isTypescriptObject(container) ? Reflect.get(container, EVENT_SOURCE_META_SYMBOL) : void 0;
}
var HibernationEventIterator = class extends AsyncIteratorClass {
	/**
	* this property is not transferred to the client, so it should be optional for type safety
	*/
	hibernationCallback;
	constructor(hibernationCallback) {
		super(async () => {
			throw new Error("Cannot iterate over hibernating iterator directly");
		}, async (reason) => {
			if (reason !== "next") throw new Error("Cannot cleanup hibernating iterator directly");
		});
		this.hibernationCallback = hibernationCallback;
	}
};
function generateContentDisposition(filename, disposition = "inline") {
	return `${disposition}; filename="${filename.replace(/[^\x20-\x7E]/g, "_").replace(/"/g, "\\\"")}"; filename*=utf-8''${encodeURIComponent(filename).replace(/['()*]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`).replace(/%(7C|60|5E)/g, (str, hex) => String.fromCharCode(Number.parseInt(hex, 16)))}`;
}
function getFilenameFromContentDisposition(contentDisposition) {
	const encodedFilenameStarMatch = contentDisposition.match(/filename\*=(UTF-8'')?([^;]*)/i);
	if (encodedFilenameStarMatch && typeof encodedFilenameStarMatch[2] === "string") return tryDecodeURIComponent(encodedFilenameStarMatch[2]);
	const encodedFilenameMatch = contentDisposition.match(/filename="((?:\\"|[^"])*)"/i);
	if (encodedFilenameMatch && typeof encodedFilenameMatch[1] === "string") return encodedFilenameMatch[1].replace(/\\"/g, "\"");
}
function flattenHeader(header) {
	if (typeof header === "string" || header === void 0) return header;
	if (header.length === 0) return;
	return header.join(", ");
}
function mapEventIterator(iterator, maps) {
	const mapError = async (error) => {
		let mappedError = await maps.error(error);
		if (mappedError !== error) {
			const meta = getEventMeta(error);
			if (meta && isTypescriptObject(mappedError)) mappedError = withEventMeta(mappedError, meta);
		}
		return mappedError;
	};
	return new AsyncIteratorClass(async () => {
		const { done, value } = await (async () => {
			try {
				return await iterator.next();
			} catch (error) {
				throw await mapError(error);
			}
		})();
		let mappedValue = await maps.value(value, done);
		if (mappedValue !== value) {
			const meta = getEventMeta(value);
			if (meta && isTypescriptObject(mappedValue)) mappedValue = withEventMeta(mappedValue, meta);
		}
		return {
			done,
			value: mappedValue
		};
	}, async () => {
		try {
			await iterator.return?.();
		} catch (error) {
			throw await mapError(error);
		}
	});
}
var ValidationError = class extends Error {
	issues;
	data;
	constructor(options) {
		super(options.message, options);
		this.issues = options.issues;
		this.data = options.data;
	}
};
function mergeErrorMap(errorMap1, errorMap2) {
	return {
		...errorMap1,
		...errorMap2
	};
}
async function validateORPCError(map, error) {
	const { code, status, message, data, cause, defined } = error;
	const config = map?.[error.code];
	if (!config || fallbackORPCErrorStatus(error.code, config.status) !== error.status) return defined ? new ORPCError(code, {
		defined: false,
		status,
		message,
		data,
		cause
	}) : error;
	if (!config.data) return defined ? error : new ORPCError(code, {
		defined: true,
		status,
		message,
		data,
		cause
	});
	const validated = await config.data["~standard"].validate(error.data);
	if (validated.issues) return defined ? new ORPCError(code, {
		defined: false,
		status,
		message,
		data,
		cause
	}) : error;
	return new ORPCError(code, {
		defined: true,
		status,
		message,
		data: validated.value,
		cause
	});
}
var ContractProcedure = class {
	/**
	* This property holds the defined options for the contract procedure.
	*/
	"~orpc";
	constructor(def) {
		if (def.route?.successStatus && isORPCErrorStatus(def.route.successStatus)) throw new Error("[ContractProcedure] Invalid successStatus.");
		if (Object.values(def.errorMap).some((val) => val && val.status && !isORPCErrorStatus(val.status))) throw new Error("[ContractProcedure] Invalid error status code.");
		this["~orpc"] = def;
	}
};
function isContractProcedure(item) {
	if (item instanceof ContractProcedure) return true;
	return (typeof item === "object" || typeof item === "function") && item !== null && "~orpc" in item && typeof item["~orpc"] === "object" && item["~orpc"] !== null && "errorMap" in item["~orpc"] && "route" in item["~orpc"] && "meta" in item["~orpc"];
}
function toEventIterator(stream, options = {}) {
	const reader = (stream?.pipeThrough(new TextDecoderStream()).pipeThrough(new EventDecoderStream()))?.getReader();
	let span;
	let isCancelled = false;
	return new AsyncIteratorClass(async () => {
		span ??= startSpan("consume_event_iterator_stream");
		try {
			while (true) {
				if (reader === void 0) return {
					done: true,
					value: void 0
				};
				const { done, value } = await runInSpanContext(span, () => reader.read());
				if (done) {
					if (isCancelled) throw new AbortError("Stream was cancelled");
					return {
						done: true,
						value: void 0
					};
				}
				switch (value.event) {
					case "message": {
						let message = parseEmptyableJSON(value.data);
						if (isTypescriptObject(message)) message = withEventMeta(message, value);
						span?.addEvent("message");
						return {
							done: false,
							value: message
						};
					}
					case "error": {
						let error = new ErrorEvent({ data: parseEmptyableJSON(value.data) });
						error = withEventMeta(error, value);
						span?.addEvent("error");
						throw error;
					}
					case "done": {
						let done2 = parseEmptyableJSON(value.data);
						if (isTypescriptObject(done2)) done2 = withEventMeta(done2, value);
						span?.addEvent("done");
						return {
							done: true,
							value: done2
						};
					}
					default: span?.addEvent("maybe_keepalive");
				}
			}
		} catch (e) {
			if (!(e instanceof ErrorEvent)) setSpanError(span, e, options);
			throw e;
		}
	}, async (reason) => {
		try {
			if (reason !== "next") {
				isCancelled = true;
				span?.addEvent("cancelled");
			}
			await runInSpanContext(span, () => reader?.cancel());
		} catch (e) {
			setSpanError(span, e, options);
			throw e;
		} finally {
			span?.end();
		}
	});
}
function toEventStream(iterator, options = {}) {
	const keepAliveEnabled = options.eventIteratorKeepAliveEnabled ?? true;
	const keepAliveInterval = options.eventIteratorKeepAliveInterval ?? 5e3;
	const keepAliveComment = options.eventIteratorKeepAliveComment ?? "";
	const initialCommentEnabled = options.eventIteratorInitialCommentEnabled ?? true;
	const initialComment = options.eventIteratorInitialComment ?? "";
	let cancelled = false;
	let timeout;
	let span;
	return new ReadableStream({
		start(controller) {
			span = startSpan("stream_event_iterator");
			if (initialCommentEnabled) controller.enqueue(encodeEventMessage({ comments: [initialComment] }));
		},
		async pull(controller) {
			try {
				if (keepAliveEnabled) timeout = setInterval(() => {
					controller.enqueue(encodeEventMessage({ comments: [keepAliveComment] }));
					span?.addEvent("keepalive");
				}, keepAliveInterval);
				const value = await runInSpanContext(span, () => iterator.next());
				clearInterval(timeout);
				if (cancelled) return;
				const meta = getEventMeta(value.value);
				if (!value.done || value.value !== void 0 || meta !== void 0) {
					const event = value.done ? "done" : "message";
					controller.enqueue(encodeEventMessage({
						...meta,
						event,
						data: stringifyJSON(value.value)
					}));
					span?.addEvent(event);
				}
				if (value.done) {
					controller.close();
					span?.end();
				}
			} catch (err) {
				clearInterval(timeout);
				if (cancelled) return;
				if (err instanceof ErrorEvent) {
					controller.enqueue(encodeEventMessage({
						...getEventMeta(err),
						event: "error",
						data: stringifyJSON(err.data)
					}));
					span?.addEvent("error");
					controller.close();
				} else {
					setSpanError(span, err);
					controller.error(err);
				}
				span?.end();
			}
		},
		async cancel() {
			try {
				cancelled = true;
				clearInterval(timeout);
				span?.addEvent("cancelled");
				await runInSpanContext(span, () => iterator.return?.());
			} catch (e) {
				setSpanError(span, e);
				throw e;
			} finally {
				span?.end();
			}
		}
	}).pipeThrough(new TextEncoderStream());
}
function toStandardBody(re, options = {}) {
	return runWithSpan({
		name: "parse_standard_body",
		signal: options.signal
	}, async () => {
		const contentDisposition = re.headers.get("content-disposition");
		if (typeof contentDisposition === "string") {
			const fileName = getFilenameFromContentDisposition(contentDisposition) ?? "blob";
			const blob2 = await re.blob();
			return new File([blob2], fileName, { type: blob2.type });
		}
		const contentType = re.headers.get("content-type");
		if (!contentType || contentType.startsWith("application/json")) return parseEmptyableJSON(await re.text());
		if (contentType.startsWith("multipart/form-data")) return await re.formData();
		if (contentType.startsWith("application/x-www-form-urlencoded")) {
			const text = await re.text();
			return new URLSearchParams(text);
		}
		if (contentType.startsWith("text/event-stream")) return toEventIterator(re.body, options);
		if (contentType.startsWith("text/plain")) return await re.text();
		const blob = await re.blob();
		return new File([blob], "blob", { type: blob.type });
	});
}
function toFetchBody(body, headers, options = {}) {
	if (body instanceof ReadableStream) return body;
	const currentContentDisposition = headers.get("content-disposition");
	headers.delete("content-type");
	headers.delete("content-disposition");
	if (body === void 0) return;
	if (body instanceof Blob) {
		headers.set("content-type", body.type);
		headers.set("content-length", body.size.toString());
		headers.set("content-disposition", currentContentDisposition ?? generateContentDisposition(body instanceof File ? body.name : "blob"));
		return body;
	}
	if (body instanceof FormData) return body;
	if (body instanceof URLSearchParams) return body;
	if (isAsyncIteratorObject(body)) {
		headers.set("content-type", "text/event-stream");
		return toEventStream(body, options);
	}
	headers.set("content-type", "application/json");
	return stringifyJSON(body);
}
function toStandardHeaders(headers, standardHeaders = {}) {
	headers.forEach((value, key) => {
		if (Array.isArray(standardHeaders[key])) standardHeaders[key].push(value);
		else if (standardHeaders[key] !== void 0) standardHeaders[key] = [standardHeaders[key], value];
		else standardHeaders[key] = value;
	});
	return standardHeaders;
}
function toFetchHeaders(headers, fetchHeaders = new Headers()) {
	for (const [key, value] of Object.entries(headers)) if (Array.isArray(value)) for (const v of value) fetchHeaders.append(key, v);
	else if (value !== void 0) fetchHeaders.append(key, value);
	return fetchHeaders;
}
function toStandardLazyRequest(request) {
	return {
		url: new URL(request.url),
		signal: request.signal,
		method: request.method,
		body: once(() => toStandardBody(request, { signal: request.signal })),
		get headers() {
			const headers = toStandardHeaders(request.headers);
			Object.defineProperty(this, "headers", {
				value: headers,
				writable: true
			});
			return headers;
		},
		set headers(value) {
			Object.defineProperty(this, "headers", {
				value,
				writable: true
			});
		}
	};
}
function toFetchResponse(response, options = {}) {
	const headers = toFetchHeaders(response.headers);
	const body = toFetchBody(response.body, headers, options);
	return new Response(body, {
		headers,
		status: response.status
	});
}
function toHttpPath(path) {
	return `/${path.map(encodeURIComponent).join("/")}`;
}
function mergeMeta(meta1, meta2) {
	return {
		...meta1,
		...meta2
	};
}
function mergeRoute(a, b) {
	return {
		...a,
		...b
	};
}
function prefixRoute(route, prefix) {
	if (!route.path) return route;
	return {
		...route,
		path: `${prefix}${route.path}`
	};
}
function unshiftTagRoute(route, tags) {
	return {
		...route,
		tags: [...tags, ...route.tags ?? []]
	};
}
function mergePrefix(a, b) {
	return a ? `${a}${b}` : b;
}
function mergeTags(a, b) {
	return a ? [...a, ...b] : b;
}
function enhanceRoute(route, options) {
	let router = route;
	if (options.prefix) router = prefixRoute(router, options.prefix);
	if (options.tags?.length) router = unshiftTagRoute(router, options.tags);
	return router;
}
function getContractRouter(router, path) {
	let current = router;
	for (let i = 0; i < path.length; i++) {
		const segment = path[i];
		if (!current) return;
		if (isContractProcedure(current)) return;
		if (typeof current !== "object") return;
		current = current[segment];
	}
	return current;
}
function enhanceContractRouter(router, options) {
	if (isContractProcedure(router)) return new ContractProcedure({
		...router["~orpc"],
		errorMap: mergeErrorMap(options.errorMap, router["~orpc"].errorMap),
		route: enhanceRoute(router["~orpc"].route, options)
	});
	if (typeof router !== "object" || router === null) return router;
	const enhanced = {};
	for (const key in router) enhanced[key] = enhanceContractRouter(router[key], options);
	return enhanced;
}
var oc = new class ContractBuilder extends ContractProcedure {
	constructor(def) {
		super(def);
		this["~orpc"].prefix = def.prefix;
		this["~orpc"].tags = def.tags;
	}
	/**
	* Sets or overrides the initial meta.
	*
	* @see {@link https://orpc.dev/docs/metadata Metadata Docs}
	*/
	$meta(initialMeta) {
		return new ContractBuilder({
			...this["~orpc"],
			meta: initialMeta
		});
	}
	/**
	* Sets or overrides the initial route.
	* This option is typically relevant when integrating with OpenAPI.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing OpenAPI Routing Docs}
	* @see {@link https://orpc.dev/docs/openapi/input-output-structure OpenAPI Input/Output Structure Docs}
	*/
	$route(initialRoute) {
		return new ContractBuilder({
			...this["~orpc"],
			route: initialRoute
		});
	}
	/**
	* Sets or overrides the initial input schema.
	*
	* @see {@link https://orpc.dev/docs/procedure#initial-configuration Initial Procedure Configuration Docs}
	*/
	$input(initialInputSchema) {
		return new ContractBuilder({
			...this["~orpc"],
			inputSchema: initialInputSchema
		});
	}
	/**
	* Adds type-safe custom errors to the contract.
	* The provided errors are spared-merged with any existing errors in the contract.
	*
	* @see {@link https://orpc.dev/docs/error-handling#type%E2%80%90safe-error-handling Type-Safe Error Handling Docs}
	*/
	errors(errors) {
		return new ContractBuilder({
			...this["~orpc"],
			errorMap: mergeErrorMap(this["~orpc"].errorMap, errors)
		});
	}
	/**
	* Sets or updates the metadata for the contract.
	* The provided metadata is spared-merged with any existing metadata in the contract.
	*
	* @see {@link https://orpc.dev/docs/metadata Metadata Docs}
	*/
	meta(meta) {
		return new ContractBuilder({
			...this["~orpc"],
			meta: mergeMeta(this["~orpc"].meta, meta)
		});
	}
	/**
	* Sets or updates the route definition for the contract.
	* The provided route is spared-merged with any existing route in the contract.
	* This option is typically relevant when integrating with OpenAPI.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing OpenAPI Routing Docs}
	* @see {@link https://orpc.dev/docs/openapi/input-output-structure OpenAPI Input/Output Structure Docs}
	*/
	route(route) {
		return new ContractBuilder({
			...this["~orpc"],
			route: mergeRoute(this["~orpc"].route, route)
		});
	}
	/**
	* Defines the input validation schema for the contract.
	*
	* @see {@link https://orpc.dev/docs/procedure#input-output-validation Input Validation Docs}
	*/
	input(schema) {
		return new ContractBuilder({
			...this["~orpc"],
			inputSchema: schema
		});
	}
	/**
	* Defines the output validation schema for the contract.
	*
	* @see {@link https://orpc.dev/docs/procedure#input-output-validation Output Validation Docs}
	*/
	output(schema) {
		return new ContractBuilder({
			...this["~orpc"],
			outputSchema: schema
		});
	}
	/**
	* Prefixes all procedures in the contract router.
	* The provided prefix is post-appended to any existing router prefix.
	*
	* @note This option does not affect procedures that do not define a path in their route definition.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing#route-prefixes OpenAPI Route Prefixes Docs}
	*/
	prefix(prefix) {
		return new ContractBuilder({
			...this["~orpc"],
			prefix: mergePrefix(this["~orpc"].prefix, prefix)
		});
	}
	/**
	* Adds tags to all procedures in the contract router.
	* This helpful when you want to group procedures together in the OpenAPI specification.
	*
	* @see {@link https://orpc.dev/docs/openapi/openapi-specification#operation-metadata OpenAPI Operation Metadata Docs}
	*/
	tag(...tags) {
		return new ContractBuilder({
			...this["~orpc"],
			tags: mergeTags(this["~orpc"].tags, tags)
		});
	}
	/**
	* Applies all of the previously defined options to the specified contract router.
	*
	* @see {@link https://orpc.dev/docs/router#extending-router Extending Router Docs}
	*/
	router(router) {
		return enhanceContractRouter(router, this["~orpc"]);
	}
}({
	errorMap: {},
	route: {},
	meta: {}
});
var DEFAULT_CONFIG$1 = {
	defaultMethod: "POST",
	defaultSuccessStatus: 200,
	defaultSuccessDescription: "OK",
	defaultInputStructure: "compact",
	defaultOutputStructure: "compact"
};
function fallbackContractConfig(key, value) {
	if (value === void 0) return DEFAULT_CONFIG$1[key];
	return value;
}
/** Access and audit requirements interpreted by the service's middleware. */
var ProcedureAccess = strictObject({
	/** Credentials required before invoking the procedure. */
	authentication: _enum([
		"public",
		"identity",
		"host"
	]),
	/** Resource type and action checked in the request's authorized scope. */
	permission: strictObject({
		resource: string().min(1),
		action: string().min(1)
	}).nullable(),
	/** Whether successful and failed attempts require security audit records. */
	audit: boolean()
});
/** Declare a procedure with explicit access requirements and conventional errors. */
function defineProcedure(access) {
	return oc.$meta(access).errors({
		NOT_IMPLEMENTED: { status: 501 },
		UNAUTHORIZED: { status: 401 },
		FORBIDDEN: { status: 403 },
		NOT_FOUND: { status: 404 },
		CONFLICT: { status: 409 },
		PRECONDITION_FAILED: { status: 412 },
		SOURCE_MANAGED: { status: 409 },
		UNSUPPORTED: { status: 422 },
		RATE_LIMITED: { status: 429 },
		UNAVAILABLE: { status: 503 }
	});
}
/** An HTTP service handled by a workload's exported function. */
var ServiceDeclaration = defineSchema(strictObject({
	/** The package-local service name. */
	name: ResourceName,
	/** The declaration format version. */
	version: literal(1),
	/** The exported function accepting a Request and returning a Response. */
	handler: string().min(1),
	/** The service transport. */
	protocol: literal("http")
}));
/** Declare an HTTP service for workload routing. */
function defineService(declaration, router) {
	return {
		...ServiceDeclaration.parse(declaration),
		...router ? { router } : {}
	};
}
var VERSION = "1.9.1";
var re = /^(\d+)\.(\d+)\.(\d+)(-(.+))?$/;
/**
* Create a function to test an API version to see if it is compatible with the provided ownVersion.
*
* The returned function has the following semantics:
* - Exact match is always compatible
* - Major versions must match exactly
*    - 1.x package cannot use global 2.x package
*    - 2.x package cannot use global 1.x package
* - The minor version of the API module requesting access to the global API must be less than or equal to the minor version of this API
*    - 1.3 package may use 1.4 global because the later global contains all functions 1.3 expects
*    - 1.4 package may NOT use 1.3 global because it may try to call functions which don't exist on 1.3
* - If the major version is 0, the minor version is treated as the major and the patch is treated as the minor
* - Patch and build tag differences are not considered at this time
*
* @param ownVersion version which should be checked against
*/
function _makeCompatibilityCheck(ownVersion) {
	const acceptedVersions = /* @__PURE__ */ new Set([ownVersion]);
	const rejectedVersions = /* @__PURE__ */ new Set();
	const myVersionMatch = ownVersion.match(re);
	if (!myVersionMatch) return () => false;
	const ownVersionParsed = {
		major: +myVersionMatch[1],
		minor: +myVersionMatch[2],
		patch: +myVersionMatch[3],
		prerelease: myVersionMatch[4]
	};
	if (ownVersionParsed.prerelease != null) return function isExactmatch(globalVersion) {
		return globalVersion === ownVersion;
	};
	function _reject(v) {
		rejectedVersions.add(v);
		return false;
	}
	function _accept(v) {
		acceptedVersions.add(v);
		return true;
	}
	return function isCompatible(globalVersion) {
		if (acceptedVersions.has(globalVersion)) return true;
		if (rejectedVersions.has(globalVersion)) return false;
		const globalVersionMatch = globalVersion.match(re);
		if (!globalVersionMatch) return _reject(globalVersion);
		const globalVersionParsed = {
			major: +globalVersionMatch[1],
			minor: +globalVersionMatch[2],
			patch: +globalVersionMatch[3],
			prerelease: globalVersionMatch[4]
		};
		if (globalVersionParsed.prerelease != null) return _reject(globalVersion);
		if (ownVersionParsed.major !== globalVersionParsed.major) return _reject(globalVersion);
		if (ownVersionParsed.major === 0) {
			if (ownVersionParsed.minor === globalVersionParsed.minor && ownVersionParsed.patch <= globalVersionParsed.patch) return _accept(globalVersion);
			return _reject(globalVersion);
		}
		if (ownVersionParsed.minor <= globalVersionParsed.minor) return _accept(globalVersion);
		return _reject(globalVersion);
	};
}
/**
* Test an API version to see if it is compatible with this API.
*
* - Exact match is always compatible
* - Major versions must match exactly
*    - 1.x package cannot use global 2.x package
*    - 2.x package cannot use global 1.x package
* - The minor version of the API module requesting access to the global API must be less than or equal to the minor version of this API
*    - 1.3 package may use 1.4 global because the later global contains all functions 1.3 expects
*    - 1.4 package may NOT use 1.3 global because it may try to call functions which don't exist on 1.3
* - If the major version is 0, the minor version is treated as the major and the patch is treated as the minor
* - Patch and build tag differences are not considered at this time
*
* @param version version of the API requesting an instance of the global API
*/
var isCompatible = _makeCompatibilityCheck(VERSION);
var major = VERSION.split(".")[0];
var GLOBAL_OPENTELEMETRY_API_KEY = Symbol.for(`opentelemetry.js.api.${major}`);
var _global$1 = typeof globalThis === "object" ? globalThis : typeof self === "object" ? self : typeof window === "object" ? window : typeof global === "object" ? global : {};
function registerGlobal(type, instance, diag, allowOverride = false) {
	var _a;
	const api = _global$1[GLOBAL_OPENTELEMETRY_API_KEY] = (_a = _global$1[GLOBAL_OPENTELEMETRY_API_KEY]) !== null && _a !== void 0 ? _a : { version: VERSION };
	if (!allowOverride && api[type]) {
		const err = /* @__PURE__ */ new Error(`@opentelemetry/api: Attempted duplicate registration of API: ${type}`);
		diag.error(err.stack || err.message);
		return false;
	}
	if (api.version !== "1.9.1") {
		const err = /* @__PURE__ */ new Error(`@opentelemetry/api: Registration of version v${api.version} for ${type} does not match previously registered API v${VERSION}`);
		diag.error(err.stack || err.message);
		return false;
	}
	api[type] = instance;
	diag.debug(`@opentelemetry/api: Registered a global for ${type} v${VERSION}.`);
	return true;
}
function getGlobal(type) {
	var _a, _b;
	const globalVersion = (_a = _global$1[GLOBAL_OPENTELEMETRY_API_KEY]) === null || _a === void 0 ? void 0 : _a.version;
	if (!globalVersion || !isCompatible(globalVersion)) return;
	return (_b = _global$1[GLOBAL_OPENTELEMETRY_API_KEY]) === null || _b === void 0 ? void 0 : _b[type];
}
function unregisterGlobal(type, diag) {
	diag.debug(`@opentelemetry/api: Unregistering a global for ${type} v${VERSION}.`);
	const api = _global$1[GLOBAL_OPENTELEMETRY_API_KEY];
	if (api) delete api[type];
}
/**
* Component Logger which is meant to be used as part of any component which
* will add automatically additional namespace in front of the log message.
* It will then forward all message to global diag logger
* @example
* const cLogger = diag.createComponentLogger({ namespace: '@opentelemetry/instrumentation-http' });
* cLogger.debug('test');
* // @opentelemetry/instrumentation-http test
*/
var DiagComponentLogger = class {
	constructor(props) {
		this._namespace = props.namespace || "DiagComponentLogger";
	}
	debug(...args) {
		return logProxy("debug", this._namespace, args);
	}
	error(...args) {
		return logProxy("error", this._namespace, args);
	}
	info(...args) {
		return logProxy("info", this._namespace, args);
	}
	warn(...args) {
		return logProxy("warn", this._namespace, args);
	}
	verbose(...args) {
		return logProxy("verbose", this._namespace, args);
	}
};
function logProxy(funcName, namespace, args) {
	const logger = getGlobal("diag");
	if (!logger) return;
	return logger[funcName](namespace, ...args);
}
/**
* Defines the available internal logging levels for the diagnostic logger, the numeric values
* of the levels are defined to match the original values from the initial LogLevel to avoid
* compatibility/migration issues for any implementation that assume the numeric ordering.
*/
var DiagLogLevel;
(function(DiagLogLevel) {
	/** Diagnostic Logging level setting to disable all logging (except and forced logs) */
	DiagLogLevel[DiagLogLevel["NONE"] = 0] = "NONE";
	/** Identifies an error scenario */
	DiagLogLevel[DiagLogLevel["ERROR"] = 30] = "ERROR";
	/** Identifies a warning scenario */
	DiagLogLevel[DiagLogLevel["WARN"] = 50] = "WARN";
	/** General informational log message */
	DiagLogLevel[DiagLogLevel["INFO"] = 60] = "INFO";
	/** General debug log message */
	DiagLogLevel[DiagLogLevel["DEBUG"] = 70] = "DEBUG";
	/**
	* Detailed trace level logging should only be used for development, should only be set
	* in a development environment.
	*/
	DiagLogLevel[DiagLogLevel["VERBOSE"] = 80] = "VERBOSE";
	/** Used to set the logging level to include all logging */
	DiagLogLevel[DiagLogLevel["ALL"] = 9999] = "ALL";
})(DiagLogLevel || (DiagLogLevel = {}));
function createLogLevelDiagLogger(maxLevel, logger) {
	if (maxLevel < DiagLogLevel.NONE) maxLevel = DiagLogLevel.NONE;
	else if (maxLevel > DiagLogLevel.ALL) maxLevel = DiagLogLevel.ALL;
	logger = logger || {};
	function _filterFunc(funcName, theLevel) {
		const theFunc = logger[funcName];
		if (typeof theFunc === "function" && maxLevel >= theLevel) return theFunc.bind(logger);
		return function() {};
	}
	return {
		error: _filterFunc("error", DiagLogLevel.ERROR),
		warn: _filterFunc("warn", DiagLogLevel.WARN),
		info: _filterFunc("info", DiagLogLevel.INFO),
		debug: _filterFunc("debug", DiagLogLevel.DEBUG),
		verbose: _filterFunc("verbose", DiagLogLevel.VERBOSE)
	};
}
var API_NAME$4 = "diag";
/**
* Singleton object which represents the entry point to the OpenTelemetry internal
* diagnostic API
*
* @since 1.0.0
*/
var DiagAPI = class DiagAPI {
	/** Get the singleton instance of the DiagAPI API */
	static instance() {
		if (!this._instance) this._instance = new DiagAPI();
		return this._instance;
	}
	/**
	* Private internal constructor
	* @private
	*/
	constructor() {
		function _logProxy(funcName) {
			return function(...args) {
				const logger = getGlobal("diag");
				if (!logger) return;
				return logger[funcName](...args);
			};
		}
		const self = this;
		const setLogger = (logger, optionsOrLogLevel = { logLevel: DiagLogLevel.INFO }) => {
			var _a, _b, _c;
			if (logger === self) {
				const err = /* @__PURE__ */ new Error("Cannot use diag as the logger for itself. Please use a DiagLogger implementation like ConsoleDiagLogger or a custom implementation");
				self.error((_a = err.stack) !== null && _a !== void 0 ? _a : err.message);
				return false;
			}
			if (typeof optionsOrLogLevel === "number") optionsOrLogLevel = { logLevel: optionsOrLogLevel };
			const oldLogger = getGlobal("diag");
			const newLogger = createLogLevelDiagLogger((_b = optionsOrLogLevel.logLevel) !== null && _b !== void 0 ? _b : DiagLogLevel.INFO, logger);
			if (oldLogger && !optionsOrLogLevel.suppressOverrideMessage) {
				const stack = (_c = (/* @__PURE__ */ new Error()).stack) !== null && _c !== void 0 ? _c : "<failed to generate stacktrace>";
				oldLogger.warn(`Current logger will be overwritten from ${stack}`);
				newLogger.warn(`Current logger will overwrite one already registered from ${stack}`);
			}
			return registerGlobal("diag", newLogger, self, true);
		};
		self.setLogger = setLogger;
		self.disable = () => {
			unregisterGlobal(API_NAME$4, self);
		};
		self.createComponentLogger = (options) => {
			return new DiagComponentLogger(options);
		};
		self.verbose = _logProxy("verbose");
		self.debug = _logProxy("debug");
		self.info = _logProxy("info");
		self.warn = _logProxy("warn");
		self.error = _logProxy("error");
	}
};
var BaggageImpl = class BaggageImpl {
	constructor(entries) {
		this._entries = entries ? new Map(entries) : /* @__PURE__ */ new Map();
	}
	getEntry(key) {
		const entry = this._entries.get(key);
		if (!entry) return;
		return Object.assign({}, entry);
	}
	getAllEntries() {
		return Array.from(this._entries.entries());
	}
	setEntry(key, entry) {
		const newBaggage = new BaggageImpl(this._entries);
		newBaggage._entries.set(key, entry);
		return newBaggage;
	}
	removeEntry(key) {
		const newBaggage = new BaggageImpl(this._entries);
		newBaggage._entries.delete(key);
		return newBaggage;
	}
	removeEntries(...keys) {
		const newBaggage = new BaggageImpl(this._entries);
		for (const key of keys) newBaggage._entries.delete(key);
		return newBaggage;
	}
	clear() {
		return new BaggageImpl();
	}
};
DiagAPI.instance();
/**
* Create a new Baggage with optional entries
*
* @param entries An array of baggage entries the new baggage should contain
*/
function createBaggage(entries = {}) {
	return new BaggageImpl(new Map(Object.entries(entries)));
}
/**
* Get a key to uniquely identify a context value
*
* @since 1.0.0
*/
function createContextKey(description) {
	return Symbol.for(description);
}
/**
* The root context is used as the default parent context when there is no active context
*
* @since 1.0.0
*/
var ROOT_CONTEXT = new class BaseContext {
	/**
	* Construct a new context which inherits values from an optional parent context.
	*
	* @param parentContext a context from which to inherit values
	*/
	constructor(parentContext) {
		const self = this;
		self._currentContext = parentContext ? new Map(parentContext) : /* @__PURE__ */ new Map();
		self.getValue = (key) => self._currentContext.get(key);
		self.setValue = (key, value) => {
			const context = new BaseContext(self._currentContext);
			context._currentContext.set(key, value);
			return context;
		};
		self.deleteValue = (key) => {
			const context = new BaseContext(self._currentContext);
			context._currentContext.delete(key);
			return context;
		};
	}
}();
/**
* NoopMeter is a noop implementation of the {@link Meter} interface. It reuses
* constant NoopMetrics for all of its methods.
*/
var NoopMeter = class {
	constructor() {}
	/**
	* @see {@link Meter.createGauge}
	*/
	createGauge(_name, _options) {
		return NOOP_GAUGE_METRIC;
	}
	/**
	* @see {@link Meter.createHistogram}
	*/
	createHistogram(_name, _options) {
		return NOOP_HISTOGRAM_METRIC;
	}
	/**
	* @see {@link Meter.createCounter}
	*/
	createCounter(_name, _options) {
		return NOOP_COUNTER_METRIC;
	}
	/**
	* @see {@link Meter.createUpDownCounter}
	*/
	createUpDownCounter(_name, _options) {
		return NOOP_UP_DOWN_COUNTER_METRIC;
	}
	/**
	* @see {@link Meter.createObservableGauge}
	*/
	createObservableGauge(_name, _options) {
		return NOOP_OBSERVABLE_GAUGE_METRIC;
	}
	/**
	* @see {@link Meter.createObservableCounter}
	*/
	createObservableCounter(_name, _options) {
		return NOOP_OBSERVABLE_COUNTER_METRIC;
	}
	/**
	* @see {@link Meter.createObservableUpDownCounter}
	*/
	createObservableUpDownCounter(_name, _options) {
		return NOOP_OBSERVABLE_UP_DOWN_COUNTER_METRIC;
	}
	/**
	* @see {@link Meter.addBatchObservableCallback}
	*/
	addBatchObservableCallback(_callback, _observables) {}
	/**
	* @see {@link Meter.removeBatchObservableCallback}
	*/
	removeBatchObservableCallback(_callback) {}
};
var NoopMetric = class {};
var NoopCounterMetric = class extends NoopMetric {
	add(_value, _attributes) {}
};
var NoopUpDownCounterMetric = class extends NoopMetric {
	add(_value, _attributes) {}
};
var NoopGaugeMetric = class extends NoopMetric {
	record(_value, _attributes) {}
};
var NoopHistogramMetric = class extends NoopMetric {
	record(_value, _attributes) {}
};
var NoopObservableMetric = class {
	addCallback(_callback) {}
	removeCallback(_callback) {}
};
var NoopObservableCounterMetric = class extends NoopObservableMetric {};
var NoopObservableGaugeMetric = class extends NoopObservableMetric {};
var NoopObservableUpDownCounterMetric = class extends NoopObservableMetric {};
var NOOP_METER = new NoopMeter();
var NOOP_COUNTER_METRIC = new NoopCounterMetric();
var NOOP_GAUGE_METRIC = new NoopGaugeMetric();
var NOOP_HISTOGRAM_METRIC = new NoopHistogramMetric();
var NOOP_UP_DOWN_COUNTER_METRIC = new NoopUpDownCounterMetric();
var NOOP_OBSERVABLE_COUNTER_METRIC = new NoopObservableCounterMetric();
var NOOP_OBSERVABLE_GAUGE_METRIC = new NoopObservableGaugeMetric();
var NOOP_OBSERVABLE_UP_DOWN_COUNTER_METRIC = new NoopObservableUpDownCounterMetric();
/**
* @since 1.0.0
*/
var defaultTextMapGetter = {
	get(carrier, key) {
		if (carrier == null) return;
		return carrier[key];
	},
	keys(carrier) {
		if (carrier == null) return [];
		return Object.keys(carrier);
	}
};
/**
* @since 1.0.0
*/
var defaultTextMapSetter = { set(carrier, key, value) {
	if (carrier == null) return;
	carrier[key] = value;
} };
var NoopContextManager = class {
	active() {
		return ROOT_CONTEXT;
	}
	with(_context, fn, thisArg, ...args) {
		return fn.call(thisArg, ...args);
	}
	bind(_context, target) {
		return target;
	}
	enable() {
		return this;
	}
	disable() {
		return this;
	}
};
var API_NAME$3 = "context";
var NOOP_CONTEXT_MANAGER = new NoopContextManager();
/**
* Singleton object which represents the entry point to the OpenTelemetry Context API
*
* @since 1.0.0
*/
var ContextAPI = class ContextAPI {
	/** Empty private constructor prevents end users from constructing a new instance of the API */
	constructor() {}
	/** Get the singleton instance of the Context API */
	static getInstance() {
		if (!this._instance) this._instance = new ContextAPI();
		return this._instance;
	}
	/**
	* Set the current context manager.
	*
	* @returns true if the context manager was successfully registered, else false
	*/
	setGlobalContextManager(contextManager) {
		return registerGlobal(API_NAME$3, contextManager, DiagAPI.instance());
	}
	/**
	* Get the currently active context
	*/
	active() {
		return this._getContextManager().active();
	}
	/**
	* Execute a function with an active context
	*
	* @param context context to be active during function execution
	* @param fn function to execute in a context
	* @param thisArg optional receiver to be used for calling fn
	* @param args optional arguments forwarded to fn
	*/
	with(context, fn, thisArg, ...args) {
		return this._getContextManager().with(context, fn, thisArg, ...args);
	}
	/**
	* Bind a context to a target function or event emitter
	*
	* @param context context to bind to the event emitter or function. Defaults to the currently active context
	* @param target function or event emitter to bind
	*/
	bind(context, target) {
		return this._getContextManager().bind(context, target);
	}
	_getContextManager() {
		return getGlobal(API_NAME$3) || NOOP_CONTEXT_MANAGER;
	}
	/** Disable and remove the global context manager */
	disable() {
		this._getContextManager().disable();
		unregisterGlobal(API_NAME$3, DiagAPI.instance());
	}
};
/**
* @since 1.0.0
*/
var TraceFlags;
(function(TraceFlags) {
	/** Represents no flag set. */
	TraceFlags[TraceFlags["NONE"] = 0] = "NONE";
	/** Bit to represent whether trace is sampled in trace flags. */
	TraceFlags[TraceFlags["SAMPLED"] = 1] = "SAMPLED";
})(TraceFlags || (TraceFlags = {}));
/**
* @since 1.0.0
*/
var INVALID_SPAN_CONTEXT = {
	traceId: "00000000000000000000000000000000",
	spanId: "0000000000000000",
	traceFlags: TraceFlags.NONE
};
/**
* The NonRecordingSpan is the default {@link Span} that is used when no Span
* implementation is available. All operations are no-op including context
* propagation.
*/
var NonRecordingSpan = class {
	constructor(spanContext = INVALID_SPAN_CONTEXT) {
		this._spanContext = spanContext;
	}
	spanContext() {
		return this._spanContext;
	}
	setAttribute(_key, _value) {
		return this;
	}
	setAttributes(_attributes) {
		return this;
	}
	addEvent(_name, _attributes) {
		return this;
	}
	addLink(_link) {
		return this;
	}
	addLinks(_links) {
		return this;
	}
	setStatus(_status) {
		return this;
	}
	updateName(_name) {
		return this;
	}
	end(_endTime) {}
	isRecording() {
		return false;
	}
	recordException(_exception, _time) {}
};
/**
* span key
*/
var SPAN_KEY = createContextKey("OpenTelemetry Context Key SPAN");
/**
* Return the span if one exists
*
* @param context context to get span from
*/
function getSpan(context) {
	return context.getValue(SPAN_KEY) || void 0;
}
/**
* Gets the span from the current context, if one exists.
*/
function getActiveSpan() {
	return getSpan(ContextAPI.getInstance().active());
}
/**
* Set the span on a context
*
* @param context context to use as parent
* @param span span to set active
*/
function setSpan(context, span) {
	return context.setValue(SPAN_KEY, span);
}
/**
* Remove current span stored in the context
*
* @param context context to delete span from
*/
function deleteSpan(context) {
	return context.deleteValue(SPAN_KEY);
}
/**
* Wrap span context in a NoopSpan and set as span in a new
* context
*
* @param context context to set active span on
* @param spanContext span context to be wrapped
*/
function setSpanContext(context, spanContext) {
	return setSpan(context, new NonRecordingSpan(spanContext));
}
/**
* Get the span context of the span if it exists.
*
* @param context context to get values from
*/
function getSpanContext(context) {
	var _a;
	return (_a = getSpan(context)) === null || _a === void 0 ? void 0 : _a.spanContext();
}
var isHex = new Uint8Array([
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	1,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	1,
	1,
	1,
	1,
	1,
	1,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	0,
	1,
	1,
	1,
	1,
	1,
	1
]);
function isValidHex(id, length) {
	if (typeof id !== "string" || id.length !== length) return false;
	let r = 0;
	for (let i = 0; i < id.length; i += 4) r += (isHex[id.charCodeAt(i)] | 0) + (isHex[id.charCodeAt(i + 1)] | 0) + (isHex[id.charCodeAt(i + 2)] | 0) + (isHex[id.charCodeAt(i + 3)] | 0);
	return r === length;
}
/**
* @since 1.0.0
*/
function isValidTraceId(traceId) {
	return isValidHex(traceId, 32) && traceId !== "00000000000000000000000000000000";
}
/**
* @since 1.0.0
*/
function isValidSpanId(spanId) {
	return isValidHex(spanId, 16) && spanId !== "0000000000000000";
}
/**
* Returns true if this {@link SpanContext} is valid.
* @return true if this {@link SpanContext} is valid.
*
* @since 1.0.0
*/
function isSpanContextValid(spanContext) {
	return isValidTraceId(spanContext.traceId) && isValidSpanId(spanContext.spanId);
}
/**
* Wrap the given {@link SpanContext} in a new non-recording {@link Span}
*
* @param spanContext span context to be wrapped
* @returns a new non-recording {@link Span} with the provided context
*/
function wrapSpanContext(spanContext) {
	return new NonRecordingSpan(spanContext);
}
var contextApi = ContextAPI.getInstance();
/**
* No-op implementations of {@link Tracer}.
*/
var NoopTracer = class {
	startSpan(name, options, context = contextApi.active()) {
		if (Boolean(options === null || options === void 0 ? void 0 : options.root)) return new NonRecordingSpan();
		const parentFromContext = context && getSpanContext(context);
		if (isSpanContext(parentFromContext) && isSpanContextValid(parentFromContext)) return new NonRecordingSpan(parentFromContext);
		else return new NonRecordingSpan();
	}
	startActiveSpan(name, arg2, arg3, arg4) {
		let opts;
		let ctx;
		let fn;
		if (arguments.length < 2) return;
		else if (arguments.length === 2) fn = arg2;
		else if (arguments.length === 3) {
			opts = arg2;
			fn = arg3;
		} else {
			opts = arg2;
			ctx = arg3;
			fn = arg4;
		}
		const parentContext = ctx !== null && ctx !== void 0 ? ctx : contextApi.active();
		const span = this.startSpan(name, opts, parentContext);
		const contextWithSpanSet = setSpan(parentContext, span);
		return contextApi.with(contextWithSpanSet, fn, void 0, span);
	}
};
function isSpanContext(spanContext) {
	return spanContext !== null && typeof spanContext === "object" && "spanId" in spanContext && typeof spanContext["spanId"] === "string" && "traceId" in spanContext && typeof spanContext["traceId"] === "string" && "traceFlags" in spanContext && typeof spanContext["traceFlags"] === "number";
}
var NOOP_TRACER = new NoopTracer();
/**
* Proxy tracer provided by the proxy tracer provider
*
* @since 1.0.0
*/
var ProxyTracer = class {
	constructor(provider, name, version, options) {
		this._provider = provider;
		this.name = name;
		this.version = version;
		this.options = options;
	}
	startSpan(name, options, context) {
		return this._getTracer().startSpan(name, options, context);
	}
	startActiveSpan(_name, _options, _context, _fn) {
		const tracer = this._getTracer();
		return Reflect.apply(tracer.startActiveSpan, tracer, arguments);
	}
	/**
	* Try to get a tracer from the proxy tracer provider.
	* If the proxy tracer provider has no delegate, return a noop tracer.
	*/
	_getTracer() {
		if (this._delegate) return this._delegate;
		const tracer = this._provider.getDelegateTracer(this.name, this.version, this.options);
		if (!tracer) return NOOP_TRACER;
		this._delegate = tracer;
		return this._delegate;
	}
};
/**
* An implementation of the {@link TracerProvider} which returns an impotent
* Tracer for all calls to `getTracer`.
*
* All operations are no-op.
*/
var NoopTracerProvider = class {
	getTracer(_name, _version, _options) {
		return new NoopTracer();
	}
};
var NOOP_TRACER_PROVIDER = new NoopTracerProvider();
/**
* Tracer provider which provides {@link ProxyTracer}s.
*
* Before a delegate is set, tracers provided are NoOp.
*   When a delegate is set, traces are provided from the delegate.
*   When a delegate is set after tracers have already been provided,
*   all tracers already provided will use the provided delegate implementation.
*
* @deprecated This will be removed in the next major version.
* @since 1.0.0
*/
var ProxyTracerProvider = class {
	/**
	* Get a {@link ProxyTracer}
	*/
	getTracer(name, version, options) {
		var _a;
		return (_a = this.getDelegateTracer(name, version, options)) !== null && _a !== void 0 ? _a : new ProxyTracer(this, name, version, options);
	}
	getDelegate() {
		var _a;
		return (_a = this._delegate) !== null && _a !== void 0 ? _a : NOOP_TRACER_PROVIDER;
	}
	/**
	* Set the delegate tracer provider
	*/
	setDelegate(delegate) {
		this._delegate = delegate;
	}
	getDelegateTracer(name, version, options) {
		var _a;
		return (_a = this._delegate) === null || _a === void 0 ? void 0 : _a.getTracer(name, version, options);
	}
};
/**
* @since 1.0.0
*/
var SpanKind;
(function(SpanKind) {
	/** Default value. Indicates that the span is used internally. */
	SpanKind[SpanKind["INTERNAL"] = 0] = "INTERNAL";
	/**
	* Indicates that the span covers server-side handling of an RPC or other
	* remote request.
	*/
	SpanKind[SpanKind["SERVER"] = 1] = "SERVER";
	/**
	* Indicates that the span covers the client-side wrapper around an RPC or
	* other remote request.
	*/
	SpanKind[SpanKind["CLIENT"] = 2] = "CLIENT";
	/**
	* Indicates that the span describes producer sending a message to a
	* broker. Unlike client and server, there is no direct critical path latency
	* relationship between producer and consumer spans.
	*/
	SpanKind[SpanKind["PRODUCER"] = 3] = "PRODUCER";
	/**
	* Indicates that the span describes consumer receiving a message from a
	* broker. Unlike client and server, there is no direct critical path latency
	* relationship between producer and consumer spans.
	*/
	SpanKind[SpanKind["CONSUMER"] = 4] = "CONSUMER";
})(SpanKind || (SpanKind = {}));
/**
* An enumeration of status codes.
*
* @since 1.0.0
*/
var SpanStatusCode;
(function(SpanStatusCode) {
	/**
	* The default status.
	*/
	SpanStatusCode[SpanStatusCode["UNSET"] = 0] = "UNSET";
	/**
	* The operation has been validated by an Application developer or
	* Operator to have completed successfully.
	*/
	SpanStatusCode[SpanStatusCode["OK"] = 1] = "OK";
	/**
	* The operation contains an error.
	*/
	SpanStatusCode[SpanStatusCode["ERROR"] = 2] = "ERROR";
})(SpanStatusCode || (SpanStatusCode = {}));
/**
* Entrypoint for context API
* @since 1.0.0
*/
var context = ContextAPI.getInstance();
/**
* An implementation of the {@link MeterProvider} which returns an impotent Meter
* for all calls to `getMeter`
*/
var NoopMeterProvider = class {
	getMeter(_name, _version, _options) {
		return NOOP_METER;
	}
};
var NOOP_METER_PROVIDER = new NoopMeterProvider();
var API_NAME$2 = "metrics";
/**
* Entrypoint for metrics API
*
* @since 1.3.0
*/
var metrics = class MetricsAPI {
	/** Empty private constructor prevents end users from constructing a new instance of the API */
	constructor() {}
	/** Get the singleton instance of the Metrics API */
	static getInstance() {
		if (!this._instance) this._instance = new MetricsAPI();
		return this._instance;
	}
	/**
	* Set the current global meter provider.
	* Returns true if the meter provider was successfully registered, else false.
	*/
	setGlobalMeterProvider(provider) {
		return registerGlobal(API_NAME$2, provider, DiagAPI.instance());
	}
	/**
	* Returns the global meter provider.
	*/
	getMeterProvider() {
		return getGlobal(API_NAME$2) || NOOP_METER_PROVIDER;
	}
	/**
	* Returns a meter from the global meter provider.
	*/
	getMeter(name, version, options) {
		return this.getMeterProvider().getMeter(name, version, options);
	}
	/** Remove the global meter provider */
	disable() {
		unregisterGlobal(API_NAME$2, DiagAPI.instance());
	}
}.getInstance();
/**
* No-op implementations of {@link TextMapPropagator}.
*/
var NoopTextMapPropagator = class {
	/** Noop inject function does nothing */
	inject(_context, _carrier) {}
	/** Noop extract function does nothing and returns the input context */
	extract(context, _carrier) {
		return context;
	}
	fields() {
		return [];
	}
};
/**
* Baggage key
*/
var BAGGAGE_KEY = createContextKey("OpenTelemetry Baggage Key");
/**
* Retrieve the current baggage from the given context
*
* @param {Context} Context that manage all context values
* @returns {Baggage} Extracted baggage from the context
*/
function getBaggage(context) {
	return context.getValue(BAGGAGE_KEY) || void 0;
}
/**
* Retrieve the current baggage from the active/current context
*
* @returns {Baggage} Extracted baggage from the context
*/
function getActiveBaggage() {
	return getBaggage(ContextAPI.getInstance().active());
}
/**
* Store a baggage in the given context
*
* @param {Context} Context that manage all context values
* @param {Baggage} baggage that will be set in the actual context
*/
function setBaggage(context, baggage) {
	return context.setValue(BAGGAGE_KEY, baggage);
}
/**
* Delete the baggage stored in the given context
*
* @param {Context} Context that manage all context values
*/
function deleteBaggage(context) {
	return context.deleteValue(BAGGAGE_KEY);
}
var API_NAME$1 = "propagation";
var NOOP_TEXT_MAP_PROPAGATOR = new NoopTextMapPropagator();
/**
* Entrypoint for propagation API
*
* @since 1.0.0
*/
var propagation = class PropagationAPI {
	/** Empty private constructor prevents end users from constructing a new instance of the API */
	constructor() {
		this.createBaggage = createBaggage;
		this.getBaggage = getBaggage;
		this.getActiveBaggage = getActiveBaggage;
		this.setBaggage = setBaggage;
		this.deleteBaggage = deleteBaggage;
	}
	/** Get the singleton instance of the Propagator API */
	static getInstance() {
		if (!this._instance) this._instance = new PropagationAPI();
		return this._instance;
	}
	/**
	* Set the current propagator.
	*
	* @returns true if the propagator was successfully registered, else false
	*/
	setGlobalPropagator(propagator) {
		return registerGlobal(API_NAME$1, propagator, DiagAPI.instance());
	}
	/**
	* Inject context into a carrier to be propagated inter-process
	*
	* @param context Context carrying tracing data to inject
	* @param carrier carrier to inject context into
	* @param setter Function used to set values on the carrier
	*/
	inject(context, carrier, setter = defaultTextMapSetter) {
		return this._getGlobalPropagator().inject(context, carrier, setter);
	}
	/**
	* Extract context from a carrier
	*
	* @param context Context which the newly created context will inherit from
	* @param carrier Carrier to extract context from
	* @param getter Function used to extract keys from a carrier
	*/
	extract(context, carrier, getter = defaultTextMapGetter) {
		return this._getGlobalPropagator().extract(context, carrier, getter);
	}
	/**
	* Return a list of all fields which may be used by the propagator.
	*/
	fields() {
		return this._getGlobalPropagator().fields();
	}
	/** Remove the global propagator */
	disable() {
		unregisterGlobal(API_NAME$1, DiagAPI.instance());
	}
	_getGlobalPropagator() {
		return getGlobal(API_NAME$1) || NOOP_TEXT_MAP_PROPAGATOR;
	}
}.getInstance();
var API_NAME = "trace";
/**
* Entrypoint for trace API
*
* @since 1.0.0
*/
var trace = class TraceAPI {
	/** Empty private constructor prevents end users from constructing a new instance of the API */
	constructor() {
		this._proxyTracerProvider = new ProxyTracerProvider();
		this.wrapSpanContext = wrapSpanContext;
		this.isSpanContextValid = isSpanContextValid;
		this.deleteSpan = deleteSpan;
		this.getSpan = getSpan;
		this.getActiveSpan = getActiveSpan;
		this.getSpanContext = getSpanContext;
		this.setSpan = setSpan;
		this.setSpanContext = setSpanContext;
	}
	/** Get the singleton instance of the Trace API */
	static getInstance() {
		if (!this._instance) this._instance = new TraceAPI();
		return this._instance;
	}
	/**
	* Set the current global tracer.
	*
	* @returns true if the tracer provider was successfully registered, else false
	*/
	setGlobalTracerProvider(provider) {
		const success = registerGlobal(API_NAME, this._proxyTracerProvider, DiagAPI.instance());
		if (success) this._proxyTracerProvider.setDelegate(provider);
		return success;
	}
	/**
	* Returns the global tracer provider.
	*/
	getTracerProvider() {
		return getGlobal(API_NAME) || this._proxyTracerProvider;
	}
	/**
	* Returns a tracer from the global tracer provider.
	*/
	getTracer(name, version) {
		return this.getTracerProvider().getTracer(name, version);
	}
	/** Remove the global tracer provider */
	disable() {
		unregisterGlobal(API_NAME, DiagAPI.instance());
		this._proxyTracerProvider = new ProxyTracerProvider();
	}
}.getInstance();
var NoopLogger = class {
	emit(_logRecord) {}
	enabled() {
		return false;
	}
};
var NOOP_LOGGER = new NoopLogger();
var GLOBAL_LOGS_API_KEY = Symbol.for("io.opentelemetry.js.api.logs");
var _global = globalThis;
/**
* Make a function which accepts a version integer and returns the instance of an API if the version
* is compatible, or a fallback version (usually NOOP) if it is not.
*
* @param requiredVersion Backwards compatibility version which is required to return the instance
* @param instance Instance which should be returned if the required version is compatible
* @param fallback Fallback instance, usually NOOP, which will be returned if the required version is not compatible
*/
function makeGetter(requiredVersion, instance, fallback) {
	return (version) => version === requiredVersion ? instance : fallback;
}
var NoopLoggerProvider = class {
	getLogger(_name, _version, _options) {
		return new NoopLogger();
	}
};
var NOOP_LOGGER_PROVIDER = new NoopLoggerProvider();
var ProxyLogger = class {
	constructor(provider, name, version, options) {
		this._provider = provider;
		this.name = name;
		this.version = version;
		this.options = options;
	}
	/**
	* Emit a log record. This method should only be used by log appenders.
	*
	* @param logRecord
	*/
	emit(logRecord) {
		this._getLogger().emit(logRecord);
	}
	enabled(options) {
		return this._getLogger().enabled(options);
	}
	/**
	* Try to get a logger from the proxy logger provider.
	* If the proxy logger provider has no delegate, return a noop logger.
	*/
	_getLogger() {
		if (this._delegate) return this._delegate;
		const logger = this._provider._getDelegateLogger(this.name, this.version, this.options);
		if (!logger) return NOOP_LOGGER;
		this._delegate = logger;
		return this._delegate;
	}
};
var ProxyLoggerProvider = class {
	getLogger(name, version, options) {
		var _a;
		return (_a = this._getDelegateLogger(name, version, options)) !== null && _a !== void 0 ? _a : new ProxyLogger(this, name, version, options);
	}
	/**
	* Get the delegate logger provider.
	* Used by tests only.
	* @internal
	*/
	_getDelegate() {
		var _a;
		return (_a = this._delegate) !== null && _a !== void 0 ? _a : NOOP_LOGGER_PROVIDER;
	}
	/**
	* Set the delegate logger provider
	* @internal
	*/
	_setDelegate(delegate) {
		this._delegate = delegate;
	}
	/**
	* @internal
	*/
	_getDelegateLogger(name, version, options) {
		var _a;
		return (_a = this._delegate) === null || _a === void 0 ? void 0 : _a.getLogger(name, version, options);
	}
};
var logs = class LogsAPI {
	constructor() {
		this._proxyLoggerProvider = new ProxyLoggerProvider();
	}
	static getInstance() {
		if (!this._instance) this._instance = new LogsAPI();
		return this._instance;
	}
	setGlobalLoggerProvider(provider) {
		if (_global[GLOBAL_LOGS_API_KEY]) return this.getLoggerProvider();
		_global[GLOBAL_LOGS_API_KEY] = makeGetter(1, provider, NOOP_LOGGER_PROVIDER);
		this._proxyLoggerProvider._setDelegate(provider);
		return provider;
	}
	/**
	* Returns the global logger provider.
	*
	* @returns LoggerProvider
	*/
	getLoggerProvider() {
		var _a, _b;
		return (_b = (_a = _global[GLOBAL_LOGS_API_KEY]) === null || _a === void 0 ? void 0 : _a.call(_global, 1)) !== null && _b !== void 0 ? _b : this._proxyLoggerProvider;
	}
	/**
	* Returns a Logger, creating one if one with the given name, version,
	* schemaUrl, and attributes is not already created.
	*
	* Getting a Logger may be expensive, especially when `attributes` are
	* provided. Reuse Logger instances where possible instead of calling
	* `getLogger()` on hot paths.
	*
	* @param name The name of the logger or instrumentation library.
	* @param version The version of the logger or instrumentation library.
	* @param options The options of the logger or instrumentation library.
	* @returns {@link Logger}
	*/
	getLogger(name, version, options) {
		return this.getLoggerProvider().getLogger(name, version, options);
	}
	/** Remove the global logger provider */
	disable() {
		delete _global[GLOBAL_LOGS_API_KEY];
		this._proxyLoggerProvider = new ProxyLoggerProvider();
	}
}.getInstance();
/** Extract a remote trace without inheriting another request's active context. */
function extractContext(headers, propagator = propagation) {
	return propagator.extract(ROOT_CONTEXT, headers, {
		keys: (headers) => [...headers.keys()],
		get: (headers, name) => headers.get(name) ?? void 0
	});
}
/** Obtain package instruments from the providers registered by the host. */
function scope(source) {
	return {
		tracer: trace.getTracer(source.name, source.version),
		meter: metrics.getMeter(source.name, source.version),
		logger: logs.getLogger(source.name, source.version)
	};
}
var package_default = {
	name: "@destack/service",
	version: "2026.9.0",
	"private": true,
	license: "MIT",
	type: "module",
	description: "Define Destack HTTP services.",
	sideEffects: false,
	scripts: {
		"check": "tsc --project tsconfig.json",
		"test": "vitest run --config vitest.config.ts"
	},
	exports: {
		"./watch": "./src/watch/index.ts",
		"./declare": "./src/declare/index.ts",
		"./operation": "./src/operation/index.ts",
		"./procedure": "./src/procedure/index.ts",
		"./page": "./src/page/index.ts",
		"./health": "./src/health/index.ts",
		"./error": "./src/error/index.ts",
		".": "./src/index.ts",
		"./client": "./src/client/index.ts",
		"./server": "./src/server/index.ts",
		"./openapi": "./src/openapi/index.ts",
		"./inspect": "./src/inspect/index.ts",
		"./schedule": "./src/schedule/index.ts"
	},
	devDependencies: { "@destack/test": "workspace:*" },
	dependencies: {
		"@destack/schema": "workspace:*",
		"@destack/telemetry": "workspace:*",
		"@orpc/shared": "1.15.1",
		"@orpc/client": "1.15.1",
		"@orpc/contract": "1.15.1",
		"@orpc/openapi": "1.15.1",
		"@orpc/openapi-client": "1.15.1",
		"@orpc/server": "1.15.1",
		"@destack/resource": "workspace:*"
	}
};
/** Service failure instrumentation. */
var instruments$1 = scope(package_default);
/** Record unexpected failures and return an error safe to send to clients. */
function reportError(error) {
	if (error instanceof ORPCError && error.status < 500) return error;
	const exception = error instanceof Error ? error : new Error(String(error));
	trace.getActiveSpan()?.recordException(exception);
	instruments$1.logger.emit({
		severityNumber: 17,
		severityText: "ERROR",
		body: "Service request failed",
		attributes: {
			"exception.type": exception.name,
			"exception.message": exception.message,
			...exception.stack ? { "exception.stacktrace": exception.stack } : {}
		}
	});
	if (error instanceof ORPCError) return error;
	return new ORPCError("INTERNAL_SERVER_ERROR", {
		message: "Internal server error",
		cause: error
	});
}
function resolveFriendlyStandardHandleOptions(options) {
	return {
		...options,
		context: options.context ?? {}
	};
}
var LAZY_SYMBOL = Symbol("ORPC_LAZY_SYMBOL");
function lazy(loader, meta = {}) {
	return { [LAZY_SYMBOL]: {
		loader,
		meta
	} };
}
function isLazy(item) {
	return (typeof item === "object" || typeof item === "function") && item !== null && LAZY_SYMBOL in item;
}
function getLazyMeta(lazied) {
	return lazied[LAZY_SYMBOL].meta;
}
function unlazy(lazied) {
	return isLazy(lazied) ? lazied[LAZY_SYMBOL].loader() : Promise.resolve({ default: lazied });
}
function isStartWithMiddlewares(middlewares, compare) {
	if (compare.length > middlewares.length) return false;
	for (let i = 0; i < middlewares.length; i++) {
		if (compare[i] === void 0) return true;
		if (middlewares[i] !== compare[i]) return false;
	}
	return true;
}
function mergeMiddlewares(first, second, options) {
	if (options.dedupeLeading && isStartWithMiddlewares(second, first)) return second;
	return [...first, ...second];
}
function addMiddleware(middlewares, addition) {
	return [...middlewares, addition];
}
var Procedure = class {
	/**
	* This property holds the defined options.
	*/
	"~orpc";
	constructor(def) {
		this["~orpc"] = def;
	}
};
function isProcedure(item) {
	if (item instanceof Procedure) return true;
	return isContractProcedure(item) && "middlewares" in item["~orpc"] && "inputValidationIndex" in item["~orpc"] && "outputValidationIndex" in item["~orpc"] && "handler" in item["~orpc"];
}
function mergeCurrentContext(context, other) {
	return {
		...context,
		...other
	};
}
function createORPCErrorConstructorMap(errors) {
	return new Proxy(errors, { get(target, code) {
		if (typeof code !== "string") return Reflect.get(target, code);
		const item = (...rest) => {
			const options = resolveMaybeOptionalOptions(rest);
			const config = errors[code];
			return new ORPCError(code, {
				defined: Boolean(config),
				status: config?.status,
				message: options.message ?? config?.message,
				data: options.data,
				cause: options.cause
			});
		};
		return item;
	} });
}
function middlewareOutputFn(output) {
	return {
		output,
		context: {}
	};
}
function createProcedureClient(lazyableProcedure, ...rest) {
	const options = resolveMaybeOptionalOptions(rest);
	return async (...[input, callerOptions]) => {
		const path = toArray(options.path);
		const { default: procedure } = await unlazy(lazyableProcedure);
		const clientContext = callerOptions?.context ?? {};
		const context = await value(options.context ?? {}, clientContext);
		const errors = createORPCErrorConstructorMap(procedure["~orpc"].errorMap);
		const validateError = async (e) => {
			if (e instanceof ORPCError) return await validateORPCError(procedure["~orpc"].errorMap, e);
			return e;
		};
		try {
			const output = await runWithSpan({
				name: "call_procedure",
				signal: callerOptions?.signal
			}, (span) => {
				span?.setAttribute("procedure.path", [...path]);
				return intercept(toArray(options.interceptors), {
					context,
					input,
					errors,
					path,
					procedure,
					signal: callerOptions?.signal,
					lastEventId: callerOptions?.lastEventId
				}, (interceptorOptions) => executeProcedureInternal(interceptorOptions.procedure, interceptorOptions));
			});
			if (isAsyncIteratorObject(output)) {
				if (output instanceof HibernationEventIterator) return output;
				return overlayProxy(output, mapEventIterator(asyncIteratorWithSpan({
					name: "consume_event_iterator_output",
					signal: callerOptions?.signal
				}, output), {
					value: (v) => v,
					error: (e) => validateError(e)
				}));
			}
			return output;
		} catch (e) {
			throw await validateError(e);
		}
	};
}
async function validateInput(procedure, input) {
	const schema = procedure["~orpc"].inputSchema;
	if (!schema) return input;
	return runWithSpan({ name: "validate_input" }, async () => {
		const result = await schema["~standard"].validate(input);
		if (result.issues) throw new ORPCError("BAD_REQUEST", {
			message: "Input validation failed",
			data: { issues: result.issues },
			cause: new ValidationError({
				message: "Input validation failed",
				issues: result.issues,
				data: input
			})
		});
		return result.value;
	});
}
async function validateOutput(procedure, output) {
	const schema = procedure["~orpc"].outputSchema;
	if (!schema) return output;
	return runWithSpan({ name: "validate_output" }, async () => {
		const result = await schema["~standard"].validate(output);
		if (result.issues) throw new ORPCError("INTERNAL_SERVER_ERROR", {
			message: "Output validation failed",
			cause: new ValidationError({
				message: "Output validation failed",
				issues: result.issues,
				data: output
			})
		});
		return result.value;
	});
}
async function executeProcedureInternal(procedure, options) {
	const middlewares = procedure["~orpc"].middlewares;
	const inputValidationIndex = Math.min(Math.max(0, procedure["~orpc"].inputValidationIndex), middlewares.length);
	const outputValidationIndex = Math.min(Math.max(0, procedure["~orpc"].outputValidationIndex), middlewares.length);
	const next = async (index, context, input) => {
		let currentInput = input;
		if (index === inputValidationIndex) currentInput = await validateInput(procedure, currentInput);
		const mid = middlewares[index];
		const output = mid ? await runWithSpan({
			name: `middleware.${mid.name}`,
			signal: options.signal
		}, async (span) => {
			span?.setAttribute("middleware.index", index);
			span?.setAttribute("middleware.name", mid.name);
			return (await mid({
				...options,
				context,
				next: async (...[nextOptions]) => {
					const nextContext = nextOptions?.context ?? {};
					return {
						output: await next(index + 1, mergeCurrentContext(context, nextContext), currentInput),
						context: nextContext
					};
				}
			}, currentInput, middlewareOutputFn)).output;
		}) : await runWithSpan({
			name: "handler",
			signal: options.signal
		}, () => procedure["~orpc"].handler({
			...options,
			context,
			input: currentInput
		}));
		if (index === outputValidationIndex) return await validateOutput(procedure, output);
		return output;
	};
	return next(0, options.context, options.input);
}
var HIDDEN_ROUTER_CONTRACT_SYMBOL = Symbol("ORPC_HIDDEN_ROUTER_CONTRACT");
function setHiddenRouterContract(router, contract) {
	return new Proxy(router, { get(target, key) {
		if (key === HIDDEN_ROUTER_CONTRACT_SYMBOL) return contract;
		return Reflect.get(target, key);
	} });
}
function getHiddenRouterContract(router) {
	return router[HIDDEN_ROUTER_CONTRACT_SYMBOL];
}
function getRouter(router, path) {
	let current = router;
	for (let i = 0; i < path.length; i++) {
		const segment = path[i];
		if (!current) return;
		if (isProcedure(current)) return;
		if (!isTypescriptObject(current)) return;
		if (!isLazy(current)) {
			current = current[segment];
			continue;
		}
		const lazied = current;
		const rest = path.slice(i);
		return lazy(async () => {
			return unlazy(getRouter((await unlazy(lazied)).default, rest));
		}, getLazyMeta(lazied));
	}
	return current;
}
function createAccessibleLazyRouter(lazied) {
	return new Proxy(lazied, { get(target, key) {
		if (typeof key !== "string") return Reflect.get(target, key);
		return createAccessibleLazyRouter(getRouter(lazied, [key]));
	} });
}
function enhanceRouter(router, options) {
	if (isLazy(router)) {
		const laziedMeta = getLazyMeta(router);
		const enhancedPrefix = laziedMeta?.prefix ? mergePrefix(options.prefix, laziedMeta?.prefix) : options.prefix;
		return createAccessibleLazyRouter(lazy(async () => {
			const { default: unlaziedRouter } = await unlazy(router);
			return unlazy(enhanceRouter(unlaziedRouter, options));
		}, {
			...laziedMeta,
			prefix: enhancedPrefix
		}));
	}
	if (isProcedure(router)) {
		const newMiddlewares = mergeMiddlewares(options.middlewares, router["~orpc"].middlewares, { dedupeLeading: options.dedupeLeadingMiddlewares });
		const newMiddlewareAdded = newMiddlewares.length - router["~orpc"].middlewares.length;
		return new Procedure({
			...router["~orpc"],
			route: enhanceRoute(router["~orpc"].route, options),
			errorMap: mergeErrorMap(options.errorMap, router["~orpc"].errorMap),
			middlewares: newMiddlewares,
			inputValidationIndex: router["~orpc"].inputValidationIndex + newMiddlewareAdded,
			outputValidationIndex: router["~orpc"].outputValidationIndex + newMiddlewareAdded
		});
	}
	if (typeof router !== "object" || router === null) return router;
	const enhanced = {};
	for (const key in router) enhanced[key] = enhanceRouter(router[key], options);
	return enhanced;
}
function traverseContractProcedures(options, callback, lazyOptions = []) {
	let currentRouter = options.router;
	const hiddenContract = isTypescriptObject(options.router) ? getHiddenRouterContract(options.router) : void 0;
	if (hiddenContract !== void 0) currentRouter = hiddenContract;
	if (isLazy(currentRouter)) lazyOptions.push({
		router: currentRouter,
		path: options.path
	});
	else if (isContractProcedure(currentRouter)) callback({
		contract: currentRouter,
		path: options.path
	});
	else if (typeof currentRouter === "object" && currentRouter !== null) for (const key in currentRouter) traverseContractProcedures({
		router: currentRouter[key],
		path: [...options.path, key]
	}, callback, lazyOptions);
	return lazyOptions;
}
function createContractedProcedure(procedure, contract) {
	return new Procedure({
		...procedure["~orpc"],
		errorMap: contract["~orpc"].errorMap,
		route: contract["~orpc"].route,
		meta: contract["~orpc"].meta
	});
}
var CompositeStandardHandlerPlugin = class {
	plugins;
	constructor(plugins = []) {
		this.plugins = [...plugins].sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
	}
	init(options, router) {
		for (const plugin of this.plugins) plugin.init?.(options, router);
	}
};
var StandardHandler = class {
	constructor(router, matcher, codec, options) {
		this.matcher = matcher;
		this.codec = codec;
		new CompositeStandardHandlerPlugin(options.plugins).init(options, router);
		this.interceptors = toArray(options.interceptors);
		this.clientInterceptors = toArray(options.clientInterceptors);
		this.rootInterceptors = toArray(options.rootInterceptors);
		this.matcher.init(router);
	}
	interceptors;
	clientInterceptors;
	rootInterceptors;
	async handle(request, options) {
		const prefix = options.prefix?.replace(/\/$/, "") || void 0;
		if (prefix && !request.url.pathname.startsWith(`${prefix}/`) && request.url.pathname !== prefix) return {
			matched: false,
			response: void 0
		};
		return intercept(this.rootInterceptors, {
			...options,
			request,
			prefix
		}, async (interceptorOptions) => {
			return runWithSpan({ name: `${request.method} ${request.url.pathname}` }, async (span) => {
				let step;
				try {
					return await intercept(this.interceptors, interceptorOptions, async ({ request: request2, context, prefix: prefix2 }) => {
						const method = request2.method;
						const url = request2.url;
						const pathname = prefix2 ? url.pathname.replace(prefix2, "") : url.pathname;
						const match = await runWithSpan({ name: "find_procedure" }, () => this.matcher.match(method, `/${pathname.replace(/^\/|\/$/g, "")}`));
						if (!match) return {
							matched: false,
							response: void 0
						};
						span?.updateName(`${ORPC_NAME}.${match.path.join("/")}`);
						span?.setAttribute("rpc.system", ORPC_NAME);
						span?.setAttribute("rpc.method", match.path.join("."));
						step = "decode_input";
						let input = await runWithSpan({ name: "decode_input" }, () => this.codec.decode(request2, match.params, match.procedure));
						step = void 0;
						if (isAsyncIteratorObject(input)) input = asyncIteratorWithSpan({
							name: "consume_event_iterator_input",
							signal: request2.signal
						}, input);
						const client = createProcedureClient(match.procedure, {
							context,
							path: match.path,
							interceptors: this.clientInterceptors
						});
						step = "call_procedure";
						const output = await client(input, {
							signal: request2.signal,
							lastEventId: flattenHeader(request2.headers["last-event-id"])
						});
						step = void 0;
						return {
							matched: true,
							response: this.codec.encode(output, match.procedure)
						};
					});
				} catch (e) {
					if (step !== "call_procedure") setSpanError(span, e);
					const error = step === "decode_input" && !(e instanceof ORPCError) ? new ORPCError("BAD_REQUEST", {
						message: `Malformed request. Ensure the request body is properly formatted and the 'Content-Type' header is set correctly.`,
						cause: e
					}) : toORPCError(e);
					return {
						matched: true,
						response: this.codec.encodeError(error)
					};
				}
			});
		});
	}
};
var CompositeFetchHandlerPlugin = class extends CompositeStandardHandlerPlugin {
	initRuntimeAdapter(options) {
		for (const plugin of this.plugins) plugin.initRuntimeAdapter?.(options);
	}
};
var FetchHandler = class {
	constructor(standardHandler, options = {}) {
		this.standardHandler = standardHandler;
		new CompositeFetchHandlerPlugin(options.plugins).initRuntimeAdapter(options);
		this.adapterInterceptors = toArray(options.adapterInterceptors);
		this.toFetchResponseOptions = options;
	}
	toFetchResponseOptions;
	adapterInterceptors;
	async handle(request, ...rest) {
		return intercept(this.adapterInterceptors, {
			...resolveFriendlyStandardHandleOptions(resolveMaybeOptionalOptions(rest)),
			request,
			toFetchResponseOptions: this.toFetchResponseOptions
		}, async ({ request: request2, toFetchResponseOptions, ...options }) => {
			const standardRequest = toStandardLazyRequest(request2);
			const result = await this.standardHandler.handle(standardRequest, options);
			if (!result.matched) return result;
			return {
				matched: true,
				response: toFetchResponse(result.response, toFetchResponseOptions)
			};
		});
	}
};
var StandardBracketNotationSerializer = class {
	maxArrayIndex;
	constructor(options = {}) {
		this.maxArrayIndex = options.maxBracketNotationArrayIndex ?? 9999;
	}
	serialize(data, segments = [], result = []) {
		if (Array.isArray(data)) data.forEach((item, i) => {
			this.serialize(item, [...segments, i], result);
		});
		else if (isObject(data)) for (const key in data) this.serialize(data[key], [...segments, key], result);
		else result.push([this.stringifyPath(segments), data]);
		return result;
	}
	deserialize(serialized) {
		if (serialized.length === 0) return {};
		const arrayPushStyles = /* @__PURE__ */ new WeakSet();
		const ref = { value: [] };
		for (const [path, value] of serialized) {
			const segments = this.parsePath(path);
			let currentRef = ref;
			let nextSegment = "value";
			segments.forEach((segment, i) => {
				if (!Array.isArray(currentRef[nextSegment]) && !isObject(currentRef[nextSegment])) currentRef[nextSegment] = [];
				if (i !== segments.length - 1) {
					if (Array.isArray(currentRef[nextSegment]) && !isValidArrayIndex(segment, this.maxArrayIndex)) {
						if (arrayPushStyles.has(currentRef[nextSegment])) {
							arrayPushStyles.delete(currentRef[nextSegment]);
							currentRef[nextSegment] = pushStyleArrayToObject(currentRef[nextSegment]);
						} else currentRef[nextSegment] = arrayToObject(currentRef[nextSegment]);
					}
				} else if (Array.isArray(currentRef[nextSegment])) {
					if (segment === "") {
						if (currentRef[nextSegment].length && !arrayPushStyles.has(currentRef[nextSegment])) currentRef[nextSegment] = arrayToObject(currentRef[nextSegment]);
					} else if (arrayPushStyles.has(currentRef[nextSegment])) {
						arrayPushStyles.delete(currentRef[nextSegment]);
						currentRef[nextSegment] = pushStyleArrayToObject(currentRef[nextSegment]);
					} else if (!isValidArrayIndex(segment, this.maxArrayIndex)) currentRef[nextSegment] = arrayToObject(currentRef[nextSegment]);
				}
				currentRef = currentRef[nextSegment];
				nextSegment = segment;
			});
			if (Array.isArray(currentRef) && nextSegment === "") {
				arrayPushStyles.add(currentRef);
				currentRef.push(value);
			} else if (nextSegment in currentRef) {
				if (Array.isArray(currentRef[nextSegment])) currentRef[nextSegment].push(value);
				else currentRef[nextSegment] = [currentRef[nextSegment], value];
			} else currentRef[nextSegment] = value;
		}
		return ref.value;
	}
	stringifyPath(segments) {
		return segments.map((segment) => {
			return segment.toString().replace(/[\\[\]]/g, (match) => {
				switch (match) {
					case "\\": return "\\\\";
					case "[": return "\\[";
					case "]": return "\\]";
					/* v8 ignore next 2 */
					default: return match;
				}
			});
		}).reduce((result, segment, i) => {
			if (i === 0) return segment;
			return `${result}[${segment}]`;
		}, "");
	}
	parsePath(path) {
		const segments = [];
		let inBrackets = false;
		let currentSegment = "";
		let backslashCount = 0;
		for (let i = 0; i < path.length; i++) {
			const char = path[i];
			const nextChar = path[i + 1];
			if (inBrackets && char === "]" && (nextChar === void 0 || nextChar === "[") && backslashCount % 2 === 0) {
				if (nextChar === void 0) inBrackets = false;
				segments.push(currentSegment);
				currentSegment = "";
				i++;
			} else if (segments.length === 0 && char === "[" && backslashCount % 2 === 0) {
				inBrackets = true;
				segments.push(currentSegment);
				currentSegment = "";
			} else if (char === "\\") backslashCount++;
			else {
				currentSegment += "\\".repeat(backslashCount / 2) + char;
				backslashCount = 0;
			}
		}
		return inBrackets || segments.length === 0 ? [path] : segments;
	}
};
function isValidArrayIndex(value, maxIndex) {
	return /^0$|^[1-9]\d*$/.test(value) && Number(value) <= maxIndex;
}
function arrayToObject(array) {
	const obj = new NullProtoObj$1();
	array.forEach((item, i) => {
		obj[i] = item;
	});
	return obj;
}
function pushStyleArrayToObject(array) {
	const obj = new NullProtoObj$1();
	obj[""] = array.length === 1 ? array[0] : array;
	return obj;
}
var StandardOpenAPIJsonSerializer = class {
	customSerializers;
	constructor(options = {}) {
		this.customSerializers = options.customJsonSerializers ?? [];
	}
	serialize(data, hasBlobRef = { value: false }) {
		for (const custom of this.customSerializers) if (custom.condition(data)) return this.serialize(custom.serialize(data), hasBlobRef);
		if (data instanceof Blob) {
			hasBlobRef.value = true;
			return [data, hasBlobRef.value];
		}
		if (data instanceof Set) return this.serialize(Array.from(data), hasBlobRef);
		if (data instanceof Map) return this.serialize(Array.from(data.entries()), hasBlobRef);
		if (Array.isArray(data)) return [data.map((v) => v === void 0 ? null : this.serialize(v, hasBlobRef)[0]), hasBlobRef.value];
		if (isObject(data)) {
			const json = {};
			for (const k in data) {
				if (k === "toJSON" && typeof data[k] === "function") continue;
				json[k] = this.serialize(data[k], hasBlobRef)[0];
			}
			return [json, hasBlobRef.value];
		}
		if (typeof data === "bigint" || data instanceof RegExp || data instanceof URL) return [data.toString(), hasBlobRef.value];
		if (data instanceof Date) return [Number.isNaN(data.getTime()) ? null : data.toISOString(), hasBlobRef.value];
		if (Number.isNaN(data)) return [null, hasBlobRef.value];
		return [data, hasBlobRef.value];
	}
};
function standardizeHTTPPath(path) {
	return `/${path.replace(/\/{2,}/g, "/").replace(/^\/|\/$/g, "")}`;
}
var StandardOpenAPISerializer = class {
	constructor(jsonSerializer, bracketNotation) {
		this.jsonSerializer = jsonSerializer;
		this.bracketNotation = bracketNotation;
	}
	serialize(data, options = {}) {
		if (isAsyncIteratorObject(data) && !options.outputFormat) return mapEventIterator(data, {
			value: async (value) => this.#serialize(value, { outputFormat: "plain" }),
			error: async (e) => {
				return new ErrorEvent({
					data: this.#serialize(toORPCError(e).toJSON(), { outputFormat: "plain" }),
					cause: e
				});
			}
		});
		return this.#serialize(data, options);
	}
	#serialize(data, options) {
		const [json, hasBlob] = this.jsonSerializer.serialize(data);
		if (options.outputFormat === "plain") return json;
		if (options.outputFormat === "URLSearchParams") {
			const params = new URLSearchParams();
			for (const [path, value] of this.bracketNotation.serialize(json)) if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") params.append(path, value.toString());
			return params;
		}
		if (json instanceof Blob || json === void 0 || !hasBlob) return json;
		const form = new FormData();
		for (const [path, value] of this.bracketNotation.serialize(json)) if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") form.append(path, value.toString());
		else if (value instanceof Blob) form.append(path, value);
		return form;
	}
	deserialize(data) {
		if (data instanceof URLSearchParams || data instanceof FormData) return this.bracketNotation.deserialize(Array.from(data.entries()));
		if (isAsyncIteratorObject(data)) return mapEventIterator(data, {
			value: async (value) => value,
			error: async (e) => {
				if (e instanceof ErrorEvent && isORPCErrorJson(e.data)) return createORPCErrorFromJson(e.data, { cause: e });
				return e;
			}
		});
		return data;
	}
};
var DEFAULT_CONFIG = {
	initialInputValidationIndex: 0,
	initialOutputValidationIndex: 0,
	dedupeLeadingMiddlewares: true
};
function fallbackConfig(key, value) {
	if (value === void 0) return DEFAULT_CONFIG[key];
	return value;
}
function decorateMiddleware(middleware) {
	const decorated = ((...args) => middleware(...args));
	decorated.mapInput = (mapInput) => {
		return decorateMiddleware((options, input, ...rest) => middleware(options, mapInput(input), ...rest));
	};
	decorated.concat = (concatMiddleware, mapInput) => {
		const mapped = mapInput ? decorateMiddleware(concatMiddleware).mapInput(mapInput) : concatMiddleware;
		return decorateMiddleware((options, input, output, ...rest) => {
			return middleware({
				...options,
				next: (...[nextOptions1]) => mapped({
					...options,
					context: {
						...options.context,
						...nextOptions1?.context
					},
					next: (...[nextOptions2]) => options.next({ context: {
						...nextOptions1?.context,
						...nextOptions2?.context
					} })
				}, input, output, ...rest)
			}, input, output, ...rest);
		});
	};
	return decorated;
}
function createActionableClient(client) {
	const action = async (input) => {
		try {
			return [null, await client(input)];
		} catch (error) {
			if (error instanceof Error && "digest" in error && typeof error.digest === "string" && error.digest.startsWith("NEXT_")) throw error;
			if (error instanceof Response && "options" in error && isObject(error.options) || isObject(error) && error.isNotFound === true) throw error;
			return [toORPCError(error).toJSON(), void 0];
		}
	};
	return action;
}
var DecoratedProcedure = class DecoratedProcedure extends Procedure {
	/**
	* Adds type-safe custom errors.
	* The provided errors are spared-merged with any existing errors.
	*
	* @see {@link https://orpc.dev/docs/error-handling#type%E2%80%90safe-error-handling Type-Safe Error Handling Docs}
	*/
	errors(errors) {
		return new DecoratedProcedure({
			...this["~orpc"],
			errorMap: mergeErrorMap(this["~orpc"].errorMap, errors)
		});
	}
	/**
	* Sets or updates the metadata.
	* The provided metadata is spared-merged with any existing metadata.
	*
	* @see {@link https://orpc.dev/docs/metadata Metadata Docs}
	*/
	meta(meta) {
		return new DecoratedProcedure({
			...this["~orpc"],
			meta: mergeMeta(this["~orpc"].meta, meta)
		});
	}
	/**
	* Sets or updates the route definition.
	* The provided route is spared-merged with any existing route.
	* This option is typically relevant when integrating with OpenAPI.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing OpenAPI Routing Docs}
	* @see {@link https://orpc.dev/docs/openapi/input-output-structure OpenAPI Input/Output Structure Docs}
	*/
	route(route) {
		return new DecoratedProcedure({
			...this["~orpc"],
			route: mergeRoute(this["~orpc"].route, route)
		});
	}
	use(middleware, mapInput) {
		const mapped = mapInput ? decorateMiddleware(middleware).mapInput(mapInput) : middleware;
		return new DecoratedProcedure({
			...this["~orpc"],
			middlewares: addMiddleware(this["~orpc"].middlewares, mapped)
		});
	}
	/**
	* Make this procedure callable (works like a function while still being a procedure).
	*
	* @see {@link https://orpc.dev/docs/client/server-side Server-side Client Docs}
	*/
	callable(...rest) {
		const client = createProcedureClient(this, ...rest);
		return new Proxy(client, {
			get: (target, key) => {
				return Reflect.has(this, key) ? Reflect.get(this, key) : Reflect.get(target, key);
			},
			has: (target, key) => {
				return Reflect.has(this, key) || Reflect.has(target, key);
			}
		});
	}
	/**
	* Make this procedure compatible with server action.
	*
	* @see {@link https://orpc.dev/docs/server-action Server Action Docs}
	*/
	actionable(...rest) {
		const action = createActionableClient(createProcedureClient(this, ...rest));
		return new Proxy(action, {
			get: (target, key) => {
				return Reflect.has(this, key) ? Reflect.get(this, key) : Reflect.get(target, key);
			},
			has: (target, key) => {
				return Reflect.has(this, key) || Reflect.has(target, key);
			}
		});
	}
};
var Builder = class Builder {
	/**
	* This property holds the defined options.
	*/
	"~orpc";
	constructor(def) {
		this["~orpc"] = def;
	}
	/**
	* Sets or overrides the config.
	*
	* @see {@link https://orpc.dev/docs/client/server-side#middlewares-order Middlewares Order Docs}
	* @see {@link https://orpc.dev/docs/best-practices/dedupe-middleware#configuration Dedupe Middleware Docs}
	*/
	$config(config) {
		const inputValidationCount = this["~orpc"].inputValidationIndex - fallbackConfig("initialInputValidationIndex", this["~orpc"].config.initialInputValidationIndex);
		const outputValidationCount = this["~orpc"].outputValidationIndex - fallbackConfig("initialOutputValidationIndex", this["~orpc"].config.initialOutputValidationIndex);
		return new Builder({
			...this["~orpc"],
			config,
			dedupeLeadingMiddlewares: fallbackConfig("dedupeLeadingMiddlewares", config.dedupeLeadingMiddlewares),
			inputValidationIndex: fallbackConfig("initialInputValidationIndex", config.initialInputValidationIndex) + inputValidationCount,
			outputValidationIndex: fallbackConfig("initialOutputValidationIndex", config.initialOutputValidationIndex) + outputValidationCount
		});
	}
	/**
	* Set or override the initial context.
	*
	* @see {@link https://orpc.dev/docs/context Context Docs}
	*/
	$context() {
		return new Builder({
			...this["~orpc"],
			middlewares: [],
			inputValidationIndex: fallbackConfig("initialInputValidationIndex", this["~orpc"].config.initialInputValidationIndex),
			outputValidationIndex: fallbackConfig("initialOutputValidationIndex", this["~orpc"].config.initialOutputValidationIndex)
		});
	}
	/**
	* Sets or overrides the initial meta.
	*
	* @see {@link https://orpc.dev/docs/metadata Metadata Docs}
	*/
	$meta(initialMeta) {
		return new Builder({
			...this["~orpc"],
			meta: initialMeta
		});
	}
	/**
	* Sets or overrides the initial route.
	* This option is typically relevant when integrating with OpenAPI.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing OpenAPI Routing Docs}
	* @see {@link https://orpc.dev/docs/openapi/input-output-structure OpenAPI Input/Output Structure Docs}
	*/
	$route(initialRoute) {
		return new Builder({
			...this["~orpc"],
			route: initialRoute
		});
	}
	/**
	* Sets or overrides the initial input schema.
	*
	* @see {@link https://orpc.dev/docs/procedure#initial-configuration Initial Procedure Configuration Docs}
	*/
	$input(initialInputSchema) {
		return new Builder({
			...this["~orpc"],
			inputSchema: initialInputSchema
		});
	}
	/**
	* Creates a middleware.
	*
	* @see {@link https://orpc.dev/docs/middleware Middleware Docs}
	*/
	middleware(middleware) {
		return decorateMiddleware(middleware);
	}
	/**
	* Adds type-safe custom errors.
	* The provided errors are spared-merged with any existing errors.
	*
	* @see {@link https://orpc.dev/docs/error-handling#type%E2%80%90safe-error-handling Type-Safe Error Handling Docs}
	*/
	errors(errors) {
		return new Builder({
			...this["~orpc"],
			errorMap: mergeErrorMap(this["~orpc"].errorMap, errors)
		});
	}
	use(middleware, mapInput) {
		const mapped = mapInput ? decorateMiddleware(middleware).mapInput(mapInput) : middleware;
		return new Builder({
			...this["~orpc"],
			middlewares: addMiddleware(this["~orpc"].middlewares, mapped)
		});
	}
	/**
	* Sets or updates the metadata.
	* The provided metadata is spared-merged with any existing metadata.
	*
	* @see {@link https://orpc.dev/docs/metadata Metadata Docs}
	*/
	meta(meta) {
		return new Builder({
			...this["~orpc"],
			meta: mergeMeta(this["~orpc"].meta, meta)
		});
	}
	/**
	* Sets or updates the route definition.
	* The provided route is spared-merged with any existing route.
	* This option is typically relevant when integrating with OpenAPI.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing OpenAPI Routing Docs}
	* @see {@link https://orpc.dev/docs/openapi/input-output-structure OpenAPI Input/Output Structure Docs}
	*/
	route(route) {
		return new Builder({
			...this["~orpc"],
			route: mergeRoute(this["~orpc"].route, route)
		});
	}
	/**
	* Defines the input validation schema.
	*
	* @see {@link https://orpc.dev/docs/procedure#input-output-validation Input Validation Docs}
	*/
	input(schema) {
		return new Builder({
			...this["~orpc"],
			inputSchema: schema,
			inputValidationIndex: fallbackConfig("initialInputValidationIndex", this["~orpc"].config.initialInputValidationIndex) + this["~orpc"].middlewares.length
		});
	}
	/**
	* Defines the output validation schema.
	*
	* @see {@link https://orpc.dev/docs/procedure#input-output-validation Output Validation Docs}
	*/
	output(schema) {
		return new Builder({
			...this["~orpc"],
			outputSchema: schema,
			outputValidationIndex: fallbackConfig("initialOutputValidationIndex", this["~orpc"].config.initialOutputValidationIndex) + this["~orpc"].middlewares.length
		});
	}
	/**
	* Defines the handler of the procedure.
	*
	* @see {@link https://orpc.dev/docs/procedure Procedure Docs}
	*/
	handler(handler) {
		return new DecoratedProcedure({
			...this["~orpc"],
			handler
		});
	}
	/**
	* Prefixes all procedures in the router.
	* The provided prefix is post-appended to any existing router prefix.
	*
	* @note This option does not affect procedures that do not define a path in their route definition.
	*
	* @see {@link https://orpc.dev/docs/openapi/routing#route-prefixes OpenAPI Route Prefixes Docs}
	*/
	prefix(prefix) {
		return new Builder({
			...this["~orpc"],
			prefix: mergePrefix(this["~orpc"].prefix, prefix)
		});
	}
	/**
	* Adds tags to all procedures in the router.
	* This helpful when you want to group procedures together in the OpenAPI specification.
	*
	* @see {@link https://orpc.dev/docs/openapi/openapi-specification#operation-metadata OpenAPI Operation Metadata Docs}
	*/
	tag(...tags) {
		return new Builder({
			...this["~orpc"],
			tags: mergeTags(this["~orpc"].tags, tags)
		});
	}
	/**
	* Applies all of the previously defined options to the specified router.
	*
	* @see {@link https://orpc.dev/docs/router#extending-router Extending Router Docs}
	*/
	router(router) {
		return enhanceRouter(router, this["~orpc"]);
	}
	/**
	* Create a lazy router
	* And applies all of the previously defined options to the specified router.
	*
	* @see {@link https://orpc.dev/docs/router#extending-router Extending Router Docs}
	*/
	lazy(loader) {
		return enhanceRouter(lazy(loader), this["~orpc"]);
	}
};
new Builder({
	config: {},
	route: {},
	meta: {},
	errorMap: {},
	inputValidationIndex: fallbackConfig("initialInputValidationIndex"),
	outputValidationIndex: fallbackConfig("initialOutputValidationIndex"),
	middlewares: [],
	dedupeLeadingMiddlewares: true
});
function implementerInternal(contract, config, middlewares) {
	if (isContractProcedure(contract)) return new Builder({
		...contract["~orpc"],
		config,
		middlewares,
		inputValidationIndex: fallbackConfig("initialInputValidationIndex", config?.initialInputValidationIndex) + middlewares.length,
		outputValidationIndex: fallbackConfig("initialOutputValidationIndex", config?.initialOutputValidationIndex) + middlewares.length,
		dedupeLeadingMiddlewares: fallbackConfig("dedupeLeadingMiddlewares", config.dedupeLeadingMiddlewares)
	});
	return new Proxy(contract, { get: (target, key) => {
		if (typeof key !== "string") return Reflect.get(target, key);
		let method;
		if (key === "middleware") method = (mid) => decorateMiddleware(mid);
		else if (key === "use") method = (mid) => {
			return implementerInternal(contract, config, addMiddleware(middlewares, mid));
		};
		else if (key === "router") method = (router) => {
			return setHiddenRouterContract(enhanceRouter(router, {
				middlewares,
				errorMap: {},
				prefix: void 0,
				tags: void 0,
				dedupeLeadingMiddlewares: fallbackConfig("dedupeLeadingMiddlewares", config.dedupeLeadingMiddlewares)
			}), contract);
		};
		else if (key === "lazy") method = (loader) => {
			return setHiddenRouterContract(enhanceRouter(lazy(loader), {
				middlewares,
				errorMap: {},
				prefix: void 0,
				tags: void 0,
				dedupeLeadingMiddlewares: fallbackConfig("dedupeLeadingMiddlewares", config.dedupeLeadingMiddlewares)
			}), contract);
		};
		const next = getContractRouter(target, [key]);
		if (!next) return method ?? next;
		const nextImpl = implementerInternal(next, config, middlewares);
		if (method) return new Proxy(method, { get(_, key2) {
			return Reflect.get(nextImpl, key2);
		} });
		return nextImpl;
	} });
}
function implement(contract, config = {}) {
	const implInternal = implementerInternal(contract, config, []);
	const impl = new Proxy(implInternal, { get: (target, key) => {
		let method;
		if (key === "$context") method = () => impl;
		else if (key === "$config") method = (config2) => implement(contract, config2);
		const next = Reflect.get(target, key);
		if (!method || !next || typeof next !== "function" && typeof next !== "object") return method || next;
		return new Proxy(method, { get(_, key2) {
			return Reflect.get(next, key2);
		} });
	} });
	return impl;
}
var NullProtoObj = /* @__PURE__ */ (() => {
	const e = function() {};
	return e.prototype = Object.create(null), Object.freeze(e.prototype), e;
})();
/**
* Create a new router context.
*/
function createRouter() {
	return {
		root: { key: "" },
		static: new NullProtoObj()
	};
}
function splitPath(path) {
	const [_, ...s] = path.split("/");
	return s[s.length - 1] === "" ? s.slice(0, -1) : s;
}
function getMatchParams(segments, paramsMap) {
	const params = new NullProtoObj();
	for (const [index, name] of paramsMap) {
		const segment = index < 0 ? segments.slice(-(index + 1)).join("/") : segments[index];
		if (typeof name === "string") params[name] = segment;
		else {
			const match = segment.match(name);
			if (match) for (const key in match.groups) params[key] = match.groups[key];
		}
	}
	return params;
}
/**
* Add a route to the router context.
*/
function addRoute(ctx, method = "", path, data) {
	method = method.toUpperCase();
	if (path.charCodeAt(0) !== 47) path = `/${path}`;
	path = path.replace(/\\:/g, "%3A");
	const segments = splitPath(path);
	let node = ctx.root;
	let _unnamedParamIndex = 0;
	const paramsMap = [];
	const paramsRegexp = [];
	for (let i = 0; i < segments.length; i++) {
		let segment = segments[i];
		if (segment.startsWith("**")) {
			if (!node.wildcard) node.wildcard = { key: "**" };
			node = node.wildcard;
			paramsMap.push([
				-(i + 1),
				segment.split(":")[1] || "_",
				segment.length === 2
			]);
			break;
		}
		if (segment === "*" || segment.includes(":")) {
			if (!node.param) node.param = { key: "*" };
			node = node.param;
			if (segment === "*") paramsMap.push([
				i,
				`_${_unnamedParamIndex++}`,
				true
			]);
			else if (segment.includes(":", 1)) {
				const regexp = getParamRegexp(segment);
				paramsRegexp[i] = regexp;
				node.hasRegexParam = true;
				paramsMap.push([
					i,
					regexp,
					false
				]);
			} else paramsMap.push([
				i,
				segment.slice(1),
				false
			]);
			continue;
		}
		if (segment === "\\*") segment = segments[i] = "*";
		else if (segment === "\\*\\*") segment = segments[i] = "**";
		const child = node.static?.[segment];
		if (child) node = child;
		else {
			const staticNode = { key: segment };
			if (!node.static) node.static = new NullProtoObj();
			node.static[segment] = staticNode;
			node = staticNode;
		}
	}
	const hasParams = paramsMap.length > 0;
	if (!node.methods) node.methods = new NullProtoObj();
	node.methods[method] ??= [];
	node.methods[method].push({
		data: data || null,
		paramsRegexp,
		paramsMap: hasParams ? paramsMap : void 0
	});
	if (!hasParams) ctx.static["/" + segments.join("/")] = node;
}
function getParamRegexp(segment) {
	const regex = segment.replace(/:(\w+)/g, (_, id) => `(?<${id}>[^/]+)`).replace(/\./g, "\\.");
	return /* @__PURE__ */ new RegExp(`^${regex}$`);
}
/**
* Find a route by path.
*/
function findRoute(ctx, method = "", path, opts) {
	if (path.charCodeAt(path.length - 1) === 47) path = path.slice(0, -1);
	const staticNode = ctx.static[path];
	if (staticNode && staticNode.methods) {
		const staticMatch = staticNode.methods[method] || staticNode.methods[""];
		if (staticMatch !== void 0) return staticMatch[0];
	}
	const segments = splitPath(path);
	const match = _lookupTree(ctx, ctx.root, method, segments, 0)?.[0];
	if (match === void 0) return;
	if (opts?.params === false) return match;
	return {
		data: match.data,
		params: match.paramsMap ? getMatchParams(segments, match.paramsMap) : void 0
	};
}
function _lookupTree(ctx, node, method, segments, index) {
	if (index === segments.length) {
		if (node.methods) {
			const match = node.methods[method] || node.methods[""];
			if (match) return match;
		}
		if (node.param && node.param.methods) {
			const match = node.param.methods[method] || node.param.methods[""];
			if (match) {
				const pMap = match[0].paramsMap;
				if (pMap?.[pMap?.length - 1]?.[2]) return match;
			}
		}
		if (node.wildcard && node.wildcard.methods) {
			const match = node.wildcard.methods[method] || node.wildcard.methods[""];
			if (match) {
				const pMap = match[0].paramsMap;
				if (pMap?.[pMap?.length - 1]?.[2]) return match;
			}
		}
		return;
	}
	const segment = segments[index];
	if (node.static) {
		const staticChild = node.static[segment];
		if (staticChild) {
			const match = _lookupTree(ctx, staticChild, method, segments, index + 1);
			if (match) return match;
		}
	}
	if (node.param) {
		const match = _lookupTree(ctx, node.param, method, segments, index + 1);
		if (match) {
			if (node.param.hasRegexParam) {
				const exactMatch = match.find((m) => m.paramsRegexp[index]?.test(segment)) || match.find((m) => !m.paramsRegexp[index]);
				return exactMatch ? [exactMatch] : void 0;
			}
			return match;
		}
	}
	if (node.wildcard && node.wildcard.methods) return node.wildcard.methods[method] || node.wildcard.methods[""];
}
var StandardOpenAPICodec = class {
	constructor(serializer, options = {}) {
		this.serializer = serializer;
		this.customErrorResponseBodyEncoder = options.customErrorResponseBodyEncoder;
	}
	customErrorResponseBodyEncoder;
	async decode(request, params, procedure) {
		if (fallbackContractConfig("defaultInputStructure", procedure["~orpc"].route.inputStructure) === "compact") {
			const data = request.method === "GET" ? this.serializer.deserialize(request.url.searchParams) : this.serializer.deserialize(await request.body());
			if (data === void 0) return params;
			if (isObject(data)) return {
				...params,
				...data
			};
			return data;
		}
		const deserializeSearchParams = () => {
			return this.serializer.deserialize(request.url.searchParams);
		};
		return {
			params,
			get query() {
				const value = deserializeSearchParams();
				Object.defineProperty(this, "query", {
					value,
					writable: true
				});
				return value;
			},
			set query(value) {
				Object.defineProperty(this, "query", {
					value,
					writable: true
				});
			},
			headers: request.headers,
			body: this.serializer.deserialize(await request.body())
		};
	}
	encode(output, procedure) {
		const successStatus = fallbackContractConfig("defaultSuccessStatus", procedure["~orpc"].route.successStatus);
		if (fallbackContractConfig("defaultOutputStructure", procedure["~orpc"].route.outputStructure) === "compact") {
			if (output instanceof ReadableStream) return {
				status: successStatus,
				headers: {},
				body: output
			};
			return {
				status: successStatus,
				headers: {},
				body: this.serializer.serialize(output)
			};
		}
		if (!this.#isDetailedOutput(output)) throw new Error(`
        Invalid "detailed" output structure:
        \u2022 Expected an object with optional properties:
          - status (number 200-399)
          - headers (Record<string, string | string[]>)
          - body (any)
        \u2022 No extra keys allowed.

        Actual value:
          ${stringifyJSON(output)}
      `);
		if (output.body instanceof ReadableStream) return {
			status: output.status ?? successStatus,
			headers: output.headers ?? {},
			body: output.body
		};
		return {
			status: output.status ?? successStatus,
			headers: output.headers ?? {},
			body: this.serializer.serialize(output.body)
		};
	}
	encodeError(error) {
		const body = this.customErrorResponseBodyEncoder?.(error) ?? error.toJSON();
		return {
			status: error.status,
			headers: {},
			body: this.serializer.serialize(body, { outputFormat: "plain" })
		};
	}
	#isDetailedOutput(output) {
		if (!isObject(output)) return false;
		if (output.headers && !isObject(output.headers)) return false;
		if (output.status !== void 0 && (typeof output.status !== "number" || !Number.isInteger(output.status) || isORPCErrorStatus(output.status))) return false;
		return true;
	}
};
function toRou3Pattern(path) {
	return standardizeHTTPPath(path).replace(/\/\{\+([^}]+)\}/g, "/**:$1").replace(/\/\{([^}]+)\}/g, "/:$1");
}
function decodeParams(params) {
	return Object.fromEntries(Object.entries(params).map(([key, value]) => [key, tryDecodeURIComponent(value)]));
}
var StandardOpenAPIMatcher = class {
	filter;
	tree = createRouter();
	pendingRouters = [];
	constructor(options = {}) {
		this.filter = options.filter ?? true;
	}
	init(router, path = []) {
		const laziedOptions = traverseContractProcedures({
			router,
			path
		}, (traverseOptions) => {
			if (!value(this.filter, traverseOptions)) return;
			const { path: path2, contract } = traverseOptions;
			const method = fallbackContractConfig("defaultMethod", contract["~orpc"].route.method);
			const httpPath = toRou3Pattern(contract["~orpc"].route.path ?? toHttpPath(path2));
			if (isProcedure(contract)) addRoute(this.tree, method, httpPath, {
				path: path2,
				contract,
				procedure: contract,
				router
			});
			else addRoute(this.tree, method, httpPath, {
				path: path2,
				contract,
				procedure: void 0,
				router
			});
		});
		this.pendingRouters.push(...laziedOptions.map((option) => ({
			...option,
			httpPathPrefix: toHttpPath(option.path),
			laziedPrefix: getLazyMeta(option.router).prefix
		})));
	}
	async match(method, pathname) {
		while (true) {
			const pendingRouter = this.pendingRouters.find((pendingRouter2) => !pendingRouter2.laziedPrefix || pathname.startsWith(pendingRouter2.laziedPrefix) || pathname.startsWith(pendingRouter2.httpPathPrefix));
			if (!pendingRouter) break;
			pendingRouter.initPromise ??= unlazy(pendingRouter.router).then(({ default: router }) => {
				this.init(router, pendingRouter.path);
				this.pendingRouters.splice(this.pendingRouters.indexOf(pendingRouter), 1);
			}).catch((error) => {
				pendingRouter.initPromise = void 0;
				throw error;
			});
			await pendingRouter.initPromise;
		}
		const match = findRoute(this.tree, method, pathname);
		if (!match) return;
		if (!match.data.procedure) {
			const { default: maybeProcedure } = await unlazy(getRouter(match.data.router, match.data.path));
			if (!isProcedure(maybeProcedure)) throw new Error(`
          [Contract-First] Missing or invalid implementation for procedure at path: ${toHttpPath(match.data.path)}.
          Ensure that the procedure is correctly defined and matches the expected contract.
        `);
			match.data.procedure = createContractedProcedure(maybeProcedure, match.data.contract);
		}
		return {
			path: match.data.path,
			procedure: match.data.procedure,
			params: match.params ? decodeParams(match.params) : void 0
		};
	}
};
var StandardOpenAPIHandler = class extends StandardHandler {
	constructor(router, options) {
		const serializer = new StandardOpenAPISerializer(new StandardOpenAPIJsonSerializer(options), new StandardBracketNotationSerializer(options));
		const matcher = new StandardOpenAPIMatcher(options);
		const codec = new StandardOpenAPICodec(serializer, options);
		super(router, matcher, codec, options);
	}
};
var OpenAPIHandler = class extends FetchHandler {
	constructor(router, options = {}) {
		super(new StandardOpenAPIHandler(router, options), options);
	}
};
/** Authorize and audit an invocation before returning its value or stream. */
async function invokeProcedure(call, next, options) {
	const audit = call.access.audit ? options.audit : void 0;
	let started = false;
	try {
		if (audit) await recordAudit({
			call,
			outcome: "started"
		}, audit);
		started = true;
		if (call.access.authentication !== "public" || call.access.permission !== null) await options.authorize(call);
		const result = await next();
		if (result !== null && typeof result === "object" && Symbol.asyncIterator in result) return streamProcedure(result, call, audit);
		if (audit) await recordAudit({
			call,
			outcome: "succeeded"
		}, audit);
		return result;
	} catch (error) {
		if (started && audit) await recordAudit({
			call,
			outcome: "failed",
			error
		}, audit);
		throw reportError(error);
	}
}
/** Report stream failures and record declared audit outcomes. */
function streamProcedure(stream, call, audit) {
	let outcome = "cancelled";
	let failure;
	const iterator = stream[Symbol.asyncIterator]();
	return new AsyncIteratorClass(async () => {
		try {
			const result = await iterator.next();
			if (result.done) outcome = "succeeded";
			return result;
		} catch (error) {
			outcome = "failed";
			failure = error;
			throw reportError(error);
		}
	}, async (reason) => {
		try {
			if (reason !== "next") await iterator.return?.();
		} catch (error) {
			outcome = "failed";
			failure = error;
			throw reportError(error);
		} finally {
			if (audit) await recordAudit({
				call,
				outcome,
				error: failure
			}, audit);
		}
	});
}
/** Record an audit event and preserve any preceding application failure. */
async function recordAudit(event, audit) {
	try {
		await audit(event);
	} catch (error) {
		throw reportError(event.outcome === "failed" ? new AggregateError([event.error, error], "Procedure failure audit failed.") : error);
	}
}
/** Record complete RPC calls, including streamed results. */
var ServiceTelemetry = class {
	/** Call durations in seconds. */
	#duration;
	/** The RPC span kind. */
	#kind;
	/** Connect tracing and metrics to the host's providers. */
	constructor(kind) {
		instrumentService();
		this.#kind = kind === "client" ? SpanKind.CLIENT : SpanKind.SERVER;
		this.#duration = scope(package_default).meter.createHistogram(`rpc.${kind}.call.duration`, {
			unit: "s",
			advice: { explicitBucketBoundaries: [
				.005,
				.01,
				.025,
				.05,
				.075,
				.1,
				.25,
				.5,
				.75,
				1,
				2.5,
				5,
				7.5,
				10
			] }
		});
	}
	/** Measure a call without retaining its inputs or results. */
	invoke(path, next) {
		const attributes = {
			"rpc.system.name": "orpc",
			"rpc.method": path.join("/")
		};
		return scope(package_default).tracer.startActiveSpan(path.join("/"), {
			kind: this.#kind,
			attributes
		}, (span) => this.#invoke(next, attributes, span));
	}
	/** Keep the span open until the value or stream completes. */
	async #invoke(next, attributes, span) {
		const started = performance.now();
		const active = context.active();
		try {
			const result = await next();
			if (result !== null && typeof result === "object" && Symbol.asyncIterator in result) return this.#watch(result, started, attributes, span, active);
			this.#record(started, attributes, span);
			return result;
		} catch (error) {
			this.#record(started, attributes, span, error);
			throw error;
		}
	}
	/** Retain timing until a consumer completes or cancels the stream. */
	#watch(stream, started, attributes, span, active) {
		let failure;
		const iterator = stream[Symbol.asyncIterator]();
		return new AsyncIteratorClass(async () => {
			try {
				return await context.with(active, () => iterator.next());
			} catch (error) {
				failure = error;
				throw error;
			}
		}, async (reason) => {
			try {
				if (reason !== "next") {
					attributes["error.type"] = "cancelled";
					await context.with(active, () => iterator.return?.());
				}
			} catch (error) {
				failure = error;
				throw error;
			} finally {
				this.#record(started, attributes, span, failure);
			}
		});
	}
	/** Record bounded failure labels and elapsed seconds. */
	#record(started, attributes, span, error) {
		if (error !== void 0) attributes["error.type"] = error instanceof ORPCError ? String(error.status) : "internal";
		if (attributes["error.type"]) span.setStatus({ code: SpanStatusCode.ERROR });
		span.setAttributes(attributes);
		span.end();
		this.#duration.record((performance.now() - started) / 1e3, attributes);
	}
};
/** Enable procedure and HTTP tracing through the application's telemetry providers. */
function instrumentService() {
	setGlobalOtelConfig({
		tracer: trace.getTracer("@destack/service"),
		trace,
		context,
		propagation
	});
}
/** Dispatch Fetch requests to service procedures using their HTTP routes. */
var ServiceHandler = class ServiceHandler extends OpenAPIHandler {
	/** Readiness shared with the hosting lifecycle. */
	health;
	/** Configure HTTP handling and extract trace context for each request. */
	constructor(router, options) {
		ServiceHandler.#checkAccess(router, options, /* @__PURE__ */ new Set());
		const telemetry = new ServiceTelemetry("server");
		super(router, {
			...options,
			clientInterceptors: [
				({ path, next }) => telemetry.invoke(path, next),
				async ({ next, procedure, path, input, context, signal }) => {
					return invokeProcedure({
						access: ProcedureAccess.parse(procedure["~orpc"].meta),
						path,
						input,
						context,
						signal
					}, next, options);
				},
				...options.clientInterceptors ?? []
			],
			adapterInterceptors: [({ request, next }) => context.with(extractContext(request.headers), next), ...options.adapterInterceptors ?? []]
		});
		this.health = options.health;
	}
	/** Answer probes through the same HTTP path on every host. */
	async handle(...args) {
		const response = this.health.probe(args[0]);
		if (response) return {
			matched: true,
			response
		};
		return super.handle(...args);
	}
	/** Require enforcement for every declared procedure before hosting. */
	static #checkAccess(router, options, ancestors) {
		if (isLazy(router)) throw new TypeError("service procedures must be declared before hosting");
		if (ancestors.has(router)) throw new TypeError("service routers must not contain cycles");
		if (isProcedure(router)) {
			const access = ProcedureAccess.parse(router["~orpc"].meta);
			if ((access.authentication !== "public" || access.permission !== null) && !options.authorize) throw new TypeError("protected procedures require authorization");
			if (access.audit && !options.audit) throw new TypeError("audited procedures require audit recording");
		} else {
			ancestors.add(router);
			for (const child of Object.values(router)) ServiceHandler.#checkAccess(child, options, ancestors);
			ancestors.delete(router);
		}
	}
};
/** A current value with bounded, coalesced change notifications. */
var Watch = class {
	/** The latest value. */
	#value;
	/** The current change number. */
	#revision = 0;
	/** Whether the producer has stopped. */
	#closed = false;
	/** Subscribers waiting for a changed value. */
	#subscribers = /* @__PURE__ */ new Set();
	/** Start with an initial value. */
	constructor(value) {
		this.#value = value;
	}
	/** Read the latest published value. */
	get value() {
		return this.#value;
	}
	/** Replace the value and wake waiting subscribers. */
	set(value) {
		if (this.#closed) throw new Error("watch is closed");
		this.#value = value;
		this.#revision++;
		for (const wake of this.#subscribers) wake();
	}
	/** Stop waiting subscribers. */
	close() {
		this.#closed = true;
		for (const wake of this.#subscribers) wake();
	}
	/** Yield the current value, then the latest value after each observed change. */
	async *watch(signal) {
		let revision = -1;
		while (!this.#closed && !signal?.aborted) {
			const pending = Promise.withResolvers();
			const wake = () => pending.resolve();
			this.#subscribers.add(wake);
			signal?.addEventListener("abort", wake, { once: true });
			try {
				if (revision !== this.#revision) {
					revision = this.#revision;
					yield this.value;
				} else await pending.promise;
			} finally {
				this.#subscribers.delete(wake);
				signal?.removeEventListener("abort", wake);
			}
		}
	}
};
/** Current service health and coalesced change notifications. */
var Health = class {
	/** The declared service name. */
	name;
	/** The current readiness state. */
	#status = new Watch("starting");
	/** Initialize health before the host starts accepting work. */
	constructor(name) {
		this.name = name;
	}
	/** Read the current status. */
	get status() {
		return this.#status.value;
	}
	/** Publish readiness after initialization, dependency changes, or shutdown. */
	set(status) {
		if (status !== this.status) this.#status.set(status);
	}
	/** Describe current service health. */
	check() {
		return {
			name: this.name,
			status: this.status
		};
	}
	/** Yield current health and changes until shutdown or subscriber cancellation. */
	async *watch(signal) {
		for await (const status of this.#status.watch(signal)) {
			yield {
				name: this.name,
				status
			};
			if (status === "draining" || status === "stopped") return;
		}
	}
	/** Answer conventional liveness and readiness probes, or return undefined for other paths. */
	probe(request) {
		const path = new URL(request.url).pathname;
		if (path !== "/livez" && path !== "/readyz") return;
		if (request.method !== "GET" && request.method !== "HEAD") return new Response(null, {
			status: 405,
			headers: { allow: "GET, HEAD" }
		});
		const ready = path === "/livez" ? this.status !== "stopped" : this.status === "serving";
		return new Response(request.method === "HEAD" ? null : ready ? "ok\n" : "unavailable\n", {
			status: ready ? 200 : 503,
			headers: {
				"content-type": "text/plain; charset=utf-8",
				"cache-control": "no-store"
			}
		});
	}
};
/** Service readiness visible to clients and load balancers. */
var HealthStatus = _enum([
	"starting",
	"serving",
	"not-serving",
	"draining",
	"stopped"
]);
strictObject({
	/** Declared service name. */
	name: string().min(1),
	/** Current readiness. */
	status: HealthStatus
});
/** Package instruments initialized from build-injected metadata. */
var instruments = scope({ "package": {
	"name": "@destack/build-service-fixture",
	"version": "2026.9.0"
} }.package);
/** The shared application database. */
var database = defineDatabase({
	name: "main",
	spec: { dialect: "sqlite" }
});
/** The application's secret collection. */
var vault = defineVault({
	name: "credentials",
	spec: {}
});
/** The secret selected during installation. */
var token = defineSecret({ name: "mail-token" });
/** The public notes API. */
var router = { list: defineProcedure({
	authentication: "public",
	permission: null,
	audit: false
}).route({
	method: "GET",
	path: "/notes"
}).output(strictObject({ path: string() })) };
/** The public HTTP service. */
var service = defineService({
	name: "notes",
	version: 1,
	protocol: "http",
	handler: "fetch"
}, router);
/** The typed notes implementation. */
var implementation = implement(router);
/** Readiness of the initialized HTTP handler. */
var health = new Health("notes");
health.set("serving");
/** The HTTP dispatcher used by both supported runtimes. */
var handler = new ServiceHandler({ list: implementation.list.handler(() => ({ path: "/notes" })) }, { health });
/** Respond through the emitted handler. */
async function fetch(request) {
	instruments.logger.emit({ body: "Request received" });
	return (await handler.handle(request)).response ?? new Response("Not found", { status: 404 });
}
/** The daily reminder schedule. */
var reminders = defineSchedule({
	name: "reminders",
	version: 1,
	handler: "remind",
	timing: "cron",
	cron: "0 9 * * *",
	timezone: "UTC",
	concurrency: "forbid",
	deadline: 6e4
});
/** Repeat from a fixed first occurrence. */
var refresh = defineSchedule({
	name: "refresh",
	version: 1,
	handler: "remind",
	timing: "interval",
	interval: 3e5,
	startsAt: 18e11,
	endsAt: 18000864e5,
	concurrency: "forbid",
	deadline: 6e4
});
/** Send a reminder at one specified time. */
var appointment = defineSchedule({
	name: "appointment",
	version: 1,
	handler: "remind",
	timing: "once",
	startsAt: 18e11,
	concurrency: "allow",
	deadline: 6e4
});
/** Return the scheduled occurrence identifier. */
function remind(occurrence) {
	return occurrence.id;
}
export { appointment, database, fetch, refresh, remind, reminders, router, service, token, vault };

//# sourceMappingURL=server-CpGptqpN.js.map