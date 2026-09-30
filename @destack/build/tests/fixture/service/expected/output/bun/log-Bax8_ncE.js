import { Dn as sql, Et as dialectSQL, G as Key, M as latestOf, N as selectHead, P as Condition, Sr as string, Un as digest, Ut as DatabaseError, W as Order, Wt as assertNever, _t as TABLE, ar as defineSchema, at as LOG_COPYING, br as record, cr as _enum, ct as LOG_HORIZON, ft as v7, gr as json, lr as array, lt as LOG_TRANSACTION, or as toJsonSchema, ot as LOG_EPOCH, qt as and, rt as LOG, st as LOG_HOLD, wn as fillPlaceholders, wt as Expression, xr as strictObject } from "./trace-api-DJ-_ZhlJ.js";
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
var KEYWORDS = /* @__PURE__ */ new Set([
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
	return KEYWORDS.has(keyword) || ANNOTATIONS.has(keyword);
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
/** The path of a part of a declaration or resource, such as `database/main/table/note/column/title`. */
var Address = Object.assign(defineSchema(string().regex(/^[^/\s]+(?:\/[^/\s]+)*$(?![\s\S])/)), { 
/** Join address parts, such as a declaration's kind and name. */
join(...parts) {
	return parts.join("/");
} });
/** A resource binding failure. */
var ResourceError = class extends Error {
	/** The binding operation that failed. */
	code;
	/** Describe a missing or duplicate binding. */
	constructor(code, message, options) {
		super(message, options);
		this.name = "ResourceError";
		this.code = code;
	}
};
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
/** What a step does to its target, after Terraform's plan actions. */
var Action = defineSchema(_enum([
	"create",
	"update",
	"delete",
	"replace",
	"rename",
	"convert",
	"restore"
]));
/** One change a plan makes, as reviewers see it. */
var Step = defineSchema(strictObject({
	/** What the step does. */
	action: Action,
	/** The changed part. */
	target: Address,
	/** How consequential the step is. */
	risk: Risk,
	/** A readable summary, such as "add column priority". */
	detail: string().min(1),
	/** Each changed field's current and planned value. */
	fields: record(string(), strictObject({
		before: json(),
		after: json()
	})).optional()
}));
/** The changes one review covers. */
var Plan = Object.assign(defineSchema(strictObject({
	/** The steps in application order. */
	steps: array(Step),
	/** Why the plan stops before later steps. */
	deferred: string().min(1).optional()
})), {
	classify,
	reaches,
	join,
	digest: digestSteps,
	values: planValues
});
/** Classify a plan by its most consequential step, safe when empty. */
function classify(plan) {
	return plan.steps.reduce((highest, next) => Risk.options.indexOf(next.risk) > Risk.options.indexOf(highest) ? next.risk : highest, "safe");
}
/** Decide whether a plan reaches a risk. */
function reaches(plan, risk) {
	return plan.steps.length > 0 && Risk.options.indexOf(classify(plan)) >= Risk.options.indexOf(risk);
}
/** Join plans in order, collecting every refusal into one PlanError. */
function join(parts) {
	const steps = [];
	const problems = [];
	for (const part of parts) try {
		steps.push(...part().steps);
	} catch (error) {
		if (!(error instanceof PlanError)) throw error;
		problems.push(...error.problems);
	}
	if (problems.length > 0) throw new PlanError(problems);
	return { steps };
}
/** Digest a plan's reviewed steps, so an apply can require the plan a review saw. */
function digestSteps(plan) {
	return digest(plan.steps.map((step) => ({
		action: step.action,
		risk: step.risk,
		target: step.target,
		detail: step.detail,
		...step.fields === void 0 ? {} : { fields: step.fields }
	})));
}
/** Plan a value schema change: widening for backward readers, narrowing for forward ones, converting otherwise. */
function planValues(change) {
	const { target, release, compatibility } = change;
	const compared = compareJsonSchemas(change.before, change.after);
	const isCompatible = compared === "same" || compared === (compatibility === "backward" ? "wider" : "narrower");
	if (compared === "same") return { steps: [] };
	if (isCompatible) return { steps: [{
		action: "update",
		target,
		risk: "safe",
		detail: `${compared} values`
	}] };
	else if (compatibility === "backward" && change.isConverted) return { steps: [{
		action: "convert",
		target,
		risk: "data-dependent",
		detail: `convert values to ${release}`
	}] };
	else if (compatibility === "backward") throw new PlanError([{
		target,
		detail: `declare a conversion for ${release}`
	}]);
	else return { steps: [{
		action: "update",
		target,
		risk: "backward-incompatible",
		detail: `${compared} values: earlier readers keep their release`
	}] };
}
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
/** The hexadecimal digits of a shape digest: 128 bits, whose collisions are negligible among any database's tables. */
var SHAPE_LENGTH = 32;
/** The logged columns of each table, described once. */
var COLUMNS = /* @__PURE__ */ new WeakMap();
/** The committed changes of one database, in commit order. */
var Log = class {
	/** The database holding the log. */
	database;
	/** Create the log of a database. */
	constructor(database) {
		this.database = database;
	}
	/** Digest the shape tables' changes carry: each one's logged columns with their kinds and values. */
	static async shape(tables) {
		const described = tables.map((table) => {
			let columns = COLUMNS.get(table);
			if (columns === void 0) {
				columns = Object.values(table[TABLE].logged).map(({ definition }) => [
					definition.name,
					definition.kind,
					definition.nullable,
					toJsonSchema(definition.json ?? definition.schema)
				]);
				COLUMNS.set(table, columns);
			}
			return [table[TABLE].sqlName, columns];
		});
		return (await digest(described)).slice(0, SHAPE_LENGTH);
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
export { encodeRow as a, Plan as c, ResourceError as d, Address as f, encodeColumns as i, Risk as l, Snapshot as n, asc as o, decodeRow as r, desc as s, Log as t, PlanError as u };

//# sourceMappingURL=log-Bax8_ncE.js.map