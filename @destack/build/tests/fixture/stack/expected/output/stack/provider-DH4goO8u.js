import { $ as and, A as compileExpression, At as fillPlaceholders, B as bigint, Bt as TableColumns, C as literal$1, Ct as DrizzleError, D as Order, Dt as Placeholder, E as Condition, Et as Param, F as boundedName, Ft as Subquery, G as numeric$1, Gt as OriginalColumn, H as boolean, Ht as getTableName, I as constraintName, It as WithSubquery, J as timestamp, Jt as canonicalize, K as real$1, Kt as entityKind, L as hashName, Lt as IsAlias, M as TABLE, Mt as noopDecoder, N as Table$1, Nt as sql, O as Expression, Ot as SQL, P as expandTrees, Pt as ViewBaseConfig, Q as wraps, R as primaryKey$1, Rt as OriginalName, S as condition, St as or, T as v7, Tt as TransactionRollbackError, U as integer$1, Ut as TableName, V as binary, Vt as TableSchema, W as json, Wt as Column, X as assertNever, Y as DatabaseError, Yt as digest, Z as classifyError, _ as LOG_EPOCH, _n as literal, _t as notBetween, a as createState, at as exists, b as LOG_TRANSACTION, bn as strictObject, bt as notInArray, c as logOf, cn as defineSchema, ct as ilike, d as unappliedTables, dn as boolean$1, dt as isNull, et as arrayContained, f as writeState, ft as like, g as LOG_COPYING, gn as json$1, gt as not, h as LOG_CHANNEL, ht as ne, i as STATE, it as eq, j as dialectSQL, jt as isSQLWrapper, k as Key, kt as View, l as readState, ln as _enum, lt as inArray, m as LOG, mn as discriminatedUnion, mt as lte, nt as arrayOverlaps, o as declareState, on as Version, ot as gt, p as PARAMETER_BUDGET, pt as lt, q as text$1, qt as is, r as DatabaseState, rt as between, s as deleteState, st as gte, t as Database$1, tt as arrayContains, u as readTables, un as array, ut as isNotNull, v as LOG_HOLD, vn as number, vt as notExists, w as quote, wt as DrizzleQueryError, x as createEpoch, xn as string, xt as notLike, y as LOG_HORIZON, yn as record, yt as notIlike, z as Column$1, zt as Table } from "./database-B2GWCpI9.js";
import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Database } from "bun:sqlite";
/** The JSON value types a schema may accept. */
var TYPES = [
	"string",
	"number",
	"integer",
	"boolean",
	"null",
	"array",
	"object"
];
/** The keywords that describe a schema without constraining its values. */
var ANNOTATIONS = /* @__PURE__ */ new Set([
	"$schema",
	"$id",
	"$comment",
	"$defs",
	"definitions",
	"id",
	"title",
	"description",
	"deprecated",
	"examples",
	"default",
	"readOnly",
	"writeOnly"
]);
/** The keywords the comparison reads, beside annotations. */
var KEYWORDS$1 = /* @__PURE__ */ new Set([
	"$ref",
	"type",
	"const",
	"enum",
	"anyOf",
	"oneOf",
	"allOf",
	"minLength",
	"maxLength",
	"pattern",
	"format",
	"minimum",
	"maximum",
	"exclusiveMinimum",
	"exclusiveMaximum",
	"multipleOf",
	"items",
	"prefixItems",
	"minItems",
	"maxItems",
	"properties",
	"required",
	"additionalProperties",
	"propertyNames",
	"minProperties",
	"maxProperties"
]);
/** Compare two JSON Schema Draft 2020-12 descriptions by the values each accepts. */
function compareJsonSchemas(before, after) {
	const isWider = new Inclusion(after, before).includes(after, before);
	const isNarrower = new Inclusion(before, after).includes(before, after);
	return isWider && isNarrower ? "same" : isWider ? "wider" : isNarrower ? "narrower" : "incompatible";
}
/** A proof that one schema accepts every value another accepts, answering no when it cannot prove it. */
var Inclusion = class {
	/** The document the including schema's references resolve in. */
	#outerRoot;
	/** The document the included schema's references resolve in. */
	#innerRoot;
	/** The node pairs under proof, assumed included when a recursive schema reaches them again. */
	#assumed = /* @__PURE__ */ new Map();
	/** Prove inclusions between nodes of two documents. */
	constructor(outerRoot, innerRoot) {
		this.#outerRoot = outerRoot;
		this.#innerRoot = innerRoot;
	}
	/** Report whether the outer node accepts every value the inner node accepts. */
	includes(outer, inner) {
		const outerNode = resolve(outer, this.#outerRoot);
		const innerNode = resolve(inner, this.#innerRoot);
		const pairs = this.#assumed.get(outerNode) ?? /* @__PURE__ */ new Set();
		if (pairs.has(innerNode)) return true;
		this.#assumed.set(outerNode, pairs.add(innerNode));
		const isIncluded = this.#includesNode(outerNode, innerNode);
		pairs.delete(innerNode);
		return isIncluded;
	}
	/** Report whether one resolved node accepts every value another accepts. */
	#includesNode(outerNode, innerNode) {
		if (innerNode === false || isAny(outerNode)) return true;
		else if (outerNode === false) return false;
		const outerKeywords = outerNode === true ? {} : outerNode;
		const innerKeywords = innerNode === true ? {} : innerNode;
		return this.#includesKeywords(outerKeywords, innerKeywords);
	}
	/** Report whether outer keywords accept every value inner keywords accept. */
	#includesKeywords(outer, inner) {
		const { allOf: outerAll, ...outerRest } = outer;
		const innerBranches = inner.anyOf ?? inner.oneOf;
		const outerBranches = outer.anyOf ?? outer.oneOf;
		if (outerAll !== void 0) return this.includes(outerRest, inner) && outerAll.every((part) => this.includes(part, inner));
		else if (inner.const !== void 0 || inner.enum !== void 0) return (inner.const === void 0 ? inner.enum : [inner.const]).every((value) => this.#accepts(outer, value));
		else if (innerBranches !== void 0) {
			const { anyOf: _any, oneOf: _one, ...rest } = inner;
			return innerBranches.every((branch) => this.includes(outer, { allOf: [rest, branch] }));
		} else if (Array.isArray(inner.type) && inner.type.length > 1) return inner.type.every((type) => this.includes(outer, {
			...inner,
			type
		}));
		else if (outerBranches !== void 0) {
			const { anyOf: _any, oneOf: _one, ...rest } = outer;
			return this.includes(rest, inner) && outerBranches.some((branch) => this.includes(branch, inner));
		} else if (inner.allOf !== void 0) {
			const { allOf, ...rest } = inner;
			return [rest, ...allOf].some((part) => this.includes(outer, part));
		}
		return this.#includesTyped(outer, inner);
	}
	/** Report whether single-alternative outer keywords accept every value of each inner type. */
	#includesTyped(outer, inner) {
		const isListed = outer.const !== void 0 || outer.enum !== void 0;
		const isUnreadable = Object.keys(outer).some((keyword) => !isKnown(keyword));
		if ((isListed || isUnreadable) && equals(outer, inner)) return true;
		else if (isListed || isUnreadable) return false;
		const outerTypes = typesOf(outer);
		return [...typesOf(inner)].every((type) => {
			if (!(outerTypes.has(type) || type === "integer" && outerTypes.has("number"))) return false;
			else if (type === "string") return includesString(outer, inner);
			else if (type === "number" || type === "integer") return includesNumber(outer, inner, type, outerTypes);
			else if (type === "array") return this.#includesArray(outer, inner);
			else if (type === "object") return this.#includesObject(outer, inner);
			return true;
		});
	}
	/** Report whether outer array keywords accept every inner array. */
	#includesArray(outer, inner) {
		if (!((outer.minItems ?? 0) <= (inner.minItems ?? 0) && innerMaxItems(inner) <= (outer.maxItems ?? Infinity))) return false;
		const outerPrefix = outer.prefixItems ?? [];
		const innerPrefix = inner.prefixItems ?? [];
		const positions = Math.max(outerPrefix.length, innerPrefix.length);
		for (let index = 0; index < positions; index += 1) {
			const outerItem = outerPrefix[index] ?? itemsOf(outer);
			const innerItem = innerPrefix[index] ?? itemsOf(inner);
			if (!this.includes(outerItem, innerItem)) return false;
		}
		return this.includes(itemsOf(outer), itemsOf(inner));
	}
	/** Report whether outer object keywords accept every inner object. */
	#includesObject(outer, inner) {
		const innerRequired = new Set(inner.required ?? []);
		if (!((outer.required ?? []).every((name) => innerRequired.has(name)) && (outer.minProperties ?? 0) <= (inner.minProperties ?? 0) && (inner.maxProperties ?? Infinity) <= (outer.maxProperties ?? Infinity))) return false;
		const outerProperties = outer.properties ?? {};
		const innerProperties = inner.properties ?? {};
		const outerRest = outer.additionalProperties ?? true;
		const innerRest = inner.additionalProperties ?? true;
		const names = /* @__PURE__ */ new Set([...Object.keys(outerProperties), ...Object.keys(innerProperties)]);
		for (const name of names) {
			const outerProperty = outerProperties[name] ?? outerRest;
			const innerProperty = innerProperties[name] ?? innerRest;
			if (!(outerProperties[name] !== void 0 || outer.propertyNames === void 0 || this.#accepts(outer.propertyNames, name)) || !this.includes(outerProperty, innerProperty)) return false;
		}
		const outerNames = outer.propertyNames ?? true;
		const innerNames = inner.propertyNames ?? { type: "string" };
		return innerRest === false || this.includes(outerRest, innerRest) && this.includes(outerNames, innerNames);
	}
	/** Report whether an outer node accepts a JSON value. */
	#accepts(outer, value) {
		const node = resolve(outer, this.#outerRoot);
		if (typeof node === "boolean") return node;
		const branches = node.anyOf ?? node.oneOf;
		const isListed = (node.const === void 0 || equals(node.const, value)) && (node.enum === void 0 || node.enum.some((entry) => equals(entry, value)));
		const isCombined = (node.allOf ?? []).every((part) => this.#accepts(part, value)) && (branches === void 0 || branches.some((branch) => this.#accepts(branch, value)));
		if (!isListed || !isCombined) return false;
		const type = typeOf(value);
		const types = typesOf(node);
		const isTyped = types.has(type) || type === "integer" && types.has("number");
		if (Object.keys(node).some((keyword) => !isKnown(keyword)) || !isTyped) return false;
		if (typeof value === "string") return acceptsString(node, value);
		else if (typeof value === "number") return acceptsNumber(node, value);
		else if (Array.isArray(value)) return this.#acceptsArray(node, value);
		else if (typeof value === "object" && value !== null) return this.#acceptsObject(node, value);
		return true;
	}
	/** Report whether array keywords accept an array. */
	#acceptsArray(node, value) {
		const prefix = node.prefixItems ?? [];
		return value.length >= (node.minItems ?? 0) && value.length <= (node.maxItems ?? Infinity) && value.every((item, index) => this.#accepts(prefix[index] ?? itemsOf(node), item));
	}
	/** Report whether object keywords accept an object. */
	#acceptsObject(node, value) {
		const names = Object.keys(value);
		const properties = node.properties ?? {};
		return (node.required ?? []).every((name) => name in value) && names.length >= (node.minProperties ?? 0) && names.length <= (node.maxProperties ?? Infinity) && names.every((name) => (properties[name] !== void 0 || this.#accepts(node.propertyNames ?? true, name)) && this.#accepts(properties[name] ?? node.additionalProperties ?? true, value[name]));
	}
};
/** Follow a node's local references within its document. */
function resolve(node, root) {
	if (typeof node === "boolean" || node.$ref === void 0) return node;
	const { $ref: reference, ...rest } = node;
	const match = /^#(?:\/(\$defs|definitions)\/(.+))?$/.exec(reference);
	const target = match?.[1] === void 0 ? root : root[match[1]];
	const found = match?.[2] === void 0 ? target : target?.[decodePointer(match[2])];
	if (match === null || found === void 0) throw new TypeError(`unresolved schema reference: ${reference}`);
	return Object.keys(rest).every((keyword) => ANNOTATIONS.has(keyword)) || found === true ? resolve(found, root) : found === false ? false : { allOf: [found, rest] };
}
/** Decode one JSON Pointer segment. */
function decodePointer(segment) {
	return segment.replaceAll("~1", "/").replaceAll("~0", "~");
}
/** Report whether a node accepts every value. */
function isAny(node) {
	return node === true || node !== false && Object.keys(node).every((key) => ANNOTATIONS.has(key));
}
/** Report whether the comparison reads or ignores a keyword. */
function isKnown(keyword) {
	return KEYWORDS$1.has(keyword) || ANNOTATIONS.has(keyword);
}
/** Read the types a node accepts, every type when it lists none. */
function typesOf(node) {
	if (node.type !== void 0) return new Set(typeof node.type === "string" ? [node.type] : node.type);
	return new Set(TYPES.filter((type) => type !== "integer"));
}
/** Read a JSON value's type, integers apart from other numbers. */
function typeOf(value) {
	if (value === null) return "null";
	else if (Array.isArray(value)) return "array";
	else if (typeof value === "number") return Number.isInteger(value) ? "integer" : "number";
	return typeof value;
}
/** Report whether outer string keywords accept every inner string. */
function includesString(outer, inner) {
	return (outer.minLength ?? 0) <= (inner.minLength ?? 0) && (inner.maxLength ?? Infinity) <= (outer.maxLength ?? Infinity) && (outer.pattern === void 0 || outer.pattern === inner.pattern) && (outer.format === void 0 || outer.format === inner.format);
}
/** Report whether outer number keywords accept every inner number of a type. */
function includesNumber(outer, inner, type, outerTypes) {
	const isLower = holdsBound(lowerBound(outer), lowerBound(inner), -1);
	const isUpper = holdsBound(upperBound(outer), upperBound(inner), 1);
	const isIntegral = outerTypes.has("number") || type === "integer";
	const isMultiple = outer.multipleOf === void 0 || inner.multipleOf !== void 0 && Number.isInteger(inner.multipleOf / outer.multipleOf);
	return isLower && isUpper && isIntegral && isMultiple;
}
/** Report whether string keywords accept a string. */
function acceptsString(node, value) {
	const length = [...value].length;
	return length >= (node.minLength ?? 0) && length <= (node.maxLength ?? Infinity) && (node.pattern === void 0 || new RegExp(node.pattern, "u").test(value)) && node.format === void 0;
}
/** Report whether number keywords accept a number. */
function acceptsNumber(node, value) {
	return holdsBound(lowerBound(node), [value, false], -1) && holdsBound(upperBound(node), [value, false], 1) && (node.multipleOf === void 0 || Number.isInteger(value / node.multipleOf));
}
/** Read a node's lower number bound and whether it is exclusive. */
function lowerBound(node) {
	const inclusive = node.minimum ?? -Infinity;
	const exclusive = boundOf(node.exclusiveMinimum) ?? -Infinity;
	return exclusive >= inclusive ? [exclusive, exclusive !== -Infinity] : [inclusive, false];
}
/** Read a node's upper number bound and whether it is exclusive. */
function upperBound(node) {
	const inclusive = node.maximum ?? Infinity;
	const exclusive = boundOf(node.exclusiveMaximum) ?? Infinity;
	return exclusive <= inclusive ? [exclusive, exclusive !== Infinity] : [inclusive, false];
}
/** Read the schema of the array positions past the prefix, every value when absent. */
function itemsOf(node) {
	if (Array.isArray(node.items)) throw new TypeError("draft 4 tuple items are unsupported");
	return node.items ?? true;
}
/** Read an exclusive number bound, refusing the draft 4 boolean form. */
function boundOf(bound) {
	if (typeof bound === "boolean") throw new TypeError("draft 4 exclusive bounds are unsupported");
	return bound;
}
/** Read an inner array's greatest length, bounded by its fixed positions when it forbids the rest. */
function innerMaxItems(inner) {
	const fixed = inner.items === false ? (inner.prefixItems ?? []).length : Infinity;
	return Math.min(inner.maxItems ?? Infinity, fixed);
}
/** Report whether an outer bound admits everything an inner bound admits, on the side a direction points to. */
function holdsBound(outer, inner, direction) {
	const [outerValue, isOuterExclusive] = outer;
	const [innerValue, isInnerExclusive] = inner;
	return (innerValue - outerValue) * direction < 0 || innerValue === outerValue && (!isOuterExclusive || isInnerExclusive);
}
/** Compare two JSON values structurally. */
function equals(left, right) {
	return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}
/** Sort object keys so equal values serialize equally. */
function canonical(value) {
	if (Array.isArray(value)) return value.map(canonical);
	else if (typeof value === "object" && value !== null) return Object.fromEntries(Object.entries(value).sort(([left], [right]) => left < right ? -1 : 1).map(([key, entry]) => [key, canonical(entry)]));
	return value;
}
/** A desired state no plan can reach until its declarations change. */
var PlanError = class extends Error {
	/** What the declarations must change, by the part of the resource it concerns. */
	problems;
	/** Describe every problem at once. */
	constructor(problems) {
		super(problems.map((problem) => `${problem.target}: ${problem.detail}`).join("; "));
		this.name = "PlanError";
		this.problems = problems;
	}
};
/** How consequential a step is, from least to most. */
var Risk = defineSchema(_enum([
	"safe",
	"data-dependent",
	"backward-incompatible",
	"destructive"
]));
/** The changes taking a resource from its applied state to its desired state. */
var Plan = {
	classify,
	digest: digestSteps,
	schema: planSchema
};
/** Classify a plan by its most consequential step, safe when empty. */
function classify(plan) {
	return plan.steps.reduce((highest, next) => Risk.options.indexOf(next.risk) > Risk.options.indexOf(highest) ? next.risk : highest, "safe");
}
/** Digest a plan's reviewed steps, so an apply can require the plan a review saw. */
function digestSteps(plan) {
	return digest(plan.steps.map((step) => ({
		kind: step.kind,
		risk: step.risk,
		target: step.target,
		detail: step.detail
	})));
}
/** Plan a schema change: widening for backward readers, narrowing for forward ones, converting otherwise. */
function planSchema(change) {
	const { target, release, compatibility } = change;
	const compared = compareJsonSchemas(change.before, change.after);
	const isCompatible = compared === "same" || compared === (compatibility === "backward" ? "wider" : "narrower");
	if (compared === "same") return { steps: [] };
	if (isCompatible) return { steps: [{
		kind: compared,
		risk: "safe",
		target,
		detail: `${compared} ${target}`
	}] };
	else if (compatibility === "backward" && change.isConverted) return { steps: [{
		kind: "convert",
		risk: "data-dependent",
		target,
		detail: `convert ${target} to ${release}`
	}] };
	else if (compatibility === "backward") throw new PlanError([{
		target,
		detail: `declare a conversion for ${release}`
	}]);
	else return { steps: [{
		kind: compared,
		risk: "backward-incompatible",
		target,
		detail: `${compared} ${target}: earlier readers keep their release`
	}] };
}
var QueryPromise = class {
	static [entityKind] = "QueryPromise";
	[Symbol.toStringTag] = "QueryPromise";
	catch(onRejected) {
		return this.then(void 0, onRejected);
	}
	finally(onFinally) {
		return this.then((value) => {
			onFinally?.();
			return value;
		}, (reason) => {
			onFinally?.();
			throw reason;
		});
	}
	then(onFulfilled, onRejected) {
		return this.execute().then(onFulfilled, onRejected);
	}
};
/**
* Used in sorting, this specifies that the given
* column or expression should be sorted in ascending
* order. By the SQL standard, ascending order is the
* default, so it is not usually necessary to specify
* ascending sort order.
*
* ## Examples
*
* ```ts
* // Return cars, starting with the oldest models
* // and going in ascending order to the newest.
* db.select().from(cars)
*   .orderBy(asc(cars.year));
* ```
*
* @see desc to sort in descending order
*/
function asc(column) {
	return sql`${column} asc`;
}
/**
* Used in sorting, this specifies that the given
* column or expression should be sorted in descending
* order.
*
* ## Examples
*
* ```ts
* // Select users, with the most recently created
* // records coming first.
* db.select().from(users)
*   .orderBy(desc(users.createdAt));
* ```
*
* @see asc to sort in ascending order
*/
function desc(column) {
	return sql`${column} desc`;
}
var ColumnTableAliasProxyHandler = class {
	static [entityKind] = "ColumnTableAliasProxyHandler";
	constructor(table, ignoreColumnAlias) {
		this.table = table;
		this.ignoreColumnAlias = ignoreColumnAlias;
	}
	get(columnObj, prop) {
		if (prop === "table") return this.table;
		if (prop === "isAlias" && this.ignoreColumnAlias) return false;
		return columnObj[prop];
	}
};
var ViewSelectionAliasProxyHandler = class {
	static [entityKind] = "ViewSelectionAliasProxyHandler";
	constructor(view, selection, ignoreColumnAlias) {
		this.view = view;
		this.selection = selection;
		this.ignoreColumnAlias = ignoreColumnAlias;
	}
	get(selection, prop) {
		const value = selection[prop];
		if (is(value, Column)) return new Proxy(value, new ColumnTableAliasProxyHandler(this.view, this.ignoreColumnAlias));
		if (is(value, Subquery) || is(value, SQL) || is(value, SQL.Aliased) || isSQLWrapper(value) || typeof value !== "object" || value === null) return value;
		return new Proxy(value, this);
	}
};
var TableAliasProxyHandler = class {
	static [entityKind] = "TableAliasProxyHandler";
	constructor(alias, replaceOriginalName, ignoreColumnAlias) {
		this.alias = alias;
		this.replaceOriginalName = replaceOriginalName;
		this.ignoreColumnAlias = ignoreColumnAlias;
	}
	get(target, prop) {
		if (prop === Table.Symbol.IsAlias) return true;
		if (prop === Table.Symbol.Name) return this.alias;
		if (this.replaceOriginalName && prop === Table.Symbol.OriginalName) return this.alias;
		if (prop === ViewBaseConfig) return {
			...target[ViewBaseConfig],
			name: this.alias,
			isAlias: true,
			selectedFields: new Proxy(target[ViewBaseConfig].selectedFields, new ViewSelectionAliasProxyHandler(new Proxy(target, this), target[ViewBaseConfig].selectedFields, this.ignoreColumnAlias))
		};
		if (prop === Table.Symbol.Columns) {
			const columns = target[Table.Symbol.Columns];
			if (!columns) return columns;
			if (is(target, View)) return new Proxy(target[Table.Symbol.Columns], new ViewSelectionAliasProxyHandler(new Proxy(target, this), target[Table.Symbol.Columns], this.ignoreColumnAlias));
			const proxiedColumns = {};
			Object.keys(columns).map((key) => {
				proxiedColumns[key] = new Proxy(columns[key], new ColumnTableAliasProxyHandler(new Proxy(target, this), this.ignoreColumnAlias));
			});
			return proxiedColumns;
		}
		const value = target[prop];
		if (is(value, Column)) return new Proxy(value, new ColumnTableAliasProxyHandler(new Proxy(target, this), this.ignoreColumnAlias));
		return value;
	}
};
var ColumnAliasProxyHandler = class {
	static [entityKind] = "ColumnAliasProxyHandler";
	constructor(alias) {
		this.alias = alias;
	}
	get(target, prop) {
		if (prop === "isAlias") return true;
		if (prop === "name") return this.alias;
		if (prop === "keyAsName") return false;
		if (prop === OriginalColumn) return () => target;
		return target[prop];
	}
};
function aliasedTable(table, tableAlias) {
	return new Proxy(table, new TableAliasProxyHandler(tableAlias, false, false));
}
function aliasedColumn(column, alias) {
	return new Proxy(column, new ColumnAliasProxyHandler(alias));
}
Column.prototype.as = function(alias) {
	return aliasedColumn(this, alias);
};
function getOriginalColumnFromAlias(column) {
	return column[OriginalColumn]();
}
/** @internal bypass bundle-time filtering */
var FnConstructor = Object.getPrototypeOf(() => null).constructor;
/** @internal */
function makeJitQueryMapperInner(columns, joinsNotNullableMap = {}) {
	const preFn = [];
	const fn = [];
	fn.push(`const [ ${columns.map((_, i) => `c${i}`).join(", ")} ] = rows[i];`);
	const nullifyMap = {};
	const objectIds = {};
	const decodes = Array.from({ length: columns.length });
	for (let idx = 0; idx < columns.length; ++idx) {
		const { field, path, codec, arrayDimensions } = columns[idx];
		let decoder;
		let decoderStr;
		let decoderFieldDestructure;
		let isColumn = false;
		if (is(field, Column)) {
			isColumn = true;
			decoder = field;
			decoderFieldDestructure = `field: decoder${idx}`;
		} else if (is(field, SQL)) {
			decoder = field.decoder;
			decoderFieldDestructure = `field: { decoder: decoder${idx} }`;
		} else if (is(field, Subquery)) {
			decoder = field._.sql.decoder;
			decoderFieldDestructure = `field: { _: { sql: { decoder: decoder${idx} } } }`;
		} else {
			decoder = field.sql.decoder;
			decoderFieldDestructure = `field: { sql: { decoder: decoder${idx} } }`;
		}
		decoderStr = `decoder${idx}.mapFromDriverValue`;
		if (decoder.mapFromDriverValue.isNoop) decoderStr = "";
		if (decoderStr) preFn.push(`const { ${decoderFieldDestructure}${codec ? `, codec: codec${idx}` : ""} } = columns[${idx}];`);
		else if (codec) preFn.push(`const { codec: codec${idx} } = columns[${idx}];`);
		const colStr = `c${idx}`;
		let decodedValue = colStr;
		if (codec) decodedValue = `codec${idx}(${decodedValue}, ${arrayDimensions})`;
		if (decoderStr) decodedValue = `${decoderStr}(${decodedValue})`;
		decodes[idx] = colStr === decodedValue ? `${colStr}` : `${colStr} === null ? ${colStr} : ${decodedValue}`;
		if (path.length !== 2 || !isColumn) continue;
		if (objectIds[path[0]] === void 0) objectIds[path[0]] = [`c${idx}`];
		else objectIds[path[0]]?.push(`c${idx}`);
		const [objectName] = path;
		const tableName = getTableName(field.table);
		nullifyMap[objectName] = joinsNotNullableMap[tableName] ? false : typeof nullifyMap[objectName] === "string" ? nullifyMap[objectName] === tableName ? tableName : false : tableName;
	}
	fn.push(`mapped[i] = {`);
	let currentObjectPath = [];
	for (let idx = 0; idx < columns.length; ++idx) {
		const { path } = columns[idx];
		const jsonPath = path.map((e) => JSON.stringify(e));
		const decodedValue = decodes[idx];
		const objectPath = path.slice(0, -1);
		let commonLen = 0;
		while (commonLen < currentObjectPath.length && commonLen < objectPath.length && currentObjectPath[commonLen] === objectPath[commonLen]) commonLen++;
		for (let d = currentObjectPath.length - 1; d >= commonLen; --d) fn.push(`${"	".repeat(d + 1)}},`);
		for (let d = commonLen; d < objectPath.length; ++d) fn.push(`${"	".repeat(d + 1)}${jsonPath[d]}: ${d === 0 && objectPath.length === 1 && typeof nullifyMap[path[0]] === "string" ? `${objectIds[path[0]]?.map((c) => `${c} === null`).join(" && ")} ? null : {` : "{"}`);
		currentObjectPath = objectPath;
		fn.push(`${"	".repeat(path.length)}${jsonPath[path.length - 1]}: ${decodedValue},`);
	}
	for (let d = currentObjectPath.length - 1; d >= 0; --d) fn.push(`${"	".repeat(d + 1)}},`);
	fn.push(`};`);
	return `${preFn.length ? `${preFn.join("\n	")}\n\t` : ""}for (let i = 0; i < length; ++i) {
		${fn.join("\n		")}
	}`;
}
function makeJitQueryMapper(columns, joinsNotNullableMap) {
	const internals = `\t"use strict";
	const { columns } = this;
	const { length } = rows;
	const mapped = Array.from({ length });
	${makeJitQueryMapperInner(columns, joinsNotNullableMap)}
	return mapped;
	//# sourceURL=drizzle:jit-query-mapper`;
	return Object.assign(new FnConstructor("rows", internals).bind({ columns }), { body: `function jitQueryMapper (rows) {\n${internals}\n}` });
}
function makeDefaultQueryMapper(columns, joinsNotNullableMap) {
	const interpretedData = columns.map(({ field, codec, arrayDimensions, path }) => {
		let processNullifyMap;
		let decoderSrc;
		if (is(field, Column)) {
			decoderSrc = field;
			if (joinsNotNullableMap && path.length === 2) {
				const objectName = path[0];
				processNullifyMap = (nullifyMap, value) => {
					if (!(objectName in nullifyMap)) nullifyMap[objectName] = value === null ? getTableName(field.table) : false;
					else if (typeof nullifyMap[objectName] === "string" && nullifyMap[objectName] !== getTableName(field.table)) nullifyMap[objectName] = false;
				};
			}
		} else if (is(field, SQL)) decoderSrc = field.decoder;
		else if (is(field, Subquery)) decoderSrc = field._.sql.decoder;
		else decoderSrc = field.sql.decoder;
		let decoder;
		if (decoderSrc.mapFromDriverValue.isNoop) decoder = codec ? (v) => codec(v, arrayDimensions) : void 0;
		else decoder = codec ? (v) => decoderSrc.mapFromDriverValue(codec(v, arrayDimensions)) : (v) => decoderSrc.mapFromDriverValue(v);
		return [decoder, processNullifyMap];
	});
	return ((rows) => rows.map((row) => {
		const nullifyMap = {};
		const result = columns.reduce((result, { path }, columnIndex) => {
			let node = result;
			for (const [pathChunkIndex, pathChunk] of path.entries()) if (pathChunkIndex < path.length - 1) {
				if (!(pathChunk in node)) node[pathChunk] = {};
				node = node[pathChunk];
			} else {
				const [decoder, processNullifyMap] = interpretedData[columnIndex];
				const rawValue = row[columnIndex];
				const value = node[pathChunk] = rawValue === null ? null : decoder ? decoder(rawValue) : rawValue;
				processNullifyMap?.(nullifyMap, value);
			}
			return result;
		}, {});
		if (joinsNotNullableMap && Object.keys(nullifyMap).length > 0) {
			for (const [objectName, tableName] of Object.entries(nullifyMap)) if (typeof tableName === "string" && !joinsNotNullableMap[tableName]) result[objectName] = null;
		}
		return result;
	}));
}
/** @internal */
function orderSelectedFields(fields, pathPrefix, codecs) {
	return Object.entries(fields).reduce((result, [name, field]) => {
		if (typeof name !== "string") return result;
		const newPath = pathPrefix ? [...pathPrefix, name] : [name];
		if (is(field, Column)) result.push({
			path: newPath,
			field,
			codec: codecs?.get(field, "normalize"),
			arrayDimensions: field.dimensions,
			column: field
		});
		else if (is(field, SQL) || is(field, SQL.Aliased)) {
			const col = getColumnFromDecoder(field);
			result.push(col ? {
				path: newPath,
				field,
				codec: codecs?.get(col, "normalize"),
				arrayDimensions: col.dimensions,
				column: col
			} : {
				path: newPath,
				field
			});
		} else if (is(field, Subquery)) {
			let column;
			const entry = Object.values(field._.selectedFields)[0];
			let fieldDecoder;
			if (is(entry, Column)) {
				column = entry;
				fieldDecoder = entry;
			} else if (is(entry, SQL)) {
				column = getColumnFromDecoder(entry);
				fieldDecoder = entry.decoder;
			} else {
				column = getColumnFromDecoder(entry);
				fieldDecoder = entry.sql.decoder;
			}
			if (fieldDecoder) field._.sql.decoder = fieldDecoder;
			result.push(column ? {
				path: newPath,
				field,
				codec: codecs?.get(column, "normalize"),
				arrayDimensions: column.dimensions,
				column
			} : {
				path: newPath,
				field
			});
		} else if (is(field, Table)) result.push(...orderSelectedFields(field[Table.Symbol.Columns], newPath, codecs));
		else result.push(...orderSelectedFields(field, newPath, codecs));
		return result;
	}, []);
}
function getColumnFromDecoder(source) {
	const query = source.getSQL();
	if (is(query.decoder, Column)) return query.decoder;
}
function haveSameKeys(left, right) {
	const leftKeys = Object.keys(left);
	const rightKeys = Object.keys(right);
	if (leftKeys.length !== rightKeys.length) return false;
	for (const [index, key] of leftKeys.entries()) if (key !== rightKeys[index]) return false;
	return true;
}
/** @internal */
function mapUpdateSet(table, values) {
	const entries = Object.entries(values).filter(([, value]) => value !== void 0).map(([key, value]) => {
		if (is(value, SQL) || is(value, Column)) return [key, value];
		else return [key, new Param(value, table[Table.Symbol.Columns][key])];
	});
	if (entries.length === 0) throw new Error("No values to set");
	return Object.fromEntries(entries);
}
/** @internal */
function applyMixins(baseClass, extendedClasses) {
	for (const extendedClass of extendedClasses) for (const name of Object.getOwnPropertyNames(extendedClass.prototype)) {
		if (name === "constructor") continue;
		Object.defineProperty(baseClass.prototype, name, Object.getOwnPropertyDescriptor(extendedClass.prototype, name) || Object.create(null));
	}
}
/**
* @deprecated
* Use `getColumns` instead
*/
function getTableColumns(table) {
	return table[Table.Symbol.Columns];
}
/** @internal */
function getTableLikeName(table) {
	return is(table, Subquery) ? table._.alias : is(table, View) ? table[ViewBaseConfig].name : is(table, SQL) ? void 0 : table[Table.Symbol.IsAlias] ? table[Table.Symbol.Name] : table[Table.Symbol.BaseName];
}
/** @internal */
function getColumnNameAndConfig(a, b) {
	return {
		name: typeof a === "string" && a.length > 0 ? a : "",
		config: typeof a === "object" ? a : b
	};
}
var textDecoder = typeof TextDecoder === "undefined" ? null : new TextDecoder();
function assertUnreachable(_x) {
	throw new Error("Didn't expect to get here");
}
var Relation = class {
	static [entityKind] = "RelationV2";
	fieldName;
	sourceColumns;
	targetColumns;
	alias;
	where;
	sourceTable;
	targetTable;
	through;
	throughTable;
	isReversed;
	/** @internal */
	sourceColumnTableNames = [];
	/** @internal */
	targetColumnTableNames = [];
	constructor(targetTable, targetTableName) {
		this.targetTableName = targetTableName;
		this.targetTable = targetTable;
	}
};
var One = class extends Relation {
	static [entityKind] = "OneV2";
	relationType = "one";
	optional;
	constructor(tables, targetTable, targetTableName, config) {
		super(targetTable, targetTableName);
		this.alias = config?.alias;
		this.where = config?.where;
		if (config?.from) this.sourceColumns = (Array.isArray(config.from) ? config.from : [config.from]).map((it) => {
			this.throughTable ??= it._.through ? tables[it._.through._.tableName] : void 0;
			this.sourceColumnTableNames.push(it._.tableName);
			return it._.column;
		});
		if (config?.to) this.targetColumns = (Array.isArray(config.to) ? config.to : [config.to]).map((it) => {
			this.throughTable ??= it._.through ? tables[it._.through._.tableName] : void 0;
			this.targetColumnTableNames.push(it._.tableName);
			return it._.column;
		});
		if (this.throughTable) this.through = {
			source: (Array.isArray(config?.from) ? config.from : config?.from ? [config.from] : []).map((c) => c._.through),
			target: (Array.isArray(config?.to) ? config.to : config?.to ? [config.to] : []).map((c) => c._.through)
		};
		this.optional = config?.optional ?? true;
	}
};
var operators = {
	and,
	between,
	eq,
	exists,
	gt,
	gte,
	ilike,
	inArray,
	arrayContains,
	arrayContained,
	arrayOverlaps,
	isNull,
	isNotNull,
	like,
	lt,
	lte,
	ne,
	not,
	notBetween,
	notExists,
	notLike,
	notIlike,
	notInArray,
	or,
	sql
};
var orderByOperators = {
	sql,
	asc,
	desc
};
function mapRelationalRow(rows, isOne, buildQueryResultSelection, parseJson = false, parseJsonIfString = false, useJsonMappers = true) {
	const maxIdx = isOne ? 1 : rows.length;
	const decoders = buildQueryResultSelection.map(({ field, codec, arrayDimensions }) => {
		let decoder;
		if (is(field, Column)) decoder = field;
		else if (is(field, SQL)) decoder = field.decoder;
		else if (is(field, SQL.Aliased)) decoder = field.sql.decoder;
		else if (is(field, Table) || is(field, View)) decoder = noopDecoder;
		else decoder = field.getSQL().decoder;
		if (useJsonMappers && field.mapFromJsonValue) return (v) => field.mapFromJsonValue(v);
		return decoder.mapFromDriverValue.isNoop ? codec ? (value) => codec(value, arrayDimensions) : void 0 : codec ? (value) => decoder.mapFromDriverValue(codec(value, arrayDimensions)) : (value) => decoder.mapFromDriverValue(value);
	});
	for (let i = 0; i < maxIdx; ++i) {
		const row = isOne ? rows : rows[i];
		for (let selectionItemIdx = 0; selectionItemIdx < buildQueryResultSelection.length; ++selectionItemIdx) {
			const selectionItem = buildQueryResultSelection[selectionItemIdx];
			if (selectionItem.selection) {
				if (row[selectionItem.key] === null) continue;
				if (parseJson) {
					row[selectionItem.key] = JSON.parse(row[selectionItem.key]);
					if (row[selectionItem.key] === null) continue;
				} else if (parseJsonIfString && typeof row[selectionItem.key] === "string") row[selectionItem.key] = JSON.parse(row[selectionItem.key]);
				if (selectionItem.isArray) {
					mapRelationalRow(row[selectionItem.key], false, selectionItem.selection, false, parseJsonIfString);
					continue;
				}
				mapRelationalRow(row[selectionItem.key], true, selectionItem.selection, false, parseJsonIfString);
				continue;
			}
			if (row[selectionItem.key] === null) continue;
			const decoder = decoders[selectionItemIdx];
			if (!decoder) continue;
			row[selectionItem.key] = decoder(row[selectionItem.key]);
		}
	}
	return rows;
}
function mapRelationalRowFromArrays(rows, isOne, buildQueryResultSelection, parseJson = false, parseJsonIfString = false) {
	const maxIdx = isOne ? 1 : rows.length;
	const decoders = buildQueryResultSelection.map(({ field, codec, arrayDimensions }) => {
		let decoder;
		if (is(field, Column)) decoder = field;
		else if (is(field, SQL)) decoder = field.decoder;
		else if (is(field, SQL.Aliased)) decoder = field.sql.decoder;
		else if (is(field, Table) || is(field, View)) decoder = noopDecoder;
		else decoder = field.getSQL().decoder;
		return decoder.mapFromDriverValue.isNoop ? codec ? (value) => codec(value, arrayDimensions) : void 0 : codec ? (value) => decoder.mapFromDriverValue(codec(value, arrayDimensions)) : (value) => decoder.mapFromDriverValue(value);
	});
	const results = Array.from({ length: maxIdx });
	for (let i = 0; i < maxIdx; ++i) {
		const row = isOne ? rows : rows[i];
		const result = {};
		for (let selectionItemIdx = 0; selectionItemIdx < buildQueryResultSelection.length; ++selectionItemIdx) {
			const selectionItem = buildQueryResultSelection[selectionItemIdx];
			let value = row[selectionItemIdx];
			if (selectionItem.selection) {
				if (value === null) {
					result[selectionItem.key] = null;
					continue;
				}
				if (parseJson) {
					value = JSON.parse(value);
					if (value === null) {
						result[selectionItem.key] = null;
						continue;
					}
				} else if (parseJsonIfString && typeof value === "string") value = JSON.parse(value);
				if (selectionItem.isArray) mapRelationalRow(value, false, selectionItem.selection, false, parseJsonIfString);
				else mapRelationalRow(value, true, selectionItem.selection, false, parseJsonIfString);
				result[selectionItem.key] = value;
				continue;
			}
			if (value === null) {
				result[selectionItem.key] = null;
				continue;
			}
			const decoder = decoders[selectionItemIdx];
			result[selectionItem.key] = decoder ? decoder(value) : value;
		}
		results[i] = result;
	}
	return isOne ? results[0] : results;
}
function makeDefaultRqbMapper({ selection, isFirst, parseJson, parseJsonIfString, rootJsonMappers, arrayModeRoot }) {
	return ((rows) => {
		if (isFirst && !rows[0]) return rows[0];
		return arrayModeRoot ? mapRelationalRowFromArrays(isFirst ? rows[0] : rows, isFirst, selection, parseJson, parseJsonIfString) : mapRelationalRow(isFirst ? rows[0] : rows, isFirst, selection, parseJson, parseJsonIfString, rootJsonMappers);
	});
}
function makeJitRqbMapperInner(selection, rowExpr, selectionVar, parseJson, parseJsonIfString, useJsonMappers, preFn, counter, accessByIdx) {
	const bodyStmts = [];
	const literalEntries = [];
	let hasWork = false;
	const fieldVars = selection.map(() => `c${counter.n++}`);
	const destructurePieces = selection.map((item, idx) => accessByIdx ? fieldVars[idx] : `${JSON.stringify(item.key)}: ${fieldVars[idx]}`);
	bodyStmts.push(accessByIdx ? `let [ ${destructurePieces.join(", ")} ] = ${rowExpr};` : `let { ${destructurePieces.join(", ")} } = ${rowExpr};`);
	for (const [idx, { field, key, codec, isArray, selection: innerSelection, arrayDimensions }] of selection.entries()) {
		const sel = `${selectionVar}[${idx}]`;
		const keyStr = JSON.stringify(key);
		const slot = fieldVars[idx];
		if (innerSelection) {
			if (parseJson) {
				bodyStmts.push(`if (${slot} !== null) ${slot} = JSON.parse(${slot});`);
				hasWork = true;
			} else if (parseJsonIfString) {
				bodyStmts.push(`if (typeof ${slot} === 'string') ${slot} = JSON.parse(${slot});`);
				hasWork = true;
			}
			const nestedSelVar = `s${counter.n++}`;
			const savedPreFnLen = preFn.length;
			preFn.push(`const { selection: ${nestedSelVar} } = ${sel};`);
			if (isArray) {
				const j = `j${counter.n++}`;
				const inner = makeJitRqbMapperInner(innerSelection, `${slot}[${j}]`, nestedSelVar, false, parseJsonIfString, true, preFn, counter, false);
				if (inner.hasWork) {
					hasWork = true;
					bodyStmts.push(`if (${slot} !== null) {`);
					bodyStmts.push(`\tfor (let ${j} = 0; ${j} < ${slot}.length; ++${j}) {`);
					for (const s of inner.bodyStmts) bodyStmts.push(`\t\t${s}`);
					bodyStmts.push(`\t\t${slot}[${j}] = ${inner.literal};`);
					bodyStmts.push(`\t}`);
					bodyStmts.push(`}`);
				} else preFn.splice(savedPreFnLen, 1);
			} else {
				const inner = makeJitRqbMapperInner(innerSelection, slot, nestedSelVar, false, parseJsonIfString, true, preFn, counter, false);
				if (inner.hasWork) {
					hasWork = true;
					bodyStmts.push(`if (${slot} !== null) {`);
					for (const s of inner.bodyStmts) bodyStmts.push(`\t${s}`);
					bodyStmts.push(`\t${slot} = ${inner.literal};`);
					bodyStmts.push(`}`);
				} else preFn.splice(savedPreFnLen, 1);
			}
			literalEntries.push(`${keyStr}: ${slot}`);
			continue;
		}
		let decoderExpr = "";
		let destructure = "";
		let bypassCodecs = false;
		if (is(field, Column)) {
			if (useJsonMappers && field.mapFromJsonValue) {
				bypassCodecs = true;
				const id = counter.n++;
				destructure = `field: dec${id}`;
				decoderExpr = `dec${id}.mapFromJsonValue`;
			} else if (!field.mapFromDriverValue.isNoop) {
				const id = counter.n++;
				destructure = `field: dec${id}`;
				decoderExpr = `dec${id}.mapFromDriverValue`;
			}
		} else if (is(field, SQL)) {
			if (useJsonMappers && field.decoder.mapFromJsonValue) {
				bypassCodecs = true;
				const id = counter.n++;
				destructure = `field: { decoder: dec${id} }`;
				decoderExpr = `dec${id}.mapFromJsonValue`;
			} else if (!field.decoder.mapFromDriverValue.isNoop) {
				const id = counter.n++;
				destructure = `field: { decoder: dec${id} }`;
				decoderExpr = `dec${id}.mapFromDriverValue`;
			}
		} else if (is(field, SQL.Aliased)) {
			if (useJsonMappers && field.sql.decoder.mapFromJsonValue) {
				bypassCodecs = true;
				const id = counter.n++;
				destructure = `field: { sql: { decoder: dec${id} } }`;
				decoderExpr = `dec${id}.mapFromJsonValue`;
			} else if (!field.sql.decoder.mapFromDriverValue.isNoop) {
				const id = counter.n++;
				destructure = `field: { sql: { decoder: dec${id} } }`;
				decoderExpr = `dec${id}.mapFromDriverValue`;
			}
		} else if (is(field, Table) || is(field, View)) {} else {
			const sqlExpr = field.getSQL();
			if (useJsonMappers && sqlExpr.decoder.mapFromJsonValue) {
				bypassCodecs = true;
				const id = counter.n++;
				preFn.push(`const dec${id} = ${sel}.field.getSQL().decoder;`);
				decoderExpr = `dec${id}.mapFromJsonValue`;
			} else if (!sqlExpr.decoder.mapFromDriverValue.isNoop) {
				const id = counter.n++;
				preFn.push(`const dec${id} = ${sel}.field.getSQL().decoder;`);
				decoderExpr = `dec${id}.mapFromDriverValue`;
			}
		}
		let codecVar = "";
		if (!bypassCodecs && codec) codecVar = `codec${counter.n++}`;
		if (destructure || codecVar) {
			const parts = [];
			if (destructure) parts.push(destructure);
			if (codecVar) parts.push(`codec: ${codecVar}`);
			preFn.push(`const { ${parts.join(", ")} } = ${sel};`);
		}
		if (decoderExpr || codecVar) {
			hasWork = true;
			let decoded = slot;
			if (codecVar) decoded = `${codecVar}(${decoded}, ${arrayDimensions})`;
			if (decoderExpr) decoded = `${decoderExpr}(${decoded})`;
			literalEntries.push(`${keyStr}: ${slot} === null ? null : ${decoded}`);
		} else literalEntries.push(`${keyStr}: ${slot}`);
	}
	return {
		bodyStmts,
		literal: `{ ${literalEntries.join(", ")} }`,
		hasWork
	};
}
function makeJitRqbMapper({ selection, isFirst, parseJson, parseJsonIfString, rootJsonMappers, arrayModeRoot }) {
	const preFn = [];
	const inner = makeJitRqbMapperInner(selection, "row", "selection", parseJson, parseJsonIfString, arrayModeRoot ? false : rootJsonMappers, preFn, { n: 0 }, !!arrayModeRoot);
	const lines = [];
	lines.push(`\t"use strict";
	const { selection } = this;`);
	for (const p of preFn) lines.push(`\t${p}`);
	if (arrayModeRoot) if (isFirst) {
		lines.push(`\tconst row = rows[0];`);
		lines.push(`\tif (!row) return undefined;`);
		for (const s of inner.bodyStmts) lines.push(`\t${s}`);
		lines.push(`\treturn ${inner.literal};`);
	} else {
		lines.push(`\tconst { length } = rows;`);
		lines.push(`\tconst mapped = Array.from({ length });`);
		lines.push(`\tfor (let i = 0; i < length; ++i) {`);
		lines.push(`\t\tconst row = rows[i];`);
		for (const s of inner.bodyStmts) lines.push(`\t\t${s}`);
		lines.push(`\t\tmapped[i] = ${inner.literal};`);
		lines.push(`\t}`);
		lines.push(`\treturn mapped;`);
	}
	else if (!inner.hasWork) lines.push(isFirst ? `\treturn rows[0];` : `\treturn rows;`);
	else if (isFirst) {
		lines.push(`\tconst row = rows[0];`);
		lines.push(`\tif (!row) return undefined;`);
		for (const s of inner.bodyStmts) lines.push(`\t${s}`);
		lines.push(`\trows[0] = ${inner.literal};`);
		lines.push(`\treturn rows[0];`);
	} else {
		lines.push(`\tfor (let i = 0; i < rows.length; ++i) {`);
		lines.push(`\t\tconst row = rows[i];`);
		for (const s of inner.bodyStmts) lines.push(`\t\t${s}`);
		lines.push(`\t\trows[i] = ${inner.literal};`);
		lines.push(`\t}`);
		lines.push(`\treturn rows;`);
	}
	lines.push("	//# sourceURL=drizzle:jit-relational-query-mapper");
	const compiled = lines.join("\n");
	return Object.assign(new FnConstructor("rows", compiled).bind({ selection }), { body: `function jitRqbMapper (rows) {\n${compiled}\n}` });
}
/** @internal */
function fieldSelectionToSQL(table, target) {
	const field = table[TableColumns][target];
	return field ? is(field, Column) ? field : is(field, SQL.Aliased) ? sql`${table}.${sql.identifier(field.fieldAlias)}` : sql`${table}.${sql.identifier(target)}` : sql`${table}.${sql.identifier(target)}`;
}
function relationsFieldFilterToSQL(column, filter) {
	if (typeof filter !== "object" || is(filter, Placeholder)) return eq(column, filter);
	const entries = Object.entries(filter);
	if (!entries.length) return void 0;
	const parts = [];
	for (const [target, value] of entries) {
		if (value === void 0) continue;
		switch (target) {
			case "NOT": {
				const res = relationsFieldFilterToSQL(column, value);
				if (!res) continue;
				parts.push(not(res));
				continue;
			}
			case "OR":
				if (!value.length) continue;
				parts.push(or(...value.map((subFilter) => relationsFieldFilterToSQL(column, subFilter))));
				continue;
			case "AND":
				if (!value.length) continue;
				parts.push(and(...value.map((subFilter) => relationsFieldFilterToSQL(column, subFilter))));
				continue;
			case "isNotNull":
			case "isNull":
				if (!value) continue;
				parts.push(operators[target](column));
				continue;
			case "in":
				parts.push(operators.inArray(column, value));
				continue;
			case "notIn":
				parts.push(operators.notInArray(column, value));
				continue;
			default:
				parts.push(operators[target](column, value));
				continue;
		}
	}
	if (!parts.length) return void 0;
	return and(...parts);
}
function relationsFilterToSQL(table, filter, tableRelations = {}, tablesRelations = {}, depth = 0) {
	const entries = Object.entries(filter);
	if (!entries.length) return void 0;
	const parts = [];
	for (const [target, value] of entries) {
		if (value === void 0) continue;
		switch (target) {
			case "RAW": {
				const processed = typeof value === "function" ? value(table, operators) : value.getSQL();
				parts.push(processed);
				continue;
			}
			case "OR":
				if (!value?.length) continue;
				parts.push(or(...value.map((subFilter) => relationsFilterToSQL(table, subFilter, tableRelations, tablesRelations, depth))));
				continue;
			case "AND":
				if (!value?.length) continue;
				parts.push(and(...value.map((subFilter) => relationsFilterToSQL(table, subFilter, tableRelations, tablesRelations, depth))));
				continue;
			case "NOT": {
				if (value === void 0) continue;
				const built = relationsFilterToSQL(table, value, tableRelations, tablesRelations, depth);
				if (!built) continue;
				parts.push(not(built));
				continue;
			}
			default: {
				if (table[TableColumns][target]) {
					const colFilter = relationsFieldFilterToSQL(fieldSelectionToSQL(table, target), value);
					if (colFilter) parts.push(colFilter);
					continue;
				}
				const relation = tableRelations[target];
				if (!relation) throw new DrizzleError({ message: `Unknown relational filter field: "${target}"` });
				const targetTable = aliasedTable(relation.targetTable, `f${depth}`);
				const throughTable = relation.throughTable ? aliasedTable(relation.throughTable, `ft${depth}`) : void 0;
				const targetConfig = tablesRelations[relation.targetTableName];
				const { filter: relationFilter, joinCondition } = relationToSQL(relation, table, targetTable, throughTable);
				const filter = and(relationFilter, typeof value === "boolean" ? void 0 : relationsFilterToSQL(targetTable, value, targetConfig.relations, tablesRelations, depth + 1));
				const subquery = throughTable ? sql`(select * from ${getTableAsAliasSQL(targetTable)} inner join ${getTableAsAliasSQL(throughTable)} on ${joinCondition}${sql` where ${filter}`.if(filter)} limit 1)` : sql`(select * from ${getTableAsAliasSQL(targetTable)}${sql` where ${filter}`.if(filter)} limit 1)`;
				if (filter) parts.push((value ? exists : notExists)(subquery));
			}
		}
	}
	return and(...parts);
}
function relationsOrderToSQL(table, orders) {
	if (typeof orders === "function") {
		const data = orders(table, orderByOperators);
		return is(data, SQL) ? data : Array.isArray(data) ? data.length ? sql.join(data.map((o) => is(o, SQL) ? o : asc(o)), sql`, `) : void 0 : is(data, Column) ? asc(data) : void 0;
	}
	const entries = Object.entries(orders).filter(([_, value]) => value);
	if (!entries.length) return void 0;
	return sql.join(entries.map(([target, value]) => (value === "asc" ? asc : desc)(fieldSelectionToSQL(table, target))), sql`, `);
}
function relationExtrasToSQL(table, extras, codecs, inJson) {
	const subqueries = [];
	const selection = [];
	for (const [key, field] of Object.entries(extras)) {
		if (!field) continue;
		const subq = (typeof field === "function" ? field(table, { sql: operators.sql }) : field).getSQL();
		const column = codecs ? getColumnFromDecoder(subq) : void 0;
		const query = column && (!inJson || !column.jsonSelectIdentifier) ? sql`${codecs.apply(column, inJson ? "castInJson" : "cast", sql`(${subq})`)} as ${sql.identifier(key)}` : sql`(${subq}) as ${sql.identifier(key)}`;
		query.decoder = subq.decoder;
		subqueries.push(query);
		selection.push(column && (!inJson || !column.mapFromJsonValue) ? {
			key,
			field: query,
			codec: codecs.get(column, inJson ? "normalizeInJson" : "normalize"),
			arrayDimensions: column.dimensions
		} : {
			key,
			field: query
		});
	}
	return {
		sql: subqueries.length ? sql.join(subqueries, sql`, `) : void 0,
		selection
	};
}
function relationToSQL(relation, sourceTable, targetTable, throughTable) {
	if (relation.through) {
		const outerColumnWhere = relation.sourceColumns.map((s, i) => {
			const t = relation.through.source[i];
			return eq(sql`${sourceTable}.${sql.identifier(s.name)}`, sql`${throughTable}.${sql.identifier(is(t._.column, Column) ? t._.column.name : t._.key)}`);
		});
		const innerColumnWhere = relation.targetColumns.map((s, i) => {
			const t = relation.through.target[i];
			return eq(sql`${throughTable}.${sql.identifier(is(t._.column, Column) ? t._.column.name : t._.key)}`, sql`${targetTable}.${sql.identifier(s.name)}`);
		});
		return {
			filter: and(relation.where ? relationsFilterToSQL(relation.isReversed ? sourceTable : targetTable, relation.where) : void 0, ...outerColumnWhere),
			joinCondition: and(...innerColumnWhere)
		};
	}
	return { filter: and(...relation.sourceColumns.map((s, i) => {
		const t = relation.targetColumns[i];
		return eq(sql`${sourceTable}.${sql.identifier(s.name)}`, sql`${targetTable}.${sql.identifier(t.name)}`);
	}), relation.where ? relationsFilterToSQL(relation.isReversed ? sourceTable : targetTable, relation.where) : void 0) };
}
function getTableAsAliasSQL(table) {
	return sql`${table[IsAlias] ? sql`${sql`${sql.identifier(table[TableSchema] ?? "")}.`.if(table[TableSchema])}${sql.identifier(table[OriginalName])} as ${table}` : table}`;
}
var ColumnBuilder = class {
	static [entityKind] = "ColumnBuilder";
	/** @internal */
	config;
	constructor(name, dataType, columnType) {
		this.config = {
			name,
			keyAsName: name === "",
			notNull: false,
			default: void 0,
			hasDefault: false,
			primaryKey: false,
			isUnique: false,
			uniqueName: void 0,
			uniqueType: void 0,
			dataType,
			columnType,
			generated: void 0
		};
	}
	/**
	* Changes the data type of the column. Commonly used with `json` columns. Also, useful for branded types.
	*
	* @example
	* ```ts
	* const users = pgTable('users', {
	* 	id: integer('id').$type<UserId>().primaryKey(),
	* 	details: json('details').$type<UserDetails>().notNull(),
	* });
	* ```
	*/
	$type() {
		return this;
	}
	/**
	* Adds a `not null` clause to the column definition.
	*
	* Affects the `select` model of the table - columns *without* `not null` will be nullable on select.
	*/
	notNull() {
		this.config.notNull = true;
		return this;
	}
	/**
	* Adds a `default <value>` clause to the column definition.
	*
	* Affects the `insert` model of the table - columns *with* `default` are optional on insert.
	*
	* If you need to set a dynamic default value, use {@link $defaultFn} instead.
	*/
	default(value) {
		this.config.default = value;
		this.config.hasDefault = true;
		return this;
	}
	/**
	* Adds a dynamic default value to the column.
	* The function will be called when the row is inserted, and the returned value will be used as the column value.
	*
	* **Note:** This value does not affect the `drizzle-kit` behavior, it is only used at runtime in `drizzle-orm`.
	*/
	$defaultFn(fn) {
		this.config.defaultFn = fn;
		this.config.hasDefault = true;
		return this;
	}
	/**
	* Alias for {@link $defaultFn}.
	*/
	$default = this.$defaultFn;
	/**
	* Adds a dynamic update value to the column.
	* The function will be called when the row is updated, and the returned value will be used as the column value if none is provided.
	* If no `default` (or `$defaultFn`) value is provided, the function will be called when the row is inserted as well, and the returned value will be used as the column value.
	*
	* **Note:** This value does not affect the `drizzle-kit` behavior, it is only used at runtime in `drizzle-orm`.
	*/
	$onUpdateFn(fn) {
		this.config.onUpdateFn = fn;
		this.config.hasDefault = true;
		return this;
	}
	/**
	* Alias for {@link $onUpdateFn}.
	*/
	$onUpdate = this.$onUpdateFn;
	/**
	* Adds a `primary key` clause to the column definition. This implicitly makes the column `not null`.
	*
	* In SQLite, `integer primary key` implicitly makes the column auto-incrementing.
	*/
	primaryKey() {
		this.config.primaryKey = true;
		this.config.notNull = true;
		return this;
	}
	/** @internal Sets the name of the column to the key within the table definition if a name was not given. */
	setName(name, casingFn) {
		if (this.config.name !== "") return;
		this.config.name = casingFn(name);
	}
};
var ConsoleLogWriter = class {
	static [entityKind] = "ConsoleLogWriter";
	write(message) {
		console.log(message);
	}
};
var DefaultLogger = class {
	static [entityKind] = "DefaultLogger";
	writer;
	constructor(config) {
		this.writer = config?.writer ?? new ConsoleLogWriter();
	}
	logQuery(query, params) {
		const stringifiedParams = params.map((p) => {
			try {
				return JSON.stringify(p);
			} catch {
				return String(p);
			}
		});
		const paramsStr = stringifiedParams.length ? ` -- params: [${stringifiedParams.join(", ")}]` : "";
		this.writer.write(`Query: ${query}${paramsStr}`);
	}
};
var NoopLogger = class {
	static [entityKind] = "NoopLogger";
	logQuery() {}
};
/**
* A statement rendered once per database and run with named values.
*
* Its constant text lets SQLite reuse its prepared statement and PostgreSQL its plan.
*/
var Statement = class {
	/** Build the statement from named values. */
	#build;
	/** The rendered query of each compiler. */
	#rendered = /* @__PURE__ */ new WeakMap();
	/** Create the statement. */
	constructor(build) {
		this.#build = build;
	}
	/** Read every row with the values. */
	all(database, values = {}) {
		const query = this.#render(database);
		return database.driver.all({
			sql: query.sql,
			params: fillPlaceholders(query.params, values)
		});
	}
	/** Read every row as value arrays. */
	values(database, values = {}) {
		const query = this.#render(database);
		return database.driver.values({
			sql: query.sql,
			params: fillPlaceholders(query.params, values)
		});
	}
	/** Render the statement once per compiler. */
	#render(database) {
		const known = this.#rendered.get(database.compiler);
		if (known !== void 0) return known;
		const compiled = database.compiler.expression(this.#build((name) => sql.placeholder(name)));
		const query = database.driver.render(compiled);
		this.#rendered.set(database.compiler, query);
		return query;
	}
};
/** Select a JSON array's elements as rows of one `value` column. */
function jsonElements(array, name) {
	return dialectSQL({
		sqlite: sql`json_each(${array}) AS ${sql.raw(name)}`,
		postgresql: sql`jsonb_array_elements(${array}::jsonb) AS ${sql.raw(name)}(value)`
	});
}
/** A position in the log: an epoch and a sequence within it. */
var LogPosition = defineSchema(strictObject({
	/** The log's epoch, renewed when the database is restored. */
	epoch: string().min(1),
	/** The log sequence within the epoch. */
	sequence: number().int().nonnegative()
}));
/**
* Select the log's head in one row: its epoch, latest sequence and horizon.
*
* Inside a transaction the head leaves out the transaction's own entries.
*/
function selectHead(dialect) {
	const log = sql.identifier(LOG);
	const logged = dialect === "sqlite" ? sql`(SELECT sequence FROM ${log} AS entry
                WHERE NOT EXISTS (SELECT 1 FROM ${sql.identifier(LOG_TRANSACTION)} AS marker
                    WHERE marker.slot = 1 AND marker.id = entry."transaction")
                ORDER BY sequence DESC LIMIT 1)` : sql`(SELECT max(sequence) FROM ${log})`;
	return sql`SELECT epoch, ${logged} AS logged,
            (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
        FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`;
}
/** Read a head's latest sequence: the latest logged one, or the horizon. */
function latestOf(logged, horizon) {
	const sequences = [logged, horizon].filter((sequence) => sequence !== null).map(Number);
	return sequences.length === 0 ? 0 : Math.max(...sequences);
}
/** Write a row's own columns in JSON form, nulls for missing values. */
function encodeRow(table, row) {
	return encodeColumns(table[TABLE].entries, row, []);
}
/** Write a row's own values in some columns in JSON form, except the excluded ones. */
function encodeColumns(columns, row, omitted) {
	const encoded = {};
	for (const [property, column] of columns) if (Object.hasOwn(row, property) && !omitted.includes(property)) {
		const value = row[property];
		encoded[property] = value === null || value === void 0 ? null : column.definition.toJson(value);
	}
	return encoded;
}
/** Read a row's own columns from JSON form. */
function decodeRow(table, row) {
	const decoded = {};
	for (const [property, column] of table[TABLE].entries) if (Object.hasOwn(row, property)) {
		const value = row[property];
		decoded[property] = value === null || value === void 0 ? null : column.definition.fromJson(value);
	}
	return decoded;
}
/** Read a driver row of positional values by property. */
function fromDriver(columns, values, dialect) {
	const read = {};
	for (const [position, [property, column]] of columns.entries()) {
		const value = values[position];
		read[property] = value === null ? null : column.definition.decode(value, dialect);
	}
	return read;
}
/** The head columns a tuple read selects before each row's own. */
var HEAD_COLUMNS = [
	"epoch",
	"logged",
	"horizon"
];
/** The keys one read by key names. */
var KEYS_PER_READ = 90;
/** The database's logged columns as they were at a log position. */
var Snapshot = class Snapshot {
	/** The database read. */
	database;
	/** The position, absent for the live database. */
	position;
	/** The source of earlier row images. */
	#rewind;
	/** Show a database as of a position. */
	constructor(database, position, rewind) {
		this.database = database;
		this.position = position;
		this.#rewind = rewind ?? database.log.rewind();
	}
	/** Show the live database with its transaction's writes. */
	static live(database) {
		return new Snapshot(database, void 0);
	}
	/** Read a table's rows matching a condition. */
	async rows(table, where) {
		const rows = await this.#read(table, render(where, table));
		const images = await this.#since(table);
		const match = Condition.compile(where, table);
		const kept = images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
		for (const image of images.values()) if (image !== null && Condition.matches(match, image)) kept.push(image);
		return kept;
	}
	/** Read a table's rows at the position with text columns that hold one of some tuples. */
	async select(table, columns, tuples) {
		if (tuples.length === 0) return [];
		const read = await tupleRead(table, columns, this.database.dialect).values(this.database, { tuples: JSON.stringify(tuples) });
		const [epoch, latest, horizon] = read[0];
		const sequence = this.#require(epoch, latestOf(latest, horizon));
		const dialect = this.database.driver.native.dialect;
		const selected = Object.entries(table[TABLE].logged);
		const key = HEAD_COLUMNS.length + selected.findIndex(([property]) => property === table[TABLE].key[0]);
		const rows = read.filter((values) => values[key] !== null).map((values) => fromDriver(selected, values.slice(HEAD_COLUMNS.length), dialect));
		const images = await this.#since(table, sequence);
		const wanted = new Set(tuples.map((tuple) => JSON.stringify(tuple)));
		const match = (row) => wanted.has(JSON.stringify(columns.map((column) => toJson(table, column, row))));
		const kept = images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
		for (const image of images.values()) if (image !== null && match(image)) kept.push(image);
		return kept;
	}
	/** Read one row by its key, absent when it did not exist at the position. */
	async row(table, key) {
		const rows = await this.#read(table, Key.match(table, key));
		const images = await this.#since(table);
		const name = Key.name(table, key);
		return images.has(name) ? images.get(name) ?? void 0 : rows[0];
	}
	/** Read up to a count of admitted matching rows in an order after a row. */
	async ordered(table, query) {
		const admits = query.admits;
		const isAdmittedInMemory = admits !== void 0 && admits.current === void 0;
		const order = Order.complete(query.order, table);
		const namespace = query.namespace ?? { computed: {} };
		const relations = query.relations;
		let changed = -1;
		let reached = this.position?.sequence;
		const images = /* @__PURE__ */ new Map();
		let unsettled = /* @__PURE__ */ new Map();
		let rows = [];
		while (unsettled.size > changed) {
			changed = unsettled.size;
			const selection = and(render(query.where, table, namespace), admits?.current, query.after === void 0 ? void 0 : Order.after(order, table, query.after, namespace));
			rows = await this.#read(table, selection, order, isAdmittedInMemory ? void 0 : query.count + changed, namespace);
			if (reached === void 0) break;
			const sequence = await this.#latest();
			for (const [name, image] of await this.#rewind(table, reached, sequence)) if (!images.has(name)) images.set(name, image);
			reached = Math.max(reached, sequence);
			unsettled = new Map(images);
			const touched = (await relations?.touched(this.position.sequence, sequence) ?? []).filter((key) => !unsettled.has(Key.name(table, key)));
			for (const [name, row] of await this.#rowsOf(table, touched)) unsettled.set(name, row);
		}
		const match = Condition.compile(query.where, table);
		const current = unsettled.size === 0 ? rows : rows.filter((row) => !unsettled.has(Key.name(table, row)));
		const decided = isAdmittedInMemory ? await Promise.all(current.map((row) => admits.image(row))) : void 0;
		const kept = decided === void 0 ? current : current.filter((_, index) => decided[index]);
		for (const image of unsettled.values()) {
			const augmented = image === null ? null : augment(image, namespace.computed, await relations?.resolve(image));
			if (augmented !== null && await decides(match, query.where, augmented, relations) && (query.after === void 0 || Order.rows(order, augmented, query.after) > 0) && (admits === void 0 || await admits.image(augmented))) kept.push(augmented);
		}
		return kept.sort((left, right) => Order.rows(order, left, right)).slice(0, query.count);
	}
	/** Read rows by key as of the position, null for missing keys. */
	async #rowsOf(table, keys) {
		const found = new Map(keys.map((key) => [Key.name(table, key), null]));
		for (let start = 0; start < keys.length; start += KEYS_PER_READ) {
			const matches = Key.any(table, keys.slice(start, start + KEYS_PER_READ));
			for (const row of await this.rows(table, matches)) found.set(Key.name(table, row), row);
		}
		return found;
	}
	/** Read a table's logged columns of the admitted rows, as of the position. */
	async #read(table, selection, order, limit, namespace = { computed: {} }) {
		const computed = Object.fromEntries(Object.entries(namespace.computed).map(([name, expression]) => {
			const value = Expression.render(expression, table, namespace);
			return [name, Expression.kind(expression, table, namespace) === "text" ? value.mapWith(String) : value.mapWith(Number)];
		}));
		const query = this.database.select({
			...table[TABLE].logged,
			...computed
		}).from(table).where(selection);
		const ordered = order === void 0 ? query : query.orderBy(...Order.render(order, table, namespace));
		return await (limit === void 0 ? ordered : ordered.limit(limit));
	}
	/** Read the log's latest sequence within the position's epoch. */
	async #latest() {
		const latest = await this.database.log.position();
		return this.#require(latest.epoch, latest.sequence);
	}
	/** Read the rows' images at the position, none when live. */
	async #since(table, read) {
		if (this.position === void 0) return /* @__PURE__ */ new Map();
		const sequence = read ?? await this.#latest();
		const images = await this.#rewind(table, this.position.sequence, sequence);
		if (!this.database.driver.transaction) return images;
		const undone = new Map(images);
		for (const change of await this.database.log.written([table])) {
			const name = Key.name(table, change.key);
			if (!undone.has(name)) undone.set(name, change.before ?? null);
		}
		return undone;
	}
	/** Require a head of the position's epoch, returning its sequence. */
	#require(epoch, sequence) {
		if (this.position !== void 0 && epoch !== this.position.epoch) throw new DatabaseError("STALE_EPOCH", `position of epoch ${this.position.epoch} is not in the log's epoch ${epoch}`);
		return sequence;
	}
};
/** Render a condition over a table. */
function render(where, table, namespace = { computed: {} }) {
	return Condition.render(where, Condition.bind(table, {}, namespace));
}
/** Add a row's computed values. */
function augment(row, computed, related) {
	const names = Object.keys(computed);
	return names.length === 0 ? row : {
		...row,
		...Object.fromEntries(names.map((name) => [name, Expression.evaluate(computed[name], row, related)]))
	};
}
/** Decide a condition on a row and read relations only when needed. */
async function decides(match, where, row, relations) {
	const binding = {
		column: (name) => row[name],
		parameter: () => null,
		exists: () => void 0
	};
	const decided = match(binding);
	if (decided !== void 0 || relations === void 0) return decided === true;
	const answers = /* @__PURE__ */ new Map();
	for (const { via, where: related } of Condition.relations(where)) answers.set(JSON.stringify([via, related ?? null]), await relations.decide(via, related, row));
	return match({
		...binding,
		exists: (via, related) => answers.get(JSON.stringify([via, related ?? null]))
	}) === true;
}
/** Build the statement reading the head and the rows holding listed tuples. */
function tupleRead(table, columns, dialect) {
	return table.statement(`tuple:${dialect}:${columns.join(",")}`, () => {
		const logged = Object.values(table[TABLE].logged);
		const definitions = table[TABLE].columns;
		const head = sql.join(HEAD_COLUMNS.map((name) => sql`head.${sql.identifier(name)}`), sql`, `);
		return new Statement((value) => sql`SELECT ${head}, ${sql.join(logged, sql`, `)}
                FROM (${selectHead(dialect)}) AS head
                CROSS JOIN ${jsonElements(value("tuples"), "wanted")}
                LEFT JOIN ${table} ON ${sql.join(columns.map((column, index) => sql`${definitions[column]} = wanted.value ->> ${sql.raw(String(index))}`), sql` AND `)}`);
	});
}
/** Write a row's column value in JSON form. */
function toJson(table, column, row) {
	return table[TABLE].columns[column].definition.toJson(row[column]);
}
/**
* The default page size of a log read, in changes.
*
* At 0.5 to 2 KB a change, a page is about 1 MB.
*/
var PAGE_LIMIT = 1e3;
/** The committed changes of one database, in commit order. */
var Log = class {
	/** The database holding the log. */
	database;
	/** Create the log of a database. */
	constructor(database) {
		this.database = database;
	}
	/** Read the images a table's rows had before their first change between two sequences, by key. */
	async images(table, after, upto) {
		return imagesOf(table, await this.#changes(table, after, upto));
	}
	/** Read images through one shared memory of each table's changes. */
	rewind() {
		const known = /* @__PURE__ */ new Map();
		return async (table, after, upto) => {
			const read = known.get(table) ?? {
				after,
				upto: after,
				changes: []
			};
			const earlier = after < read.after ? await this.#changes(table, after, read.after) : [];
			const later = upto > read.upto ? await this.#changes(table, read.upto, upto) : [];
			const changes = [
				...earlier,
				...read.changes,
				...later
			];
			known.set(table, {
				after: Math.min(after, read.after),
				upto: Math.max(upto, read.upto),
				changes
			});
			return imagesOf(table, changes.filter((change) => change.sequence > after && change.sequence <= upto));
		};
	}
	/** Read a table's changes between two sequences. */
	async #changes(table, after, upto) {
		const changes = [];
		for (let reached = after; reached < upto;) {
			const read = await this.read({
				tables: [table],
				after: reached
			});
			changes.push(...read.changes.filter((change) => change.sequence <= upto));
			if (read.sequence <= reached) break;
			reached = read.sequence;
		}
		return changes;
	}
	/** Show the database's logged columns as they were at a position. */
	at(position, rewind) {
		return new Snapshot(this.database, position, rewind);
	}
	/** Read the position of the latest commit. */
	async position() {
		const [position] = await this.database.execute(selectHead(this.database.dialect));
		return {
			epoch: position.epoch,
			sequence: latestOf(position.logged, position.horizon)
		};
	}
	/**
	* Read the position the open transaction's reads reach.
	*
	* SQLite numbers a transaction's entries on write, and PostgreSQL at commit.
	*/
	async reached() {
		const committed = await this.position();
		if (this.database.dialect !== "sqlite" || !this.database.driver.transaction) return committed;
		const [own] = await this.database.execute(sql`
            SELECT max(sequence) AS sequence FROM ${sql.identifier(LOG)}
            WHERE "transaction" = (SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)
        `);
		const sequence = own?.sequence === null || own === void 0 ? 0 : Number(own.sequence);
		return {
			epoch: committed.epoch,
			sequence: Math.max(committed.sequence, sequence)
		};
	}
	/** Name the open transaction's log identifier as an SQL expression. */
	stamp() {
		if (!this.database.driver.transaction) throw new TypeError("read the transaction identity inside a transaction");
		const dialect = this.database.dialect;
		if (dialect === "sqlite") return this.database.state.isLogged ? sql`(SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)` : sql`NULL`;
		else if (dialect === "postgresql") return sql`pg_current_xact_id_if_assigned()::TEXT`;
		else return assertNever(dialect);
	}
	/** Write rows a source already derived, without deriving aggregates again. */
	async copying(run) {
		if (!this.database.driver.transaction) throw new TypeError("copy rows inside a transaction");
		const marker = sql.identifier(LOG_COPYING);
		await this.database.execute(sql`INSERT INTO ${marker} (slot) VALUES (1)`);
		const result = await run();
		await this.database.execute(sql`DELETE FROM ${marker} WHERE slot = 1`);
		return result;
	}
	/** Read the log's epoch. */
	async epoch() {
		const [row] = await this.database.execute(sql`SELECT epoch FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`);
		return row.epoch;
	}
	/** Start a new epoch after a restore. */
	async renew() {
		const epoch = v7();
		await this.database.execute(sql`UPDATE ${sql.identifier(LOG_EPOCH)} SET epoch = ${epoch} WHERE slot = 1`);
		return epoch;
	}
	/** Read committed changes after a sequence, ending each page with a whole transaction. */
	async read(selection) {
		const limit = selection.limit ?? PAGE_LIMIT;
		const tables = new Map(selection.tables.map((table) => [table[TABLE].sqlName, table]));
		const rows = await this.#entries(selection, tables, selection.after, { limit });
		const bounds = rows[0];
		const horizon = bounds.horizon === null ? 0 : Number(bounds.horizon);
		if (selection.tables.some((table) => table[TABLE].retention === "window") && selection.after < horizon) throw new DatabaseError("CHANGES_COMPACTED", `changes after ${selection.after} were compacted; list the tables again`);
		let entries = rows.filter((entry) => entry.sequence !== null);
		const last = entries.at(-1);
		if (entries.length === limit && last !== void 0 && last.transaction !== null) {
			const rest = await this.#entries(selection, tables, Number(last.sequence), { transaction: last.transaction });
			entries = [...entries, ...rest.filter((entry) => entry.sequence !== null)];
		}
		const dialect = this.database.dialect;
		const changes = entries.map((entry) => ({
			sequence: Number(entry.sequence),
			...decodeChange(entry, tables.get(entry.table), dialect)
		}));
		const end = changes.length > 0 ? changes.at(-1).sequence : selection.after;
		return {
			changes,
			sequence: changes.length >= limit ? end : Math.max(end, latestOf(bounds.logged, bounds.horizon))
		};
	}
	/** Read the open transaction's changes before commit. */
	async written(tables) {
		if (this.database.dialect === "sqlite" && !this.database.state.isLogged) throw new TypeError("read written changes of a logged database");
		const [current] = await this.database.execute(sql`SELECT ${this.stamp()} AS id`);
		const transaction = current?.id ?? null;
		if (transaction === null) return [];
		const byName = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
		const names = sql.join([...byName.keys()].map((name) => sql`${name}`), sql`, `);
		const written = this.database.dialect === "postgresql" ? sql`sequence IS NULL ORDER BY id` : sql`true ORDER BY sequence`;
		return (await this.database.execute(sql`
            SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
            FROM ${sql.identifier(LOG)}
            WHERE "transaction" = ${transaction} AND "table" IN (${names}) AND ${written}
        `)).map((entry) => decodeChange(entry, byName.get(entry.table), this.database.dialect));
	}
	/** Wait until the log holds a sequence, returning false once the signal aborts. */
	async wait(sequence, signal) {
		if (this.database.driver.transaction) throw new TypeError("wait for changes outside a transaction");
		return this.until(async () => (await this.position()).sequence >= sequence, signal);
	}
	/** Wait until a check holds after a commit, returning false once the signal aborts. */
	until(check, signal) {
		return this.database.state.commits.until(check, signal);
	}
	/** Yield each page past a sequence, with its selected changes, until the signal aborts. */
	async *follow(selection, signal) {
		let after = selection.after;
		while (!signal.aborted) {
			const page = await this.read({
				...selection,
				after
			});
			if (page.sequence > after) {
				after = page.sequence;
				yield page;
			} else await this.wait(after + 1, signal);
		}
	}
	/** Keep the changes after a position for a consumer until a time. */
	async hold(name, sequence, expiresAt) {
		const hold = sql.identifier(LOG_HOLD);
		await this.database.execute(sql`
            INSERT INTO ${hold} (name, sequence, expires_at)
            VALUES (${name}, ${sequence}, ${expiresAt})
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at
        `);
	}
	/** Stop keeping changes for a consumer. */
	async release(name) {
		await this.database.execute(sql`DELETE FROM ${sql.identifier(LOG_HOLD)} WHERE name = ${name}`);
	}
	/** Delete old windowed changes no hold keeps, and advance the horizon. */
	async compact(before, now = Date.now()) {
		await this.database.transaction(async (transaction) => {
			const log = sql.identifier(LOG);
			const [newest] = await transaction.execute(sql`
                SELECT max(sequence) AS sequence
                FROM ${log}
                WHERE retention = 'window'
                    AND changed_at < ${before}
                    AND sequence IS NOT NULL
            `);
			if (newest?.sequence === null || newest?.sequence === void 0) return;
			const [held] = await transaction.execute(sql`
                SELECT
                    (SELECT min(sequence) FROM ${sql.identifier(LOG_HOLD)} WHERE expires_at > ${now}) AS sequence,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            `);
			const cap = held?.sequence === null || held?.sequence === void 0 ? Number(newest.sequence) : Math.min(Number(newest.sequence), Number(held.sequence));
			if (cap <= Number(held?.horizon ?? 0)) return;
			const [split] = await transaction.execute(sql`
                SELECT min(earlier.sequence) AS first
                FROM ${log} change
                JOIN ${log} later ON later."transaction" = change."transaction" AND later.sequence > change.sequence
                JOIN ${log} earlier ON earlier."transaction" = change."transaction"
                WHERE change.sequence = ${cap}
            `);
			const sequence = split?.first === null || split?.first === void 0 ? cap : Number(split.first) - 1;
			await transaction.execute(sql`
                DELETE FROM ${log}
                WHERE retention = 'window'
                    AND sequence <= ${sequence}
            `);
			const horizon = sql.identifier(LOG_HORIZON);
			const dialect = transaction.dialect;
			const highest = dialect === "sqlite" ? sql`max(${horizon}.sequence, excluded.sequence)` : dialect === "postgresql" ? sql`GREATEST(${horizon}.sequence, excluded.sequence)` : assertNever(dialect);
			await transaction.execute(sql`
                INSERT INTO ${horizon} (slot, sequence)
                VALUES (1, ${sequence})
                ON CONFLICT (slot) DO UPDATE SET sequence = ${highest}
            `);
		});
	}
	/** Read the positions and times of a change's transaction. */
	async bounds(sequence) {
		const log = sql.identifier(LOG);
		const [bounds] = await this.database.execute(sql`
            SELECT min(sequence) AS first, max(sequence) AS last,
                min(changed_at) AS "startedAt", max(changed_at) AS "committedAt"
            FROM ${log}
            WHERE sequence = ${sequence}
                OR "transaction" = (SELECT "transaction" FROM ${log} WHERE sequence = ${sequence})
        `);
		if (bounds === void 0 || bounds.first === null) throw new DatabaseError("CHANGES_COMPACTED", `change ${sequence} is not in the log`);
		return {
			before: Number(bounds.first) - 1,
			after: Number(bounds.last),
			startedAt: Number(bounds.startedAt),
			committedAt: Number(bounds.committedAt)
		};
	}
	/** Read log entries after a sequence, or one transaction's rest, with the bounds. */
	async #entries(selection, tables, after, options) {
		return entryRead(selection.scopes !== void 0, options.transaction !== void 0).all(this.database, {
			after,
			tables: JSON.stringify([...tables.keys()]),
			scopes: JSON.stringify(selection.scopes ?? []),
			limit: options.limit ?? null,
			transaction: options.transaction ?? null
		});
	}
};
/** The entry statements by shape. */
var ENTRY_READS = /* @__PURE__ */ new Map();
/** Build the statement reading log entries after a sequence. */
function entryRead(isScoped, isRest) {
	const key = `${isScoped}:${isRest}`;
	const known = ENTRY_READS.get(key);
	if (known !== void 0) return known;
	const listed = (column, array) => dialectSQL({
		sqlite: sql`${column} IN (SELECT value FROM json_each(${array}))`,
		postgresql: sql`${column} IN (SELECT jsonb_array_elements_text(${array}::jsonb))`
	});
	const log = sql.identifier(LOG);
	const statement = new Statement((value) => sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
                FROM ${log}
                WHERE sequence > ${value("after")}
                    ${isRest ? sql`AND "transaction" = ${value("transaction")}` : sql``}
                    AND ${listed(sql`"table"`, value("tables"))}
                    ${isScoped ? sql`AND ${listed(sql`scope`, value("scopes"))}` : sql``}
                ORDER BY sequence
                ${isRest ? sql`` : sql`LIMIT ${value("limit")}`}
            )
            SELECT bounds.logged, bounds.horizon, entries.*
            FROM bounds
            LEFT JOIN entries ON 1 = 1
            ORDER BY entries.sequence
        `);
	ENTRY_READS.set(key, statement);
	return statement;
}
/** Decode a log entry. */
function decodeChange(entry, table, dialect) {
	const key = typeof entry.key === "string" ? JSON.parse(entry.key) : entry.key;
	const row = typeof entry.row === "string" ? JSON.parse(entry.row) : entry.row;
	const previous = typeof entry.previous === "string" ? JSON.parse(entry.previous) : entry.previous;
	const { columns, entries, key: keyProperties, logged: recorded } = table[TABLE];
	const decode = (column, value) => value === null || value === void 0 ? null : column.definition.decode(value, dialect);
	const logged = {};
	const changed = {};
	for (const [property, column] of entries) {
		if (!Object.hasOwn(recorded, property)) continue;
		const name = column.definition.name;
		logged[property] = decode(column, row[name]);
		if (previous !== null && Object.hasOwn(previous, name)) changed[property] = decode(column, previous[name]);
	}
	const decodedKey = {};
	for (const [index, property] of keyProperties.entries()) decodedKey[property] = decode(columns[property], key[index]);
	return {
		transaction: entry.transaction,
		table,
		key: decodedKey,
		operation: entry.operation,
		...entry.operation === "insert" ? { after: logged } : entry.operation === "delete" ? { before: logged } : {
			before: {
				...logged,
				...changed
			},
			after: logged
		},
		scope: entry.scope,
		changedAt: Number(entry.changed_at)
	};
}
/** Keep each row's image before its first change, null for an insert. */
function imagesOf(table, changes) {
	const images = /* @__PURE__ */ new Map();
	for (const change of changes) {
		const key = Key.name(table, change.key);
		if (!images.has(key)) images.set(key, change.before ?? null);
	}
	return images;
}
/** A notifier for a database only its own connection writes. */
var soleWriter = {
	listen: () => async () => {},
	notify: () => {}
};
/** The commits one physical connection's readers wait for. */
var CommitWatch = class {
	/** The notifications exchanged with the other writers. */
	#notifier;
	/** The readers waiting for the next commit. */
	#waiting = /* @__PURE__ */ new Set();
	/** Stop listening for other writers' commits. */
	#stop;
	/** The failure that ended listening. */
	#failure;
	/** Create the watch. */
	constructor(notifier) {
		this.#notifier = notifier;
	}
	/** Whether readers wait for a commit. */
	get isWaiting() {
		return this.#waiting.size > 0;
	}
	/** Wake every reader waiting for a commit. */
	wake() {
		const waiting = [...this.#waiting];
		this.#waiting.clear();
		for (const wake of waiting) wake();
	}
	/** Wake this connection's readers and notify the other writers. */
	notify() {
		this.wake();
		this.#notifier.notify();
	}
	/** Fail every waiting and later reader. */
	fail(error) {
		this.#failure = { error };
		const waiting = [...this.#waiting];
		this.#waiting.clear();
		for (const wake of waiting) wake(error);
	}
	/** Wait until a check holds after a commit, returning false once the signal aborts. */
	async until(check, signal) {
		while (!signal.aborted) {
			const checked = new AbortController();
			const next = this.#next(AbortSignal.any([signal, checked.signal]));
			if (await check().catch(async (error) => {
				checked.abort();
				await Promise.allSettled([next]);
				throw error;
			})) {
				checked.abort();
				await next;
				return true;
			}
			await next;
		}
		return false;
	}
	/** Wake every reader and stop listening for other writers. */
	async stop() {
		this.wake();
		await this.#stop?.();
	}
	/** Wait for the next commit or the abort. */
	#next(signal) {
		if (this.#failure) return Promise.reject(this.#failure.error);
		this.#stop ??= this.#notifier.listen(this);
		return new Promise((resolve, reject) => {
			if (signal.aborted) {
				resolve();
				return;
			}
			const wake = (failure) => {
				signal.removeEventListener("abort", abort);
				if (failure === void 0) resolve();
				else reject(failure);
			};
			const abort = () => {
				this.#waiting.delete(wake);
				resolve();
			};
			this.#waiting.add(wake);
			signal.addEventListener("abort", abort, { once: true });
		});
	}
};
/** Merge the table states of several declarations. */
function mergeStates(declarations) {
	const groups = /* @__PURE__ */ new Map();
	for (const state of declarations.flat()) groups.set(state.table.name, [...groups.get(state.table.name) ?? [], state]);
	const declared = [];
	const conflicts = [];
	for (const [name, states] of groups) {
		const distinct = [...new Map(states.map((state) => [canonicalize(state), state])).values()];
		const merged = distinct.length === 1 ? { state: distinct[0] } : mergeTable(distinct);
		declared.push(merged.state);
		if (merged.reason !== void 0) conflicts.push({
			table: name,
			reason: merged.reason
		});
	}
	for (const state of declared) {
		const previous = state.moved?.table;
		if (previous !== void 0 && groups.has(previous)) conflicts.push({
			table: state.table.name,
			reason: `a running release still uses ${previous}`
		});
	}
	return {
		declared,
		conflicts
	};
}
/** Merge one table's states into the newest and keep older columns. */
function mergeTable(states) {
	const newest = [...states].sort((left, right) => Version.compare(right.package.version, left.package.version) || Number(renames(right, states)) - Number(renames(left, states)) || canonicalize(left).localeCompare(canonicalize(right)))[0];
	const others = states.filter((state) => state !== newest);
	const declaresColumn = (name) => others.some((state) => state.table.columns.some((column) => column.name === name));
	const moves = { ...newest.moved?.columns };
	const bridges = [...newest.bridges ?? []];
	for (const [column, previous] of Object.entries(moves)) if (declaresColumn(previous)) {
		delete moves[column];
		bridges.push({
			from: previous,
			to: column
		});
	}
	const bridged = new Set(bridges.map((bridge) => bridge.to));
	const columns = newest.table.columns.map((column) => bridged.has(column.name) ? relax(column) : column);
	const byName = new Map(columns.map((column) => [column.name, column]));
	for (const state of others) for (const column of state.table.columns) {
		const existing = byName.get(column.name);
		if (!existing) {
			if (state.table.constraints.some((constraint) => constraint.kind === "primaryKey" && constraint.columns.includes(column.name))) return {
				state: newest,
				reason: `releases disagree on the key ${column.name}`
			};
			const retained = relax(column);
			columns.push(retained);
			byName.set(column.name, retained);
		} else if (!bridged.has(column.name) && canonicalize(existing) !== canonicalize(column)) return {
			state: newest,
			reason: `releases disagree on column ${column.name}`
		};
	}
	const ordered = [newest, ...others];
	const constraints = unite(ordered.map((state) => state.table.constraints));
	const indexes = unite(ordered.map((state) => state.table.indexes));
	const disagreement = constraints.conflict ?? indexes.conflict;
	if (disagreement !== void 0) return {
		state: newest,
		reason: `releases disagree on ${disagreement}`
	};
	const table = {
		...newest.table,
		columns,
		constraints: constraints.entries,
		indexes: indexes.entries
	};
	const logs = new Set(states.map((state) => logOf(state)));
	const trees = new Set(states.map((state) => canonicalize(state.tree ?? null)));
	if (logs.size > 1 || trees.size > 1) return {
		state: newest,
		reason: "releases disagree on the table's log or tree"
	};
	const log = newest.log && {
		...newest.log,
		columns: union$1(states.map((state) => state.log?.columns ?? [])),
		exact: union$1(states.map((state) => state.log?.exact ?? [])),
		compared: union$1(states.map((state) => state.log?.compared ?? []))
	};
	return { state: {
		...newest,
		table,
		...log ? { log } : {},
		...newest.moved ? { moved: {
			...newest.moved,
			columns: moves
		} } : {},
		...bridges.length === 0 ? {} : { bridges }
	} };
}
/** Make a required column without a default nullable. */
function relax(column) {
	return column.nullable || column.default !== void 0 || column.generated ? column : {
		...column,
		nullable: true
	};
}
/** Unite named entries, or name the first differing entry. */
function unite(lists) {
	const united = /* @__PURE__ */ new Map();
	for (const entry of lists.flat()) {
		const existing = united.get(entry.name);
		if (existing !== void 0 && canonicalize(existing) !== canonicalize(entry)) return {
			entries: [],
			conflict: entry.name
		};
		united.set(entry.name, existing ?? entry);
	}
	return { entries: [...united.values()] };
}
/** Unite name lists in order. */
function union$1(lists) {
	return [...new Set(lists.flat())];
}
/** Report whether a release renames a column another release still declares. */
function renames(state, states) {
	return Object.values(state.moved?.columns ?? {}).some((previous) => states.some((other) => other !== state && other.table.columns.some((column) => column.name === previous)));
}
/**
* The rows one chunk of a table carries.
*
* At 0.1 to 2 KB a row, a chunk is up to about 2 MB.
*/
var CHUNK_ROWS = 1e3;
/**
* How long a replication keeps the source's log after its cursor, in milliseconds.
*
* A pass over 1 GB at about 100 MB/s fits well within an hour.
*/
var HOLD_MILLISECONDS = 36e5;
/** The column builder of each logical kind. */
var COLUMNS = {
	text: (name) => text$1(name),
	integer: integer$1,
	real: real$1,
	boolean,
	json: (name) => json(name, json$1()),
	binary,
	bigint,
	numeric: numeric$1,
	timestamp
};
/** Where an export continues. */
var ReplicationCursor = defineSchema(strictObject({
	/** The position the live tables' rows are read at. */
	position: LogPosition,
	/** The step: live rows and links, their changes, then fenced rows and links. */
	step: number().int().nonnegative(),
	/** The key of the last row a table step read. */
	key: record(string(), json$1()).optional(),
	/** The log sequence the changes step read up to. */
	sequence: number().int().nonnegative()
}));
/** The content of one replication chunk. */
var ReplicationChunk = defineSchema(discriminatedUnion("kind", [
	strictObject({
		/** A key range of a table's rows. */
		kind: literal("rows"),
		/** The table's SQL name. */
		table: string(),
		/** The key the range follows, absent from the table's start. */
		after: record(string(), json$1()).optional(),
		/** The last key of the range, absent to the table's end. */
		last: record(string(), json$1()).optional(),
		/** The range's rows in their JSON form. */
		rows: array(record(string(), json$1()))
	}),
	strictObject({
		/** A key range of a table's references to its own rows. */
		kind: literal("links"),
		/** The table's SQL name. */
		table: string(),
		/** Each row's key and self-references, in JSON form. */
		links: array(record(string(), json$1()))
	}),
	strictObject({
		/** Changes of the live tables in commit order. */
		kind: literal("changes"),
		/** Each change's table and row, or the deleted key. */
		changes: array(strictObject({
			/** The table's SQL name. */
			table: string(),
			/** Whether the change deleted the row. */
			isDeleted: boolean$1(),
			/** The row after the change, or its deleted key, in JSON form. */
			row: record(string(), json$1())
		}))
	})
]));
/**
* A replication of a database's tables into another database, exact across dialects.
*
* Live tables are read at a snapshot position, then followed through the log.
* Fenced tables, such as unlogged tables or tables with binary or sensitive columns, are copied whole.
*/
var Replication = class Replication {
	/** The fully logged tables, parents first. */
	live;
	/** The other tables, parents first. */
	fenced;
	/** The copied tables, by SQL name. */
	#tables;
	/** The self-referencing columns of each table. */
	#links;
	/** The export steps, in order. */
	#steps;
	/** Order table states parents first and split them into live and fenced tables. */
	constructor(states) {
		const derived = new Set(states.flatMap((state) => state.tree === void 0 ? [] : [state.tree.ancestors, state.tree.revision]));
		const copied = states.filter((state) => !derived.has(state.table.name));
		const tables = copied.map((state) => Replication.#describedTable(state));
		this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
		const references = new Map(copied.map((state, position) => {
			const parents = state.table.constraints.flatMap((constraint) => constraint.kind === "foreignKey" ? [constraint.table] : []);
			return [tables[position], parents.flatMap((name) => this.#tables.get(name) ?? [])];
		}));
		this.#links = new Map(copied.flatMap((state, position) => {
			const columns = state.table.constraints.flatMap((constraint) => constraint.kind === "foreignKey" && constraint.table === state.table.name ? constraint.columns : []);
			const required = columns.find((name) => !state.table.columns.find((column) => column.name === name).nullable);
			if (required !== void 0) throw new TypeError(`table ${state.table.name} references its own rows through required column ${required}`);
			const textual = (name) => state.table.columns.find((column) => column.name === name).kind === "text";
			const keyColumns = state.table.constraints.flatMap((constraint) => constraint.kind === "primaryKey" ? constraint.columns : []);
			if (columns.length > 0 && ![...columns, ...keyColumns].every(textual)) throw new TypeError(`table ${state.table.name} references its own rows through columns other than text`);
			return columns.length === 0 ? [] : [[tables[position], columns]];
		}));
		const ordered = parentsFirst(tables, references);
		const live = /* @__PURE__ */ new Set();
		for (const table of ordered) {
			const isLogged = table[TABLE].retention !== "none" && Object.keys(table[TABLE].logged).length === table[TABLE].entries.length;
			const parents = references.get(table).filter((parent) => parent !== table);
			if (isLogged && parents.every((parent) => live.has(parent))) live.add(table);
		}
		this.live = ordered.filter((table) => live.has(table));
		this.fenced = ordered.filter((table) => !live.has(table));
		const steps = (tables, isLive) => tables.flatMap((table) => [{
			kind: "rows",
			table,
			isLive
		}, ...this.#links.has(table) ? [{
			kind: "links",
			table,
			isLive
		}] : []]);
		this.#steps = [
			...steps(this.live, true),
			{ kind: "changes" },
			...steps(this.fenced, false)
		];
	}
	/** The copied tables. */
	get tables() {
		return [...this.live, ...this.fenced];
	}
	/** Copy the tables of a database's desired states. */
	static of(desired, dialect) {
		const tables = desired.map((state) => DatabaseState.parse(state).tables[dialect]);
		return new Replication(mergeStates(tables).declared);
	}
	/** Build a table from its state. */
	static #describedTable(state) {
		const description = state.table;
		const logged = new Set(state.log?.columns ?? []);
		const columns = Object.fromEntries(description.columns.map((column) => {
			const definition = {
				...COLUMNS[column.kind](column.name).definition,
				nullable: column.nullable,
				...state.log !== void 0 && column.kind !== "binary" && !logged.has(column.name) ? { classification: "sensitive" } : {},
				...column.generated === void 0 ? {} : { generated: {
					expression: sql.raw(column.generated.expression),
					mode: column.generated.mode ?? "virtual"
				} }
			};
			return [column.name, new Column$1(description.name, definition)];
		}));
		const key = description.constraints.find((constraint) => constraint.kind === "primaryKey");
		return new Table$1({
			package: state.package,
			name: description.name,
			sqlName: description.name
		}, columns, {
			constraints: () => key === void 0 ? [] : [primaryKey$1({ columns: key.columns.map((name) => columns[name]) })],
			retention: state.log?.retention ?? "none",
			moved: {},
			convert: {},
			aggregates: [],
			dependents: []
		});
	}
	/**
	* Read a source's content after a cursor as chunks until the stage ends.
	*
	* The live stage reads live rows at a snapshot and their changes.
	* The fenced stage reads the later changes, then the fenced rows.
	*/
	async *export(database, name, stage, after, signal) {
		let cursor = after === void 0 ? void 0 : ReplicationCursor.parse(JSON.parse(after));
		const position = await database.log.position();
		if (cursor === void 0 || cursor.position.epoch !== position.epoch) cursor = {
			position,
			step: 0,
			sequence: position.sequence
		};
		const changes = this.#steps.findIndex((step) => step.kind === "changes");
		const end = stage === "live" ? changes : this.#steps.length - 1;
		while (cursor.step <= end && !signal.aborted) {
			await database.log.hold(name, cursor.sequence, Date.now() + HOLD_MILLISECONDS);
			const step = this.#steps[cursor.step];
			if (step.kind === "changes") {
				const page = await database.log.read({
					tables: this.live,
					after: cursor.sequence
				});
				if (page.changes.length > 0) {
					const body = {
						kind: "changes",
						changes: page.changes.map((change) => ({
							table: change.table[TABLE].sqlName,
							isDeleted: change.operation === "delete",
							row: encodeRow(change.table, change.after ?? change.key)
						}))
					};
					cursor = {
						...cursor,
						sequence: page.sequence
					};
					yield {
						cursor: JSON.stringify(cursor),
						body
					};
				} else cursor = {
					...cursor,
					step: cursor.step + 1,
					sequence: page.sequence
				};
				continue;
			}
			const table = step.table;
			const from = cursor.key === void 0 ? void 0 : decodeRow(table, cursor.key);
			const rows = step.isLive ? await new Snapshot(database, cursor.position).ordered(table, {
				where: Condition.all(),
				order: Order.complete([], table),
				...from === void 0 ? {} : { after: from },
				count: CHUNK_ROWS
			}) : await this.#rows(database, table, from, CHUNK_ROWS);
			const last = rows.length < CHUNK_ROWS ? void 0 : keyOf(table, rows.at(-1));
			const links = this.#links.get(table) ?? [];
			const body = step.kind === "rows" ? {
				kind: "rows",
				table: table[TABLE].sqlName,
				...cursor.key === void 0 ? {} : { after: cursor.key },
				...last === void 0 ? {} : { last },
				rows: rows.map((row) => encodeRow(table, {
					...row,
					...Object.fromEntries(links.map((column) => [column, null]))
				}))
			} : {
				kind: "links",
				table: table[TABLE].sqlName,
				links: rows.map((row) => ({
					...keyOf(table, row),
					...Object.fromEntries(links.map((column) => [column, encodeRow(table, row)[column] ?? null]))
				}))
			};
			cursor = last === void 0 ? {
				position: cursor.position,
				step: cursor.step + 1,
				sequence: cursor.sequence
			} : {
				...cursor,
				key: last
			};
			yield {
				cursor: JSON.stringify(cursor),
				body
			};
		}
	}
	/** Write one chunk into a target in one transaction. */
	async import(database, chunk) {
		const body = ReplicationChunk.parse(chunk.body);
		await database.transaction(async (transaction) => {
			if (body.kind === "rows") {
				const table = this.#table(body.table);
				const rows = body.rows.map((row) => decodeRow(table, row));
				const kept = new Set(rows.map((row) => Key.name(table, row)));
				const held = await this.#range(transaction, table, body.after, body.last);
				await transaction.remove(table, held.filter((row) => !kept.has(Key.name(table, row))));
				await transaction.upsert(table, rows);
			} else if (body.kind === "links") {
				const table = this.#table(body.table);
				const links = this.#links.get(table);
				const key = table[TABLE].key;
				const field = (name) => sql`linked.value ->> ${name}`;
				await transaction.execute(sql`
                        UPDATE ${table}
                        SET ${sql.join(links.map((name) => sql`${sql.identifier(name)} = ${field(name)}`), sql`, `)}
                        FROM ${jsonElements(sql`${JSON.stringify(body.links)}`, "linked")}
                        WHERE ${sql.join(key.map((name) => sql`${table}.${sql.identifier(name)} = ${field(name)}`), sql` AND `)}
                    `);
			} else {
				const last = /* @__PURE__ */ new Map();
				for (const change of body.changes) {
					const table = this.#table(change.table);
					const row = decodeRow(table, change.row);
					last.set(`${change.table}/${Key.name(table, row)}`, {
						table,
						row,
						isDeleted: change.isDeleted
					});
				}
				for (const table of this.live) {
					const changed = [...last.values()].filter((entry) => entry.table === table);
					await transaction.remove(table, changed.filter((entry) => entry.isDeleted).map((entry) => entry.row));
					await transaction.upsert(table, changed.filter((entry) => !entry.isDeleted).map((entry) => entry.row));
				}
			}
		}, { constraints: "deferred" });
	}
	/** Read up to a count of a table's rows in key order after a row. */
	async #rows(database, table, after, count) {
		const order = Order.complete([], table);
		return database.select().from(table).where(after === void 0 ? void 0 : Order.after(order, table, after)).orderBy(...Order.render(order, table)).limit(count);
	}
	/** Read the keys in a key range. */
	async #range(database, table, after, last) {
		const order = Order.complete([], table);
		const columns = table[TABLE].columns;
		const key = Object.fromEntries(table[TABLE].key.map((name) => [name, columns[name]]));
		return database.select(key).from(table).where(and(after === void 0 ? void 0 : Order.after(order, table, decodeRow(table, after)), last === void 0 ? void 0 : not(Order.after(order, table, decodeRow(table, last)))));
	}
	/** Find a copied table by SQL name. */
	#table(name) {
		const table = this.#tables.get(name);
		if (table === void 0) throw new TypeError(`copied chunk names ${name}, which the database does not hold`);
		return table;
	}
};
/** Write a row's key in JSON form. */
function keyOf(table, row) {
	return encodeColumns(table[TABLE].entries.filter(([property]) => table[TABLE].key.includes(property)), row, []);
}
/** Order tables parents first and refuse cycles. */
function parentsFirst(tables, references) {
	const ordered = [];
	const visiting = [];
	const pending = [...tables].reverse().map((table) => ({
		table,
		isExpanded: false
	}));
	while (pending.length > 0) {
		const { table, isExpanded } = pending.pop();
		if (isExpanded) {
			visiting.splice(visiting.indexOf(table), 1);
			ordered.push(table);
		} else if (visiting.includes(table)) throw new TypeError(`tables referencing ${table[TABLE].sqlName} form a cycle`);
		else if (!ordered.includes(table)) {
			visiting.push(table);
			pending.push({
				table,
				isExpanded: true
			});
			for (const parent of references.get(table)) if (parent !== table && !ordered.includes(parent)) pending.push({
				table: parent,
				isExpanded: false
			});
		}
	}
	return ordered;
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
var _global = typeof globalThis === "object" ? globalThis : typeof self === "object" ? self : typeof window === "object" ? window : typeof global === "object" ? global : {};
function registerGlobal(type, instance, diag, allowOverride = false) {
	var _a;
	const api = _global[GLOBAL_OPENTELEMETRY_API_KEY] = (_a = _global[GLOBAL_OPENTELEMETRY_API_KEY]) !== null && _a !== void 0 ? _a : { version: VERSION };
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
	const globalVersion = (_a = _global[GLOBAL_OPENTELEMETRY_API_KEY]) === null || _a === void 0 ? void 0 : _a.version;
	if (!globalVersion || !isCompatible(globalVersion)) return;
	return (_b = _global[GLOBAL_OPENTELEMETRY_API_KEY]) === null || _b === void 0 ? void 0 : _b[type];
}
function unregisterGlobal(type, diag) {
	diag.debug(`@opentelemetry/api: Unregistering a global for ${type} v${VERSION}.`);
	const api = _global[GLOBAL_OPENTELEMETRY_API_KEY];
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
var API_NAME$2 = "diag";
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
			unregisterGlobal(API_NAME$2, self);
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
var API_NAME$1 = "context";
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
		return registerGlobal(API_NAME$1, contextManager, DiagAPI.instance());
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
		return getGlobal(API_NAME$1) || NOOP_CONTEXT_MANAGER;
	}
	/** Disable and remove the global context manager */
	disable() {
		this._getContextManager().disable();
		unregisterGlobal(API_NAME$1, DiagAPI.instance());
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
var ForeignKeyBuilder = class {
	static [entityKind] = "SQLiteForeignKeyBuilder";
	/** @internal */
	reference;
	/** @internal */
	_onUpdate;
	/** @internal */
	_onDelete;
	constructor(config, actions) {
		this.reference = () => {
			const { name, columns, foreignColumns } = config();
			return {
				name,
				columns,
				foreignTable: foreignColumns[0].table,
				foreignColumns
			};
		};
		if (actions) {
			this._onUpdate = actions.onUpdate;
			this._onDelete = actions.onDelete;
		}
	}
	onUpdate(action) {
		this._onUpdate = action;
		return this;
	}
	onDelete(action) {
		this._onDelete = action;
		return this;
	}
	/** @internal */
	build(table) {
		return new ForeignKey(table, this);
	}
};
var ForeignKey = class {
	static [entityKind] = "SQLiteForeignKey";
	reference;
	onUpdate;
	onDelete;
	constructor(table, builder) {
		this.table = table;
		this.reference = builder.reference;
		this.onUpdate = builder._onUpdate;
		this.onDelete = builder._onDelete;
	}
	getName() {
		const { name, columns, foreignColumns } = this.reference();
		const columnNames = columns.map((column) => column.name);
		const foreignColumnNames = foreignColumns.map((column) => column.name);
		const chunks = [
			this.table[TableName],
			...columnNames,
			foreignColumns[0].table[TableName],
			...foreignColumnNames
		];
		return name ?? `${chunks.join("_")}_fk`;
	}
	isNameExplicit() {
		return !!this.reference().name;
	}
};
function foreignKey(config) {
	function mappedConfig() {
		if (typeof config === "function") {
			const { name, columns, foreignColumns } = config();
			return {
				name,
				columns,
				foreignColumns
			};
		}
		return config;
	}
	return new ForeignKeyBuilder(mappedConfig);
}
var SQLiteColumnBuilder = class extends ColumnBuilder {
	static [entityKind] = "SQLiteColumnBuilder";
	foreignKeyConfigs = [];
	references(ref, actions = {}) {
		this.foreignKeyConfigs.push({
			ref,
			actions
		});
		return this;
	}
	unique(name) {
		this.config.isUnique = true;
		this.config.uniqueName = name;
		return this;
	}
	generatedAlwaysAs(as, config) {
		this.config.generated = {
			as,
			type: "always",
			mode: config?.mode ?? "virtual"
		};
		return this;
	}
	/** @internal */
	buildForeignKeys(column, table) {
		return this.foreignKeyConfigs.map(({ ref, actions }) => {
			return ((ref, actions) => {
				const builder = new ForeignKeyBuilder(() => {
					const foreignColumn = ref();
					return {
						columns: [column],
						foreignColumns: [foreignColumn]
					};
				});
				if (actions.onUpdate) builder.onUpdate(actions.onUpdate);
				if (actions.onDelete) builder.onDelete(actions.onDelete);
				return builder.build(table);
			})(ref, actions);
		});
	}
};
var SQLiteColumn = class extends Column {
	static [entityKind] = "SQLiteColumn";
	/** @internal */
	table;
	constructor(table, config) {
		super(table, config);
		this.table = table;
	}
};
function hexToText(hexString) {
	let result = "";
	for (let i = 0; i < hexString.length; i += 2) {
		const hexPair = hexString.slice(i, i + 2);
		const decimalValue = Number.parseInt(hexPair, 16);
		result += String.fromCodePoint(decimalValue);
	}
	return result;
}
var SQLiteBigIntBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteBigIntBuilder";
	constructor(name) {
		super(name, "bigint int64", "SQLiteBigInt");
	}
	/** @internal */
	build(table) {
		return new SQLiteBigInt(table, this.config);
	}
};
var SQLiteBigInt = class extends SQLiteColumn {
	static [entityKind] = "SQLiteBigInt";
	getSQLType() {
		return "blob";
	}
	mapFromDriverValue = (value) => {
		if (typeof value === "string") return BigInt(hexToText(value));
		if (typeof Buffer !== "undefined" && Buffer.from) {
			const buf = Buffer.isBuffer(value) ? value : value instanceof ArrayBuffer ? Buffer.from(value) : value.buffer ? Buffer.from(value.buffer, value.byteOffset, value.byteLength) : Buffer.from(value);
			return BigInt(buf.toString("utf8"));
		}
		return BigInt(textDecoder.decode(value));
	};
	mapToDriverValue = (value) => {
		return Buffer.from(value.toString());
	};
};
var SQLiteBlobJsonBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteBlobJsonBuilder";
	constructor(name) {
		super(name, "object json", "SQLiteBlobJson");
	}
	/** @internal */
	build(table) {
		return new SQLiteBlobJson(table, this.config);
	}
};
var SQLiteBlobJson = class extends SQLiteColumn {
	static [entityKind] = "SQLiteBlobJson";
	getSQLType() {
		return "blob";
	}
	mapFromDriverValue = (value) => {
		if (typeof value === "string") return JSON.parse(hexToText(value));
		if (typeof Buffer !== "undefined" && Buffer.from) {
			const buf = Buffer.isBuffer(value) ? value : value instanceof ArrayBuffer ? Buffer.from(value) : value.buffer ? Buffer.from(value.buffer, value.byteOffset, value.byteLength) : Buffer.from(value);
			return JSON.parse(buf.toString("utf8"));
		}
		return JSON.parse(textDecoder.decode(value));
	};
	mapToDriverValue = (value) => {
		return Buffer.from(JSON.stringify(value));
	};
};
var SQLiteBlobBufferBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteBlobBufferBuilder";
	constructor(name) {
		super(name, "object buffer", "SQLiteBlobBuffer");
	}
	/** @internal */
	build(table) {
		return new SQLiteBlobBuffer(table, this.config);
	}
};
var SQLiteBlobBuffer = class extends SQLiteColumn {
	static [entityKind] = "SQLiteBlobBuffer";
	mapFromDriverValue = (value) => {
		if (Buffer.isBuffer(value)) return value;
		if (typeof value === "string") return Buffer.from(value, "hex");
		return Buffer.from(value);
	};
	getSQLType() {
		return "blob";
	}
};
function blob(a, b) {
	const { name, config } = getColumnNameAndConfig(a, b);
	if (config?.mode === "bigint") return new SQLiteBigIntBuilder(name);
	if (config?.mode === "buffer") return new SQLiteBlobBufferBuilder(name);
	return new SQLiteBlobJsonBuilder(name);
}
var SQLiteCustomColumnBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteCustomColumnBuilder";
	constructor(name, fieldConfig, customTypeParams) {
		super(name, "custom", "SQLiteCustomColumn");
		this.config.fieldConfig = fieldConfig;
		this.config.customTypeParams = customTypeParams;
	}
	/** @internal */
	build(table) {
		return new SQLiteCustomColumn(table, this.config);
	}
};
var SQLiteCustomColumn = class extends SQLiteColumn {
	static [entityKind] = "SQLiteCustomColumn";
	sqlName;
	mapTo;
	mapFrom;
	mapJson;
	forJsonSelect;
	constructor(table, config) {
		super(table, config);
		this.sqlName = config.customTypeParams.dataType(config.fieldConfig);
		this.mapTo = config.customTypeParams.toDriver;
		this.mapFrom = config.customTypeParams.fromDriver;
		this.mapJson = config.customTypeParams.fromJson;
		this.forJsonSelect = config.customTypeParams.forJsonSelect;
	}
	getSQLType() {
		return this.sqlName;
	}
	mapFromDriverValue = (value) => {
		return typeof this.mapFrom === "function" ? this.mapFrom(value) : value;
	};
	mapFromJsonValue(value) {
		return typeof this.mapJson === "function" ? this.mapJson(value) : this.mapFromDriverValue(value);
	}
	jsonSelectIdentifier(identifier, sql) {
		if (typeof this.forJsonSelect === "function") return this.forJsonSelect(identifier, sql);
		const rawType = this.getSQLType().toLowerCase();
		const parenPos = rawType.indexOf("(");
		switch (parenPos + 1 ? rawType.slice(0, parenPos) : rawType) {
			case "numeric":
			case "decimal":
			case "bigint": return sql`cast(${identifier} as text)`;
			case "blob": return sql`hex(${identifier})`;
			default: return identifier;
		}
	}
	mapToDriverValue = (value) => {
		return typeof this.mapTo === "function" ? this.mapTo(value) : value;
	};
};
/**
* Custom sqlite database data type generator
*/
function customType(customTypeParams) {
	return (a, b) => {
		const { name, config } = getColumnNameAndConfig(a, b);
		return new SQLiteCustomColumnBuilder(name, config, customTypeParams);
	};
}
var SQLiteBaseIntegerBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteBaseIntegerBuilder";
	constructor(name, dataType, columnType) {
		super(name, dataType, columnType);
		this.config.autoIncrement = false;
	}
	primaryKey(config) {
		if (config?.autoIncrement) this.config.autoIncrement = true;
		this.config.hasDefault = true;
		return super.primaryKey();
	}
};
var SQLiteBaseInteger = class extends SQLiteColumn {
	static [entityKind] = "SQLiteBaseInteger";
	autoIncrement = this.config.autoIncrement;
	getSQLType() {
		return "integer";
	}
};
var SQLiteIntegerBuilder = class extends SQLiteBaseIntegerBuilder {
	static [entityKind] = "SQLiteIntegerBuilder";
	constructor(name) {
		super(name, "number int53", "SQLiteInteger");
	}
	build(table) {
		return new SQLiteInteger(table, this.config);
	}
};
var SQLiteInteger = class extends SQLiteBaseInteger {
	static [entityKind] = "SQLiteInteger";
};
var SQLiteTimestampBuilder = class extends SQLiteBaseIntegerBuilder {
	static [entityKind] = "SQLiteTimestampBuilder";
	constructor(name, mode) {
		super(name, "object date", "SQLiteTimestamp");
		this.config.mode = mode;
	}
	/**
	* @deprecated Use `default()` with your own expression instead.
	*
	* Adds `DEFAULT (cast((julianday('now') - 2440587.5)*86400000 as integer))` to the column, which is the current epoch timestamp in milliseconds.
	*/
	defaultNow() {
		return this.default(sql`(cast((julianday('now') - 2440587.5)*86400000 as integer))`);
	}
	build(table) {
		return new SQLiteTimestamp(table, this.config);
	}
};
var SQLiteTimestamp = class extends SQLiteBaseInteger {
	static [entityKind] = "SQLiteTimestamp";
	mode = this.config.mode;
	mapFromDriverValue = (value) => {
		if (typeof value === "string") return new Date(value.replaceAll("\"", ""));
		if (this.config.mode === "timestamp") return /* @__PURE__ */ new Date(value * 1e3);
		return new Date(value);
	};
	mapToDriverValue = (value) => {
		if (typeof value === "number") return value;
		const unix = value.getTime();
		if (this.config.mode === "timestamp") return Math.floor(unix / 1e3);
		return unix;
	};
};
var SQLiteBooleanBuilder = class extends SQLiteBaseIntegerBuilder {
	static [entityKind] = "SQLiteBooleanBuilder";
	constructor(name, mode) {
		super(name, "boolean", "SQLiteBoolean");
		this.config.mode = mode;
	}
	build(table) {
		return new SQLiteBoolean(table, this.config);
	}
};
var SQLiteBoolean = class extends SQLiteBaseInteger {
	static [entityKind] = "SQLiteBoolean";
	mode = this.config.mode;
	mapFromDriverValue = (value) => {
		return Number(value) === 1;
	};
	mapToDriverValue = (value) => {
		return value ? 1 : 0;
	};
};
function integer(a, b) {
	const { name, config } = getColumnNameAndConfig(a, b);
	if (config?.mode === "timestamp" || config?.mode === "timestamp_ms") return new SQLiteTimestampBuilder(name, config.mode);
	if (config?.mode === "boolean") return new SQLiteBooleanBuilder(name, config.mode);
	return new SQLiteIntegerBuilder(name);
}
var SQLiteNumericBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteNumericBuilder";
	constructor(name) {
		super(name, "string numeric", "SQLiteNumeric");
	}
	/** @internal */
	build(table) {
		return new SQLiteNumeric(table, this.config);
	}
};
var SQLiteNumeric = class extends SQLiteColumn {
	static [entityKind] = "SQLiteNumeric";
	mapFromDriverValue = (value) => {
		if (typeof value === "string") return value;
		return String(value);
	};
	getSQLType() {
		return "numeric";
	}
};
var SQLiteNumericNumberBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteNumericNumberBuilder";
	constructor(name) {
		super(name, "number", "SQLiteNumericNumber");
	}
	/** @internal */
	build(table) {
		return new SQLiteNumericNumber(table, this.config);
	}
};
var SQLiteNumericNumber = class extends SQLiteColumn {
	static [entityKind] = "SQLiteNumericNumber";
	mapFromDriverValue = (value) => {
		if (typeof value === "number") return value;
		return Number(value);
	};
	mapToDriverValue = String;
	getSQLType() {
		return "numeric";
	}
};
var SQLiteNumericBigIntBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteNumericBigIntBuilder";
	constructor(name) {
		super(name, "bigint int64", "SQLiteNumericBigInt");
	}
	/** @internal */
	build(table) {
		return new SQLiteNumericBigInt(table, this.config);
	}
};
var SQLiteNumericBigInt = class extends SQLiteColumn {
	static [entityKind] = "SQLiteNumericBigInt";
	mapFromDriverValue = BigInt;
	mapToDriverValue = String;
	getSQLType() {
		return "numeric";
	}
};
function numeric(a, b) {
	const { name, config } = getColumnNameAndConfig(a, b);
	const mode = config?.mode;
	return mode === "number" ? new SQLiteNumericNumberBuilder(name) : mode === "bigint" ? new SQLiteNumericBigIntBuilder(name) : new SQLiteNumericBuilder(name);
}
var SQLiteRealBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteRealBuilder";
	constructor(name) {
		super(name, "number double", "SQLiteReal");
	}
	/** @internal */
	build(table) {
		return new SQLiteReal(table, this.config);
	}
};
var SQLiteReal = class extends SQLiteColumn {
	static [entityKind] = "SQLiteReal";
	getSQLType() {
		return "real";
	}
};
function real(name) {
	return new SQLiteRealBuilder(name ?? "");
}
var SQLiteTextBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteTextBuilder";
	constructor(name, config) {
		super(name, config.enum?.length ? "string enum" : "string", "SQLiteText");
		this.config.enumValues = config.enum;
		this.config.length = config.length;
	}
	/** @internal */
	build(table) {
		return new SQLiteText(table, this.config);
	}
};
var SQLiteText = class extends SQLiteColumn {
	static [entityKind] = "SQLiteText";
	enumValues = this.config.enumValues;
	constructor(table, config) {
		super(table, config);
	}
	getSQLType() {
		return `text${this.config.length ? `(${this.config.length})` : ""}`;
	}
};
var SQLiteTextJsonBuilder = class extends SQLiteColumnBuilder {
	static [entityKind] = "SQLiteTextJsonBuilder";
	constructor(name) {
		super(name, "object json", "SQLiteTextJson");
	}
	/** @internal */
	build(table) {
		return new SQLiteTextJson(table, this.config);
	}
};
var SQLiteTextJson = class extends SQLiteColumn {
	static [entityKind] = "SQLiteTextJson";
	getSQLType() {
		return "text";
	}
	mapFromDriverValue = (value) => {
		return JSON.parse(value);
	};
	mapToDriverValue = (value) => {
		return JSON.stringify(value);
	};
};
function text(a, b = {}) {
	const { name, config } = getColumnNameAndConfig(a, b);
	if (config.mode === "json") return new SQLiteTextJsonBuilder(name);
	return new SQLiteTextBuilder(name, config);
}
function getSQLiteColumnBuilders() {
	return {
		blob,
		customType,
		integer,
		numeric,
		real,
		text
	};
}
function toSnakeCase(input) {
	return (input.replace(/['\u2019]/g, "").match(/[\da-z]+|[A-Z]+(?![a-z])|[A-Z][\da-z]+/g) ?? []).map((word) => word.toLowerCase()).join("_");
}
function toCamelCase(input) {
	return (input.replace(/['\u2019]/g, "").match(/[\da-z]+|[A-Z]+(?![a-z])|[A-Z][\da-z]+/g) ?? []).reduce((acc, word, i) => {
		return acc + (i === 0 ? word.toLowerCase() : `${word[0].toUpperCase()}${word.slice(1)}`);
	}, "");
}
function getCasingFn(casing) {
	if (casing === "snake_case") return toSnakeCase;
	if (casing === "camelCase") return toCamelCase;
	return (name) => name;
}
/** @internal */
var InlineForeignKeys = Symbol.for("drizzle:SQLiteInlineForeignKeys");
var SQLiteTable = class extends Table {
	static [entityKind] = "SQLiteTable";
	/** @internal */
	static Symbol = Object.assign({}, Table.Symbol, { InlineForeignKeys });
	/** @internal */
	[Table.Symbol.Columns];
	/** @internal */
	[InlineForeignKeys] = [];
	/** @internal */
	[Table.Symbol.ExtraConfigBuilder] = void 0;
};
/** @internal */
function sqliteTableBase(name, columns, extraConfig, schema, casing, baseName = name) {
	const casingFn = getCasingFn(casing);
	const rawTable = new SQLiteTable(name, schema, baseName);
	const parsedColumns = typeof columns === "function" ? columns(getSQLiteColumnBuilders()) : columns;
	const builtColumns = Object.fromEntries(Object.entries(parsedColumns).map(([name, colBuilderBase]) => {
		const colBuilder = colBuilderBase;
		colBuilder.setName(name, casingFn);
		const column = colBuilder.build(rawTable).postBuild();
		rawTable[InlineForeignKeys].push(...colBuilder.buildForeignKeys(column, rawTable));
		return [name, column];
	}));
	const table = Object.assign(rawTable, builtColumns);
	table[Table.Symbol.Columns] = builtColumns;
	table[Table.Symbol.ExtraConfigColumns] = builtColumns;
	if (extraConfig) table[SQLiteTable.Symbol.ExtraConfigBuilder] = extraConfig;
	return table;
}
/** @internal */
function sqliteTableWithCasing(casing) {
	return (name, columns, extraConfig) => sqliteTableBase(name, columns, extraConfig, void 0, casing);
}
var sqliteTable = sqliteTableWithCasing(void 0);
var CheckBuilder = class {
	static [entityKind] = "SQLiteCheckBuilder";
	brand;
	constructor(name, value) {
		this.name = name;
		this.value = value;
	}
	build(table) {
		return new Check(table, this);
	}
};
var Check = class {
	static [entityKind] = "SQLiteCheck";
	name;
	value;
	constructor(table, builder) {
		this.table = table;
		this.name = builder.name;
		this.value = builder.value;
	}
};
function check(name, value) {
	return new CheckBuilder(name, value);
}
var IndexBuilderOn = class {
	static [entityKind] = "SQLiteIndexBuilderOn";
	constructor(name, unique) {
		this.name = name;
		this.unique = unique;
	}
	on(...columns) {
		return new IndexBuilder(this.name, columns, this.unique);
	}
};
var IndexBuilder = class {
	static [entityKind] = "SQLiteIndexBuilder";
	/** @internal */
	config;
	constructor(name, columns, unique) {
		this.config = {
			name,
			columns,
			unique,
			where: void 0
		};
	}
	/**
	* Condition for partial index.
	*/
	where(condition) {
		this.config.where = condition;
		return this;
	}
	/** @internal */
	build(table) {
		return new Index(this.config, table);
	}
};
var Index = class {
	static [entityKind] = "SQLiteIndex";
	config;
	isNameExplicit;
	constructor(config, table) {
		this.config = {
			...config,
			table
		};
		this.isNameExplicit = !!config.name;
	}
};
function index(name) {
	return new IndexBuilderOn(name, false);
}
function uniqueIndex(name) {
	return new IndexBuilderOn(name, true);
}
function primaryKey(...config) {
	if (config[0].columns) return new PrimaryKeyBuilder(config[0].columns, config[0].name);
	return new PrimaryKeyBuilder(config);
}
var PrimaryKeyBuilder = class {
	static [entityKind] = "SQLitePrimaryKeyBuilder";
	/** @internal */
	columns;
	/** @internal */
	name;
	constructor(columns, name) {
		this.columns = columns;
		this.name = name;
	}
	/** @internal */
	build(table) {
		return new PrimaryKey(table, this.columns, this.name);
	}
};
var PrimaryKey = class {
	static [entityKind] = "SQLitePrimaryKey";
	columns;
	name;
	isNameExplicit;
	constructor(table, columns, name) {
		this.table = table;
		this.columns = columns;
		this.name = name;
		this.isNameExplicit = !!name;
	}
	getName() {
		return this.name ?? `${this.table[SQLiteTable.Symbol.Name]}_${this.columns.map((column) => column.name).join("_")}_pk`;
	}
};
function uniqueKeyName(table, columns) {
	return `${table[TableName]}_${columns.join("_")}_unique`;
}
function unique(name) {
	return new UniqueOnConstraintBuilder(name);
}
var UniqueConstraintBuilder = class {
	static [entityKind] = "SQLiteUniqueConstraintBuilder";
	/** @internal */
	columns;
	constructor(columns, name) {
		this.name = name;
		this.columns = columns;
	}
	/** @internal */
	build(table) {
		return new UniqueConstraint(table, this.columns, this.name);
	}
};
var UniqueOnConstraintBuilder = class {
	static [entityKind] = "SQLiteUniqueOnConstraintBuilder";
	/** @internal */
	name;
	constructor(name) {
		this.name = name;
	}
	on(...columns) {
		return new UniqueConstraintBuilder(columns, this.name);
	}
};
var UniqueConstraint = class {
	static [entityKind] = "SQLiteUniqueConstraint";
	columns;
	name;
	isNameExplicit;
	constructor(table, columns, name) {
		this.table = table;
		this.columns = columns;
		this.isNameExplicit = !!name;
		this.name = name ?? uniqueKeyName(this.table, this.columns.map((column) => column.name));
	}
	getName() {
		return this.name;
	}
};
function extractUsedTable(table) {
	if (is(table, SQLiteTable)) return [`${table[Table.Symbol.BaseName]}`];
	if (is(table, Subquery)) return table._.usedTables ?? [];
	if (is(table, SQL)) return table.usedTables ?? [];
	return [];
}
var SQLiteViewBase = class extends View {
	static [entityKind] = "SQLiteViewBase";
};
var SelectionProxyHandler = class SelectionProxyHandler {
	static [entityKind] = "SelectionProxyHandler";
	config;
	constructor(config) {
		this.config = { ...config };
	}
	get(subquery, prop) {
		if (prop === "_") return {
			...subquery["_"],
			selectedFields: new Proxy(subquery._.selectedFields, this)
		};
		if (prop === ViewBaseConfig) return {
			...subquery[ViewBaseConfig],
			selectedFields: new Proxy(subquery[ViewBaseConfig].selectedFields, this)
		};
		if (typeof prop === "symbol") return subquery[prop];
		const value = (is(subquery, Subquery) ? subquery._.selectedFields : is(subquery, View) ? subquery[ViewBaseConfig].selectedFields : subquery)[prop];
		if (is(value, SQL.Aliased)) {
			if (this.config.sqlAliasedBehavior === "sql" && !value.isSelectionField) return value.sql;
			const newValue = value.clone();
			newValue.isSelectionField = true;
			newValue.origin = this.config.alias;
			return newValue;
		}
		if (is(value, SQL)) {
			if (this.config.sqlBehavior === "sql") return value;
			throw new Error(`You tried to reference "${prop}" field from a subquery, which is a raw SQL field, but it doesn't have an alias declared. Please add an alias to the field using ".as('alias')" method.`);
		}
		if (is(value, Column)) {
			if (this.config.alias) return new Proxy(value, new ColumnTableAliasProxyHandler(new Proxy(value.table, new TableAliasProxyHandler(this.config.alias, this.config.replaceOriginalName ?? false, true)), true));
			return value;
		}
		if (typeof value !== "object" || value === null) return value;
		return new Proxy(value, new SelectionProxyHandler(this.config));
	}
};
var TypedQueryBuilder = class {
	static [entityKind] = "TypedQueryBuilder";
	/** @internal */
	getSelectedFields() {
		return this._.selectedFields;
	}
	/** @internal */
	withoutSelectionCastCodecs() {
		return this;
	}
};
var SQLiteSelectBuilder = class {
	static [entityKind] = "SQLiteSelectBuilder";
	fields;
	session;
	dialect;
	withList;
	distinct;
	constructor(config, builder = SQLiteSelectBase) {
		this.builder = builder;
		this.fields = config.fields;
		this.session = config.session;
		this.dialect = config.dialect;
		this.withList = config.withList;
		this.distinct = config.distinct;
	}
	from(source) {
		const isPartialSelect = !!this.fields;
		let fields;
		if (this.fields) fields = this.fields;
		else if (is(source, Subquery)) fields = Object.fromEntries(Object.keys(source._.selectedFields).map((key) => [key, source[key]]));
		else if (is(source, SQLiteViewBase)) fields = source[ViewBaseConfig].selectedFields;
		else if (is(source, SQL)) fields = {};
		else fields = getTableColumns(source);
		return new this.builder({
			table: source,
			fields,
			isPartialSelect,
			session: this.session,
			dialect: this.dialect,
			withList: this.withList ?? [],
			distinct: this.distinct
		});
	}
};
var SQLiteSelectBase = class extends TypedQueryBuilder {
	static [entityKind] = "SQLiteSelectQueryBuilder";
	_;
	/** @internal */
	config;
	joinsNotNullableMap;
	tableName;
	isPartialSelect;
	session;
	dialect;
	cacheConfig = void 0;
	usedTables = /* @__PURE__ */ new Set();
	constructor({ table, fields, isPartialSelect, session, dialect, withList, distinct }) {
		super();
		this.config = {
			withList,
			table,
			fields: { ...fields },
			distinct,
			setOperators: []
		};
		this.isPartialSelect = isPartialSelect;
		this.session = session;
		this.dialect = dialect;
		this._ = {
			selectedFields: fields,
			config: this.config
		};
		this.tableName = getTableLikeName(table);
		this.joinsNotNullableMap = typeof this.tableName === "string" ? { [this.tableName]: true } : {};
		for (const item of extractUsedTable(table)) this.usedTables.add(item);
	}
	/** @internal */
	getUsedTables() {
		return [...this.usedTables];
	}
	createJoin(joinType) {
		return (table, on) => {
			const baseTableName = this.tableName;
			const tableName = getTableLikeName(table);
			for (const item of extractUsedTable(table)) this.usedTables.add(item);
			if (typeof tableName === "string" && this.config.joins?.some((join) => join.alias === tableName)) throw new Error(`Alias "${tableName}" is already used in this query`);
			if (!this.isPartialSelect) {
				if (Object.keys(this.joinsNotNullableMap).length === 1 && typeof baseTableName === "string") this.config.fields = { [baseTableName]: this.config.fields };
				if (typeof tableName === "string" && !is(table, SQL)) {
					const selection = is(table, Subquery) ? table._.selectedFields : is(table, View) ? table[ViewBaseConfig].selectedFields : table[Table.Symbol.Columns];
					this.config.fields[tableName] = selection;
				}
			}
			if (typeof on === "function") on = on(new Proxy(this.config.fields, new SelectionProxyHandler({
				sqlAliasedBehavior: "sql",
				sqlBehavior: "sql"
			})));
			if (!this.config.joins) this.config.joins = [];
			this.config.joins.push({
				on,
				table,
				joinType,
				alias: tableName
			});
			if (typeof tableName === "string") switch (joinType) {
				case "left":
					this.joinsNotNullableMap[tableName] = false;
					break;
				case "right":
					this.joinsNotNullableMap = Object.fromEntries(Object.entries(this.joinsNotNullableMap).map(([key]) => [key, false]));
					this.joinsNotNullableMap[tableName] = true;
					break;
				case "cross":
				case "inner":
					this.joinsNotNullableMap[tableName] = true;
					break;
				case "full":
					this.joinsNotNullableMap = Object.fromEntries(Object.entries(this.joinsNotNullableMap).map(([key]) => [key, false]));
					this.joinsNotNullableMap[tableName] = false;
			}
			return this;
		};
	}
	/**
	* Executes a `left join` operation by adding another table to the current query.
	*
	* Calling this method associates each row of the table with the corresponding row from the joined table, if a match is found. If no matching row exists, it sets all columns of the joined table to null.
	*
	* See docs: {@link https://orm.drizzle.team/docs/joins#left-join}
	*
	* @param table the table to join.
	* @param on the `on` clause.
	*
	* @example
	*
	* ```ts
	* // Select all users and their pets
	* const usersWithPets: { user: User; pets: Pet | null; }[] = await db.select()
	*   .from(users)
	*   .leftJoin(pets, eq(users.id, pets.ownerId))
	*
	* // Select userId and petId
	* const usersIdsAndPetIds: { userId: number; petId: number | null; }[] = await db.select({
	*   userId: users.id,
	*   petId: pets.id,
	* })
	*   .from(users)
	*   .leftJoin(pets, eq(users.id, pets.ownerId))
	* ```
	*/
	leftJoin = this.createJoin("left");
	/**
	* Executes a `right join` operation by adding another table to the current query.
	*
	* Calling this method associates each row of the joined table with the corresponding row from the main table, if a match is found. If no matching row exists, it sets all columns of the main table to null.
	*
	* See docs: {@link https://orm.drizzle.team/docs/joins#right-join}
	*
	* @param table the table to join.
	* @param on the `on` clause.
	*
	* @example
	*
	* ```ts
	* // Select all users and their pets
	* const usersWithPets: { user: User | null; pets: Pet; }[] = await db.select()
	*   .from(users)
	*   .rightJoin(pets, eq(users.id, pets.ownerId))
	*
	* // Select userId and petId
	* const usersIdsAndPetIds: { userId: number | null; petId: number; }[] = await db.select({
	*   userId: users.id,
	*   petId: pets.id,
	* })
	*   .from(users)
	*   .rightJoin(pets, eq(users.id, pets.ownerId))
	* ```
	*/
	rightJoin = this.createJoin("right");
	/**
	* Executes an `inner join` operation, creating a new table by combining rows from two tables that have matching values.
	*
	* Calling this method retrieves rows that have corresponding entries in both joined tables. Rows without matching entries in either table are excluded, resulting in a table that includes only matching pairs.
	*
	* See docs: {@link https://orm.drizzle.team/docs/joins#inner-join}
	*
	* @param table the table to join.
	* @param on the `on` clause.
	*
	* @example
	*
	* ```ts
	* // Select all users and their pets
	* const usersWithPets: { user: User; pets: Pet; }[] = await db.select()
	*   .from(users)
	*   .innerJoin(pets, eq(users.id, pets.ownerId))
	*
	* // Select userId and petId
	* const usersIdsAndPetIds: { userId: number; petId: number; }[] = await db.select({
	*   userId: users.id,
	*   petId: pets.id,
	* })
	*   .from(users)
	*   .innerJoin(pets, eq(users.id, pets.ownerId))
	* ```
	*/
	innerJoin = this.createJoin("inner");
	/**
	* Executes a `full join` operation by combining rows from two tables into a new table.
	*
	* Calling this method retrieves all rows from both main and joined tables, merging rows with matching values and filling in `null` for non-matching columns.
	*
	* See docs: {@link https://orm.drizzle.team/docs/joins#full-join}
	*
	* @param table the table to join.
	* @param on the `on` clause.
	*
	* @example
	*
	* ```ts
	* // Select all users and their pets
	* const usersWithPets: { user: User | null; pets: Pet | null; }[] = await db.select()
	*   .from(users)
	*   .fullJoin(pets, eq(users.id, pets.ownerId))
	*
	* // Select userId and petId
	* const usersIdsAndPetIds: { userId: number | null; petId: number | null; }[] = await db.select({
	*   userId: users.id,
	*   petId: pets.id,
	* })
	*   .from(users)
	*   .fullJoin(pets, eq(users.id, pets.ownerId))
	* ```
	*/
	fullJoin = this.createJoin("full");
	/**
	* Executes a `cross join` operation by combining rows from two tables into a new table.
	*
	* Calling this method retrieves all rows from both main and joined tables, merging all rows from each table.
	*
	* See docs: {@link https://orm.drizzle.team/docs/joins#cross-join}
	*
	* @param table the table to join.
	*
	* @example
	*
	* ```ts
	* // Select all users, each user with every pet
	* const usersWithPets: { user: User; pets: Pet; }[] = await db.select()
	*   .from(users)
	*   .crossJoin(pets)
	*
	* // Select userId and petId
	* const usersIdsAndPetIds: { userId: number; petId: number; }[] = await db.select({
	*   userId: users.id,
	*   petId: pets.id,
	* })
	*   .from(users)
	*   .crossJoin(pets)
	* ```
	*/
	crossJoin = this.createJoin("cross");
	createSetOperator(type, isAll) {
		return (rightSelection) => {
			const rightSelect = typeof rightSelection === "function" ? rightSelection(getSQLiteSetOperators()) : rightSelection;
			if (!haveSameKeys(this.getSelectedFields(), rightSelect.getSelectedFields())) throw new Error("Set operator error (union / intersect / except): selected fields are not the same or are in a different order");
			this.config.setOperators.push({
				type,
				isAll,
				rightSelect
			});
			return this;
		};
	}
	/**
	* Adds `union` set operator to the query.
	*
	* Calling this method will combine the result sets of the `select` statements and remove any duplicate rows that appear across them.
	*
	* See docs: {@link https://orm.drizzle.team/docs/set-operations#union}
	*
	* @example
	*
	* ```ts
	* // Select all unique names from customers and users tables
	* await db.select({ name: users.name })
	*   .from(users)
	*   .union(
	*     db.select({ name: customers.name }).from(customers)
	*   );
	* // or
	* import { union } from 'drizzle-orm/sqlite-core'
	*
	* await union(
	*   db.select({ name: users.name }).from(users),
	*   db.select({ name: customers.name }).from(customers)
	* );
	* ```
	*/
	union = this.createSetOperator("union", false);
	/**
	* Adds `union all` set operator to the query.
	*
	* Calling this method will combine the result-set of the `select` statements and keep all duplicate rows that appear across them.
	*
	* See docs: {@link https://orm.drizzle.team/docs/set-operations#union-all}
	*
	* @example
	*
	* ```ts
	* // Select all transaction ids from both online and in-store sales
	* await db.select({ transaction: onlineSales.transactionId })
	*   .from(onlineSales)
	*   .unionAll(
	*     db.select({ transaction: inStoreSales.transactionId }).from(inStoreSales)
	*   );
	* // or
	* import { unionAll } from 'drizzle-orm/sqlite-core'
	*
	* await unionAll(
	*   db.select({ transaction: onlineSales.transactionId }).from(onlineSales),
	*   db.select({ transaction: inStoreSales.transactionId }).from(inStoreSales)
	* );
	* ```
	*/
	unionAll = this.createSetOperator("union", true);
	/**
	* Adds `intersect` set operator to the query.
	*
	* Calling this method will retain only the rows that are present in both result sets and eliminate duplicates.
	*
	* See docs: {@link https://orm.drizzle.team/docs/set-operations#intersect}
	*
	* @example
	*
	* ```ts
	* // Select course names that are offered in both departments A and B
	* await db.select({ courseName: depA.courseName })
	*   .from(depA)
	*   .intersect(
	*     db.select({ courseName: depB.courseName }).from(depB)
	*   );
	* // or
	* import { intersect } from 'drizzle-orm/sqlite-core'
	*
	* await intersect(
	*   db.select({ courseName: depA.courseName }).from(depA),
	*   db.select({ courseName: depB.courseName }).from(depB)
	* );
	* ```
	*/
	intersect = this.createSetOperator("intersect", false);
	/**
	* Adds `except` set operator to the query.
	*
	* Calling this method will retrieve all unique rows from the left query, except for the rows that are present in the result set of the right query.
	*
	* See docs: {@link https://orm.drizzle.team/docs/set-operations#except}
	*
	* @example
	*
	* ```ts
	* // Select all courses offered in department A but not in department B
	* await db.select({ courseName: depA.courseName })
	*   .from(depA)
	*   .except(
	*     db.select({ courseName: depB.courseName }).from(depB)
	*   );
	* // or
	* import { except } from 'drizzle-orm/sqlite-core'
	*
	* await except(
	*   db.select({ courseName: depA.courseName }).from(depA),
	*   db.select({ courseName: depB.courseName }).from(depB)
	* );
	* ```
	*/
	except = this.createSetOperator("except", false);
	/** @internal */
	addSetOperators(setOperators) {
		this.config.setOperators.push(...setOperators);
		return this;
	}
	/**
	* Adds a `where` clause to the query.
	*
	* Calling this method will select only those rows that fulfill a specified condition.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#filtering}
	*
	* @param where the `where` clause.
	*
	* @example
	* You can use conditional operators and `sql function` to filter the rows to be selected.
	*
	* ```ts
	* // Select all cars with green color
	* await db.select().from(cars).where(eq(cars.color, 'green'));
	* // or
	* await db.select().from(cars).where(sql`${cars.color} = 'green'`)
	* ```
	*
	* You can logically combine conditional operators with `and()` and `or()` operators:
	*
	* ```ts
	* // Select all BMW cars with a green color
	* await db.select().from(cars).where(and(eq(cars.color, 'green'), eq(cars.brand, 'BMW')));
	*
	* // Select all cars with the green or blue color
	* await db.select().from(cars).where(or(eq(cars.color, 'green'), eq(cars.color, 'blue')));
	* ```
	*/
	where(where) {
		if (typeof where === "function") where = where(new Proxy(this.config.fields, new SelectionProxyHandler({
			sqlAliasedBehavior: "sql",
			sqlBehavior: "sql"
		})));
		this.config.where = where;
		return this;
	}
	/**
	* Adds a `having` clause to the query.
	*
	* Calling this method will select only those rows that fulfill a specified condition. It is typically used with aggregate functions to filter the aggregated data based on a specified condition.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#aggregations}
	*
	* @param having the `having` clause.
	*
	* @example
	*
	* ```ts
	* // Select all brands with more than one car
	* await db.select({
	* 	brand: cars.brand,
	* 	count: sql<number>`cast(count(${cars.id}) as int)`,
	* })
	*   .from(cars)
	*   .groupBy(cars.brand)
	*   .having(({ count }) => gt(count, 1));
	* ```
	*/
	having(having) {
		if (typeof having === "function") having = having(new Proxy(this.config.fields, new SelectionProxyHandler({
			sqlAliasedBehavior: "sql",
			sqlBehavior: "sql"
		})));
		this.config.having = having;
		return this;
	}
	groupBy(...columns) {
		if (typeof columns[0] === "function") {
			const groupBy = columns[0](new Proxy(this.config.fields, new SelectionProxyHandler({
				sqlAliasedBehavior: "alias",
				sqlBehavior: "sql"
			})));
			this.config.groupBy = Array.isArray(groupBy) ? groupBy : [groupBy];
		} else this.config.groupBy = columns;
		return this;
	}
	orderBy(...columns) {
		if (typeof columns[0] === "function") {
			const orderBy = columns[0](new Proxy(this.config.fields, new SelectionProxyHandler({
				sqlAliasedBehavior: "alias",
				sqlBehavior: "sql"
			})));
			const orderByArray = Array.isArray(orderBy) ? orderBy : [orderBy];
			if (this.config.setOperators.length > 0) this.config.setOperators.at(-1).orderBy = orderByArray;
			else this.config.orderBy = orderByArray;
		} else {
			const orderByArray = columns;
			if (this.config.setOperators.length > 0) this.config.setOperators.at(-1).orderBy = orderByArray;
			else this.config.orderBy = orderByArray;
		}
		return this;
	}
	/**
	* Adds a `limit` clause to the query.
	*
	* Calling this method will set the maximum number of rows that will be returned by this query.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#limit--offset}
	*
	* @param limit the `limit` clause.
	*
	* @example
	*
	* ```ts
	* // Get the first 10 people from this query.
	* await db.select().from(people).limit(10);
	* ```
	*/
	limit(limit) {
		if (this.config.setOperators.length > 0) this.config.setOperators.at(-1).limit = limit;
		else this.config.limit = limit;
		return this;
	}
	/**
	* Adds an `offset` clause to the query.
	*
	* Calling this method will skip a number of rows when returning results from this query.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#limit--offset}
	*
	* @param offset the `offset` clause.
	*
	* @example
	*
	* ```ts
	* // Get the 10th-20th people from this query.
	* await db.select().from(people).offset(10).limit(10);
	* ```
	*/
	offset(offset) {
		if (this.config.setOperators.length > 0) this.config.setOperators.at(-1).offset = offset;
		else this.config.offset = offset;
		return this;
	}
	$withCache(config) {
		this.cacheConfig = config === void 0 ? {
			config: {},
			enabled: true,
			autoInvalidate: true
		} : config === false ? { enabled: false } : {
			enabled: true,
			autoInvalidate: true,
			...config
		};
		return this;
	}
	getSQL() {
		this.config.fieldsFlat = orderSelectedFields(this.config.fields);
		return this.dialect.buildSelectQuery(this.config);
	}
	toSQL() {
		return this.dialect.sqlToQuery(this.getSQL());
	}
	as(alias) {
		const usedTables = [];
		usedTables.push(...extractUsedTable(this.config.table));
		if (this.config.joins) for (const it of this.config.joins) usedTables.push(...extractUsedTable(it.table));
		return new Proxy(new Subquery(this.getSQL(), this.config.fields, alias, false, [...new Set(usedTables)]), new SelectionProxyHandler({
			alias,
			sqlAliasedBehavior: "alias",
			sqlBehavior: "error"
		}));
	}
	/** @internal */
	getSelectedFields() {
		return new Proxy(this.config.fields, new SelectionProxyHandler({
			alias: this.tableName,
			sqlAliasedBehavior: "alias",
			sqlBehavior: "error"
		}));
	}
	/** @internal */
	withoutSelectionCastCodecs() {
		return this;
	}
	$dynamic() {
		return this;
	}
};
function createSetOperator(type, isAll) {
	return (leftSelect, rightSelect, ...restSelects) => {
		const setOperators = [rightSelect, ...restSelects].map((select) => ({
			type,
			isAll,
			rightSelect: select
		}));
		for (const setOperator of setOperators) if (!haveSameKeys(leftSelect.getSelectedFields(), setOperator.rightSelect.getSelectedFields())) throw new Error("Set operator error (union / intersect / except): selected fields are not the same or are in a different order");
		return leftSelect.addSetOperators(setOperators);
	};
}
var getSQLiteSetOperators = () => ({
	union,
	unionAll,
	intersect,
	except
});
/**
* Adds `union` set operator to the query.
*
* Calling this method will combine the result sets of the `select` statements and remove any duplicate rows that appear across them.
*
* See docs: {@link https://orm.drizzle.team/docs/set-operations#union}
*
* @example
*
* ```ts
* // Select all unique names from customers and users tables
* import { union } from 'drizzle-orm/sqlite-core'
*
* await union(
*   db.select({ name: users.name }).from(users),
*   db.select({ name: customers.name }).from(customers)
* );
* // or
* await db.select({ name: users.name })
*   .from(users)
*   .union(
*     db.select({ name: customers.name }).from(customers)
*   );
* ```
*/
var union = createSetOperator("union", false);
/**
* Adds `union all` set operator to the query.
*
* Calling this method will combine the result-set of the `select` statements and keep all duplicate rows that appear across them.
*
* See docs: {@link https://orm.drizzle.team/docs/set-operations#union-all}
*
* @example
*
* ```ts
* // Select all transaction ids from both online and in-store sales
* import { unionAll } from 'drizzle-orm/sqlite-core'
*
* await unionAll(
*   db.select({ transaction: onlineSales.transactionId }).from(onlineSales),
*   db.select({ transaction: inStoreSales.transactionId }).from(inStoreSales)
* );
* // or
* await db.select({ transaction: onlineSales.transactionId })
*   .from(onlineSales)
*   .unionAll(
*     db.select({ transaction: inStoreSales.transactionId }).from(inStoreSales)
*   );
* ```
*/
var unionAll = createSetOperator("union", true);
/**
* Adds `intersect` set operator to the query.
*
* Calling this method will retain only the rows that are present in both result sets and eliminate duplicates.
*
* See docs: {@link https://orm.drizzle.team/docs/set-operations#intersect}
*
* @example
*
* ```ts
* // Select course names that are offered in both departments A and B
* import { intersect } from 'drizzle-orm/sqlite-core'
*
* await intersect(
*   db.select({ courseName: depA.courseName }).from(depA),
*   db.select({ courseName: depB.courseName }).from(depB)
* );
* // or
* await db.select({ courseName: depA.courseName })
*   .from(depA)
*   .intersect(
*     db.select({ courseName: depB.courseName }).from(depB)
*   );
* ```
*/
var intersect = createSetOperator("intersect", false);
/**
* Adds `except` set operator to the query.
*
* Calling this method will retrieve all unique rows from the left query, except for the rows that are present in the result set of the right query.
*
* See docs: {@link https://orm.drizzle.team/docs/set-operations#except}
*
* @example
*
* ```ts
* // Select all courses offered in department A but not in department B
* import { except } from 'drizzle-orm/sqlite-core'
*
* await except(
*   db.select({ courseName: depA.courseName }).from(depA),
*   db.select({ courseName: depB.courseName }).from(depB)
* );
* // or
* await db.select({ courseName: depA.courseName })
*   .from(depA)
*   .except(
*     db.select({ courseName: depB.courseName }).from(depB)
*   );
* ```
*/
var except = createSetOperator("except", false);
var SQLiteDialect = class {
	static [entityKind] = "SQLiteDialect";
	mapperGenerators;
	constructor(config) {
		this.mapperGenerators = config?.useJitMappers ? {
			rows: makeJitQueryMapper,
			relationalRows: makeJitRqbMapper
		} : {
			rows: makeDefaultQueryMapper,
			relationalRows: makeDefaultRqbMapper
		};
	}
	escapeName(name) {
		return `"${name.replace(/"/g, "\"\"")}"`;
	}
	escapeParam(_num) {
		return "?";
	}
	escapeString(str) {
		return `'${str.replace(/'/g, "''")}'`;
	}
	buildWithCTE(queries) {
		if (!queries?.length) return void 0;
		const withSqlChunks = [sql`with `];
		for (const [i, w] of queries.entries()) {
			withSqlChunks.push(sql`${sql.identifier(w._.alias)} as (${w._.sql})`);
			if (i < queries.length - 1) withSqlChunks.push(sql`, `);
		}
		withSqlChunks.push(sql` `);
		return sql.join(withSqlChunks);
	}
	buildDeleteQuery({ table, where, returning, withList, limit, orderBy }) {
		const withSql = this.buildWithCTE(withList);
		const returningSql = returning ? sql` returning ${this.buildSelection(returning, { isSingleTable: true })}` : void 0;
		return sql`${withSql}delete from ${table}${where ? sql` where ${where}` : void 0}${returningSql}${this.buildOrderBy(orderBy)}${this.buildLimit(limit)}`;
	}
	buildUpdateSet(table, set) {
		const tableColumns = table[Table.Symbol.Columns];
		const columnNames = Object.keys(tableColumns).filter((colName) => set[colName] !== void 0 || tableColumns[colName]?.onUpdateFn !== void 0);
		const setLength = columnNames.length;
		return sql.join(columnNames.flatMap((colName, i) => {
			const col = tableColumns[colName];
			const onUpdateFnResult = col.onUpdateFn?.();
			const value = set[colName] ?? (is(onUpdateFnResult, SQL) ? onUpdateFnResult : sql.param(onUpdateFnResult, col));
			const res = sql`${sql.identifier(col.name)} = ${value}`;
			if (i < setLength - 1) return [res, sql.raw(", ")];
			return [res];
		}));
	}
	buildUpdateQuery({ table, set, where, returning, withList, joins, from, limit, orderBy }) {
		const withSql = this.buildWithCTE(withList);
		const setSql = this.buildUpdateSet(table, set);
		const fromSql = from && sql.join([sql.raw(" from "), this.buildFromTable(from)]);
		const joinsSql = this.buildJoins(joins);
		const returningSql = returning ? sql` returning ${this.buildSelection(returning, { isSingleTable: true })}` : void 0;
		return sql`${withSql}update ${table} set ${setSql}${fromSql}${joinsSql}${where ? sql` where ${where}` : void 0}${returningSql}${this.buildOrderBy(orderBy)}${this.buildLimit(limit)}`;
	}
	/**
	* Builds selection SQL with provided fields/expressions
	*
	* Examples:
	*
	* `select <selection> from`
	*
	* `insert ... returning <selection>`
	*
	* If `isSingleTable` is true, then columns won't be prefixed with table name
	*/
	buildSelection(fields, { isSingleTable = false } = {}) {
		const columnsLen = fields.length;
		const chunks = fields.flatMap(({ field }, i) => {
			const chunk = [];
			if (is(field, SQL.Aliased)) if (field.isSelectionField) {
				if (!isSingleTable && field.origin !== void 0) chunk.push(sql.identifier(field.origin), sql.raw("."));
				chunk.push(sql.identifier(field.fieldAlias));
			} else {
				const query = field.sql;
				if (isSingleTable) {
					const newSql = new SQL(query.queryChunks.map((c) => {
						if (is(c, Column)) return sql.identifier(c.name);
						return c;
					}));
					chunk.push(query.shouldInlineParams ? newSql.inlineParams() : newSql);
				} else chunk.push(query);
				chunk.push(sql` as ${sql.identifier(field.fieldAlias)}`);
			}
			else if (is(field, SQL)) {
				const query = field;
				if (isSingleTable) {
					const newSql = new SQL(query.queryChunks.map((c) => {
						if (is(c, Column)) return sql.identifier(c.name);
						return c;
					}));
					chunk.push(query.shouldInlineParams ? newSql.inlineParams() : newSql);
				} else chunk.push(query);
			} else if (is(field, Column)) if (field.columnType === "SQLiteNumericBigInt" || field.columnType === "SQLiteNumeric") if (isSingleTable) chunk.push(field.isAlias ? sql`cast(${sql.identifier(getOriginalColumnFromAlias(field).name)} as text) as ${field}` : sql`cast(${sql.identifier(field.name)} as text)`);
			else chunk.push(field.isAlias ? sql`cast(${getOriginalColumnFromAlias(field)} as text) as ${field}` : sql`cast(${field} as text)`);
			else if (isSingleTable) chunk.push(field.isAlias ? sql`${sql.identifier(getOriginalColumnFromAlias(field).name)} as ${field}` : sql.identifier(field.name));
			else chunk.push(field.isAlias ? sql`${getOriginalColumnFromAlias(field)} as ${field}` : field);
			else if (is(field, Subquery)) if (!field._.isWith) chunk.push(sql`(${field._.sql}) ${sql.identifier(field._.alias)}`);
			else chunk.push(field);
			if (i < columnsLen - 1) chunk.push(sql`, `);
			return chunk;
		});
		return sql.join(chunks);
	}
	buildJoins(joins) {
		if (!joins || joins.length === 0) return;
		const joinsArray = [];
		if (joins) for (const [index, joinMeta] of joins.entries()) {
			if (index === 0) joinsArray.push(sql` `);
			const table = joinMeta.table;
			const onSql = joinMeta.on ? sql` on ${joinMeta.on}` : void 0;
			if (is(table, SQLiteTable)) {
				const tableName = table[SQLiteTable.Symbol.Name];
				const tableSchema = table[SQLiteTable.Symbol.Schema];
				const origTableName = table[SQLiteTable.Symbol.OriginalName];
				const alias = tableName === origTableName ? void 0 : joinMeta.alias;
				joinsArray.push(sql`${sql.raw(joinMeta.joinType)} join ${tableSchema ? sql`${sql.identifier(tableSchema)}.` : void 0}${sql.identifier(origTableName)}${alias && sql` ${sql.identifier(alias)}`}${onSql}`);
			} else joinsArray.push(sql`${sql.raw(joinMeta.joinType)} join ${table}${onSql}`);
			if (index < joins.length - 1) joinsArray.push(sql` `);
		}
		return sql.join(joinsArray);
	}
	buildLimit(limit) {
		return typeof limit === "object" || typeof limit === "number" && limit >= 0 ? sql` limit ${limit}` : void 0;
	}
	buildOrderBy(orderBy) {
		const orderByList = [];
		if (orderBy) for (const [index, orderByValue] of orderBy.entries()) {
			orderByList.push(orderByValue);
			if (index < orderBy.length - 1) orderByList.push(sql`, `);
		}
		return orderByList.length > 0 ? sql` order by ${sql.join(orderByList)}` : void 0;
	}
	buildFromTable(table) {
		if (is(table, Table) && table[Table.Symbol.IsAlias]) return sql`${sql`${sql.identifier(table[Table.Symbol.Schema] ?? "")}.`.if(table[Table.Symbol.Schema])}${sql.identifier(table[Table.Symbol.OriginalName])} ${sql.identifier(table[Table.Symbol.Name])}`;
		if (is(table, View) && table[ViewBaseConfig].isAlias) {
			let fullName = sql`${sql.identifier(table[ViewBaseConfig].originalName)}`;
			if (table[ViewBaseConfig].schema) fullName = sql`${sql.identifier(table[ViewBaseConfig].schema)}.${fullName}`;
			return sql`${fullName} ${sql.identifier(table[ViewBaseConfig].name)}`;
		}
		return table;
	}
	buildSelectQuery({ withList, fields, fieldsFlat, where, having, table, joins, orderBy, groupBy, limit, offset, distinct, setOperators }) {
		const fieldsList = fieldsFlat ?? orderSelectedFields(fields);
		for (const f of fieldsList) if (is(f.field, Column) && getTableName(f.field.table) !== (is(table, Subquery) ? table._.alias : is(table, SQLiteViewBase) ? table[ViewBaseConfig].name : is(table, SQL) ? void 0 : getTableName(table)) && !((table) => joins?.some(({ alias }) => alias === (table[Table.Symbol.IsAlias] ? getTableName(table) : table[Table.Symbol.BaseName])))(f.field.table)) {
			const tableName = getTableName(f.field.table);
			throw new Error(`Your "${f.path.join("->")}" field references a column "${tableName}"."${f.field.name}", but the table "${tableName}" is not part of the query! Did you forget to join it?`);
		}
		const isSingleTable = !joins || joins.length === 0;
		const withSql = this.buildWithCTE(withList);
		const distinctSql = distinct ? sql` distinct` : void 0;
		const selection = this.buildSelection(fieldsList, { isSingleTable });
		const tableSql = this.buildFromTable(table);
		const joinsSql = this.buildJoins(joins);
		const whereSql = where ? sql` where ${where}` : void 0;
		const havingSql = having ? sql` having ${having}` : void 0;
		const groupByList = [];
		if (groupBy) for (const [index, groupByValue] of groupBy.entries()) {
			groupByList.push(groupByValue);
			if (index < groupBy.length - 1) groupByList.push(sql`, `);
		}
		const finalQuery = sql`${withSql}select${distinctSql} ${selection} from ${tableSql}${joinsSql}${whereSql}${groupByList.length > 0 ? sql` group by ${sql.join(groupByList)}` : void 0}${havingSql}${this.buildOrderBy(orderBy)}${this.buildLimit(limit)}${offset ? sql` offset ${offset}` : void 0}`;
		if (setOperators.length > 0) return this.buildSetOperations(finalQuery, setOperators);
		return finalQuery;
	}
	buildSetOperations(leftSelect, setOperators) {
		const [setOperator, ...rest] = setOperators;
		if (!setOperator) throw new Error("Cannot pass undefined values to any set operator");
		if (rest.length === 0) return this.buildSetOperationQuery({
			leftSelect,
			setOperator
		});
		return this.buildSetOperations(this.buildSetOperationQuery({
			leftSelect,
			setOperator
		}), rest);
	}
	buildSetOperationQuery({ leftSelect, setOperator: { type, isAll, rightSelect, limit, orderBy, offset } }) {
		const leftChunk = sql`${leftSelect.getSQL()} `;
		const rightChunk = sql`${rightSelect.getSQL()}`;
		let orderBySql;
		if (orderBy && orderBy.length > 0) {
			const orderByValues = [];
			for (const singleOrderBy of orderBy) if (is(singleOrderBy, SQLiteColumn)) orderByValues.push(sql.identifier(singleOrderBy.name));
			else if (is(singleOrderBy, SQL)) {
				for (let i = 0; i < singleOrderBy.queryChunks.length; i++) {
					const chunk = singleOrderBy.queryChunks[i];
					if (is(chunk, SQLiteColumn)) singleOrderBy.queryChunks[i] = sql.identifier(chunk.name);
				}
				orderByValues.push(sql`${singleOrderBy}`);
			} else orderByValues.push(sql`${singleOrderBy}`);
			orderBySql = sql` order by ${sql.join(orderByValues, sql`, `)}`;
		}
		const limitSql = typeof limit === "object" || typeof limit === "number" && limit >= 0 ? sql` limit ${limit}` : void 0;
		const operatorChunk = sql.raw(`${type} ${isAll ? "all " : ""}`);
		const offsetSql = offset ? sql` offset ${offset}` : void 0;
		return sql`${leftChunk}${operatorChunk}${rightChunk}${orderBySql}${limitSql}${offsetSql}`;
	}
	buildInsertQuery({ table, values: valuesOrSelect, onConflict, returning, withList, select }) {
		const valuesSqlList = [];
		const columns = table[Table.Symbol.Columns];
		const colEntries = Object.entries(columns);
		const colEntriesFiltered = select && !is(valuesOrSelect, SQL) ? Object.keys(valuesOrSelect.getSelectedFields()).map((key) => [key, columns[key]]) : colEntries.filter(([_, col]) => !col.shouldDisableInsert());
		const insertOrder = colEntriesFiltered.map(([, column]) => sql.identifier(column.name));
		if (select) {
			const select = valuesOrSelect;
			if (is(select, SQL)) valuesSqlList.push(select);
			else valuesSqlList.push(select.getSQL());
		} else {
			const values = valuesOrSelect;
			valuesSqlList.push(sql.raw("values "));
			for (const [valueIndex, value] of values.entries()) {
				const valueList = [];
				for (const [fieldName, col] of colEntriesFiltered) {
					const colValue = value[fieldName];
					if (colValue === void 0 || is(colValue, Param) && colValue.value === void 0) {
						let defaultValue;
						if (col.default !== null && col.default !== void 0) defaultValue = is(col.default, SQL) ? col.default : sql.param(col.default, col);
						else if (col.defaultFn !== void 0) {
							const defaultFnResult = col.defaultFn();
							defaultValue = is(defaultFnResult, SQL) ? defaultFnResult : sql.param(defaultFnResult, col);
						} else if (!col.default && col.onUpdateFn !== void 0) {
							const onUpdateFnResult = col.onUpdateFn();
							defaultValue = is(onUpdateFnResult, SQL) ? onUpdateFnResult : sql.param(onUpdateFnResult, col);
						} else defaultValue = sql`null`;
						valueList.push(defaultValue);
					} else valueList.push(colValue);
				}
				valuesSqlList.push(valueList);
				if (valueIndex < values.length - 1) valuesSqlList.push(sql`, `);
			}
		}
		const withSql = this.buildWithCTE(withList);
		const valuesSql = sql.join(valuesSqlList);
		const returningSql = returning ? sql` returning ${this.buildSelection(returning, { isSingleTable: true })}` : void 0;
		return sql`${withSql}insert into ${table} ${insertOrder} ${valuesSql}${onConflict?.length ? sql.join(onConflict) : void 0}${returningSql}`;
	}
	sqlToQuery(sql, invokeSource) {
		return sql.toQuery({
			escapeName: this.escapeName,
			escapeParam: this.escapeParam,
			escapeString: this.escapeString,
			invokeSource
		});
	}
	nestedSelectionerror() {
		throw new DrizzleError({ message: `Views with nested selections are not supported by the relational query builder` });
	}
	buildRqbColumn(table, column, key, inJson) {
		if (is(column, Column)) {
			const name = sql`${table}.${sql.identifier(column.name)}`;
			switch (column.columnType) {
				case "SQLiteBigInt":
				case "SQLiteBlobJson":
				case "SQLiteBlobBuffer":
					if (!inJson) return sql`${name} as ${sql.identifier(key)}`;
					return sql`hex(${name}) as ${sql.identifier(key)}`;
				case "SQLiteNumeric":
				case "SQLiteNumericNumber":
				case "SQLiteNumericBigInt": return sql`cast(${name} as text) as ${sql.identifier(key)}`;
				case "SQLiteCustomColumn":
					if (!inJson) return sql`${name} as ${sql.identifier(key)}`;
					return sql`${column.jsonSelectIdentifier(name, sql)} as ${sql.identifier(key)}`;
				default: return sql`${name} as ${sql.identifier(key)}`;
			}
		}
		return sql`${table}.${is(column, SQL.Aliased) ? sql.identifier(column.fieldAlias) : isSQLWrapper(column) ? sql.identifier(key) : this.nestedSelectionerror()} as ${sql.identifier(key)}`;
	}
	unwrapAllColumns = (table, selection, inJson) => {
		return sql.join(Object.entries(table[TableColumns]).map(([k, v]) => {
			selection.push({
				key: k,
				field: v
			});
			return this.buildRqbColumn(table, v, k, inJson);
		}), sql`, `);
	};
	getSelectedTableColumns = (table, columns) => {
		const selectedColumns = [];
		const columnContainer = table[TableColumns];
		const entries = Object.entries(columns);
		let colSelectionMode;
		for (const [k, v] of entries) {
			if (v === void 0) continue;
			colSelectionMode = colSelectionMode || v;
			if (v) {
				const column = columnContainer[k];
				selectedColumns.push({
					column,
					tsName: k
				});
			}
		}
		if (colSelectionMode === false) for (const [k, v] of Object.entries(columnContainer)) {
			if (columns[k] === false) continue;
			selectedColumns.push({
				column: v,
				tsName: k
			});
		}
		return selectedColumns;
	};
	buildColumns = (table, selection, inJson, params) => params?.columns ? (() => {
		const columnIdentifiers = [];
		const selectedColumns = this.getSelectedTableColumns(table, params?.columns);
		for (const { column, tsName } of selectedColumns) {
			columnIdentifiers.push(this.buildRqbColumn(table, column, tsName, inJson));
			selection.push({
				key: tsName,
				field: column
			});
		}
		return columnIdentifiers.length ? sql.join(columnIdentifiers, sql`, `) : void 0;
	})() : this.unwrapAllColumns(table, selection, inJson);
	buildRelationalQuery({ schema, table, tableConfig, queryConfig: config, relationWhere, mode, isNested, errorPath, depth, throughJoin, jsonb }) {
		const selection = [];
		const isSingle = mode === "first";
		const params = config === true ? void 0 : config;
		const currentPath = errorPath ?? "";
		const currentDepth = depth ?? 0;
		if (!currentDepth) table = aliasedTable(table, `d${currentDepth}`);
		const limit = isSingle ? 1 : params?.limit;
		const offset = params?.offset;
		const columns = this.buildColumns(table, selection, !!isNested, params);
		const where = params?.where && relationWhere ? and(relationsFilterToSQL(table, params.where, tableConfig.relations, schema), relationWhere) : params?.where ? relationsFilterToSQL(table, params.where, tableConfig.relations, schema) : relationWhere;
		const order = params?.orderBy ? relationsOrderToSQL(table, params.orderBy) : void 0;
		const extras = params?.extras ? relationExtrasToSQL(table, params.extras) : void 0;
		if (extras) selection.push(...extras.selection);
		const joins = params ? (() => {
			const { with: joins } = params;
			if (!joins) return;
			const withEntries = Object.entries(joins).filter(([_, v]) => v);
			if (!withEntries.length) return;
			return sql.join(withEntries.map(([k, join]) => {
				const relation = tableConfig.relations[k];
				const isSingle = is(relation, One);
				const targetTable = aliasedTable(relation.targetTable, `d${currentDepth + 1}`);
				const throughTable = relation.throughTable ? aliasedTable(relation.throughTable, `tr${currentDepth}`) : void 0;
				const { filter, joinCondition } = relationToSQL(relation, table, targetTable, throughTable);
				const throughJoin = throughTable ? sql` inner join ${getTableAsAliasSQL(throughTable)} on ${joinCondition}` : void 0;
				const innerQuery = this.buildRelationalQuery({
					table: targetTable,
					mode: isSingle ? "first" : "many",
					schema,
					queryConfig: join,
					tableConfig: schema[relation.targetTableName],
					relationWhere: filter,
					isNested: true,
					errorPath: `${currentPath.length ? `${currentPath}.` : ""}${k}`,
					depth: currentDepth + 1,
					throughJoin,
					jsonb
				});
				selection.push({
					field: targetTable,
					key: k,
					selection: innerQuery.selection,
					isArray: !isSingle,
					isOptional: (relation.optional ?? false) || join !== true && !!join.where
				});
				const jsonColumns = sql.join(innerQuery.selection.map((s) => {
					return sql`${sql.raw(this.escapeString(s.key))}, ${s.selection ? sql`${jsonb}(${sql.identifier(s.key)})` : sql.identifier(s.key)}`;
				}), sql`, `);
				const json = isNested ? jsonb : sql`json`;
				return isSingle ? sql`(select ${json}_object(${jsonColumns}) as ${sql.identifier("r")} from (${innerQuery.sql}) as ${sql.identifier("t")}) as ${sql.identifier(k)}` : sql`coalesce((select ${json}_group_array(json_object(${jsonColumns})) as ${sql.identifier("r")} from (${innerQuery.sql}) as ${sql.identifier("t")}), ${jsonb}_array()) as ${sql.identifier(k)}`;
			}), sql`, `);
		})() : void 0;
		const selectionArr = [
			columns,
			extras?.sql,
			joins
		].filter((e) => e !== void 0);
		if (!selectionArr.length) throw new DrizzleError({ message: `No fields selected for table "${tableConfig.name}"${currentPath ? ` ("${currentPath}")` : ""}` });
		return {
			sql: sql`select ${sql.join(selectionArr, sql`, `)} from ${getTableAsAliasSQL(table)}${throughJoin}${sql` where ${where}`.if(where)}${sql` order by ${order}`.if(order)}${sql` limit ${limit}`.if(limit !== void 0)}${sql` offset ${offset}`.if(offset !== void 0)}`,
			selection
		};
	}
};
var QueryBuilder = class {
	static [entityKind] = "SQLiteQueryBuilder";
	dialect;
	dialectConfig;
	constructor(dialect) {
		this.dialect = is(dialect, SQLiteDialect) ? dialect : void 0;
		this.dialectConfig = is(dialect, SQLiteDialect) ? void 0 : dialect;
	}
	$with = (alias, selection) => {
		const queryBuilder = this;
		const as = (qb) => {
			if (typeof qb === "function") qb = qb(queryBuilder);
			return new Proxy(new WithSubquery(qb.getSQL(), selection ?? ("getSelectedFields" in qb ? qb.getSelectedFields() ?? {} : {}), alias, true), new SelectionProxyHandler({
				alias,
				sqlAliasedBehavior: "alias",
				sqlBehavior: "error"
			}));
		};
		return { as };
	};
	with(...queries) {
		const self = this;
		function select(fields) {
			return new SQLiteSelectBuilder({
				fields: fields ?? void 0,
				session: void 0,
				dialect: self.getDialect(),
				withList: queries
			});
		}
		function selectDistinct(fields) {
			return new SQLiteSelectBuilder({
				fields: fields ?? void 0,
				session: void 0,
				dialect: self.getDialect(),
				withList: queries,
				distinct: true
			});
		}
		return {
			select,
			selectDistinct
		};
	}
	select(fields) {
		return new SQLiteSelectBuilder({
			fields: fields ?? void 0,
			session: void 0,
			dialect: this.getDialect()
		});
	}
	selectDistinct(fields) {
		return new SQLiteSelectBuilder({
			fields: fields ?? void 0,
			session: void 0,
			dialect: this.getDialect(),
			distinct: true
		});
	}
	getDialect() {
		if (!this.dialect) this.dialect = new SQLiteDialect(this.dialectConfig);
		return this.dialect;
	}
};
function alias$1(table, alias) {
	return new Proxy(table, new TableAliasProxyHandler(alias, false));
}
var SQLiteCountBuilder = class SQLiteCountBuilder extends SQL {
	static [entityKind] = "SQLiteCountBuilder";
	dialect;
	session;
	static buildCount(source, filters, parens) {
		const query = sql`select count(*) from ${source}${sql` where ${filters}`.if(filters)}`;
		return parens ? sql`(${query})` : query;
	}
	constructor(countConfig) {
		super(SQLiteCountBuilder.buildCount(countConfig.source, countConfig.filters, true).queryChunks);
		this.countConfig = countConfig;
		this.dialect = countConfig.dialect;
		this.session = countConfig.session;
		this.mapWith((e) => {
			if (typeof e === "number") return e;
			return Number(e ?? 0);
		});
	}
	executableSql;
	build() {
		if (!this.executableSql) {
			const { source, filters } = this.countConfig;
			this.executableSql = SQLiteCountBuilder.buildCount(source, filters);
		}
		return this.dialect.sqlToQuery(this.executableSql);
	}
};
var SQLiteAsyncCountBuilder = class extends SQLiteCountBuilder {
	static [entityKind] = "SQLiteAsyncCountBuilder";
	constructor(countConfig) {
		super(countConfig);
	}
	/** @internal */
	executeRaw(placeholderValues) {
		return this.session.prepareQuery(this.build(), "arrays", false, "all", (rows) => {
			const v = rows[0]?.[0];
			if (typeof v === "number") return v;
			return v ? Number(v) : 0;
		}).execute(placeholderValues);
	}
	async execute(placeholderValues) {
		return await this.executeRaw(placeholderValues);
	}
};
applyMixins(SQLiteAsyncCountBuilder, [QueryPromise]);
var SQLiteSyncCountBuilder = class extends SQLiteAsyncCountBuilder {
	static [entityKind] = "SQLiteSyncCountBuilder";
	sync(placeholderValues) {
		return this.executeRaw(placeholderValues).sync();
	}
};
var RelationalQueryBuilder = class {
	static [entityKind] = "SQLiteRelationalQueryBuilderV2";
	constructor(mode, schema, table, tableConfig, dialect, session, forbidJsonb, builder = SQLiteRelationalQuery) {
		this.mode = mode;
		this.schema = schema;
		this.table = table;
		this.tableConfig = tableConfig;
		this.dialect = dialect;
		this.session = session;
		this.forbidJsonb = forbidJsonb;
		this.builder = builder;
	}
	findMany(config) {
		return new this.builder(this.mode, this.schema, this.table, this.tableConfig, this.dialect, this.session, config ?? true, "many", this.forbidJsonb);
	}
	findFirst(config) {
		return new this.builder(this.mode, this.schema, this.table, this.tableConfig, this.dialect, this.session, config ?? true, "first", this.forbidJsonb);
	}
};
var SQLiteRelationalQuery = class {
	static [entityKind] = "SQLiteRelationalQueryV2";
	/** @internal */
	mode;
	/** @internal */
	table;
	/** @internal */
	resultKind;
	constructor(resultKind, schema, table, tableConfig, dialect, session, config, mode, forbidJsonb) {
		this.schema = schema;
		this.tableConfig = tableConfig;
		this.dialect = dialect;
		this.session = session;
		this.config = config;
		this.forbidJsonb = forbidJsonb;
		this.resultKind = resultKind;
		this.mode = mode;
		this.table = table;
	}
	getSQL() {
		return this._getQuery().sql;
	}
	_getQuery() {
		const jsonb = this.forbidJsonb ? sql`json` : sql`jsonb`;
		return this.dialect.buildRelationalQuery({
			schema: this.schema,
			table: this.table,
			tableConfig: this.tableConfig,
			queryConfig: this.config,
			mode: this.mode,
			jsonb
		});
	}
	_toSQL() {
		const query = this._getQuery();
		return {
			query,
			builtQuery: this.dialect.sqlToQuery(query.sql)
		};
	}
	toSQL() {
		return this._toSQL().builtQuery;
	}
};
var SQLiteAsyncRelationalQuery = class extends SQLiteRelationalQuery {
	static [entityKind] = "SQLiteAsyncRelationalQueryV2";
	/** @internal */
	_prepare(prepare = false) {
		const { query, builtQuery } = this._toSQL();
		const mapper = this.dialect.mapperGenerators.relationalRows({
			isFirst: this.mode === "first",
			parseJson: true,
			parseJsonIfString: false,
			rootJsonMappers: false,
			selection: query.selection,
			arrayModeRoot: true
		});
		return this.session.prepareQuery(builtQuery, "arrays", prepare, "all", mapper);
	}
	prepare() {
		return this._prepare(true);
	}
	async execute(placeholderValues) {
		return this._prepare().execute(placeholderValues);
	}
};
var SQLiteSyncRelationalQuery = class extends SQLiteAsyncRelationalQuery {
	static [entityKind] = "SQLiteSyncRelationalQueryV2";
	sync(placeholderValues) {
		return this._prepare().execute(placeholderValues).sync();
	}
};
applyMixins(SQLiteAsyncRelationalQuery, [QueryPromise]);
var SQLiteDeleteBase = class {
	static [entityKind] = "SQLiteDelete";
	/** @internal */
	config;
	constructor(table, session, dialect, withList) {
		this.table = table;
		this.session = session;
		this.dialect = dialect;
		this.config = {
			table,
			withList
		};
	}
	/**
	* Adds a `where` clause to the query.
	*
	* Calling this method will delete only those rows that fulfill a specified condition.
	*
	* See docs: {@link https://orm.drizzle.team/docs/delete}
	*
	* @param where the `where` clause.
	*
	* @example
	* You can use conditional operators and `sql function` to filter the rows to be deleted.
	*
	* ```ts
	* // Delete all cars with green color
	* db.delete(cars).where(eq(cars.color, 'green'));
	* // or
	* db.delete(cars).where(sql`${cars.color} = 'green'`)
	* ```
	*
	* You can logically combine conditional operators with `and()` and `or()` operators:
	*
	* ```ts
	* // Delete all BMW cars with a green color
	* db.delete(cars).where(and(eq(cars.color, 'green'), eq(cars.brand, 'BMW')));
	*
	* // Delete all cars with the green or blue color
	* db.delete(cars).where(or(eq(cars.color, 'green'), eq(cars.color, 'blue')));
	* ```
	*/
	where(where) {
		this.config.where = where;
		return this;
	}
	orderBy(...columns) {
		if (typeof columns[0] === "function") {
			const orderBy = columns[0](new Proxy(this.config.table[Table.Symbol.Columns], new SelectionProxyHandler({
				sqlAliasedBehavior: "alias",
				sqlBehavior: "sql"
			})));
			const orderByArray = Array.isArray(orderBy) ? orderBy : [orderBy];
			this.config.orderBy = orderByArray;
		} else {
			const orderByArray = columns;
			this.config.orderBy = orderByArray;
		}
		return this;
	}
	limit(limit) {
		this.config.limit = limit;
		return this;
	}
	returning(fields = this.table[SQLiteTable.Symbol.Columns]) {
		this.config.returning = orderSelectedFields(fields);
		return this;
	}
	getSQL() {
		return this.dialect.buildDeleteQuery(this.config);
	}
	toSQL() {
		return this.dialect.sqlToQuery(this.getSQL());
	}
	$dynamic() {
		return this;
	}
};
var SQLiteAsyncDeleteBase = class extends SQLiteDeleteBase {
	static [entityKind] = "SQLiteAsyncDelete";
	/** @internal */
	_prepare(prepare = false) {
		return this.session.prepareQuery(this.dialect.sqlToQuery(this.getSQL()), "arrays", prepare, this.config.returning ? "all" : "run", this.config.returning ? this.dialect.mapperGenerators.rows(this.config.returning, void 0) : void 0, {
			type: "delete",
			tables: extractUsedTable(this.config.table)
		});
	}
	prepare() {
		return this._prepare(true);
	}
	run = (placeholderValues) => {
		return this._prepare().run(placeholderValues);
	};
	all = (placeholderValues) => {
		return this._prepare().all(placeholderValues);
	};
	get = (placeholderValues) => {
		return this._prepare().get(placeholderValues);
	};
	values = (placeholderValues) => {
		return this._prepare().values(placeholderValues);
	};
	async execute(placeholderValues) {
		return this._prepare().execute(placeholderValues);
	}
};
applyMixins(SQLiteAsyncDeleteBase, [QueryPromise]);
var SQLiteInsertBuilder = class {
	static [entityKind] = "SQLiteInsertBuilder";
	constructor(table, session, dialect, withList, builder = SQLiteInsertBase) {
		this.table = table;
		this.session = session;
		this.dialect = dialect;
		this.withList = withList;
		this.builder = builder;
	}
	values(values) {
		values = Array.isArray(values) ? values : [values];
		if (values.length === 0) throw new Error("values() must be called with at least one value");
		const mappedValues = values.map((entry) => {
			const result = {};
			const cols = this.table[Table.Symbol.Columns];
			for (const colKey of Object.keys(entry)) {
				const colValue = entry[colKey];
				result[colKey] = is(colValue, SQL) ? colValue : new Param(colValue, cols[colKey]);
			}
			return result;
		});
		return new this.builder(this.table, mappedValues, this.session, this.dialect, this.withList);
	}
	select(selectQuery) {
		const select = typeof selectQuery === "function" ? selectQuery(new QueryBuilder()) : selectQuery;
		if (!is(select, SQL)) {
			const insertCols = Object.keys(this.table[Table.Symbol.Columns]);
			const selected = Object.keys(select._.selectedFields);
			for (const col of selected) if (!insertCols.includes(col)) throw new Error(`Insert select error: column "${col}" does not exist in table "${this.table[Table.Symbol.Name]}"`);
		}
		return new this.builder(this.table, select, this.session, this.dialect, this.withList, true);
	}
};
var SQLiteInsertBase = class {
	static [entityKind] = "SQLiteInsert";
	/** @internal */
	config;
	constructor(table, values, session, dialect, withList, select) {
		this.session = session;
		this.dialect = dialect;
		this.config = {
			table,
			values,
			withList,
			select
		};
	}
	returning(fields = this.config.table[SQLiteTable.Symbol.Columns]) {
		this.config.returning = orderSelectedFields(fields);
		return this;
	}
	/**
	* Adds an `on conflict do nothing` clause to the query.
	*
	* Calling this method simply avoids inserting a row as its alternative action.
	*
	* See docs: {@link https://orm.drizzle.team/docs/insert#on-conflict-do-nothing}
	*
	* @param config The `target` and `where` clauses.
	*
	* @example
	* ```ts
	* // Insert one row and cancel the insert if there's a conflict
	* await db.insert(cars)
	*   .values({ id: 1, brand: 'BMW' })
	*   .onConflictDoNothing();
	*
	* // Explicitly specify conflict target
	* await db.insert(cars)
	*   .values({ id: 1, brand: 'BMW' })
	*   .onConflictDoNothing({ target: cars.id });
	* ```
	*/
	onConflictDoNothing(config = {}) {
		if (!this.config.onConflict) this.config.onConflict = [];
		if (config.target === void 0) this.config.onConflict.push(sql` on conflict do nothing`);
		else {
			const targetSql = Array.isArray(config.target) ? sql`${config.target}` : sql`${[config.target]}`;
			const whereSql = config.where ? sql` where ${config.where}` : sql``;
			this.config.onConflict.push(sql` on conflict ${targetSql} do nothing${whereSql}`);
		}
		return this;
	}
	/**
	* Adds an `on conflict do update` clause to the query.
	*
	* Calling this method will update the existing row that conflicts with the row proposed for insertion as its alternative action.
	*
	* See docs: {@link https://orm.drizzle.team/docs/insert#upserts-and-conflicts}
	*
	* @param config The `target`, `set` and `where` clauses.
	*
	* @example
	* ```ts
	* // Update the row if there's a conflict
	* await db.insert(cars)
	*   .values({ id: 1, brand: 'BMW' })
	*   .onConflictDoUpdate({
	*     target: cars.id,
	*     set: { brand: 'Porsche' }
	*   });
	*
	* // Upsert with 'where' clause
	* await db.insert(cars)
	*   .values({ id: 1, brand: 'BMW' })
	*   .onConflictDoUpdate({
	*     target: cars.id,
	*     set: { brand: 'newBMW' },
	*     where: sql`${cars.createdAt} > '2023-01-01'::date`,
	*   });
	* ```
	*/
	onConflictDoUpdate(config) {
		if (config.where && (config.targetWhere || config.setWhere)) throw new Error("You cannot use both \"where\" and \"targetWhere\"/\"setWhere\" at the same time - \"where\" is deprecated, use \"targetWhere\" or \"setWhere\" instead.");
		if (!this.config.onConflict) this.config.onConflict = [];
		const whereSql = config.where ? sql` where ${config.where}` : void 0;
		const targetWhereSql = config.targetWhere ? sql` where ${config.targetWhere}` : void 0;
		const setWhereSql = config.setWhere ? sql` where ${config.setWhere}` : void 0;
		const targetSql = Array.isArray(config.target) ? sql`${config.target}` : sql`${[config.target]}`;
		const setSql = this.dialect.buildUpdateSet(this.config.table, mapUpdateSet(this.config.table, config.set));
		this.config.onConflict.push(sql` on conflict ${targetSql}${targetWhereSql} do update set ${setSql}${whereSql}${setWhereSql}`);
		return this;
	}
	getSQL() {
		return this.dialect.buildInsertQuery(this.config);
	}
	toSQL() {
		return this.dialect.sqlToQuery(this.getSQL());
	}
	$dynamic() {
		return this;
	}
};
var SQLiteAsyncInsertBase = class extends SQLiteInsertBase {
	static [entityKind] = "SQLiteAsyncInsert";
	/** @internal */
	_prepare(prepare = false) {
		return this.session.prepareQuery(this.dialect.sqlToQuery(this.getSQL()), "arrays", prepare, this.config.returning ? "all" : "run", this.config.returning ? this.dialect.mapperGenerators.rows(this.config.returning, void 0) : void 0, {
			type: "insert",
			tables: extractUsedTable(this.config.table)
		});
	}
	prepare() {
		return this._prepare(true);
	}
	run = (placeholderValues) => {
		return this._prepare().run(placeholderValues);
	};
	all = (placeholderValues) => {
		return this._prepare().all(placeholderValues);
	};
	get = (placeholderValues) => {
		return this._prepare().get(placeholderValues);
	};
	values = (placeholderValues) => {
		return this._prepare().values(placeholderValues);
	};
	async execute() {
		return this._prepare().execute();
	}
};
applyMixins(SQLiteAsyncInsertBase, [QueryPromise]);
var SQLiteRaw = class {
	static [entityKind] = "SQLiteRaw";
	constructor(prepared, sql, query) {
		this.prepared = prepared;
		this.sql = sql;
		this.query = query;
	}
	getSQL() {
		return this.sql;
	}
	getQuery() {
		return this.query;
	}
	_prepare() {
		return this.prepared;
	}
};
var SQLiteAsyncRaw = class extends SQLiteRaw {
	static [entityKind] = "SQLiteAsyncRaw";
	constructor(prepared, sql, query) {
		super(prepared, sql, query);
	}
	execute(placeholderValues) {
		return this.prepared.execute(placeholderValues);
	}
};
applyMixins(SQLiteAsyncRaw, [QueryPromise]);
var SQLiteAsyncSelectBase = class extends SQLiteSelectBase {
	static [entityKind] = "SQLiteAsyncSelect";
	/** @internal */
	_prepare(prepare = false) {
		const query = this.dialect.sqlToQuery(this.getSQL());
		const fieldsList = this.config.fieldsFlat;
		const mapper = this.dialect.mapperGenerators.rows(fieldsList, this.joinsNotNullableMap);
		return this.session.prepareQuery(query, "arrays", prepare, "all", mapper, {
			type: "select",
			tables: [...this.usedTables]
		}, this.cacheConfig);
	}
	prepare() {
		return this._prepare(true);
	}
	run = (placeholderValues) => {
		return this._prepare().run(placeholderValues);
	};
	all = (placeholderValues) => {
		return this._prepare().all(placeholderValues);
	};
	get = (placeholderValues) => {
		return this._prepare().get(placeholderValues);
	};
	values = (placeholderValues) => {
		return this._prepare().values(placeholderValues);
	};
	async execute() {
		return this._prepare().execute();
	}
};
applyMixins(SQLiteAsyncSelectBase, [QueryPromise]);
var SQLiteUpdateBuilder = class {
	static [entityKind] = "SQLiteUpdateBuilder";
	constructor(table, session, dialect, withList, builder = SQLiteUpdateBase) {
		this.table = table;
		this.session = session;
		this.dialect = dialect;
		this.withList = withList;
		this.builder = builder;
	}
	set(values) {
		return new this.builder(this.table, mapUpdateSet(this.table, values), this.session, this.dialect, this.withList);
	}
};
var SQLiteUpdateBase = class {
	static [entityKind] = "SQLiteUpdate";
	/** @internal */
	config;
	constructor(table, set, session, dialect, withList) {
		this.session = session;
		this.dialect = dialect;
		this.config = {
			set,
			table,
			withList,
			joins: []
		};
	}
	from(source) {
		this.config.from = source;
		return this;
	}
	createJoin(joinType) {
		return ((table, on) => {
			const tableName = getTableLikeName(table);
			if (typeof tableName === "string" && this.config.joins.some((join) => join.alias === tableName)) throw new Error(`Alias "${tableName}" is already used in this query`);
			if (typeof on === "function") {
				const from = this.config.from ? is(table, SQLiteTable) ? table[Table.Symbol.Columns] : is(table, Subquery) ? table._.selectedFields : is(table, SQLiteViewBase) ? table[ViewBaseConfig].selectedFields : void 0 : void 0;
				on = on(new Proxy(this.config.table[Table.Symbol.Columns], new SelectionProxyHandler({
					sqlAliasedBehavior: "sql",
					sqlBehavior: "sql"
				})), from && new Proxy(from, new SelectionProxyHandler({
					sqlAliasedBehavior: "sql",
					sqlBehavior: "sql"
				})));
			}
			this.config.joins.push({
				on,
				table,
				joinType,
				alias: tableName
			});
			return this;
		});
	}
	leftJoin = this.createJoin("left");
	rightJoin = this.createJoin("right");
	innerJoin = this.createJoin("inner");
	fullJoin = this.createJoin("full");
	/**
	* Adds a 'where' clause to the query.
	*
	* Calling this method will update only those rows that fulfill a specified condition.
	*
	* See docs: {@link https://orm.drizzle.team/docs/update}
	*
	* @param where the 'where' clause.
	*
	* @example
	* You can use conditional operators and `sql function` to filter the rows to be updated.
	*
	* ```ts
	* // Update all cars with green color
	* db.update(cars).set({ color: 'red' })
	*   .where(eq(cars.color, 'green'));
	* // or
	* db.update(cars).set({ color: 'red' })
	*   .where(sql`${cars.color} = 'green'`)
	* ```
	*
	* You can logically combine conditional operators with `and()` and `or()` operators:
	*
	* ```ts
	* // Update all BMW cars with a green color
	* db.update(cars).set({ color: 'red' })
	*   .where(and(eq(cars.color, 'green'), eq(cars.brand, 'BMW')));
	*
	* // Update all cars with the green or blue color
	* db.update(cars).set({ color: 'red' })
	*   .where(or(eq(cars.color, 'green'), eq(cars.color, 'blue')));
	* ```
	*/
	where(where) {
		this.config.where = where;
		return this;
	}
	orderBy(...columns) {
		if (typeof columns[0] === "function") {
			const orderBy = columns[0](new Proxy(this.config.table[Table.Symbol.Columns], new SelectionProxyHandler({
				sqlAliasedBehavior: "alias",
				sqlBehavior: "sql"
			})));
			const orderByArray = Array.isArray(orderBy) ? orderBy : [orderBy];
			this.config.orderBy = orderByArray;
		} else {
			const orderByArray = columns;
			this.config.orderBy = orderByArray;
		}
		return this;
	}
	limit(limit) {
		this.config.limit = limit;
		return this;
	}
	returning(fields = this.config.table[SQLiteTable.Symbol.Columns]) {
		this.config.returning = orderSelectedFields(fields);
		return this;
	}
	getSQL() {
		return this.dialect.buildUpdateQuery(this.config);
	}
	toSQL() {
		return this.dialect.sqlToQuery(this.getSQL());
	}
	$dynamic() {
		return this;
	}
};
var SQLiteAsyncUpdateBase = class extends SQLiteUpdateBase {
	static [entityKind] = "SQLiteAsyncUpdate";
	/** @internal */
	_prepare(prepare = false) {
		return this.session.prepareQuery(this.dialect.sqlToQuery(this.getSQL()), "arrays", prepare, this.config.returning ? "all" : "run", this.config.returning ? this.dialect.mapperGenerators.rows(this.config.returning, void 0) : void 0, {
			type: "update",
			tables: extractUsedTable(this.config.table)
		});
	}
	prepare() {
		return this._prepare(true);
	}
	run = (placeholderValues) => {
		return this._prepare().run(placeholderValues);
	};
	all = (placeholderValues) => {
		return this._prepare().all(placeholderValues);
	};
	get = (placeholderValues) => {
		return this._prepare().get(placeholderValues);
	};
	values = (placeholderValues) => {
		return this._prepare().values(placeholderValues);
	};
	async execute() {
		return this.config.returning ? this.all() : this.run();
	}
};
applyMixins(SQLiteAsyncUpdateBase, [QueryPromise]);
var SQLiteAsyncDatabase = class {
	static [entityKind] = "BaseSQLiteDatabase";
	query;
	constructor(resultKind, dialect, session, relations, forbidJsonb) {
		this.resultKind = resultKind;
		this.dialect = dialect;
		this.session = session;
		this.forbidJsonb = forbidJsonb;
		this._ = {
			relations,
			session,
			resultKind
		};
		this.query = {};
		for (const [tableName, relation] of Object.entries(relations)) this.query[tableName] = new RelationalQueryBuilder(resultKind, relations, relations[relation.name].table, relation, dialect, session, forbidJsonb, resultKind === "sync" ? SQLiteSyncRelationalQuery : SQLiteAsyncRelationalQuery);
		this.$cache = { invalidate: async (_params) => {} };
	}
	/**
	* Creates a subquery that defines a temporary named result set as a CTE.
	*
	* It is useful for breaking down complex queries into simpler parts and for reusing the result set in subsequent parts of the query.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#with-clause}
	*
	* @param alias The alias for the subquery.
	*
	* Failure to provide an alias will result in a DrizzleTypeError, preventing the subquery from being referenced in other queries.
	*
	* @example
	*
	* ```ts
	* // Create a subquery with alias 'sq' and use it in the select query
	* const sq = db.$with('sq').as(db.select().from(users).where(eq(users.id, 42)));
	*
	* const result = await db.with(sq).select().from(sq);
	* ```
	*
	* To select arbitrary SQL values as fields in a CTE and reference them in other CTEs or in the main query, you need to add aliases to them:
	*
	* ```ts
	* // Select an arbitrary SQL value as a field in a CTE and reference it in the main query
	* const sq = db.$with('sq').as(db.select({
	*   name: sql<string>`upper(${users.name})`.as('name'),
	* })
	* .from(users));
	*
	* const result = await db.with(sq).select({ name: sq.name }).from(sq);
	* ```
	*/
	$with = (alias, selection) => {
		const self = this;
		const as = (qb) => {
			if (typeof qb === "function") qb = qb(new QueryBuilder(self.dialect));
			return new Proxy(new WithSubquery(qb.getSQL(), selection ?? ("getSelectedFields" in qb ? qb.getSelectedFields() ?? {} : {}), alias, true), new SelectionProxyHandler({
				alias,
				sqlAliasedBehavior: "alias",
				sqlBehavior: "error"
			}));
		};
		return { as };
	};
	$count(source, filters) {
		return this.resultKind === "async" ? new SQLiteAsyncCountBuilder({
			source,
			filters,
			session: this.session,
			dialect: this.dialect
		}) : new SQLiteSyncCountBuilder({
			source,
			filters,
			session: this.session,
			dialect: this.dialect
		});
	}
	/**
	* Incorporates a previously defined CTE (using `$with`) into the main query.
	*
	* This method allows the main query to reference a temporary named result set.
	*
	* See docs: {@link https://orm.drizzle.team/docs/select#with-clause}
	*
	* @param queries The CTEs to incorporate into the main query.
	*
	* @example
	*
	* ```ts
	* // Define a subquery 'sq' as a CTE using $with
	* const sq = db.$with('sq').as(db.select().from(users).where(eq(users.id, 42)));
	*
	* // Incorporate the CTE 'sq' into the main query and select from it
	* const result = await db.with(sq).select().from(sq);
	* ```
	*/
	with(...queries) {
		const self = this;
		function select(fields) {
			return new SQLiteSelectBuilder({
				fields: fields ?? void 0,
				session: self.session,
				dialect: self.dialect,
				withList: queries
			}, SQLiteAsyncSelectBase);
		}
		function selectDistinct(fields) {
			return new SQLiteSelectBuilder({
				fields: fields ?? void 0,
				session: self.session,
				dialect: self.dialect,
				withList: queries,
				distinct: true
			}, SQLiteAsyncSelectBase);
		}
		/**
		* Creates an update query.
		*
		* Calling this method without `.where()` clause will update all rows in a table. The `.where()` clause specifies which rows should be updated.
		*
		* Use `.set()` method to specify which values to update.
		*
		* See docs: {@link https://orm.drizzle.team/docs/update}
		*
		* @param table The table to update.
		*
		* @example
		*
		* ```ts
		* // Update all rows in the 'cars' table
		* await db.update(cars).set({ color: 'red' });
		*
		* // Update rows with filters and conditions
		* await db.update(cars).set({ color: 'red' }).where(eq(cars.brand, 'BMW'));
		*
		* // Update with returning clause
		* const updatedCar: Car[] = await db.update(cars)
		*   .set({ color: 'red' })
		*   .where(eq(cars.id, 1))
		*   .returning();
		* ```
		*/
		function update(table) {
			return new SQLiteUpdateBuilder(table, self.session, self.dialect, queries, SQLiteAsyncUpdateBase);
		}
		/**
		* Creates an insert query.
		*
		* Calling this method will create new rows in a table. Use `.values()` method to specify which values to insert.
		*
		* See docs: {@link https://orm.drizzle.team/docs/insert}
		*
		* @param table The table to insert into.
		*
		* @example
		*
		* ```ts
		* // Insert one row
		* await db.insert(cars).values({ brand: 'BMW' });
		*
		* // Insert multiple rows
		* await db.insert(cars).values([{ brand: 'BMW' }, { brand: 'Porsche' }]);
		*
		* // Insert with returning clause
		* const insertedCar: Car[] = await db.insert(cars)
		*   .values({ brand: 'BMW' })
		*   .returning();
		* ```
		*/
		function insert(into) {
			return new SQLiteInsertBuilder(into, self.session, self.dialect, queries, SQLiteAsyncInsertBase);
		}
		/**
		* Creates a delete query.
		*
		* Calling this method without `.where()` clause will delete all rows in a table. The `.where()` clause specifies which rows should be deleted.
		*
		* See docs: {@link https://orm.drizzle.team/docs/delete}
		*
		* @param table The table to delete from.
		*
		* @example
		*
		* ```ts
		* // Delete all rows in the 'cars' table
		* await db.delete(cars);
		*
		* // Delete rows with filters and conditions
		* await db.delete(cars).where(eq(cars.color, 'green'));
		*
		* // Delete with returning clause
		* const deletedCar: Car[] = await db.delete(cars)
		*   .where(eq(cars.id, 1))
		*   .returning();
		* ```
		*/
		function delete_(from) {
			return new SQLiteAsyncDeleteBase(from, self.session, self.dialect, queries);
		}
		return {
			select,
			selectDistinct,
			update,
			insert,
			delete: delete_
		};
	}
	select(fields) {
		return new SQLiteSelectBuilder({
			fields: fields ?? void 0,
			session: this.session,
			dialect: this.dialect
		}, SQLiteAsyncSelectBase);
	}
	selectDistinct(fields) {
		return new SQLiteSelectBuilder({
			fields: fields ?? void 0,
			session: this.session,
			dialect: this.dialect,
			distinct: true
		}, SQLiteAsyncSelectBase);
	}
	/**
	* Creates an update query.
	*
	* Calling this method without `.where()` clause will update all rows in a table. The `.where()` clause specifies which rows should be updated.
	*
	* Use `.set()` method to specify which values to update.
	*
	* See docs: {@link https://orm.drizzle.team/docs/update}
	*
	* @param table The table to update.
	*
	* @example
	*
	* ```ts
	* // Update all rows in the 'cars' table
	* await db.update(cars).set({ color: 'red' });
	*
	* // Update rows with filters and conditions
	* await db.update(cars).set({ color: 'red' }).where(eq(cars.brand, 'BMW'));
	*
	* // Update with returning clause
	* const updatedCar: Car[] = await db.update(cars)
	*   .set({ color: 'red' })
	*   .where(eq(cars.id, 1))
	*   .returning();
	* ```
	*/
	update(table) {
		return new SQLiteUpdateBuilder(table, this.session, this.dialect, void 0, SQLiteAsyncUpdateBase);
	}
	$cache;
	/**
	* Creates an insert query.
	*
	* Calling this method will create new rows in a table. Use `.values()` method to specify which values to insert.
	*
	* See docs: {@link https://orm.drizzle.team/docs/insert}
	*
	* @param table The table to insert into.
	*
	* @example
	*
	* ```ts
	* // Insert one row
	* await db.insert(cars).values({ brand: 'BMW' });
	*
	* // Insert multiple rows
	* await db.insert(cars).values([{ brand: 'BMW' }, { brand: 'Porsche' }]);
	*
	* // Insert with returning clause
	* const insertedCar: Car[] = await db.insert(cars)
	*   .values({ brand: 'BMW' })
	*   .returning();
	* ```
	*/
	insert(into) {
		return new SQLiteInsertBuilder(into, this.session, this.dialect, void 0, SQLiteAsyncInsertBase);
	}
	/**
	* Creates a delete query.
	*
	* Calling this method without `.where()` clause will delete all rows in a table. The `.where()` clause specifies which rows should be deleted.
	*
	* See docs: {@link https://orm.drizzle.team/docs/delete}
	*
	* @param table The table to delete from.
	*
	* @example
	*
	* ```ts
	* // Delete all rows in the 'cars' table
	* await db.delete(cars);
	*
	* // Delete rows with filters and conditions
	* await db.delete(cars).where(eq(cars.color, 'green'));
	*
	* // Delete with returning clause
	* const deletedCar: Car[] = await db.delete(cars)
	*   .where(eq(cars.id, 1))
	*   .returning();
	* ```
	*/
	delete(from) {
		return new SQLiteAsyncDeleteBase(from, this.session, this.dialect);
	}
	run(query) {
		const sequel = typeof query === "string" ? sql.raw(query) : query.getSQL();
		const builtQuery = this.dialect.sqlToQuery(sequel);
		const prepared = this.session.prepareQuery(builtQuery, "raw", false, "run");
		if (this.resultKind === "async") return new SQLiteAsyncRaw(prepared, sequel, builtQuery);
		return this.session.run(sequel);
	}
	all(query) {
		const sequel = typeof query === "string" ? sql.raw(query) : query.getSQL();
		const builtQuery = this.dialect.sqlToQuery(sequel);
		const prepared = this.session.prepareQuery(builtQuery, "objects", false, "all");
		if (this.resultKind === "async") return new SQLiteAsyncRaw(prepared, sequel, builtQuery);
		return this.session.objects(sequel);
	}
	get(query) {
		const sequel = typeof query === "string" ? sql.raw(query) : query.getSQL();
		const builtQuery = this.dialect.sqlToQuery(sequel);
		const prepared = this.session.prepareQuery(builtQuery, "objects", false, "get");
		if (this.resultKind === "async") return new SQLiteAsyncRaw(prepared, sequel, builtQuery);
		return this.session.object(sequel);
	}
	values(query) {
		const sequel = typeof query === "string" ? sql.raw(query) : query.getSQL();
		const builtQuery = this.dialect.sqlToQuery(sequel);
		const prepared = this.session.prepareQuery(builtQuery, "objects", false, "values");
		if (this.resultKind === "async") return new SQLiteAsyncRaw(prepared, sequel, builtQuery);
		return this.session.arrays(sequel);
	}
	transaction(transaction, config) {
		return this.session.transaction(transaction, config);
	}
};
var Cache = class {
	static [entityKind] = "Cache";
};
var NoopCache = class extends Cache {
	static [entityKind] = "NoopCache";
	strategy() {
		return "all";
	}
	async get(_key) {}
	async put(_hashedQuery, _response, _tables, _config) {}
	async onMutate(_params) {}
};
var strategyFor = async (query, params, queryMetadata, withCacheConfig) => {
	if (!queryMetadata) return { type: "skip" };
	const { type, tables } = queryMetadata;
	if ((type === "insert" || type === "update" || type === "delete") && tables.length > 0) return {
		type: "invalidate",
		tables
	};
	if (!withCacheConfig) return { type: "skip" };
	if (!withCacheConfig.enabled) return { type: "skip" };
	if (type === "select") return {
		type: "try",
		key: withCacheConfig.tag ?? await hashQuery(query, params),
		isTag: typeof withCacheConfig.tag !== "undefined",
		autoInvalidate: withCacheConfig.autoInvalidate,
		tables: queryMetadata.tables,
		config: withCacheConfig.config
	};
	return { type: "skip" };
};
async function hashQuery(sql, params) {
	const dataToHash = `${sql}-${JSON.stringify(params, (_, v) => typeof v === "bigint" ? `${v}n` : v)}`;
	const data = new TextEncoder().encode(dataToHash);
	const hashBuffer = await crypto.subtle.digest("SHA-256", data);
	return [...new Uint8Array(hashBuffer)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
var SQLitePreparedQuery = class {
	static [entityKind] = "SQLiteBasePreparedQuery";
	/** @internal */
	mapper;
	/** @internal */
	executeMethod;
	constructor(executeMethod, query, mapper, mode) {
		this.query = query;
		this.mode = mode;
		this.mapper = mapper;
		this.executeMethod = executeMethod;
	}
	getQuery() {
		return this.query;
	}
};
var SQLiteSession = class {
	static [entityKind] = "SQLiteSession";
	constructor(dialect) {
		this.dialect = dialect;
	}
};
var ExecuteResultSync = class extends QueryPromise {
	static [entityKind] = "ExecuteResultSync";
	constructor(resultCb) {
		super();
		this.resultCb = resultCb;
	}
	async execute() {
		return this.resultCb();
	}
	sync() {
		return this.resultCb();
	}
};
var SQLiteAsyncPreparedQuery = class extends SQLitePreparedQuery {
	static [entityKind] = "SQLiteAsyncPreparedQuery";
	fastPath;
	constructor(resultKind, executeMethod = "all", executors, query, mapper, mode, logger, cache, queryMetadata, cacheConfig) {
		super(executeMethod, query, mapper, mode);
		this.resultKind = resultKind;
		this.executors = executors;
		this.logger = logger;
		this.cache = cache;
		this.queryMetadata = queryMetadata;
		this.cacheConfig = cacheConfig;
		if (cache && cache.strategy() === "all" && cacheConfig === void 0) this.cacheConfig = {
			enabled: true,
			autoInvalidate: true
		};
		if (!this.cacheConfig?.enabled) this.cacheConfig = void 0;
		this.fastPath = cacheConfig === void 0 && (cache === void 0 || is(cache, NoopCache));
	}
	/** @internal */
	async queryWithCache(queryString, params, executeMethod, query) {
		const cacheStrat = this.cache !== void 0 && !is(this.cache, NoopCache) ? await strategyFor(queryString, params, this.queryMetadata, this.cacheConfig) : { type: "skip" };
		if (cacheStrat.type === "skip") return query().catch((e) => {
			throw new DrizzleQueryError(queryString, params, e);
		});
		const cache = this.cache;
		if (cacheStrat.type === "invalidate") return Promise.all([query(), cache.onMutate({ tables: cacheStrat.tables })]).then((res) => res[0]).catch((e) => {
			throw new DrizzleQueryError(queryString, params, e);
		});
		if (cacheStrat.type === "try") {
			const { tables, key: _key, isTag, autoInvalidate, config } = cacheStrat;
			const key = `${executeMethod}_${_key}`;
			const fromCache = await cache.get(key, tables, isTag, autoInvalidate);
			if (fromCache === void 0) {
				const result = await query().catch((e) => {
					throw new DrizzleQueryError(queryString, params, e);
				});
				await cache.put(key, result, autoInvalidate ? tables : [], isTag, config);
				return result;
			}
			return fromCache;
		}
		assertUnreachable(cacheStrat);
	}
	run(placeholderValues = {}) {
		const { query, logger, executors, fastPath, resultKind } = this;
		const sql = query._sql ? query._sql.join(" ") : query.sql;
		const params = query.params.length === 0 ? query.params : fillPlaceholders(query.params, placeholderValues);
		logger.logQuery(sql, params);
		if (resultKind === "sync") try {
			return executors.run(params);
		} catch (e) {
			throw new DrizzleQueryError(sql, params, e);
		}
		return fastPath ? executors.run(params).catch((e) => {
			throw new DrizzleQueryError(sql, params, e);
		}) : this.queryWithCache(sql, params, "run", () => executors.run(params));
	}
	all(placeholderValues = {}) {
		const { query, logger, executors, mapper, fastPath, resultKind } = this;
		const sql = query._sql ? query._sql.join(" ") : query.sql;
		const params = query.params.length === 0 ? query.params : fillPlaceholders(query.params, placeholderValues);
		logger.logQuery(sql, params);
		if (resultKind === "sync") {
			let res;
			try {
				res = executors.all(params);
			} catch (e) {
				throw new DrizzleQueryError(sql, params, e);
			}
			if (!mapper) return res;
			return mapper(res);
		}
		const res = fastPath ? executors.all(params).catch((e) => {
			throw new DrizzleQueryError(sql, params, e);
		}) : this.queryWithCache(sql, params, "all", () => executors.all(params));
		if (!mapper) return res;
		return res.then((rows) => mapper(rows));
	}
	get(placeholderValues = {}) {
		const { query, logger, executors, mapper, fastPath, resultKind } = this;
		const sql = query._sql ? query._sql.join(" ") : query.sql;
		const params = query.params.length === 0 ? query.params : fillPlaceholders(query.params, placeholderValues);
		logger.logQuery(sql, params);
		if (resultKind === "sync") {
			let res;
			try {
				res = executors.get(params);
			} catch (e) {
				throw new DrizzleQueryError(sql, params, e);
			}
			if (!res) return void 0;
			if (!mapper) return res;
			return mapper([res])[0];
		}
		const res = fastPath ? executors.get(params).catch((e) => {
			throw new DrizzleQueryError(sql, params, e);
		}) : this.queryWithCache(sql, params, "get", () => executors.get(params));
		if (!mapper) return res.then((row) => row ? row : void 0);
		return res.then((row) => row ? mapper([row])[0] : void 0);
	}
	values(placeholderValues = {}) {
		const { query, logger, executors, fastPath, resultKind } = this;
		const sql = query._sql ? query._sql.join(" ") : query.sql;
		const params = query.params.length === 0 ? query.params : fillPlaceholders(query.params, placeholderValues);
		logger.logQuery(sql, params);
		if (resultKind === "sync") try {
			return executors.values(params);
		} catch (e) {
			throw new DrizzleQueryError(sql, params, e);
		}
		return fastPath ? executors.values(params).catch((e) => {
			throw new DrizzleQueryError(sql, params, e);
		}) : this.queryWithCache(sql, params, "values", () => executors.values(params));
	}
	execute(placeholderValues) {
		if (this.resultKind === "async") return this[this.executeMethod](placeholderValues);
		return new ExecuteResultSync(() => this[this.executeMethod](placeholderValues));
	}
};
var SQLiteAsyncSession = class extends SQLiteSession {
	static [entityKind] = "SQLiteAsyncSession";
	constructor(dialect, resultKind) {
		super(dialect);
		this.resultKind = resultKind;
	}
	run(query) {
		return this.prepareQuery(this.dialect.sqlToQuery(query), "raw", false).run();
	}
	objects(query) {
		return this.prepareQuery(this.dialect.sqlToQuery(query), "objects", false).all();
	}
	object(query) {
		return this.prepareQuery(this.dialect.sqlToQuery(query), "objects", false).get();
	}
	arrays(query) {
		return this.prepareQuery(this.dialect.sqlToQuery(query), "arrays", false).all();
	}
	array(query) {
		return this.prepareQuery(this.dialect.sqlToQuery(query), "arrays", false).get();
	}
};
var SQLiteAsyncTransaction = class extends SQLiteAsyncDatabase {
	static [entityKind] = "SQLiteAsyncTransaction";
	constructor(resultType, dialect, session, relations, nestedIndex = 0, forbidJsonb) {
		super(resultType, dialect, session, relations, forbidJsonb);
		this.nestedIndex = nestedIndex;
	}
	rollback() {
		throw new TransactionRollbackError();
	}
};
/** Run Drizzle queries through a SQLite connection or transaction. */
var Session = class extends SQLiteAsyncSession {
	/** The executing connection or transaction. */
	client;
	/** The Drizzle options. */
	options;
	/** The query logger. */
	logger;
	/** Create the session. */
	constructor(client, options) {
		super(new SQLiteDialect({ useJitMappers: options.jit ?? false }), "async");
		this.client = client;
		this.options = options;
		this.logger = options.logger === true ? new DefaultLogger() : options.logger || new NoopLogger();
	}
	/** Execute a script in one call. */
	async exec(script) {
		await this.client.exec(script);
	}
	/** Prepare a query with Drizzle's result mapping. */
	prepareQuery(...arguments_) {
		const [query, mode, prepare, method, mapper, metadata, cache] = arguments_;
		const client = this.client;
		let statement;
		return new SQLiteAsyncPreparedQuery("async", method, {
			run: async (parameters) => {
				if (!prepare) return client.run(query.sql, ...parameters);
				statement ??= client.prepare(query.sql);
				return (await statement).run(...parameters);
			},
			all: async (parameters) => {
				if (!prepare && mode !== "arrays") return client.all(query.sql, ...parameters);
				statement ??= client.prepare(query.sql);
				return (await statement).safeIntegers(mode === "arrays").raw(mode === "arrays").all(...parameters);
			},
			get: async (parameters) => {
				if (!prepare && mode !== "arrays") return client.get(query.sql, ...parameters);
				statement ??= client.prepare(query.sql);
				return (await statement).safeIntegers(mode === "arrays").raw(mode === "arrays").get(...parameters);
			},
			values: async (parameters) => {
				statement ??= client.prepare(query.sql);
				return (await statement).raw(true).all(...parameters);
			}
		}, query, mapper, mode, this.logger, this.options.cache, metadata, cache);
	}
};
/** A session that starts SQLite transactions. */
var ConnectionSession = class extends Session {
	/** The physical connection. */
	connection;
	/** The relations of queries and transactions. */
	relations;
	/** Create the session. */
	constructor(client, relations, options) {
		super(client, options);
		this.connection = client;
		this.relations = relations;
	}
	/** Commit or roll back a callback in a transaction. */
	transaction(operation, configuration) {
		return this.connection.transactionAsync(async (client) => {
			const session = new TransactionSession(client, this.relations, this.options, 0);
			return await operation(new Transaction("async", session.dialect, session, this.relations));
		})[configuration?.behavior ?? "deferred"]();
	}
};
/** Drizzle queries in one transaction or savepoint. */
var Transaction = class extends SQLiteAsyncTransaction {};
/** Queries and savepoints in one SQLite transaction. */
var TransactionSession = class TransactionSession extends Session {
	/** The relations of nested transactions. */
	relations;
	/** The current savepoint depth. */
	depth;
	/** Create the session. */
	constructor(client, relations, options, depth) {
		super(client, options);
		this.relations = relations;
		this.depth = depth;
	}
	/** Run a nested transaction in a savepoint. */
	async transaction(operation) {
		const name = sql.identifier(`destack_savepoint_${this.depth}`);
		await this.run(sql`SAVEPOINT ${name}`);
		try {
			const session = new TransactionSession(this.client, this.relations, this.options, this.depth + 1);
			const result = await operation(new Transaction("async", session.dialect, session, this.relations));
			await this.run(sql`RELEASE SAVEPOINT ${name}`);
			return result;
		} catch (error) {
			try {
				await this.run(sql`ROLLBACK TO SAVEPOINT ${name}`);
				await this.run(sql`RELEASE SAVEPOINT ${name}`);
			} catch (rollback) {
				throw new AggregateError([error, rollback], "SQLite savepoint and rollback failed");
			}
			throw error;
		}
	}
};
/** Native Drizzle queries over a SQLite client. */
var SqliteNative = class extends SQLiteAsyncDatabase {
	/** The SQLite connection client. */
	$client;
	/** Create the native database. */
	constructor(client, options = {}) {
		const relations = {};
		const session = new ConnectionSession(client, relations, options);
		super("async", session.dialect, session, relations);
		this.$client = client;
	}
};
/** Identify a SQLite transaction to the change triggers. */
async function openTransaction(transaction, connection) {
	if (!connection.isLogged) {
		const [found] = await transaction.all(sql`SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ${LOG_TRANSACTION}`);
		if (!found) return false;
		connection.isLogged = true;
	}
	await transaction.run(sql`INSERT INTO ${sql.identifier(LOG_TRANSACTION)} (slot, id) VALUES (1, ${v7()})`);
	return true;
}
/** Clear the transaction identity before commit. */
async function closeTransaction(transaction) {
	await transaction.run(sql`DELETE FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1`);
}
/** The statements each span ran and their summed duration, in milliseconds. */
var TOTALS = /* @__PURE__ */ new WeakMap();
/** A native Drizzle database and its query lifetime. */
var DatabaseDriver = class {
	/** The native database connection. */
	native;
	/** The shared connection lifecycle. */
	state;
	/** The active transaction state, when present. */
	transaction;
	/** Create the driver. */
	constructor(native, state, transaction) {
		this.native = native;
		this.state = state;
		this.transaction = transaction;
	}
	/** Submit work through the transaction or connection. */
	run(operation) {
		this.state.statements += 1;
		const reported = async () => {
			const span = trace.getActiveSpan();
			const started = performance.now();
			try {
				return await operation();
			} catch (error) {
				throw classifyError(error);
			} finally {
				if (span !== void 0) total(span, performance.now() - started);
			}
		};
		return this.transaction ? this.transaction.run(reported) : this.state.run(reported);
	}
	/**
	* Submit a write and notify readers and writers after it commits outside a transaction.
	*
	* On SQLite, a write outside a transaction runs as one identified transaction.
	*/
	async write(operation, options = {}) {
		const result = await this.run(() => options.isTransaction ? operation(this.native) : this.#identified(operation));
		if (!this.transaction) this.state.commits.notify();
		return result;
	}
	/** Run a SQLite write outside a transaction as one identified transaction. */
	async #identified(operation) {
		if (this.native.dialect !== "sqlite" || this.transaction !== void 0) return await operation(this.native);
		return await this.native.database.transaction(async (transaction) => {
			const isMarked = await openTransaction(transaction, this.state);
			const result = await operation({
				dialect: "sqlite",
				database: transaction
			});
			if (isMarked) await closeTransaction(transaction);
			return result;
		}, { behavior: "immediate" });
	}
	/** Render a statement to its text and parameters. */
	render(statement) {
		return this.native.database.dialect.sqlToQuery(statement);
	}
	/** Read every row of a rendered query. */
	all(query) {
		return this.run(async () => {
			if (this.native.dialect === "sqlite") {
				const { client } = this.native.database.session;
				return await client.all(query.sql, ...query.params);
			} else if (this.native.dialect === "postgresql") {
				const { client } = this.native.database.session;
				return [...await client.unsafe(query.sql, query.params, { prepare: true })];
			} else return assertNever(this.native);
		});
	}
	/** Read every row of a rendered query as value arrays. */
	values(query) {
		return this.run(async () => {
			if (this.native.dialect === "sqlite") {
				const { client } = this.native.database.session;
				return await (await client.prepare(query.sql)).safeIntegers(true).raw(true).all(...query.params);
			} else if (this.native.dialect === "postgresql") {
				const { client } = this.native.database.session;
				return [...await client.unsafe(query.sql, query.params, { prepare: true }).values()];
			} else return assertNever(this.native);
		});
	}
};
/** Add a statement to a span's totals and stamp them on it. */
function total(span, milliseconds) {
	const totals = TOTALS.get(span) ?? {
		statements: 0,
		milliseconds: 0
	};
	totals.statements += 1;
	totals.milliseconds += milliseconds;
	TOTALS.set(span, totals);
	span.setAttributes({
		"destack.db.statements": totals.statements,
		"destack.db.duration": totals.milliseconds
	});
}
/** Translate selected fields. */
function selectFields(fields, compiler) {
	const selection = {};
	for (const [property, field] of Object.entries(fields)) if (field instanceof Column$1) selection[property] = compiler.column(field);
	else if (field instanceof Column) selection[property] = field;
	else if (field instanceof SQL) selection[property] = compiler.expression(field);
	else if (field instanceof SQL.Aliased) {
		const expression = compiler.expression(field.sql);
		selection[property] = Object.assign(expression.as(field.fieldAlias), field, { sql: expression });
	} else if (field instanceof Table$1) selection[property] = selectFields(field[TABLE].columns, compiler);
	else selection[property] = selectFields(field, compiler);
	return selection;
}
/** A typed selection. */
var SelectQuery = class {
	/** The native database driver. */
	driver;
	/** The table compiler. */
	compiler;
	/** The source table. */
	table;
	/** The selected fields. */
	fields;
	/** Whether duplicate rows are removed. */
	isDistinct;
	/** The common table expressions. */
	withList;
	/** Whether joins select every column. */
	isAutomatic;
	/** The joins in evaluation order. */
	joins = [];
	/** The row predicate. */
	predicate;
	/** The grouping expressions. */
	groups = [];
	/** The group predicate. */
	groupPredicate;
	/** The ordering expressions. */
	order = [];
	/** The maximum returned row count. */
	count;
	/** The number of rows skipped. */
	skip;
	/** Create the selection. */
	constructor(driver, compiler, table, fields, options) {
		this.withList = options.withList;
		this.isAutomatic = options.isAutomatic;
		this.driver = driver;
		this.compiler = compiler;
		this.table = table;
		this.fields = fields;
		this.isDistinct = options.isDistinct;
	}
	/** Filter rows before grouping. */
	where(predicate) {
		this.predicate = predicate;
		return this;
	}
	/** Include matching rows from another table. */
	innerJoin(table, on) {
		this.#join("inner", table, on);
		return this;
	}
	/** Include matching or null rows from another table. */
	leftJoin(table, on) {
		this.#join("left", table, on);
		return this;
	}
	/** Include every row of the joined table. */
	rightJoin(table, on) {
		this.#join("right", table, on);
		return this;
	}
	/** Include unmatched rows from both sides. */
	fullJoin(table, on) {
		this.#join("full", table, on);
		return this;
	}
	/** Include every row combination. */
	crossJoin(table) {
		this.#join("cross", table);
		return this;
	}
	/** Register a join. */
	#join(kind, table, on) {
		this.joins.push({
			kind,
			table,
			on
		});
		if (this.isAutomatic) Object.assign(this.fields, { [sourceName(table)]: sourceFields(table) });
	}
	/** Group rows by the selected expressions. */
	groupBy(...expressions) {
		this.groups = expressions;
		return this;
	}
	/** Filter grouped rows. */
	having(predicate) {
		this.groupPredicate = predicate;
		return this;
	}
	/** Order rows by the selected expressions. */
	orderBy(...expressions) {
		this.order = expressions;
		return this;
	}
	/** Limit the returned row count. */
	limit(count) {
		this.count = count;
		return this;
	}
	/** Skip rows before returning results. */
	offset(count) {
		this.skip = count;
		return this;
	}
	/** Execute and return the first row, if any. */
	async get() {
		return (await this.driver.run(() => this.#compile(1)))[0];
	}
	/** Execute the selection. */
	async execute() {
		return await this.driver.run(() => this.#compile());
	}
	/** Render the SQL and parameters. */
	toSQL() {
		return this.#compile().toSQL();
	}
	/** Embed the selection as a subquery. */
	getSQL() {
		return this.#compile().getSQL();
	}
	/** Compile a reusable query. */
	prepare() {
		const query = this.#compile().prepare();
		return { execute: async (parameters) => {
			this.driver.transaction?.assertActive();
			return await this.driver.run(() => query.execute(parameters));
		} };
	}
	/** Name the selection for FROM or JOIN. */
	as(alias) {
		return this.#compile().as(alias);
	}
	/** Read the native selected fields. */
	getSelectedFields() {
		return this.#compile().getSelectedFields();
	}
	/** Build the native query. */
	#compile(maximum) {
		this.driver.transaction?.assertActive();
		const table = this.table instanceof Subquery ? this.table : this.compiler.table(this.table);
		for (const join of this.joins) if (!(join.table instanceof Subquery)) this.compiler.table(join.table);
		const fields = selectFields(this.isAutomatic && this.joins.length === 0 ? sourceFields(this.table) : this.fields, this.compiler);
		let query;
		if (this.driver.native.dialect === "sqlite") {
			const database = this.driver.native.database.with(...this.withList);
			const selection = fields;
			query = (this.isDistinct ? database.selectDistinct(selection) : database.select(selection)).from(table).$dynamic();
		} else if (this.driver.native.dialect === "postgresql") {
			const database = this.driver.native.database.with(...this.withList);
			const selection = fields;
			query = (this.isDistinct ? database.selectDistinct(selection) : database.select(selection)).from(table).$dynamic();
		} else return assertNever(this.driver.native);
		for (const join of this.joins) {
			const table = join.table instanceof Subquery ? join.table : this.compiler.table(join.table);
			const on = join.on && this.compiler.expression(join.on);
			switch (join.kind) {
				case "inner":
					query = query.innerJoin(table, on);
					break;
				case "left":
					query = query.leftJoin(table, on);
					break;
				case "right":
					query = query.rightJoin(table, on);
					break;
				case "full":
					query = query.fullJoin(table, on);
					break;
				case "cross":
					query = query.crossJoin(table);
					break;
				default: assertNever(join.kind);
			}
		}
		const expression = (value) => value instanceof Column$1 ? this.compiler.column(value) : value instanceof SQL ? this.compiler.expression(value) : value;
		if (this.predicate) query = query.where(this.compiler.expression(this.predicate));
		if (this.groups.length) query = query.groupBy(...this.groups.map(expression));
		if (this.groupPredicate) query = query.having(this.compiler.expression(this.groupPredicate));
		if (this.order.length) query = query.orderBy(...this.order.map(expression));
		const count = maximum === void 0 ? this.count : Math.min(this.count ?? maximum, maximum);
		if (count !== void 0) query = query.limit(count);
		if (this.skip !== void 0) query = query.offset(this.skip);
		return query;
	}
	/** Await execution. */
	then(fulfilled, rejected) {
		return this.execute().then(fulfilled, rejected);
	}
};
/** A selection awaiting its source table. */
var SelectBuilder = class {
	/** The native database driver. */
	driver;
	/** The table compiler. */
	compiler;
	/** The explicitly selected fields. */
	fields;
	/** Whether duplicate rows are removed. */
	isDistinct;
	/** The common table expressions. */
	withList;
	/** Create the builder. */
	constructor(driver, compiler, fields, options = {}) {
		this.withList = options.withList ?? [];
		this.driver = driver;
		this.compiler = compiler;
		this.fields = fields;
		this.isDistinct = options.isDistinct ?? false;
	}
	/** Select rows from a declared table. */
	from(table) {
		const fields = this.fields ?? { [sourceName(table)]: sourceFields(table) };
		return new SelectQuery(this.driver, this.compiler, table, fields, {
			isDistinct: this.isDistinct,
			isAutomatic: this.fields === void 0,
			withList: this.withList
		});
	}
};
/** Read a source's result key. */
function sourceName(source) {
	return source instanceof Subquery ? source._.alias : source[TABLE].name;
}
/** Read a source's selected fields. */
function sourceFields(source) {
	return source instanceof Subquery ? source._.selectedFields : source[TABLE].columns;
}
/** The rendered plain inserts of each native database, by shape. */
var RENDERED = /* @__PURE__ */ new WeakMap();
/** A typed insertion, update or deletion. */
var MutationQuery = class MutationQuery {
	/** The native database driver. */
	driver;
	/** The table compiler. */
	compiler;
	/** The affected logical table. */
	table;
	/** The SQL operation. */
	operation;
	/** The inserted application records. */
	records = [];
	/** The updated application properties. */
	changes = {};
	/** The row predicate for updates and deletions. */
	predicate;
	/** The fields returned after mutation. */
	fields;
	/** The insert conflict handling. */
	conflict;
	/** Create the mutation. */
	constructor(driver, compiler, table, operation) {
		this.driver = driver;
		this.compiler = compiler;
		this.table = table;
		this.operation = operation;
	}
	/** Supply records for insertion. */
	values(values) {
		this.records = Array.isArray(values) ? values : [values];
		return this;
	}
	/** Supply properties for an update. */
	set(values) {
		this.changes = values;
		return this;
	}
	/** Filter the affected rows. */
	where(predicate) {
		this.predicate = predicate;
		return this;
	}
	/** Ignore inserts that conflict with a unique key. */
	onConflictDoNothing(options = {}) {
		this.conflict = {
			action: "nothing",
			target: options.target === void 0 ? void 0 : columnList(options.target),
			targetWhere: options.where
		};
		return this;
	}
	/** Update a row that conflicts with a unique key. */
	onConflictDoUpdate(options) {
		this.conflict = {
			action: "update",
			...options,
			target: columnList(options.target)
		};
		return this;
	}
	returning(fields = this.table[TABLE].columns) {
		this.fields = fields;
		return this;
	}
	/** Execute the mutation, returning the changed rows when asked. */
	async execute() {
		const result = await this.driver.write((native) => {
			const shape = this.#shape();
			return shape === void 0 ? this.#compile(native) : this.#rendered(native, shape).execute(this.records[0]);
		});
		return this.fields ? result : void 0;
	}
	/** Name the shape of a plain single-record insert, absent for other mutations. */
	#shape() {
		const [record] = this.records;
		const fields = this.fields === void 0 ? [] : Object.entries(this.fields);
		return this.operation === "insert" && this.records.length === 1 && this.conflict === void 0 && Object.values(record).every((value) => !(value instanceof SQL) && !is(value, Placeholder)) && fields.every(([, field]) => field instanceof Column$1) ? `${this.table[TABLE].sqlName}:${defined(record).join(",")}:${fields.map(([name, field]) => `${name}=${field.definition.name}`).join(",")}` : void 0;
	}
	/** Take the rendered statement of a shape, rendering it the first time. */
	#rendered(native, shape) {
		const statements = RENDERED.get(native.database) ?? /* @__PURE__ */ new Map();
		RENDERED.set(native.database, statements);
		const known = statements.get(shape);
		if (known !== void 0) return known;
		const placeholders = Object.fromEntries(defined(this.records[0]).map((property) => [property, sql.placeholder(property)]));
		const rendered = new MutationQuery(this.driver, this.compiler, this.table, "insert").values(placeholders).#withFields(this.fields).#compile(native).prepare();
		statements.set(shape, rendered);
		return rendered;
	}
	/** Copy the returned fields onto another query. */
	#withFields(fields) {
		this.fields = fields;
		return this;
	}
	/** Render the SQL and parameters. */
	toSQL() {
		return this.#compile(this.driver.native).toSQL();
	}
	/** Compile a reusable mutation. */
	prepare() {
		const query = this.#compile(this.driver.native).prepare();
		const hasReturning = this.fields !== void 0;
		return { execute: async (parameters) => {
			this.driver.transaction?.assertActive();
			const result = await this.driver.write((native) => (native === this.driver.native ? query : this.#compile(native).prepare()).execute(parameters));
			return hasReturning ? result : void 0;
		} };
	}
	/** Translate expressions in a record. */
	#values(record) {
		return Object.fromEntries(Object.entries(record).map(([property, value]) => [property, value instanceof SQL ? this.compiler.expression(value) : value]));
	}
	/** Build the native mutation. */
	#compile(native, isReturning = true) {
		this.driver.transaction?.assertActive();
		const values = (record) => this.#values(record);
		const predicate = this.predicate && this.compiler.expression(this.predicate);
		const fields = isReturning && this.fields && selectFields(this.fields, this.compiler);
		const conflict = this.conflict;
		const database = native.database;
		const table = this.compiler.table(this.table);
		let query;
		if (this.operation === "insert") query = database.insert(table).values(this.records.map(values)).$dynamic();
		else if (this.operation === "update") query = database.update(table).set(values(this.changes)).where(predicate);
		else if (this.operation === "delete") query = database.delete(table).where(predicate);
		else return assertNever(this.operation);
		if (conflict?.action === "nothing") query = query.onConflictDoNothing({
			target: conflict.target?.map((column) => this.compiler.column(column)),
			where: conflict.targetWhere && this.compiler.expression(conflict.targetWhere)
		});
		else if (conflict?.action === "update") query = query.onConflictDoUpdate({
			target: conflict.target.map((column) => this.compiler.column(column)),
			set: values(conflict.set),
			targetWhere: conflict.targetWhere && this.compiler.expression(conflict.targetWhere),
			setWhere: conflict.setWhere && this.compiler.expression(conflict.setWhere)
		});
		return fields ? query.returning(fields) : query;
	}
	/** Execute and return the first changed row. */
	async get() {
		return (await this.execute())[0];
	}
	/** Await execution. */
	then(fulfilled, rejected) {
		return this.execute().then(fulfilled, rejected);
	}
};
/** Normalize a conflict key. */
function columnList(columns) {
	return Array.isArray(columns) ? columns : [columns];
}
/** List a record's defined properties. */
function defined(record) {
	return Object.entries(record).flatMap(([property, value]) => value === void 0 ? [] : [property]);
}
/** The most key and value pairs of one JSON function call. */
var JSON_PAIR_LIMIT = Math.floor(999 / 2);
/** The SQLite log: its tables and a table's change triggers. */
var sqliteLog = {
	create: () => createSQLiteLog(),
	install: (description) => sqliteLogTriggers(description),
	remove: (description) => [
		"insert",
		"update",
		"move",
		"delete"
	].map((suffix) => `DROP TRIGGER IF EXISTS ${quote(`${description.table}__change_${suffix}`)}`)
};
/** Create the SQLite log, its horizon and the transaction identity. */
function createSQLiteLog() {
	return [
		`CREATE TABLE IF NOT EXISTS ${quote(LOG)} (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            "transaction" TEXT,
            "table" TEXT NOT NULL,
            key TEXT NOT NULL,
            operation TEXT NOT NULL,
            "row" TEXT NOT NULL,
            previous TEXT,
            scope TEXT NOT NULL,
            retention TEXT NOT NULL,
            changed_at INTEGER NOT NULL
        )`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${quote(LOG)}(retention, changed_at)`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_scope`)} ON ${quote(LOG)}(scope, sequence)`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_transaction_sequence`)} ON ${quote(LOG)}("transaction", sequence)`,
		`CREATE TABLE IF NOT EXISTS ${quote(LOG_HORIZON)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            sequence INTEGER NOT NULL
        )`,
		`CREATE TABLE IF NOT EXISTS ${quote(LOG_TRANSACTION)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            id TEXT NOT NULL
        )`,
		...createEpoch()
	];
}
/** Generate the SQLite triggers recording one table's changes. */
function sqliteLogTriggers(description) {
	const table = quote(description.table);
	const value = (row, name) => description.exact.includes(name) ? `CAST(${row}.${quote(name)} AS TEXT)` : `${row}.${quote(name)}`;
	const key = (row) => `json_array(${description.key.map((name) => value(row, name)).join(", ")})`;
	const row = (source) => {
		const recorded = description.columns.map((name) => `${literal$1(name)}, ${value(source, name)}`);
		return recorded.length ? chunks(recorded, JSON_PAIR_LIMIT).map((chunk) => `json_object(${chunk.join(", ")})`).reduce((merged, next) => `json_patch(${merged}, ${next})`) : "json_object()";
	};
	const log = quote(LOG);
	const transaction = `(SELECT id FROM ${quote(LOG_TRANSACTION)} WHERE slot = 1)`;
	const now = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";
	const scope = (source) => `CAST(${source}.${quote(description.scope)} AS TEXT)`;
	const previous = `json_remove(${chunks(description.columns.map((name) => `CASE WHEN NEW.${quote(name)} IS NOT OLD.${quote(name)} THEN ${literal$1(`$."${name.replaceAll("\"", "\\\"")}"`)} ELSE '$.__unchanged' END, ${value("OLD", name)}`), JSON_PAIR_LIMIT).reduce((merged, chunk) => `json_insert(${merged}, ${chunk.join(", ")})`, "json_object()")}, '$.__unchanged')`;
	const entry = (operation, source, condition = "1 = 1", prior = "NULL") => `INSERT INTO ${log} ("transaction", "table", key, operation, "row", previous, scope, retention, changed_at)
            SELECT
                ${transaction},
                ${literal$1(description.table)},
                ${key(source)},
                ${operation},
                ${row(source)},
                ${prior},
                ${scope(source)},
                ${literal$1(description.retention)},
                ${now}
            WHERE ${condition};`;
	const changed = description.compared.map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`).join(" OR ");
	const moved = [...description.key, description.scope].map((name) => `NEW.${quote(name)} IS NOT OLD.${quote(name)}`).join(" OR ");
	const prefix = `${description.table}__change`;
	return [
		`CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${table} BEGIN
            ${entry("'insert'", "NEW")}
        END`,
		`CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${table} WHEN (${changed}) AND NOT (${moved}) BEGIN
            ${entry("'update'", "NEW", "1 = 1", previous)}
        END`,
		`CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE ON ${table} WHEN ${moved} BEGIN
            ${entry("'delete'", "OLD")}
            ${entry("'insert'", "NEW")}
        END`,
		`CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${table} BEGIN
            ${entry("'delete'", "OLD")}
        END`
	];
}
/** Split values into groups within the argument limit. */
function chunks(values, size) {
	const groups = [];
	for (let start = 0; start < values.length; start += size) groups.push(values.slice(start, start + size));
	return groups;
}
/** The transaction-local setting naming the already stamped transaction. */
var STAMPED_SETTING = "destack.stamped";
/**
* The advisory lock class serialising PostgreSQL commit stamping, keyed by the log's schema.
*
* NOTE #Architecture: one lock per log caps logging commits at about 500 to 1000 a second; following only transactions below the snapshot's xmin, as PgQ does, would drop it.
*/
var COMMIT_LOCK = 471026381;
/** The PostgreSQL log: its tables, functions and triggers. */
var postgresLog = {
	create: () => createPostgresLog(),
	install: (description) => postgresLogTriggers(description),
	remove: (description) => [`DROP TRIGGER IF EXISTS ${quote("destack_change")} ON ${quote(description.table)}`]
};
/** Create the PostgreSQL log, its horizon and its functions. */
function createPostgresLog() {
	const log = quote(LOG);
	return [
		`CREATE TABLE IF NOT EXISTS ${log} (
            id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            sequence BIGINT UNIQUE,
            "transaction" TEXT NOT NULL,
            "table" TEXT NOT NULL,
            key JSONB NOT NULL,
            operation TEXT NOT NULL,
            "row" JSONB NOT NULL,
            previous JSONB,
            scope TEXT NOT NULL,
            retention TEXT NOT NULL,
            changed_at BIGINT NOT NULL
        )`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_compaction`)} ON ${log}(retention, changed_at)`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_scope`)} ON ${log}(scope, sequence)`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_transaction_sequence`)} ON ${log}("transaction", sequence)`,
		`CREATE INDEX IF NOT EXISTS ${quote(`${LOG}_unstamped`)} ON ${log}("transaction", id) WHERE sequence IS NULL`,
		`CREATE SEQUENCE IF NOT EXISTS ${quote(`${LOG}_sequence`)}`,
		`CREATE TABLE IF NOT EXISTS ${quote(LOG_HORIZON)} (
            slot INTEGER PRIMARY KEY CHECK (slot = 1),
            sequence BIGINT NOT NULL
        )`,
		...createEpoch(),
		`CREATE OR REPLACE FUNCTION ${quote(`${LOG}_stamp`)}() RETURNS trigger LANGUAGE plpgsql AS $$
        DECLARE
            unstamped BIGINT;
            base BIGINT;
        BEGIN
            IF current_setting('${STAMPED_SETTING}', true) = NEW."transaction" THEN RETURN NULL; END IF;
            PERFORM pg_advisory_xact_lock(${COMMIT_LOCK}, hashtext(TG_TABLE_SCHEMA));
            SELECT count(*) INTO unstamped FROM ${log} WHERE "transaction" = NEW."transaction" AND sequence IS NULL;
            base := nextval('${LOG}_sequence');
            PERFORM setval('${LOG}_sequence', base + unstamped - 1);
            UPDATE ${log} SET sequence = numbered.sequence
            FROM (
                SELECT id, base + row_number() OVER (ORDER BY id) - 1 AS sequence
                FROM ${log} WHERE "transaction" = NEW."transaction" AND sequence IS NULL
            ) AS numbered
            WHERE ${log}.id = numbered.id;
            PERFORM set_config('${STAMPED_SETTING}', NEW."transaction", true);
            PERFORM pg_notify('${LOG_CHANNEL}', '');
            RETURN NULL;
        END $$`,
		`DO $$ BEGIN
            IF NOT EXISTS (SELECT FROM pg_trigger WHERE tgname = '${LOG}_stamp' AND tgrelid = '${log}'::regclass) THEN
                CREATE CONSTRAINT TRIGGER ${quote(`${LOG}_stamp`)} AFTER INSERT ON ${log}
                    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ${quote(`${LOG}_stamp`)}();
            END IF;
        END $$`,
		`CREATE OR REPLACE FUNCTION ${quote(`${LOG}_record`)}() RETURNS trigger LANGUAGE plpgsql AS $$
        DECLARE
            retention TEXT := TG_ARGV[0];
            key_columns TEXT[] := TG_ARGV[1]::TEXT[];
            recorded TEXT[] := TG_ARGV[2]::TEXT[];
            exact TEXT[] := TG_ARGV[3]::TEXT[];
            scope_column TEXT := TG_ARGV[4];
            old_scope TEXT;
            new_scope TEXT;
            previous JSONB;
            now_ms BIGINT := floor(extract(epoch FROM clock_timestamp()) * 1000);
            transaction_id TEXT := pg_current_xact_id()::TEXT;
            old_row JSONB;
            new_row JSONB;
            old_key JSONB;
            new_key JSONB;
            old_recorded JSONB;
            new_recorded JSONB;
            name TEXT;
        BEGIN
            IF TG_OP <> 'INSERT' THEN old_row := to_jsonb(OLD); END IF;
            IF TG_OP <> 'DELETE' THEN new_row := to_jsonb(NEW); END IF;
            IF TG_OP = 'UPDATE' AND old_row = new_row THEN RETURN NULL; END IF;
            IF old_row IS NOT NULL THEN
                SELECT jsonb_agg(old_row -> column_name ORDER BY position) INTO old_key
                FROM unnest(key_columns) WITH ORDINALITY AS entry(column_name, position);
                SELECT jsonb_object_agg(column_name, old_row -> column_name) INTO old_recorded
                FROM unnest(recorded) AS entry(column_name);
                FOREACH name IN ARRAY exact LOOP
                    IF old_recorded -> name <> 'null'::jsonb THEN
                        old_recorded := jsonb_set(old_recorded, ARRAY[name], to_jsonb(old_recorded ->> name));
                    END IF;
                END LOOP;
            END IF;
            IF new_row IS NOT NULL THEN
                SELECT jsonb_agg(new_row -> column_name ORDER BY position) INTO new_key
                FROM unnest(key_columns) WITH ORDINALITY AS entry(column_name, position);
                SELECT jsonb_object_agg(column_name, new_row -> column_name) INTO new_recorded
                FROM unnest(recorded) AS entry(column_name);
                FOREACH name IN ARRAY exact LOOP
                    IF new_recorded -> name <> 'null'::jsonb THEN
                        new_recorded := jsonb_set(new_recorded, ARRAY[name], to_jsonb(new_recorded ->> name));
                    END IF;
                END LOOP;
            END IF;
            old_scope := old_row ->> scope_column;
            new_scope := new_row ->> scope_column;
            IF TG_OP = 'UPDATE' AND old_key = new_key AND old_scope IS NOT DISTINCT FROM new_scope THEN
                SELECT jsonb_object_agg(entry.key, entry.value) INTO previous
                FROM jsonb_each(old_recorded) AS entry
                WHERE entry.value IS DISTINCT FROM new_recorded -> entry.key;
            END IF;
            IF TG_OP = 'DELETE' OR (TG_OP = 'UPDATE' AND (old_key <> new_key OR old_scope IS DISTINCT FROM new_scope)) THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", scope, retention, changed_at)
                VALUES (transaction_id, TG_TABLE_NAME, old_key, 'delete', old_recorded, old_scope, retention, now_ms);
            END IF;
            IF TG_OP <> 'DELETE' THEN
                INSERT INTO ${log}("transaction", "table", key, operation, "row", previous, scope, retention, changed_at)
                VALUES (transaction_id, TG_TABLE_NAME, new_key,
                    CASE WHEN TG_OP = 'INSERT' OR old_key <> new_key OR old_scope IS DISTINCT FROM new_scope THEN 'insert' ELSE 'update' END,
                    new_recorded, previous, new_scope, retention, now_ms);
            END IF;
            RETURN NULL;
        END $$`
	];
}
/** Generate a table's trigger calling the shared PostgreSQL function. */
function postgresLogTriggers(description) {
	const table = quote(description.table);
	const array = (names) => `{${names.map((name) => `"${name.replaceAll("\"", "\\\"")}"`).join(",")}}`;
	const parameters = [
		literal$1(description.retention),
		literal$1(array(description.key)),
		literal$1(array(description.columns)),
		literal$1(array(description.exact)),
		literal$1(description.scope)
	];
	return [`CREATE TRIGGER ${quote("destack_change")}
            AFTER INSERT OR UPDATE OR DELETE ON ${table}
            FOR EACH ROW
            EXECUTE FUNCTION ${quote(`${LOG}_record`)}(${parameters.join(", ")})`];
}
/** The log of each dialect. */
var LOGS = {
	sqlite: sqliteLog,
	postgresql: postgresLog
};
/** Create the log once per database. */
function createLog(dialect) {
	return LOGS[dialect].create();
}
/** The change triggers of a logged table. */
var logTriggers = {
	install: (state, dialect) => state.log ? LOGS[dialect].install(state.log) : [],
	remove: (state, dialect) => state.log ? LOGS[dialect].remove(state.log) : []
};
/** Generate a tree's indexes and triggers. */
function install$3(tree, dialect) {
	const source = quote(tree.table);
	const closure = quote(tree.ancestors);
	const id = quote(tree.id);
	const scope = quote(tree.scope);
	const parent = quote(tree.parent);
	const prefix = tree.ancestors;
	const prepare = [`CREATE INDEX ${quote(`${prefix}_parent`)} ON ${source} (${scope}, ${parent})`];
	const insert = `
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        VALUES (NEW.${scope}, NEW.${id}, NEW.${id}, 0);
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        SELECT scope, ancestor, NEW.${id}, depth + 1
        FROM ${closure}
        WHERE scope = NEW.${scope}
            AND descendant = NEW.${parent};`;
	const move = `
        DELETE FROM ${closure}
        WHERE scope = OLD.${scope}
            AND descendant IN (
                SELECT descendant
                FROM ${closure}
                WHERE scope = OLD.${scope}
                    AND ancestor = OLD.${id}
            )
            AND ancestor IN (
                SELECT ancestor
                FROM ${closure}
                WHERE scope = OLD.${scope}
                    AND descendant = OLD.${id}
                    AND ancestor <> OLD.${id}
            );
        INSERT INTO ${closure} (scope, ancestor, descendant, depth)
        SELECT above.scope, above.ancestor, below.descendant, above.depth + below.depth + 1
        FROM ${closure} above
        CROSS JOIN ${closure} below
        WHERE above.scope = NEW.${scope}
            AND below.scope = NEW.${scope}
            AND above.descendant = NEW.${parent}
            AND below.ancestor = NEW.${id};`;
	const remove = `
        DELETE FROM ${closure}
        WHERE scope = OLD.${scope}
            AND (ancestor = OLD.${id} OR descendant = OLD.${id});`;
	const missingParent = `
        NEW.${parent} IS NOT NULL
        AND NOT EXISTS (
            SELECT 1
            FROM ${source}
            WHERE ${scope} = NEW.${scope}
                AND ${id} = NEW.${parent}
        )`;
	const cycle = `
        EXISTS (
            SELECT 1
            FROM ${closure}
            WHERE scope = OLD.${scope}
                AND ancestor = OLD.${id}
                AND descendant = NEW.${parent}
        )`;
	const children = `
        EXISTS (
            SELECT 1
            FROM ${source}
            WHERE ${scope} = OLD.${scope}
                AND ${parent} = OLD.${id}
        )`;
	const duplicate = `
        EXISTS (
            SELECT 1
            FROM ${closure}
            WHERE scope = NEW.${scope}
                AND descendant = NEW.${id}
        )`;
	if (dialect === "sqlite") return [
		...prepare,
		`CREATE TRIGGER ${quote(`${prefix}_insert_check`)} BEFORE INSERT ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree identity already exists') WHERE ${duplicate};
            END`,
		`CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${source} BEGIN ${insert} END`,
		`CREATE TRIGGER ${quote(`${prefix}_move_check`)}
            BEFORE UPDATE OF ${id}, ${scope}, ${parent} ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree identity is immutable')
                WHERE NEW.${id} IS NOT OLD.${id} OR NEW.${scope} IS NOT OLD.${scope};
                SELECT RAISE(ABORT, 'tree parent is missing') WHERE ${missingParent};
                SELECT RAISE(ABORT, 'tree move creates a cycle') WHERE ${cycle};
            END`,
		`CREATE TRIGGER ${quote(`${prefix}_move`)} AFTER UPDATE OF ${parent} ON ${source}
                WHEN NEW.${parent} IS NOT OLD.${parent} BEGIN ${move} END`,
		`CREATE TRIGGER ${quote(`${prefix}_delete_check`)} BEFORE DELETE ON ${source} BEGIN
                SELECT RAISE(ABORT, 'tree node has children') WHERE ${children};
            END`,
		`CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${source} BEGIN ${remove} END`
	];
	if (dialect !== "postgresql") return assertNever(dialect);
	const lock = quote(tree.revision);
	const before = quote(`${prefix}_before`);
	const after = quote(`${prefix}_after`);
	return [
		...prepare,
		`CREATE FUNCTION ${before}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            INSERT INTO ${lock} (scope, revision)
            VALUES (CASE WHEN TG_OP = 'DELETE' THEN OLD.${scope} ELSE NEW.${scope} END, 1)
            ON CONFLICT (scope) DO UPDATE SET revision = ${lock}.revision + 1;
            IF TG_OP = 'DELETE' THEN
                IF ${children} THEN RAISE EXCEPTION 'tree node has children'; END IF;
                RETURN OLD;
            END IF;
            IF TG_OP = 'UPDATE' THEN
                IF NEW.${id} IS DISTINCT FROM OLD.${id} OR NEW.${scope} IS DISTINCT FROM OLD.${scope} THEN
                    RAISE EXCEPTION 'tree identity is immutable';
                END IF;
                IF ${cycle} THEN RAISE EXCEPTION 'tree move creates a cycle'; END IF;
            END IF;
            IF ${missingParent} THEN RAISE EXCEPTION 'tree parent is missing'; END IF;
            RETURN NEW;
        END $$`,
		`CREATE FUNCTION ${after}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            IF TG_OP = 'INSERT' THEN ${insert}
            ELSIF TG_OP = 'DELETE' THEN ${remove}
            ELSIF NEW.${parent} IS DISTINCT FROM OLD.${parent} THEN ${move}
            END IF;
            RETURN NULL;
        END $$`,
		`CREATE TRIGGER ${quote(`${prefix}_check`)}
            BEFORE INSERT OR UPDATE OF ${id}, ${scope}, ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${before}()`,
		`CREATE TRIGGER ${quote(`${prefix}_maintain`)}
            AFTER INSERT OR UPDATE OF ${parent} OR DELETE ON ${source}
            FOR EACH ROW
            EXECUTE FUNCTION ${after}()`
	];
}
/** Remove a tree's triggers, functions and parent index. */
function remove$3(tree, dialect) {
	const statements = [`DROP INDEX IF EXISTS ${quote(`${tree.ancestors}_parent`)}`];
	if (dialect === "sqlite") for (const suffix of [
		"insert_check",
		"insert",
		"move_check",
		"move",
		"delete_check",
		"delete"
	]) statements.push(`DROP TRIGGER IF EXISTS ${quote(`${tree.ancestors}_${suffix}`)}`);
	else if (dialect === "postgresql") statements.push(`DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_before`)}() CASCADE`, `DROP FUNCTION IF EXISTS ${quote(`${tree.ancestors}_after`)}() CASCADE`);
	else return assertNever(dialect);
	return statements;
}
/** The triggers keeping tree indexes. */
var treeTriggers = {
	install: (state, dialect) => state.tree ? install$3(state.tree, dialect) : [],
	remove: (state, dialect) => state.tree ? remove$3(state.tree, dialect) : []
};
/** Generate the triggers keeping an aggregate current. */
function install$2(aggregate, dialect) {
	const prefix = triggerPrefix(aggregate);
	const idle = `NOT EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)})`;
	if (dialect === "sqlite") return [
		`CREATE TRIGGER ${quote(`${prefix}_insert`)} AFTER INSERT ON ${quote(aggregate.source)}
                WHEN ${idle} BEGIN ${adjust(aggregate, "NEW", 1, dialect)} END`,
		`CREATE TRIGGER ${quote(`${prefix}_delete`)} AFTER DELETE ON ${quote(aggregate.source)}
                WHEN ${idle} BEGIN ${adjust(aggregate, "OLD", -1, dialect)} END`,
		`CREATE TRIGGER ${quote(`${prefix}_update`)} AFTER UPDATE ON ${quote(aggregate.source)}
                WHEN ${idle} AND (${changed(aggregate, "IS NOT")}) BEGIN
                    ${adjust(aggregate, "OLD", -1, dialect)}
                    ${adjust(aggregate, "NEW", 1, dialect)}
                END`
	];
	else if (dialect === "postgresql") return [`CREATE OR REPLACE FUNCTION ${quote(`${prefix}_maintain`)}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)}) THEN RETURN NULL; END IF;
                IF TG_OP = 'UPDATE' AND NOT (${changed(aggregate, "IS DISTINCT FROM")}) THEN RETURN NULL; END IF;
                IF TG_OP <> 'INSERT' THEN ${adjust(aggregate, "OLD", -1, dialect)} END IF;
                IF TG_OP <> 'DELETE' THEN ${adjust(aggregate, "NEW", 1, dialect)} END IF;
                RETURN NULL;
            END $$`, `CREATE TRIGGER ${quote(`${prefix}_maintain`)}
                AFTER INSERT OR UPDATE OR DELETE ON ${quote(aggregate.source)}
                FOR EACH ROW EXECUTE FUNCTION ${quote(`${prefix}_maintain`)}()`];
	else return assertNever(dialect);
}
/** Remove an aggregate's triggers. */
function remove$2(aggregate, dialect) {
	const prefix = triggerPrefix(aggregate);
	if (dialect === "sqlite") return [
		"insert",
		"delete",
		"update"
	].map((suffix) => `DROP TRIGGER IF EXISTS ${quote(`${prefix}_${suffix}`)}`);
	else if (dialect === "postgresql") return [`DROP FUNCTION IF EXISTS ${quote(`${prefix}_maintain`)}() CASCADE`];
	else return assertNever(dialect);
}
/** Compute an aggregate for every holding row. */
function recomputeAggregate(aggregate, dialect) {
	return `UPDATE ${quote(aggregate.table)} SET ${quote(aggregate.column)} = (${computed(aggregate, dialect)})`;
}
/** Adjust the aggregate of the row a changed row references. */
function adjust(aggregate, row, sign, dialect) {
	const holding = `${quote(aggregate.table)}.${quote(aggregate.id)} = ${row}.${quote(aggregate.key)}`;
	const matching = aggregate.where.map((entry) => `${row}.${quote(entry.column)} ${condition(entry.value, dialect)}`).join(" AND ");
	const guard = matching === "" ? "" : ` AND ${matching}`;
	const column = quote(aggregate.column);
	if (aggregate.function === "count" || aggregate.function === "sum") {
		const amount = aggregate.function === "count" ? "1" : `coalesce(${row}.${quote(aggregate.value)}, 0)`;
		return `UPDATE ${quote(aggregate.table)} SET ${column} = ${column} ${sign > 0 ? "+" : "-"} ${amount} WHERE ${holding}${guard};`;
	}
	return `UPDATE ${quote(aggregate.table)} SET ${column} = (${computed(aggregate, dialect)}) WHERE ${holding};`;
}
/** Select an aggregate of the rows referencing a holding row. */
function computed(aggregate, dialect) {
	const source = quote(aggregate.source);
	const expression = aggregate.function === "count" ? "count(*)" : aggregate.function === "sum" ? `coalesce(sum(${source}.${quote(aggregate.value)}), 0)` : `${aggregate.function}(${source}.${quote(aggregate.value)})`;
	const matching = aggregate.where.map((entry) => ` AND ${source}.${quote(entry.column)} ${condition(entry.value, dialect)}`);
	return `SELECT ${expression} FROM ${source} WHERE ${source}.${quote(aggregate.key)} = ${quote(aggregate.table)}.${quote(aggregate.id)}${matching.join("")}`;
}
/** Build the names of an aggregate's triggers with room for their suffixes. */
function triggerPrefix(aggregate) {
	return boundedName(`${aggregate.source}__${aggregate.table}_${aggregate.column}`, 9);
}
/** Test whether an update changed a column the aggregate reads. */
function changed(aggregate, operator) {
	return [
		aggregate.key,
		...aggregate.value ? [aggregate.value] : [],
		...aggregate.where.map((entry) => entry.column)
	].map((column) => `OLD.${quote(column)} ${operator} NEW.${quote(column)}`).join(" OR ");
}
/** The triggers keeping aggregates current. */
var aggregateTriggers = {
	install: (state, dialect) => (state.aggregates ?? []).flatMap((aggregate) => install$2(aggregate, dialect)),
	remove: (state, dialect) => (state.aggregates ?? []).flatMap((aggregate) => remove$2(aggregate, dialect))
};
/** The message refusing a deletion, classified as a broken reference. */
var RESTRICTED = "FOREIGN KEY constraint failed";
/** Generate the trigger cascading or refusing a row's deletion for its dependents. */
function install$1(dependent, dialect) {
	const name = quote(boundedName(`${dependent.table}__${dependent.source}_${dependent.key}`));
	const idle = `NOT EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)})`;
	const source = quote(dependent.source);
	const rows = `${source} WHERE ${[`${source}.${quote(dependent.key)} = OLD.${quote(dependent.id)}`, ...dependent.where.map((entry) => `${source}.${quote(entry.column)} ${condition(entry.value, dialect)}`)].join(" AND ")}`;
	const message = `${RESTRICTED}: ${dependent.source} references ${dependent.table}`;
	if (dialect === "sqlite") return [dependent.onDelete === "cascade" ? `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                    WHEN ${idle} BEGIN DELETE FROM ${rows}; END` : `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                    WHEN ${idle} AND EXISTS (SELECT 1 FROM ${rows})
                    BEGIN SELECT RAISE(ABORT, ${literal$1(message)}); END`];
	else if (dialect === "postgresql") {
		const effect = dependent.onDelete === "cascade" ? `DELETE FROM ${rows};` : `IF EXISTS (SELECT 1 FROM ${rows}) THEN
                      RAISE EXCEPTION USING ERRCODE = 'foreign_key_violation', MESSAGE = ${literal$1(message)};
                  END IF;`;
		return [`CREATE OR REPLACE FUNCTION ${name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF EXISTS (SELECT 1 FROM ${quote(LOG_COPYING)}) THEN RETURN NULL; END IF;
                ${effect}
                RETURN NULL;
            END $$`, `CREATE TRIGGER ${name} AFTER DELETE ON ${quote(dependent.table)}
                FOR EACH ROW EXECUTE FUNCTION ${name}()`];
	} else return assertNever(dialect);
}
/** Remove a dependent's trigger. */
function remove$1(dependent, dialect) {
	const name = quote(boundedName(`${dependent.table}__${dependent.source}_${dependent.key}`));
	if (dialect === "sqlite") return [`DROP TRIGGER IF EXISTS ${name}`];
	else if (dialect === "postgresql") return [`DROP FUNCTION IF EXISTS ${name}() CASCADE`];
	else return assertNever(dialect);
}
/** The triggers keeping dependents consistent. */
var dependentTriggers = {
	install: (state, dialect) => (state.dependents ?? []).flatMap((dependent) => install$1(dependent, dialect)),
	remove: (state, dialect) => (state.dependents ?? []).flatMap((dependent) => remove$1(dependent, dialect))
};
/** Create a table; PostgreSQL adds foreign keys separately. */
function createTable(table, name = table.name) {
	const definitions = [...table.columns.map((column) => columnDefinition(column, table.dialect)), ...table.constraints.filter((constraint) => constraint.kind !== "foreignKey" || table.dialect === "sqlite").map(constraintDefinition)];
	return `CREATE TABLE ${quote(name)} (\n    ${definitions.join(",\n    ")}\n)`;
}
/** Remove a PostgreSQL constraint. */
function dropConstraint(table, name) {
	return `ALTER TABLE ${quote(table)} DROP CONSTRAINT ${quote(name)}`;
}
/** Add a PostgreSQL constraint to an existing table. */
function addConstraint(table, constraint) {
	return `ALTER TABLE ${quote(table)} ADD ${constraintDefinition(constraint)}`;
}
/** Drop a table. */
function dropTable(name) {
	return `DROP TABLE ${quote(name)}`;
}
/** Rename a table. */
function renameTable(from, to) {
	return `ALTER TABLE ${quote(from)} RENAME TO ${quote(to)}`;
}
/** Add a column to an existing table. */
function addColumn(table, column, dialect) {
	return `ALTER TABLE ${quote(table)} ADD COLUMN ${columnDefinition(column, dialect)}`;
}
/** Drop a column. */
function dropColumn(table, column) {
	return `ALTER TABLE ${quote(table)} DROP COLUMN ${quote(column)}`;
}
/** Rename a column. */
function renameColumn(table, from, to) {
	return `ALTER TABLE ${quote(table)} RENAME COLUMN ${quote(from)} TO ${quote(to)}`;
}
/** Change a PostgreSQL column in place. */
function alterColumn(table, from, to) {
	const target = quote(table);
	const column = quote(to.name);
	const statements = [];
	if (from.type !== to.type) statements.push(`ALTER TABLE ${target} ALTER COLUMN ${column} TYPE ${to.type} USING ${column}::${to.type}`);
	if (from.nullable !== to.nullable) statements.push(`ALTER TABLE ${target} ALTER COLUMN ${column} ${to.nullable ? "DROP" : "SET"} NOT NULL`);
	if (from.default !== to.default) statements.push(to.default === void 0 ? `ALTER TABLE ${target} ALTER COLUMN ${column} DROP DEFAULT` : `ALTER TABLE ${target} ALTER COLUMN ${column} SET DEFAULT ${to.default}`);
	return statements;
}
/** Create an index. */
function createIndex(table, index) {
	const columns = index.columns.map((entry) => "column" in entry ? quote(entry.column) : `(${entry.expression})`).join(", ");
	const where = index.where === void 0 ? "" : ` WHERE ${index.where}`;
	return `CREATE ${index.unique ? "UNIQUE " : ""}INDEX ${quote(index.name)} ON ${quote(table)} (${columns})${where}`;
}
/** Drop an index. */
function dropIndex(name) {
	return `DROP INDEX ${quote(name)}`;
}
/** Rebuild a SQLite table, copying the shared columns. */
function rebuildTable(table, copied) {
	const staging = `${table.name}__rebuild`;
	const targets = [...copied.keys()].map(quote).join(", ");
	const sources = [...copied.values()].map(quote).join(", ");
	return [
		createTable(table, staging),
		`INSERT INTO ${quote(staging)} (${targets}) SELECT ${sources} FROM ${quote(table.name)}`,
		dropTable(table.name),
		renameTable(staging, table.name),
		...table.indexes.map((index) => createIndex(table.name, index))
	];
}
/** Write one column's definition. */
function columnDefinition(column, dialect) {
	const parts = [quote(column.name), column.type];
	if (column.generated) parts.push(`GENERATED ALWAYS AS (${column.generated.expression}) ${storage(column, dialect)}`);
	else if (column.default !== void 0) parts.push(`DEFAULT ${column.default}`);
	if (!column.nullable) parts.push("NOT NULL");
	return parts.join(" ");
}
/** Write a generated column's storage. */
function storage(column, dialect) {
	if (dialect === "postgresql") return "STORED";
	else if (dialect === "sqlite") return column.generated?.mode === "virtual" ? "VIRTUAL" : "STORED";
	else return assertNever(dialect);
}
/** Write a constraint's definition. */
function constraintDefinition(constraint) {
	const name = `CONSTRAINT ${quote(constraint.name)}`;
	if (constraint.kind === "check") return `${name} CHECK (${constraint.expression})`;
	else if (constraint.kind === "foreignKey") {
		const onDelete = constraint.onDelete?.toUpperCase();
		const onUpdate = constraint.onUpdate?.toUpperCase();
		const actions = [onDelete === void 0 ? "" : ` ON DELETE ${onDelete}`, onUpdate === void 0 ? "" : ` ON UPDATE ${onUpdate}`].join("");
		const references = `${quote(constraint.table)} (${names(constraint.references)})`;
		return `${name} FOREIGN KEY (${names(constraint.columns)}) REFERENCES ${references}${actions} DEFERRABLE INITIALLY IMMEDIATE`;
	} else return `${name} ${constraint.kind === "primaryKey" ? "PRIMARY KEY" : "UNIQUE"} (${names(constraint.columns)})`;
}
/** Write a quoted column list. */
function names(columns) {
	return columns.map(quote).join(", ");
}
/** Create the triggers copying writes between bridged columns. */
function install(table, bridge, dialect) {
	const name = bridgeName(table, bridge);
	const target = quote(table);
	const from = quote(bridge.from);
	const to = quote(bridge.to);
	if (dialect === "sqlite") return [
		`CREATE TRIGGER ${quote(`${name}_insert`)} AFTER INSERT ON ${target} BEGIN
                UPDATE ${target} SET ${to} = coalesce(NEW.${to}, NEW.${from}),
                    ${from} = coalesce(NEW.${from}, NEW.${to}) WHERE rowid = NEW.rowid;
            END`,
		`CREATE TRIGGER ${quote(`${name}_from`)} AFTER UPDATE OF ${from} ON ${target}
                WHEN NEW.${from} IS NOT OLD.${from} AND NEW.${from} IS NOT NEW.${to} BEGIN
                UPDATE ${target} SET ${to} = NEW.${from} WHERE rowid = NEW.rowid;
            END`,
		`CREATE TRIGGER ${quote(`${name}_to`)} AFTER UPDATE OF ${to} ON ${target}
                WHEN NEW.${to} IS NOT OLD.${to} AND NEW.${to} IS NOT NEW.${from} BEGIN
                UPDATE ${target} SET ${from} = NEW.${to} WHERE rowid = NEW.rowid;
            END`
	];
	else if (dialect === "postgresql") return [`CREATE FUNCTION ${quote(name)}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                IF TG_OP = 'INSERT' THEN
                    NEW.${to} := coalesce(NEW.${to}, NEW.${from});
                    NEW.${from} := coalesce(NEW.${from}, NEW.${to});
                ELSIF NEW.${from} IS DISTINCT FROM OLD.${from} THEN
                    NEW.${to} := NEW.${from};
                ELSIF NEW.${to} IS DISTINCT FROM OLD.${to} THEN
                    NEW.${from} := NEW.${to};
                END IF;
                RETURN NEW;
            END $$`, `CREATE TRIGGER ${quote(name)} BEFORE INSERT OR UPDATE ON ${target}
                FOR EACH ROW EXECUTE FUNCTION ${quote(name)}()`];
	else return assertNever(dialect);
}
/** Remove the triggers of one bridge. */
function remove(table, bridge, dialect) {
	const name = bridgeName(table, bridge);
	if (dialect === "sqlite") return [
		"insert",
		"from",
		"to"
	].map((suffix) => `DROP TRIGGER IF EXISTS ${quote(`${name}_${suffix}`)}`);
	else if (dialect === "postgresql") return [`DROP FUNCTION IF EXISTS ${quote(name)}() CASCADE`];
	else return assertNever(dialect);
}
/** Name a bridge's triggers by a digest. */
function bridgeName(table, bridge) {
	return `destack_bridge_${hashName(`${table}\0${bridge.from}\0${bridge.to}`)}`;
}
/** The generated triggers every plan removes and reinstalls. */
var TRIGGERS = [
	logTriggers,
	treeTriggers,
	aggregateTriggers,
	dependentTriggers,
	{
		install: (state, dialect) => (state.bridges ?? []).flatMap((bridge) => install(state.table.name, bridge, dialect)),
		remove: (state, dialect) => (state.bridges ?? []).flatMap((bridge) => remove(state.table.name, bridge, dialect))
	}
];
/** Plan a database's migration, or name every problem to fix. */
function planTables(input) {
	const { applied, existing, declared, dialect } = input;
	const remaining = new Map(applied.map((state) => [state.table.name, state]));
	const problems = (input.conflicts ?? []).map((conflict) => ({
		target: conflict.table,
		detail: conflict.reason
	}));
	const steps = [];
	const created = [];
	const conflicting = new Set((input.conflicts ?? []).map((conflict) => conflict.table));
	for (const state of declared) {
		const name = state.table.name;
		if (conflicting.has(name)) {
			remaining.delete(name);
			continue;
		}
		const previous = remaining.get(name) ?? (state.moved?.table === void 0 ? void 0 : remaining.get(state.moved.table));
		remaining.delete(previous?.table.name ?? name);
		if (!previous) {
			if (existing.includes(name)) problems.push({
				target: name,
				detail: "table exists without applied state"
			});
			else {
				steps.push(step("createTable", "safe", name, "create table", createStatements(state.table)));
				created.push(state.table);
			}
			continue;
		}
		if (previous.table.name !== name) steps.push(step("renameTable", "backward-incompatible", name, `rename table from ${previous.table.name}`, [renameTable(previous.table.name, name)]));
		const changes = changeTable({
			...previous.table,
			name
		}, state.table, state.moved?.columns ?? {}, problems);
		const releases = Object.keys(state.conversions ?? {}).filter((release) => Version.compare(release, previous.package.version) > 0 && Version.compare(release, state.package.version) <= 0).sort(Version.compare);
		for (const release of releases) changes.push(convertRows(state, release));
		changes.push(...changeValues(previous, state, releases, problems));
		const bridged = new Set((previous.bridges ?? []).map((bridge) => bridge.to));
		for (const bridge of state.bridges ?? []) if (!bridged.has(bridge.to)) changes.push(step("bridgeColumn", "data-dependent", name, `copy ${bridge.from} into ${bridge.to}`, [`UPDATE ${quote(name)} SET ${quote(bridge.to)} = ${quote(bridge.from)}`]));
		if (state.tree && canonicalize(state.tree) !== canonicalize(previous.tree ?? null)) changes.push({
			...step("rebuildTree", "safe", name, "rebuild the ancestor index", []),
			tree: state.tree
		});
		if (changes.length === 0 && canonicalize(state.log ?? null) !== canonicalize(previous.log ?? null)) changes.push(step("updateLog", "safe", name, `log changes with retention ${state.log?.retention ?? "none"}`, []));
		steps.push(...changes);
	}
	for (const state of declared) {
		const previous = applied.find((entry) => entry.table.name === state.table.name);
		for (const aggregate of state.aggregates ?? []) if (!(previous?.aggregates ?? []).some((entry) => canonicalize(entry) === canonicalize(aggregate))) steps.push(step("recomputeAggregate", "safe", aggregate.table, `compute ${aggregate.column} from ${aggregate.source}`, [recomputeAggregate(aggregate, dialect)]));
	}
	for (const previous of applied) {
		const state = declared.find((entry) => entry.table.name === previous.table.name);
		for (const aggregate of previous.aggregates ?? []) if (!(state?.aggregates ?? []).some((entry) => canonicalize(entry) === canonicalize(aggregate))) steps.push(step("dropAggregate", "safe", aggregate.table, `stop keeping ${aggregate.column} from ${aggregate.source}`, []));
	}
	for (const state of declared) {
		const previous = applied.find((entry) => entry.table.name === state.table.name);
		for (const dependent of state.dependents ?? []) if (!(previous?.dependents ?? []).some((entry) => canonicalize(entry) === canonicalize(dependent))) steps.push(step("addDependent", "safe", dependent.table, `${dependent.onDelete} deletes into ${dependent.source}`, []));
	}
	for (const previous of applied) {
		const state = declared.find((entry) => entry.table.name === previous.table.name);
		for (const dependent of previous.dependents ?? []) if (!(state?.dependents ?? []).some((entry) => canonicalize(entry) === canonicalize(dependent))) steps.push(step("dropDependent", "safe", dependent.table, `stop ${dependent.onDelete} deletes into ${dependent.source}`, []));
	}
	if (dialect === "postgresql") for (const table of created) {
		const keys = table.constraints.filter((constraint) => constraint.kind === "foreignKey");
		if (keys.length > 0) steps.push(step("alterConstraint", "safe", table.name, "add foreign keys", keys.map((key) => addConstraint(table.name, key))));
	}
	const dropped = [...remaining.keys()];
	for (const name of dropped) steps.push(step("dropTable", "destructive", name, "drop table", [dropTable(name), deleteState(name)]));
	if (problems.length > 0) throw new PlanError(problems);
	const isChanged = steps.length > 0;
	return {
		dialect,
		steps,
		before: isChanged ? applied.flatMap((state) => TRIGGERS.flatMap((triggers) => triggers.remove(state, dialect))) : [],
		after: isChanged ? [
			...createLog(dialect),
			createState(),
			...declared.flatMap((state) => TRIGGERS.flatMap((triggers) => triggers.install(state, dialect)))
		] : [],
		state: declared,
		dropped
	};
}
/** Plan the changes of one table kept under its name. */
function changeTable(previous, next, moved, problems) {
	const steps = [];
	const columns = new Map(previous.columns.map((column) => [column.name, column]));
	for (const column of next.columns) {
		const from = moved[column.name];
		const renamed = from === void 0 ? void 0 : columns.get(from);
		if (renamed && !columns.has(column.name)) {
			steps.push(step("renameColumn", "backward-incompatible", next.name, `rename column ${from} to ${column.name}`, [renameColumn(next.name, from, column.name)]));
			columns.delete(from);
			columns.set(column.name, {
				...renamed,
				name: column.name
			});
		}
	}
	const current = {
		...previous,
		columns: [...columns.values()]
	};
	const added = next.columns.filter((column) => !columns.has(column.name));
	const removed = current.columns.filter((column) => !next.columns.some((entry) => entry.name === column.name));
	const changed = next.columns.filter((column) => {
		const old = columns.get(column.name);
		return old !== void 0 && canonicalize({
			...old,
			value: void 0
		}) !== canonicalize({
			...column,
			value: void 0
		});
	});
	const unfilled = added.filter((column) => !column.nullable && column.default === void 0 && !column.generated);
	for (const column of unfilled) problems.push({
		target: next.name,
		detail: `declare a default for the required column ${column.name}`
	});
	return [...steps, ...next.dialect === "sqlite" ? changeSQLiteTable(current, next, added, removed, changed) : changePostgresTable(current, next, added, removed, changed)];
}
/** Lower table changes to SQLite, rebuilding for anything beyond additions and indexes. */
function changeSQLiteTable(previous, next, added, removed, changed) {
	if (removed.length > 0 || changed.length > 0 || added.some((column) => column.generated?.mode === "stored") || canonicalize(previous.constraints) !== canonicalize(next.constraints)) {
		const copied = new Map(next.columns.filter((column) => !column.generated && previous.columns.some((old) => old.name === column.name && !old.generated)).map((column) => [column.name, column.name]));
		const isLossy = removed.length > 0 || changed.some((column) => previous.columns.find((old) => old.name === column.name).type !== column.type);
		const isChecked = changed.some((column) => previous.columns.find((old) => old.name === column.name).nullable && !column.nullable) || next.constraints.some((constraint) => !previous.constraints.some((old) => canonicalize(old) === canonicalize(constraint)));
		const detail = [
			...added.map((column) => `add ${column.name}`),
			...removed.map((column) => `drop ${column.name}`),
			...changed.map((column) => `change ${column.name}`)
		].join(", ");
		return [step("rebuildTable", isLossy ? "destructive" : isChecked ? "data-dependent" : "safe", next.name, `rebuild table: ${detail || "constraints"}`, rebuildTable(next, copied))];
	}
	return [...added.map((column) => step("addColumn", "safe", next.name, `add column ${column.name}`, [addColumn(next.name, column, "sqlite")])), ...changeIndexes(previous, next)];
}
/** Lower table changes to PostgreSQL alterations. */
function changePostgresTable(previous, next, added, removed, changed) {
	const steps = [...added.map((column) => step("addColumn", "safe", next.name, `add column ${column.name}`, [addColumn(next.name, column, "postgresql")])), ...removed.map((column) => step("dropColumn", "destructive", next.name, `drop column ${column.name}`, [dropColumn(next.name, column.name)]))];
	for (const column of changed) {
		const old = previous.columns.find((entry) => entry.name === column.name);
		if (old.generated || column.generated) steps.push(step("alterColumn", "safe", next.name, `recreate column ${column.name}`, [dropColumn(next.name, column.name), addColumn(next.name, column, "postgresql")]));
		else steps.push(step("alterColumn", old.type !== column.type ? "destructive" : old.nullable && !column.nullable ? "data-dependent" : "safe", next.name, `change column ${column.name}`, alterColumn(next.name, old, column)));
	}
	return [
		...steps,
		...changeConstraints(previous, next),
		...changeIndexes(previous, next)
	];
}
/** Replace differing PostgreSQL constraints. */
function changeConstraints(previous, next) {
	const before = new Map(previous.constraints.map((entry) => [entry.name, canonicalize(entry)]));
	const after = new Map(next.constraints.map((entry) => [entry.name, canonicalize(entry)]));
	return [...previous.constraints.filter((constraint) => after.get(constraint.name) !== before.get(constraint.name)).map((constraint) => step("alterConstraint", "safe", next.name, `drop constraint ${constraint.name}`, [dropConstraint(next.name, constraint.name)])), ...next.constraints.filter((constraint) => before.get(constraint.name) !== after.get(constraint.name)).map((constraint) => step("alterConstraint", "data-dependent", next.name, `add constraint ${constraint.name}`, [addConstraint(next.name, constraint)]))];
}
/** Replace removed, new or changed indexes. */
function changeIndexes(previous, next) {
	const before = new Map(previous.indexes.map((index) => [index.name, canonicalize(index)]));
	const after = new Map(next.indexes.map((index) => [index.name, canonicalize(index)]));
	return [...previous.indexes.filter((index) => after.get(index.name) !== before.get(index.name)).map((index) => step("dropIndex", "safe", next.name, `drop index ${index.name}`, [dropIndex(index.name)])), ...next.indexes.filter((index) => before.get(index.name) !== after.get(index.name)).map((index) => step("createIndex", index.unique ? "data-dependent" : "safe", next.name, `create index ${index.name}`, [createIndex(next.name, index)]))];
}
/** Create a table and its indexes. */
function createStatements(table) {
	return [createTable(table), ...table.indexes.map((index) => createIndex(table.name, index))];
}
/** Convert every row of a table from earlier releases by the conversion one release introduces. */
function convertRows(state, release) {
	const name = state.table.name;
	const set = Object.entries(state.conversions[release]).map(([column, expression]) => `${quote(column)} = ${expression}`).join(", ");
	return step("convertRows", "data-dependent", name, `convert rows to ${release}`, [`UPDATE ${quote(name)} SET ${set}`]);
}
/** Check each kept column's values: widened values are safe, narrowed ones need a conversion. */
function changeValues(previous, state, releases, problems) {
	const steps = [];
	const name = state.table.name;
	for (const column of state.table.columns) {
		const from = state.moved?.columns[column.name] ?? column.name;
		const old = previous.table.columns.find((entry) => entry.name === from || entry.name === column.name);
		if (old === void 0) continue;
		const isConverted = releases.some((release) => state.conversions[release][column.name] !== void 0);
		try {
			const planned = Plan.schema({
				target: `${name}.${column.name}`,
				before: old.value,
				after: column.value,
				release: state.package.version,
				compatibility: "backward",
				isConverted
			});
			for (const change of planned.steps.filter((entry) => entry.kind === "wider")) steps.push(step("widenColumn", change.risk, name, change.detail, []));
		} catch (error) {
			if (!(error instanceof PlanError)) throw error;
			problems.push(...error.problems);
		}
	}
	return steps;
}
/** Build a step. */
function step(kind, risk, target, detail, statements) {
	return {
		kind,
		risk,
		target,
		detail,
		statements
	};
}
/** Plan a connected database's migration to declared states. */
async function planMigration(connection, declared, conflicts = []) {
	return planTables({
		applied: await readState(connection),
		existing: await readTables(connection),
		declared,
		conflicts,
		dialect: connection.dialect
	});
}
/** Plan the union of the desired states of a connected database. */
async function planStates(connection, desired) {
	const dialect = connection.dialect;
	const merged = mergeStates(desired.map((state) => DatabaseState.parse(state).tables[dialect]));
	return await planMigration(connection, merged.declared, merged.conflicts);
}
/** The most ancestor records one statement writes. */
var BATCH_SIZE = Math.floor(PARAMETER_BUDGET / 4);
/** The temporary staging table of a rebuild. */
var STAGED = "destack_tree_rebuild";
/** Rebuild a tree index in the caller's write transaction. */
async function rebuildTree(database, tree) {
	if (!database.driver.transaction) throw new DatabaseError("TRANSACTION_REQUIRED", "tree rebuild requires a transaction");
	if (database.dialect === "postgresql") await database.execute(sql`LOCK TABLE ${sql.identifier(tree.table)} IN SHARE ROW EXCLUSIVE MODE`);
	const rows = await database.execute(sql`SELECT ${sql.identifier(tree.id)} AS id, ${sql.identifier(tree.scope)} AS scope,
            ${sql.identifier(tree.parent)} AS parent FROM ${sql.identifier(tree.table)}`);
	const scopes = /* @__PURE__ */ new Map();
	for (const row of rows) {
		let parents = scopes.get(row.scope);
		if (!parents) {
			parents = /* @__PURE__ */ new Map();
			scopes.set(row.scope, parents);
		}
		if (parents.has(row.id)) throw new DatabaseError("INVALID_MIGRATION", "tree identity already exists");
		parents.set(row.id, row.parent);
	}
	for (const parents of scopes.values()) {
		const complete = /* @__PURE__ */ new Set();
		for (const id of parents.keys()) {
			const path = /* @__PURE__ */ new Set();
			let current = id;
			while (current !== null && !complete.has(current)) {
				if (path.has(current)) throw new DatabaseError("INVALID_MIGRATION", "tree contains a cycle");
				if (!parents.has(current)) throw new DatabaseError("INVALID_MIGRATION", "tree parent is missing");
				path.add(current);
				current = parents.get(current);
			}
			for (const member of path) complete.add(member);
		}
	}
	const staged = sql.identifier(STAGED);
	await database.execute(sql`CREATE TEMPORARY TABLE ${staged}
        (scope text NOT NULL, ancestor text NOT NULL, descendant text NOT NULL, depth integer NOT NULL)`);
	const flush = async (batch) => database.execute(sql`INSERT INTO ${staged}
            (scope, ancestor, descendant, depth) VALUES ${sql.join([...batch], sql`, `)}`);
	let batch = [];
	for (const [scope, parents] of scopes) for (const descendant of parents.keys()) {
		let ancestor = descendant;
		let depth = 0;
		while (ancestor !== null) {
			batch.push(sql`(${scope}, ${ancestor}, ${descendant}, ${depth})`);
			ancestor = parents.get(ancestor);
			depth++;
			if (batch.length === BATCH_SIZE) {
				await flush(batch);
				batch = [];
			}
		}
	}
	if (batch.length > 0) await flush(batch);
	const ancestors = sql.identifier(tree.ancestors);
	const same = (left, right) => sql`${left}.scope = ${right}.scope
        AND ${left}.ancestor = ${right}.ancestor
        AND ${left}.descendant = ${right}.descendant
        AND ${left}.depth = ${right}.depth`;
	await database.execute(sql`DELETE FROM ${ancestors}
        WHERE NOT EXISTS (SELECT 1 FROM ${staged} WHERE ${same(staged, ancestors)})`);
	await database.execute(sql`INSERT INTO ${ancestors} (scope, ancestor, descendant, depth)
        SELECT scope, ancestor, descendant, depth FROM ${staged}
        WHERE NOT EXISTS (SELECT 1 FROM ${ancestors} WHERE ${same(ancestors, staged)})`);
	await database.execute(sql`DROP TABLE ${staged}`);
}
/** The step kinds applied after the triggers for the log to record their writes. */
var LOGGED_KINDS = /* @__PURE__ */ new Set([
	"convertRows",
	"bridgeColumn",
	"rebuildTree"
]);
/** Apply a plan in one transaction and record the declared state. */
async function applyPlan(database, plan) {
	if (plan.steps.length === 0) return;
	const changes = [
		plan.dialect === "postgresql" ? `SELECT pg_advisory_xact_lock(hashtextextended('${STATE}', 0))` : "PRAGMA defer_foreign_keys = ON",
		...plan.before,
		...plan.steps.filter((step) => !LOGGED_KINDS.has(step.kind)).flatMap((step) => step.statements),
		...plan.after
	];
	const isRebuilt = plan.dialect === "sqlite" && plan.steps.some((step) => step.kind === "rebuildTable");
	if (isRebuilt) await database.executeScript("PRAGMA foreign_keys = OFF");
	const appliedAt = Date.now();
	try {
		await applySteps(database, plan, changes, appliedAt, isRebuilt);
	} finally {
		if (isRebuilt) await database.executeScript("PRAGMA foreign_keys = ON");
	}
}
/** Apply a plan's phases and record its state. */
async function applySteps(database, plan, changes, appliedAt, isRebuilt) {
	await database.transaction(async (transaction) => {
		await runScript(transaction, changes);
		let pending = [];
		for (const step of plan.steps.filter((entry) => LOGGED_KINDS.has(entry.kind))) if (step.tree) {
			await runScript(transaction, pending);
			pending = [];
			await rebuildTree(transaction, step.tree).catch((cause) => {
				throw new DatabaseError("MIGRATION_FAILED", `${step.target}: ${step.detail} failed`, { cause });
			});
		} else pending.push(...step.statements);
		await runScript(transaction, [...pending, ...plan.state.map((state) => writeState(state, appliedAt))]);
		if (isRebuilt) {
			const violations = await transaction.execute(sql`PRAGMA foreign_key_check`);
			if (violations.length > 0) {
				const tables = [...new Set(violations.map((row) => row.table))].join(", ");
				throw new DatabaseError("MIGRATION_FAILED", `migration leaves foreign key violations in ${tables}`);
			}
		}
	});
}
/** Run statements as one script. */
async function runScript(database, statements) {
	if (statements.length === 0) return;
	try {
		await database.executeScript(statements.map((statement) => `${statement};`).join("\n"));
	} catch (cause) {
		const message = cause instanceof Error ? cause.message : String(cause);
		throw new DatabaseError("MIGRATION_FAILED", `migration failed: ${message}`, { cause });
	}
}
/** The lifetime of a transaction callback and its queries. */
var TransactionState = class {
	/** Whether the callback can still submit queries. */
	#isActive = true;
	/** The queries submitted before the callback finished. */
	#pending = /* @__PURE__ */ new Set();
	/** The failed queries. */
	#failures = [];
	/** The caller's cancellation signal. */
	signal;
	/** Create the state. */
	constructor(signal) {
		this.signal = signal;
	}
	/** Run a callback and settle its queries. */
	async execute(operation) {
		this.assertActive();
		let result;
		try {
			result = await operation();
		} catch (error) {
			this.close();
			await Promise.all(this.#pending);
			const unreported = this.#failures.filter((failure) => !wraps(error, failure));
			if (unreported.length > 0) throw new AggregateError([error, ...unreported], "transaction callback and queries failed");
			throw error;
		}
		await this.finish();
		return result;
	}
	/** Reject queries after the callback finished. */
	assertActive() {
		this.signal?.throwIfAborted();
		if (!this.#isActive) throw new DatabaseError("TRANSACTION_CLOSED", "the transaction has finished");
	}
	/** Track submitted work until it settles. */
	run(operation, failure = "rollback") {
		this.assertActive();
		let result;
		try {
			result = Promise.resolve(operation());
		} catch (error) {
			result = Promise.reject(error);
		}
		const settled = result.then(() => {
			this.#pending.delete(settled);
		}, (error) => {
			this.#pending.delete(settled);
			if (failure === "rollback") this.#failures.push(error);
		});
		this.#pending.add(settled);
		return result;
	}
	/** Drain submitted queries. */
	async finish() {
		this.close();
		await Promise.all(this.#pending);
		if (this.#failures.length === 1) throw this.#failures[0];
		if (this.#failures.length > 1) throw new AggregateError(this.#failures, "transaction queries failed");
		this.signal?.throwIfAborted();
	}
	/** End submissions. */
	close() {
		this.#isActive = false;
	}
};
/** Queries over one database connection or transaction. */
var DatabaseConnection = class DatabaseConnection {
	/** The native Drizzle database. */
	driver;
	/** The table compiler. */
	compiler;
	/** The physical connection state. */
	state;
	/** Bind a driver and compiler. */
	constructor(driver, compiler) {
		this.state = driver.state;
		this.driver = driver;
		this.compiler = compiler;
	}
	/** The database's SQL dialect. */
	get dialect() {
		return this.driver.native.dialect;
	}
	/** The log of the database. */
	get log() {
		return new Log(this);
	}
	/** Select application records or explicit fields. */
	select(fields) {
		return new SelectBuilder(this.driver, this.compiler, fields);
	}
	/** Select distinct application records or explicit fields. */
	selectDistinct(fields) {
		return new SelectBuilder(this.driver, this.compiler, fields, { isDistinct: true });
	}
	/** Declare a named common table expression. */
	$with(alias) {
		return { as: (query) => {
			return this.driver.native.database.$with(alias).as(query);
		} };
	}
	/** Include common table expressions in the next selection. */
	with(...queries) {
		return {
			select: (fields) => new SelectBuilder(this.driver, this.compiler, fields, { withList: queries }),
			selectDistinct: (fields) => new SelectBuilder(this.driver, this.compiler, fields, {
				isDistinct: true,
				withList: queries
			})
		};
	}
	/** Insert application records. */
	insert(table) {
		return new MutationQuery(this.driver, this.compiler, table, "insert");
	}
	/** Update application records. */
	update(table) {
		return new MutationQuery(this.driver, this.compiler, table, "update");
	}
	/** Delete application records. */
	delete(table) {
		return new MutationQuery(this.driver, this.compiler, table, "delete");
	}
	/** Upsert rows, a batch per statement. */
	async upsert(table, rows) {
		const key = table[TABLE].key;
		const columns = table[TABLE].columns;
		const written = [...new Set(rows.flatMap((row) => Object.keys(row)))];
		const set = Object.fromEntries(written.filter((name) => !key.includes(name)).map((name) => [name, sql`excluded.${sql.identifier(columns[name].definition.name)}`]));
		const size = Math.max(1, Math.floor(PARAMETER_BUDGET / Math.max(written.length, 1)));
		for (let start = 0; start < rows.length; start += size) {
			const batch = rows.slice(start, start + size).map((row) => Object.fromEntries(written.map((name) => [name, row[name] ?? null])));
			const insert = this.insert(table).values(batch);
			await (Object.keys(set).length === 0 ? insert.onConflictDoNothing() : insert.onConflictDoUpdate({
				target: key.map((name) => columns[name]),
				set
			}));
		}
	}
	/** Delete rows by key, a chain of keys per statement. */
	async remove(table, rows) {
		for (let start = 0; start < rows.length; start += 90) {
			const matches = rows.slice(start, start + 90).map((row) => Key.match(table, row));
			await this.delete(table).where(or(...matches));
		}
	}
	/** Execute a SQL script in one round trip. */
	async executeScript(script) {
		await this.driver.write(async (native) => {
			const session = native.database._.session;
			if (native.dialect === "sqlite" && session instanceof Session) await session.exec(script);
			else if (native.dialect === "postgresql") await session.client.unsafe(script).simple();
			else throw new TypeError(`${native.dialect} session cannot run scripts`);
		}, { isTransaction: true });
	}
	/** Execute SQL and return its rows. */
	execute(statement) {
		return this.driver.all(this.driver.render(this.compiler.expression(statement)));
	}
	/** Plan and apply tables at once. */
	async migrate(tables, options = {}) {
		const plan = await planMigration(this, declareState(tables, this.dialect, options));
		await this.apply(plan);
		return plan;
	}
	/** List the tables with an unapplied declaration. */
	async unapplied(tables, options = {}) {
		return unappliedTables(await readState(this), declareState(tables, this.dialect, options));
	}
	/** Plan the union of the desired states. */
	plan(desired) {
		return planStates(this, desired);
	}
	/** Apply a plan in one transaction and record the declared state. */
	apply(plan) {
		return applyPlan(this, plan);
	}
	/** Commit a callback, or roll back on failure or once the callback rolls back, then resolving undefined. */
	async transaction(operation, options = {}) {
		this.driver.transaction?.assertActive();
		const signals = [this.driver.transaction?.signal, options.signal].filter((signal) => signal !== void 0);
		const signal = signals.length ? AbortSignal.any(signals) : void 0;
		signal?.throwIfAborted();
		const execute = async () => {
			if (this.driver.native.dialect === "sqlite") {
				const root = this.driver.native.database;
				const isNested = this.driver.transaction !== void 0;
				return await root.transaction(async (transaction) => {
					if (options.constraints === "deferred") await transaction.run(sql`PRAGMA defer_foreign_keys = ON`);
					const isMarked = !isNested && !options.isReadOnly && await openTransaction(transaction, this.state);
					const result = await this.#transact({
						dialect: "sqlite",
						database: transaction
					}, operation, signal);
					if (isMarked) await closeTransaction(transaction);
					return result;
				}, { behavior: options.isReadOnly ? "deferred" : "immediate" });
			} else if (this.driver.native.dialect === "postgresql") return await this.driver.native.database.transaction(async (transaction) => {
				if (options.constraints === "deferred") await transaction.execute(sql`SET CONSTRAINTS ALL DEFERRED`);
				return this.#transact({
					dialect: "postgresql",
					database: transaction
				}, operation, signal);
			}, {
				isolationLevel: options.isolationLevel ?? "repeatable read",
				accessMode: options.isReadOnly ? "read only" : "read write"
			});
			else return assertNever(this.driver.native);
		};
		try {
			return this.driver.transaction ? await this.driver.transaction.run(execute, "report") : await this.driver.write(execute, { isTransaction: true });
		} catch (error) {
			if (error instanceof Rollback) return;
			throw classifyError(error);
		}
	}
	/** Roll back the transaction this connection runs, ending its callback. */
	rollback() {
		if (this.driver.transaction === void 0) throw new TypeError("only a transaction rolls back");
		throw new Rollback();
	}
	/** Bind a transaction session and drain its queries. */
	#transact(connection, operation, signal) {
		const state = new TransactionState(signal);
		const transaction = new DatabaseConnection(new DatabaseDriver(connection, this.state, state), this.compiler);
		return state.execute(() => operation(transaction));
	}
};
/** The operations, shutdown and commit watch of one physical connection. */
var ConnectionState = class {
	/** Where the connection's database runs. */
	locality;
	/** The commits this connection's readers wait for. */
	commits;
	/** Whether the database holds a log. */
	isLogged = false;
	/** The submitted statements and transactions. */
	operations = 0;
	/** The run statements and transactions, including nested ones. */
	statements = 0;
	/** The operations to finish before close. */
	#pending = /* @__PURE__ */ new Set();
	/** The shutdown. */
	#closing;
	/** Create the state of a new connection. */
	constructor(locality, notifier) {
		this.locality = locality;
		this.commits = new CommitWatch(notifier);
	}
	/** Submit an operation. */
	run(operation) {
		if (this.#closing) throw new DatabaseError("CONNECTION_CLOSED", "the connection is closing or closed");
		this.operations += 1;
		const result = Promise.resolve().then(operation);
		const settled = result.then(() => {
			this.#pending.delete(settled);
		}, () => {
			this.#pending.delete(settled);
		});
		this.#pending.add(settled);
		return result;
	}
	/** Stop submissions, drain work, and close the client once. */
	close(operation) {
		this.#closing ??= Promise.all(this.#pending).then(() => this.commits.stop()).then(operation);
		return this.#closing;
	}
};
/** The unwinding of a transaction its callback rolls back. */
var Rollback = class extends Error {};
function alias(table, alias) {
	return new Proxy(table, new TableAliasProxyHandler(alias, false));
}
/** Compile declarations into native Drizzle tables and columns. */
var SchemaCompiler = class {
	/** The selected SQL dialect. */
	dialect;
	/** The physical tables by SQL name. */
	tables = /* @__PURE__ */ new Map();
	/** The physical columns by declaration. */
	columns = /* @__PURE__ */ new WeakMap();
	/** The physical tables by declaration. */
	declarations = /* @__PURE__ */ new WeakMap();
	/** Compile each declared table once. */
	constructor(dialect, declarations) {
		this.dialect = dialect;
		for (const declaration of declarations) {
			const definition = declaration[TABLE];
			if (definition.source) throw new TypeError(`query aliases cannot declare SQL tables: ${definition.name}`);
			if (this.tables.has(definition.sqlName)) throw new TypeError(`duplicate SQL table: ${definition.sqlName}`);
			const physical = this.compileTable(declaration);
			this.tables.set(definition.sqlName, physical);
			this.declarations.set(declaration, physical);
			const columns = getTableColumns(physical);
			for (const [property, column] of Object.entries(definition.columns)) this.columns.set(column, columns[property]);
		}
		const relations = new Set(this.tables.keys());
		for (const declaration of declarations) for (const constraint of declaration.constraints(dialect)) {
			if (constraint.name === void 0 || ![
				"index",
				"unique",
				"primaryKey"
			].includes(constraint.kind)) continue;
			const name = constraintName(declaration[TABLE].package, constraint.name);
			if (relations.has(name)) throw new TypeError(`duplicate SQL relation name: ${name}`);
			relations.add(name);
		}
	}
	/** Translate an expression to physical columns. */
	expression(expression) {
		compileExpression(expression, this.dialect, (chunk) => {
			if (chunk instanceof Table$1) this.table(chunk);
			return chunk;
		});
		return compileExpression(expression, this.dialect, (chunk) => this.#chunk(chunk));
	}
	/** Find the physical column of a declaration. */
	column(column) {
		const physical = this.columns.get(column);
		if (!physical) throw new TypeError(`undeclared SQL column: ${column.table}.${column.definition.name}`);
		return physical;
	}
	/** Find the physical table of a declaration. */
	table(declaration) {
		let physical = this.declarations.get(declaration);
		const source = declaration[TABLE].source;
		if (!physical && source) {
			const original = this.table(source);
			if (this.dialect === "sqlite") physical = alias$1(original, declaration[TABLE].name);
			else if (this.dialect === "postgresql") physical = alias(original, declaration[TABLE].name);
			else assertNever(this.dialect);
			this.declarations.set(declaration, physical);
			const columns = getTableColumns(physical);
			for (const [property, column] of Object.entries(declaration[TABLE].columns)) this.columns.set(column, columns[property]);
		}
		if (!physical) throw new TypeError(`undeclared SQL table: ${declaration[TABLE].name}`);
		return physical;
	}
	/** Translate nested declaration expressions. */
	#chunk(chunk) {
		if (chunk instanceof Column$1) return this.column(chunk);
		if (chunk instanceof Param && chunk.encoder instanceof Column$1) return new Param(chunk.value, this.column(chunk.encoder));
		if (chunk instanceof SQL) {
			const decoder = chunk.decoder;
			if (decoder instanceof Column$1) chunk.mapWith(this.column(decoder));
		}
		return chunk;
	}
};
/** Apply column constraints through Drizzle's builder. */
function applyColumn(builder, column, dialect) {
	const definition = column.definition;
	if (!definition.nullable) builder.notNull();
	if (definition.primaryKey) builder.primaryKey();
	if (definition.unique) builder.unique(definition.unique.name);
	if (definition.default !== void 0) {
		const value = definition.default instanceof SQL ? compileExpression(definition.default, dialect) : definition.default;
		builder.default(value);
	}
	const runtimeDefault = definition.runtimeDefault;
	if (runtimeDefault) builder.$defaultFn(() => {
		const value = runtimeDefault();
		return value instanceof SQL ? compileExpression(value, dialect) : value;
	});
	const runtimeUpdate = definition.runtimeUpdate;
	if (runtimeUpdate) builder.$onUpdateFn(() => {
		const value = runtimeUpdate();
		return value instanceof SQL ? compileExpression(value, dialect) : value;
	});
}
/** Compile declarations into SQLite Drizzle tables. */
var SqliteSchemaCompiler = class extends SchemaCompiler {
	/** Compile the declared tables. */
	constructor(declarations) {
		super("sqlite", declarations);
	}
	/** Compile one SQLite table. */
	compileTable(declaration) {
		const definition = declaration[TABLE];
		const columns = Object.fromEntries(Object.entries(definition.columns).map(([property, column]) => {
			const definition = column.definition;
			const builder = customType({
				dataType: () => definition.types.sqlite,
				toDriver: (value) => definition.encode(value, "sqlite"),
				fromDriver: (value) => definition.decode(value, "sqlite"),
				fromJson: (value) => definition.decode(definition.kind === "binary" && typeof value === "string" ? Uint8Array.fromHex(value) : value, "sqlite"),
				forJsonSelect: definition.kind === "bigint" ? (identifier, sql) => sql`cast(${identifier} as text)` : void 0
			})(definition.name);
			applyColumn(builder, column, this.dialect);
			if (definition.generated) {
				const generated = definition.generated.expression;
				builder.generatedAlwaysAs(() => this.expression(typeof generated === "function" ? generated() : generated), { mode: definition.generated.mode });
			}
			return [property, builder];
		}));
		return sqliteTable(definition.sqlName, columns, () => declaration.constraints(this.dialect).map((constraint) => this.#compileConstraint(constraint, definition.package)));
	}
	/** Translate one SQLite constraint. */
	#compileConstraint(constraint, owner) {
		const columns = (values) => values.map((column) => this.column(column));
		switch (constraint.kind) {
			case "check": return check(constraint.name, this.expression(constraint.expression));
			case "primaryKey": return primaryKey({
				name: constraintName(owner, constraint.name),
				columns: columns(constraint.columns)
			});
			case "unique": return unique(constraintName(owner, constraint.name)).on(...columns(constraint.columns));
			case "index": {
				const builder = constraint.isUnique ? uniqueIndex(constraintName(owner, constraint.name)) : index(constraintName(owner, constraint.name));
				const indexed = constraint.columns.map((column) => column instanceof Column$1 ? this.column(column) : this.expression(column));
				const index$1 = builder.on(...indexed);
				return constraint.predicate ? index$1.where(this.expression(constraint.predicate)) : index$1;
			}
			case "foreignKey": {
				const key = foreignKey({
					name: constraint.name,
					columns: columns(constraint.columns),
					foreignColumns: columns(constraint.foreignColumns)
				});
				if (constraint.actions.onDelete) key.onDelete(constraint.actions.onDelete);
				if (constraint.actions.onUpdate) key.onUpdate(constraint.actions.onUpdate);
				return key;
			}
		}
	}
};
/** A SQLite database with its own connection. */
var SqliteDatabase = class extends DatabaseConnection {
	/** The SQLite connection client. */
	$client;
	/** The native SQL API. */
	native;
	/** Bind tables to a SQLite connection. */
	constructor(client, tables, locality, notifier, options = {}) {
		const compiler = new SqliteSchemaCompiler(tables instanceof Database$1 ? tables.tables : expandTrees(tables));
		const native = new SqliteNative(client, options);
		super(new DatabaseDriver({
			dialect: "sqlite",
			database: native
		}, new ConnectionState(locality, notifier)), compiler);
		this.$client = client;
		this.native = native;
	}
	/** Execute an SQL statement. */
	async run(statement) {
		return await this.driver.write((native) => native.database.run(this.compiler.expression(statement)));
	}
	/** Close the physical database. */
	close() {
		return this.state.close(() => this.$client.close());
	}
};
/** Work run one piece at a time, in arrival order. */
var WorkQueue = class {
	/** The tail of the waiting work. */
	#tail = Promise.resolve();
	/** Run work after the earlier work. */
	run(work) {
		const result = this.#tail.then(work);
		this.#tail = result.catch(() => void 0);
		return result;
	}
};
/**
* The transitions of SQLite's statement completion state machine, as `sqlite3_complete` defines them.
*
* States: invalid, start, normal, explain, create, trigger, semicolon within a trigger, end of a trigger.
* Tokens: semicolon, whitespace, other, EXPLAIN, CREATE, TEMP, TRIGGER, END.
*/
var TRANSITIONS = [
	[
		1,
		0,
		2,
		3,
		4,
		2,
		2,
		2
	],
	[
		1,
		1,
		2,
		3,
		4,
		2,
		2,
		2
	],
	[
		1,
		2,
		2,
		2,
		2,
		2,
		2,
		2
	],
	[
		1,
		3,
		3,
		2,
		4,
		2,
		2,
		2
	],
	[
		1,
		4,
		2,
		2,
		2,
		4,
		5,
		2
	],
	[
		6,
		5,
		5,
		5,
		5,
		5,
		5,
		5
	],
	[
		6,
		6,
		5,
		5,
		5,
		5,
		5,
		7
	],
	[
		1,
		7,
		5,
		5,
		5,
		5,
		5,
		5
	]
];
/** The state after a complete statement's closing semicolon. */
var START = 1;
/** The token columns of the transitions. */
var TOKEN = {
	semicolon: 0,
	whitespace: 1,
	other: 2,
	explain: 3,
	create: 4,
	temp: 5,
	trigger: 6,
	end: 7
};
/** The keywords the state machine tells apart. */
var KEYWORDS = {
	explain: TOKEN.explain,
	create: TOKEN.create,
	temp: TOKEN.temp,
	temporary: TOKEN.temp,
	trigger: TOKEN.trigger,
	end: TOKEN.end
};
/** Scripts of several SQLite statements. */
var SqliteScript = { 
/** Split a script into complete SQLite statements with whole trigger bodies. */
statements(script) {
	const statements = [];
	let state = 0;
	let start = 0;
	for (let index = 0; index < script.length;) {
		const { token, end } = next(script, index);
		state = TRANSITIONS[state][token];
		if (token === TOKEN.semicolon && state === START) {
			statements.push(script.slice(start, end).trim());
			start = end;
		}
		index = end;
	}
	const rest = script.slice(start).trim();
	if (rest.length > 0 && state !== 0 && state !== START) statements.push(rest);
	return statements;
} };
/** Read the token at an index and the index after it. */
function next(script, index) {
	const character = script[index];
	if (character === ";") return {
		token: TOKEN.semicolon,
		end: index + 1
	};
	else if (/\s/.test(character)) return {
		token: TOKEN.whitespace,
		end: index + 1
	};
	else if (character === "-" && script[index + 1] === "-") {
		const end = script.indexOf("\n", index);
		return {
			token: TOKEN.whitespace,
			end: end === -1 ? script.length : end + 1
		};
	} else if (character === "/" && script[index + 1] === "*") {
		const end = script.indexOf("*/", index + 2);
		return {
			token: TOKEN.whitespace,
			end: end === -1 ? script.length : end + 2
		};
	} else if (character === "'" || character === "\"" || character === "`" || character === "[") {
		const close = character === "[" ? "]" : character;
		const end = script.indexOf(close, index + 1);
		return {
			token: TOKEN.other,
			end: end === -1 ? script.length : end + 1
		};
	} else if (/[\w$]/.test(character)) {
		let end = index + 1;
		while (end < script.length && /[\w$]/.test(script[end])) end += 1;
		return {
			token: KEYWORDS[script.slice(index, end).toLowerCase()] ?? TOKEN.other,
			end
		};
	}
	return {
		token: TOKEN.other,
		end: index + 1
	};
}
/** The most prepared statement texts per connection: a service runs 400 to 600, at 2 to 10 KB each. */
var PREPARED_TEXTS = 512;
/** The work queue of each database file, shared by the process's connections to it. */
var FILE_QUEUES = /* @__PURE__ */ new Map();
/** Statements on one SQLite database, prepared once per text. */
var BunQuery = class {
	/** The database. */
	database;
	/** The prepared statements. */
	statements;
	/** The work queue, absent within a transaction. */
	queue;
	/** Create the client. */
	constructor(database, statements, queue) {
		this.database = database;
		this.statements = statements;
		this.queue = queue;
	}
	/** Prepare a statement on the cached native statement. */
	async prepare(sql) {
		let isRaw = false;
		let isSafe = false;
		const run = (execute) => this.#schedule(async () => execute(this.statements.take(sql).safeIntegers(isSafe)));
		const statement = {
			safeIntegers: (enabled) => {
				isSafe = enabled;
				return statement;
			},
			raw: (enabled) => {
				isRaw = enabled;
				return statement;
			},
			run: (...parameters) => run((native) => native.run(...parameters)),
			all: (...parameters) => run((native) => isRaw ? native.values(...parameters) : native.all(...parameters)),
			get: (...parameters) => run((native) => isRaw ? native.values(...parameters)[0] : native.get(...parameters))
		};
		return statement;
	}
	/** Run a statement. */
	async run(sql, ...parameters) {
		return (await this.prepare(sql)).run(...parameters);
	}
	/** Read rows as named objects. */
	async all(sql, ...parameters) {
		return (await this.prepare(sql)).all(...parameters);
	}
	/** Read the first row as a named object. */
	async get(sql, ...parameters) {
		return (await this.prepare(sql)).get(...parameters);
	}
	/** Run a script statement by statement, since Bun reports only the last failure of a whole script. */
	exec(script) {
		return this.#schedule(async () => {
			for (const statement of SqliteScript.statements(script)) this.database.run(statement);
		});
	}
	/** Run work behind the queue, or at once within a transaction. */
	#schedule(work) {
		return this.queue === void 0 ? work() : this.queue.run(work);
	}
};
/** A connection client over one SQLite database, one transaction at a time per file and process. */
var BunClient = class BunClient extends BunQuery {
	/** The work queue of the database file. */
	#queue;
	/** Create the client. */
	constructor(database) {
		const queue = BunClient.queue(database);
		super(database, new StatementCache(database), queue);
		this.#queue = queue;
	}
	/** Read the work queue of a database's file, or a queue of its own for a memory database. */
	static queue(database) {
		const file = database.filename;
		if (file === "" || file === ":memory:") return new WorkQueue();
		let queue = FILE_QUEUES.get(file);
		if (queue === void 0) {
			queue = new WorkQueue();
			FILE_QUEUES.set(file, queue);
		}
		return queue;
	}
	/** Close the database after its work. */
	async close() {
		await this.#queue.run(async () => {
			this.statements.close();
			this.database.close();
		});
	}
	/** Run a callback in a transaction. */
	transactionAsync(operation) {
		const begin = (mode) => this.#queue.run(async () => {
			this.database.run(`BEGIN ${mode}`);
			try {
				const value = await operation(new BunQuery(this.database, this.statements));
				this.database.run("COMMIT");
				return value;
			} catch (error) {
				this.database.run("ROLLBACK");
				throw error;
			}
		});
		return {
			deferred: () => begin("DEFERRED"),
			immediate: () => begin("IMMEDIATE"),
			exclusive: () => begin("EXCLUSIVE")
		};
	}
};
/**
* The prepared statements of one connection by text, least recently used first.
*
* Statements run synchronously, and one statement serves each text.
*/
var StatementCache = class {
	/** The database preparing the statements. */
	#database;
	/** The statements by text, least recently used first. */
	#statements = /* @__PURE__ */ new Map();
	/** Create the cache. */
	constructor(database) {
		this.#database = database;
	}
	/** Take a text's statement, preparing it the first time. */
	take(sql) {
		const statement = this.#statements.get(sql) ?? this.#database.prepare(sql);
		this.#statements.delete(sql);
		this.#statements.set(sql, statement);
		if (this.#statements.size > PREPARED_TEXTS) {
			const [oldest, evicted] = this.#statements.entries().next().value;
			this.#statements.delete(oldest);
			evicted.finalize();
		}
		return statement;
	}
	/** Finalize every statement. */
	close() {
		for (const statement of this.#statements.values()) statement.finalize();
		this.#statements.clear();
	}
};
/**
* The wait for another process's write lock, in milliseconds.
*
* A write commits within milliseconds, and only a stuck writer exceeds five seconds.
*/
var BUSY_TIMEOUT_MILLISECONDS = 5e3;
/** Open a SQLite database file or `:memory:`. */
async function connect(connection, tables = [], options = {}) {
	const database = typeof connection === "string" ? new Database(connection) : connection;
	try {
		if (typeof connection === "string") for (const pragma of [
			"journal_mode = WAL",
			"synchronous = FULL",
			"foreign_keys = ON",
			`busy_timeout = ${BUSY_TIMEOUT_MILLISECONDS}`
		]) database.run(`PRAGMA ${pragma}`);
		const { notifier = soleWriter, ...drizzle } = options;
		return new SqliteDatabase(new BunClient(database), tables, "embedded", notifier, drizzle);
	} catch (error) {
		if (typeof connection === "string") database.close();
		throw error;
	}
}
/** Provide databases as SQLite files, one folder per space. */
function sqliteProvider(root) {
	if (root.protocol !== "file:" || !root.pathname.endsWith("/")) throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
	return {
		kind: "database",
		code: "sqlite",
		provision: async (record) => {
			const space = new URL(`${record.scope}/`, root);
			const file = new URL(`${record.id}.db`, space);
			await mkdir(space, { recursive: true });
			await (await connect(fileURLToPath(file))).close();
			return { reference: file.href };
		},
		plan: async (record, desired) => {
			const connection = await open(record, []);
			try {
				return await connection.plan(desired);
			} finally {
				await connection.close();
			}
		},
		apply: async (record, desired, digest) => {
			const connection = await open(record, []);
			try {
				const plan = await connection.plan(desired);
				if (await Plan.digest(plan) !== digest) throw new DatabaseError("PLAN_CHANGED", `plan of ${record.id} changed since review`);
				await connection.apply(plan);
			} finally {
				await connection.close();
			}
		},
		connect: async (record, declaration) => {
			const database = requireDatabase(declaration);
			const connection = await open(record, database.tables);
			const unapplied = await database.check(connection);
			if (unapplied.length > 0) {
				await connection.close();
				throw new DatabaseError("NOT_APPLIED", `database ${database.name} has not applied ${unapplied.join(", ")}`);
			}
			return connection;
		},
		export: async function* (copy, after, signal) {
			const replication = Replication.of(copy.desired, "sqlite");
			const connection = await open(copy.record, replication.tables);
			try {
				yield* replication.export(connection, `copy:${copy.record.id}`, copy.stage, after, signal);
			} finally {
				await connection.close();
			}
		},
		import: async (copy, chunk) => {
			const replication = Replication.of(copy.desired, "sqlite");
			const connection = await open(copy.record, replication.tables);
			try {
				await replication.import(connection, chunk);
			} finally {
				await connection.close();
			}
		},
		destroy: async (record) => {
			const path = fileURLToPath(requireReference(record.reference));
			for (const suffix of [
				"",
				"-wal",
				"-shm"
			]) await rm(`${path}${suffix}`, { force: true });
		}
	};
}
/** Require a provisioned resource reference. */
function requireReference(reference) {
	if (reference === null) throw new TypeError("resource is not provisioned");
	return reference;
}
/** Require a database declaration. */
function requireDatabase(declaration) {
	if (!(declaration instanceof Database$1)) throw new TypeError("not a database declaration");
	return declaration;
}
/** Open a provisioned database file. */
function open(record, tables) {
	return connect(fileURLToPath(requireReference(record.reference)), tables);
}
export { sqliteProvider };

//# sourceMappingURL=provider-DH4goO8u.js.map