import { $n as Package, A as LogPosition, At as foreignKey, B as lt, Bt as json$1, Cr as templateLiteral, Dn as sql, Dr as uuidv7, Dt as Expression$1, Er as url, Et as dialectSQL, F as and, Ft as Column$1, G as Key, Gn as ResourceDescription, H as ne, Hn as canonicalize, Ht as text$1, I as eq, It as ColumnBuilder, Kn as ResourceReference, L as gt, Lt as boolean$1, Mr as __require, Mt as primaryKey, Nr as __toCommonJS, Nt as unique, Or as registry, Ot as Scalar, P as Condition, Pt as uniqueIndex, R as gte, Rn as Column, Rt as identifier$1, Sr as string, Tr as union, U as or, Ut as DatabaseError, V as lte, Vn as is, Vt as real, W as Order, Xn as DeclarationName, Yn as declaringModule, Zn as DependencyName, _r as lazy, _t as TABLE, an as isNull, ar as defineSchema, b as createContextKey, br as record$1, c as isSpanContextValid, cr as _enum, dr as boolean, dt as describeLog, er as PackageId, f as INVALID_SPAN_CONTEXT, fr as cidrv4, ft as v7, gr as json, h as TraceFlags, hr as int, in as isNotNull, ir as identifier, jt as index, kr as __commonJSMin, kt as check, lr as array, mr as discriminatedUnion, n as trace, nr as PackagePath, or as toJsonSchema, pr as cidrv6, rr as Version, sr as ZodType, t as init_trace_api, tr as Digest$1, un as not, ur as base64, vr as literal, w as DiagLogLevel, wr as tuple, wt as Expression, xr as strictObject, yr as number, yt as defineTable, z as inArray, zt as integer } from "./trace-api-DJ-_ZhlJ.js";
import { a as encodeRow, c as Plan, d as ResourceError, i as encodeColumns, l as Risk, n as Snapshot, o as asc, r as decodeRow, s as desc, t as Log } from "./log-Bax8_ncE.js";
import { $t as isObject, A as copyRequest, At as none, B as createNoopLogger, Bt as toFetchResponse, C as createClient, Ct as subjectKey, D as ServiceTelemetry, Dt as grants, E as Watermark, Et as condition, F as exceptionAttributes, Ft as ObjectReference, G as propagation, Gt as isORPCErrorStatus, H as esm_exports, Ht as ValidationError, I as instrument, It as ObjectTypeReference, J as diag, Jt as NullProtoObj$1, Kt as toORPCError, L as scope, Lt as eventIterator, M as StandardOpenAPISerializer, Mt as relation, N as standardizeHTTPPath, Nt as through, O as RequestId, Ot as intersection, P as StandardBracketNotationSerializer, Pt as union$1, Q as SpanStatusCode, Qt as isAsyncIteratorObject, R as extractContext, Rt as fallbackContractConfig, S as unlazy, St as sameSubject, T as Bookmark, Tt as Attribute, U as init_esm, Ut as flattenHeader, V as SeverityNumber, Vt as toStandardLazyRequest, Wt as ORPCError, X as context, Xt as asyncIteratorWithSpan, Yt as ORPC_NAME, Z as init_context_api, Zt as intercept, _ as getLazyMeta, _t as relationsOf$1, an as tryDecodeURIComponent, b as isProcedure, bt as Subject, cn as TRIGGER_KINDS, ct as baggageEntryMetadataFromString, d as vault, dn as ComputeDefinition, dt as ProcedureMeta, en as resolveMaybeOptionalOptions, et as SpanKind, f as web, ft as defineProcedure, g as createProcedureClient, gt as permissionKey, h as createContractedProcedure, ht as objectKey, i as notes, in as toArray, it as ValueType, j as StandardOpenAPIJsonSerializer, jt as permission, k as VERSION_HEADER, kt as nameCheck, ln as WorkloadDescription, m as AuditActionReference, mt as Policy, n as database, nn as setSpanError, nt as SamplingDecision$1, on as value, ot as createNoopMeter, p as AuditActionName, pt as PermissionReference, q as metrics, qt as AsyncIteratorClass, rn as stringifyJSON, sn as CONCURRENCIES, tn as runWithSpan, un as reference$1, ut as defineService, v as getRouter, vt as subjectType, w as BOOKMARK_HEADER, wt as AccessName, x as traverseContractProcedures, xt as keySubject, y as isLazy, yt as AccessError, z as logs, zt as toHttpPath } from "./server-DVf9XgjR.js";
import { serve } from "bun";
/** The schemas whose values are sensitive, which nothing derived from a request or record keeps, as zod metadata. */
var SENSITIVE = registry();
/** Mark a schema's values sensitive, so that nothing derived from a request or record keeps them. */
function sensitive(value) {
	SENSITIVE.add(value, { sensitive: true });
	return value;
}
/** The runtime selected by the compiler adapter. */
var Runtime = defineSchema(_enum([
	"browser",
	"bun",
	"workerd"
]));
defineSchema(_enum(["browser", "server"]));
/** A runtime that hosts execute workloads in. */
var ServerRuntime = defineSchema(Runtime.exclude(["browser"]));
/** A view a browser output compiles, as manifests describe it. */
var ViewDescription = defineSchema(strictObject({
	/** The emitted chunk mounting the view. */
	entrypoint: PackagePath,
	/** The permissions the view requests. */
	permissions: array(strictObject({
		/** The package declaring the permission. */
		packageId: PackageId,
		/** The object type. */
		type: string().min(1),
		/** The permission on that type. */
		name: string().min(1)
	}).strict())
}).strict());
/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
var ResourceBinding = defineSchema(strictObject({
	/** The resource. */
	resource: identifier("resource"),
	/** The resource kind, such as database. */
	kind: string().min(1),
	/** The provider holding the resource, whose connector the workload opens it with, such as sqlite. */
	provider: string().min(1),
	/** The provider's reference, such as a file URL or an egress URL. */
	reference: string().min(1),
	/** The credential the host lends the workload for the resource, absent when the reference needs none. */
	credential: string().min(1).optional()
}));
new TextEncoder().encode("destack seal v1");
/** One piece of a resource's content on its way from a transfer's source to its target, in its JSON form. */
var Chunk$1 = defineSchema(strictObject({
	/** Where the export continues after this chunk, which only the exporting provider reads. */
	cursor: string().min(1),
	/** The content, in the provider's own JSON form. */
	body: json()
}));
/** How far a copy goes: live content while the source still serves writes, and all of it once the source is fenced. */
var CopyStage = defineSchema(_enum(["live", "fenced"]));
defineSchema(strictObject({
	/** The ephemeral P-256 public key, as base64 of its uncompressed point. */
	key: base64(),
	/** The base64 AES-GCM nonce. */
	nonce: base64(),
	/** The base64 ciphertext with its authentication tag. */
	ciphertext: base64()
}));
/**
* Returns the number of values in `expression`.
*
* ## Examples
*
* ```ts
* // Number employees with null values
* db.select({ value: count() }).from(employees)
* // Number of employees where `name` is not null
* db.select({ value: count(employees.name) }).from(employees)
* ```
*
* @see countDistinct to get the number of non-duplicate values in `expression`
*/
function count(expression) {
	return sql`count(${expression || sql.raw("*")})`.mapWith(Number);
}
/**
* Returns the sum of all non-null values in `expression`.
*
* ## Examples
*
* ```ts
* // Sum of every employee's salary
* db.select({ value: sum(employees.salary) }).from(employees)
* ```
*
* @see sumDistinct to get the sum of all non-null and non-duplicate values in `expression`
*/
function sum(expression) {
	return sql`sum(${expression})`.mapWith(String);
}
/**
* Returns the maximum value in `expression`.
*
* ## Examples
*
* ```ts
* // The employee with the highest salary
* db.select({ value: max(employees.salary) }).from(employees)
* ```
*/
function max$1(expression) {
	return sql`max(${expression})`.mapWith(is(expression, Column) ? expression : String);
}
/**
* Returns the minimum value in `expression`.
*
* ## Examples
*
* ```ts
* // The employee with the lowest salary
* db.select({ value: min(employees.salary) }).from(employees)
* ```
*/
function min$1(expression) {
	return sql`min(${expression})`.mapWith(is(expression, Column) ? expression : String);
}
var guard$1 = (func, shouldGuard) => {
	const _guard = (err) => {
		if (shouldGuard && !shouldGuard(err)) throw err;
	};
	const isPromise2 = (result) => result instanceof Promise;
	try {
		const result = func();
		return isPromise2(result) ? result.catch(_guard) : result;
	} catch (err) {
		return _guard(err);
	}
};
/** One measure of a group: its row count, or a function of one column. */
var Measure = defineSchema(strictObject({
	/** The function. */
	function: _enum([
		"count",
		"sum",
		"avg",
		"min",
		"max"
	]),
	/** The measured column, absent for a row count. */
	column: string().min(1).optional()
}));
/** Aggregates of a query's rows per group. */
var Aggregate = defineSchema(strictObject({
	/** The group columns, one group when absent. */
	groupBy: array(string().min(1)).optional(),
	/** The measures of each group, by name. */
	values: record$1(string(), Measure)
}));
/** The source's outcome of a subscriber's mutation. */
var MutationOutcome = defineSchema(strictObject({
	/** The mutation's request identifier. */
	id: string().min(1),
	/** The recorded failure, absent once executed. */
	error: json().optional()
}));
/** One row entering, changing within or leaving a subscriber's rows. */
var RowChange = defineSchema(strictObject({
	/** The row's table, by SQL name. */
	table: string().min(1),
	/** Whether the row enters, changes or leaves. */
	operation: _enum([
		"insert",
		"update",
		"delete"
	]),
	/** The row's columns as JSON, or its key for a deletion. */
	row: record$1(string(), json()),
	/** The row before an update, as JSON. */
	before: record$1(string(), json()).optional(),
	/** The columns the subscriber may not read. */
	concealed: array(string()).optional()
}));
/** One aggregate group taking new values or leaving, or every group leaving. */
var ResultChange = defineSchema(strictObject({
	/** The query, by its path of names. */
	query: string().min(1),
	/** The group's values, the join value first for an include, null for every group. */
	group: record$1(string(), Scalar).nullable(),
	/** The group's measures, or null once empty. */
	values: record$1(string(), Scalar).nullable(),
	/** The group's row count. */
	rows: number().int().positive().optional(),
	/** Each average's sum and count of present values. */
	parts: record$1(string(), strictObject({
		sum: Scalar,
		count: number().int()
	})).optional()
}));
/** An event sent to one topic, never stored. */
var Broadcast = defineSchema(strictObject({
	/** The topic the event was sent to. */
	topic: string().min(1),
	/** The event. */
	event: json()
}));
/** A page of query rows and aggregates. */
var QueryPage = defineSchema(strictObject({
	/** Whether the page starts a snapshot. */
	reset: boolean(),
	/** Whether the subscriber holds all its queries after the page. */
	complete: boolean(),
	/** The row changes. */
	changes: array(RowChange),
	/** The group changes. */
	results: array(ResultChange).optional(),
	/** The log position to continue after. */
	position: LogPosition,
	/** The outcomes of the subscriber's mutations. */
	outcomes: array(MutationOutcome).optional(),
	/** The events since the last page. */
	broadcasts: array(Broadcast).optional(),
	/** The scopes the subscription reads, nearest first, when its source sends them. */
	scopes: array(string().min(1)).optional()
}));
/** Return the maximum value. */
function max(expression) {
	const aggregate = max$1(expression);
	return expression instanceof Column$1 ? aggregate.mapWith(expression) : aggregate;
}
/** Return the minimum value. */
function min(expression) {
	const aggregate = min$1(expression);
	return expression instanceof Column$1 ? aggregate.mapWith(expression) : aggregate;
}
/** Validate a selected record, in application or JSON form. */
function createSelectSchema(table, form = "application") {
	return createSchema(table, "select", form);
}
/** Validate an inserted record, in application or JSON form. */
function createInsertSchema(table, form = "application") {
	return createSchema(table, "insert", form);
}
/** Build a record validator. */
function createSchema(table, operation, form = "application") {
	const fields = {};
	for (const [property, column] of Object.entries(table[TABLE].columns)) {
		const definition = column.definition;
		if (operation !== "select" && definition.generated) continue;
		let validator = form === "json" && definition.json !== void 0 ? definition.json : definition.schema;
		if (definition.nullable) validator = validator.nullable();
		if (operation === "update" || operation === "insert" && (definition.nullable || definition.default !== void 0 || definition.runtimeDefault !== void 0 || definition.runtimeUpdate !== void 0)) validator = validator.optional();
		fields[property] = validator;
	}
	return strictObject(fields);
}
/** The column kinds sums and averages add up. */
var SUMMED_KINDS = /* @__PURE__ */ new Set([
	"integer",
	"real",
	"bigint"
]);
/** One node of a resolved query tree. */
var Node = class Node {
	/** The node's path of names, such as `tasks.comments`. */
	name;
	/** The logged table. */
	table;
	/** The scopes the node's rows live in. */
	scopes;
	/** The logged columns, by property. */
	columns;
	/** The logged columns in declaration order. */
	#logged;
	/** The primary key's properties in key order. */
	key;
	/** The computed values, by name. */
	computed;
	/** The condition the rows meet. */
	where;
	/** How the rows sort, completed by the primary key. */
	order;
	/** The most rows the node holds per held parent row, or in all for a root. */
	limit;
	/** Whether the node's windows arrange every candidate. */
	isArranged;
	/** How the rows relate to the parent node's rows, absent for a root. */
	path;
	/** The tree index of a descendants or ancestors path. */
	closure;
	/** The parent node, absent for a root. */
	parent;
	/** The count of nodes above this one. */
	depth;
	/** The included nodes and the join row nodes of junction paths. */
	children;
	/** What the node's rows are to its parent: included rows, join rows or relation witnesses. */
	kind;
	/** The relations the condition and computed values follow, by name and condition. */
	#relations = /* @__PURE__ */ new Map();
	/** The relations by name and condition object. */
	#found = /* @__PURE__ */ new Map();
	/** The relations computed values look up. */
	#lookups;
	/** The relations and conditions computed values measure. */
	#rollups;
	/** The columns the parent node reads through this relation node. */
	#looks = /* @__PURE__ */ new Set();
	/** The node's aggregates. */
	aggregate;
	/** Whether the node holds rows. */
	isHolding;
	/** The rows in the tree's scopes meeting the condition. */
	#selection;
	/** The name of what the node selects. */
	selection;
	/** Resolve a query or include as a node. */
	constructor(name, query, within, parent, kind = "include") {
		const table = query.table;
		const definition = table[TABLE];
		const scopes = "on" in query && query.on.kind === "key" && query.on.column === "scope" ? "every" : within;
		this.name = name;
		this.kind = kind;
		this.table = table;
		this.scopes = scopes;
		this.columns = describeLog(table) === void 0 ? definition.columns : table[TABLE].logged;
		this.#logged = Object.entries(this.columns);
		this.key = table[TABLE].key;
		this.computed = query.compute ?? {};
		for (const computed of Object.keys(this.computed)) if (Object.hasOwn(definition.columns, computed)) throw new DatabaseError("INVALID_QUERY", `computed value ${computed} shadows a column of ${definition.name}`);
		this.where = query.where;
		this.#selection = Condition.all(Node.scoped(scopes), ...query.where === void 0 ? [] : [query.where]);
		this.selection = JSON.stringify([
			definition.sqlName,
			this.computed,
			this.#selection,
			query.order ?? [],
			"on" in query ? describePath(query.on) : null,
			describeRelations(query.relations)
		]);
		this.order = Order.complete(query.order ?? [], table);
		if (query.limit !== void 0 && (!Number.isInteger(query.limit) || query.limit < 1)) throw new DatabaseError("INVALID_QUERY", `query limit must be a positive integer: ${name}`);
		this.limit = query.limit;
		this.isArranged = query.limit !== void 0 && this.order.some(({ column }) => {
			const expression = this.computed[column];
			return expression !== void 0 && (Expression.lookups(expression).length > 0 || Expression.rollups(expression).length > 0);
		});
		this.parent = parent;
		this.depth = parent === void 0 ? 0 : parent.depth + 1;
		this.path = "on" in query ? query.on : void 0;
		if (this.path !== void 0) this.#requirePath(this.path, parent);
		this.closure = this.path?.kind === "descendants" || this.path?.kind === "ancestors" ? definition.tree.ancestors : void 0;
		this.aggregate = query.aggregate;
		this.isHolding = query.aggregate === void 0 && (parent?.isHolding ?? true);
		const includes = Object.entries(query.include ?? {}).map(([child, include]) => new Node(`${name}.${child}`, include, scopes, this));
		const lookups = Object.values(this.computed).flatMap(Expression.lookups);
		const rollups = Object.values(this.computed).flatMap(Expression.rollups);
		const uses = /* @__PURE__ */ new Map();
		const use = (via, where, measured, measure) => {
			const key = JSON.stringify([via, where ?? null]);
			const entry = uses.get(key) ?? {
				via,
				where,
				values: { count: { function: "count" } }
			};
			uses.set(key, entry);
			if (measured !== void 0) entry.values[measured] = measure;
		};
		for (const { via, where } of Condition.relations(query.where ?? Condition.all())) use(via, where);
		for (const { via, column } of lookups) use(via, void 0, measureName("min", column), {
			function: "min",
			column
		});
		for (const { function: measure, via, column, where } of rollups) if (measure === "count") use(via, where);
		else use(via, where, measureName(measure, column), {
			function: measure,
			column
		});
		const relations = [...uses.entries()].map(([key, { via, where, values }]) => {
			const relation = query.relations?.[via];
			if (relation === void 0) throw new DatabaseError("INVALID_QUERY", `relation ${via} is not declared by ${name}`);
			else if (relation.on.kind !== "key" && relation.on.kind !== "junction") throw new DatabaseError("INVALID_QUERY", `relation ${via} of ${name} follows a key or junction path`);
			else if (relation.table === table && rollups.some((rollup) => rollup.via === via)) throw new DatabaseError("INVALID_QUERY", `rollup ${via} of ${name} measures another table`);
			const conditions = [relation.where, where].filter((condition) => condition !== void 0);
			const node = new Node(relationName(name, via, where), {
				table: relation.table,
				on: relation.on,
				...conditions.length === 0 ? {} : { where: Condition.all(...conditions) },
				...relation.relations === void 0 ? {} : { relations: relation.relations },
				aggregate: { values }
			}, scopes, this, "relation");
			this.#relations.set(key, node);
			return node;
		});
		this.#lookups = new Set(lookups.map(({ via }) => via));
		for (const { via, column } of lookups) this.relation(via, void 0).#looks.add(column);
		this.#rollups = new Set(rollups.map(({ via, where }) => JSON.stringify([via, where ?? null])));
		for (const rollup of rollups) if (rollup.column !== void 0) this.relation(rollup.via, rollup.where).#looks.add(rollup.column);
		for (const via of this.#lookups) {
			const relation = this.relation(via, void 0);
			const path = relation.path;
			if (path.kind !== "key" || relation.table === table || relation.key.length !== 1 || relation.key[0] !== path.column) throw new DatabaseError("INVALID_QUERY", `lookup ${via} of ${name} follows a key path onto another table's key`);
		}
		this.children = [...includes, ...relations].flatMap((child) => [child, ...child.path?.kind === "junction" && child.kind === "include" ? [new Node(`${child.name}#join`, {
			table: child.path.table,
			on: {
				kind: "key",
				column: child.path.from.column,
				parent: child.path.from.key
			}
		}, scopes, this, "join")] : []]);
		const namespace = this.namespace();
		for (const expression of Object.values(this.computed)) {
			Expression.require(expression, table, namespace);
			this.#requireLogged(Expression.columns(expression));
		}
		if (query.where !== void 0) {
			Condition.require(query.where, table, namespace);
			this.#requireLogged(Condition.columns(query.where));
		}
		Order.require(query.order ?? [], table, namespace);
		this.#requireLogged(new Set((query.order ?? []).map((key) => key.column)));
		if (query.aggregate !== void 0) this.#requireAggregate(query);
	}
	/** List the columns the node reads. */
	reads() {
		const names = [
			...this.where === void 0 ? [] : Condition.columns(this.where),
			...this.order.map((key) => key.column),
			...this.path === void 0 ? [] : columnsOf(this.path, "included"),
			...this.children.flatMap((child) => child.path === void 0 ? [] : columnsOf(child.path, "held")),
			...this.grouping,
			...Object.values(this.aggregate?.values ?? {}).flatMap((measure) => measure.column === void 0 ? [] : [measure.column]),
			...this.#looks
		];
		return [...new Set(names.flatMap((name) => this.columnsOf(name)))];
	}
	/** List the logged columns a column or computed value reads. */
	columnsOf(name) {
		return Object.hasOwn(this.computed, name) ? [...Expression.columns(this.computed[name])] : [name];
	}
	/** List this node and every node below it. */
	nodes() {
		return [this, ...this.children.flatMap((child) => child.nodes())];
	}
	/** Name a row by its table and its key's values. */
	keyOf(row) {
		return Key.name(this.table, row);
	}
	/** The held row's column with the partition's value, absent for a root. */
	get parentColumn() {
		const path = this.path;
		return path === void 0 ? void 0 : path.kind === "key" ? path.parent : path.kind === "junction" ? path.from.key : this.key[0];
	}
	/** The group field naming the held row of a group, absent for a root. */
	get partitionName() {
		const path = this.path;
		return path === void 0 ? void 0 : path.kind === "key" ? path.column : path.kind === "junction" ? path.from.column : path.kind === "descendants" ? "ancestor" : "descendant";
	}
	/** Name the group of a held value's related rows. */
	groupFor(value) {
		return { [this.partitionName]: this.parent.json(this.parentColumn, value) };
	}
	/** Read the value naming a held parent row's partition. */
	valueOf(parent) {
		return parent[this.parentColumn];
	}
	/** Name the partition of a held value. */
	partition(value) {
		return this.path === void 0 ? "" : JSON.stringify(this.parent.json(this.parentColumn, value));
	}
	/** Express rows in some scopes as a condition. */
	static scoped(scopes) {
		return scopes === "every" ? Condition.all() : Condition.oneOf("scope", scopes);
	}
	/** Decide whether a row lives in some scopes. */
	static isScoped(row, scopes) {
		return scopes === "every" || scopes.includes(row.scope);
	}
	/** Read the values a row sorts by. */
	orderOf(row) {
		const values = {};
		for (const { column } of this.order) values[column] = row[column];
		return values;
	}
	/** Write a row's logged columns in JSON form without concealed ones. */
	encode(row, concealed) {
		return encodeColumns(this.#logged, row, concealed);
	}
	/** Name the closure rows holding a tree row at one end. */
	paths(row, end) {
		const tree = this.table[TABLE].tree.definition;
		return {
			scope: row[tree.scope],
			[end]: row[tree.id]
		};
	}
	/** Read the measures of no rows. */
	emptyValues() {
		return Object.fromEntries(Object.entries(this.aggregate.values).map(([name, measure]) => [name, measure.function === "count" ? 0 : null]));
	}
	/** Compare two rows by the node's order. */
	compare(left, right) {
		return Order.rows(this.order, left, right);
	}
	/** Find the node of a relation under a condition. */
	relation(via, where) {
		const known = this.#found.get(via)?.get(where);
		if (known !== void 0) return known;
		const found = this.#relations.get(JSON.stringify([via, where ?? null]));
		const byCondition = this.#found.get(via) ?? /* @__PURE__ */ new Map();
		this.#found.set(via, byCondition.set(where, found));
		return found;
	}
	/** Whether computed values read related rows. */
	get isRelating() {
		return this.#lookups.size > 0 || this.#rollups.size > 0;
	}
	/** The nodes of the relations the condition follows. */
	get relations() {
		return [...this.#relations.values()];
	}
	/** Select the node's rows, within one partition when given one. */
	select(partition, visibility) {
		return Condition.render(this.condition(partition), this.bind(visibility));
	}
	/** Bind the node's condition. */
	bind(visibility) {
		return Condition.bind(this.table, {}, this.namespace(visibility));
	}
	/** Name what the node's condition, order and computed values read beyond its columns. */
	namespace(visibility) {
		return {
			computed: this.computed,
			exists: (via, where) => this.relation(via, where).within(visibility),
			lookup: (via, column) => {
				const relation = this.relation(via, void 0);
				const read = relation.columns[column];
				if (read === void 0) throw new DatabaseError("INVALID_QUERY", `relation ${via} looks up no column ${column} of ${relation.table[TABLE].name}`);
				return {
					definition: read.definition,
					value: sql`(SELECT ${read} FROM ${relation.table} WHERE ${relation.joins(this, visibility)}
                        AND ${relation.select(void 0, visibility)} AND ${visibility?.(relation.table) ?? sql`true`})`
				};
			},
			rollup: (measure, via, column, where) => {
				const relation = this.relation(via, where);
				const read = column === void 0 ? void 0 : relation.columns[column];
				if (column !== void 0 && read === void 0) throw new DatabaseError("INVALID_QUERY", `relation ${via} measures no column ${column} of ${relation.table[TABLE].name}`);
				const aggregate = measure === "count" ? sql`count(*)` : sql`${sql.raw(measure)}(${measure === "sum" ? read : Order.text(read)})`;
				return {
					...read === void 0 ? {} : { definition: read.definition },
					value: sql`(SELECT ${aggregate} FROM ${relation.table} WHERE ${relation.joins(this, visibility)}
                        AND ${relation.select(void 0, visibility)} AND ${visibility?.(relation.table) ?? sql`true`})`
				};
			}
		};
	}
	/** Join a relation node's rows to a held parent row in SQL. */
	joins(parent, visibility) {
		const path = this.path;
		if (path.kind === "key") return sql`${this.columns[path.column]} = ${parent.columns[path.parent]}`;
		else if (path.kind === "junction") {
			const joins = path.table[TABLE].columns;
			return sql`${this.columns[path.to.key]} IN (SELECT ${joins[path.to.column]} FROM ${path.table}
                WHERE ${joins[path.from.column]} = ${parent.columns[path.from.key]}
                AND ${this.#joinRows(path.table, visibility)})`;
		}
		throw new TypeError(`a ${path.kind} relation has no SQL join`);
	}
	/** Test in SQL whether a relation node holds a related row for its parent's rows. */
	within(visibility) {
		const path = this.path;
		const held = this.parent.columns[this.parentColumn];
		const related = (column) => sql`SELECT ${this.columns[column]} FROM ${this.table} WHERE ${this.columns[column]} IS NOT NULL
                AND ${this.select(void 0, visibility)} AND ${visibility?.(this.table) ?? sql`true`}`;
		let values;
		if (path.kind === "key") values = related(path.column);
		else if (path.kind === "junction") {
			const joins = path.table[TABLE].columns;
			values = sql`SELECT ${joins[path.from.column]} FROM ${path.table} WHERE ${joins[path.from.column]} IS NOT NULL
                AND ${this.#joinRows(path.table, visibility)}
                AND ${joins[path.to.column]} IN (${related(path.to.key)})`;
		} else throw new TypeError(`a ${path.kind} relation has no SQL test`);
		return sql`(${held} IS NOT NULL AND ${held} IN (${values}))`;
	}
	/** Match the visible join rows of a junction path. */
	#joinRows(table, visibility) {
		const scoped = Condition.render(Node.scoped(this.scopes), Condition.bind(table));
		return sql`${scoped} AND ${visibility?.(table) ?? sql`true`}`;
	}
	/** Express the node's rows as a condition, within one partition when given one. */
	condition(partition) {
		if (this.path === void 0 || partition === void 0) return this.#selection;
		else if (this.path.kind === "key") return Condition.all(this.#selection, Condition.eq(this.path.column, this.json(this.path.column, partition)));
		throw new TypeError(`a ${this.path.kind} path selects its partitions through other rows`);
	}
	/** The group columns, the key-joined column first for an include. */
	get grouping() {
		return [...this.path?.kind === "key" ? [this.path.column] : [], ...this.aggregate?.groupBy ?? []];
	}
	/** Read a row's group within a partition, in JSON form. */
	groupOf(row, value) {
		const grouped = (this.aggregate?.groupBy ?? []).map((name) => [name, this.json(name, row[name])]);
		const partition = this.path === void 0 ? [] : [[this.partitionName, this.path.kind === "key" ? this.json(this.path.column, row[this.path.column]) : this.parent.json(this.parentColumn, value)]];
		return Object.fromEntries([...partition, ...grouped]);
	}
	/** Write a column value in JSON form. */
	json(name, value) {
		return value === null || value === void 0 ? null : Object.hasOwn(this.computed, name) ? value === 0 ? 0 : value : this.columns[name].definition.toJson(value);
	}
	/** Read a column value from its JSON form. */
	fromJson(name, json) {
		return json === null || Object.hasOwn(this.computed, name) ? json : this.columns[name].definition.fromJson(json);
	}
	/** Read the kind of a column or computed value. */
	kindOf(name) {
		return Object.hasOwn(this.computed, name) ? Expression.kind(this.computed[name], this.table, this.namespace()) : this.columns[name].definition.kind;
	}
	/** Render a column or computed value in SQL. */
	sql(name, visibility) {
		return Object.hasOwn(this.computed, name) ? Expression.render(this.computed[name], this.table, this.namespace(visibility)) : this.columns[name];
	}
	/** Require logged columns or computed values. */
	#requireLogged(names) {
		for (const name of names) if (!Object.hasOwn(this.columns, name) && !Object.hasOwn(this.computed, name)) throw new DatabaseError("INVALID_QUERY", `query column is not logged: ${this.table[TABLE].name}.${name}`);
	}
	/** Require a path over logged columns. */
	#requirePath(path, parent) {
		if (path.kind === "key") {
			this.#requireLogged(/* @__PURE__ */ new Set([path.column]));
			parent.#requireLogged(/* @__PURE__ */ new Set([path.parent]));
		} else if (path.kind === "junction") {
			const joins = path.table[TABLE];
			const logged = path.table[TABLE].logged;
			if (describeLog(path.table) === void 0) throw new DatabaseError("INVALID_QUERY", `join table is not logged: ${joins.name}`);
			for (const name of [path.from.column, path.to.column]) if (!Object.hasOwn(logged, name)) throw new DatabaseError("INVALID_QUERY", `join column is not logged: ${joins.name}.${name}`);
			this.#requireLogged(/* @__PURE__ */ new Set([path.to.key]));
			parent.#requireLogged(/* @__PURE__ */ new Set([path.from.key]));
		} else if (parent.table !== this.table || this.key.length !== 1) throw new DatabaseError("INVALID_QUERY", `a ${path.kind} path follows one table keyed by one column: ${this.name}`);
		else if (this.table[TABLE].tree?.definition.parent !== path.column || this.table[TABLE].tree.definition.id !== this.key[0]) throw new DatabaseError("INVALID_QUERY", `a ${path.kind} path follows the tree index of its table's parent column: ${this.name}`);
		else this.#requireLogged(/* @__PURE__ */ new Set([path.column]));
	}
	/** Require a valid aggregate. */
	#requireAggregate(query) {
		const aggregate = query.aggregate;
		if (query.include !== void 0 || query.order !== void 0 || query.limit !== void 0) throw new DatabaseError("INVALID_QUERY", `aggregate query holds no rows to order or include: ${this.name}`);
		Order.require((aggregate.groupBy ?? []).map((column) => ({
			column,
			direction: "asc"
		})), this.table, this.namespace());
		this.#requireLogged(new Set(aggregate.groupBy ?? []));
		for (const [name, measure] of Object.entries(aggregate.values)) if (measure.function === "count" !== (measure.column === void 0)) throw new DatabaseError("INVALID_QUERY", `measure ${name} counts rows or measures one column`);
		else if (measure.column !== void 0) {
			Order.require([{
				column: measure.column,
				direction: "asc"
			}], this.table, this.namespace());
			this.#requireLogged(/* @__PURE__ */ new Set([measure.column]));
			const kind = this.kindOf(measure.column);
			if ((measure.function === "sum" || measure.function === "avg") && !SUMMED_KINDS.has(kind)) throw new DatabaseError("INVALID_QUERY", `measure ${name} adds up a non-numeric column`);
		}
	}
};
/** Name a measure of a relation's rows. */
function measureName(measure, column) {
	return `${measure}(${column})`;
}
/** Name the aggregate of a relation under a condition. */
function relationName(name, via, where) {
	return where === void 0 ? `${name}?${via}` : `${name}?${via}${canonicalize(where)}`;
}
/** Describe relations in JSON. */
function describeRelations(relations) {
	return Object.entries(relations ?? {}).map(([via, relation]) => [
		via,
		relation.table[TABLE].sqlName,
		describePath(relation.on),
		describeRelations(relation.relations)
	]);
}
/** Describe a path in JSON. */
function describePath(path) {
	return path.kind === "junction" ? {
		...path,
		table: path.table[TABLE].sqlName
	} : path;
}
/** List the columns a path reads on one side. */
function columnsOf(path, side) {
	return path.kind === "key" ? [side === "included" ? path.column : path.parent] : path.kind === "junction" ? [side === "included" ? path.to.key : path.from.key] : side === "included" ? [path.column] : [];
}
/** A copy, subscription or scope the source cannot serve. */
var SyncError = class extends Error {
	/** The error classification. */
	code;
	/** Create the error with its code and cause. */
	constructor(code, message, options) {
		super(message, options);
		this.name = "SyncError";
		this.code = code;
	}
};
/** An audience that holds and reads every row. */
var EVERYONE = {
	watches: [],
	where: () => sql`true`,
	key: "everyone",
	admits: async (_table, rows) => new Set(rows.keys()),
	concealable: () => [],
	conceals: async (_table, rows) => rows.map(() => []),
	dependents: async () => [],
	until: async () => void 0,
	refresh: async () => {}
};
/** Join the scopes some watches read for one log read, absent when one reads every scope. */
function watchedScopes(watches) {
	const scopes = /* @__PURE__ */ new Set();
	for (const { scopes: watched } of watches) {
		if (watched === "every") return;
		for (const scope of watched) scopes.add(scope);
	}
	return [...scopes];
}
/** Decide a node's rows: compute their values and keep the visible candidates. */
var Filter = class {
	/** The node whose rows the filter decides. */
	node;
	/** The arrangement of the relations' measures. */
	#arrangement;
	/** The computed values' names and expressions. */
	#formulas;
	/** The node's compiled scopes and condition. */
	#match;
	/** Each row with its computed values, for nodes without lookups. */
	#computed = /* @__PURE__ */ new WeakMap();
	/** The rows holding their computed values. */
	#resolved = /* @__PURE__ */ new WeakSet();
	/** Create the filter of a node. */
	constructor(node, arrangement) {
		this.node = node;
		this.#arrangement = arrangement;
		this.#formulas = Object.entries(node.computed);
		this.#match = Condition.compile(node.condition(), node.table);
	}
	/** Decide some rows' visibility and read their relations' measures. */
	async prepare(rows, run) {
		await run.decide(this.node.table, rows);
		if (this.node.relations.length > 0) await this.#arrangement.prepare(this.node, rows, run);
	}
	/** Decide whether a prepared row is a visible, matching candidate. */
	isCandidate(row, run) {
		return run.isVisible(row) && this.meets(row, run);
	}
	/** Decide whether a prepared row is in the node's scopes and meets its condition. */
	meets(row, run) {
		const resolved = this.resolve(row, run);
		return this.#match({
			column: (name) => resolved[name],
			parameter: unbound,
			exists: (via, where) => {
				const relation = this.node.relation(via, where);
				return (this.#arrangement.measured(relation, relation.valueOf(row), run).count ?? 0) > 0;
			}
		}) === true;
	}
	/** Read a prepared row with the node's computed values. */
	resolve(row, run) {
		if (this.#formulas.length === 0 || this.#resolved.has(row)) return row;
		const byRow = this.node.isRelating ? run.resolvedBy(this) : this.#computed;
		const known = byRow.get(row);
		if (known !== void 0) return known;
		const related = this.related(row, run);
		const values = { ...row };
		for (const [name, expression] of this.#formulas) values[name] = Expression.evaluate(expression, row, related);
		byRow.set(row, values);
		this.#resolved.add(values);
		return values;
	}
	/** Read a prepared row's lookups and rollups. */
	related(row, run) {
		const node = this.node;
		return {
			lookup: (via, column) => {
				const relation = node.relation(via, void 0);
				const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);
				return relation.fromJson(column, measured[measureName("min", column)] ?? null);
			},
			rollup: (measure, via, column, where) => {
				const relation = node.relation(via, where);
				const measured = this.#arrangement.measured(relation, relation.valueOf(row), run);
				return measure === "count" ? measured.count ?? 0 : relation.fromJson(column, measured[measureName(measure, column)] ?? null);
			}
		};
	}
	/** Take a row as holding its computed values. */
	adopt(row) {
		this.#resolved.add(row);
		return row;
	}
};
/** Refuse parameters. */
function unbound(name) {
	throw new DatabaseError("INVALID_QUERY", `query conditions bind no parameters: ${name}`);
}
/** The source of a node's rows: a scan of a root's scopes, or a join to held parent rows. */
var Input = class {
	/** The node whose rows the input reads. */
	node;
	/** Read a node's rows. */
	constructor(node) {
		this.node = node;
	}
	/** Create the input of a node's path. */
	static of(node, hasTreeIndex) {
		const path = node.path;
		return path === void 0 ? new Scan(node) : path.kind === "key" ? new KeyJoin(node) : path.kind === "junction" ? new JunctionJoin(node) : new TreeJoin(node, hasTreeIndex);
	}
	/** Add the rows a changed row moves beyond itself to the rows to decide again. */
	async moves(_table, _before, _after, _isAccess, _run, _dirty) {}
	/** Keep the visible rows of a table in the node's scopes. */
	async passable(table, rows, run) {
		return run.seen(table, rows.filter((row) => Node.isScoped(row, this.node.scopes)));
	}
};
/** A root's one partition: every row of its scopes. */
var Scan = class extends Input {
	/** Read every row of the scopes. */
	async members(values, run) {
		const rows = await run.view.matching(this.node.table, Node.scoped(this.node.scopes));
		return values.map(() => rows);
	}
	/** Place every row in the one partition. */
	async locate(rows) {
		return rows.map(() => [void 0]);
	}
};
/** A key path: the rows whose column holds a held row's value. */
var KeyJoin = class extends Input {
	/** The included row's column. */
	get #column() {
		return this.node.path.column;
	}
	/** Read the rows holding each value. */
	async members(values, run) {
		return (await run.view.lookup(this.node.table, values.map((value) => ({ [this.#column]: value })))).map((rows) => [...rows]);
	}
	/** Name each row's own value. */
	async locate(rows) {
		return rows.map((row) => {
			const value = row[this.#column];
			return value === null || value === void 0 ? [] : [value];
		});
	}
};
/** A junction path: the rows a held row's visible join rows name. */
var JunctionJoin = class extends Input {
	/** The path. */
	get #path() {
		return this.node.path;
	}
	/** Read the rows each value's visible join rows name, once each. */
	async members(values, run) {
		const path = this.#path;
		const joins = await run.view.lookup(path.table, values.map((value) => ({ [path.from.column]: value })));
		const passable = new Set(await this.passable(path.table, joins.flat(), run));
		const named = joins.map((rows) => rows.filter((join) => passable.has(join)));
		return unflatten(await run.view.lookup(this.node.table, named.flat().map((join) => ({ [path.to.key]: join[path.to.column] }))), named).map((reached) => [...new Map(reached.flat().map((row) => [this.node.keyOf(row), row])).values()]);
	}
	/** Name the held values each row's visible join rows join. */
	async locate(rows, run) {
		const path = this.#path;
		const joins = await run.view.lookup(path.table, rows.map((row) => ({ [path.to.column]: row[path.to.key] })));
		const passable = new Set(await this.passable(path.table, joins.flat(), run));
		return joins.map((entries) => {
			const values = /* @__PURE__ */ new Map();
			for (const join of entries) {
				const value = join[path.from.column];
				if (passable.has(join) && value !== null && value !== void 0) values.set(this.node.partition(value), value);
			}
			return [...values.values()];
		});
	}
	/** Decide again the rows a changed join row names. */
	async moves(table, before, after, _isAccess, run, dirty) {
		const path = this.#path;
		if (table !== path.table) return;
		const images = [before, after].filter((image) => image !== void 0);
		const named = await run.view.lookup(this.node.table, images.map((image) => ({ [path.to.key]: image[path.to.column] })));
		for (const row of named.flat()) dirty.set(this.node.keyOf(row), row);
	}
};
/** A tree path: every visible row below or above a held row. */
var TreeJoin = class extends Input {
	/** Whether the database holds the tree index. */
	#isIndexed;
	/** Whether the path reaches down, to descendants. */
	#isDown;
	/** The column naming a row's parent. */
	#column;
	/** The table's one key column. */
	#key;
	/** Create the input of a tree path. */
	constructor(node, isIndexed) {
		super(node);
		this.#isIndexed = isIndexed;
		this.#isDown = node.path.kind === "descendants";
		this.#column = node.path.column;
		this.#key = node.key[0];
	}
	/** Read the rows below each held row, or above it. */
	async members(values, run) {
		const held = await run.view.keyed(this.node.table, values.map((value) => ({ [this.#key]: value })));
		const present = held.filter((row) => row !== void 0);
		const reached = this.#isDown ? await this.below(present, run) : await this.above(present, run);
		const byRow = new Map(present.map((row, index) => [row, reached[index]]));
		return held.map((row) => row === void 0 ? [] : byRow.get(row));
	}
	/** Name the visible rows above or below each row. */
	async locate(rows, run) {
		return (this.#isDown ? await this.above(rows, run) : await this.below(rows, run)).map((entries) => entries.map((entry) => entry[this.#key]));
	}
	/** Decide again the rows a changed row carries along a tree. */
	async moves(table, before, after, isAccess, run, dirty) {
		const node = this.node;
		const end = this.#isDown ? "descendant" : "ancestor";
		if (!this.#isIndexed && table === node.table) {
			const images = isAccess || before === void 0 || after === void 0 || Order.missingFirst(before[this.#column], after[this.#column]) !== 0 || await this.#passes(before, run) !== await this.#passes(after, run) ? [before, after].filter((image) => image !== void 0) : [];
			const reached = this.#isDown ? await this.below(images, run) : await this.above(images, run);
			for (const row of reached.flat()) dirty.set(node.keyOf(row), row);
		} else if (this.#isIndexed && table === node.closure) {
			const ends = [before, after].filter((image) => image !== void 0 && image.depth !== 0).map((image) => image[end]);
			await this.#mark(ends, run, dirty);
		} else if (this.#isIndexed && table === node.table && (isAccess || await this.#passes(before, run) !== await this.#passes(after, run))) {
			const other = end === "descendant" ? "ancestor" : "descendant";
			const [paths] = await run.view.lookup(node.closure, [node.paths(after ?? before, other)]);
			const ends = paths.filter((path) => path.depth !== 0).map((path) => path[end]);
			await this.#mark(ends, run, dirty);
		}
	}
	/** Read the rows strictly between each member and its held row. */
	async between(pairs, run) {
		const key = this.#key;
		const held = this.#isDown ? [] : await run.view.keyed(this.node.table, pairs.map(({ value }) => ({ [key]: value })));
		const lower = pairs.map(({ member }, index) => this.#isDown ? member : held[index]);
		const present = lower.filter((row) => row !== void 0);
		const reached = await this.above(present, run);
		const byRow = new Map(present.map((row, index) => [row, reached[index]]));
		return pairs.map(({ member, value }, index) => {
			const upper = this.#isDown ? value : member[key];
			const rows = [];
			const row = lower[index];
			for (const entry of row === void 0 ? [] : byRow.get(row)) {
				if (Order.values(entry[key], upper) === 0) return rows;
				rows.push(entry);
			}
			return [];
		});
	}
	/** Read the visible rows above each row, nearest first. */
	async above(rows, run) {
		if (!this.#isIndexed) return this.#walkUp(rows, run);
		const node = this.node;
		const nearest = (await run.view.lookup(node.closure, rows.map((row) => node.paths(row, "descendant")))).map((entries) => entries.filter((path) => path.depth > 0).sort((left, right) => left.depth - right.depth));
		const ancestors = await run.view.keyed(node.table, nearest.flat().map((path) => ({ [this.#key]: path.ancestor })));
		const passable = new Set(await this.passable(node.table, ancestors.filter((row) => row !== void 0), run));
		return unflatten(ancestors, nearest).map((entries) => {
			const blocked = entries.findIndex((ancestor) => ancestor === void 0 || !passable.has(ancestor));
			return blocked === -1 ? entries : entries.slice(0, blocked);
		});
	}
	/** Read the visible rows below each row, shallowest first. */
	async below(rows, run) {
		if (!this.#isIndexed) return this.#walkDown(rows, run);
		const node = this.node;
		const shallowest = (await run.view.lookup(node.closure, rows.map((row) => node.paths(row, "ancestor")))).map((entries) => entries.filter((path) => path.depth > 0).sort((left, right) => left.depth - right.depth));
		const descendants = await run.view.keyed(node.table, shallowest.flat().map((path) => ({ [this.#key]: path.descendant })));
		const passable = new Set(await this.passable(node.table, descendants.filter((row) => row !== void 0), run));
		return unflatten(descendants, shallowest).map((entries, position) => {
			const reached = /* @__PURE__ */ new Set([rows[position][this.#key]]);
			const kept = [];
			for (const descendant of entries) if (descendant !== void 0 && passable.has(descendant) && reached.has(descendant[this.#column])) {
				reached.add(descendant[this.#key]);
				kept.push(descendant);
			}
			return kept;
		});
	}
	/** Walk each row's parent column up, a level at a time. */
	async #walkUp(rows, run) {
		const key = this.#key;
		const walks = rows.map((row) => ({
			rows: [],
			seen: /* @__PURE__ */ new Set([row[key]]),
			current: row
		}));
		for (;;) {
			const open = walks.filter((walk) => {
				const parent = walk.current?.[this.#column];
				const isOpen = parent !== null && parent !== void 0 && !walk.seen.has(parent);
				if (!isOpen) walk.current = void 0;
				return isOpen;
			});
			if (open.length === 0) return walks.map((walk) => walk.rows);
			const parents = await run.view.keyed(this.node.table, open.map((walk) => ({ [key]: walk.current[this.#column] })));
			const passable = new Set(await this.passable(this.node.table, parents.filter((row) => row !== void 0), run));
			for (const [index, walk] of open.entries()) {
				const parent = parents[index];
				if (parent === void 0 || !passable.has(parent)) walk.current = void 0;
				else {
					walk.seen.add(parent[key]);
					walk.rows.push(parent);
					walk.current = parent;
				}
			}
		}
	}
	/** Walk each row's children down, a level at a time. */
	async #walkDown(rows, run) {
		const key = this.#key;
		const walks = rows.map((row) => ({
			rows: [],
			seen: /* @__PURE__ */ new Set([row[key]]),
			level: [row]
		}));
		while (walks.some((walk) => walk.level.length > 0)) {
			const parents = walks.flatMap((walk) => walk.level);
			const children = await run.view.lookup(this.node.table, parents.map((parent) => ({ [this.#column]: parent[key] })));
			const passable = new Set(await this.passable(this.node.table, children.flat(), run));
			const levels = unflatten(children, walks.map((walk) => walk.level));
			for (const [position, walk] of walks.entries()) {
				const next = [];
				for (const child of levels[position].flat()) if (passable.has(child) && !walk.seen.has(child[key])) {
					walk.seen.add(child[key]);
					next.push(child);
				}
				walk.rows.push(...next);
				walk.level = next;
			}
		}
		return walks.map((walk) => walk.rows);
	}
	/** Decide whether the path may pass through a row. */
	async #passes(row, run) {
		return row !== void 0 && (await this.passable(this.node.table, [row], run)).length > 0;
	}
	/** Decide again the rows at some tree path ends. */
	async #mark(ends, run, dirty) {
		const keys = ends.map((end) => ({ [this.#key]: end }));
		const rows = await run.view.keyed(this.node.table, keys);
		for (const [index, key] of keys.entries()) dirty.set(Key.name(this.node.table, key), rows[index]);
	}
};
/** Split a flat list into groups as long as some lists. */
function unflatten(flat, lists) {
	let offset = 0;
	return lists.map((list) => {
		const group = flat.slice(offset, offset + list.length);
		offset += list.length;
		return group;
	});
}
/** The partitions of a row no partition holds. */
var NOWHERE = /* @__PURE__ */ new Set();
/** One node's operators, with its open partitions and members. */
var Pipeline = class {
	/** The node the pipeline evaluates. */
	node;
	/** The source of the node's rows. */
	input;
	/** How the node decides its rows. */
	filter;
	/** The dataflow's shared context. */
	context;
	/** The pipeline of the parent node, absent for a root. */
	parent;
	/** The pipelines of the node's children, absent for stateless children. */
	children = [];
	/** The open partitions, by name. */
	partitions = /* @__PURE__ */ new Map();
	/** The rows the node holds, by key. */
	members = /* @__PURE__ */ new Map();
	/** Create the pipeline of a node. */
	constructor(node, context, hasTreeIndex) {
		this.node = node;
		this.context = context;
		this.input = Input.of(node, hasTreeIndex);
		this.filter = new Filter(node, context.arrangement);
	}
	/** Forget everything the pipeline holds. */
	forget() {
		this.partitions.clear();
		this.members.clear();
	}
	/** Open a root's one partition, starting after a row when given one. */
	openRoot(start) {
		this.partitions.set("", {
			...partitionOf(void 0, 1),
			...start === void 0 ? {} : { start }
		});
	}
	/** Count a held value's holders up or down, opening or closing its partition. */
	hold(value, step, run) {
		const name = this.node.partition(value);
		const partition = this.partitions.get(name);
		const work = run.workOf(this);
		if (step > 0 && partition === void 0) {
			this.partitions.set(name, partitionOf(value, 1));
			work.opened.set(name, value);
		} else if (step > 0) {
			partition.holders += 1;
			work.closed.delete(name);
		} else if (partition !== void 0 && --partition.holders === 0) {
			if (work.opened.delete(name)) this.partitions.delete(name);
			else work.closed.add(name);
		}
	}
	/** Decide rows again as of the run's position. */
	async decide(rows, run) {
		if (rows.size === 0) return;
		run.decided += rows.size;
		const present = [...rows.values()].filter((row) => row !== void 0);
		await this.filter.prepare(present, run);
		const candidates = present.filter((row) => this.filter.isCandidate(row, run));
		const located = await this.input.locate(candidates, run);
		const partitions = new Map(candidates.map((row, index) => [row, located[index]]));
		for (const [key, row] of rows) {
			const values = row === void 0 ? void 0 : partitions.get(row);
			const names = /* @__PURE__ */ new Set();
			for (const value of values ?? []) {
				const name = this.node.partition(value);
				if (this.partitions.has(name)) names.add(name);
			}
			this.admit(key, values === void 0 ? void 0 : this.filter.resolve(row, run), names, run);
		}
	}
	/** Move a row between partitions and return the previous member. */
	move(key, names) {
		const member = this.members.get(key);
		const before = member?.partitions ?? NOWHERE;
		for (const name of before) if (!names.has(name)) this.partitions.get(name)?.members.delete(key);
		for (const name of names) if (!before.has(name)) this.partitions.get(name).members.add(key);
		return member;
	}
	/** Read each partition's candidates with their computed values. */
	async matched(values, run) {
		const members = await this.input.members(values, run);
		await this.filter.prepare(members.flat(), run);
		return members.map((rows) => rows.filter((row) => this.filter.isCandidate(row, run)).map((row) => this.filter.resolve(row, run)));
	}
	/** Read up to a count of each segment's candidates in the node's order after its row. */
	async ordered(segments, count, run) {
		const node = this.node;
		const isMeasuredUpstream = this.context.upstream !== void 0 && node.relations.length > 0;
		if ((node.path === void 0 || node.path.kind === "key") && !isMeasuredUpstream) return (await run.view.ordered(node, segments, count, this.context.audience, run, node.relations.length === 0 ? void 0 : this.relationView(run))).map((rows) => rows.map((row) => this.filter.adopt(row)));
		return (await this.matched(segments.map((segment) => segment.value), run)).map((rows, index) => {
			const after = segments[index].after;
			return rows.filter((row) => after === void 0 || node.compare(row, after) > 0).sort((left, right) => node.compare(left, right)).slice(0, count);
		});
	}
	/** Follow the node's relations as of the run's position. */
	relationView(run) {
		const node = this.node;
		const arrangement = this.context.arrangement;
		return {
			decide: async (via, where, row) => {
				const relation = node.relation(via, where);
				await arrangement.prepare(node, [row], run);
				return (arrangement.measured(relation, relation.valueOf(row), run).count ?? 0) > 0;
			},
			touched: (after, upto) => run.view.dependents(node, after, upto),
			resolve: async (row) => {
				await arrangement.prepare(node, [row], run);
				return this.filter.related(row, run);
			}
		};
	}
};
/** Create a partition. */
function partitionOf(value, holders) {
	return {
		value,
		holders,
		members: /* @__PURE__ */ new Set(),
		window: void 0,
		sequence: -1
	};
}
/** Copy a set of partitions without one. */
function without(names, name) {
	const next = new Set(names);
	next.delete(name);
	return next;
}
/** Key rows of a node. */
function keyed(node, rows) {
	return new Map(rows.map((row) => [node.keyOf(row), row]));
}
/**
* The most entries one chunk of a window holds before it splits.
*
* An insert moves at most 512 pointers, well under a microsecond, and 100k entries span 200 chunks.
*/
var CHUNK_ENTRIES = 512;
/** One partition of a limited node with its first rows in order up to its limit. */
var Window = class {
	/** The complete row order, ending with the key. */
	#compare;
	/** The most rows the window holds. */
	#limit;
	/** The known rows in order, in chunks, the held ones first. */
	#chunks = [];
	/** The row each known key sorts as. */
	#rows = /* @__PURE__ */ new Map();
	/** The partition's join value, absent for a root. */
	value;
	/** Whether the window keeps every candidate. */
	isArranged;
	/** Whether no candidate follows the last known row. */
	#isExhaustive;
	/** Order the rows of a partition and hold the first up to a limit. */
	constructor(compare, limit, value, entries, isArranged) {
		this.#compare = compare;
		this.#limit = limit;
		this.value = value;
		this.isArranged = isArranged;
		this.#isExhaustive = isArranged || entries.length < limit;
		const sorted = [...entries].sort((left, right) => compare(left.row, right.row));
		for (let start = 0; start < sorted.length; start += CHUNK_ENTRIES) this.#chunks.push(sorted.slice(start, start + CHUNK_ENTRIES));
		for (const entry of sorted) this.#rows.set(entry.key, entry.row);
	}
	/** Whether the window holds a row. */
	holds(key) {
		const row = this.#rows.get(key);
		return row !== void 0 && this.#isHeldAt(...this.#locate(row));
	}
	/** The keys of the held rows, in order. */
	held() {
		const keys = [];
		for (const chunk of this.#chunks) for (const entry of chunk) {
			if (keys.length === this.#limit) return keys;
			keys.push(entry.key);
		}
		return keys;
	}
	/** The keys of the rows the window knows. */
	keys() {
		return this.#rows.keys();
	}
	/** The rows the window knows. */
	get size() {
		return this.#rows.size;
	}
	/** Whether the window holds its limit of rows. */
	get isFull() {
		return this.#rows.size >= this.#limit;
	}
	/** Whether no candidate follows the last known row. */
	get isExhaustive() {
		return this.#isExhaustive;
	}
	/** The last known row. */
	get last() {
		return this.#chunks.at(-1)?.at(-1)?.row;
	}
	/** Place a row in order, returning whether it is held and the key it evicted. */
	place(key, row) {
		const boundary = this.last;
		const wasHeld = this.remove(key);
		const isBeyond = boundary === void 0 ? !this.#isExhaustive : this.#compare(row, boundary) > 0;
		if (!this.isArranged && isBeyond && (!this.#isExhaustive || this.isFull)) {
			this.#isExhaustive &&= !this.isFull;
			return { isHeld: false };
		}
		const [chunk, index] = this.#locate(row);
		const isHeld = this.#isHeldAt(chunk, index);
		this.#insert(chunk, index, {
			key,
			row
		});
		if (!isHeld || wasHeld || this.#rows.size <= this.#limit) return { isHeld };
		const evicted = this.#at(this.#limit);
		if (!this.isArranged) {
			this.remove(evicted.key);
			this.#isExhaustive = false;
		}
		return {
			isHeld,
			evicted: evicted.key
		};
	}
	/** Append a refill's rows after the last row. */
	extend(entries, requested) {
		for (const entry of entries) if (!this.#rows.has(entry.key)) {
			const [chunk, index] = this.#locate(entry.row);
			this.#insert(chunk, index, entry);
		}
		this.#isExhaustive = entries.length < requested;
	}
	/** Take a row out, returning whether the window held it. */
	remove(key) {
		const row = this.#rows.get(key);
		if (row === void 0) return false;
		const [chunk, index] = this.#locate(row);
		const wasHeld = this.#isHeldAt(chunk, index);
		const entries = this.#chunks[chunk];
		entries.splice(index, 1);
		if (entries.length === 0) this.#chunks.splice(chunk, 1);
		this.#rows.delete(key);
		return wasHeld;
	}
	/** Find the chunk and index where a row sorts. */
	#locate(row) {
		let low = 0;
		let high = this.#chunks.length - 1;
		while (low < high) {
			const middle = low + high >>> 1;
			if (this.#compare(this.#chunks[middle].at(-1).row, row) < 0) low = middle + 1;
			else high = middle;
		}
		const entries = this.#chunks[low] ?? [];
		let start = 0;
		let end = entries.length;
		while (start < end) {
			const middle = start + end >>> 1;
			if (this.#compare(entries[middle].row, row) < 0) start = middle + 1;
			else end = middle;
		}
		return [low, start];
	}
	/** Insert an entry, splitting a full chunk. */
	#insert(chunk, index, entry) {
		this.#rows.set(entry.key, entry.row);
		const entries = this.#chunks[chunk];
		if (entries === void 0) {
			this.#chunks.push([entry]);
			return;
		}
		entries.splice(index, 0, entry);
		if (entries.length > CHUNK_ENTRIES) this.#chunks.splice(chunk + 1, 0, entries.splice(entries.length >>> 1));
	}
	/** Whether the entry at a chunk's index is among the held rows. */
	#isHeldAt(chunk, index) {
		let position = index;
		for (let before = 0; before < chunk && position < this.#limit; before += 1) position += this.#chunks[before].length;
		return position < this.#limit;
	}
	/** Read the entry at a position in the order. */
	#at(position) {
		let rest = position;
		for (const chunk of this.#chunks) {
			if (rest < chunk.length) return chunk[rest];
			rest -= chunk.length;
		}
	}
};
/** The most rows of one root read: about 1 MB at about 1 KB a row. */
var PAGE_ROWS = 1e3;
/** The rows a node holds per partition: every candidate, or a window of the first ones. */
var Selection = class extends Pipeline {
	/** The partitions arranging each candidate row, by key. */
	#arranged = /* @__PURE__ */ new Map();
	/** The rows the open partitions' windows know. */
	#windowed = 0;
	/** The windows that lost rows and wait for a refill. */
	#refills = /* @__PURE__ */ new Set();
	/** The tree path whose members hold chains of the rows between. */
	#chaining;
	/** The rows between each member and its partitions' held rows, by member and partition. */
	#chains = /* @__PURE__ */ new Map();
	/** How many chains hold each row, by key. */
	#chained = /* @__PURE__ */ new Map();
	/** The members whose chains the run moved. */
	#rechains = /* @__PURE__ */ new Map();
	/** Create the selection of a node. */
	constructor(node, context, hasTreeIndex) {
		super(node, context, hasTreeIndex);
		this.#chaining = this.input instanceof TreeJoin && (node.limit !== void 0 || node.where !== void 0) ? this.input : void 0;
	}
	/** The rows the windows know. */
	get size() {
		return this.#windowed;
	}
	/** Forget everything. */
	forget() {
		super.forget();
		this.#arranged.clear();
		this.#windowed = 0;
		this.#refills.clear();
		this.#chains.clear();
		this.#chained.clear();
		this.#rechains.clear();
	}
	/** Empty closed partitions, fill opened ones, and decide dirty rows again. */
	async step(work, run) {
		for (const name of work.closed) {
			const partition = this.partitions.get(name);
			if (partition === void 0 || partition.holders > 0) continue;
			for (const key of partition.members) this.#assign(key, void 0, without(this.members.get(key).partitions, name), run);
			for (const key of partition.window?.isArranged === true ? partition.window.keys() : []) this.#unarrange(key, name);
			this.#windowed -= partition.window?.size ?? 0;
			this.partitions.delete(name);
		}
		const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
		await this.#fill(opened, run);
		await this.decide(work.dirty, run);
		await this.settle(run);
	}
	/** Hold the next page of a root's rows after a row, returning the rows read. */
	async page(after, run) {
		if (after !== void 0) await this.filter.prepare([after], run);
		const start = after === void 0 ? void 0 : this.filter.resolve(after, run);
		const [rows] = await this.ordered([start === void 0 ? {} : { after: start }], PAGE_ROWS, run);
		await this.decide(keyed(this.node, rows), run);
		await this.settle(run);
		return rows;
	}
	/** Refill the windows and chain the moved members. */
	async settle(run) {
		await this.#refill(run);
		await this.#rechain(run);
	}
	/** Read the keys of a partition's held rows. */
	keys(name) {
		const partition = this.partitions.get(name);
		return partition?.window?.held() ?? [...partition?.members ?? []];
	}
	/** Fill opened partitions. */
	async #fill(opened, run) {
		const node = this.node;
		const limit = node.limit;
		if (opened.length === 0) return;
		const starts = opened.map(([name]) => this.partitions.get(name).start);
		await this.filter.prepare(starts.filter((start) => start !== void 0), run);
		for (const [index, start] of starts.entries()) starts[index] = start === void 0 ? void 0 : this.filter.resolve(start, run);
		const read = limit !== void 0 && !node.isArranged ? await this.ordered(opened.map(([, value], index) => {
			const start = starts[index];
			return start === void 0 ? { value } : {
				value,
				after: start
			};
		}), limit, run) : (await this.matched(opened.map(([, value]) => value), run)).map((rows, index) => {
			const start = starts[index];
			return start === void 0 ? rows : rows.filter((row) => node.compare(row, start) > 0);
		});
		for (const [index, [name, value]] of opened.entries()) {
			const rows = read[index];
			if (limit === void 0) {
				for (const row of rows) this.#join(node.keyOf(row), row, name, run);
				continue;
			}
			const byKey = keyed(node, rows);
			const window = new Window((left, right) => node.compare(left, right), limit, value, [...byKey].map(([key, row]) => ({
				key,
				row: node.isArranged ? node.orderOf(row) : row
			})), node.isArranged);
			this.partitions.get(name).window = window;
			this.#windowed += window.size;
			for (const key of node.isArranged ? byKey.keys() : []) this.#arrange(key, name);
			for (const key of window.held()) this.#join(key, byKey.get(key), name, run);
		}
	}
	/** Admit a decided row to its windows or partitions. */
	admit(key, row, names, run) {
		this.context.sink.touch(run, this.node.table, key, row);
		if (this.node.limit === void 0) this.#assign(key, row, names, run);
		else this.#place(key, row, names, run);
	}
	/** Place a row in its partitions' windows and take it out of the others. */
	#place(key, row, names, run) {
		const node = this.node;
		const held = new Set(this.members.get(key)?.partitions ?? NOWHERE);
		for (const name of /* @__PURE__ */ new Set([
			...held,
			...this.#arranged.get(key) ?? NOWHERE,
			...names
		])) {
			const window = this.partitions.get(name)?.window;
			if (window === void 0) continue;
			if (names.has(name)) {
				const wasHeld = window.holds(key);
				const size = window.size;
				const placed = window.place(key, window.isArranged ? node.orderOf(row) : row);
				this.#windowed += window.size - size;
				if (window.isArranged) this.#arrange(key, name);
				if (placed.isHeld) held.add(name);
				else held.delete(name);
				if (wasHeld && !placed.isHeld) this.#refills.add(name);
				const evicted = placed.evicted === void 0 ? void 0 : this.members.get(placed.evicted);
				if (evicted !== void 0) this.#assign(placed.evicted, void 0, without(evicted.partitions, name), run);
				else if (placed.evicted !== void 0 && !window.isArranged) throw new TypeError(`window of ${node.name} pushed out unheld row ${placed.evicted}`);
			} else {
				const size = window.size;
				const wasHeld = window.remove(key);
				this.#windowed += window.size - size;
				this.#unarrange(key, name);
				held.delete(name);
				if (wasHeld) this.#refills.add(name);
			}
		}
		this.#assign(key, row, held, run);
	}
	/** Hold a row in one more partition. */
	#join(key, row, name, run) {
		const partitions = this.members.get(key)?.partitions ?? NOWHERE;
		this.context.sink.touch(run, this.node.table, key, row);
		this.#assign(key, row, /* @__PURE__ */ new Set([...partitions, name]), run);
	}
	/** Set the partitions that hold a row and hold or let go of its held rows. */
	#assign(key, row, names, run) {
		const node = this.node;
		const sink = this.context.sink;
		const member = this.move(key, names);
		const joins = row === void 0 ? member?.joins ?? [] : node.children.map((child) => child.valueOf(row) ?? null);
		if (names.size === 0) {
			if (member !== void 0) {
				this.members.delete(key);
				sink.retain(run, node.table, key, row, -1);
				this.#holdChildren(member.joins, -1, run);
			}
		} else if (member === void 0) {
			this.members.set(key, {
				partitions: names,
				joins,
				...this.context.isMaterialized ? { row } : {}
			});
			sink.retain(run, node.table, key, row, 1);
			this.#holdChildren(joins, 1, run);
		} else {
			member.partitions = names;
			if (this.context.isMaterialized && row !== void 0) member.row = row;
			if (!sameValues(member.joins, joins)) {
				this.#holdChildren(member.joins, -1, run);
				this.#holdChildren(joins, 1, run);
				member.joins = joins;
			}
		}
		if (this.#chaining !== void 0) this.#rechains.set(key, row ?? this.#rechains.get(key));
	}
	/** Hold or let go of the children's partitions a member names. */
	#holdChildren(joins, step, run) {
		for (const [index, child] of this.children.entries()) {
			const value = joins[index];
			if (child !== void 0 && value !== null && value !== void 0) child.hold(value, step, run);
		}
	}
	/** Refill the windows that lost rows, all at once. */
	async #refill(run) {
		const names = [...this.#refills];
		this.#refills.clear();
		const promoted = [];
		const segments = [];
		for (const name of names) {
			const partition = this.partitions.get(name);
			const window = partition?.window;
			if (window?.isArranged === true) for (const key of window.held().filter((entry) => !partition.members.has(entry))) promoted.push([name, key]);
			else if (window !== void 0 && !window.isExhaustive && !window.isFull) segments.push([name, window]);
		}
		const rows = await this.#rowsOf(promoted.map(([, key]) => key), run);
		for (const [index, [name, key]] of promoted.entries()) this.#join(key, rows[index], name, run);
		const missing = segments.map(([, window]) => this.node.limit - window.size);
		const read = await Promise.all(segments.map(([, window], index) => this.ordered([window.last === void 0 ? { value: window.value } : {
			value: window.value,
			after: window.last
		}], missing[index], run)));
		for (const [index, [name, window]] of segments.entries()) {
			const rows = read[index][0];
			const size = window.size;
			window.extend(rows.map((row) => ({
				key: this.node.keyOf(row),
				row
			})), missing[index]);
			this.#windowed += window.size - size;
			for (const row of rows) this.#join(this.node.keyOf(row), row, name, run);
		}
	}
	/** Read arranged candidate rows by key with their computed values. */
	async #rowsOf(keys, run) {
		const node = this.node;
		const rows = (await run.view.keyed(node.table, keys.map((key) => Key.parse(node.table, key)))).map((row, index) => {
			if (row === void 0) throw new TypeError(`arrangement of ${node.name} keeps missing row ${keys[index]}`);
			return row;
		});
		await this.filter.prepare(rows, run);
		return rows.map((row, index) => {
			if (!this.filter.isCandidate(row, run)) throw new TypeError(`arrangement of ${node.name} keeps non-candidate row ${keys[index]}`);
			return this.filter.resolve(row, run);
		});
	}
	/** Note that a partition arranges a row. */
	#arrange(key, name) {
		let names = this.#arranged.get(key);
		if (names === void 0) {
			names = /* @__PURE__ */ new Set();
			this.#arranged.set(key, names);
		}
		names.add(name);
	}
	/** Note that a partition no longer arranges a row. */
	#unarrange(key, name) {
		const names = this.#arranged.get(key);
		names?.delete(name);
		if (names?.size === 0) this.#arranged.delete(key);
	}
	/** Chain the moved members again to the rows between them and their held rows. */
	async #rechain(run) {
		const moved = [...this.#rechains];
		this.#rechains.clear();
		if (moved.length === 0) return;
		const pairs = [];
		for (const [key, row] of moved) {
			const names = row === void 0 ? NOWHERE : this.members.get(key)?.partitions ?? NOWHERE;
			for (const name of names) pairs.push({
				key,
				name,
				member: row,
				value: this.partitions.get(name).value
			});
		}
		const between = await this.#chaining.between(pairs, run);
		const found = /* @__PURE__ */ new Map();
		for (const [index, pair] of pairs.entries()) {
			const chains = found.get(pair.key) ?? /* @__PURE__ */ new Map();
			found.set(pair.key, chains.set(pair.name, between[index]));
		}
		for (const [key, row] of moved) {
			const known = this.#chains.get(key);
			const next = /* @__PURE__ */ new Map();
			for (const name of this.members.get(key)?.partitions ?? NOWHERE) {
				const chain = row === void 0 ? known?.get(name) : found.get(key)?.get(name);
				if (chain !== void 0) next.set(name, chain);
			}
			for (const rows of next.values()) for (const between of rows) this.#chain(between, 1, run);
			for (const rows of known?.values() ?? []) for (const between of rows) this.#chain(between, -1, run);
			if (next.size === 0) this.#chains.delete(key);
			else this.#chains.set(key, next);
		}
	}
	/** Count a row's chains up or down and hold it while any chain does. */
	#chain(row, step, run) {
		const node = this.node;
		const key = node.keyOf(row);
		const count = (this.#chained.get(key) ?? 0) + step;
		if (count === 0) {
			this.#chained.delete(key);
			this.context.sink.retain(run, node.table, key, void 0, -1);
		} else {
			this.#chained.set(key, count);
			if (count === 1 && step > 0) this.context.sink.retain(run, node.table, key, row, 1);
		}
	}
};
/** Whether two lists hold equal values in order. */
function sameValues(left, right) {
	return left.length === right.length && left.every((value, index) => Order.missingFirst(value, right[index]) === 0);
}
/** One group's measures, kept current from the log sequence of its count. */
var Tally = class Tally {
	/** The aggregate node. */
	#node;
	/** The group's values in JSON form, the join value first for an include. */
	group;
	/** The measures by name. */
	#measures;
	/** The columns sums and averages add up. */
	#summed;
	/** The rows the group holds. */
	#count = 0;
	/** The sums of the added-up columns, by property. */
	#sums = /* @__PURE__ */ new Map();
	/** The extreme values by measure name, null while the group holds no value. */
	#extremes = /* @__PURE__ */ new Map();
	/** The log sequence the tally holds changes up to. */
	sequence;
	/** The cached measures. */
	#values;
	/** Start a tally from a count read at a log sequence. */
	constructor(node, group, count) {
		const measures = node.aggregate.values;
		this.#node = node;
		this.group = group;
		this.#measures = measures;
		this.#summed = summedColumns(measures);
		this.sequence = count.sequence;
		this.#count = count.rows;
		for (const [column, sum] of Object.entries(count.sums)) this.#sums.set(column, sum);
		for (const [name, measure] of Object.entries(measures)) if (measure.function === "min" || measure.function === "max") this.#extremes.set(name, count.extremes[name] ?? null);
	}
	/** Measure an aggregate node's groups among the rows a selection matches, at one log sequence. */
	static async measure(database, node, selection, visibility, admit) {
		const aggregate = node.aggregate;
		const grouping = node.grouping;
		const summed = summedColumns(aggregate.values);
		if (admit !== void 0 || summed.some((name) => node.kindOf(name) === "real")) return Tally.#tally(database, node, selection, visibility, admit);
		const extremes = Object.entries(aggregate.values).filter(([, measure]) => measure.function === "min" || measure.function === "max");
		const fields = {
			...Object.fromEntries(grouping.map((name, index) => [`g${index}`, node.sql(name, visibility)])),
			rows: count(),
			...Object.fromEntries(summed.flatMap((name, index) => [[`s${index}`, sum(node.sql(name, visibility))], [`c${index}`, count(node.sql(name, visibility))]])),
			...Object.fromEntries(extremes.map(([, measure], index) => [`e${index}`, extremeOf(node, measure, visibility)]))
		};
		const { sequence, rows } = await database.transaction(async (transaction) => ({
			sequence: (await transaction.log.position()).sequence,
			rows: await transaction.select(fields).from(node.table).where(selection).groupBy(...grouping.map((_, index) => sql.raw(String(index + 1))))
		}), {
			isolationLevel: "repeatable read",
			isReadOnly: true
		});
		return {
			sequence,
			tallies: rows.map((row) => new Tally(node, node.groupOf(Object.fromEntries(grouping.map((name, index) => [name, row[`g${index}`]]))), {
				sequence,
				rows: Number(row.rows),
				sums: Object.fromEntries(summed.map((name, index) => [name, {
					sum: node.kindOf(name) === "bigint" ? BigInt(row[`s${index}`] ?? 0) : [Number(row[`s${index}`] ?? 0)],
					count: Number(row[`c${index}`])
				}])),
				extremes: Object.fromEntries(extremes.map(([name], index) => [name, row[`e${index}`] ?? null]))
			}))
		};
	}
	/** Tally the matched and admitted rows by group in memory, at one log sequence. */
	static async #tally(database, node, selection, visibility, admit) {
		const measured = Object.values(node.aggregate.values).flatMap((measure) => measure.column === void 0 ? [] : [measure.column]);
		const names = new Set([...node.grouping, ...measured].flatMap((name) => node.columnsOf(name)));
		const fields = {
			...admit === void 0 ? {} : node.table[TABLE].logged,
			...Object.fromEntries([...names].map((name) => [name, node.columns[name]])),
			...Object.fromEntries(Object.keys(node.computed).map((name) => [name, valueOf(node, name, visibility)]))
		};
		const { sequence, rows } = await database.transaction(async (transaction) => ({
			sequence: (await transaction.log.position()).sequence,
			rows: await transaction.select(fields).from(node.table).where(selection)
		}), {
			isolationLevel: "repeatable read",
			isReadOnly: true
		});
		const admitted = admit === void 0 ? void 0 : await admit(rows);
		const tallies = /* @__PURE__ */ new Map();
		for (const [index, row] of rows.entries()) {
			if (admitted !== void 0 && !admitted.has(index)) continue;
			const group = node.groupOf(row);
			const key = JSON.stringify(group);
			const tally = tallies.get(key) ?? Tally.empty(node, group, sequence);
			tallies.set(key, tally);
			tally.count(row, 1);
		}
		return {
			sequence,
			tallies: [...tallies.values()]
		};
	}
	/** Start an empty tally at a log sequence. */
	static empty(node, group, sequence) {
		return new Tally(node, group, {
			sequence,
			rows: 0,
			sums: {},
			extremes: {}
		});
	}
	/** The rows the group holds. */
	get rows() {
		return this.#count;
	}
	/** Whether the group holds no rows. */
	get isEmpty() {
		return this.#count === 0;
	}
	/** Count a row in or out, returning false when counting out took an extreme. */
	count(row, sign) {
		this.#values = void 0;
		this.#count += sign;
		this.#add(row, sign);
		let isExact = true;
		for (const [name, measure] of Object.entries(this.#measures)) {
			const value = measure.column === void 0 ? null : row[measure.column] ?? null;
			if (value === null || measure.function !== "min" && measure.function !== "max") continue;
			const extreme = this.#extremes.get(name);
			const order = extreme === null ? void 0 : Order.values(value, extreme);
			if (sign < 0) isExact &&= order !== 0;
			else if (order === void 0 || (measure.function === "min" ? order < 0 : order > 0)) this.#extremes.set(name, value);
		}
		return isExact;
	}
	/** Read the measures in JSON form. */
	values() {
		this.#values ??= Object.fromEntries(Object.entries(this.#measures).map(([name, measure]) => [name, this.#value(name, measure)]));
		return this.#values;
	}
	/** Read each average's sum and count of present values. */
	parts() {
		return Object.fromEntries(Object.entries(this.#measures).filter(([, measure]) => measure.function === "avg").map(([name, measure]) => {
			const { sum, count } = this.#sums.get(measure.column) ?? {
				sum: [],
				count: 0
			};
			return [name, {
				sum: this.#node.json(measure.column, total(sum)),
				count
			}];
		}));
	}
	/** Read one measure in JSON form. */
	#value(name, measure) {
		if (measure.function === "count") return this.#count;
		if (measure.function === "sum" || measure.function === "avg") {
			const { sum, count } = this.#sums.get(measure.column) ?? {
				sum: [],
				count: 0
			};
			return count === 0 ? null : measure.function === "avg" ? Number(total(sum)) / count : this.#node.json(measure.column, total(sum));
		}
		const extreme = this.#extremes.get(name) ?? null;
		return extreme === null ? null : this.#node.json(measure.column, extreme);
	}
	/** Add or subtract a row's summed columns. */
	#add(row, sign) {
		for (const column of this.#summed) {
			const value = row[column];
			if (value === null || value === void 0) continue;
			const entry = this.#sums.get(column) ?? {
				sum: typeof value === "bigint" ? 0n : [],
				count: 0
			};
			const sum = typeof value === "bigint" ? entry.sum + BigInt(sign) * value : grow(entry.sum, sign * value);
			this.#sums.set(column, {
				sum,
				count: entry.count + sign
			});
		}
	}
};
/** Select a computed value, a number as a number. */
function valueOf(node, name, visibility) {
	const value = sql`${node.sql(name, visibility)}`;
	return node.kindOf(name) === "text" ? value.mapWith(String) : value.mapWith(Number);
}
/** Select a measure's extreme. */
function extremeOf(node, measure, visibility) {
	const extreme = (measure.function === "min" ? min : max)(node.sql(measure.column, visibility));
	return node.computed[measure.column] !== void 0 && node.kindOf(measure.column) !== "text" ? extreme.mapWith(Number) : extreme;
}
/** List the distinct summed columns. */
function summedColumns(measures) {
	return [...new Set(Object.values(measures).filter((measure) => measure.function === "sum" || measure.function === "avg").map((measure) => measure.column))];
}
/** Add a number to nonoverlapping partials exactly, as Shewchuk's expansion sum does. */
function grow(partials, value) {
	const next = [];
	let carried = value;
	for (const partial of partials) {
		const [larger, smaller] = Math.abs(carried) < Math.abs(partial) ? [partial, carried] : [carried, partial];
		const high = larger + smaller;
		const low = smaller - (high - larger);
		if (low !== 0) next.push(low);
		carried = high;
	}
	next.push(carried);
	return next;
}
/** Round the exact sum of partials to the nearest number. */
function total(sum) {
	if (typeof sum === "bigint") return sum;
	let index = sum.length - 1;
	let high = sum[index] ?? 0;
	let low = 0;
	while (index > 0) {
		index -= 1;
		const next = sum[index];
		const rounded = high + next;
		low = next - (rounded - high);
		high = rounded;
		if (low !== 0) break;
	}
	if (index > 0 && (low < 0 && sum[index - 1] < 0 || low > 0 && sum[index - 1] > 0)) {
		const doubled = low * 2;
		const rounded = high + doubled;
		if (doubled === rounded - high) high = rounded;
	}
	return high === 0 ? 0 : high;
}
/** The most partitions one grouped read counts, bounded by the parameter budget. */
var COUNTED_PARTITIONS = 500;
/** The aggregate groups of a node's rows per partition with their measures in tallies. */
var Aggregation = class extends Pipeline {
	/** The tallies, by group name. */
	tallies = /* @__PURE__ */ new Map();
	/** What each tracked member adds, by key. */
	contributions = /* @__PURE__ */ new Map();
	/** Whether the aggregate decides its rows one by one. */
	isTracked;
	/** The groups the run changed, by name, with their results before the change. */
	#changed = /* @__PURE__ */ new Map();
	/** The groups that lost an extreme in the run, by name. */
	#lost = /* @__PURE__ */ new Set();
	/** Create the aggregation of a node. */
	constructor(node, context, hasTreeIndex) {
		super(node, context, hasTreeIndex);
		this.isTracked = node.path !== void 0 && node.path.kind !== "key" || node.relations.length > 0 || node.isRelating;
	}
	/** The groups the tallies hold. */
	get size() {
		return this.tallies.size;
	}
	/** Forget every group. */
	forget() {
		super.forget();
		this.tallies.clear();
		this.contributions.clear();
		this.#changed.clear();
		this.#lost.clear();
	}
	/** Decide whether the subscriber is shown a group: every group of a query. */
	isShown(_group) {
		return true;
	}
	/** Empty closed partitions, count opened ones, and decide dirty rows again. */
	async step(work, run) {
		for (const name of work.closed) {
			const partition = this.partitions.get(name);
			if (partition === void 0 || partition.holders > 0) continue;
			for (const key of this.isTracked ? partition.members : []) this.#assign(key, void 0, without(this.members.get(key).partitions, name), run);
			if (!this.isTracked) this.#release(partition.value, run);
			this.partitions.delete(name);
		}
		const opened = [...work.opened].filter(([name]) => this.partitions.has(name));
		if (!this.isTracked) await this.#countAll(opened, run);
		else if (this.node.parent !== void 0) {
			const read = await this.matched(opened.map(([, value]) => value), run);
			for (const [index, [name]] of opened.entries()) for (const row of read[index]) {
				const key = this.node.keyOf(row);
				const partitions = this.members.get(key)?.partitions ?? NOWHERE;
				this.#assign(key, row, /* @__PURE__ */ new Set([...partitions, name]), run);
			}
		} else if (opened.length > 0) await this.pages(run, (rows) => this.decide(keyed(this.node, rows), run));
		await this.decide(work.dirty, run);
		await this.regroup(run);
	}
	/** Count a linear aggregate's changes and send the moved groups. */
	async tally(run) {
		await this.countImages(run, (value) => this.partitions.get(this.node.partition(value))?.sequence);
		await this.regroup(run);
	}
	/** Count each change of the node's table out before and in after. */
	async countImages(run, countedAt) {
		const node = this.node;
		const changes = run.changes.filter((change) => change.table === node.table);
		const affected = [...run.affected.get(node.table)?.values() ?? []];
		await this.filter.prepare([...changes.flatMap((change) => [change.before, change.after]), ...affected].filter((image) => image !== void 0), run);
		for (const change of changes) for (const [image, sign] of [[change.before, -1], [change.after, 1]]) {
			const value = image === void 0 ? void 0 : joinedOf(node, image);
			const sequence = image === void 0 ? void 0 : countedAt(value);
			if (image === void 0 || sequence === void 0 || change.sequence <= sequence || !this.filter.isCandidate(image, run)) continue;
			const resolved = this.filter.resolve(image, run);
			const group = node.groupOf(resolved, value);
			const name = JSON.stringify(group);
			if (!this.tallies.has(name) && sign < 0) throw new TypeError(`tally of ${node.name} lost group ${name}`);
			this.#note(name, group);
			const tally = this.#tally(name, group, sequence);
			if (change.sequence > tally.sequence && !tally.count(resolved, sign)) this.#lost.add(name);
		}
		for (const row of affected) {
			const value = joinedOf(node, row);
			if (countedAt(value) !== void 0) {
				const group = node.groupOf(this.filter.resolve(row, run), value);
				const name = JSON.stringify(group);
				this.#note(name, group);
				this.#lost.add(name);
			}
		}
	}
	/** Move a tracked member's contributions from its old groups to its new ones. */
	count(key, next, run) {
		for (const entry of this.contributions.get(key) ?? []) {
			this.#note(entry.name, entry.group);
			if (!this.tallies.get(entry.name).count(entry.row, -1)) this.#lost.add(entry.name);
		}
		for (const entry of next) {
			this.#note(entry.name, entry.group);
			this.#tally(entry.name, entry.group, run.view.position.sequence).count(entry.row, 1);
		}
		if (next.length === 0) this.contributions.delete(key);
		else this.contributions.set(key, next);
	}
	/** Send and return the groups with results the run changed. */
	async regroup(run) {
		const changed = [...this.#changed.values()];
		const lost = new Set(this.#lost);
		this.#changed.clear();
		this.#lost.clear();
		const moved = [];
		for (const { group, before } of changed) {
			const name = JSON.stringify(group);
			if (lost.has(name)) await this.#recount(group, run);
			const tally = this.tallies.get(name);
			if (tally?.isEmpty === true) this.tallies.delete(name);
			if (signatureOf(tally) !== before) {
				this.result(group, tally, run);
				moved.push(group);
			}
		}
		return moved;
	}
	/** Forget the groups a hydration changed. */
	settleHydration() {
		this.#changed.clear();
		this.#lost.clear();
	}
	/** Send a shown group's values while it holds rows, or its leaving. */
	result(group, tally, run) {
		this.context.sink.result(run, this.node, group, tally, this.isShown(group));
	}
	/** Read the node's selection a page at a time, in order. */
	async pages(run, each) {
		let after;
		for (let isDone = false; !isDone;) {
			const [rows] = await run.view.ordered(this.node, [after === void 0 ? {} : { after }], PAGE_ROWS, this.context.audience, run, this.node.relations.length === 0 ? void 0 : this.relationView(run));
			isDone = rows.length < PAGE_ROWS;
			after = rows.at(-1);
			await each(rows);
		}
	}
	/** Admit a tracked member to its partitions. */
	admit(key, row, names, run) {
		this.#assign(key, row, names, run);
	}
	/** Set a tracked member's partitions and count it into their groups. */
	#assign(key, row, names, run) {
		const node = this.node;
		const member = this.move(key, names);
		this.count(key, row === void 0 ? (this.contributions.get(key) ?? []).filter((entry) => names.has(entry.partition)) : [...names].map((partition) => contributionOf(node, row, partition, this.partitions.get(partition).value)), run);
		if (names.size === 0) this.members.delete(key);
		else if (member === void 0) this.members.set(key, {
			partitions: names,
			joins: []
		});
		else member.partitions = names;
	}
	/** Note a group's result before the run first changes it. */
	#note(name, group) {
		if (!this.#changed.has(name)) this.#changed.set(name, {
			group,
			before: signatureOf(this.tallies.get(name))
		});
	}
	/** Read a group's tally, starting an empty one at a sequence. */
	#tally(name, group, sequence) {
		let tally = this.tallies.get(name);
		if (tally === void 0) {
			tally = Tally.empty(this.node, group, sequence);
			this.tallies.set(name, tally);
		}
		return tally;
	}
	/** Count a linear aggregate's opened partitions, one grouped read per batch. */
	async #countAll(opened, run) {
		const node = this.node;
		const path = node.path;
		const sequence = run.view.position.sequence;
		const batches = [];
		if (path?.kind === "key") for (let start = 0; start < opened.length; start += COUNTED_PARTITIONS) {
			const batch = opened.slice(start, start + COUNTED_PARTITIONS);
			batches.push(Condition.oneOf(path.column, batch.map(([, value]) => node.json(path.column, value))));
		}
		else if (opened.length > 0) batches.push(Condition.all());
		for (const [name] of opened) this.partitions.get(name).sequence = sequence;
		for (const within of batches) for (const [name, tally] of await this.tallyAt(within, run)) {
			this.tallies.set(name, tally);
			this.result(tally.group, tally, run);
		}
	}
	/** Count one group again as of the run's position. */
	async #recount(group, run) {
		const node = this.node;
		const name = JSON.stringify(group);
		const counted = node.path === void 0 || node.path.kind === "key" ? await this.tallyAt(Condition.all(...Object.entries(group).filter(([column]) => Object.hasOwn(node.columns, column)).map(([column, entry]) => entry === null ? Condition.missing(column) : Condition.eq(column, entry))), run, group) : await this.tallyRows(run, group, { value: node.parent.fromJson(node.parentColumn, group[node.partitionName]) });
		this.tallies.set(name, counted.get(name) ?? Tally.empty(node, { ...group }, run.view.position.sequence));
	}
	/** Measure the groups of the rows a condition selects, as of the run's position. */
	async tallyAt(within, run, only) {
		const node = this.node;
		const read = await run.view.measure(node, within, this.context.audience);
		const tallies = new Map(read.tallies.filter((tally) => only === void 0 || sameGroup(tally.group, only)).map((tally) => [JSON.stringify(tally.group), tally]));
		const position = run.view.position.sequence;
		const beyond = await this.context.changesThrough([{
			table: node.table,
			scopes: node.scopes
		}], position, read.sequence);
		if (beyond === void 0) return this.tallyRows(run, only, { within });
		const match = Condition.compile(within, node.table);
		const images = beyond.flatMap((change) => [change.after, change.before].filter((image) => image !== void 0 && Condition.matches(match, image)));
		await this.filter.prepare(images, run);
		const lost = /* @__PURE__ */ new Set();
		for (const change of beyond.toReversed()) for (const [image, sign] of [[change.after, -1], [change.before, 1]]) {
			if (image === void 0 || !Condition.matches(match, image) || !this.filter.isCandidate(image, run)) continue;
			const resolved = this.filter.resolve(image, run);
			const group = node.groupOf(resolved, joinedOf(node, image));
			if (only !== void 0 && !sameGroup(group, only)) continue;
			const name = JSON.stringify(group);
			const tally = tallies.get(name) ?? Tally.empty(node, group, position);
			tallies.set(name, tally);
			if (!tally.count(resolved, sign)) lost.add(name);
		}
		for (const name of lost) {
			const group = tallies.get(name).group;
			const exact = await this.tallyRows(run, group, { within });
			tallies.set(name, exact.get(name) ?? Tally.empty(node, group, position));
		}
		for (const [name, tally] of tallies) {
			tally.sequence = position;
			if (tally.isEmpty) tallies.delete(name);
		}
		return tallies;
	}
	/** Tally rows by group as of the run's position. */
	async tallyRows(run, only, selected) {
		const node = this.node;
		let rows;
		if ("value" in selected) [rows] = await this.matched([selected.value], run);
		else {
			const read = await run.view.matching(node.table, Condition.all(node.condition(), selected.within));
			await this.filter.prepare(read, run);
			rows = read.filter((row) => this.filter.isCandidate(row, run)).map((row) => this.filter.resolve(row, run));
		}
		const tallies = /* @__PURE__ */ new Map();
		for (const row of rows) {
			const group = node.groupOf(row, "value" in selected ? selected.value : joinedOf(node, row));
			if (only !== void 0 && !sameGroup(group, only)) continue;
			const name = JSON.stringify(group);
			const tally = tallies.get(name) ?? Tally.empty(node, group, run.view.position.sequence);
			tally.count(row, 1);
			tallies.set(name, tally);
		}
		return tallies;
	}
	/** Let go of the groups of a partition no held row names. */
	#release(value, run) {
		const node = this.node;
		const partition = JSON.stringify(node.parent.json(node.parentColumn, value));
		for (const name of this.tallies.keys()) {
			const group = JSON.parse(name);
			if (JSON.stringify(group[node.partitionName]) === partition) {
				this.tallies.delete(name);
				this.result(group, void 0, run);
			}
		}
	}
};
/** The measures of a relation the node's condition and computed values read, by held value. */
var Relation = class extends Aggregation {
	/** The log sequence a linear relation's count read at. */
	#sequence = -1;
	/** Forget every group. */
	forget() {
		super.forget();
		this.#sequence = -1;
	}
	/** Decide whether the subscriber is shown a group: one a held row of the parent names. */
	isShown(group) {
		const node = this.node;
		return node.parent.isHolding && (this.partitions.get(JSON.stringify(group[node.partitionName]))?.holders ?? 0) > 0;
	}
	/** Measure every group as of a hydration's position. */
	async hydrate(run) {
		if (!this.isTracked) {
			for (const [name, tally] of await this.tallyAt(Condition.all(), run)) this.tallies.set(name, tally);
			this.#sequence = run.view.position.sequence;
			return;
		}
		await this.pages(run, (rows) => this.#measure(keyed(this.node, rows), run));
		this.settleHydration();
	}
	/** Apply a run's changes to every group, deciding the parent's naming rows again. */
	async step(work, run) {
		if (this.isTracked) {
			const dirty = new Map(work.dirty);
			work.dirty.clear();
			await this.#measure(dirty, run);
		} else await this.countImages(run, () => this.#sequence);
		const node = this.node;
		const values = (await this.regroup(run)).map((group) => group[node.partitionName]).filter((value) => value !== null && value !== void 0).map((value) => node.parent.fromJson(node.parentColumn, value));
		const parent = this.parent;
		const holders = await run.view.lookup(parent.node.table, values.map((value) => ({ [node.parentColumn]: value })));
		const dirty = run.workOf(parent).dirty;
		for (const row of holders.flat()) dirty.set(parent.node.keyOf(row), row);
	}
	/** Send the groups held rows newly name, and let go of the others. */
	show(work, run) {
		const node = this.node;
		for (const name of work.closed) {
			const partition = this.partitions.get(name);
			if (partition !== void 0 && partition.holders === 0) {
				this.partitions.delete(name);
				this.result(node.groupFor(partition.value), void 0, run);
			}
		}
		for (const [name, value] of work.opened) if (this.partitions.has(name)) {
			const group = node.groupFor(value);
			this.result(group, this.tallies.get(JSON.stringify(group)), run);
		}
	}
	/** Decide related rows again and count each into its groups. */
	async #measure(rows, run) {
		const node = this.node;
		const present = [...rows.values()].filter((row) => row !== void 0);
		await this.filter.prepare(present, run);
		const candidates = present.filter((row) => this.filter.isCandidate(row, run));
		const located = await this.input.locate(candidates, run);
		const values = new Map(candidates.map((row, index) => [row, located[index]]));
		for (const [key, row] of rows) {
			const named = row === void 0 ? void 0 : values.get(row);
			const resolved = named === void 0 ? void 0 : this.filter.resolve(row, run);
			this.count(key, (named ?? []).map((value) => contributionOf(node, resolved, node.partition(value), value)), run);
		}
	}
};
/** The aggregate groups of a node as a copy's source measured them, with the copy's predictions. */
var Mirror = class extends Pipeline {
	/** The shown groups by name, with their measures and canonical text. */
	#shown = /* @__PURE__ */ new Map();
	/** The groups shown. */
	get size() {
		return this.#shown.size;
	}
	/** Forget every group. */
	forget() {
		super.forget();
		this.#shown.clear();
	}
	/** Show the groups of opened partitions and let go of closed ones. */
	async step(work, run) {
		for (const name of work.closed) {
			const partition = this.partitions.get(name);
			if (partition !== void 0 && partition.holders === 0) this.partitions.delete(name);
		}
		await this.refresh(run);
	}
	/** Read the groups again and send the changed ones. */
	async refresh(run) {
		const node = this.node;
		const groups = /* @__PURE__ */ new Map();
		for (const group of await this.context.upstream.groups(node)) {
			const partition = node.path === void 0 ? "" : JSON.stringify(group.group[node.partitionName]);
			if (this.partitions.has(partition)) groups.set(canonicalize(group.group), group);
		}
		for (const [name, { group }] of this.#shown) if (!groups.has(name)) {
			this.#shown.delete(name);
			this.context.sink.result(run, node, group, void 0);
		}
		for (const [name, group] of groups) {
			const shown = this.#shown.get(name);
			if (shown === void 0 || canonicalize(shown.values) !== canonicalize(group.values)) {
				this.#shown.set(name, group);
				this.context.sink.result(run, node, group.group, resultOf(group));
			}
		}
	}
	/** List the shown groups. */
	groups() {
		return [...this.#shown.values()];
	}
	/** Admit no rows. */
	admit() {}
};
/** Read a source's group as a result. */
function resultOf(group) {
	return {
		rows: group.rows,
		isEmpty: group.rows === 0,
		values: () => group.values,
		parts: () => group.parts
	};
}
/** Whether two groups hold the same values. */
function sameGroup(left, right) {
	return canonicalize(left) === canonicalize(right);
}
/** Write a group's result as text, empty once the group is empty. */
function signatureOf(tally) {
	return tally === void 0 || tally.isEmpty ? "" : JSON.stringify([
		tally.rows,
		tally.values(),
		tally.parts()
	]);
}
/** Build what a row adds to its group in one partition. */
function contributionOf(node, row, partition, value) {
	const group = node.groupOf(row, value);
	return {
		partition,
		group,
		name: JSON.stringify(group),
		row
	};
}
/** Read the held value a row of a root or key-joined node joins. */
function joinedOf(node, row) {
	return node.path?.kind === "key" ? row[node.path.column] : void 0;
}
/** What the subscriber holds: each row by its holder count, and each shown group. */
var Sink = class {
	/** How many members and chains hold each row, by key. */
	#rows = /* @__PURE__ */ new Map();
	/** The aggregate groups the subscriber holds, by query and group. */
	#groups = /* @__PURE__ */ new Set();
	/** Decide whether the subscriber holds a row. */
	holds(key) {
		return this.#rows.has(key);
	}
	/** Note a row a run touched, and whether the subscriber held it before. */
	touch(run, table, key, row) {
		const touched = run.touched.get(key);
		if (touched === void 0) run.touched.set(key, {
			table,
			row,
			wasHeld: this.#rows.has(key)
		});
		else if (row !== void 0) touched.row = row;
	}
	/** Count a row's holders up or down. */
	retain(run, table, key, row, step) {
		this.touch(run, table, key, row);
		const count = (this.#rows.get(key) ?? 0) + step;
		if (count === 0) this.#rows.delete(key);
		else this.#rows.set(key, count);
	}
	/** Record a shown group's values while it holds rows, or its leaving. */
	result(run, node, group, tally, isShown = true) {
		const key = `${node.name}${JSON.stringify(group)}`;
		if (tally !== void 0 && !tally.isEmpty && isShown) {
			const isNew = !this.#groups.has(key);
			this.#groups.add(key);
			run.patch.result(node, group, tally, isNew);
		} else if (this.#groups.delete(key)) run.patch.result(node, group, void 0);
	}
	/** Send the rows a run moved into, out of or within the subscriber's set. */
	async emit(run) {
		const unread = /* @__PURE__ */ new Map();
		for (const entry of run.touched) {
			const [key, touched] = entry;
			if (touched.row === void 0 && this.#rows.has(key)) unread.set(touched.table, [...unread.get(touched.table) ?? [], entry]);
		}
		for (const [table, entries] of unread) {
			const rows = await run.view.keyed(table, entries.map(([key]) => Key.parse(table, key)));
			for (const [index, [key, touched]] of entries.entries()) {
				const row = rows[index];
				if (row === void 0) throw new TypeError(`subscriber holds ${key}, missing at ${run.view.position.sequence}`);
				touched.row = row;
			}
		}
		for (const [key, touched] of run.touched) {
			const isHeld = this.#rows.has(key);
			const table = touched.table;
			if (run.walk === "rebuild") continue;
			else if (isHeld && !touched.wasHeld) run.patch.rows.set(key, {
				table,
				row: touched.row,
				operation: "insert"
			});
			else if (!isHeld && touched.wasHeld) run.patch.rows.set(key, {
				table,
				row: Key.parse(table, key),
				operation: "delete"
			});
			else if (isHeld && run.changed.has(key)) run.patch.rows.set(key, {
				table,
				row: touched.row,
				operation: "update"
			});
		}
		run.touched.clear();
	}
	/** Forget everything the subscriber holds. */
	forget() {
		this.#rows.clear();
		this.#groups.clear();
	}
};
/** The row and group decisions of a page. */
var Patch = class Patch {
	/** The row decisions, by row key. */
	rows = /* @__PURE__ */ new Map();
	/** The group results, by query and group. */
	results = /* @__PURE__ */ new Map();
	/** The groups the page shows first. */
	#entered = /* @__PURE__ */ new Set();
	/** The decisions the patch holds. */
	get size() {
		return this.rows.size + this.results.size;
	}
	/** Record a group's new values, or its leaving. */
	result(node, group, tally, isNew = false) {
		const key = `${node.name}${JSON.stringify(group)}`;
		const isHeld = tally !== void 0 && !tally.isEmpty;
		if (!isHeld && this.#entered.delete(key)) {
			this.results.delete(key);
			return;
		} else if (isNew) this.#entered.add(key);
		const parts = isHeld ? tally.parts() : {};
		this.results.set(key, {
			query: node.name,
			group: { ...group },
			values: isHeld ? tally.values() : null,
			...isHeld ? { rows: tally.rows } : {},
			...Object.keys(parts).length === 0 ? {} : { parts }
		});
	}
	/** Forget the decisions a page sent. */
	drain() {
		this.rows.clear();
		this.results.clear();
		this.#entered.clear();
	}
	/** Record that the subscriber lets go of every group of a query. */
	clear(node) {
		this.results.set(node.name, {
			query: node.name,
			group: null,
			values: null
		});
	}
	/** Encode the patch into a page at a position without concealed columns. */
	async page(position, audience, cache, flags) {
		const byTable = /* @__PURE__ */ new Map();
		for (const [key, decision] of this.rows) {
			const decisions = byTable.get(decision.table) ?? [];
			decisions.push([key, decision]);
			byTable.set(decision.table, decisions);
		}
		const changes = [];
		for (const [table, decisions] of byTable) {
			const held = decisions.filter(([, decision]) => decision.operation !== "delete");
			const hidden = await audience.conceals(table, held.map(([, decision]) => decision.row), position);
			let index = 0;
			for (const [key, decision] of decisions) changes.push(decision.operation === "delete" ? {
				table: table[TABLE].sqlName,
				operation: "delete",
				row: encodeRow(table, keyOf$1(table, decision.row))
			} : encode$2(table, key, decision, hidden[index++], position, cache));
		}
		return {
			...flags,
			changes,
			...this.results.size === 0 ? {} : { results: [...this.results.values()] },
			position
		};
	}
	/** Build the patch from one collection to another at one position. */
	static difference(held, holding, nodes, isResent) {
		const patch = new Patch();
		for (const [key, decision] of holding.rows) if (!held.rows.has(key)) patch.rows.set(key, decision);
		else if (isResent(decision.table)) patch.rows.set(key, {
			...decision,
			operation: "update"
		});
		for (const [key, decision] of held.rows) if (!holding.rows.has(key)) patch.rows.set(key, {
			...decision,
			operation: "delete"
		});
		for (const node of nodes) if (node.aggregate !== void 0) patch.clear(node);
		for (const [key, result] of holding.results) patch.results.set(key, result);
		return patch;
	}
};
/** Encode a held row's logged columns at a position without concealed ones. */
function encode$2(table, key, decision, concealed, position, cache) {
	return cache.share(position.sequence, `encoded:${key}:${decision.operation}:${concealed.join(",")}`, () => ({
		table: table[TABLE].sqlName,
		operation: decision.operation,
		row: encodeColumns(Object.entries(table[TABLE].logged), decision.row, concealed),
		...concealed.length === 0 ? {} : { concealed: [...concealed] }
	}));
}
/** Read the values of a row's key. */
function keyOf$1(table, row) {
	return Object.fromEntries(table[TABLE].key.map((column) => [column, row[column]]));
}
/** One step of a dataflow: the database as of one position, its decisions, and the page it fills. */
var Run$1 = class {
	/** The database as of the position. */
	view;
	/** Who the dataflow serves. */
	#audience;
	/** How a hydration treats its rows, absent for a step. */
	walk;
	/** The committed changes the step applies, in commit order. */
	changes;
	/** The page the run fills. */
	patch = new Patch();
	/** The rows the audience decides again, by table and key. */
	affected = /* @__PURE__ */ new Map();
	/** The keys of rows whose readable columns or access the run changed. */
	changed = /* @__PURE__ */ new Set();
	/** What each pipeline has to do. */
	work = /* @__PURE__ */ new Map();
	/** The rows whose holding the run touched, by key. */
	touched = /* @__PURE__ */ new Map();
	/** The rows the run's pipelines decided. */
	decided = 0;
	/** The pending visibility decision of each row. */
	#deciding = /* @__PURE__ */ new WeakMap();
	/** Each row's decided visibility. */
	#visible = /* @__PURE__ */ new WeakMap();
	/** The rows each relation-reading filter resolved in this run. */
	#resolved = /* @__PURE__ */ new Map();
	/** Start a run as of a view. */
	constructor(view, audience, walk, changes = []) {
		this.view = view;
		this.#audience = audience;
		this.walk = walk;
		this.changes = changes;
	}
	/** Decide the visibility of many rows of a table at once. */
	async decide(table, rows) {
		const pending = rows.filter((row) => !this.#deciding.has(row));
		if (pending.length > 0) {
			const decided = this.#audience.admits(table, pending, this.view.position).then((held) => {
				for (const [index, row] of pending.entries()) this.#visible.set(row, held.has(index));
			});
			for (const row of pending) this.#deciding.set(row, decided);
		}
		await Promise.all(new Set(rows.map((row) => this.#deciding.get(row))));
	}
	/** Read a decided row's visibility. */
	isVisible(row) {
		const visible = this.#visible.get(row);
		if (visible === void 0) throw new TypeError("a row's visibility is read before it is decided");
		return visible;
	}
	/** Keep the visible rows of a table. */
	async seen(table, rows) {
		await this.decide(table, rows);
		return rows.filter((row) => this.isVisible(row));
	}
	/** Read a pipeline's work in this run. */
	workOf(pipeline) {
		let work = this.work.get(pipeline);
		if (work === void 0) {
			work = {
				opened: /* @__PURE__ */ new Map(),
				closed: /* @__PURE__ */ new Set(),
				dirty: /* @__PURE__ */ new Map()
			};
			this.work.set(pipeline, work);
		}
		return work;
	}
	/** Read the rows a relation-reading filter resolved in this run. */
	resolvedBy(filter) {
		let resolved = this.#resolved.get(filter);
		if (resolved === void 0) {
			resolved = /* @__PURE__ */ new WeakMap();
			this.#resolved.set(filter, resolved);
		}
		return resolved;
	}
};
/** The arrangement of a copy's relations as its source measured them, with the copy's predictions. */
var Trace = class {
	/** What the copy knows of its source. */
	#upstream;
	/** Each relation's measures by group name, read once per run. */
	#read = /* @__PURE__ */ new WeakMap();
	/** The same measures once read. */
	#held = /* @__PURE__ */ new WeakMap();
	/** The measures last seen for each held value, by relation and partition. */
	#seen = /* @__PURE__ */ new Map();
	/** Each relation's input. */
	#inputs = /* @__PURE__ */ new Map();
	/** Create the trace of a copy. */
	constructor(upstream) {
		this.#upstream = upstream;
	}
	/** Read the groups of every relation of a node once in the run. */
	async prepare(node, _rows, run) {
		await Promise.all(node.relations.map((relation) => this.#groups(relation, run)));
	}
	/** Look up a relation's measures of the rows naming a held value. */
	measured(relation, value, run) {
		const groups = this.#held.get(run)?.get(relation);
		if (groups === void 0) throw new TypeError(`measures of ${relation.name} are read before they are prepared`);
		return groups.get(canonicalize(relation.groupFor(value))) ?? {};
	}
	/** Decide again the holders with measures a run changed. */
	async relate(relations, parentOf, run) {
		for (const node of relations) {
			const parent = parentOf(node);
			if (parent === void 0) continue;
			let input = this.#inputs.get(node);
			if (input === void 0) {
				input = Input.of(node, false);
				this.#inputs.set(node, input);
			}
			const path = node.path;
			const values = /* @__PURE__ */ new Map();
			const name = (value) => {
				if (value !== null && value !== void 0) values.set(node.partition(value), value);
			};
			const images = run.changes.filter((change) => change.table === node.table).flatMap((change) => [change.before, change.after]).filter((image) => image !== void 0);
			for (const located of await input.locate(images, run)) located.forEach(name);
			for (const change of run.changes) {
				for (const image of [change.before, change.after]) if (image !== void 0 && path.kind === "junction" && change.table === path.table) name(image[path.from.column]);
				const moved = this.#upstream.groupOf(change);
				const value = moved?.query === node.name ? moved.group[node.partitionName] : void 0;
				if (value !== null && value !== void 0) name(node.parent.fromJson(node.parentColumn, value));
			}
			const seen = this.#seen.get(node) ?? /* @__PURE__ */ new Map();
			this.#seen.set(node, seen);
			await this.#groups(node, run);
			const moved = [];
			for (const [partition, value] of values) {
				const measured = JSON.stringify(this.measured(node, value, run));
				if (seen.get(partition) !== measured) {
					seen.set(partition, measured);
					moved.push(value);
				}
			}
			const column = node.parentColumn;
			const holders = await run.view.lookup(parent.node.table, moved.map((value) => ({ [column]: value })));
			const dirty = run.workOf(parent).dirty;
			for (const row of holders.flat()) dirty.set(parent.node.keyOf(row), row);
		}
	}
	/** Forget the measures seen. */
	forget() {
		this.#seen.clear();
	}
	/** Read a relation's groups once in the run. */
	#groups(relation, run) {
		let reads = this.#read.get(run);
		if (reads === void 0) {
			reads = /* @__PURE__ */ new Map();
			this.#read.set(run, reads);
		}
		const known = reads.get(relation);
		if (known !== void 0) return known;
		const read = this.#upstream.groups(relation).then((groups) => {
			const byName = new Map(groups.map((group) => [canonicalize(group.group), group.values]));
			let held = this.#held.get(run);
			if (held === void 0) {
				held = /* @__PURE__ */ new Map();
				this.#held.set(run, held);
			}
			held.set(relation, byName);
			return byName;
		});
		reads.set(relation, read);
		return read;
	}
};
/** Queries compiled to one pipeline per node, kept current as of a log position. */
var Dataflow = class {
	/** The root nodes of the queries. */
	roots;
	/** Every node of the queries, roots first. */
	nodes;
	/** What the subscriber holds. */
	sink = new Sink();
	/** The held log position, absent before the first hydration. */
	#position;
	/** What the pipelines share. */
	#context;
	/** Each node's pipeline, absent for stateless nodes. */
	#pipelines = /* @__PURE__ */ new Map();
	/** The row and aggregate pipelines, shallowest first. */
	#steps;
	/** The relation pipelines, deepest first. */
	#relations;
	/** The pipelines deciding rows one by one, by table. */
	#deciding = /* @__PURE__ */ new Map();
	/** The aggregations counting by images. */
	#linear;
	/** The aggregations a copy reads from its source. */
	#mirrors;
	/** The arrangement of a copy's relations. */
	#trace;
	/** The relations a copy follows through its source, deepest first. */
	#traced;
	/** The tables and scopes the queries read. */
	#reads;
	/** The canonical form of the queries. */
	#queries;
	/** The most rows and groups the pipelines may know. */
	#capacity;
	/** Report each run's cost. */
	#observe;
	/** The cost of the last run. */
	#last;
	/** The summed cost of every run, and the run count. */
	#total = {
		runs: 0,
		changes: 0,
		operations: 0,
		decided: 0,
		sent: 0,
		milliseconds: 0
	};
	/** Compile queries for an audience over a database or a copy. */
	constructor(queries, options) {
		const upstream = options.upstream;
		this.#queries = canonicalize(Object.entries(queries).map(([name, query]) => [name, describe$1(query)]));
		this.#capacity = options.capacity;
		this.#observe = options.observe;
		this.roots = Object.entries(queries).map(([name, query]) => new Node(name, query, query.scopes));
		this.nodes = this.roots.flatMap((root) => root.nodes());
		this.#trace = upstream === void 0 ? void 0 : new Trace(upstream);
		this.#context = {
			audience: options.audience,
			sink: this.sink,
			arrangement: this.#trace ?? this,
			upstream,
			database: options.database,
			isMaterialized: options.isMaterialized ?? false,
			changesThrough: options.changesThrough
		};
		this.#reads = this.nodes.flatMap((node) => [
			{
				table: node.table,
				scopes: node.scopes
			},
			...node.closure === void 0 ? [] : [{
				table: node.closure,
				scopes: node.scopes
			}],
			...node.path?.kind === "junction" ? [{
				table: node.path.table,
				scopes: node.scopes
			}] : []
		]);
		for (const watch of this.watches()) if (watch.table[TABLE].retention === "none") throw new DatabaseError("INVALID_QUERY", `watched table is not logged: ${watch.table[TABLE].name}`);
		for (const node of this.nodes) {
			const concealable = options.audience.concealable(node.table);
			const read = node.reads().filter((name) => concealable.includes(name));
			if (read.length > 0) throw new DatabaseError("INVALID_QUERY", `query ${node.name} reads concealable columns of ${node.table[TABLE].name}: ${read.join(", ")}`);
		}
		const hasTreeIndex = upstream === void 0;
		for (const node of this.nodes) {
			const pipeline = node.kind === "relation" ? upstream === void 0 ? new Relation(node, this.#context, hasTreeIndex) : void 0 : node.aggregate !== void 0 ? upstream === void 0 || upstream.isMeasured?.(node) === false ? new Aggregation(node, this.#context, hasTreeIndex) : new Mirror(node, this.#context, hasTreeIndex) : node.isHolding ? new Selection(node, this.#context, hasTreeIndex) : void 0;
			if (pipeline !== void 0) this.#pipelines.set(node, pipeline);
		}
		for (const [node, pipeline] of this.#pipelines) {
			pipeline.parent = node.parent === void 0 ? void 0 : this.#pipelines.get(node.parent);
			pipeline.children = node.children.map((child) => this.#pipelines.get(child));
			if (pipeline instanceof Selection || pipeline instanceof Aggregation && pipeline.isTracked) this.#deciding.set(node.table, [...this.#deciding.get(node.table) ?? [], pipeline]);
		}
		const pipelines = [...this.#pipelines.values()];
		this.#steps = pipelines.filter((pipeline) => !(pipeline instanceof Relation)).sort((left, right) => left.node.depth - right.node.depth);
		this.#relations = pipelines.filter((pipeline) => pipeline instanceof Relation).sort((left, right) => right.node.depth - left.node.depth);
		this.#linear = pipelines.filter((pipeline) => pipeline instanceof Aggregation && !(pipeline instanceof Relation) && !pipeline.isTracked);
		this.#mirrors = pipelines.filter((pipeline) => pipeline instanceof Mirror);
		this.#traced = this.nodes.filter((node) => upstream !== void 0 && node.kind === "relation").sort((left, right) => right.depth - left.depth);
	}
	/** The held log position, absent before the first hydration. */
	get position() {
		return this.#position;
	}
	/** Move to a later position of the same epoch without watched changes. */
	advance(position) {
		this.#position = position;
	}
	/** The name of the queries and the audience. */
	get key() {
		return `${this.#queries}\n${this.#context.audience.key}`;
	}
	/** List the tables and scopes the dataflow reads. */
	watches() {
		return [
			...this.#reads,
			...this.#context.audience.watches,
			...this.#context.upstream?.watches ?? []
		];
	}
	/** The rows and groups the pipelines know. */
	get size() {
		let size = 0;
		for (const pipeline of this.#pipelines.values()) size += pipeline.size;
		return size;
	}
	/** Forget everything and hold nothing as of a position. */
	forget(position) {
		this.#position = position;
		for (const pipeline of this.#pipelines.values()) pipeline.forget();
		this.sink.forget();
		this.#trace?.forget();
	}
	/** Hold every root's rows and results as of a run's position, yielding after each page. */
	async *hydrate(run, start) {
		const cost = this.#measure(run);
		for (const relation of this.#relations) await relation.hydrate(run);
		for (const root of this.roots) {
			const pipeline = this.#pipelines.get(root);
			pipeline.openRoot(start);
			if (root.aggregate !== void 0 || root.limit !== void 0) {
				run.workOf(pipeline).opened.set("", void 0);
				await this.#settle(run);
				await this.sink.emit(run);
				yield;
				continue;
			}
			const selection = pipeline;
			let after = start;
			for (let isDone = false; !isDone;) {
				const rows = await selection.page(after, run);
				isDone = rows.length < PAGE_ROWS;
				after = rows.at(-1);
				await this.#settle(run);
				await this.sink.emit(run);
				yield;
			}
		}
		this.#position = run.view.position;
		cost();
		this.#requireCapacity();
	}
	/** Hold every root's rows and results as of a view. */
	async fill(view, start) {
		this.forget(view.position);
		const run = new Run$1(view, this.#context.audience, "collect");
		for await (const _batch of this.hydrate(run, start));
	}
	/** Apply a run of committed changes, returning false when access changed too far to follow. */
	async step(run) {
		const cost = this.#measure(run);
		const transitions = /* @__PURE__ */ new Map();
		for (const change of run.changes) {
			const rows = transitions.get(change.table) ?? /* @__PURE__ */ new Map();
			transitions.set(change.table, rows);
			const key = Key.name(change.table, change.key);
			const known = rows.get(key);
			rows.set(key, {
				before: known === void 0 ? change.before : known.before,
				after: change.after
			});
		}
		for (const [table, rows] of transitions) for (const [key, { before, after }] of rows) {
			run.changed.add(key);
			this.#mark(run, table, key, after);
			await this.#moved(run, table, before, after, false);
		}
		const affected = /* @__PURE__ */ new Map();
		for (const change of run.changes) {
			const entries = await this.#context.audience.dependents(change);
			if (entries === "everything") return false;
			for (const entry of entries) affected.set(entry.table, [...affected.get(entry.table) ?? [], entry.key]);
		}
		for (const [table, keys] of affected) {
			const rows = await run.view.keyed(table, keys);
			const byKey = run.affected.get(table) ?? /* @__PURE__ */ new Map();
			run.affected.set(table, byKey);
			for (const [index, key] of keys.entries()) {
				const name = Key.name(table, key);
				const row = rows[index];
				if (row !== void 0) byKey.set(name, row);
				run.changed.add(name);
				this.#mark(run, table, name, row);
				await this.#moved(run, table, row, row, true);
			}
		}
		if (this.#trace !== void 0) await this.#trace.relate(this.#traced, (relation) => this.#pipelines.get(relation.parent), run);
		else for (const relation of this.#relations) await relation.step(run.workOf(relation), run);
		await this.#settle(run);
		for (const aggregation of this.#linear) await aggregation.tally(run);
		for (const mirror of this.#mirrors) {
			const upstream = this.#context.upstream;
			if (run.changes.some((change) => change.table === mirror.node.table || upstream.groupOf(change)?.query === mirror.node.name)) await mirror.refresh(run);
		}
		await this.sink.emit(run);
		this.#position = run.view.position;
		cost();
		this.#requireCapacity();
		return true;
	}
	/** Read a root's held result: its rows with their includes nested, or its groups. */
	async read(name) {
		if (!this.#context.isMaterialized) throw new TypeError("a dataflow reads its results only when it materializes its rows");
		const root = this.roots.find((entry) => entry.name === name);
		const pipeline = this.#pipelines.get(root);
		return pipeline instanceof Selection ? (await this.#nest(pipeline, [""])).get("") : groupsOf(pipeline, "").map(({ group, values }) => ({
			group,
			values
		}));
	}
	/** Look up a relation's measures of the rows naming a held value. */
	measured(relation, value) {
		const tally = this.#pipelines.get(relation).tallies.get(JSON.stringify(relation.groupFor(value)));
		return tally === void 0 || tally.isEmpty ? {} : tally.values();
	}
	/** Prepare nothing. */
	async prepare() {}
	/** Describe the pipelines and run costs. */
	inspect() {
		return {
			...this.#position === void 0 ? {} : { position: this.#position },
			size: this.size,
			...this.#capacity === void 0 ? {} : { capacity: this.#capacity },
			...this.#last === void 0 ? {} : { last: this.#last },
			total: this.#total,
			pipelines: [...this.#pipelines.values()].map((pipeline) => ({
				node: pipeline.node.name,
				kind: pipeline instanceof Selection ? "selection" : pipeline instanceof Relation ? "relation" : pipeline instanceof Aggregation ? "aggregation" : "mirror",
				path: pipeline.node.path?.kind ?? "root",
				partitions: pipeline.partitions.size,
				members: pipeline.members.size,
				size: pipeline.size
			}))
		};
	}
	/** Start measuring a run, returning the function that records its cost. */
	#measure(run) {
		const database = this.#context.database;
		const operations = database.driver.state.operations;
		const started = performance.now();
		return () => {
			const cost = {
				changes: run.changes.length,
				operations: database.driver.state.operations - operations,
				decided: run.decided,
				sent: run.patch.size,
				milliseconds: performance.now() - started
			};
			const total = this.#total;
			this.#last = cost;
			this.#total = {
				runs: total.runs + 1,
				changes: total.changes + cost.changes,
				operations: total.operations + cost.operations,
				decided: total.decided + cost.decided,
				sent: total.sent + cost.sent,
				milliseconds: total.milliseconds + cost.milliseconds
			};
			this.#observe?.(cost);
		};
	}
	/** Require the pipelines to stay within the capacity. */
	#requireCapacity() {
		const held = this.size;
		if (this.#capacity !== void 0 && held > this.#capacity) throw new SyncError("OVER_CAPACITY", `queries hold ${held} rows and groups, more than ${this.#capacity}`);
	}
	/** Step every pipeline with work, then show the relation groups. */
	async #settle(run) {
		for (const pipeline of this.#steps) {
			const work = run.work.get(pipeline);
			if (work !== void 0) {
				run.work.delete(pipeline);
				await pipeline.step(work, run);
			}
		}
		for (const relation of this.#relations) {
			const work = run.work.get(relation);
			if (work !== void 0) {
				run.work.delete(relation);
				relation.show(work, run);
			}
		}
	}
	/** Decide a row again at every pipeline of its table. */
	#mark(run, table, key, row) {
		for (const pipeline of this.#deciding.get(table) ?? []) run.workOf(pipeline).dirty.set(key, row);
	}
	/** Decide again the rows a changed row moves through each pipeline's path. */
	async #moved(run, table, before, after, isAccess) {
		for (const pipeline of [...this.#steps, ...this.#relations]) {
			const dirty = /* @__PURE__ */ new Map();
			await pipeline.input.moves(table, before, after, isAccess, run, dirty);
			for (const [key, row] of dirty) run.workOf(pipeline).dirty.set(key, row);
		}
	}
	/** Read a selection's held rows in order, with concealed columns left out and includes nested. */
	async #nest(pipeline, names) {
		const node = pipeline.node;
		const ordered = names.map((name) => pipeline.keys(name).map((key) => pipeline.members.get(key).row).sort((left, right) => node.compare(left, right)));
		const concealed = await this.#context.audience.conceals(node.table, ordered.flat(), this.#position);
		let offset = 0;
		const entries = ordered.map((list) => list.map((row) => {
			const hidden = concealed[offset++];
			return Object.fromEntries(Object.entries(row).filter(([column]) => !hidden.includes(column)));
		}));
		for (const [index, child] of node.children.entries()) {
			const included = pipeline.children[index];
			if (child.kind !== "include" || included === void 0) continue;
			const property = child.name.slice(node.name.length + 1);
			const partitions = ordered.map((list) => list.map((row) => child.valueOf(row)));
			const named = [...new Set(partitions.flat().filter(isPresent).map((value) => child.partition(value)))];
			const nested = included instanceof Selection ? await this.#nest(included, named) : void 0;
			const isOne = child.path?.kind === "key" && child.key.length === 1 && child.key[0] === child.path.column;
			for (const [position, list] of entries.entries()) for (const [index, entry] of list.entries()) {
				const value = partitions[position][index];
				const name = isPresent(value) ? child.partition(value) : void 0;
				const rows = name === void 0 ? [] : nested?.get(name) ?? [];
				entry[property] = nested === void 0 ? measuresOf(child, included, name) : isOne ? rows[0] ?? null : rows;
			}
		}
		return new Map(names.map((name, index) => [name, entries[index]]));
	}
};
/** Read an aggregate include's result for one held row's partition. */
function measuresOf(child, pipeline, name) {
	const listed = (name === void 0 ? [] : groupsOf(pipeline, name)).map(({ group: { [child.partitionName]: _joined, ...group }, values }) => ({
		group,
		values
	}));
	return child.aggregate.groupBy === void 0 || child.aggregate.groupBy.length === 0 ? listed[0]?.values ?? child.emptyValues() : listed;
}
/** Read an aggregate pipeline's groups in one partition. */
function groupsOf(pipeline, name) {
	const node = pipeline.node;
	return (pipeline instanceof Mirror ? pipeline.groups() : [...pipeline.tallies.values()].filter((tally) => !tally.isEmpty).map((tally) => ({
		group: tally.group,
		values: tally.values()
	}))).filter(({ group }) => node.path === void 0 || JSON.stringify(group[node.partitionName]) === name).sort((left, right) => canonicalize(left.group) < canonicalize(right.group) ? -1 : 1);
}
/** Whether a value is present. */
function isPresent(value) {
	return value !== null && value !== void 0;
}
/** Read the watched changes between two sequences from a database's log. */
function changesThroughLog(database) {
	return async (watches, after, through) => {
		const scopes = watchedScopes(watches);
		const changes = [];
		for (let sequence = after; sequence < through;) {
			let read;
			try {
				read = await database.log.read({
					tables: [...new Set(watches.map((entry) => entry.table))],
					after: sequence,
					...scopes === void 0 ? {} : { scopes }
				});
			} catch (error) {
				if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") return;
				throw error;
			}
			changes.push(...read.changes.filter((change) => change.sequence <= through));
			if (read.sequence <= sequence) break;
			sequence = read.sequence;
		}
		return changes;
	};
}
/** Describe a query or include in JSON. */
function describe$1(query) {
	return {
		...query,
		table: query.table[TABLE].sqlName,
		..."on" in query && query.on.kind === "junction" ? { on: {
			...query.on,
			table: query.on.table[TABLE].sqlName
		} } : {},
		include: Object.fromEntries(Object.entries("include" in query ? query.include ?? {} : {}).map(([name, include]) => [name, describe$1(include)])),
		relations: Object.fromEntries(Object.entries(query.relations ?? {}).map(([name, relation]) => [name, describe$1(relation)]))
	};
}
/** The most values one batched read names, bounded by the parameter budget. */
var BATCH_VALUES = 500;
/** The database as of a log position, with batched reads shared among its readers. */
var View$1 = class View$1 {
	/** The database read. */
	#database;
	/** The shared reads and images. */
	#cache;
	/** The database as of the position. */
	#snapshot;
	/** The position shown. */
	position;
	/** Show a database as of a position. */
	constructor(database, position, cache, snapshot) {
		this.#database = database;
		this.position = position;
		this.#cache = cache ?? new Memory(database);
		this.#snapshot = snapshot ?? database.log.at(position, (table, after, upto) => this.#cache.images(table, after, upto));
	}
	/** Show a database as its open transaction reads it now. */
	static async latest(database) {
		return new View$1(database, await database.log.reached(), void 0, Snapshot.live(database));
	}
	/** Read the rows of a table matching each match's values, aligned with the matches. */
	async lookup(table, matches) {
		if (matches.length === 0) return [];
		const sequence = this.position.sequence;
		const columns = Object.keys(matches[0]);
		const tuples = matches.map((match) => columns.every((column) => match[column] !== null && match[column] !== void 0) ? tupleOf(table, columns, match) : void 0);
		const keys = tuples.map((tuple) => tuple === void 0 ? void 0 : rowsKey(table, columns, tuple));
		const missing = /* @__PURE__ */ new Map();
		for (const [index, key] of keys.entries()) if (key !== void 0 && !this.#cache.isShared(sequence, key)) missing.set(key, tuples[index]);
		const entries = [...missing];
		const shared = /* @__PURE__ */ new Map();
		for (let start = 0; start < entries.length; start += BATCH_VALUES) {
			const batch = entries.slice(start, start + BATCH_VALUES);
			const grouped = this.#read(table, columns, batch.map(([, tuple]) => tuple)).then((rows) => {
				const byKey = new Map(batch.map(([key]) => [key, []]));
				for (const row of rows) byKey.get(rowsKey(table, columns, tupleOf(table, columns, row)))?.push(row);
				return byKey;
			});
			for (const [key] of batch) shared.set(key, this.#cache.share(sequence, key, async () => (await grouped).get(key)));
		}
		return Promise.all(keys.map((key, index) => key === void 0 ? Promise.resolve([]) : shared.get(key) ?? this.#cache.share(sequence, key, () => this.#read(table, columns, [tuples[index]]))));
	}
	/** Read rows of a table by key, aligned with the keys. */
	async keyed(table, keys) {
		const [column, ...rest] = table[TABLE].key;
		if (rest.length === 0) return (await this.lookup(table, keys.map((key) => ({ [column]: key[column] })))).map((rows) => rows[0]);
		return Promise.all(keys.map((key) => this.#cache.share(this.position.sequence, `row:${Key.name(table, key)}`, () => this.#snapshot.row(table, key))));
	}
	/** Read one row by its key. */
	async row(table, key) {
		return (await this.keyed(table, [key]))[0];
	}
	/** Read a table's rows matching a condition. */
	matching(table, where) {
		return this.#snapshot.rows(table, where);
	}
	/** Read the first visible rows of a node's partitions in order after each segment's row, up to a count each. */
	ordered(node, segments, count, audience, run, relations) {
		return Promise.all(segments.map((segment) => {
			const after = segment.after === void 0 ? "" : Key.name(node.table, segment.after);
			const key = `ordered:${node.selection}:${node.partition(segment.value)}:${after}:${count}:${audience.key}`;
			const current = audience.where(node.table);
			return this.#cache.share(this.position.sequence, key, () => this.#snapshot.ordered(node.table, {
				where: node.condition(segment.value),
				order: node.order,
				namespace: node.namespace((table) => admittedSQL(audience, table)),
				...segment.after === void 0 ? {} : { after: segment.after },
				count,
				admits: {
					...current === "memory" ? {} : { current },
					image: async (row) => (await run.seen(node.table, [row])).length > 0
				},
				...relations === void 0 ? {} : { relations }
			}));
		}));
	}
	/** Measure an aggregate node's visible groups among the rows a condition selects, in one grouped read. */
	measure(node, within, audience) {
		const admitted = (table) => admittedSQL(audience, table);
		const current = audience.where(node.table);
		const selection = Condition.render(Condition.all(node.condition(), within), node.bind(admitted));
		if (current === "memory") return Tally.measure(this.#database, node, selection, admitted, async (rows) => audience.admits(node.table, rows, this.position));
		return Tally.measure(this.#database, node, and(selection, current), admitted);
	}
	/** Read a node's rows before and after their related rows changed between two sequences. */
	async dependents(node, after, upto) {
		const rows = /* @__PURE__ */ new Map();
		for (const relation of node.relations) {
			const path = relation.path;
			const values = [];
			const changed = [...await this.#changed(relation.table, after, upto), ...await this.dependents(relation, after, upto)];
			if (path.kind === "key") values.push(...changed.map((row) => row[path.column]));
			else if (path.kind === "junction") {
				const targets = changed.map((row) => row[path.to.key]);
				const joins = await this.#current(path.table, path.to.column, targets);
				values.push(...joins.map((join) => join[path.from.column]));
				for (const join of await this.#changed(path.table, after, upto)) values.push(join[path.from.column]);
			}
			const column = relation.parentColumn;
			const present = values.filter((entry) => entry !== null && entry !== void 0);
			const read = await this.lookup(node.table, present.map((value) => ({ [column]: value })));
			for (const row of [...read.flat(), ...await this.#current(node.table, column, present)]) rows.set(node.keyOf(row), row);
		}
		return [...rows.values()];
	}
	/** Read a table's rows whose columns hold one of some tuples. */
	#read(table, columns, tuples) {
		const definitions = table[TABLE].columns;
		if (columns.some((column) => definitions[column].definition.kind !== "text")) return this.#snapshot.rows(table, columns.length === 1 ? Condition.oneOf(columns[0], tuples.map((tuple) => tuple[0])) : Condition.any(...tuples.map((tuple) => Condition.all(...columns.map((column, index) => Condition.eq(column, tuple[index]))))));
		return this.#snapshot.select(table, columns, tuples);
	}
	/** Read the rows of a table changed between two sequences, as they were and are. */
	async #changed(table, after, upto) {
		const images = await this.#cache.images(table, after, upto);
		const rows = [...images.values()].filter((image) => image !== null);
		const keys = [...images.keys()].map((name) => Key.parse(table, name));
		for (let start = 0; start < keys.length; start += 90) {
			const matches = Key.any(table, keys.slice(start, start + 90));
			const now = await this.#database.select().from(table).where(Condition.render(matches, Condition.bind(table)));
			rows.push(...now);
		}
		return rows;
	}
	/** Read a table's current rows whose column holds one of some values. */
	async #current(table, column, values) {
		const definition = table[TABLE].columns[column].definition;
		const present = [...new Set(values.filter((value) => value !== null && value !== void 0))].map((value) => definition.toJson(value));
		const rows = [];
		for (let start = 0; start < present.length; start += 90) {
			const matches = Condition.oneOf(column, present.slice(start, start + 90));
			const current = await this.#database.select().from(table).where(Condition.render(matches, Condition.bind(table)));
			rows.push(...current);
		}
		return rows;
	}
};
/** A view's own shared reads, with images from the log. */
var Memory = class {
	/** The images of the log's changes, read once. */
	#rewind;
	/** The reads made, by key. */
	#reads = /* @__PURE__ */ new Map();
	/** Create the memory of a database's views. */
	constructor(database) {
		this.#rewind = database.log.rewind();
	}
	/** Compute a keyed value once. */
	share(_sequence, key, compute) {
		if (!this.#reads.has(key)) this.#reads.set(key, compute());
		return this.#reads.get(key);
	}
	/** Decide whether a key was read. */
	isShared(_sequence, key) {
		return this.#reads.has(key);
	}
	/** Read images from the log. */
	images(table, after, upto) {
		return this.#rewind(table, after, upto);
	}
};
/** Write a row's values in some columns in JSON form. */
function tupleOf(table, columns, row) {
	const definitions = table[TABLE].columns;
	return columns.map((column) => definitions[column].definition.toJson(row[column]));
}
/** Name the shared read of a tuple. */
function rowsKey(table, columns, tuple) {
	return `rows:${table[TABLE].sqlName}.${columns.join(",")}=${JSON.stringify(tuple)}`;
}
/** Match the rows of a related table an audience admits in SQL. */
function admittedSQL(audience, table) {
	const where = audience.where(table);
	if (where === "memory") throw new DatabaseError("INVALID_QUERY", `relations read no rows of ${table[TABLE].name}, which the audience decides in memory`);
	return where;
}
var __destackModule$42 = Object.freeze({ "package": {
	"id": "package-01a0d8d9-b82d-7100-8445-a0a43b3cfeb7",
	"name": "@destack/sync",
	"version": "2026.9.0"
} });
/** The staged pages a run's completion reads at once. */
var STAGED_BATCH = 16;
/** The held keys a snapshot's completion reads at once. */
var PRUNE_BATCH = 1e3;
/** The copies a database holds, with their source and home positions. */
var replica = defineTable("replica", {
	/** The copy's name. */
	name: text$1("name").notNull(),
	/** The scope whose rows the copy holds. */
	scope: text$1("scope").notNull(),
	/** The source log's epoch, absent before the first complete snapshot. */
	epoch: text$1("epoch"),
	/** The source log sequence, absent before the first complete snapshot. */
	sequence: integer("sequence"),
	/** The home log's epoch the rows reflect, absent when the source is their home. */
	originEpoch: text$1("origin_epoch"),
	/** The home log sequence the rows reflect, absent when the source is their home. */
	originSequence: integer("origin_sequence"),
	/** The held queries in the follower's terms, absent for the whole scope. */
	queries: json$1("queries", json()),
	/** The scopes the copy reads, nearest first, absent until its source sends them. */
	scopes: json$1("scopes", array(string())),
	/** The shape of the copied tables' logged columns. */
	shape: text$1("shape"),
	/** The last time the home confirmed the rows current, in UTC epoch milliseconds. */
	confirmedAt: integer("confirmed_at").notNull()
}, {
	log: {},
	constraints: (copy) => [primaryKey({
		name: "replica_key",
		columns: [copy.name, copy.scope]
	})]
}, __destackModule$42);
/** The staged pages of a copy's run in progress. */
var replicaPage = defineTable("replica_page", {
	/** The copy's name. */
	name: text$1("name").notNull(),
	/** The copy's scope. */
	scope: text$1("scope").notNull(),
	/** The page's place in the run, from zero. */
	index: integer("index").notNull(),
	/** The page as the source sent it. */
	page: json$1("page", QueryPage).notNull()
}, { constraints: (staged) => [primaryKey({
	name: "replica_page_key",
	columns: [
		staged.name,
		staged.scope,
		staged.index
	]
})] }, __destackModule$42);
/** The aggregate groups a copy holds. */
var replicaResult = defineTable("replica_result", {
	/** The copy's name. */
	name: text$1("name").notNull(),
	/** The copy's scope. */
	scope: text$1("scope").notNull(),
	/** The aggregate query, by its path of names. */
	query: text$1("query").notNull(),
	/** The group's values as canonical JSON text. */
	group: text$1("group").notNull(),
	/** The group's measures by name. */
	values: json$1("values", record$1(string(), Scalar)).notNull(),
	/** The rows the group holds. */
	rows: integer("rows").notNull(),
	/** Each average's sum and count of present values, by measure name. */
	parts: json$1("parts", record$1(string(), strictObject({
		sum: Scalar,
		count: number().int()
	})))
}, {
	log: {},
	constraints: (result) => [primaryKey({
		name: "replica_result_key",
		columns: [
			result.name,
			result.scope,
			result.query,
			result.group
		]
	})]
}, __destackModule$42);
/**
* The rows of each copy's shape, as the follower stores them: the rows it includes and the columns it hides on each.
*
* A row several copies include stays until none includes it, and keeps a column while one of them shows it.
*/
var replicaRow = defineTable("replica_row", {
	/** The copy's name. */
	name: text$1("name").notNull(),
	/** The copy's scope. */
	scope: text$1("scope").notNull(),
	/** The row's table, by SQL name. */
	table: text$1("table").notNull(),
	/** The row's key. */
	key: text$1("key").notNull(),
	/** The columns the copy's shape hides on the row, by property. */
	concealed: json$1("concealed", array(string())).notNull()
}, { constraints: (row) => [primaryKey({
	name: "replica_row_key",
	columns: [
		row.name,
		row.scope,
		row.table,
		row.key
	]
}), index("replica_row_table").on(row.table, row.key)] }, __destackModule$42);
/** The tables of a database holding copies. */
var replicaTables = [
	replica,
	replicaPage,
	replicaResult,
	replicaRow
];
/** A local copy of one scope's rows in a remote database's tables. */
var Replica = class {
	/** The copy's name. */
	name;
	/** The scope whose rows the copy holds. */
	scope;
	/** The copied tables, parents first. */
	tables;
	/** The condition on each table's copied rows. */
	where;
	/** The scope of each table whose rows live outside the copy's scope. */
	scopes;
	/** The tables whose rows are copied from whichever scope they live in. */
	everywhere;
	/** The copied tables, keyed by `id`, whose rows are the scopes of each table's rows, for the tables copied across scopes. */
	within;
	/** The copied tables, by SQL name. */
	#copied;
	/** The shape of the copied tables' logged columns. */
	#shape;
	/** Define a copy of a scope's rows in some tables. */
	constructor(definition) {
		this.name = definition.name;
		this.scope = definition.scope;
		this.tables = definition.tables;
		this.where = definition.where ?? /* @__PURE__ */ new Map();
		this.scopes = definition.scopes ?? /* @__PURE__ */ new Map();
		this.everywhere = definition.everywhere ?? /* @__PURE__ */ new Set();
		this.within = definition.within ?? /* @__PURE__ */ new Map();
		this.#copied = new Map(this.tables.map((table) => [table[TABLE].sqlName, table]));
		this.#shape = Log.shape(this.tables);
	}
	/** The queries the copy holds, by table name, including the source's own copy record. */
	get queries() {
		const include = (parent) => {
			const children = this.tables.filter((table) => this.within.get(table)?.includes(parent));
			return children.length === 0 ? {} : { include: Object.fromEntries(children.map((table) => {
				const where = this.where.get(table);
				return [table[TABLE].sqlName, {
					table,
					on: {
						kind: "key",
						column: "scope",
						parent: "id"
					},
					...where === void 0 ? {} : { where },
					...include(table)
				}];
			})) };
		};
		return Object.fromEntries([...this.tables.filter((table) => !this.within.has(table)).map((table) => {
			const where = this.where.get(table);
			const scope = this.scopes.get(table) ?? this.scope;
			return [table[TABLE].sqlName, {
				table,
				scopes: this.everywhere.has(table) ? "every" : [scope],
				...where === void 0 ? {} : { where },
				...include(table)
			}];
		}), [replica[TABLE].sqlName, {
			table: replica,
			scopes: [this.scope],
			where: Condition.eq("name", this.name)
		}]]);
	}
	/** Match the rows of a table keyed by a text `id` that a named copy includes, as SQL. */
	static includes(name, table) {
		const { sqlName, columns } = table[TABLE];
		const key = sql`'["' || ${sqlName} || '","' || ${columns.id} || '"]'`;
		return sql`EXISTS (SELECT 1 FROM ${replicaRow} WHERE ${replicaRow.name} = ${name} AND ${replicaRow.table} = ${sqlName} AND ${replicaRow.key} = ${key})`;
	}
	/** List the keys of some rows of a table that a named copy includes. */
	static async keysIncluded(database, name, table, rows) {
		const included = await database.select({ key: replicaRow.key }).from(replicaRow).where(and(eq(replicaRow.name, name), eq(replicaRow.table, table[TABLE].sqlName), inArray(replicaRow.key, rows.map((row) => Key.name(table, row)))));
		return new Set(included.map((row) => row.key));
	}
	/** Report whether a database copies a scope. */
	static async isCopied(database, scope) {
		const [copy] = await database.select({ name: replica.name }).from(replica).where(eq(replica.scope, scope)).limit(1);
		return copy !== void 0;
	}
	/** Wait until every copy of a scope reflects its home up to a position, returning false once the signal aborts. */
	static async reach(database, scope, position, signal) {
		return database.log.until(async () => {
			const copies = (await database.select().from(replica).where(eq(replica.scope, scope))).map((record) => recordOrigin(record));
			if (copies.some((copy) => copy !== void 0 && copy.position.epoch > position.epoch)) throw new DatabaseError("STALE_EPOCH", `${scope} started a new epoch after the position`);
			return copies.length > 0 && copies.every((copy) => copy !== void 0 && copy.position.epoch === position.epoch && copy.position.sequence >= position.sequence);
		}, signal);
	}
	/** Refuse relaying a copy that holds no position yet. */
	static async requireRelayable(database, name, scope) {
		const [record] = await database.select().from(replica).where(and(eq(replica.name, name), eq(replica.scope, scope)));
		if (record !== void 0 && recordOrigin(record) === void 0) throw new SyncError("STALE", `copy of ${scope} holds no position yet`);
	}
	/** Read each complete copy's home position and confirmation time, by scope. */
	static async origins(database, name, scopes) {
		const records = await database.select().from(replica).where(and(eq(replica.name, name), inArray(replica.scope, scopes)));
		return new Map(records.flatMap((record) => {
			const origin = recordOrigin(record);
			return origin === void 0 ? [] : [[record.scope, origin]];
		}));
	}
	/** Read the source position the copy holds, absent before its first snapshot or after a shape change. */
	async position(database) {
		return this.#held(await this.#record(database), await this.#shape);
	}
	/** Read the copy's record, absent before it registered. */
	async #record(database) {
		const [record] = await database.select().from(replica).where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
		return record;
	}
	/** Read a record's source position, absent before its first snapshot or after a shape change. */
	#held(record, shape) {
		return record === void 0 || record.epoch === null || record.sequence === null || record.shape !== shape ? void 0 : {
			epoch: record.epoch,
			sequence: record.sequence
		};
	}
	/** Describe the copy's position, shape, holdings and staged pages. */
	async inspect(database) {
		const [row] = await database.select({
			epoch: replica.epoch,
			sequence: replica.sequence,
			originEpoch: replica.originEpoch,
			originSequence: replica.originSequence,
			queries: replica.queries,
			shape: replica.shape,
			confirmedAt: replica.confirmedAt
		}).from(replica).where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
		const [staged] = await database.select({ pages: count() }).from(replicaPage).where(this.#staged());
		const [results] = await database.select({ groups: count() }).from(replicaResult).where(and(eq(replicaResult.name, this.name), eq(replicaResult.scope, this.scope)));
		return {
			name: this.name,
			scope: this.scope,
			...row?.epoch === null || row?.sequence === null || row === void 0 ? {} : { position: {
				epoch: row.epoch,
				sequence: row.sequence
			} },
			...row?.originEpoch === null || row?.originSequence === null || row === void 0 ? {} : { origin: {
				epoch: row.originEpoch,
				sequence: row.originSequence
			} },
			isShaped: row?.shape === await this.#shape,
			queries: typeof row?.queries === "object" && row.queries !== null ? Object.keys(row.queries) : [],
			...row === void 0 ? {} : { confirmedAt: row.confirmedAt },
			staged: staged.pages,
			results: results.groups
		};
	}
	/** Read the queries the copy holds, in its follower's terms. */
	async holding(database) {
		const [row] = await database.select({ queries: replica.queries }).from(replica).where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
		return row?.queries ?? void 0;
	}
	/** Read the scope chain the copy reads, nearest first: its own scope alone until its source sends it. */
	async chain(database) {
		const [row] = await database.select({ scopes: replica.scopes }).from(replica).where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
		return row?.scopes ?? [this.scope];
	}
	/** Read a query's rows from the copy, predictions included. */
	async rows(database, name, query, outbox) {
		const dataflow = new Dataflow({ [name]: {
			...query,
			scopes: [this.scope]
		} }, {
			audience: EVERYONE,
			database,
			upstream: this.upstream(database, outbox),
			changesThrough: changesThroughLog(database),
			isMaterialized: true
		});
		await dataflow.fill(await View$1.latest(database));
		return dataflow.read(name);
	}
	/** Read an aggregate query's groups from the copy, predictions included. */
	async results(database, name, query, outbox) {
		const node = new Node(name, {
			...query,
			scopes: [this.scope]
		}, [this.scope]);
		return (await this.groupsOf(database, node, outbox)).map(({ group, values }) => ({
			group,
			values
		}));
	}
	/** Measure relations and aggregates through the source's groups, predictions included. */
	upstream(database, outbox) {
		return {
			watches: [{
				table: replicaResult,
				scopes: [this.scope]
			}],
			groups: (node) => this.groupsOf(database, node, outbox),
			groupOf: (change) => {
				const row = change.after ?? change.before;
				return change.table !== replicaResult || row === void 0 || row.name !== this.name || row.scope !== this.scope ? void 0 : {
					query: row.query,
					group: JSON.parse(row.group)
				};
			}
		};
	}
	/** Read a node's source groups with the uncounted predictions. */
	async groupsOf(database, node, outbox) {
		const rows = await database.select({
			group: replicaResult.group,
			values: replicaResult.values,
			rows: replicaResult.rows,
			parts: replicaResult.parts
		}).from(replicaResult).where(and(eq(replicaResult.name, this.name), eq(replicaResult.scope, this.scope), eq(replicaResult.query, node.name)));
		const groups = new Map(rows.map((row) => [row.group, {
			group: JSON.parse(row.group),
			values: { ...row.values },
			rows: row.rows,
			parts: { ...row.parts }
		}]));
		const table = node.table[TABLE].sqlName;
		const images = (outbox === void 0 ? [] : await outbox.predicted(database)).filter((change) => change.table === table).flatMap((change) => {
			const row = decodeRow(node.table, change.row);
			const before = change.operation === "insert" ? void 0 : change.operation === "update" ? decodeRow(node.table, change.before) : row;
			const after = change.operation === "delete" ? void 0 : row;
			return [...before === void 0 ? [] : [{
				image: before,
				sign: -1
			}], ...after === void 0 ? [] : [{
				image: after,
				sign: 1
			}]];
		});
		if (images.length > 0) {
			const run = new Run$1(await View$1.latest(database), EVERYONE, "collect");
			const filter = new Filter(node, new Trace(this.upstream(database, outbox)));
			await filter.prepare(images.map(({ image }) => image), run);
			for (const { image, sign } of images) if (filter.isCandidate(image, run)) predict(node, groups, filter.resolve(image, run), sign);
		}
		return [...groups.values()].filter((group) => group.rows > 0);
	}
	/** Record the queries the copy holds once a run holding them completed. */
	async hold(database, queries) {
		await database.update(replica).set({ queries: json().parse(queries) }).where(and(eq(replica.name, this.name), eq(replica.scope, this.scope)));
	}
	/** Keep the copied rows as the database's own. */
	async promote(database) {
		await database.transaction(async (transaction) => {
			if (await this.position(transaction) === void 0) throw new SyncError("STALE", `copy of ${this.scope} holds no position yet`);
			const own = (table) => and(eq(table.name, this.name), eq(table.scope, this.scope));
			await transaction.delete(replicaPage).where(own(replicaPage));
			await transaction.delete(replicaResult).where(own(replicaResult));
			await transaction.delete(replicaRow).where(own(replicaRow));
			await transaction.delete(replica).where(own(replica));
		});
	}
	/** Record the copy before its first snapshot. */
	async register(database) {
		await database.insert(replica).values({
			name: this.name,
			scope: this.scope,
			epoch: null,
			sequence: null,
			queries: null,
			confirmedAt: Date.now()
		}).onConflictDoNothing();
	}
	/** Apply one stream of pages, staging each run until it completes. */
	async *apply(database, pages, outbox, request) {
		await database.delete(replicaPage).where(this.#staged());
		let staged = 0;
		let isSnapshot = false;
		for await (const page of pages) {
			if (page.reset && staged > 0) {
				await database.delete(replicaPage).where(this.#staged());
				staged = 0;
			}
			if (staged === 0) isSnapshot = page.reset;
			if (page.complete) {
				await this.#complete(database, page, {
					staged,
					isSnapshot
				}, outbox, request);
				staged = 0;
			} else {
				await database.insert(replicaPage).values({
					name: this.name,
					scope: this.scope,
					index: staged,
					page
				});
				staged += 1;
			}
			yield page;
		}
	}
	/** Apply a source's pages until the signal aborts, resuming each stream that ends, from scratch after a changed request. */
	async follow(database, source, signal, options = {}) {
		await this.register(database);
		while (!signal.aborted) {
			const after = options.request === void 0 || canonicalize(await this.holding(database) ?? null) === canonicalize(options.request) ? await this.position(database) : void 0;
			let isReceived = false;
			const pages = source(after, signal);
			for await (const _page of this.apply(database, pages, options.outbox, options.request)) isReceived = true;
			if (!isReceived && !signal.aborted) throw new SyncError("INVALID_STREAM", `the source of ${this.scope} ended a stream without a page`);
		}
	}
	/** Apply a run's pages in one transaction, rebasing local predictions. */
	async #complete(database, page, run, outbox, request) {
		const { staged, isSnapshot } = run;
		await database.transaction(async (transaction) => {
			const rebased = outbox !== void 0 && (isSnapshot || staged > 0 || (page.results ?? []).length > 0 || (page.outcomes ?? []).length > 0 || page.changes.length > 0 && await outbox.reaches(transaction, new Set(page.changes.map((change) => change.table)))) ? outbox : void 0;
			const record = await this.#record(transaction);
			const held = this.#held(record, await this.#shape);
			if (!isSnapshot && held !== void 0 && held.epoch !== page.position.epoch) throw new DatabaseError("STALE_EPOCH", `page of epoch ${page.position.epoch} continues a copy of ${held.epoch}`);
			const outcomes = [];
			await transaction.log.copying(async () => {
				await rebased?.revert(transaction);
				if (isSnapshot) await transaction.delete(replicaResult).where(and(eq(replicaResult.name, this.name), eq(replicaResult.scope, this.scope)));
				const delivered = isSnapshot ? new Map(this.tables.map((table) => [table[TABLE].sqlName, /* @__PURE__ */ new Set()])) : void 0;
				let relayed;
				for (let start = 0; start < staged; start += STAGED_BATCH) {
					const batch = await transaction.select({ page: replicaPage.page }).from(replicaPage).where(and(this.#staged(), gte(replicaPage.index, start), lt(replicaPage.index, start + STAGED_BATCH))).orderBy(asc(replicaPage.index));
					for (const row of batch) {
						const written = await this.#write(transaction, row.page, delivered);
						outcomes.push(...written.outcomes);
						relayed = written.origin ?? relayed;
					}
				}
				const written = await this.#write(transaction, page, delivered);
				outcomes.push(...written.outcomes);
				relayed = written.origin ?? relayed;
				if (delivered !== void 0) for (const table of [...this.tables].reverse()) {
					const name = table[TABLE].sqlName;
					await this.#prune(transaction, table, delivered.get(name));
				}
				const isHome = relayed === void 0 && (isSnapshot || record?.originEpoch === null);
				const origin = relayed !== void 0 ? {
					originEpoch: relayed.position.epoch,
					originSequence: relayed.position.sequence,
					confirmedAt: relayed.confirmedAt
				} : isHome ? {
					originEpoch: null,
					originSequence: null,
					confirmedAt: Date.now()
				} : {};
				const advanced = {
					...page.position,
					...origin,
					...page.scopes === void 0 ? {} : { scopes: page.scopes },
					...request === void 0 ? {} : { queries: json().parse(request) },
					shape: await this.#shape
				};
				await transaction.insert(replica).values({
					name: this.name,
					scope: this.scope,
					confirmedAt: record?.confirmedAt ?? Date.now(),
					...advanced
				}).onConflictDoUpdate({
					target: [replica.name, replica.scope],
					set: advanced
				});
				await transaction.delete(replicaPage).where(this.#staged());
			});
			await rebased?.replay(transaction, outcomes, isSnapshot ? page.position : void 0);
		}, { constraints: "deferred" });
	}
	/** Write a page's changes in commit order, returning its outcomes and home position. */
	async #write(transaction, page, delivered) {
		const batches = /* @__PURE__ */ new Map();
		let origin;
		for (const change of page.changes) {
			if (change.table === replica[TABLE].sqlName) {
				origin = change.operation === "delete" ? void 0 : recordOrigin(decodeRow(replica, change.row));
				if (origin === void 0) throw new DatabaseError("STALE_EPOCH", `source stopped copying ${this.scope}`);
				continue;
			}
			const table = this.#copied.get(change.table);
			if (!table) throw new TypeError(`page names a table outside the replica: ${change.table}`);
			const row = decodeRow(table, change.row);
			delivered?.get(change.table).add(Key.name(table, row));
			const batch = batches.get(table) ?? {
				held: [],
				hidden: [],
				removed: []
			};
			batches.set(table, batch);
			if (change.operation === "delete") batch.removed.push(row);
			else {
				batch.held.push(row);
				batch.hidden.push(change.concealed ?? []);
			}
		}
		for (const [table, batch] of batches) {
			await this.#exclude(transaction, table, batch.removed.map((row) => Key.name(table, row)));
			await transaction.upsert(table, await this.#conceal(transaction, table, batch.held, batch.hidden));
			await this.#include(transaction, table, batch.held, batch.hidden);
		}
		for (const result of page.results ?? []) await this.#hold(transaction, result);
		return {
			outcomes: page.outcomes ?? [],
			...origin === void 0 ? {} : { origin }
		};
	}
	/** Hold or let go of a group, or of every group of its query. */
	async #hold(transaction, result) {
		const matched = and(eq(replicaResult.name, this.name), eq(replicaResult.scope, this.scope), eq(replicaResult.query, result.query), result.group === null ? void 0 : eq(replicaResult.group, canonicalize(result.group)));
		if (result.values === null) await transaction.delete(replicaResult).where(matched);
		else if (result.rows === void 0) throw new TypeError(`page holds a group of ${result.query} without its rows`);
		else {
			const values = { ...result.values };
			const rows = result.rows;
			const parts = result.parts === void 0 ? null : { ...result.parts };
			await transaction.insert(replicaResult).values({
				name: this.name,
				scope: this.scope,
				query: result.query,
				group: canonicalize(result.group),
				values,
				rows,
				parts
			}).onConflictDoUpdate({
				target: [
					replicaResult.name,
					replicaResult.scope,
					replicaResult.query,
					replicaResult.group
				],
				set: {
					values,
					rows,
					parts
				}
			});
		}
	}
	/** Match the pages the copy staged. */
	#staged() {
		return and(eq(replicaPage.name, this.name), eq(replicaPage.scope, this.scope));
	}
	/** Take the rows a completed snapshot left out of the copy, a batch at a time in key order. */
	async #prune(database, table, delivered) {
		if (this.within.has(table) || this.everywhere.has(table)) {
			const name = table[TABLE].sqlName;
			let last;
			do {
				const rows = await database.select({ key: replicaRow.key }).from(replicaRow).where(and(this.#included(), eq(replicaRow.table, name), last === void 0 ? void 0 : gt(replicaRow.key, last))).orderBy(asc(replicaRow.key)).limit(PRUNE_BATCH);
				last = rows.at(-1)?.key;
				const stale = rows.map((row) => row.key).filter((key) => !delivered.has(key));
				await this.#exclude(database, table, stale);
			} while (last !== void 0);
		} else {
			const columns = table[TABLE].columns;
			const key = table[TABLE].key;
			const order = Order.complete([], table);
			const fields = Object.fromEntries(key.map((name) => [name, columns[name]]));
			let last;
			do {
				const rows = await database.select(fields).from(table).where(and(Condition.render(Condition.all(Node.scoped([this.scopes.get(table) ?? this.scope]), this.where.get(table) ?? Condition.all()), Condition.bind(table)), last && Order.after(order, table, last))).orderBy(...Order.render(order, table)).limit(PRUNE_BATCH);
				last = rows.at(-1);
				const stale = rows.map((row) => Key.name(table, row)).filter((name) => !delivered.has(name));
				await this.#exclude(database, table, stale);
			} while (last !== void 0);
		}
	}
	/** Clear the columns the copy's shape hides on some rows, keeping a column another copy's shape shows. */
	async #conceal(database, table, rows, hidden) {
		const hiding = rows.filter((_, position) => hidden[position].length > 0);
		const others = await this.#others(database, table, hiding.map((row) => Key.name(table, row)));
		const key = table[TABLE].key;
		const shared = hiding.filter((row) => others.has(Key.name(table, row)));
		const stored = new Map((await Snapshot.live(database).select(table, key, shared.map((row) => key.map((name) => row[name])))).map((row) => [Key.name(table, row), row]));
		return rows.map((row, position) => {
			const name = Key.name(table, row);
			const shapes = others.get(name) ?? [];
			const values = hidden[position].map((column) => [column, shapes.some((shape) => !shape.includes(column)) ? stored.get(name)?.[column] ?? null : null]);
			return {
				...row,
				...Object.fromEntries(values)
			};
		});
	}
	/** Read the columns the other copies' shapes hide on some rows, by row key. */
	async #others(database, table, keys) {
		const name = table[TABLE].sqlName;
		const others = /* @__PURE__ */ new Map();
		for (let start = 0; start < keys.length; start += PRUNE_BATCH) {
			const found = await database.select({
				key: replicaRow.key,
				concealed: replicaRow.concealed
			}).from(replicaRow).where(and(not(this.#included()), eq(replicaRow.table, name), inArray(replicaRow.key, keys.slice(start, start + PRUNE_BATCH))));
			for (const row of found) others.set(row.key, [...others.get(row.key) ?? [], row.concealed]);
		}
		return others;
	}
	/** Record the rows the copy includes and the columns its shape hides on each. */
	async #include(database, table, rows, hidden) {
		const name = table[TABLE].sqlName;
		await database.upsert(replicaRow, rows.map((row, position) => ({
			name: this.name,
			scope: this.scope,
			table: name,
			key: Key.name(table, row),
			concealed: hidden[position]
		})));
	}
	/** Take some rows out of the copy, deleting those no copy includes any longer. */
	async #exclude(database, table, keys) {
		const name = table[TABLE].sqlName;
		for (let start = 0; start < keys.length; start += PRUNE_BATCH) {
			const batch = keys.slice(start, start + PRUNE_BATCH);
			const selected = and(this.#included(), eq(replicaRow.table, name), inArray(replicaRow.key, batch));
			const shown = await database.select({
				key: replicaRow.key,
				concealed: replicaRow.concealed
			}).from(replicaRow).where(selected);
			await database.delete(replicaRow).where(selected);
			const others = await this.#others(database, table, batch);
			await database.remove(table, batch.filter((key) => !others.has(key)).map((key) => Key.parse(table, key)));
			const cleared = shown.flatMap(({ key, concealed }) => {
				const shapes = others.get(key) ?? [];
				const columns = (shapes[0] ?? []).filter((column) => !concealed.includes(column) && shapes.every((shape) => shape.includes(column)));
				return columns.length === 0 ? [] : [{
					key: Key.parse(table, key),
					columns
				}];
			});
			const primary = table[TABLE].key;
			const stored = await Snapshot.live(database).select(table, primary, cleared.map(({ key }) => primary.map((column) => key[column])));
			await database.upsert(table, stored.map((row) => {
				const entry = cleared.find(({ key }) => Key.name(table, key) === Key.name(table, row));
				return {
					...row,
					...Object.fromEntries(entry.columns.map((column) => [column, null]))
				};
			}));
		}
	}
	/** Match the rows the copy includes. */
	#included() {
		return and(eq(replicaRow.name, this.name), eq(replicaRow.scope, this.scope));
	}
};
/** Read a record's home position. */
function recordOrigin(record) {
	if (record.epoch === null || record.sequence === null) return;
	return {
		position: record.originEpoch === null || record.originSequence === null ? {
			epoch: record.epoch,
			sequence: record.sequence
		} : {
			epoch: record.originEpoch,
			sequence: record.originSequence
		},
		confirmedAt: record.confirmedAt
	};
}
/** Add a predicted row image to its group, or take it away. */
function predict(node, groups, row, sign) {
	const group = node.groupOf(row);
	const key = canonicalize(group);
	const known = groups.get(key) ?? {
		group,
		values: node.emptyValues(),
		rows: 0,
		parts: {}
	};
	groups.set(key, known);
	known.rows += sign;
	for (const [name, measure] of Object.entries(node.aggregate.values)) {
		const current = known.values[name] ?? null;
		const value = measure.column === void 0 ? null : row[measure.column] ?? null;
		const json = value === null ? null : node.json(measure.column, value);
		if (measure.function === "count") known.values[name] = Number(current ?? 0) + sign;
		else if (measure.function === "sum" && json !== null) known.values[name] = add(current, json, sign);
		else if (measure.function === "avg" && json !== null) {
			const part = known.parts[name] ?? {
				sum: 0,
				count: 0
			};
			const next = {
				sum: add(part.sum, json, sign),
				count: part.count + sign
			};
			known.parts[name] = next;
			known.values[name] = next.count === 0 ? null : Number(next.sum) / next.count;
		} else if ((measure.function === "min" || measure.function === "max") && json !== null && sign === 1) {
			const order = current === null ? void 0 : Order.values(node.fromJson(measure.column, json), node.fromJson(measure.column, current));
			if (order === void 0 || (measure.function === "min" ? order < 0 : order > 0)) known.values[name] = json;
		}
	}
}
/** Add a JSON value to a sum, or take it away. */
function add(sum, value, sign) {
	return typeof sum === "string" || typeof value === "string" ? String(BigInt(sum ?? 0) + BigInt(sign) * BigInt(value ?? 0)) : Number(sum ?? 0) + sign * Number(value);
}
/** What a copy asks its source for: rows of one scope, decided for the scope the follower serves. */
var ReplicaRequest = defineSchema(strictObject({
	/** The copy's name, shared with a relaying source's own copy. */
	name: string().min(1),
	/** The copied scope. */
	scope: string().min(1),
	/** The scope the follower serves, whose principal the source decides for. */
	below: string().min(1),
	/** Whether the copy includes the scope's access rows. */
	access: boolean(),
	/** The object types whose access rows live in the follower's own database. */
	held: array(ObjectTypeReference),
	/** The object types the follower keeps copies of: the inherited rows of inherited types, and the scope's own row of scope types. */
	copied: array(ObjectTypeReference),
	/** The global rows the follower reads, by object type, decided for it where they live. */
	rows: array(strictObject({
		/** The object type. */
		type: ObjectTypeReference,
		/** The rows copied. */
		where: Condition.schema,
		/** The requested object types whose copied rows are the scopes of these rows, absent for rows of the copied scope. */
		within: array(ObjectTypeReference).min(1).optional()
	})),
	/** The log position the copy reached, absent before its first snapshot. */
	after: LogPosition.optional()
}));
var __destackModule$41 = Object.freeze({ "package": {
	"id": "package-01a0d8d9-b82d-7100-8445-a0a43b3cfeb7",
	"name": "@destack/sync",
	"version": "2026.9.0"
} });
/** A scope object's row: its parent, type, suspension and transfer fence, stored in the scope itself. */
var scopeTable = defineTable("scope", {
	/** The scope of the record and the scope object's identifier. */
	scope: text$1("scope").primaryKey().notNull(),
	/** The scope containing this one, the universe for users and organisations. */
	parent: text$1("parent").notNull(),
	/** The enclosing scopes nearest first, as far as this row's own database knew them when recording it. */
	ancestors: json$1("ancestors", array(string())).notNull().default(sql`'[]'`),
	/** The package declaring the scope object's type. */
	packageId: identifier$1("package_id", "package").notNull(),
	/** The scope object's type. */
	type: text$1("type").notNull(),
	/** When the scope was suspended, denying every access within it until resumed. */
	suspendedAt: integer("suspended_at"),
	/** When a transfer stopped the scope's writes. */
	fencedAt: integer("fenced_at"),
	/** The cell a transfer moves the scope to. */
	movedTo: text$1("moved_to")
}, {
	log: { retention: "history" },
	constraints: (scope) => [check("scope_fence", sql`(${scope.fencedAt} IS NULL) = (${scope.movedTo} IS NULL)`)]
}, __destackModule$41);
/** The package declaring sync's own type: the universe. */
var SYNC_PACKAGE = Object.freeze({ "package": {
	"id": "package-01a0d8d9-b82d-7100-8445-a0a43b3cfeb7",
	"name": "@destack/sync",
	"version": "2026.9.0"
} }).package;
/** The identifier of the universe and its scope. */
var UNIVERSE_ID = "universe";
/** The universe: the deployment, and the root scope every other scope is inside of. */
var universe$1 = {
	packageId: SYNC_PACKAGE.id,
	type: "universe",
	scope: UNIVERSE_ID,
	id: UNIVERSE_ID
};
/** The universe's link, the last of every chain. */
var UNIVERSE_LINK = {
	object: universe$1,
	parent: UNIVERSE_ID,
	isSuspended: false,
	movedTo: void 0
};
/** The prefix of every scope's fence lock key. */
var FENCE_LOCK = "destack-fence";
/** A scope's chain and transfer fence, read from and written to its own row. */
var Scope = {
	table: scopeTable,
	universe: universe$1,
	chain,
	object,
	fence,
	guard,
	unfence
};
/** Read a scope and the scopes enclosing it, nearest first. */
async function chain(snapshot, scope) {
	const rows = /* @__PURE__ */ new Map();
	for (let wanted = [scope]; wanted.length > 0;) {
		const read = await snapshot.select(scopeTable, ["scope"], wanted.map((id) => [id]));
		for (const row of read) rows.set(row.scope, row);
		wanted = [...new Set(read.flatMap((row) => row.ancestors))].filter((id) => !rows.has(id) && !wanted.includes(id));
	}
	const links = [];
	for (let current = rows.get(scope); current !== void 0;) {
		if (links.some((link) => link.object.id === current.scope)) throw new SyncError("INVALID_SCOPE", `cyclic scope: ${current.scope}`);
		links.push({
			object: {
				packageId: current.packageId,
				type: current.type,
				scope: current.parent,
				id: current.scope
			},
			parent: current.parent,
			isSuspended: current.suspendedAt !== null,
			movedTo: current.movedTo ?? void 0
		});
		current = current.parent === UNIVERSE_ID ? void 0 : rows.get(current.parent);
	}
	return scope === UNIVERSE_ID || links.at(-1)?.parent === UNIVERSE_ID ? [...links, UNIVERSE_LINK] : links;
}
/** Read a scope's own object and refuse an unknown scope. */
async function object(snapshot, id) {
	const [link] = await chain(snapshot, id);
	if (link === void 0) throw new SyncError("NOT_FOUND", `unknown scope: ${id}`);
	return link.object;
}
/** Send a scope's writes to another cell once the writes guarding it commit; the scopes below it follow. */
async function fence(database, scope, cell, now) {
	await database.transaction(async (transaction) => {
		await lock(transaction, scope, "exclusive");
		const [fenced] = await transaction.update(scopeTable).set({
			fencedAt: now,
			movedTo: cell
		}).where(eq(scopeTable.scope, scope)).returning({ scope: scopeTable.scope });
		if (fenced === void 0) throw new SyncError("NOT_FOUND", `unknown scope: ${scope}`);
	});
}
/** Keep a write's scopes unfenced until it commits. */
async function guard(database, scopes) {
	for (const scope of scopes.filter((id) => id !== UNIVERSE_ID)) await lock(database, scope, "shared");
}
/** Take a scope's fence lock for the transaction; SQLite serializes writes already. */
async function lock(database, scope, mode) {
	if (database.dialect === "postgresql") {
		const key = sql`hashtextextended(${`${FENCE_LOCK}:${scope}`}, 0)`;
		await database.execute(mode === "shared" ? sql`SELECT pg_advisory_xact_lock_shared(${key})` : sql`SELECT pg_advisory_xact_lock(${key})`);
	}
}
/** Lift a scope's fence on the database now holding it. */
async function unfence(database, scope) {
	await database.update(scopeTable).set({
		fencedAt: null,
		movedTo: null
	}).where(eq(scopeTable.scope, scope));
}
var __destackModule$39 = Object.freeze({ "package": {
	"id": "package-01a0d8d9-b82d-7100-8445-a0a43b3cfeb7",
	"name": "@destack/sync",
	"version": "2026.9.0"
} });
/** One method call within a mutation. */
var Call$1 = defineSchema(strictObject({
	/** The object type and method, such as page.create. */
	method: string().min(1),
	/** The method's input, with its scope and target. */
	input: record$1(string(), json()),
	/** The release of the object type's package the call was made against. */
	release: Version
}));
/** Calls a client makes atomically. */
var Mutation = defineSchema(strictObject({
	/** The request identifier. */
	id: uuidv7(),
	/** The calls in order. */
	calls: array(Call$1).min(1)
}));
defineTable("mutation", {
	/** The request identifier. */
	id: text$1("id").primaryKey().validate(uuidv7()),
	/** The party that queued the mutation, such as a browser tab. */
	origin: text$1("origin").notNull(),
	/** The mutation's place in the outbox. */
	position: integer("position").notNull().unique(),
	/** The calls. */
	calls: json$1("calls", array(Call$1)).notNull(),
	/** The mutation's predicted changes, as JSON text. */
	changes: text$1("changes").notNull(),
	/** The tables its prediction reads or writes, as a JSON array of SQL names. */
	reach: text$1("reach").notNull(),
	/** The server log's epoch of the executed mutation. */
	epoch: text$1("epoch"),
	/** The server log sequence of the mutation's changes. */
	sequence: integer("sequence"),
	/** The failure the server recorded. */
	error: json$1("error", json()),
	/** The branch holding the mutation, absent on the main line. */
	branch: text$1("branch")
}, void 0, __destackModule$39);
defineTable("checkout", {
	/** The one row's key. */
	slot: integer("slot").primaryKey(),
	/** The checked-out branch. */
	branch: text$1("branch").notNull()
}, void 0, __destackModule$39);
/** The package declaring access's own types. */
var OWNER = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739e-a21a-66fd749e17ca",
	"name": "@destack/access",
	"version": "2026.9.0"
} }).package;
OWNER.id;
/** The kinds of principal that authenticate: people, machines and installed software. */
var principal = {
	/** A person, identified globally, and the scope holding their own objects. */
	user: new Policy(OWNER, {
		name: "user",
		permissions: {},
		scope: true,
		isGlobal: true
	}),
	/** A machine running Destack, living in its account, and the scope of its local operations. */
	host: new Policy(OWNER, {
		name: "host",
		permissions: {},
		scope: true
	}),
	/** A region of Destack's hosted platform, administering the spaces placed in it. */
	region: new Policy(OWNER, {
		name: "region",
		permissions: {},
		isGlobal: true
	}),
	/** An application installed into an account or space: the principal of software. */
	installation: new Policy(OWNER, {
		name: "installation",
		permissions: {}
	}),
	/** A space, the principal its cell copies rows for. */
	space: new Policy(OWNER, {
		name: "space",
		permissions: {},
		isGlobal: true
	}),
	/** A host or region serving zones, as the directory knows it. */
	cell: new Policy(OWNER, {
		name: "cell",
		permissions: {},
		isGlobal: true
	}),
	/** A non-person identity an account creates for automation, living in that account. */
	serviceAccount: new Policy(OWNER, {
		name: "service-account",
		permissions: {}
	})
};
new Policy(SYNC_PACKAGE, {
	name: "universe",
	permissions: {},
	scope: true
});
/** Every caller, signed in or anonymous, related through its wildcard; links condition such grants on a capability. */
var anyone = new Policy(OWNER, {
	name: "anyone",
	permissions: {}
});
/** A named set of permissions, including the permissions of the roles it includes. */
var role = new Policy(OWNER, {
	name: "role",
	relations: { includes: { subjects: ["role"] } },
	permissions: {
		create: none(),
		read: none(),
		update: none(),
		delete: none(),
		share: none()
	}
});
new Policy(OWNER, {
	name: "relationship",
	relations: { subject: { subjects: [
		principal.user,
		principal.host,
		principal.installation
	] } },
	permissions: { read: union$1(relation("subject"), grants("object")) }
});
new Policy(OWNER, {
	name: "proposal",
	relations: {
		proposer: { subjects: [
			principal.user,
			principal.host,
			principal.installation
		] },
		addressee: { subjects: [
			principal.user,
			principal.host,
			principal.installation
		] },
		lender: { subjects: [
			principal.user,
			principal.host,
			principal.installation
		] }
	},
	permissions: { read: union$1(relation("proposer"), relation("addressee"), relation("lender"), grants("object")) }
});
principal.user, principal.host, principal.region, principal.installation;
/** Determine whether a subject is one principal rather than a set or another object. */
function isPrincipal(subject) {
	return Object.values(principal).some((kind) => kind.is(subject));
}
/** Read the principal acting in a request. */
function principalOf(context) {
	return (context.delegates?.findLast((delegate) => delegate.authority === "lent"))?.subject ?? context.subject ?? context.subjects.find(isPrincipal);
}
/** Require the principal acting in a request. */
function requirePrincipal(context) {
	const acting = principalOf(context);
	if (acting === void 0) throw new AccessError("FORBIDDEN", "the request needs an authenticated principal");
	return acting;
}
init_esm();
var SUPPRESS_TRACING_KEY = createContextKey("OpenTelemetry SDK Context Key SUPPRESS_TRACING");
function suppressTracing(context) {
	return context.setValue(SUPPRESS_TRACING_KEY, true);
}
function isTracingSuppressed(context) {
	return context.getValue(SUPPRESS_TRACING_KEY) === true;
}
var BAGGAGE_HEADER = "baggage";
var BAGGAGE_MAX_PER_NAME_VALUE_PAIRS = 4096;
init_esm();
function serializeKeyPairs(keyPairs) {
	return keyPairs.reduce((hValue, current) => {
		const value = `${hValue}${hValue !== "" ? "," : ""}${current}`;
		return value.length > 8192 ? hValue : value;
	}, "");
}
function getKeyPairs(baggage) {
	return baggage.getAllEntries().map(([key, value]) => {
		let entry = `${encodeURIComponent(key)}=${encodeURIComponent(value.value)}`;
		if (value.metadata !== void 0) entry += ";" + value.metadata.toString();
		return entry;
	});
}
function parsePairKeyValue(entry) {
	if (!entry) return;
	const metadataSeparatorIndex = entry.indexOf(";");
	const keyPairPart = metadataSeparatorIndex === -1 ? entry : entry.substring(0, metadataSeparatorIndex);
	const separatorIndex = keyPairPart.indexOf("=");
	if (separatorIndex <= 0) return;
	const rawKey = keyPairPart.substring(0, separatorIndex).trim();
	const rawValue = keyPairPart.substring(separatorIndex + 1).trim();
	if (!rawKey || !rawValue) return;
	let key;
	let value;
	try {
		key = decodeURIComponent(rawKey);
		value = decodeURIComponent(rawValue);
	} catch {
		return;
	}
	let metadata;
	if (metadataSeparatorIndex !== -1 && metadataSeparatorIndex < entry.length - 1) {
		const metadataString = entry.substring(metadataSeparatorIndex + 1);
		metadata = baggageEntryMetadataFromString(metadataString);
	}
	return {
		key,
		value,
		metadata
	};
}
/**
* Parses a single baggage header string into the provided record, applying limits defined in this package.
* Uses indexOf/substring in a while loop to avoid allocating a full array of split entries.
* Returns the updated pair count so callers can track totals across multiple header values.
*/
function parseBaggageHeaderString(value, baggage, count, totalSize) {
	let start = 0;
	while (start < value.length && count < 180) {
		const end = value.indexOf(",", start);
		const entryEnd = end === -1 ? value.length : end;
		const entryLength = entryEnd - start;
		if (entryLength <= 4096) {
			const keyPair = parsePairKeyValue(value.substring(start, entryEnd));
			if (keyPair) {
				const entrySize = (count === 0 ? 0 : 1) + entryLength;
				if (totalSize + entrySize > 8192) break;
				baggage[keyPair.key] = keyPair.metadata ? {
					value: keyPair.value,
					metadata: keyPair.metadata
				} : { value: keyPair.value };
				count++;
				totalSize += entrySize;
			}
		}
		if (end === -1) break;
		start = end + 1;
	}
	return [count, totalSize];
}
init_esm();
/**
* Propagates {@link Baggage} through Context format propagation.
*
* Based on the Baggage specification:
* https://w3c.github.io/baggage/
*/
var W3CBaggagePropagator = class {
	inject(context, carrier, setter) {
		const baggage = propagation.getBaggage(context);
		if (!baggage || isTracingSuppressed(context)) return;
		const headerValue = serializeKeyPairs(getKeyPairs(baggage).filter((pair) => {
			return pair.length <= BAGGAGE_MAX_PER_NAME_VALUE_PAIRS;
		}).slice(0, 180));
		if (headerValue.length > 0) setter.set(carrier, BAGGAGE_HEADER, headerValue);
	}
	extract(context, carrier, getter) {
		const headerValue = getter.get(carrier, BAGGAGE_HEADER);
		if (!headerValue) return context;
		const baggage = {};
		let count = 0;
		let totalSize = 0;
		if (Array.isArray(headerValue)) for (let i = 0; i < headerValue.length; i++) [count, totalSize] = parseBaggageHeaderString(headerValue[i], baggage, count, totalSize);
		else [count] = parseBaggageHeaderString(headerValue, baggage, count, totalSize);
		if (count === 0) return context;
		return propagation.setBaggage(context, propagation.createBaggage(baggage));
	}
	fields() {
		return [BAGGAGE_HEADER];
	}
};
init_esm();
function sanitizeAttributes(attributes) {
	const out = {};
	if (typeof attributes !== "object" || attributes == null) return out;
	for (const key in attributes) {
		if (!Object.prototype.hasOwnProperty.call(attributes, key)) continue;
		if (!isAttributeKey(key)) {
			diag.warn(`Invalid attribute key: ${key}`);
			continue;
		}
		const val = attributes[key];
		if (!isAttributeValue(val)) {
			diag.warn(`Invalid attribute value set for key: ${key}`);
			continue;
		}
		if (Array.isArray(val)) out[key] = val.slice();
		else out[key] = val;
	}
	return out;
}
function isAttributeKey(key) {
	return typeof key === "string" && key !== "";
}
function isAttributeValue(val) {
	if (val == null) return true;
	if (Array.isArray(val)) return isHomogeneousAttributeValueArray(val);
	return isValidPrimitiveAttributeValueType(typeof val);
}
function isHomogeneousAttributeValueArray(arr) {
	let type;
	for (const element of arr) {
		if (element == null) continue;
		const elementType = typeof element;
		if (elementType === type) continue;
		if (!type) {
			if (isValidPrimitiveAttributeValueType(elementType)) {
				type = elementType;
				continue;
			}
			return false;
		}
		return false;
	}
	return true;
}
function isValidPrimitiveAttributeValueType(valType) {
	switch (valType) {
		case "number":
		case "boolean":
		case "string": return true;
	}
	return false;
}
init_esm();
/**
* Returns a function that logs an error using the provided logger, or a
* console logger if one was not provided.
*/
function loggingErrorHandler() {
	return (ex) => {
		diag.error(stringifyException(ex));
	};
}
/**
* Converts an exception into a string representation
* @param {Exception} ex
*/
function stringifyException(ex) {
	if (typeof ex === "string") return ex;
	else return JSON.stringify(flattenException(ex));
}
/**
* Flattens an exception into key-value pairs by traversing the prototype chain
* and coercing values to strings. Duplicate properties will not be overwritten;
* the first insert wins.
*/
function flattenException(ex) {
	const result = {};
	let current = ex;
	while (current !== null) {
		Object.getOwnPropertyNames(current).forEach((propertyName) => {
			if (result[propertyName]) return;
			const value = current[propertyName];
			if (value) result[propertyName] = String(value);
		});
		current = Object.getPrototypeOf(current);
	}
	return result;
}
/** The global error handler delegate */
var delegateHandler = loggingErrorHandler();
/**
* Return the global error handler
* @param {Exception} ex
*/
function globalErrorHandler(ex) {
	try {
		delegateHandler(ex);
	} catch {}
}
var VERSION$4 = "2.11.0";
/**
* Describes a class of error the operation ended with.
*
* @example timeout
* @example java.net.UnknownHostException
* @example server_certificate_invalid
* @example 500
*
* @note The `error.type` **SHOULD** be predictable, and **SHOULD** have low cardinality.
*
* When `error.type` is set to a type (e.g., an exception type), its
* canonical class name identifying the type within the artifact **SHOULD** be used.
*
* If the recorded error type is a wrapper that is not meaningful for
* failure classification, instrumentation **MAY** use the type of the inner
* error instead. For example, in Go, errors created with `fmt.Errorf`
* using `%w` **MAY** be unwrapped when the wrapper type does not help
* classify the failure.
*
* Instrumentations **SHOULD** document the list of errors they report.
*
* The cardinality of `error.type` within one instrumentation library **SHOULD** be low.
* Telemetry consumers that aggregate data from multiple instrumentation libraries and applications
* should be prepared for `error.type` to have high cardinality at query time when no
* additional filters are applied.
*
* If the operation has completed successfully, instrumentations **SHOULD NOT** set `error.type`.
*
* If a specific domain defines its own set of error identifiers (such as HTTP or RPC status codes),
* it's **RECOMMENDED** to:
*
*   - Use a domain-specific attribute
*   - Set `error.type` to capture all errors, regardless of whether they are defined within the domain-specific set or not.
*/
var ATTR_ERROR_TYPE$2 = "error.type";
/**
* The exception message.
*
* @example Division by zero
* @example Can't convert 'int' object to str implicitly
*
* @note > [!WARNING]
*
* > This attribute may contain sensitive information.
*/
var ATTR_EXCEPTION_MESSAGE = "exception.message";
/**
* A stacktrace as a string in the natural representation for the language runtime. The representation is to be determined and documented by each language SIG.
*
* @example "Exception in thread "main" java.lang.RuntimeException: Test exception\\n at com.example.GenerateTrace.methodB(GenerateTrace.java:13)\\n at com.example.GenerateTrace.methodA(GenerateTrace.java:9)\\n at com.example.GenerateTrace.main(GenerateTrace.java:5)\\n"
*/
var ATTR_EXCEPTION_STACKTRACE = "exception.stacktrace";
/**
* The type of the exception (its fully-qualified class name, if applicable). The dynamic type of the exception should be preferred over the static type in languages that support it.
*
* @example java.net.ConnectException
* @example OSError
*
* @note If the recorded exception type is a wrapper that is not meaningful for
* failure classification, instrumentation **MAY** use the type of the inner
* exception instead. For example, in Go, errors created with `fmt.Errorf`
* using `%w` **MAY** be unwrapped when the wrapper type does not help
* classify the failure.
*/
var ATTR_EXCEPTION_TYPE = "exception.type";
/**
* Logical name of the service.
*
* @example shoppingcart
*
* @note **MUST** be the same for all instances of horizontally scaled services. If the value was not specified, SDKs **MUST** fallback to `unknown_service:` concatenated with the process executable name, e.g. `unknown_service:bash`. If the process executable name is not available, the value **MUST** be set to `unknown_service`.
* The process executable name is the name of the process executable, the same value as described by the [`process.executable.name`](process.md) resource attribute.
*/
var ATTR_SERVICE_NAME = "service.name";
/**
* The language of the telemetry SDK.
*/
var ATTR_TELEMETRY_SDK_LANGUAGE = "telemetry.sdk.language";
/**
* Enum value "nodejs" for attribute {@link ATTR_TELEMETRY_SDK_LANGUAGE}.
*/
var TELEMETRY_SDK_LANGUAGE_VALUE_NODEJS = "nodejs";
/**
* The name of the telemetry SDK as defined above.
*
* @example opentelemetry
*
* @note The OpenTelemetry SDK **MUST** set the `telemetry.sdk.name` attribute to `opentelemetry`.
* If another SDK, like a fork or a vendor-provided implementation, is used, this SDK **MUST** set the
* `telemetry.sdk.name` attribute to the fully-qualified class or module name of this SDK's main entry point
* or another suitable identifier depending on the language.
* The identifier `opentelemetry` is reserved and **MUST NOT** be used in this case.
* All custom identifiers **SHOULD** be stable across different versions of an implementation.
*/
var ATTR_TELEMETRY_SDK_NAME = "telemetry.sdk.name";
/**
* The version string of the telemetry SDK.
*
* @example 1.2.3
*/
var ATTR_TELEMETRY_SDK_VERSION = "telemetry.sdk.version";
/**
* The name of the runtime of this process.
*
* @example OpenJDK Runtime Environment
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_PROCESS_RUNTIME_NAME = "process.runtime.name";
/** Constants describing the SDK in use */
var SDK_INFO = {
	[ATTR_TELEMETRY_SDK_NAME]: "opentelemetry",
	[ATTR_PROCESS_RUNTIME_NAME]: "node",
	[ATTR_TELEMETRY_SDK_LANGUAGE]: TELEMETRY_SDK_LANGUAGE_VALUE_NODEJS,
	[ATTR_TELEMETRY_SDK_VERSION]: VERSION$4
};
/**
* @deprecated Use performance directly.
*/
var otperformance = performance;
var NANOSECOND_DIGITS = 9;
var MILLISECONDS_TO_NANOSECONDS = Math.pow(10, 6);
var SECOND_TO_NANOSECONDS = Math.pow(10, NANOSECOND_DIGITS);
/**
* Converts a number of milliseconds from epoch to HrTime([seconds, remainder in nanoseconds]).
* @param epochMillis
*/
function millisToHrTime(epochMillis) {
	const epochSeconds = epochMillis / 1e3;
	return [Math.trunc(epochSeconds), Math.round(epochMillis % 1e3 * MILLISECONDS_TO_NANOSECONDS)];
}
/**
* Returns an hrtime calculated via performance component.
* @param performanceNow
*/
function hrTime(performanceNow) {
	return addHrTimes(millisToHrTime(otperformance.timeOrigin), millisToHrTime(typeof performanceNow === "number" ? performanceNow : otperformance.now()));
}
/**
*
* Converts a TimeInput to an HrTime, defaults to _hrtime().
* @param time
*/
function timeInputToHrTime(time) {
	if (isTimeInputHrTime(time)) return time;
	else if (typeof time === "number") {
		if (time < otperformance.timeOrigin / 2) return hrTime(time);
		else return millisToHrTime(time);
	} else if (time instanceof Date) return millisToHrTime(time.getTime());
	else throw TypeError("Invalid input type");
}
/**
* Returns a duration of two hrTime.
* @param startTime
* @param endTime
*/
function hrTimeDuration(startTime, endTime) {
	let seconds = endTime[0] - startTime[0];
	let nanos = endTime[1] - startTime[1];
	if (nanos < 0) {
		seconds -= 1;
		nanos += SECOND_TO_NANOSECONDS;
	}
	return [seconds, nanos];
}
/**
* Convert hrTime to nanoseconds.
* @param time
*/
function hrTimeToNanoseconds(time) {
	return time[0] * SECOND_TO_NANOSECONDS + time[1];
}
/**
* Convert hrTime to microseconds.
* @param time
*/
function hrTimeToMicroseconds(time) {
	return time[0] * 1e6 + time[1] / 1e3;
}
/**
* Convert hrTime to seconds.
* @param time
*/
function hrTimeToSeconds(time) {
	return time[0] + time[1] / SECOND_TO_NANOSECONDS;
}
/**
* check if time is HrTime
* @param value
*/
function isTimeInputHrTime(value) {
	return Array.isArray(value) && value.length === 2 && typeof value[0] === "number" && typeof value[1] === "number";
}
/**
* check if input value is a correct types.TimeInput
* @param value
*/
function isTimeInput(value) {
	return isTimeInputHrTime(value) || typeof value === "number" || value instanceof Date;
}
/**
* Given 2 HrTime formatted times, return their sum as an HrTime.
*/
function addHrTimes(time1, time2) {
	const out = [time1[0] + time2[0], time1[1] + time2[1]];
	if (out[1] >= SECOND_TO_NANOSECONDS) {
		out[1] -= SECOND_TO_NANOSECONDS;
		out[0] += 1;
	}
	return out;
}
var ExportResultCode;
(function(ExportResultCode) {
	ExportResultCode[ExportResultCode["SUCCESS"] = 0] = "SUCCESS";
	ExportResultCode[ExportResultCode["FAILED"] = 1] = "FAILED";
})(ExportResultCode || (ExportResultCode = {}));
init_esm();
/** Combines multiple propagators into a single propagator. */
var CompositePropagator = class {
	_propagators;
	_fields;
	/**
	* Construct a composite propagator from a list of propagators.
	*
	* @param [config] Configuration object for composite propagator
	*/
	constructor(config = {}) {
		this._propagators = config.propagators ?? [];
		const fields = /* @__PURE__ */ new Set();
		for (const propagator of this._propagators) {
			const propagatorFields = typeof propagator.fields === "function" ? propagator.fields() : [];
			for (const field of propagatorFields) fields.add(field);
		}
		this._fields = Array.from(fields);
	}
	/**
	* Run each of the configured propagators with the given context and carrier.
	* Propagators are run in the order they are configured, so if multiple
	* propagators write the same carrier key, the propagator later in the list
	* will "win".
	*
	* @param context Context to inject
	* @param carrier Carrier into which context will be injected
	*/
	inject(context, carrier, setter) {
		for (const propagator of this._propagators) try {
			propagator.inject(context, carrier, setter);
		} catch (err) {
			diag.warn(`Failed to inject with ${propagator.constructor.name}. Err: ${err.message}`);
		}
	}
	/**
	* Run each of the configured propagators with the given context and carrier.
	* Propagators are run in the order they are configured, so if multiple
	* propagators write the same context key, the propagator later in the list
	* will "win".
	*
	* @param context Context to add values to
	* @param carrier Carrier from which to extract context
	*/
	extract(context, carrier, getter) {
		return this._propagators.reduce((ctx, propagator) => {
			try {
				return propagator.extract(ctx, carrier, getter);
			} catch (err) {
				diag.warn(`Failed to extract with ${propagator.constructor.name}. Err: ${err.message}`);
			}
			return ctx;
		}, context);
	}
	fields() {
		return this._fields.slice();
	}
};
var VALID_KEY_CHAR_RANGE = "[_0-9a-z-*/]";
var VALID_KEY_REGEX = new RegExp(`^(?:${`[a-z]${VALID_KEY_CHAR_RANGE}{0,255}`}|${`[a-z0-9]${VALID_KEY_CHAR_RANGE}{0,240}@[a-z]${VALID_KEY_CHAR_RANGE}{0,13}`})$`);
var VALID_VALUE_BASE_REGEX = /^[ -~]{0,255}[!-~]$/;
var INVALID_VALUE_COMMA_EQUAL_REGEX = /,|=/;
/**
* Key is opaque string up to 256 characters printable. It MUST begin with a
* lowercase letter, and can only contain lowercase letters a-z, digits 0-9,
* underscores _, dashes -, asterisks *, and forward slashes /.
* For multi-tenant vendor scenarios, an at sign (@) can be used to prefix the
* vendor name. Vendors SHOULD set the tenant ID at the beginning of the key.
* see https://www.w3.org/TR/trace-context/#key
*/
function validateKey(key) {
	return VALID_KEY_REGEX.test(key);
}
/**
* Value is opaque string up to 256 characters printable ASCII RFC0020
* characters (i.e., the range 0x20 to 0x7E) except comma , and =.
*/
function validateValue(value) {
	return VALID_VALUE_BASE_REGEX.test(value) && !INVALID_VALUE_COMMA_EQUAL_REGEX.test(value);
}
var MAX_TRACE_STATE_ITEMS = 32;
var MAX_TRACE_STATE_LEN = 512;
var LIST_MEMBERS_SEPARATOR = ",";
var LIST_MEMBER_KEY_VALUE_SPLITTER = "=";
/**
* TraceState must be a class and not a simple object type because of the spec
* requirement (https://www.w3.org/TR/trace-context/#tracestate-field).
*
* Here is the list of allowed mutations:
* - New key-value pair should be added into the beginning of the list
* - The value of any key can be updated. Modified keys MUST be moved to the
* beginning of the list.
*/
var TraceState = class TraceState {
	_length;
	_rawTraceState;
	_internalState;
	constructor(rawTraceState) {
		this._rawTraceState = typeof rawTraceState === "string" ? rawTraceState : "";
		this._length = this._rawTraceState.length;
	}
	set(key, value) {
		if (!validateKey(key) || !validateValue(value)) return this;
		const currState = this._getState();
		const currValue = currState.get(key);
		let newLength = this._length;
		if (typeof currValue === "string") newLength += value.length - currValue.length;
		else newLength += key.length + value.length + (currState.size > 0 ? 2 : 1);
		if (newLength > MAX_TRACE_STATE_LEN) return this;
		const newState = new Map(currState);
		newState.delete(key);
		newState.set(key, value);
		return this._fromState(newState, newLength);
	}
	unset(key) {
		const currState = this._getState();
		const currValue = currState.get(key);
		if (typeof currValue !== "string") return this;
		let newLength = this._length - (key.length + currValue.length + 1);
		if (currState.size > 1) newLength = newLength - 1;
		const newState = new Map(currState);
		newState.delete(key);
		return this._fromState(newState, newLength);
	}
	get(key) {
		return this._getState().get(key);
	}
	serialize() {
		let serialized = "";
		let index = 0;
		for (const entry of this._getState()) {
			if (index > 0) serialized = LIST_MEMBERS_SEPARATOR + serialized;
			serialized = `${entry[0]}${LIST_MEMBER_KEY_VALUE_SPLITTER}${entry[1]}` + serialized;
			index++;
		}
		return serialized;
	}
	_getState() {
		if (this._internalState) return this._internalState;
		const vendorMembers = this._rawTraceState.split(LIST_MEMBERS_SEPARATOR);
		const vendorEntries = /* @__PURE__ */ new Map();
		let currentLength = 0;
		for (const member of vendorMembers) {
			const m = member.trim();
			const idx = m.indexOf(LIST_MEMBER_KEY_VALUE_SPLITTER);
			if (idx === -1) continue;
			const key = m.slice(0, idx);
			const value = m.slice(idx + 1);
			if (!validateKey(key) || !validateValue(value)) continue;
			const futureLength = currentLength + m.length + (vendorEntries.size > 0 ? 1 : 0);
			if (futureLength > MAX_TRACE_STATE_LEN) continue;
			vendorEntries.set(key, value);
			currentLength = futureLength;
			if (vendorEntries.size >= MAX_TRACE_STATE_ITEMS) break;
		}
		this._length = currentLength;
		this._internalState = new Map(Array.from(vendorEntries.entries()).reverse());
		return this._internalState;
	}
	_fromState(state, length) {
		const traceState = Object.create(TraceState.prototype);
		traceState._internalState = state;
		traceState._length = length;
		return traceState;
	}
};
init_esm();
var TRACE_PARENT_HEADER = "traceparent";
var TRACE_STATE_HEADER = "tracestate";
var VERSION$3 = "00";
var TRACE_PARENT_REGEX = new RegExp(`^\\s?((?!ff)[\\da-f]{2})-((?![0]{32})[\\da-f]{32})-((?![0]{16})[\\da-f]{16})-([\\da-f]{2})(-.*)?\\s?$`);
/**
* Parses information from the [traceparent] span tag and converts it into {@link SpanContext}
* @param traceParent - A meta property that comes from server.
*     It should be dynamically generated server side to have the server's request trace Id,
*     a parent span Id that was set on the server's request span,
*     and the trace flags to indicate the server's sampling decision
*     (01 = sampled, 00 = not sampled).
*     for example: '{version}-{traceId}-{spanId}-{sampleDecision}'
*     For more information see {@link https://www.w3.org/TR/trace-context/}
*/
function parseTraceParent(traceParent) {
	const match = TRACE_PARENT_REGEX.exec(traceParent);
	if (!match) return null;
	if (match[1] === "00" && match[5]) return null;
	return {
		traceId: match[2],
		spanId: match[3],
		traceFlags: parseInt(match[4], 16)
	};
}
/**
* Propagates {@link SpanContext} through Trace Context format propagation.
*
* Based on the Trace Context specification:
* https://www.w3.org/TR/trace-context/
*/
var W3CTraceContextPropagator = class {
	inject(context, carrier, setter) {
		const spanContext = trace.getSpanContext(context);
		if (!spanContext || isTracingSuppressed(context) || !isSpanContextValid(spanContext)) return;
		const traceParent = `${VERSION$3}-${spanContext.traceId}-${spanContext.spanId}-0${Number(spanContext.traceFlags || TraceFlags.NONE).toString(16)}`;
		setter.set(carrier, TRACE_PARENT_HEADER, traceParent);
		if (spanContext.traceState) setter.set(carrier, TRACE_STATE_HEADER, spanContext.traceState.serialize());
	}
	extract(context, carrier, getter) {
		const traceParentHeader = getter.get(carrier, TRACE_PARENT_HEADER);
		if (!traceParentHeader) return context;
		const traceParent = Array.isArray(traceParentHeader) ? traceParentHeader[0] : traceParentHeader;
		if (typeof traceParent !== "string") return context;
		const spanContext = parseTraceParent(traceParent);
		if (!spanContext) return context;
		spanContext.isRemote = true;
		const traceStateHeader = getter.get(carrier, TRACE_STATE_HEADER);
		if (traceStateHeader) {
			const state = Array.isArray(traceStateHeader) ? traceStateHeader.join(",") : traceStateHeader;
			spanContext.traceState = new TraceState(typeof state === "string" ? state : void 0);
		}
		return trace.setSpanContext(context, spanContext);
	}
	fields() {
		return [TRACE_PARENT_HEADER, TRACE_STATE_HEADER];
	}
};
/**
* Error that is thrown on timeouts.
*/
var TimeoutError$1 = class TimeoutError$1 extends Error {
	constructor(message) {
		super(message);
		Object.setPrototypeOf(this, TimeoutError$1.prototype);
	}
};
/**
* Adds a timeout to a promise and rejects if the specified timeout has elapsed. Also rejects if the specified promise
* rejects, and resolves if the specified promise resolves.
*
* <p> NOTE: this operation will continue even after it throws a {@link TimeoutError}.
*
* @param promise promise to use with timeout.
* @param timeout the timeout in milliseconds until the returned promise is rejected.
*/
function callWithTimeout$1(promise, timeout) {
	let timeoutHandle;
	const timeoutPromise = new Promise(function timeoutFunction(_resolve, reject) {
		timeoutHandle = setTimeout(function timeoutHandler() {
			reject(new TimeoutError$1("Operation timed out."));
		}, timeout);
	});
	return Promise.race([promise, timeoutPromise]).then((result) => {
		clearTimeout(timeoutHandle);
		return result;
	}, (reason) => {
		clearTimeout(timeoutHandle);
		throw reason;
	});
}
var Deferred = class {
	_promise;
	_resolve;
	_reject;
	constructor() {
		this._promise = new Promise((resolve, reject) => {
			this._resolve = resolve;
			this._reject = reject;
		});
	}
	get promise() {
		return this._promise;
	}
	resolve(val) {
		this._resolve(val);
	}
	reject(err) {
		this._reject(err);
	}
};
/**
* Bind the callback and only invoke the callback once regardless how many times `BindOnceFuture.call` is invoked.
*/
var BindOnceFuture = class {
	_isCalled = false;
	_deferred = new Deferred();
	_callback;
	_that;
	constructor(callback, that) {
		this._callback = callback;
		this._that = that;
	}
	get isCalled() {
		return this._isCalled;
	}
	get promise() {
		return this._deferred.promise;
	}
	call(...args) {
		if (!this._isCalled) {
			this._isCalled = true;
			try {
				Promise.resolve(this._callback.call(this._that, ...args)).then((val) => this._deferred.resolve(val), (err) => this._deferred.reject(err));
			} catch (err) {
				this._deferred.reject(err);
			}
		}
		return this._deferred.promise;
	}
};
init_esm();
/**
* @internal
* Shared functionality used by Exporters while exporting data, including suppression of Traces.
*/
function _export(exporter, arg) {
	return new Promise((resolve) => {
		context.with(suppressTracing(context.active()), () => {
			exporter.export(arg, resolve);
		});
	});
}
var internal = { _export };
/** Hide a denial from the caller as a missing resource and keep the denial as the cause for audit. */
function conceal(denial, message) {
	return new ORPCError("NOT_FOUND", {
		message,
		cause: denial
	});
}
/** Read the denial a failure carries: itself for a 401 or 403, the cause of a concealed one. */
function denialOf(failure) {
	if (failure.status === 401 || failure.status === 403) return failure;
	else if (failure.status === 404 && failure.cause instanceof ORPCError) return denialOf(failure.cause);
}
/** The path a host mounts each package's service under. */
var MOUNTED = /^\/service\/([^/]+)(\/.*)?$/;
/** The paths hosts serve packages' services at, one mount per package. */
var ServiceMount = {
	/** Name the path a package's service is mounted at. */
	path(packageId) {
		return `/service/${packageId}`;
	},
	/** Name the URL of a package's service on a host's origin. */
	url(origin, packageId) {
		return new URL(ServiceMount.path(packageId), origin).href;
	},
	/** Split a request into the mounted package and the request below its mount, absent outside any mount. */
	route(request) {
		const url = new URL(request.url);
		const [, packageId, path] = MOUNTED.exec(url.pathname) ?? [];
		if (packageId === void 0) return;
		url.pathname = path ?? "/";
		return {
			packageId,
			request: copyRequest(request, {}, url.href)
		};
	}
};
/** The path below which a host takes a workload's calls to addresses. */
var EGRESS_PATH = "/.destack/egress";
/** An address followed by the path below it: a scoped package as two segments, an installation as one. */
var ADDRESSED = /^\/(@[^/]+\/[^/]+|[^/@][^/]*)(\/.*)?$/;
/** The paths at which a host takes its workloads' calls: `<egress>/<address>/<path>`, the address an installation (`notes`, `notes.work.acme`) or a package's service (`@destack/audit`). */
var Egress = {
	/** The path of a host's egress on its loopback origin. */
	path: EGRESS_PATH,
	/** Write the URL at which a workload reaches an address through its host's egress. */
	url(egress, address) {
		return `${egress.replace(/\/$/, "")}/${address}`;
	},
	/** Split a request to a host's egress into the address and the request below it, absent outside the egress. */
	route(request) {
		const url = new URL(request.url);
		if (!url.pathname.startsWith(`${EGRESS_PATH}/`)) return;
		const [, address, path] = ADDRESSED.exec(url.pathname.slice(16)) ?? [];
		if (address === void 0) return;
		url.pathname = path ?? "/";
		return {
			address,
			request: copyRequest(request, {}, url.href)
		};
	}
};
/** An identifier a principal proves control of, such as `email:bob@acme.com`, written `scheme:value`. */
var VerifiedIdentifier = defineSchema(string().min(3).max(320).regex(/^[a-z][a-z0-9-]*:\S+$(?![\s\S])/));
/** One principal acting in a request, for the principal before it in the chain, the first for the represented subject. */
var Delegate = defineSchema(strictObject({
	/** The acting principal. */
	subject: Subject,
	/** The authority it acts with: lent by the one before, or all of the one before's authority. */
	authority: _enum(["lent", "full"])
}));
/** How strongly and how recently a caller authenticated. */
var AuthenticationAssurance = defineSchema(strictObject({
	/** The assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant factors. */
	level: number().int().min(1).max(3),
	/** The authentication time in UTC epoch milliseconds. */
	authenticatedAt: number().int().nonnegative()
}));
/** Pair each delegate with the principal it acts for, from the represented subject to the principal sending the request. */
function delegationChain(context) {
	const delegates = context.delegates ?? [];
	if (delegates.length === 0) return [];
	const represented = context.subject;
	if (!represented || !context.subjects.some((subject) => sameSubject(subject, represented))) throw new AccessError("INVALID_CONTEXT", "delegation chain is inconsistent");
	let delegator = represented;
	return delegates.map(({ subject, authority }) => {
		const link = {
			delegate: subject,
			delegator,
			authority
		};
		if (authority === "lent") delegator = subject;
		return link;
	});
}
/** Read the identifiers the represented subject proved, empty under lent authority. */
function verifiedIdentifiers(context) {
	return context.delegates?.some((delegate) => delegate.authority === "lent") ?? false ? [] : context.identifiers ?? [];
}
var __destackModule$37 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739e-a21a-66fd749e17ca",
	"name": "@destack/access",
	"version": "2026.9.0"
} });
/** A named set of permissions defined within a scope. */
var accessRole = defineTable("role", {
	/** The immutable role identifier. */
	id: identifier$1("id", "role").primaryKey(),
	/** Creation time in UTC epoch milliseconds. */
	createdAt: integer("created_at").notNull(),
	/** Last modification time in UTC epoch milliseconds. */
	updatedAt: integer("updated_at").notNull(),
	/** The revision conditional updates name. */
	revision: integer("revision").notNull().default(1),
	/** The installation whose declaration manages the role, absent for roles defined at runtime. */
	managerInstallationId: identifier$1("manager_installation_id", "installation"),
	/** The package of the managing declaration. */
	managerPackageId: identifier$1("manager_package_id", "package"),
	/** The managing declaration's name within its package. */
	managerName: text$1("manager_name"),
	/** When the declaration stopped managing the role, null while it manages it. */
	detachedAt: integer("detached_at"),
	/** The scope defining the role, routing its changes. */
	scope: text$1("scope").notNull(),
	/** The name, unique within the scope. */
	name: text$1("name").notNull(),
	/** The purpose shown when granting the role. */
	description: text$1("description").notNull(),
	/** Whether the role grants every permission in its scope, as an owner does, except reserved ones. */
	isUniversal: boolean$1("is_universal").notNull().default(false)
}, {
	log: { retention: "history" },
	constraints: (role) => [...managerChecks("role", role), unique("role_scope_name").on(role.scope, role.name)]
}, __destackModule$37);
/** One permission a role grants wherever it is bound. */
var accessRolePermission = defineTable("role_permission", {
	/** The permission identifier. */
	id: identifier$1("id", "role-permission").primaryKey().notNull(),
	/** The role containing the permission. */
	roleId: identifier$1("role_id", "role").notNull().references(() => accessRole.id, { onDelete: "cascade" }),
	/** The scope defining the role, routing its changes. */
	scope: text$1("scope").notNull(),
	/** The package declaring the permission. */
	packageId: identifier$1("package_id", "package").notNull(),
	/** The declared object type. */
	type: text$1("type").notNull(),
	/** The permission name within the object type. */
	name: text$1("name").notNull()
}, {
	log: { retention: "history" },
	constraints: (permission) => [
		uniqueIndex("role_permission_name").on(permission.roleId, permission.packageId, permission.type, permission.name),
		index("role_permission_reference").on(permission.packageId, permission.type, permission.name),
		nameCheck("role_permission_type", permission.type),
		nameCheck("role_permission_name", permission.name)
	]
}, __destackModule$37);
/** Require a complete manager or none, a detachment only of a managed row, and one row per declaration. */
function managerChecks(name, columns) {
	return [
		check(`${name}_manager`, sql`(${columns.managerInstallationId} IS NULL AND ${columns.managerPackageId} IS NULL AND ${columns.managerName} IS NULL) OR (${columns.managerInstallationId} IS NOT NULL AND ${columns.managerPackageId} IS NOT NULL AND ${columns.managerName} IS NOT NULL)`),
		check(`${name}_detached`, sql`${columns.detachedAt} IS NULL OR (${columns.managerInstallationId} IS NOT NULL AND ${columns.detachedAt} >= 0)`),
		uniqueIndex(`${name}_manager`).on(columns.managerInstallationId, columns.managerPackageId, columns.managerName).where(sql`${columns.managerInstallationId} IS NOT NULL AND ${columns.detachedAt} IS NULL`)
	];
}
var __destackModule$36 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739e-a21a-66fd749e17ca",
	"name": "@destack/access",
	"version": "2026.9.0"
} });
/** Relate an object to a subject through a declared relation or a bound role. */
var accessRelationship = defineTable("relationship", {
	/** The immutable relationship identifier. */
	id: identifier$1("id", "relationship").primaryKey(),
	/** Creation time in UTC epoch milliseconds. */
	createdAt: integer("created_at").notNull(),
	/** Last modification time in UTC epoch milliseconds. */
	updatedAt: integer("updated_at").notNull(),
	/** The revision a declaration's reconciliation advances. */
	revision: integer("revision").notNull().default(1),
	/** The installation whose declaration manages the relationship, absent for relationships granted at runtime. */
	managerInstallationId: identifier$1("manager_installation_id", "installation"),
	/** The package of the managing declaration. */
	managerPackageId: identifier$1("manager_package_id", "package"),
	/** The managing declaration's name within its package. */
	managerName: text$1("manager_name"),
	/** When the declaration stopped managing the relationship, null while it manages it. */
	detachedAt: integer("detached_at"),
	/** The scope the relationship lives in and routes changes to. */
	scope: text$1("scope").notNull(),
	/** The scope containing the object. */
	objectScope: text$1("object_scope").notNull(),
	/** The package declaring the object's type. */
	packageId: identifier$1("package_id", "package").notNull(),
	/** The object's type. */
	type: text$1("type").notNull(),
	/** The object's identifier. */
	objectId: text$1("object_id").notNull(),
	/** The declared relation, absent for a role binding. */
	relation: text$1("relation"),
	/** The bound role, absent for a declared relation. */
	roleId: identifier$1("role_id", "role").references(() => accessRole.id, { onDelete: "restrict" }),
	/** The package declaring the subject's type. */
	subjectPackageId: identifier$1("subject_package_id", "package").notNull(),
	/** The subject's type. */
	subjectType: text$1("subject_type").notNull(),
	/** The scope containing the subject, or `*` for every scope. */
	subjectScope: text$1("subject_scope").notNull(),
	/** The subject's identifier, or `*` for every object of its type in its scope. */
	subjectId: text$1("subject_id").notNull(),
	/** The relation whose members are the subjects, for a subject set. */
	subjectRelation: text$1("subject_relation"),
	/** Optional expiry in UTC epoch milliseconds. */
	expiresAt: integer("expires_at"),
	/** The request the relationship applies to alone, until it commits. */
	requestId: text$1("request_id"),
	/** The session or agent instance the relationship applies within. */
	sessionId: text$1("session_id"),
	/** The digest of the capability a request must present. */
	capability: text$1("capability"),
	/** The minimum authentication assurance level. */
	assurance: integer("assurance"),
	/** The longest time since authentication, in milliseconds. */
	maxAge: integer("max_age"),
	/** The key of the principal a delegate must act for, making the relationship a delegation. */
	onBehalfOf: text$1("on_behalf_of")
}, {
	log: { retention: "history" },
	constraints: (relationship) => [
		index("relationship_request").on(relationship.requestId),
		index("relationship_expiry").on(relationship.expiresAt),
		index("relationship_max_age").on(relationship.maxAge),
		index("relationship_delegation").on(relationship.onBehalfOf),
		check("relationship_assurance", sql`${relationship.assurance} IS NULL OR ${relationship.assurance} BETWEEN 1 AND 3`),
		check("relationship_max_age", sql`${relationship.maxAge} IS NULL OR ${relationship.maxAge} > 0`),
		...managerChecks("relationship", relationship),
		uniqueIndex("relationship_tuple").on(relationship.objectScope, relationship.packageId, relationship.type, relationship.objectId, sql`coalesce(${relationship.relation}, '')`, sql`coalesce(${relationship.roleId}, '')`, relationship.subjectPackageId, relationship.subjectType, relationship.subjectScope, relationship.subjectId, sql`coalesce(${relationship.subjectRelation}, '')`, sql`coalesce(${relationship.onBehalfOf}, '')`, sql`coalesce(${relationship.requestId}, '')`, sql`coalesce(${relationship.sessionId}, '')`, sql`coalesce(${relationship.capability}, '')`, sql`coalesce(${relationship.assurance}, 0)`, sql`coalesce(${relationship.maxAge}, 0)`),
		index("relationship_subject").on(relationship.subjectPackageId, relationship.subjectType, relationship.subjectScope, relationship.subjectId, relationship.subjectRelation),
		index("relationship_role").on(relationship.roleId),
		check("relationship_label", sql`(${relationship.relation} IS NULL) <> (${relationship.roleId} IS NULL)`),
		check("relationship_wildcard", sql`${relationship.subjectRelation} IS NULL OR (${relationship.subjectId} <> '*' AND ${relationship.subjectScope} <> '*')`),
		check("relationship_expiry", sql`${relationship.expiresAt} IS NULL OR ${relationship.expiresAt} > ${relationship.createdAt}`),
		...[
			relationship.type,
			relationship.relation,
			relationship.subjectType,
			relationship.subjectRelation
		].map((column, position) => nameCheck(`relationship_name_${position}`, column))
	]
}, __destackModule$36);
/** What a request must satisfy for a relationship to apply, beyond its lifetime. */
var RelationshipCondition = defineSchema(strictObject({
	/** Apply only to this request, and only until it commits. */
	request: string().min(1).optional(),
	/** Apply only within this session or agent instance. */
	session: string().min(1).optional(),
	/** Apply only when the request presents the capability with this digest. */
	capability: string().regex(/^[0-9a-f]{64}$(?![\s\S])/).optional(),
	/** Apply only when the caller authenticated at this assurance level or higher. */
	assurance: number().int().min(1).max(3).optional(),
	/** Apply only within this many milliseconds of the caller's authentication. */
	maxAge: number().int().positive().optional(),
	/** Apply only to a delegate acting for this principal, lending the principal's authority: a delegation. */
	onBehalfOf: Subject.optional()
}));
/** An object related to a subject through a declared relation or a bound role: its schema, and its rows. */
var Relationship = {
	/** The schema of a relationship. */
	schema: defineSchema(strictObject({
		/** The stable relationship identifier. */
		id: string().min(1),
		/** The related object. */
		object: ObjectReference,
		/** The declared relation, absent for a role binding. */
		relation: AccessName.optional(),
		/** The bound role, absent for a declared relation. */
		role: string().min(1).optional(),
		/** The subject, subject set or wildcard related to the object. */
		subject: Subject,
		/** The creation time in Unix milliseconds. */
		createdAt: number().int(),
		/** The exclusive expiry time in Unix milliseconds, or null for no expiry. */
		expiresAt: number().int().nullable(),
		/** What a request must satisfy for the relationship to apply. */
		conditions: RelationshipCondition.optional()
	})),
	encode: encode$1,
	decode: decode$1,
	on: on$1,
	readByObject,
	readBySubject,
	subjectColumns,
	replace: replace$1
};
/** Write a relationship as a row in the scope it decides access for. */
function encode$1(relationship, scope) {
	return {
		id: identifier("relationship").parse(relationship.id),
		createdAt: relationship.createdAt,
		updatedAt: relationship.createdAt,
		scope,
		objectScope: relationship.object.scope,
		packageId: relationship.object.packageId,
		type: relationship.object.type,
		objectId: relationship.object.id,
		relation: relationship.relation ?? null,
		roleId: relationship.role === void 0 ? null : identifier("role").parse(relationship.role),
		...subjectColumns(relationship.subject),
		expiresAt: relationship.expiresAt,
		requestId: relationship.conditions?.request ?? null,
		sessionId: relationship.conditions?.session ?? null,
		capability: relationship.conditions?.capability ?? null,
		assurance: relationship.conditions?.assurance ?? null,
		maxAge: relationship.conditions?.maxAge ?? null,
		onBehalfOf: relationship.conditions?.onBehalfOf === void 0 ? null : subjectKey(relationship.conditions.onBehalfOf)
	};
}
/** Write a relationship's subject as its columns. */
function subjectColumns(subject) {
	return {
		subjectPackageId: subject.packageId,
		subjectType: subject.type,
		subjectScope: subject.scope,
		subjectId: subject.id,
		subjectRelation: subject.relation ?? null
	};
}
/** Read a relationship from its row. */
function decode$1(row) {
	const conditions = {
		...row.requestId === null ? {} : { request: row.requestId },
		...row.sessionId === null ? {} : { session: row.sessionId },
		...row.capability === null ? {} : { capability: row.capability },
		...row.assurance === null ? {} : { assurance: row.assurance },
		...row.maxAge === null ? {} : { maxAge: row.maxAge },
		...row.onBehalfOf === null ? {} : { onBehalfOf: keySubject(row.onBehalfOf) }
	};
	return {
		id: row.id,
		object: {
			packageId: row.packageId,
			type: row.type,
			scope: row.objectScope,
			id: row.objectId
		},
		...row.relation === null ? {} : { relation: row.relation },
		...row.roleId === null ? {} : { role: row.roleId },
		subject: {
			packageId: row.subjectPackageId,
			type: row.subjectType,
			scope: row.subjectScope,
			id: row.subjectId,
			...row.subjectRelation === null ? {} : { relation: row.subjectRelation }
		},
		createdAt: row.createdAt,
		expiresAt: row.expiresAt,
		...Object.keys(conditions).length === 0 ? {} : { conditions }
	};
}
/** Read the relationships on some objects as a snapshot shows them. */
async function readByObject(snapshot, objects) {
	return await snapshot.select(accessRelationship, [
		"objectScope",
		"packageId",
		"type",
		"objectId"
	], objects.map((object) => [
		object.scope,
		object.packageId,
		object.type,
		object.id
	]));
}
/** Read the relationships some subjects hold as a snapshot shows them: plain subjects exactly and through wildcards, sets exactly. */
async function readBySubject(snapshot, subjects) {
	const wanted = /* @__PURE__ */ new Map();
	for (const subject of subjects) {
		const isPlain = subject.relation === void 0;
		for (const scope of isPlain ? [subject.scope, "*"] : [subject.scope]) for (const id of isPlain ? [subject.id, "*"] : [subject.id]) {
			const held = [
				subject.packageId,
				subject.type,
				scope,
				id,
				subject.relation ?? null
			];
			wanted.set(JSON.stringify(held), held);
		}
	}
	const tuples = new Map([...wanted.values()].map((held) => [JSON.stringify(held.slice(0, 4)), held.slice(0, 4)]));
	return (await snapshot.select(accessRelationship, [
		"subjectPackageId",
		"subjectType",
		"subjectScope",
		"subjectId"
	], [...tuples.values()])).filter((row) => wanted.has(JSON.stringify([
		row.subjectPackageId,
		row.subjectType,
		row.subjectScope,
		row.subjectId,
		row.subjectRelation
	])));
}
/** Match the relationships on one object, in the relationship table or one of its aliases. */
function on$1(object, relationship = accessRelationship) {
	return sql`(
        ${relationship.objectScope} = ${object.scope}
        AND ${relationship.packageId} = ${object.packageId}
        AND ${relationship.type} = ${object.type}
        AND ${relationship.objectId} = ${object.id}
    )`;
}
/** Relate a subject through a relation to exactly the wanted objects of the selected types, as the system. */
async function replace$1(database, selection, wanted, now) {
	const columns = subjectColumns(selection.subject);
	const held = await database.select({
		id: accessRelationship.id,
		objectScope: accessRelationship.objectScope,
		packageId: accessRelationship.packageId,
		type: accessRelationship.type,
		objectId: accessRelationship.objectId
	}).from(accessRelationship).where(and(eq(accessRelationship.scope, selection.scope), selection.objects.length === 0 ? sql`false` : or(...selection.objects.map((object) => and(eq(accessRelationship.packageId, object.packageId), eq(accessRelationship.type, object.type)))), eq(accessRelationship.relation, selection.relation), eq(accessRelationship.subjectPackageId, columns.subjectPackageId), eq(accessRelationship.subjectType, columns.subjectType), eq(accessRelationship.subjectScope, columns.subjectScope), eq(accessRelationship.subjectId, columns.subjectId), columns.subjectRelation === null ? isNull(accessRelationship.subjectRelation) : eq(accessRelationship.subjectRelation, columns.subjectRelation)));
	const missing = new Map(wanted.map((object) => [objectKey(object), object]));
	const stale = held.filter((row) => !missing.delete(objectKey({
		scope: row.objectScope,
		packageId: row.packageId,
		type: row.type,
		id: row.objectId
	})));
	if (stale.length > 0) await database.delete(accessRelationship).where(inArray(accessRelationship.id, stale.map((row) => row.id)));
	if (missing.size > 0) await database.insert(accessRelationship).values([...missing.values()].map((object) => encode$1({
		id: `relationship-${v7()}`,
		object,
		relation: selection.relation,
		subject: selection.subject,
		createdAt: now,
		expiresAt: null
	}, selection.scope)));
}
var __destackModule$35 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739e-a21a-66fd749e17ca",
	"name": "@destack/access",
	"version": "2026.9.0"
} });
/** A relationship as proposed, before anyone accepts it: its subject may await a recipient. */
var ProposedRelationship = defineSchema(Relationship.schema.omit({
	id: true,
	createdAt: true,
	subject: true
}).extend({ 
/** The proposed subject, absent while an offer awaits whoever proves the recipient identifier. */
subject: Subject.optional() }));
/** A relationship awaiting acceptance, kept apart from the relationships that apply. */
var accessProposal = defineTable("proposal", {
	/** The proposal identifier. */
	id: identifier$1("id", "proposal").primaryKey(),
	/** Creation time in UTC epoch milliseconds. */
	createdAt: integer("created_at").notNull(),
	/** The scope the proposal lives in and routes changes to. */
	scope: text$1("scope").notNull(),
	/** The scope containing the object. */
	objectScope: text$1("object_scope").notNull(),
	/** The package declaring the object's type. */
	packageId: identifier$1("package_id", "package").notNull(),
	/** The object's type. */
	type: text$1("type").notNull(),
	/** The object's identifier. */
	objectId: text$1("object_id").notNull(),
	/** The proposed relation, absent for a proposed role binding. */
	relation: text$1("relation"),
	/** The proposed role, absent for a proposed relation. */
	roleId: identifier$1("role_id", "role").references(() => accessRole.id, { onDelete: "cascade" }),
	/** The relationship accepting the proposal creates. */
	relationship: json$1("relationship", ProposedRelationship).notNull(),
	/** The principal proposing the relationship. */
	proposer: json$1("proposer", Subject).notNull(),
	/** The proposer's key, listing what a principal proposed. */
	proposerKey: text$1("proposer_key").notNull(),
	/** The proposed subject's key or the recipient identifier, listing what addresses a principal. */
	addressee: text$1("addressee").notNull(),
	/** The key of the principal a proposed delegation lends from, listing what awaits its consent. */
	lender: text$1("lender"),
	/** Why the proposer asks for or offers the relationship. */
	purpose: text$1("purpose"),
	/** The exclusive time the proposal lapses, in Unix milliseconds. */
	expiresAt: integer("expires_at").notNull()
}, {
	log: { retention: "history" },
	constraints: (proposal) => [
		index("proposal_object").on(proposal.objectScope, proposal.packageId, proposal.type, proposal.objectId),
		index("proposal_addressee").on(proposal.addressee),
		index("proposal_proposer").on(proposal.proposerKey),
		index("proposal_lender").on(proposal.lender),
		check("proposal_label", sql`(${proposal.relation} IS NULL) <> (${proposal.roleId} IS NULL)`),
		check("proposal_expiry", sql`${proposal.expiresAt} > ${proposal.createdAt}`)
	]
}, __destackModule$35);
/** The tables of every database holding protected objects. */
var accessTables = [
	...[
		Scope.table,
		accessRole,
		accessRolePermission,
		accessRelationship
	],
	accessProposal,
	...replicaTables
];
/** Close each defined role over the defined roles it includes, following only roles the same chain defines. */
function close(roles, includes) {
	const defined = new Map(roles.map((role) => [role.id, role]));
	const included = /* @__PURE__ */ new Map();
	for (const entry of includes) included.set(entry.role, [...included.get(entry.role) ?? [], entry.included]);
	const closure = /* @__PURE__ */ new Map();
	for (const role of roles) {
		const permissions = /* @__PURE__ */ new Map();
		const visited = /* @__PURE__ */ new Set();
		let isUniversal = false;
		const pending = [role.id];
		while (pending.length > 0) {
			const next = defined.get(pending.pop());
			if (next === void 0 || visited.has(next.id)) continue;
			visited.add(next.id);
			isUniversal ||= next.isUniversal;
			for (const permission of next.permissions) permissions.set(permissionKey(permission), permission);
			pending.push(...included.get(next.id) ?? []);
		}
		closure.set(role.id, {
			isUniversal,
			permissions: [...permissions.values()]
		});
	}
	return closure;
}
/** The role every scope's owners hold, granting every permission but reserved ones. */
var OWNER_ROLE = {
	name: "owner",
	description: "Every permission in this scope except reserved ones"
};
/** A role: its schema, its rows and a scope's owners. */
var Role = {
	/** The schema of a role. */
	schema: defineSchema(strictObject({
		/** The role identifier. */
		id: identifier("role"),
		/** The scope defining the role. */
		scope: string().min(1),
		/** The name, unique within the scope. */
		name: string().min(1),
		/** The purpose shown when granting the role. */
		description: string(),
		/** Whether the role grants every permission in its scope but reserved ones. */
		isUniversal: boolean(),
		/** The permissions the role grants itself. */
		permissions: array(PermissionReference),
		/** The revision conditional changes name. */
		revision: number().int().positive()
	})),
	read: read$2,
	permissions,
	permit,
	replace,
	describe,
	own: own$1,
	close
};
/** Read one role of a scope. */
async function read$2(database, scope, id) {
	const [record] = await database.select().from(accessRole).where(and(eq(accessRole.scope, scope), eq(accessRole.id, identifier("role").parse(id))));
	if (!record) throw new AccessError("NOT_FOUND", "role not found");
	return record;
}
/** Read the permissions a role grants itself. */
async function permissions(database, roleId) {
	return (await database.select().from(accessRolePermission).where(eq(accessRolePermission.roleId, roleId)).orderBy(asc(accessRolePermission.packageId), asc(accessRolePermission.type), asc(accessRolePermission.name))).map(referenceOf);
}
/** Record the permissions a role grants. */
async function permit(database, roleId, scope, granted) {
	if (granted.length > 0) await database.insert(accessRolePermission).values(granted.map((permission) => ({
		id: identifier("role-permission").parse(`role-permission-${v7()}`),
		roleId,
		scope,
		packageId: permission.packageId,
		type: permission.type,
		name: permission.name
	})));
}
/** Replace the permissions a role grants with a declared set. */
async function replace(database, roleId, scope, granted) {
	await database.delete(accessRolePermission).where(eq(accessRolePermission.roleId, roleId));
	await permit(database, roleId, scope, granted);
}
/** Describe a role row and its permissions. */
function describe(record, granted) {
	return {
		id: record.id,
		scope: record.scope,
		name: record.name,
		description: record.description,
		isUniversal: record.isUniversal,
		permissions: [...granted],
		revision: record.revision
	};
}
/** Define a scope's owner role and bind it to an owner on the scope's object, returning the role. */
async function own$1(database, scope, owner, now) {
	const role = identifier("role").parse(`role-${v7()}`);
	await database.insert(accessRole).values({
		id: role,
		createdAt: now,
		updatedAt: now,
		scope: scope.id,
		...OWNER_ROLE,
		isUniversal: true
	});
	await database.insert(accessRelationship).values(Relationship.encode({
		id: identifier("relationship").parse(`relationship-${v7()}`),
		object: scope,
		role,
		subject: owner,
		createdAt: now,
		expiresAt: null
	}, scope.id));
	return role;
}
/** Read a permission reference from a role permission row. */
function referenceOf(row) {
	return {
		packageId: row.packageId,
		type: row.type,
		name: row.name
	};
}
/** Take the earliest of some moments, absent when none is present. */
function earliest(moments) {
	const present = moments.filter((moment) => moment !== void 0);
	return present.length === 0 ? void 0 : Math.min(...present);
}
/** The form of a capability secret: 32 bytes as lowercase hexadecimal. */
var SECRET = /^[0-9a-f]{64}$(?![\s\S])/;
/** An unguessable secret that admits its holder through a relationship, stored as a digest. */
var Capability = class Capability {
	/** The secret a request presents. */
	secret;
	/** The digest relationships keep. */
	digest;
	/** Pair a secret with its digest. */
	constructor(secret, digest) {
		this.secret = secret;
		this.digest = digest;
	}
	/** Create a new secret with its digest. */
	static async create() {
		const secret = crypto.getRandomValues(/* @__PURE__ */ new Uint8Array(32)).toHex();
		return new Capability(secret, await Capability.digest(secret));
	}
	/** Hash a presented secret into the digest requests carry and refuse unknown secrets. */
	static async digest(secret) {
		if (!SECRET.test(secret)) throw new AccessError("FORBIDDEN", "invalid capability");
		const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(secret));
		return new Uint8Array(digest).toHex();
	}
};
/** The most proposals one page lists, a screen of pending requests at a few hundred bytes each. */
var PROPOSAL_PAGE_LIMIT = 100;
/** A proposal: its schema, its rows and its addressees. */
var Proposal = {
	/** The schema of a proposal. */
	schema: defineSchema(strictObject({
		/** The stable proposal identifier. */
		id: string().min(1),
		/** The relationship accepting the proposal creates. */
		relationship: ProposedRelationship,
		/** The identifier of the owner who may accept in place of a known subject, such as `email:bob@acme.com`. */
		recipient: VerifiedIdentifier.optional(),
		/** The principal proposing the relationship. */
		proposer: Subject,
		/** Why the proposer asks for or offers the relationship. */
		purpose: string().min(1).optional(),
		/** The creation time in Unix milliseconds. */
		createdAt: number().int(),
		/** The exclusive time the proposal lapses, in Unix milliseconds. */
		expiresAt: number().int()
	})),
	id: proposalId,
	read: read$1,
	on,
	pending,
	encode,
	decode,
	asksForItself,
	addresses
};
/** Select the proposals of a page that have not lapsed, after its starting identifier. */
function pending(page, now) {
	if (!Number.isInteger(page.limit) || page.limit < 1 || page.limit > PROPOSAL_PAGE_LIMIT) throw new AccessError("INVALID_CONTEXT", `proposal page limit must be 1 to ${PROPOSAL_PAGE_LIMIT}`);
	return and(gt(accessProposal.expiresAt, now), page.after === void 0 ? void 0 : gt(accessProposal.id, proposalId(page.after)));
}
/** Select the proposals on one object. */
function on(object) {
	return and(eq(accessProposal.objectScope, object.scope), eq(accessProposal.packageId, object.packageId), eq(accessProposal.type, object.type), eq(accessProposal.objectId, object.id));
}
/** Parse a proposal identifier. */
function proposalId(id) {
	return identifier("proposal").parse(id);
}
/** Read a pending proposal on an object. */
async function read$1(database, object, id) {
	const [row] = await database.select().from(accessProposal).where(and(on(object), eq(accessProposal.id, proposalId(id))));
	if (!row) throw new AccessError("NOT_FOUND", "proposal not found");
	return decode(row);
}
/** Determine whether a principal asked for a relationship with itself. */
function asksForItself(proposal) {
	const subject = proposal.relationship.subject;
	return subject !== void 0 && sameSubject(subject, proposal.proposer);
}
/** Determine whether an offer addresses a principal directly or through a verified identifier. */
function addresses(proposal, principal, context) {
	const subject = proposal.relationship.subject;
	return subject === void 0 ? verifiedIdentifiers(context).includes(proposal.recipient) : sameSubject(subject, principal);
}
/** Write a proposal as its row, keyed for listing by proposer, addressee and lender. */
function encode(proposal, scope) {
	const proposed = proposal.relationship;
	const lender = proposed.conditions?.onBehalfOf;
	return {
		id: proposalId(proposal.id),
		createdAt: proposal.createdAt,
		scope,
		objectScope: proposed.object.scope,
		packageId: proposed.object.packageId,
		type: proposed.object.type,
		objectId: proposed.object.id,
		relation: proposed.relation ?? null,
		roleId: proposed.role === void 0 ? null : identifier("role").parse(proposed.role),
		relationship: proposed,
		proposer: proposal.proposer,
		proposerKey: subjectKey(proposal.proposer),
		addressee: proposed.subject === void 0 ? proposal.recipient : subjectKey(proposed.subject),
		lender: lender === void 0 ? null : subjectKey(lender),
		purpose: proposal.purpose ?? null,
		expiresAt: proposal.expiresAt
	};
}
/** Read a proposal from its row, deriving the recipient of an offer to whoever proves it. */
function decode(row) {
	return {
		id: row.id,
		relationship: row.relationship,
		...row.relationship.subject === void 0 ? { recipient: row.addressee } : {},
		proposer: row.proposer,
		...row.purpose === null ? {} : { purpose: row.purpose },
		createdAt: row.createdAt,
		expiresAt: row.expiresAt
	};
}
/** A caller's authorization under a set of policies in one database. */
var Authorization$1 = class Authorization$1 {
	/** The policies deciding the caller's access. */
	authorizer;
	/** The database, or transaction, holding the access rows that decide. */
	database;
	/** The database as its decisions read it, as each read finds it. */
	snapshot;
	/** Bind the verified caller to a scope it acts in, rejecting scopes its credential excludes. */
	#bind;
	/** The caller's access resolved in each scope during this call, until renewed. */
	#resolved = /* @__PURE__ */ new Map();
	/** The caller's access in the scopes below each scope, absent for a scope it does not enclose, until renewed. */
	#below = /* @__PURE__ */ new Map();
	/** Authorize one caller, bound to each scope it acts in, with its access already resolved in some scope. */
	constructor(authorizer, database, bind, resolved) {
		this.authorizer = authorizer;
		this.database = database;
		this.snapshot = Snapshot.live(database);
		this.#bind = bind;
		if (resolved !== void 0) this.#resolved.set(resolved.scope, Promise.resolve(resolved));
	}
	/** Authorize the same caller within a transaction, resolving afresh there. */
	within(transaction) {
		return new Authorization$1(this.authorizer, transaction, this.#bind);
	}
	/** Bind the verified caller to a scope it acts in. */
	context(scope) {
		return this.#bind(scope);
	}
	/** Resolve the caller's access in a scope once per call: its subject sets, roles and scope chain. */
	in(scope) {
		const known = this.#resolved.get(scope);
		if (known !== void 0) return known;
		const resolved = this.authorizer.resolve(this.snapshot, scope, this.context(scope));
		this.#resolved.set(scope, resolved);
		return resolved;
	}
	/** Forget every resolved scope so the next decision reads current access. */
	renew() {
		this.#resolved.clear();
		this.#below.clear();
	}
	/** Require the caller to hold permissions on a scope as a transaction shows its access, resolved afresh. */
	async #requireFresh(transaction, scope, permissions) {
		const snapshot = Snapshot.live(transaction);
		const access = await this.authorizer.resolve(snapshot, scope.id, this.context(scope.id));
		await this.authorizer.require(snapshot, permissions, scope, access);
	}
	/** Decide whether the caller holds a permission on one object and until when. */
	async check(permission, target, reader) {
		const access = await this.in(this.authorizer.governingScope(target));
		return this.authorizer.check(this.snapshot, permission, target, access, reader);
	}
	/** Start reading grants for decisions in a scope. */
	async reader(scope) {
		return this.authorizer.reader(this.snapshot, (await this.in(scope)).scopes);
	}
	/** Require the caller to hold a permission on one object. */
	async require(permission, target) {
		const access = await this.in(this.authorizer.governingScope(target));
		await this.authorizer.require(this.snapshot, [permission], target, access);
	}
	/** Check a permission on current or past rows of a scope and return the rows held and until when. */
	async checkRows(permission, scope, rows, reader, below) {
		const access = await this.in(scope);
		return this.authorizer.checkRows(this.snapshot, permission, access, rows, reader, below);
	}
	/**
	* Resolve the caller's access in scopes another scope encloses, in reads shared by all of them.
	*
	* A scope it does not enclose is left out, and a credential pinned to the enclosing scope is refused below it.
	*/
	async descend(scope, below) {
		const known = this.#below.get(scope) ?? /* @__PURE__ */ new Map();
		this.#below.set(scope, known);
		const missing = [...new Set(below)].filter((enclosed) => !known.has(enclosed));
		if (missing.length > 0) {
			const resolved = await (await this.in(scope)).descend(this.snapshot, missing);
			for (const enclosed of missing) known.set(enclosed, resolved.get(enclosed));
			for (const enclosed of resolved.keys()) this.context(enclosed);
		}
		const enclosing = /* @__PURE__ */ new Map();
		for (const enclosed of below) {
			const access = known.get(enclosed);
			if (access !== void 0) enclosing.set(enclosed, access);
		}
		return enclosing;
	}
	/**
	* Record a new object's access: a scope's place among the scopes containing it, its first relationships, and a scope's owner role.
	*
	* The object's own create permission is its caller's to require; the first holders relate without a grant, since none can precede them.
	*/
	async create(object, creation) {
		await this.authorizer.requireHeld(this.database, object);
		const [existing] = await this.database.select({ id: accessRelationship.id }).from(accessRelationship).where(Relationship.on(object)).limit(1);
		if (existing) throw new AccessError("INVALID_CONTEXT", "only a new object relates without a grant");
		const isScope = this.authorizer.policy(object).definition.scope === true;
		if (!isScope && creation.owner !== void 0) throw new AccessError("INVALID_CONTEXT", "only a new scope has owners");
		const scope = this.authorizer.governingScope(object);
		const context = this.context(scope);
		if (isScope) {
			const [parent] = object.scope === Scope.universe.id ? [] : await this.database.select({ ancestors: Scope.table.ancestors }).from(Scope.table).where(eq(Scope.table.scope, object.scope));
			if (object.scope !== Scope.universe.id && parent === void 0) throw new AccessError("NOT_FOUND", `unknown scope: ${object.scope}`);
			await this.database.insert(Scope.table).values({
				scope: object.id,
				parent: object.scope,
				packageId: object.packageId,
				type: object.type,
				ancestors: object.scope === Scope.universe.id ? [] : [object.scope, ...parent.ancestors]
			});
		}
		const relationships = this.authorizer.initialRelationships(object, creation, context.now);
		if (relationships.length > 0) await this.database.insert(accessRelationship).values(relationships);
		if (creation.owner !== void 0) await Role.own(this.database, object, creation.owner, context.now);
		this.renew();
	}
	/** Suspend a scope and withhold every permission in it except administration until resumed. */
	async suspend(object) {
		await this.#suspension(object, this.context(object.id).now);
	}
	/** Resume a suspended scope; its caller requires the permission to. */
	async resume(object) {
		await this.#suspension(object, null);
	}
	/** Relate a subject to an object through a relation or a bound role, as the caller may grant. */
	async grant(request) {
		const relationship = await this.database.transaction(async (transaction) => {
			const authorization = this.within(transaction);
			await authorization.authorizeGrant(request);
			return authorization.#insert(request);
		});
		this.renew();
		return relationship;
	}
	/** Revoke one relationship of an object, as the caller may; the log keeps its history. */
	async revoke(object, id) {
		return this.database.transaction(async (transaction) => {
			const authorization = this.within(transaction);
			const key = identifier("relationship").parse(id);
			const [row] = await transaction.select().from(accessRelationship).where(and(Relationship.on(object), eq(accessRelationship.id, key)));
			if (!row) throw new AccessError("NOT_FOUND", "relationship not found");
			await this.authorizer.requireHeld(transaction, object);
			requireUnmanaged$1(row);
			const relationship = Relationship.decode(row);
			await authorization.authorizeRevoke(relationship);
			await requireRemainingOwner(transaction, row);
			await transaction.delete(accessRelationship).where(eq(accessRelationship.id, key));
			this.renew();
			return relationship;
		});
	}
	/** Relate the holder of a new secret to an object and return the secret once. */
	async link(request) {
		const capability = await Capability.create();
		return {
			id: (await this.grant({
				...request,
				subject: anyone.reference("*", "*"),
				conditions: { capability: capability.digest }
			})).id,
			secret: capability.secret
		};
	}
	/**
	* Propose a relationship that applies only once accepted.
	*
	* A principal asks for a relationship with itself as subject; a grantor offers one to a principal or to whoever proves a recipient identifier.
	*/
	async propose(request) {
		return this.database.transaction(async (transaction) => {
			const authorization = this.within(transaction);
			const proposed = request.relationship;
			await this.authorizer.requireHeld(transaction, proposed.object);
			const context = this.context(this.authorizer.governingScope(proposed.object));
			const proposer = requirePrincipal(context);
			if (proposed.subject === void 0 === (request.recipient === void 0) || request.recipient !== void 0 && !VerifiedIdentifier.safeParse(request.recipient).success) throw new AccessError("FORBIDDEN", "proposal needs exactly one subject or recipient");
			this.authorizer.validate(proposed, context.now);
			const expiresAt = request.expiresAt ?? context.now + 6048e5;
			if (!Number.isFinite(expiresAt) || expiresAt <= context.now) throw new AccessError("FORBIDDEN", "proposal must lapse in the future");
			if (proposed.subject === void 0 || !sameSubject(proposed.subject, proposer)) {
				if (proposed.subject !== void 0 && (proposed.subject.relation !== void 0 || proposed.subject.id === "*" || proposed.subject.scope === "*")) throw new AccessError("FORBIDDEN", "proposal subject must be one principal");
				await authorization.authorizeGrant(proposed);
			}
			const proposal = {
				id: identifier("proposal").parse(`proposal-${v7()}`),
				relationship: {
					object: proposed.object,
					...proposed.relation === void 0 ? {} : { relation: proposed.relation },
					...proposed.role === void 0 ? {} : { role: proposed.role },
					...proposed.subject === void 0 ? {} : { subject: proposed.subject },
					expiresAt: proposed.expiresAt ?? null,
					...proposed.conditions === void 0 ? {} : { conditions: proposed.conditions }
				},
				...request.recipient === void 0 ? {} : { recipient: request.recipient },
				proposer,
				...request.purpose === void 0 ? {} : { purpose: request.purpose },
				createdAt: context.now,
				expiresAt
			};
			await transaction.insert(accessProposal).values(Proposal.encode(proposal, this.authorizer.governingScope(proposed.object)));
			return proposal;
		});
	}
	/**
	* Accept a proposal on an object so its relationship applies.
	*
	* A grantor accepts a principal's request; the offered principal, or whoever proves the recipient identifier, accepts an offer and becomes its subject.
	*/
	async accept(object, id) {
		return this.database.transaction(async (transaction) => {
			const authorization = this.within(transaction);
			const context = this.context(this.authorizer.governingScope(object));
			const proposal = await Proposal.read(transaction, object, id);
			if (proposal.expiresAt <= context.now) throw new AccessError("FORBIDDEN", "proposal has lapsed");
			const proposed = proposal.relationship;
			let subject;
			if (Proposal.asksForItself(proposal)) {
				await authorization.authorizeGrant(proposed);
				subject = proposed.subject;
			} else {
				const accepting = requirePrincipal(context);
				if (!Proposal.addresses(proposal, accepting, context)) throw new AccessError("FORBIDDEN", "proposal is addressed to someone else");
				subject = accepting;
				await new Authorization$1(this.authorizer, transaction, () => ({
					subjects: [proposal.proposer],
					now: context.now,
					attributes: {}
				})).authorizeGrant(proposed);
			}
			const { expiresAt, ...granted } = proposed;
			const relationship = await authorization.#insert({
				...granted,
				subject,
				...expiresAt === null ? {} : { expiresAt }
			});
			await transaction.delete(accessProposal).where(eq(accessProposal.id, Proposal.id(id)));
			this.renew();
			return relationship;
		});
	}
	/** Decline a proposal on an object: its proposer withdrawing, its addressee refusing, or a grantor rejecting it. */
	async decline(object, id) {
		return this.database.transaction(async (transaction) => {
			const authorization = this.within(transaction);
			const context = this.context(this.authorizer.governingScope(object));
			const proposal = await Proposal.read(transaction, object, id);
			const acting = principalOf(context);
			if (acting === void 0 || !sameSubject(acting, proposal.proposer) && !Proposal.addresses(proposal, acting, context)) await authorization.authorizeRevoke(proposal.relationship);
			await transaction.delete(accessProposal).where(eq(accessProposal.id, Proposal.id(id)));
			return proposal;
		});
	}
	/** List a page of the proposals pending on an object, ordered by identifier, as its grantors may. */
	async proposals(request, page) {
		return this.database.transaction(async (transaction) => {
			const context = this.context(this.authorizer.governingScope(request.object));
			await this.within(transaction).authorizeRevoke(request);
			return (await transaction.select().from(accessProposal).where(and(Proposal.on(request.object), request.relation === void 0 ? void 0 : eq(accessProposal.relation, request.relation), request.role === void 0 ? void 0 : eq(accessProposal.roleId, identifier("role").parse(request.role)), Proposal.pending(page, context.now))).orderBy(asc(accessProposal.id)).limit(page.limit)).map(Proposal.decode);
		});
	}
	/** List a page of the proposals the caller made, may lend authority for, or receives in a scope. */
	async addressed(scope, page) {
		const context = this.context(scope);
		const own = subjectKey(requirePrincipal(context));
		return (await this.database.select().from(accessProposal).where(and(or(eq(accessProposal.proposerKey, own), eq(accessProposal.lender, own), ...[own, ...verifiedIdentifiers(context)].map((addressee) => eq(accessProposal.addressee, addressee))), Proposal.pending(page, context.now))).orderBy(asc(accessProposal.id)).limit(page.limit)).map(Proposal.decode);
	}
	/** Define a role in a scope, granting only permissions the caller holds there. */
	async createRole(scope, request) {
		return this.database.transaction(async (transaction) => {
			await this.authorizer.requireHeld(transaction, scope);
			const context = this.context(scope.id);
			await this.#requireFresh(transaction, scope, [role.permission("create"), ...request.permissions]);
			const [record] = await transaction.insert(accessRole).values({
				id: identifier("role").parse(`role-${v7()}`),
				createdAt: context.now,
				updatedAt: context.now,
				scope: scope.id,
				name: request.name,
				description: request.description
			}).onConflictDoNothing().returning();
			if (!record) throw new AccessError("CONFLICT", "role name is already in use");
			await Role.permit(transaction, record.id, scope.id, request.permissions);
			this.renew();
			return Role.describe(record, request.permissions);
		});
	}
	/** Change a role's name, purpose or permissions at a revision within the caller's permissions. */
	async updateRole(scope, id, changes) {
		return this.database.transaction(async (transaction) => {
			await this.authorizer.requireHeld(transaction, scope);
			const context = this.context(scope.id);
			const record = await Role.read(transaction, scope.id, id);
			requireUnmanaged$1(record);
			if (record.revision !== changes.revision) throw new AccessError("CONFLICT", "role revision has changed");
			await this.#requireFresh(transaction, scope, [role.permission("update"), ...changes.permissions ?? []]);
			const [updated] = await transaction.update(accessRole).set({
				...changes.name === void 0 ? {} : { name: changes.name },
				...changes.description === void 0 ? {} : { description: changes.description },
				revision: record.revision + 1,
				updatedAt: context.now
			}).where(and(eq(accessRole.id, record.id), eq(accessRole.revision, record.revision))).returning();
			if (!updated) throw new AccessError("CONFLICT", "role revision has changed");
			if (changes.permissions !== void 0) {
				await transaction.delete(accessRolePermission).where(eq(accessRolePermission.roleId, record.id));
				await Role.permit(transaction, record.id, scope.id, changes.permissions);
			}
			this.renew();
			return Role.describe(updated, changes.permissions ?? await Role.permissions(transaction, record.id));
		});
	}
	/** Delete an unbound, editable role at a revision. */
	async deleteRole(scope, id, revision) {
		return this.database.transaction(async (transaction) => {
			await this.authorizer.requireHeld(transaction, scope);
			const record = await Role.read(transaction, scope.id, id);
			requireUnmanaged$1(record);
			if (record.revision !== revision) throw new AccessError("CONFLICT", "role revision has changed");
			await this.#requireFresh(transaction, scope, [role.permission("delete")]);
			const [binding] = await transaction.select({ id: accessRelationship.id }).from(accessRelationship).where(or(eq(accessRelationship.roleId, record.id), and(eq(accessRelationship.subjectPackageId, role.definition.packageId), eq(accessRelationship.subjectType, role.name), eq(accessRelationship.subjectId, record.id)))).limit(1);
			if (binding) throw new AccessError("CONFLICT", "role is still bound or included");
			const permissions = await Role.permissions(transaction, record.id);
			await transaction.delete(accessRole).where(eq(accessRole.id, record.id));
			this.renew();
			return Role.describe(record, permissions);
		});
	}
	/** Require the grant permission and, for a role, every permission the role grants; a delegation needs only its lender. */
	async authorizeGrant(request) {
		await this.authorizer.requireHeld(this.database, request.object);
		const scope = this.authorizer.governingScope(request.object);
		const context = this.context(scope);
		const onBehalfOf = request.conditions?.onBehalfOf;
		if (onBehalfOf !== void 0) {
			if (!this.#lends(request)) throw new AccessError("FORBIDDEN", "only a principal may lend its own authority");
			const delegate = request.subject;
			if (delegate === void 0 || !isPrincipal(delegate) || delegate.id === "*" || delegate.scope === "*") throw new AccessError("FORBIDDEN", "a delegate must be one principal");
		}
		const access = await this.authorizer.resolve(this.snapshot, scope, context);
		const required = onBehalfOf === void 0 || request.role === void 0 ? [this.#grantPermission(request)] : [];
		const grants = request.role === void 0 ? void 0 : access.grants.get(request.role);
		if (request.role !== void 0 && grants === void 0) throw new AccessError("NOT_FOUND", "role not found");
		required.push(...grants?.permissions ?? []);
		await this.authorizer.require(this.snapshot, required, request.object, access);
		if (grants?.isUniversal && !await this.authorizer.owns(this.snapshot, request.object, access)) throw new AccessError("FORBIDDEN", "only owners may bind a role granting everything");
	}
	/** Require the permission granting what a relationship grants, unless the caller lent it. */
	async authorizeRevoke(request) {
		if (this.#lends(request)) return;
		const scope = this.authorizer.governingScope(request.object);
		const access = await this.authorizer.resolve(this.snapshot, scope, this.context(scope));
		await this.authorizer.require(this.snapshot, [this.#grantPermission(request)], request.object, access);
	}
	/** Insert a valid relationship under a new identifier and refuse a duplicate. */
	async #insert(request) {
		const scope = this.authorizer.governingScope(request.object);
		const context = this.context(scope);
		this.authorizer.validate(request, context.now);
		const relationship = {
			id: identifier("relationship").parse(`relationship-${v7()}`),
			object: request.object,
			...request.relation === void 0 ? {} : { relation: request.relation },
			...request.role === void 0 ? {} : { role: request.role },
			subject: request.subject,
			createdAt: context.now,
			expiresAt: request.expiresAt ?? null,
			...request.conditions === void 0 ? {} : { conditions: request.conditions }
		};
		const [inserted] = await this.database.insert(accessRelationship).values(Relationship.encode(relationship, scope)).onConflictDoNothing().returning({ id: accessRelationship.id });
		if (!inserted) throw new AccessError("CONFLICT", "relationship already exists");
		return relationship;
	}
	/** Determine whether the caller is the principal a delegation lends authority from. */
	#lends(request) {
		const onBehalfOf = request.conditions?.onBehalfOf;
		const caller = principalOf(this.context(this.authorizer.governingScope(request.object)));
		return onBehalfOf !== void 0 && caller !== void 0 && sameSubject(caller, onBehalfOf);
	}
	/** Resolve the permission granting a relation, or binding roles, on the object. */
	#grantPermission(request) {
		const policy = this.authorizer.policy(request.object);
		const grant = request.relation === void 0 ? policy.definition.grantedBy : this.authorizer.relation(policy, request.relation).grantedBy;
		if (grant === void 0) throw new AccessError("FORBIDDEN", "relationship cannot be granted");
		return policy.permission(grant);
	}
	/** Set when a scope this database holds was suspended, or clear it. */
	async #suspension(object, suspendedAt) {
		await this.authorizer.requireHeld(this.database, object);
		if (this.authorizer.policy(object).definition.scope !== true) throw new AccessError("INVALID_CONTEXT", `${object.type} is not a scope`);
		const [marked] = await this.database.update(Scope.table).set({ suspendedAt }).where(eq(Scope.table.scope, object.id)).returning({ scope: Scope.table.scope });
		if (marked === void 0) throw new AccessError("NOT_FOUND", `unknown scope: ${object.id}`);
		this.renew();
	}
};
/** Refuse changing a row that a declaration manages until it is detached. */
function requireUnmanaged$1(record) {
	if (record.managerInstallationId !== null && record.detachedAt === null) throw new AccessError("CONFLICT", "record is managed by its source declaration");
}
/** Refuse removing the last binding of a role that grants everything on an object. */
async function requireRemainingOwner(database, row) {
	if (row.roleId === null) return;
	const owners = await database.select({ id: accessRelationship.id }).from(accessRelationship).innerJoin(accessRole, eq(accessRole.id, accessRelationship.roleId)).where(and(Relationship.on(Relationship.decode(row).object), eq(accessRole.isUniversal, true)));
	if (owners.length === 1 && owners[0].id === row.id) throw new AccessError("CONFLICT", "the last owner cannot be removed");
}
defineSchema(strictObject({
	/** Whether the caller holds the permission. */
	isAllowed: boolean(),
	/** The next moment time alone may change the decision, absent when only changes to access do. */
	until: number().int().optional()
}));
/** The gate that refuses a request before any grant: credential, elevation, suspension or scope. */
var Gate = defineSchema(_enum([
	"restricted",
	"elevation",
	"suspended",
	"outside"
]));
/** Why one grant fails to admit an authority. */
var GrantFailure = defineSchema(_enum([
	"subject",
	"field",
	"role",
	"pending",
	"expired",
	"request",
	"session",
	"capability",
	"assurance",
	"age",
	"delegation",
	"arrow"
]));
/** Whether and why a caller holds a permission on one object. */
var Explanation = defineSchema(strictObject({
	/** The permission explained. */
	permission: PermissionReference,
	/** The object explained. */
	object: ObjectReference,
	/** Whether the caller holds the permission. */
	isAllowed: boolean(),
	/** The gate the request fails before any grant. */
	gate: Gate.optional(),
	/** The represented subject and each delegate, all of which must be admitted. */
	authorities: array(strictObject({
		/** The delegate, absent for the represented subject. */
		delegate: Subject.optional(),
		/** The principal the delegate acts for. */
		delegator: Subject.optional(),
		/** Whether a grant admits the authority. */
		isAllowed: boolean(),
		/** Every grant of the permission on the object, with why it fails when it does. */
		grants: array(strictObject({
			/** How the permission reaches the grant, in order. */
			path: array(string()),
			/** The object the relationship, field or role binding sits on. */
			object: ObjectReference,
			/** The subject the grant admits. */
			subject: Subject,
			/** The bound role, for role bindings. */
			role: string().optional(),
			/** Why the grant fails to admit the authority, absent when it admits it. */
			failure: GrantFailure.optional()
		}))
	}))
}));
init_trace_api();
var __destackModule$34 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} });
/** The failure log records. */
var { log: log$1 } = scope(__destackModule$34.package);
/** Record unexpected failures and return an error safe for clients. */
function reportError(error) {
	if (error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE") return new ORPCError("CONFLICT", { message: error.message });
	if (error instanceof ORPCError && error.code === "BAD_REQUEST" && error.cause instanceof ValidationError) {
		const issues = error.cause.issues.map((issue) => {
			const path = (issue.path ?? []).map((key) => typeof key === "object" ? key.key : key).join(".");
			const message = issue.message.charAt(0).toLowerCase() + issue.message.slice(1);
			return path === "" ? message : `${path}: ${message}`;
		});
		return new ORPCError("BAD_REQUEST", {
			message: `invalid input: ${issues.join("; ")}`,
			cause: error.cause
		});
	} else if (error instanceof ORPCError && error.status < 500) return error;
	const failure = domainFailure(error);
	if (failure !== void 0) return failure;
	trace.getActiveSpan()?.recordException(error instanceof Error ? error : String(error));
	log$1.error("service.request.failed", exceptionAttributes(error));
	if (error instanceof ORPCError && error.code !== "INTERNAL_SERVER_ERROR") return error;
	return new ORPCError("INTERNAL_SERVER_ERROR", {
		message: "internal server error",
		cause: error
	});
}
/** Map a domain failure to a service failure. */
function domainFailure(error) {
	if (error instanceof AccessError && error.code === "INSUFFICIENT_AUTHENTICATION") return new ORPCError(error.code, {
		status: 401,
		message: error.message,
		data: error.stepUp
	});
	else if (error instanceof AccessError && error.code !== "INVALID_DECLARATION") {
		const code = error.code === "INVALID_CONTEXT" ? "FORBIDDEN" : error.code === "STALE" ? "SERVICE_UNAVAILABLE" : error.code;
		return new ORPCError(code, { message: error.message });
	} else if (error instanceof SyncError && error.code === "NOT_FOUND") return new ORPCError("NOT_FOUND", { message: error.message });
	else if (error instanceof DatabaseError && (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE")) return new ORPCError("CONFLICT", { message: error.message });
	else if (error instanceof DatabaseError && (error.code === "INVALID_QUERY" || error.code === "INVALID_RECORD")) return new ORPCError("BAD_REQUEST", { message: error.message });
}
/** Log a failed reconciliation. */
function reportReconciliation(controller, key, error) {
	log$1.warn("controller.reconcile.failed", {
		"destack.controller": controller.name,
		"destack.key": key,
		...exceptionAttributes(error)
	});
}
function resolveFriendlyStandardHandleOptions(options) {
	return {
		...options,
		context: options.context ?? {}
	};
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
		const jsonSerializer = new StandardOpenAPIJsonSerializer(options);
		const bracketNotationSerializer = new StandardBracketNotationSerializer(options);
		const serializer = new StandardOpenAPISerializer(jsonSerializer, bracketNotationSerializer);
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
/** Authorize a call and audit it after it ends. */
async function invokeProcedure(call, next, options) {
	const audit = options.audit;
	try {
		if (options.authorize) await options.authorize(call);
	} catch (error) {
		const failure = reportError(error);
		if (audit) await recordAudit({
			call,
			outcome: outcomeOf(failure),
			error: failure
		}, audit);
		throw failure;
	}
	let result;
	try {
		result = await next();
	} catch (error) {
		const failure = reportError(error);
		if (audit) await recordAudit({
			call,
			outcome: outcomeOf(failure),
			error: failure
		}, audit);
		throw failure;
	}
	if (result !== null && typeof result === "object" && Symbol.asyncIterator in result) return streamProcedure(result, call, options);
	if (audit) await recordAudit({
		call,
		outcome: "success"
	}, audit);
	return result;
}
/** Authorize and audit a stream. */
function streamProcedure(stream, call, options) {
	const audit = options.audit;
	let outcome = "cancelled";
	let failure;
	const iterator = stream[Symbol.asyncIterator]();
	return new AsyncIteratorClass(async () => {
		try {
			if (options.authorize) await options.authorize(call);
			const result = await iterator.next();
			if (options.authorize && !result.done) await options.authorize(call);
			if (result.done) outcome = "success";
			return result;
		} catch (error) {
			const reported = reportError(error);
			failure = reported;
			outcome = outcomeOf(reported);
			throw reported;
		}
	}, async (reason) => {
		try {
			if (reason !== "next" || outcome !== "success") await iterator.return?.();
		} catch (error) {
			outcome = "failure";
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
/** Record an audit event and keep any handler failure. */
async function recordAudit(event, audit) {
	try {
		await audit(event);
	} catch (error) {
		throw reportError(event.error !== void 0 ? new AggregateError([event.error, error], "procedure failure audit failed") : error);
	}
}
/** Classify a failure as a denial, concealed or not, or a failure. */
function outcomeOf(failure) {
	return denialOf(failure) === void 0 ? "failure" : "denied";
}
/**
* Content encoding strategy enum.
*
* - [Content-Transfer-Encoding Syntax](https://datatracker.ietf.org/doc/html/rfc2045#section-6.1)
* - [7bit vs 8bit encoding](https://stackoverflow.com/questions/25710599/content-transfer-encoding-7bit-or-8-bit/28531705#28531705)
*/
var ContentEncoding;
(function(ContentEncoding) {
	/**
	* Only US-ASCII characters, which use the lower 7 bits for each character.
	*
	* Each line must be less than 1,000 characters.
	*/
	ContentEncoding["7bit"] = "7bit";
	/**
	* Allow extended ASCII characters which can use the 8th (highest) bit to
	* indicate special characters not available in 7bit.
	*
	* Each line must be less than 1,000 characters.
	*/
	ContentEncoding["8bit"] = "8bit";
	/**
	* Useful for data that is mostly non-text.
	*/
	ContentEncoding["Base64"] = "base64";
	/**
	* Same character set as 8bit, with no line length restriction.
	*/
	ContentEncoding["Binary"] = "binary";
	/**
	* An extension token defined by a standards-track RFC and registered with
	* IANA.
	*/
	ContentEncoding["IETFToken"] = "ietf-token";
	/**
	* Lines are limited to 76 characters, and line breaks are represented using
	* special characters that are escaped.
	*/
	ContentEncoding["QuotedPrintable"] = "quoted-printable";
	/**
	* The two characters "X-" or "x-" followed, with no intervening white space,
	* by any token.
	*/
	ContentEncoding["XToken"] = "x-token";
})(ContentEncoding || (ContentEncoding = {}));
/**
* This enum provides well-known formats that apply to strings.
*/
var Format;
(function(Format) {
	/**
	* A string instance is valid against this attribute if it is a valid
	* representation according to the "full-date" production in
	* [RFC 3339][RFC3339].
	*
	* [RFC3339]: https://datatracker.ietf.org/doc/html/rfc3339
	*/
	Format["Date"] = "date";
	/**
	* A string instance is valid against this attribute if it is a valid
	* representation according to the "date-time" production in
	* [RFC 3339][RFC3339].
	*
	* [RFC3339]: https://datatracker.ietf.org/doc/html/rfc3339
	*/
	Format["DateTime"] = "date-time";
	/**
	* A string instance is valid against this attribute if it is a valid
	* representation according to the "duration" production.
	*/
	Format["Duration"] = "duration";
	/**
	* A string instance is valid against this attribute if it is a valid Internet
	* email address as defined by by the "Mailbox" ABNF rule in [RFC
	* 5321][RFC5322], section 4.1.2.
	*
	* [RFC5321]: https://datatracker.ietf.org/doc/html/rfc5321
	*/
	Format["Email"] = "email";
	/**
	* As defined by [RFC 1123, section 2.1][RFC1123], including host names
	* produced using the Punycode algorithm specified in
	* [RFC 5891, section 4.4][RFC5891].
	*
	* [RFC1123]: https://datatracker.ietf.org/doc/html/rfc1123
	* [RFC5891]: https://datatracker.ietf.org/doc/html/rfc5891
	*/
	Format["Hostname"] = "hostname";
	/**
	* A string instance is valid against this attribute if it is a valid Internet
	* email address as defined by the extended "Mailbox" ABNF rule in
	* [RFC 6531][RFC6531], section 3.3.
	*
	* [RFC6531]: https://datatracker.ietf.org/doc/html/rfc6531
	*/
	Format["IDNEmail"] = "idn-email";
	/**
	* As defined by either [RFC 1123, section 2.1][RFC1123] as for hostname, or
	* an internationalized hostname as defined by
	* [RFC 5890, section 2.3.2.3][RFC5890].
	*
	* [RFC1123]: https://datatracker.ietf.org/doc/html/rfc1123
	* [RFC5890]: https://datatracker.ietf.org/doc/html/rfc5890
	*/
	Format["IDNHostname"] = "idn-hostname";
	/**
	* An IPv4 address according to the "dotted-quad" ABNF syntax as defined in
	* [RFC 2673, section 3.2][RFC2673].
	*
	* [RFC2673]: https://datatracker.ietf.org/doc/html/rfc2673
	*/
	Format["IPv4"] = "ipv4";
	/**
	* An IPv6 address as defined in [RFC 4291, section 2.2][RFC4291].
	*
	* [RFC4291]: https://datatracker.ietf.org/doc/html/rfc4291
	*/
	Format["IPv6"] = "ipv6";
	/**
	* A string instance is valid against this attribute if it is a valid IRI,
	* according to [RFC 3987][RFC3987].
	*
	* [RFC3987]: https://datatracker.ietf.org/doc/html/rfc3987
	*/
	Format["IRI"] = "iri";
	/**
	* A string instance is valid against this attribute if it is a valid IRI
	* Reference (either an IRI or a relative-reference), according to
	* [RFC 3987][RFC3987].
	*
	* [RFC3987]: https://datatracker.ietf.org/doc/html/rfc3987
	*/
	Format["IRIReference"] = "iri-reference";
	/**
	* A string instance is valid against this attribute if it is a valid JSON
	* string representation of a JSON Pointer, according to
	* [RFC 6901, section 5][RFC6901].
	*
	* [RFC6901]: https://datatracker.ietf.org/doc/html/rfc6901
	*/
	Format["JSONPointer"] = "json-pointer";
	/**
	* A string instance is valid against this attribute if it is a valid JSON
	* string representation of a JSON Pointer fragment, according to
	* [RFC 6901, section 5][RFC6901].
	*
	* [RFC6901]: https://datatracker.ietf.org/doc/html/rfc6901
	*/
	Format["JSONPointerURIFragment"] = "json-pointer-uri-fragment";
	/**
	* This attribute applies to string instances.
	*
	* A regular expression, which SHOULD be valid according to the
	* [ECMA-262][ecma262] regular expression dialect.
	*
	* Implementations that validate formats MUST accept at least the subset of
	* [ECMA-262][ecma262] defined in the [Regular Expressions][regexInterop]
	* section of this specification, and SHOULD accept all valid
	* [ECMA-262][ecma262] expressions.
	*
	* [ecma262]: https://www.ecma-international.org/publications-and-standards/standards/ecma-262/
	* [regexInterop]: https://json-schema.org/draft/2020-12/json-schema-validation.html#regexInterop
	*/
	Format["RegEx"] = "regex";
	/**
	* A string instance is valid against this attribute if it is a valid
	* [Relative JSON Pointer][relative-json-pointer].
	*
	* [relative-json-pointer]: https://datatracker.ietf.org/doc/html/draft-handrews-relative-json-pointer-01
	*/
	Format["RelativeJSONPointer"] = "relative-json-pointer";
	/**
	* A string instance is valid against this attribute if it is a valid
	* representation according to the "time" production in [RFC 3339][RFC3339].
	*
	* [RFC3339]: https://datatracker.ietf.org/doc/html/rfc3339
	*/
	Format["Time"] = "time";
	/**
	* A string instance is valid against this attribute if it is a valid URI,
	* according to [RFC3986][RFC3986].
	*
	* [RFC3986]: https://datatracker.ietf.org/doc/html/rfc3986
	*/
	Format["URI"] = "uri";
	/**
	* A string instance is valid against this attribute if it is a valid URI
	* Reference (either a URI or a relative-reference), according to
	* [RFC3986][RFC3986].
	*
	* [RFC3986]: https://datatracker.ietf.org/doc/html/rfc3986
	*/
	Format["URIReference"] = "uri-reference";
	/**
	* A string instance is valid against this attribute if it is a valid URI
	* Template (of any level), according to [RFC 6570][RFC6570].
	*
	* Note that URI Templates may be used for IRIs; there is no separate IRI
	* Template specification.
	*
	* [RFC6570]: https://datatracker.ietf.org/doc/html/rfc6570
	*/
	Format["URITemplate"] = "uri-template";
	/**
	* A string instance is valid against this attribute if it is a valid string
	* representation of a UUID, according to [RFC 4122][RFC4122].
	*
	* [RFC4122]: https://datatracker.ietf.org/doc/html/rfc4122
	*/
	Format["UUID"] = "uuid";
})(Format || (Format = {}));
/**
* Enum consisting of simple type names for the `type` keyword
*/
var TypeName;
(function(TypeName) {
	/**
	* Value MUST be an array.
	*/
	TypeName["Array"] = "array";
	/**
	* Value MUST be a boolean.
	*/
	TypeName["Boolean"] = "boolean";
	/**
	* Value MUST be an integer, no floating point numbers are allowed. This is a
	* subset of the number type.
	*/
	TypeName["Integer"] = "integer";
	/**
	* Value MUST be null. Note this is mainly for purpose of being able use union
	* types to define nullability. If this type is not included in a union, null
	* values are not allowed (the primitives listed above do not allow nulls on
	* their own).
	*/
	TypeName["Null"] = "null";
	/**
	* Value MUST be a number, floating point numbers are allowed.
	*/
	TypeName["Number"] = "number";
	/**
	* Value MUST be an object.
	*/
	TypeName["Object"] = "object";
	/**
	* Value MUST be a string.
	*/
	TypeName["String"] = "string";
})(TypeName || (TypeName = {}));
TypeName.String, TypeName.Number, TypeName.Integer, TypeName.Boolean, TypeName.Null;
var CompositeSchemaConverter = class {
	converters;
	constructor(converters) {
		this.converters = converters;
	}
	async convert(schema, options) {
		for (const converter of this.converters) if (await converter.condition(schema, options)) return converter.convert(schema, options);
		return [false, {}];
	}
};
var JsonSchemaXNativeType = /* @__PURE__ */ ((JsonSchemaXNativeType2) => {
	JsonSchemaXNativeType2["BigInt"] = "bigint";
	JsonSchemaXNativeType2["RegExp"] = "regexp";
	JsonSchemaXNativeType2["Date"] = "date";
	JsonSchemaXNativeType2["Url"] = "url";
	JsonSchemaXNativeType2["Set"] = "set";
	JsonSchemaXNativeType2["Map"] = "map";
	return JsonSchemaXNativeType2;
})(JsonSchemaXNativeType || {});
var FLEXIBLE_DATE_FORMAT_REGEX = /^[^-]+-[^-]+-[^-]+$/;
var JsonSchemaCoercer = class {
	coerce(schema, value, options = {}) {
		const [, coerced] = this.#coerce(schema, value, options);
		return coerced;
	}
	#coerce(schema, originalValue, options) {
		if (typeof schema === "boolean") return [schema, originalValue];
		if (Array.isArray(schema.type)) return this.#coerce({ anyOf: schema.type.map((type) => ({
			...schema,
			type
		})) }, originalValue, options);
		let coerced = originalValue;
		let satisfied = true;
		if (typeof schema.$ref === "string") {
			const refSchema = options?.components?.[schema.$ref];
			if (refSchema !== void 0) {
				const [subSatisfied, subCoerced] = this.#coerce(refSchema, coerced, options);
				coerced = subCoerced;
				satisfied = subSatisfied;
			}
		}
		const enumValues = schema.const !== void 0 ? [schema.const] : schema.enum;
		if (enumValues !== void 0 && !enumValues.includes(coerced)) {
			if (typeof coerced === "string") {
				const numberValue = this.#stringToNumber(coerced);
				if (enumValues.includes(numberValue)) coerced = numberValue;
				else {
					const booleanValue = this.#stringToBoolean(coerced);
					if (enumValues.includes(booleanValue)) coerced = booleanValue;
					else satisfied = false;
				}
			} else satisfied = false;
		}
		if (typeof schema.type === "string") switch (schema.type) {
			case "null":
				if (coerced !== null) satisfied = false;
				break;
			case "string":
				if (typeof coerced !== "string") satisfied = false;
				break;
			case "number":
				if (typeof coerced === "string") coerced = this.#stringToNumber(coerced);
				if (typeof coerced !== "number") satisfied = false;
				break;
			case "integer":
				if (typeof coerced === "string") coerced = this.#stringToInteger(coerced);
				if (typeof coerced !== "number" || !Number.isInteger(coerced)) satisfied = false;
				break;
			case "boolean":
				if (typeof coerced === "string") coerced = this.#stringToBoolean(coerced);
				if (typeof coerced !== "boolean") satisfied = false;
				break;
			case "array":
				if (Array.isArray(coerced)) {
					const prefixItemSchemas = "prefixItems" in schema ? toArray(schema.prefixItems) : Array.isArray(schema.items) ? schema.items : [];
					const itemSchema = Array.isArray(schema.items) ? schema.additionalItems : schema.items;
					let shouldUseCoercedItems = false;
					const coercedItems = coerced.map((item, i) => {
						const subSchema = prefixItemSchemas[i] ?? itemSchema;
						if (subSchema === void 0) {
							satisfied = false;
							return item;
						}
						const [subSatisfied, subCoerced] = this.#coerce(subSchema, item, options);
						if (!subSatisfied) satisfied = false;
						if (subCoerced !== item) shouldUseCoercedItems = true;
						return subCoerced;
					});
					if (coercedItems.length < prefixItemSchemas.length) satisfied = false;
					if (shouldUseCoercedItems) coerced = coercedItems;
				} else satisfied = false;
				break;
			case "object":
				if (Array.isArray(coerced)) coerced = { ...coerced };
				if (isObject(coerced)) {
					let shouldUseCoercedItems = false;
					const coercedItems = new NullProtoObj$1();
					const patternProperties = Object.entries(schema.patternProperties ?? {}).map(([key, value]) => [new RegExp(key), value]);
					for (const key in coerced) {
						const value = coerced[key];
						const subSchema = (schema.properties !== void 0 && Object.hasOwn(schema.properties, key) ? schema.properties[key] : void 0) ?? patternProperties.find(([pattern]) => pattern.test(key))?.[1] ?? schema.additionalProperties;
						if (value === void 0 && !schema.required?.includes(key)) coercedItems[key] = value;
						else if (subSchema === void 0) {
							coercedItems[key] = value;
							satisfied = false;
						} else {
							const [subSatisfied, subCoerced] = this.#coerce(subSchema, value, options);
							coercedItems[key] = subCoerced;
							if (!subSatisfied) satisfied = false;
							if (subCoerced !== value) shouldUseCoercedItems = true;
						}
					}
					if (schema.required?.some((key) => !Object.hasOwn(coercedItems, key))) satisfied = false;
					if (shouldUseCoercedItems) coerced = coercedItems;
				} else satisfied = false;
		}
		if ("x-native-type" in schema && typeof schema["x-native-type"] === "string") switch (schema["x-native-type"]) {
			case JsonSchemaXNativeType.Date:
				if (typeof coerced === "string") coerced = this.#stringToDate(coerced);
				if (!(coerced instanceof Date)) satisfied = false;
				break;
			case JsonSchemaXNativeType.BigInt:
				switch (typeof coerced) {
					case "string":
						coerced = this.#stringToBigInt(coerced);
						break;
					case "number": coerced = this.#numberToBigInt(coerced);
				}
				if (typeof coerced !== "bigint") satisfied = false;
				break;
			case JsonSchemaXNativeType.RegExp:
				if (typeof coerced === "string") coerced = this.#stringToRegExp(coerced);
				if (!(coerced instanceof RegExp)) satisfied = false;
				break;
			case JsonSchemaXNativeType.Url:
				if (typeof coerced === "string") coerced = this.#stringToURL(coerced);
				if (!(coerced instanceof URL)) satisfied = false;
				break;
			case JsonSchemaXNativeType.Set:
				if (Array.isArray(coerced)) coerced = this.#arrayToSet(coerced);
				if (!(coerced instanceof Set)) satisfied = false;
				break;
			case JsonSchemaXNativeType.Map:
				if (Array.isArray(coerced)) coerced = this.#arrayToMap(coerced);
				if (!(coerced instanceof Map)) satisfied = false;
		}
		if (schema.allOf) for (const subSchema of schema.allOf) {
			const [subSatisfied, subCoerced] = this.#coerce(subSchema, coerced, options);
			coerced = subCoerced;
			if (!subSatisfied) satisfied = false;
		}
		for (const key of ["anyOf", "oneOf"]) if (schema[key]) {
			let bestOptions;
			for (const subSchema of schema[key]) {
				const [subSatisfied, subCoerced] = this.#coerce(subSchema, coerced, options);
				if (subSatisfied) {
					if (!bestOptions || subCoerced === coerced) bestOptions = {
						coerced: subCoerced,
						satisfied: subSatisfied
					};
					if (subCoerced === coerced) break;
				}
			}
			coerced = bestOptions ? bestOptions.coerced : coerced;
			satisfied = bestOptions ? bestOptions.satisfied : false;
		}
		if (typeof schema.not !== "undefined") {
			const [notSatisfied] = this.#coerce(schema.not, coerced, options);
			if (notSatisfied) satisfied = false;
		}
		return [satisfied, coerced];
	}
	#stringToNumber(value) {
		const num = Number.parseFloat(value);
		if (Number.isNaN(num) || num !== Number(value)) return value;
		return num;
	}
	#stringToInteger(value) {
		const num = Number.parseInt(value);
		if (Number.isNaN(num) || num !== Number(value)) return value;
		return num;
	}
	#stringToBoolean(value) {
		const lower = value.toLowerCase();
		if (lower === "false" || lower === "off") return false;
		if (lower === "true" || lower === "on") return true;
		return value;
	}
	#stringToBigInt(value) {
		return guard$1(() => BigInt(value)) ?? value;
	}
	#numberToBigInt(value) {
		return guard$1(() => BigInt(value)) ?? value;
	}
	#stringToDate(value) {
		const date = new Date(value);
		if (Number.isNaN(date.getTime()) || !FLEXIBLE_DATE_FORMAT_REGEX.test(value)) return value;
		return date;
	}
	#stringToRegExp(value) {
		const match = value.match(/^\/(.*)\/([a-z]*)$/);
		if (match) {
			const [, pattern, flags] = match;
			return guard$1(() => new RegExp(pattern, flags)) ?? value;
		}
		return value;
	}
	#stringToURL(value) {
		return guard$1(() => new URL(value)) ?? value;
	}
	#arrayToSet(value) {
		const set = new Set(value);
		if (set.size !== value.length) return value;
		return set;
	}
	#arrayToMap(value) {
		if (value.some((item) => !Array.isArray(item) || item.length !== 2)) return value;
		const result = new Map(value);
		if (result.size !== value.length) return value;
		return result;
	}
};
var SmartCoercionPlugin = class {
	converter;
	coercer;
	cache = /* @__PURE__ */ new WeakMap();
	constructor(options = {}) {
		this.converter = new CompositeSchemaConverter(toArray(options.schemaConverters));
		this.coercer = new JsonSchemaCoercer();
	}
	init(options) {
		options.clientInterceptors ??= [];
		options.clientInterceptors.unshift(async (options2) => {
			const inputSchema = options2.procedure["~orpc"].inputSchema;
			if (!inputSchema) return options2.next();
			const coercedInput = await this.#coerce(inputSchema, options2.input);
			return options2.next({
				...options2,
				input: coercedInput
			});
		});
	}
	async #coerce(schema, value) {
		let jsonSchema = this.cache.get(schema);
		if (!jsonSchema) {
			jsonSchema = (await this.converter.convert(schema, { strategy: "input" }))[1];
			this.cache.set(schema, jsonSchema);
		}
		return this.coercer.coerce(jsonSchema, value);
	}
};
init_context_api();
/** Convert Destack schemas for HTTP decoding and OpenAPI. */
var schemaConverter = {
	condition: (validator) => validator instanceof ZodType,
	convert: (validator) => {
		if (!(validator instanceof ZodType)) throw new TypeError("expected a Destack schema");
		return [true, toJsonSchema(validator)];
	}
};
/** Dispatch requests to service procedures. */
var ServiceHandler = class ServiceHandler extends OpenAPIHandler {
	/** The health. */
	health;
	/** Create the handler. */
	constructor(router, options) {
		ServiceHandler.#checkAccess(router, options, /* @__PURE__ */ new Set());
		const telemetry = new ServiceTelemetry("server");
		super(router, {
			...options,
			plugins: [new SmartCoercionPlugin({ schemaConverters: [schemaConverter] }), ...options.plugins ?? []],
			clientInterceptors: [
				({ path, next, context }) => telemetry.invoke(path, next, { "destack.caller.version": ServiceHandler.#observedRelease(context.request) }),
				(call) => {
					const caller = ServiceHandler.#requireRelease(call.context.request, options.service);
					const input = ServiceHandler.#convert(call.procedure, call.input, caller, options.service.package.version);
					return call.next({
						...call,
						input
					});
				},
				async ({ next, procedure, path, input, context, signal }) => {
					const { convert: _convert, ...access } = ProcedureMeta.of(procedure);
					return invokeProcedure({
						access,
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
	/** Answer probes, then dispatch. */
	async handle(...args) {
		const response = this.health.probe(args[0]);
		if (response) return {
			matched: true,
			response
		};
		return super.handle(...args);
	}
	/** Read the release a request speaks for telemetry, bounded to releases and two markers. */
	static #observedRelease(request) {
		const header = request.headers.get(VERSION_HEADER);
		return header === null ? "absent" : Version.safeParse(header).success ? header : "invalid";
	}
	/** Require a request's release to lie within the releases the service serves. */
	static #requireRelease(request, service) {
		const header = request.headers.get(VERSION_HEADER);
		const caller = header === null ? void 0 : Version.safeParse(header).data;
		const served = service.package.version;
		if (header === null) throw new ORPCError("BAD_REQUEST", { message: `service ${service.name} requires ${VERSION_HEADER}` });
		else if (caller === void 0) throw new ORPCError("BAD_REQUEST", { message: `invalid ${VERSION_HEADER}: ${header}` });
		else if (Version.compare(caller, served) > 0) throw new ORPCError("CONFLICT", { message: `service ${service.name} serves ${served}, the caller speaks ${caller}` });
		else if (service.since !== void 0 && Version.compare(caller, service.since) < 0) throw new ORPCError("CONFLICT", { message: `service ${service.name} no longer serves releases before ${service.since}` });
		return caller;
	}
	/** Convert an input of an earlier release through each later release's conversion, dropping the fields this release no longer declares. */
	static #convert(procedure, input, caller, served) {
		const convert = ProcedureMeta.of(procedure).convert ?? {};
		if (Version.between(Object.keys(convert), caller, served).length === 0) return input;
		if (typeof input !== "object" || input === null || Array.isArray(input)) throw new ORPCError("BAD_REQUEST", { message: "a converted input must be an object" });
		const converted = Expression$1.upgrade(convert, input, caller, served);
		const shape = procedure["~orpc"].inputSchema?.shape;
		return shape === void 0 ? converted : Object.fromEntries(Object.entries(converted).filter(([field]) => field in shape));
	}
	/** Require enforcement for every procedure. */
	static #checkAccess(router, options, ancestors) {
		if (isLazy(router)) throw new TypeError("service procedures must be declared before hosting");
		if (ancestors.has(router)) throw new TypeError("service routers must not contain cycles");
		if (isProcedure(router)) {
			const access = ProcedureMeta.of(router);
			if ((access.authentication !== "public" || access.permission !== null) && !options.authorize) throw new TypeError("protected procedures require authorization");
			if (access.audit !== false && !options.audit) throw new TypeError("audited procedures require audit recording");
			const { inputSchema, outputSchema } = router["~orpc"];
			for (const described of [inputSchema, outputSchema]) if (described instanceof ZodType) toJsonSchema(described);
		} else {
			ancestors.add(router);
			for (const child of Object.values(router)) ServiceHandler.#checkAccess(child, options, ancestors);
			ancestors.delete(router);
		}
	}
};
/** The verified context of a service call. */
var ServiceContext = class {
	/** The request. */
	request;
	/** The receiving package. */
	audience;
	/** The call's scope. */
	scope;
	/** The authenticated caller, or null. */
	caller;
	/** The installation's resource clients. */
	resources;
	/** The credential failure. */
	authenticationError;
	/** The server-generated request identifier. */
	requestId = crypto.randomUUID();
	/** Abort when the request closes or its caller lapses, ending handlers at their next consistent point. */
	signal;
	/** The digests of the presented capabilities. */
	capabilities;
	/** The watermarks the caller requires. */
	bookmark;
	/** The watermarks this request's writes reached. */
	observed = new Bookmark();
	/** The caller's authorization under the service's policies. */
	authorization;
	/** The object the call's permission was decided on. */
	target;
	/** Create the context of a request. */
	constructor(request, options) {
		const { audience, scope, caller, resources, access, authenticationError } = options;
		const capabilities = options.capabilities ?? [];
		if (caller !== null && authenticationError !== void 0) throw new TypeError("a failed authentication has no caller");
		this.request = request;
		this.audience = audience;
		this.scope = scope;
		this.caller = caller;
		this.resources = resources;
		this.authenticationError = authenticationError;
		this.capabilities = capabilities;
		this.bookmark = Bookmark.parse(request.headers.get(BOOKMARK_HEADER));
		this.authorization = access && new Authorization$1(access.authorizer, access.database, (scope) => {
			const context = this.access(scope);
			delegationChain(context);
			return context;
		});
		this.requireCaller = this.requireCaller.bind(this);
		this.access = this.access.bind(this);
		this.signal = caller === null ? this.request.signal : AbortSignal.any([this.request.signal, AbortSignal.timeout(Math.max(0, Math.ceil(caller.lapsesAt - Date.now())))]);
	}
	/** Require an authenticated caller, current as of its lapse at latest. */
	requireCaller() {
		if (this.authenticationError !== void 0) throw this.authenticationError;
		if (!this.caller) throw new ORPCError("UNAUTHORIZED", { message: "missing caller credential" });
		this.caller.requireCurrent(this.audience, this.caller.within(Date.now()), this.scope);
		return this.caller;
	}
	/** Read the access context at the current time, held at the caller's lapse once passed. */
	access(scope = this.scope) {
		if (this.authenticationError !== void 0) throw this.authenticationError;
		const context = this.caller ? this.caller.context(this.audience, this.caller.within(Date.now()), scope) : {
			subjects: [],
			attributes: {},
			now: Date.now()
		};
		return this.capabilities.length === 0 ? context : {
			...context,
			capabilities: this.capabilities
		};
	}
};
/** The longest timer delay, the largest signed 32 bit integer, in milliseconds. */
var MAX_TIMER_DELAY = 2 ** 31 - 1;
/** Wait a delay in milliseconds, as `scheduler.wait` does, rejecting once the signal aborts. */
function wait(delay, options = {}) {
	const signal = options.signal;
	return new Promise((resolve, reject) => {
		if (signal?.aborted) {
			reject(signal.reason);
			return;
		}
		let remaining = delay;
		let timer;
		const next = () => {
			const step = Math.min(remaining, MAX_TIMER_DELAY);
			remaining -= step;
			timer = setTimeout(() => {
				if (remaining > 0) next();
				else {
					signal?.removeEventListener("abort", abort);
					resolve();
				}
			}, step);
		};
		function abort() {
			clearTimeout(timer);
			reject(signal.reason);
		}
		signal?.addEventListener("abort", abort, { once: true });
		next();
	});
}
/** Temporal's default policy: a second, doubling, at most 100 seconds apart. */
var DEFAULT_POLICY = {
	initialInterval: 1e3,
	backoffCoefficient: 2,
	maximumInterval: 1e5
};
/** Retry waits of failing operations. */
var RetryPolicy = {
	/** Wait before the next attempt, resolving false once the signal aborts. */
	async pause(policy, failures, signal) {
		return wait(RetryPolicy.interval(policy, failures), { signal }).then(() => true, (error) => {
			if (!signal.aborted) throw error;
			return false;
		});
	},
	/** Complete a policy from the default. */
	of(changes = {}) {
		return {
			...DEFAULT_POLICY,
			...changes
		};
	},
	/** Read the wait before the next attempt, in milliseconds. */
	interval(policy, failures, random = Math.random) {
		const grown = policy.initialInterval * policy.backoffCoefficient ** (failures - 1);
		const interval = Math.min(grown, policy.maximumInterval);
		return policy.jitter === "full" ? random() * interval : interval;
	},
	/** Decide whether an operation retries. */
	isRetried(policy, failures) {
		return policy.maximumAttempts === void 0 || failures < policy.maximumAttempts;
	}
};
var __destackModule$33 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} });
/** The instance reconciling a controller's key, and until when. */
var controllerLease = defineTable("controller_lease", {
	/** The controller's name. */
	controller: text$1("controller").notNull(),
	/** The key. */
	key: text$1("key").notNull(),
	/** The instance holding the lease. */
	holder: text$1("holder").notNull(),
	/** The count of takeovers that fences writes. */
	epoch: integer("epoch").notNull(),
	/** The lapse time, in UTC epoch milliseconds. */
	expiresAt: integer("expires_at").notNull()
}, { constraints: (lease) => [primaryKey({
	name: "controller_lease_key",
	columns: [lease.controller, lease.key]
})] }, __destackModule$33);
/** The leases of instances over one database. */
var Leases = {
	/** Acquire or renew a lease, returning its epoch or the time the other holder's lease lapses. */
	async acquire(database, controller, key, holder, duration) {
		const now = Date.now();
		const [held] = await database.insert(controllerLease).values({
			controller,
			key,
			holder,
			epoch: 1,
			expiresAt: now + duration
		}).onConflictDoUpdate({
			target: [controllerLease.controller, controllerLease.key],
			set: {
				holder,
				expiresAt: now + duration,
				epoch: sql`CASE WHEN ${controllerLease.holder} = ${holder} THEN ${controllerLease.epoch} ELSE ${controllerLease.epoch} + 1 END`
			},
			setWhere: or(eq(controllerLease.holder, holder), lt(controllerLease.expiresAt, now))
		}).returning({ epoch: controllerLease.epoch });
		if (held !== void 0) return { epoch: held.epoch };
		const [other] = await database.select({ expiresAt: controllerLease.expiresAt }).from(controllerLease).where(and(eq(controllerLease.controller, controller), eq(controllerLease.key, key)));
		return { lapsesAt: other?.expiresAt ?? now };
	},
	/** Release a holder's leases. */
	async release(database, holder) {
		await database.delete(controllerLease).where(eq(controllerLease.holder, holder));
	}
};
var __destackModule$32 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} });
/** The control loop's spans. */
var { span: span$1 } = scope(__destackModule$32.package);
/** The retry of a failed reconciliation, doubling from a second up to 5 minutes. */
var RETRY = RetryPolicy.of({ maximumInterval: 3e5 });
/** Run controllers over one database. */
var ControlLoop = class {
	/** The database with the log that selects keys. */
	database;
	/** The controllers run. */
	controllers;
	/** Report a failed reconciliation. */
	#report;
	/** How a failed key retries. */
	#retry;
	/** The keys due, by controller and key, with the time each is due at. */
	#due = /* @__PURE__ */ new Map();
	/** The due keys, earliest first. */
	#queue = new DueQueue();
	/** The consecutive failures by controller and key. */
	#failures = /* @__PURE__ */ new Map();
	/** The keys reconciling now by controller with their abort controllers. */
	#running = /* @__PURE__ */ new Map();
	/** The running reconciliations. */
	#reconciling = /* @__PURE__ */ new Set();
	/** The lease holder and duration, when instances share the loop. */
	#lease;
	/** Wake the worker waiting for due work. */
	#wake = () => {};
	/** Create the loop. */
	constructor(database, controllers, options) {
		this.database = database;
		this.controllers = controllers;
		this.#report = options.report;
		this.#retry = {
			...RETRY,
			...options.retry
		};
		if (options.lease !== void 0) this.#lease = {
			holder: options.lease.holder,
			duration: options.lease.duration ?? 15e3
		};
		for (const controller of controllers) {
			this.#due.set(controller, /* @__PURE__ */ new Map());
			this.#failures.set(controller, /* @__PURE__ */ new Map());
			this.#running.set(controller, /* @__PURE__ */ new Map());
		}
	}
	/** Run the controllers until the signal aborts. */
	async run(signal) {
		await Promise.all([
			this.#follow(signal),
			this.#list(signal),
			this.#work(signal)
		]);
		if (this.#lease !== void 0) await Leases.release(this.database, this.#lease.holder);
	}
	/** Name a key due. */
	enqueue(controller, key, at = Date.now()) {
		const due = this.#due.get(controller);
		const current = due.get(key);
		if (current === void 0 || at < current) {
			due.set(key, at);
			this.#queue.push({
				controller,
				key,
				at
			});
		}
		this.#wake();
	}
	/** List every controller, then follow the watched logged tables. */
	async #follow(signal) {
		const tables = [...new Set(this.controllers.flatMap((controller) => controller.watches ?? []))].filter((table) => table[TABLE].retention !== "none");
		while (!signal.aborted) {
			const after = (await this.database.log.position()).sequence;
			for (const controller of this.controllers) for (const key of await controller.list()) this.enqueue(controller, key);
			if (tables.length === 0) return;
			try {
				for await (const page of this.database.log.follow({
					tables,
					after
				}, signal)) for (const controller of this.controllers) {
					const watched = controller.watches ?? [];
					const changes = page.changes.filter((change) => watched.includes(change.table));
					if (controller.mode === "follow") {
						if (changes.length > 0) await this.#relist(controller);
					} else for (const change of changes) for (const key of await controller.keys?.(change) ?? []) this.enqueue(controller, key);
				}
			} catch (error) {
				if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) throw error;
			}
		}
	}
	/** List the controllers of unlogged tables after each commit. */
	async #list(signal) {
		const listed = this.controllers.filter((controller) => (controller.watches ?? []).some((table) => table[TABLE].retention === "none"));
		if (listed.length === 0) return;
		await this.database.log.until(async () => {
			for (const controller of listed) if (controller.mode === "follow") await this.#relist(controller);
			else {
				const failures = this.#failures.get(controller);
				for (const key of await controller.list()) if (!failures.has(key)) this.enqueue(controller, key);
			}
			return false;
		}, signal);
	}
	/** Stop the keys a controller's list dropped, and enqueue the listed ones not running. */
	async #relist(controller) {
		const listed = new Set(await controller.list());
		const running = this.#running.get(controller);
		for (const [key, stop] of running) if (!listed.has(key)) stop.abort(/* @__PURE__ */ new Error(`${controller.name} no longer lists ${key}`));
		const due = this.#due.get(controller);
		for (const key of due.keys()) if (!listed.has(key)) due.delete(key);
		const failures = this.#failures.get(controller);
		for (const key of listed) if (!running.has(key) && !failures.has(key)) this.enqueue(controller, key);
	}
	/** Start due keys and sleep until more are due. */
	async #work(signal) {
		while (!signal.aborted) {
			const now = Date.now();
			const { runnable, due } = this.#take(now);
			if (runnable !== void 0) this.#start(runnable);
			else await this.#sleep(due === void 0 ? void 0 : Math.max(0, due - now), signal);
		}
		for (const running of this.#running.values()) for (const stop of running.values()) stop.abort(/* @__PURE__ */ new Error("the control loop stopped"));
		await Promise.all(this.#reconciling);
	}
	/** Take the earliest runnable key, or the next due time. */
	#take(now) {
		const held = [];
		let found = {};
		for (let next = this.#next(); next !== void 0; next = this.#next()) {
			const running = this.#running.get(next.controller);
			const isFull = running.size >= (next.controller.concurrency ?? 1);
			if (next.at > now) {
				found = { due: next.at };
				break;
			} else if (isFull || running.has(next.key)) held.push(this.#queue.pop());
			else {
				this.#queue.pop();
				this.#due.get(next.controller).delete(next.key);
				found = { runnable: next };
				break;
			}
		}
		for (const entry of held) this.#queue.push(entry);
		return found;
	}
	/** Start a reconciliation. */
	#start(work) {
		const running = this.#running.get(work.controller);
		const stop = new AbortController();
		running.set(work.key, stop);
		const reconciling = this.#reconcile(work, stop.signal).finally(() => {
			running.delete(work.key);
			this.#reconciling.delete(reconciling);
			this.#wake();
		});
		this.#reconciling.add(reconciling);
	}
	/** Reconcile one key until settled, or follow it until stopped, retrying failures. */
	async #reconcile(work, stopped) {
		const { controller, key } = work;
		const failures = this.#failures.get(controller);
		try {
			const delay = await this.#leased(work, stopped, async (reconciliation) => {
				if (controller.mode === "follow") {
					await controller.reconcile(key, reconciliation);
					if (!reconciliation.signal.aborted) throw new Error(`${controller.name} stopped following ${key} unasked`);
					return stopped.aborted ? void 0 : 0;
				}
				return span$1("controller.reconcile", {
					"destack.controller": controller.name,
					"destack.key": key
				}, () => controller.reconcile(key, reconciliation));
			});
			failures.delete(work.key);
			if (delay !== void 0) this.enqueue(work.controller, work.key, Date.now() + delay);
		} catch (error) {
			if (stopped.aborted) {
				failures.delete(work.key);
				return;
			}
			const failed = (failures.get(work.key) ?? 0) + 1;
			failures.set(work.key, failed);
			this.#report(work.controller, work.key, error);
			this.enqueue(work.controller, work.key, Date.now() + RetryPolicy.interval(this.#retry, failed));
		}
	}
	/** Reconcile a key under this instance's lease, stopped by the loop or a lost lease. */
	async #leased(work, stopped, reconcile) {
		const options = this.#lease;
		if (options === void 0) return reconcile({ signal: stopped });
		const { holder, duration } = options;
		const acquire = () => Leases.acquire(this.database, work.controller.name, work.key, holder, duration);
		const acquired = await acquire();
		if ("lapsesAt" in acquired) return Math.max(0, acquired.lapsesAt - Date.now());
		const lost = new AbortController();
		const renewing = setInterval(() => {
			acquire().then((renewed) => {
				if ("lapsesAt" in renewed) lost.abort(/* @__PURE__ */ new Error(`lease of ${work.key} lost`));
			}, (error) => lost.abort(error));
		}, duration / 3);
		try {
			const signal = AbortSignal.any([stopped, lost.signal]);
			return await reconcile({
				epoch: acquired.epoch,
				signal
			});
		} finally {
			clearInterval(renewing);
		}
	}
	/** Find the earliest due key. */
	#next() {
		for (let next = this.#queue.peek(); next !== void 0; next = this.#queue.peek()) {
			if (this.#due.get(next.controller).get(next.key) === next.at) return next;
			this.#queue.pop();
		}
	}
	/** Sleep for a delay or until woken. */
	async #sleep(delay, signal) {
		const woken = new AbortController();
		this.#wake = () => woken.abort();
		const until = AbortSignal.any([signal, woken.signal]);
		try {
			await wait(delay ?? Number.POSITIVE_INFINITY, { signal: until });
		} catch (error) {
			if (!until.aborted) throw error;
		} finally {
			this.#wake = () => {};
		}
	}
};
/** Due keys in a binary min-heap by due time. */
var DueQueue = class {
	/** The heap. */
	#heap = [];
	/** Read the earliest entry. */
	peek() {
		return this.#heap[0];
	}
	/** Add an entry. */
	push(entry) {
		const heap = this.#heap;
		heap.push(entry);
		let index = heap.length - 1;
		while (index > 0) {
			const parent = index - 1 >> 1;
			if (heap[parent].at <= heap[index].at) break;
			[heap[parent], heap[index]] = [heap[index], heap[parent]];
			index = parent;
		}
	}
	/** Remove the earliest entry. */
	pop() {
		const heap = this.#heap;
		const earliest = heap[0];
		const last = heap.pop();
		if (heap.length === 0 || last === void 0) return earliest;
		heap[0] = last;
		let index = 0;
		while (true) {
			const left = 2 * index + 1;
			const right = left + 1;
			let smallest = index;
			if (left < heap.length && heap[left].at < heap[smallest].at) smallest = left;
			if (right < heap.length && heap[right].at < heap[smallest].at) smallest = right;
			if (smallest === index) return earliest;
			[heap[smallest], heap[index]] = [heap[index], heap[smallest]];
			index = smallest;
		}
	}
};
/** A hosted HTTP service. */
var Server = class Server {
	/** The health. */
	health;
	/** The server options. */
	#options;
	/** The HTTP handler. */
	#handler;
	/** The accepted requests by their controllers. */
	#requests = /* @__PURE__ */ new Map();
	/** Resolve when accepted requests complete. */
	#drained = Promise.withResolvers();
	/** Resolve when draining and disposal finish. */
	#stopped = Promise.withResolvers();
	/** The shutdown. */
	#closing;
	/** Stop the controllers after draining. */
	#reconciling = new AbortController();
	/** The running controllers. */
	#controlled;
	/** Wait until the server stopped. */
	get stopped() {
		return this.#stopped.promise;
	}
	/** Create the server. */
	constructor(options) {
		for (const callback of ["authenticate", "authorizeHost"]) if (typeof options[callback] !== "function") throw new TypeError(`${callback} must be configured before starting a server`);
		if (!Number.isInteger(options.drainTimeout) || options.drainTimeout <= 0 || options.drainTimeout > MAX_TIMER_DELAY) throw new RangeError("drain timeout must be a positive integer within the runtime timer limit");
		Server.#requirePolicies(options.router, options.access);
		this.#options = options;
		this.health = options.health;
		this.#handler = new ServiceHandler(options.router, {
			...options,
			authorize: (call) => Server.#authorize(call, options)
		});
		const controllers = options.controllers ?? [];
		if (controllers.length > 0 && options.access === void 0) throw new TypeError("a service running controllers needs the database of its access");
		else if (controllers.length > 0) {
			const loop = new ControlLoop(options.access.database, controllers, {
				report: reportReconciliation,
				...options.instance === void 0 ? {} : { lease: { holder: options.instance } }
			});
			this.#controlled = loop.run(this.#reconciling.signal);
			this.#controlled.catch(reportError);
		}
	}
	/** Start the server. */
	static start(options) {
		const server = new Server(options);
		server.health.set("serving");
		return server;
	}
	/** Serve a request. */
	async fetch(request) {
		const probe = this.health.probe(request);
		if (probe) return this.#headers(probe);
		if (this.#closing || this.health.status !== "serving") return this.#headers(new Response(null, { status: 503 }));
		const controller = new AbortController();
		const signal = AbortSignal.any([request.signal, controller.signal]);
		this.#requests.set(controller, request);
		try {
			const accepted = copyRequest(request, { signal });
			this.#requests.set(controller, accepted);
			const context = await Server.#authenticate(accepted, this.#options);
			signal.throwIfAborted();
			let response = await this.#options.route?.(accepted, context);
			if (response === void 0) {
				const result = await this.#handler.handle(accepted, { context });
				response = result.matched ? result.response : new Response(null, { status: 404 });
				response = attachBookmark(response, context.observed);
			}
			return this.#respond(this.#headers(response), controller, signal);
		} catch (error) {
			this.#finish(controller);
			if (signal.aborted) throw error;
			const failure = reportError(error);
			return this.#headers(Response.json(failure.toJSON(), { status: failure.status }));
		}
	}
	/** Refuse new work, drain requests and dispose resources, once. */
	close() {
		this.#closing ??= this.#close();
		return this.#closing;
	}
	/** Apply the response headers. */
	#headers(response) {
		if (!this.#options.responseHeaders) return response;
		const headers = new Headers(response.headers);
		for (const [name, value] of new Headers(this.#options.responseHeaders)) headers.set(name, value);
		return new Response(response.body, {
			status: response.status,
			statusText: response.statusText,
			headers
		});
	}
	/** Close the service. */
	async [Symbol.asyncDispose]() {
		await this.close();
	}
	/** Authenticate a request and build its context. */
	static async #authenticate(request, options) {
		let caller = null;
		let authenticationError;
		try {
			const authenticated = await options.authenticate(request);
			authenticated?.requireCurrent(options.audience, Date.now(), options.scope ?? authenticated.authentication.scope);
			caller = authenticated;
		} catch (error) {
			authenticationError = error ?? new ORPCError("UNAUTHORIZED", { message: "authentication failed" });
		}
		const presented = (request.headers.get("destack-capability") ?? "").split(",").map((secret) => secret.trim()).filter((secret) => secret.length > 0);
		const capabilities = await Promise.all(presented.map((secret) => Capability.digest(secret))).catch((error) => {
			throw new ORPCError("UNAUTHORIZED", {
				message: "invalid capability",
				cause: error
			});
		});
		return new ServiceContext(request, {
			audience: options.audience,
			scope: options.scope ?? caller?.authentication.scope,
			caller,
			resources: options.resources,
			access: options.access,
			authenticationError,
			capabilities
		});
	}
	/** Authorize a call. */
	static async #authorize(call, options) {
		if (call.access.authentication !== "public") call.context.requireCaller();
		call.context.authorization?.renew();
		const permission = call.access.permission;
		if (permission !== null) {
			const target = await options.access.target(call);
			await call.context.authorization.require(permission, target);
			call.context.target = target;
		} else delegationChain(call.context.access());
		await options.authorizeHost(call);
	}
	/** Hold a request until its response body settles. */
	#respond(response, controller, signal) {
		if (!response.body) {
			this.#finish(controller);
			return response;
		}
		const reader = response.body.getReader();
		const finish = () => {
			signal.removeEventListener("abort", abort);
			this.#finish(controller);
		};
		const abort = () => {
			reader.cancel(signal.reason).catch((error) => {
				reportError(error);
				responseError = error;
			}).finally(finish);
		};
		let responseError;
		signal.addEventListener("abort", abort, { once: true });
		if (signal.aborted) abort();
		const body = new ReadableStream({
			async pull(stream) {
				try {
					const next = await reader.read();
					if (responseError !== void 0) throw responseError;
					signal.throwIfAborted();
					if (next.done) {
						finish();
						stream.close();
					} else stream.enqueue(next.value);
				} catch (error) {
					finish();
					stream.error(error);
				}
			},
			async cancel(reason) {
				try {
					await reader.cancel(reason);
				} finally {
					finish();
				}
			}
		});
		return new Response(body, {
			status: response.status,
			statusText: response.statusText,
			headers: response.headers
		});
	}
	/** Release a request. */
	#finish(controller) {
		this.#requests.delete(controller);
		if (this.#closing && !this.#requests.size) this.#drained.resolve();
	}
	/** Drain within the deadline. */
	async #close() {
		this.health.set("draining");
		if (!this.#requests.size) this.#drained.resolve();
		const completion = this.#drained.promise.then(() => {
			this.health.set("stopped");
			this.#stopped.resolve();
		});
		const deadline = Promise.withResolvers();
		const timer = setTimeout(() => {
			const error = new DOMException("service drain deadline exceeded", "TimeoutError");
			for (const controller of this.#requests.keys()) controller.abort(error);
			deadline.reject(error);
		}, this.#options.drainTimeout);
		try {
			await Promise.race([completion, deadline.promise]);
		} finally {
			clearTimeout(timer);
			this.#reconciling.abort();
			await this.#controlled;
		}
	}
	/** Require a policy for every procedure permission. */
	static #requirePolicies(router, access) {
		if (isProcedure(router)) {
			const permission = ProcedureMeta.of(router).permission;
			if (permission === null) return;
			if (access?.target === void 0) throw new TypeError(`procedures requiring ${permission.name} need service access with targets`);
			if (!Object.hasOwn(access.authorizer.policy(permission).definition.permissions, permission.name)) throw new TypeError(`no policy declares ${permission.type}.${permission.name}`);
		} else if (router !== null && typeof router === "object") for (const child of Object.values(router)) Server.#requirePolicies(child, access);
	}
};
/** Attach the request's watermarks to its response. */
function attachBookmark(response, observed) {
	if (observed.watermarks.length === 0) return response;
	const headers = new Headers(response.headers);
	headers.set(BOOKMARK_HEADER, observed.format());
	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers
	});
}
/** A current value with change notifications. */
var Observable = class {
	/** Resubscribe after each completed subscription until cancelled. */
	static async *observe(open, signal) {
		while (!signal.aborted) {
			let isReceived = false;
			for await (const value of await open(signal)) {
				isReceived = true;
				yield value;
			}
			if (!isReceived && !signal.aborted) throw new Error("snapshot subscription closed without a value");
		}
	}
	/** Yield the combined latest values of several streams, once each has one. */
	static async *latest(streams, combine) {
		const values = /* @__PURE__ */ new Map();
		const next = (index) => streams[index].next().then((result) => ({
			index,
			result
		}));
		const pending = new Map(streams.map((_, index) => [index, next(index)]));
		while (pending.size > 0) {
			const { index, result } = await Promise.race(pending.values());
			if (result.done) pending.delete(index);
			else {
				values.set(index, result.value);
				pending.set(index, next(index));
				if (values.size === streams.length) yield combine(streams.map((_, position) => values.get(position)));
			}
		}
	}
	/** The latest value. */
	#value;
	/** The current change number. */
	#revision = 0;
	/** Whether the producer has stopped. */
	#isClosed = false;
	/** The waiting subscribers. */
	#subscribers = /* @__PURE__ */ new Set();
	/** Create the observable. */
	constructor(value) {
		this.#value = value;
	}
	/** The latest value. */
	get value() {
		return this.#value;
	}
	/** Set the value. */
	set(value) {
		if (this.#isClosed) throw new Error("observable is closed");
		this.#value = value;
		this.#revision++;
		for (const wake of this.#subscribers) wake();
	}
	/** Close the observable. */
	close() {
		this.#isClosed = true;
		for (const wake of this.#subscribers) wake();
	}
	/** Yield the value, then the latest value after each change. */
	async *watch(signal) {
		let revision = -1;
		while (!this.#isClosed && !signal?.aborted) {
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
/** The health of a service. */
var Health = class {
	/** The declared service name. */
	name;
	/** The current status. */
	#status = new Observable("starting");
	/** Create the health. */
	constructor(name) {
		this.name = name;
	}
	/** Read the current status. */
	get status() {
		return this.#status.value;
	}
	/** Set the status. */
	set(status) {
		if (status !== this.status) this.#status.set(status);
	}
	/** Describe the health. */
	check() {
		return {
			name: this.name,
			status: this.status
		};
	}
	/** Yield the health and its changes. */
	async *watch(signal) {
		for await (const status of this.#status.watch(signal)) {
			yield {
				name: this.name,
				status
			};
			if (status === "draining" || status === "stopped") return;
		}
	}
	/** Answer liveness and readiness probes. */
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
/** The readiness of a service. */
var HealthStatus = _enum([
	"starting",
	"serving",
	"not-serving",
	"draining",
	"stopped"
]);
strictObject({
	/** The service name. */
	name: string().min(1),
	/** The status. */
	status: HealthStatus
});
/** A running workload instance. */
var WorkloadInstance = class WorkloadInstance {
	/** The services and their servers by declaration key. */
	#services = /* @__PURE__ */ new Map();
	/** The trigger handlers by key. */
	#triggers = /* @__PURE__ */ new Map();
	/** The shutdown controller. */
	#controller = new AbortController();
	/** The resources released after draining. */
	#cleanup = new AsyncDisposableStack();
	/** The shutdown. */
	#closing;
	/** Start a workload. */
	static async start(workload, options) {
		const instance = new WorkloadInstance();
		try {
			const implementation = await workload.start({
				resources: options.resources,
				history: options.history,
				...options.replicas === void 0 ? {} : { replicas: options.replicas },
				signal: instance.#controller.signal,
				shutdown: () => instance.shutdown(),
				defer: (dispose) => instance.#cleanup.defer(dispose)
			});
			for (const service of implementation.services) {
				instance.signal.throwIfAborted();
				const key = keyOf(service.service);
				if (instance.#services.has(key)) throw new TypeError(`duplicate workload service: ${key}`);
				const server = Server.start({
					...service,
					...options.service(service.service),
					health: new Health(service.service.name),
					resources: options.resources
				});
				instance.#services.set(key, {
					service: service.service,
					server
				});
			}
			for (const handler of implementation.triggers ?? []) {
				const key = triggerKey(handler.trigger);
				if (instance.#triggers.has(key)) throw new TypeError(`duplicate workload ${key}`);
				instance.#triggers.set(key, handler);
			}
			instance.signal.throwIfAborted();
		} catch (error) {
			try {
				await instance.close();
			} catch (cleanup) {
				throw new AggregateError([error, cleanup], "workload startup and cleanup failed");
			}
			throw error;
		}
		return instance;
	}
	/** The shutdown signal. */
	get signal() {
		return this.#controller.signal;
	}
	/** The implemented services, in workload order. */
	get services() {
		return [...this.#services.values()].map((served) => served.service);
	}
	/** The implemented triggers, in workload order. */
	get triggers() {
		return [...this.#triggers.values()].map((handler) => handler.trigger);
	}
	/** Report whether the workload implements a service. */
	has(service) {
		return this.#services.has(keyOf(service));
	}
	/** Request shutdown. */
	shutdown() {
		this.#controller.abort();
	}
	/** Dispatch a request to a service. */
	fetch(service, request) {
		const server = this.#services.get(keyOf(service))?.server;
		if (!server) throw new ORPCError("NOT_FOUND", { message: `unknown workload service: ${keyOf(service)}` });
		return server.fetch(request);
	}
	/** Deliver one trigger event to its handler. */
	deliver(trigger, event, signal) {
		const key = triggerKey(trigger);
		const handler = this.#triggers.get(key);
		if (!handler) throw new ORPCError("NOT_FOUND", { message: `unknown workload ${key}` });
		return handler.handle(event, AbortSignal.any([signal, this.signal]));
	}
	/** Close the workload. */
	close() {
		this.shutdown();
		this.#closing ??= this.#close();
		return this.#closing;
	}
	/** Close the workload. */
	async [Symbol.asyncDispose]() {
		await this.close();
	}
	/** Close every service and resource, reporting all failures. */
	async #close() {
		const servers = [...this.#services.values()].map((served) => served.server);
		const errors = (await Promise.allSettled(servers.map((server) => server.close()))).flatMap((result) => result.status === "rejected" ? [result.reason] : []);
		await Promise.all(servers.map((server) => server.stopped));
		try {
			await this.#cleanup.disposeAsync();
		} catch (error) {
			errors.push(error);
		}
		if (errors.length) throw new AggregateError(errors, "workload shutdown failed");
	}
};
/** Key a declaration by its package and name. */
function keyOf(declaration) {
	const { packageId, name } = reference$1(declaration);
	return `${packageId}/${name}`;
}
/** Key a trigger by its kind, package and name. */
function triggerKey(trigger) {
	return `${trigger.kind}: ${keyOf(trigger)}`;
}
/** The first line a host writes to a runner's input, starting one workload of an installation. */
var WorkloadStart = defineSchema(strictObject({
	/** The instance, the holder name of its leases. */
	instance: identifier("instance"),
	/** The space the installation serves. */
	scope: identifier("space"),
	/** The installation the workload runs. */
	installation: identifier("installation"),
	/** The resources the installation binds, by the package's resource name. */
	bindings: record$1(DeclarationName, ResourceBinding),
	/** The secret the host and the runner prove each other's requests with. */
	secret: string().min(1),
	/** The host's egress, below which the workload reaches addresses as its installation. */
	egress: url(),
	/** The share of traces the workload keeps beside every failed or slow one, from 0 to 1. */
	sampling: number().min(0).max(1)
}));
defineSchema(strictObject({ 
/** The loopback port serving the package's service below its mount. */
port: number().int().min(1).max(65535) }));
/** Resource clients authorised by the host for one invocation or local operation. */
var ResourceContext = class {
	/** Clients indexed by their imported declaration objects. */
	#clients = /* @__PURE__ */ new WeakMap();
	/** Bind an authorised client before invoking application code. */
	bind(resource, client) {
		if (this.#clients.has(resource)) throw new ResourceError("ALREADY_BOUND", `resource already bound: ${resource.name}`);
		this.#clients.set(resource, client);
		return this;
	}
	/** Return the client selected for this declaration. */
	get(resource) {
		if (!this.#clients.has(resource)) throw new ResourceError("NOT_BOUND", `resource is not bound: ${resource.name}`);
		return this.#clients.get(resource);
	}
};
/**
* AggregationTemporality indicates the way additive quantities are expressed.
*/
var AggregationTemporality;
(function(AggregationTemporality) {
	AggregationTemporality[AggregationTemporality["DELTA"] = 0] = "DELTA";
	AggregationTemporality[AggregationTemporality["CUMULATIVE"] = 1] = "CUMULATIVE";
})(AggregationTemporality || (AggregationTemporality = {}));
/**
* Supported types of metric instruments.
*/
var InstrumentType;
(function(InstrumentType) {
	InstrumentType["COUNTER"] = "COUNTER";
	InstrumentType["GAUGE"] = "GAUGE";
	InstrumentType["HISTOGRAM"] = "HISTOGRAM";
	InstrumentType["UP_DOWN_COUNTER"] = "UP_DOWN_COUNTER";
	InstrumentType["OBSERVABLE_COUNTER"] = "OBSERVABLE_COUNTER";
	InstrumentType["OBSERVABLE_GAUGE"] = "OBSERVABLE_GAUGE";
	InstrumentType["OBSERVABLE_UP_DOWN_COUNTER"] = "OBSERVABLE_UP_DOWN_COUNTER";
})(InstrumentType || (InstrumentType = {}));
/**
* The aggregated point data type.
*/
var DataPointType;
(function(DataPointType) {
	/**
	* A histogram data point contains a histogram statistics of collected
	* values with a list of explicit bucket boundaries and statistics such
	* as min, max, count, and sum of all collected values.
	*/
	DataPointType[DataPointType["HISTOGRAM"] = 0] = "HISTOGRAM";
	/**
	* An exponential histogram data point contains a histogram statistics of
	* collected values where bucket boundaries are automatically calculated
	* using an exponential function, and statistics such as min, max, count,
	* and sum of all collected values.
	*/
	DataPointType[DataPointType["EXPONENTIAL_HISTOGRAM"] = 1] = "EXPONENTIAL_HISTOGRAM";
	/**
	* A gauge metric data point has only a single numeric value.
	*/
	DataPointType[DataPointType["GAUGE"] = 2] = "GAUGE";
	/**
	* A sum metric data point has a single numeric value and a
	* monotonicity-indicator.
	*/
	DataPointType[DataPointType["SUM"] = 3] = "SUM";
})(DataPointType || (DataPointType = {}));
/**
* Converting the unordered attributes into unique identifier string.
* @param attributes user provided unordered Attributes.
*/
function hashAttributes(attributes) {
	let keys = Object.keys(attributes);
	if (keys.length === 0) return "";
	keys = keys.sort();
	return JSON.stringify(keys.map((key) => [key, attributes[key]]));
}
/**
* Converting the instrumentation scope object to a unique identifier string.
* @param instrumentationScope
*/
function instrumentationScopeId(instrumentationScope) {
	return `${instrumentationScope.name}:${instrumentationScope.version ?? ""}:${instrumentationScope.schemaUrl ?? ""}`;
}
/**
* Error that is thrown on timeouts.
*/
var TimeoutError = class TimeoutError extends Error {
	constructor(message) {
		super(message);
		Object.setPrototypeOf(this, TimeoutError.prototype);
	}
};
/**
* Adds a timeout to a promise and rejects if the specified timeout has elapsed. Also rejects if the specified promise
* rejects, and resolves if the specified promise resolves.
*
* <p> NOTE: this operation will continue even after it throws a {@link TimeoutError}.
*
* @param promise promise to use with timeout.
* @param timeout the timeout in milliseconds until the returned promise is rejected.
*/
function callWithTimeout(promise, timeout) {
	let timeoutHandle;
	const timeoutPromise = new Promise(function timeoutFunction(_resolve, reject) {
		timeoutHandle = setTimeout(function timeoutHandler() {
			reject(new TimeoutError("Operation timed out."));
		}, timeout);
	});
	return Promise.race([promise, timeoutPromise]).then((result) => {
		clearTimeout(timeoutHandle);
		return result;
	}, (reason) => {
		clearTimeout(timeoutHandle);
		throw reason;
	});
}
function setEquals(lhs, rhs) {
	if (lhs.size !== rhs.size) return false;
	for (const item of lhs) if (!rhs.has(item)) return false;
	return true;
}
/**
* Binary search the sorted array to the find upper bound for the value.
* @param arr
* @param value
* @returns
*/
function binarySearchUB(arr, value) {
	let lo = 0;
	let hi = arr.length - 1;
	let ret = arr.length;
	while (hi >= lo) {
		const mid = lo + Math.trunc((hi - lo) / 2);
		if (arr[mid] < value) lo = mid + 1;
		else {
			ret = mid;
			hi = mid - 1;
		}
	}
	return ret;
}
function equalsCaseInsensitive(lhs, rhs) {
	return lhs.toLowerCase() === rhs.toLowerCase();
}
/** The kind of aggregator. */
var AggregatorKind;
(function(AggregatorKind) {
	AggregatorKind[AggregatorKind["DROP"] = 0] = "DROP";
	AggregatorKind[AggregatorKind["SUM"] = 1] = "SUM";
	AggregatorKind[AggregatorKind["LAST_VALUE"] = 2] = "LAST_VALUE";
	AggregatorKind[AggregatorKind["HISTOGRAM"] = 3] = "HISTOGRAM";
	AggregatorKind[AggregatorKind["EXPONENTIAL_HISTOGRAM"] = 4] = "EXPONENTIAL_HISTOGRAM";
})(AggregatorKind || (AggregatorKind = {}));
/** Basic aggregator for None which keeps no recorded value. */
var DropAggregator = class {
	kind = AggregatorKind.DROP;
	createAccumulation() {}
	merge(_previous, _delta) {}
	diff(_previous, _current) {}
	toMetricData(_descriptor, _aggregationTemporality, _accumulationByAttributes, _endTime) {}
};
function createNewEmptyCheckpoint(boundaries) {
	const counts = boundaries.map(() => 0);
	counts.push(0);
	return {
		buckets: {
			boundaries,
			counts
		},
		sum: 0,
		count: 0,
		hasMinMax: false,
		min: Infinity,
		max: -Infinity
	};
}
var HistogramAccumulation = class {
	startTime;
	_boundaries;
	_recordMinMax;
	_current;
	constructor(startTime, boundaries, recordMinMax = true, current = createNewEmptyCheckpoint(boundaries)) {
		this.startTime = startTime;
		this._boundaries = boundaries;
		this._recordMinMax = recordMinMax;
		this._current = current;
	}
	record(value) {
		if (Number.isNaN(value)) return;
		this._current.count += 1;
		this._current.sum += value;
		if (this._recordMinMax) {
			this._current.min = Math.min(value, this._current.min);
			this._current.max = Math.max(value, this._current.max);
			this._current.hasMinMax = true;
		}
		const idx = binarySearchUB(this._boundaries, value);
		this._current.buckets.counts[idx] += 1;
	}
	setStartTime(startTime) {
		this.startTime = startTime;
	}
	toPointValue() {
		return this._current;
	}
};
/**
* Basic aggregator which observes events and counts them in pre-defined buckets
* and provides the total sum and count of all observations.
*/
var HistogramAggregator = class {
	kind = AggregatorKind.HISTOGRAM;
	_boundaries;
	_recordMinMax;
	/**
	* @param _boundaries sorted upper bounds of recorded values.
	* @param _recordMinMax If set to true, min and max will be recorded. Otherwise, min and max will not be recorded.
	*/
	constructor(boundaries, recordMinMax) {
		this._boundaries = boundaries;
		this._recordMinMax = recordMinMax;
	}
	createAccumulation(startTime) {
		return new HistogramAccumulation(startTime, this._boundaries, this._recordMinMax);
	}
	/**
	* Return the result of the merge of two histogram accumulations. As long as one Aggregator
	* instance produces all Accumulations with constant boundaries we don't need to worry about
	* merging accumulations with different boundaries.
	*/
	merge(previous, delta) {
		const previousValue = previous.toPointValue();
		const deltaValue = delta.toPointValue();
		const previousCounts = previousValue.buckets.counts;
		const deltaCounts = deltaValue.buckets.counts;
		const mergedCounts = new Array(previousCounts.length);
		for (let idx = 0; idx < previousCounts.length; idx++) mergedCounts[idx] = previousCounts[idx] + deltaCounts[idx];
		let min = Infinity;
		let max = -Infinity;
		if (this._recordMinMax) {
			if (previousValue.hasMinMax && deltaValue.hasMinMax) {
				min = Math.min(previousValue.min, deltaValue.min);
				max = Math.max(previousValue.max, deltaValue.max);
			} else if (previousValue.hasMinMax) {
				min = previousValue.min;
				max = previousValue.max;
			} else if (deltaValue.hasMinMax) {
				min = deltaValue.min;
				max = deltaValue.max;
			}
		}
		return new HistogramAccumulation(previous.startTime, previousValue.buckets.boundaries, this._recordMinMax, {
			buckets: {
				boundaries: previousValue.buckets.boundaries,
				counts: mergedCounts
			},
			count: previousValue.count + deltaValue.count,
			sum: previousValue.sum + deltaValue.sum,
			hasMinMax: this._recordMinMax && (previousValue.hasMinMax || deltaValue.hasMinMax),
			min,
			max
		});
	}
	/**
	* Returns a new DELTA aggregation by comparing two cumulative measurements.
	*/
	diff(previous, current) {
		const previousValue = previous.toPointValue();
		const currentValue = current.toPointValue();
		const previousCounts = previousValue.buckets.counts;
		const currentCounts = currentValue.buckets.counts;
		const diffedCounts = new Array(previousCounts.length);
		for (let idx = 0; idx < previousCounts.length; idx++) diffedCounts[idx] = currentCounts[idx] - previousCounts[idx];
		return new HistogramAccumulation(current.startTime, previousValue.buckets.boundaries, this._recordMinMax, {
			buckets: {
				boundaries: previousValue.buckets.boundaries,
				counts: diffedCounts
			},
			count: currentValue.count - previousValue.count,
			sum: currentValue.sum - previousValue.sum,
			hasMinMax: false,
			min: Infinity,
			max: -Infinity
		});
	}
	toMetricData(descriptor, aggregationTemporality, accumulationByAttributes, endTime) {
		return {
			descriptor,
			aggregationTemporality,
			dataPointType: DataPointType.HISTOGRAM,
			dataPoints: accumulationByAttributes.map(([attributes, accumulation]) => {
				const pointValue = accumulation.toPointValue();
				const allowsNegativeValues = descriptor.type === InstrumentType.GAUGE || descriptor.type === InstrumentType.UP_DOWN_COUNTER || descriptor.type === InstrumentType.OBSERVABLE_GAUGE || descriptor.type === InstrumentType.OBSERVABLE_UP_DOWN_COUNTER;
				return {
					attributes,
					startTime: accumulation.startTime,
					endTime,
					value: {
						min: pointValue.hasMinMax ? pointValue.min : void 0,
						max: pointValue.hasMinMax ? pointValue.max : void 0,
						sum: !allowsNegativeValues ? pointValue.sum : void 0,
						buckets: pointValue.buckets,
						count: pointValue.count
					}
				};
			})
		};
	}
};
var Buckets = class Buckets {
	backing;
	indexBase;
	indexStart;
	indexEnd;
	/**
	* The term index refers to the number of the exponential histogram bucket
	* used to determine its boundaries. The lower boundary of a bucket is
	* determined by base ** index and the upper boundary of a bucket is
	* determined by base ** (index + 1). index values are signed to account
	* for values less than or equal to 1.
	*
	* indexBase is the index of the 0th position in the
	* backing array, i.e., backing[0] is the count
	* in the bucket with index `indexBase`.
	*
	* indexStart is the smallest index value represented
	* in the backing array.
	*
	* indexEnd is the largest index value represented in
	* the backing array.
	*/
	constructor(backing = new BucketsBacking(), indexBase = 0, indexStart = 0, indexEnd = 0) {
		this.backing = backing;
		this.indexBase = indexBase;
		this.indexStart = indexStart;
		this.indexEnd = indexEnd;
	}
	/**
	* Offset is the bucket index of the smallest entry in the counts array
	* @returns {number}
	*/
	get offset() {
		return this.indexStart;
	}
	/**
	* Buckets is a view into the backing array.
	* @returns {number}
	*/
	get length() {
		if (this.backing.length === 0) return 0;
		if (this.indexEnd === this.indexStart && this.at(0) === 0) return 0;
		return this.indexEnd - this.indexStart + 1;
	}
	/**
	* An array of counts, where count[i] carries the count
	* of the bucket at index (offset+i).  count[i] is the count of
	* values greater than base^(offset+i) and less than or equal to
	* base^(offset+i+1).
	* @returns {number} The logical counts based on the backing array
	*/
	counts() {
		return Array.from({ length: this.length }, (_, i) => this.at(i));
	}
	/**
	* At returns the count of the bucket at a position in the logical
	* array of counts.
	* @param position
	* @returns {number}
	*/
	at(position) {
		const bias = this.indexBase - this.indexStart;
		if (position < bias) position += this.backing.length;
		position -= bias;
		return this.backing.countAt(position);
	}
	/**
	* incrementBucket increments the backing array index by `increment`
	* @param bucketIndex
	* @param increment
	*/
	incrementBucket(bucketIndex, increment) {
		this.backing.increment(bucketIndex, increment);
	}
	/**
	* decrementBucket decrements the backing array index by `decrement`
	* if decrement is greater than the current value, it's set to 0.
	* @param bucketIndex
	* @param decrement
	*/
	decrementBucket(bucketIndex, decrement) {
		this.backing.decrement(bucketIndex, decrement);
	}
	/**
	* trim removes leading and / or trailing zero buckets (which can occur
	* after diffing two histos) and rotates the backing array so that the
	* smallest non-zero index is in the 0th position of the backing array
	*/
	trim() {
		for (let i = 0; i < this.length; i++) if (this.at(i) !== 0) {
			this.indexStart += i;
			break;
		} else if (i === this.length - 1) {
			this.indexStart = this.indexEnd = this.indexBase = 0;
			return;
		}
		for (let i = this.length - 1; i >= 0; i--) if (this.at(i) !== 0) {
			this.indexEnd -= this.length - i - 1;
			break;
		}
		this._rotate();
	}
	/**
	* downscale first rotates, then collapses 2**`by`-to-1 buckets.
	* @param by
	*/
	downscale(by) {
		this._rotate();
		const size = 1 + this.indexEnd - this.indexStart;
		const each = 1 << by;
		let inpos = 0;
		let outpos = 0;
		for (let pos = this.indexStart; pos <= this.indexEnd;) {
			let mod = pos % each;
			if (mod < 0) mod += each;
			for (let i = mod; i < each && inpos < size; i++) {
				this._relocateBucket(outpos, inpos);
				inpos++;
				pos++;
			}
			outpos++;
		}
		this.indexStart >>= by;
		this.indexEnd >>= by;
		this.indexBase = this.indexStart;
	}
	/**
	* Clone returns a deep copy of Buckets
	* @returns {Buckets}
	*/
	clone() {
		return new Buckets(this.backing.clone(), this.indexBase, this.indexStart, this.indexEnd);
	}
	/**
	* _rotate shifts the backing array contents so that indexStart ==
	* indexBase to simplify the downscale logic.
	*/
	_rotate() {
		const bias = this.indexBase - this.indexStart;
		if (bias === 0) return;
		else if (bias > 0) {
			this.backing.reverse(0, this.backing.length);
			this.backing.reverse(0, bias);
			this.backing.reverse(bias, this.backing.length);
		} else {
			this.backing.reverse(0, this.backing.length);
			this.backing.reverse(0, this.backing.length + bias);
		}
		this.indexBase = this.indexStart;
	}
	/**
	* _relocateBucket adds the count in counts[src] to counts[dest] and
	* resets count[src] to zero.
	*/
	_relocateBucket(dest, src) {
		if (dest === src) return;
		this.incrementBucket(dest, this.backing.emptyBucket(src));
	}
};
/**
* BucketsBacking holds the raw buckets and some utility methods to
* manage them.
*/
var BucketsBacking = class BucketsBacking {
	_counts;
	constructor(counts = [0]) {
		this._counts = counts;
	}
	/**
	* length returns the physical size of the backing array, which
	* is >= buckets.length()
	*/
	get length() {
		return this._counts.length;
	}
	/**
	* countAt returns the count in a specific bucket
	*/
	countAt(pos) {
		return this._counts[pos];
	}
	/**
	* growTo grows a backing array and copies old entries
	* into their correct new positions.
	*/
	growTo(newSize, oldPositiveLimit, newPositiveLimit) {
		const tmp = new Array(newSize).fill(0);
		tmp.splice(newPositiveLimit, this._counts.length - oldPositiveLimit, ...this._counts.slice(oldPositiveLimit));
		tmp.splice(0, oldPositiveLimit, ...this._counts.slice(0, oldPositiveLimit));
		this._counts = tmp;
	}
	/**
	* reverse the items in the backing array in the range [from, limit).
	*/
	reverse(from, limit) {
		const num = Math.floor((from + limit) / 2) - from;
		for (let i = 0; i < num; i++) {
			const tmp = this._counts[from + i];
			this._counts[from + i] = this._counts[limit - i - 1];
			this._counts[limit - i - 1] = tmp;
		}
	}
	/**
	* emptyBucket empties the count from a bucket, for
	* moving into another.
	*/
	emptyBucket(src) {
		const tmp = this._counts[src];
		this._counts[src] = 0;
		return tmp;
	}
	/**
	* increments a bucket by `increment`
	*/
	increment(bucketIndex, increment) {
		this._counts[bucketIndex] += increment;
	}
	/**
	* decrements a bucket by `decrement`
	*/
	decrement(bucketIndex, decrement) {
		if (this._counts[bucketIndex] >= decrement) this._counts[bucketIndex] -= decrement;
		else this._counts[bucketIndex] = 0;
	}
	/**
	* clone returns a deep copy of BucketsBacking
	*/
	clone() {
		return new BucketsBacking([...this._counts]);
	}
};
/**
* The functions and constants in this file allow us to interact
* with the internal representation of an IEEE 64-bit floating point
* number. We need to work with all 64-bits, thus, care needs to be
* taken when working with Javascript's bitwise operators (<<, >>, &,
* |, etc) as they truncate operands to 32-bits. In order to work around
* this we work with the 64-bits as two 32-bit halves and perform bitwise
* operations on each half independently.
*/
/**
* EXPONENT_MASK is set to 1 for the hi 32-bits of an IEEE 754
* floating point exponent: 0x7ff00000.
*/
var EXPONENT_MASK = 2146435072;
/**
* SIGNIFICAND_MASK is the mask for the significand portion of the hi 32-bits
* of an IEEE 754 double-precision floating-point value: 0xfffff
*/
var SIGNIFICAND_MASK = 1048575;
/**
* EXPONENT_BIAS is the exponent bias specified for encoding
* the IEEE 754 double-precision floating point exponent: 1023
*/
var EXPONENT_BIAS = 1023;
/**
* MIN_NORMAL_EXPONENT is the minimum exponent of a normalized
* floating point: -1022.
*/
var MIN_NORMAL_EXPONENT = -1022;
/**
* MAX_NORMAL_EXPONENT is the maximum exponent of a normalized
* floating point: 1023.
*/
var MAX_NORMAL_EXPONENT = EXPONENT_BIAS;
/**
* MIN_VALUE is the smallest normal number
*/
var MIN_VALUE = Math.pow(2, -1022);
var dv = /* @__PURE__ */ new DataView(/* @__PURE__ */ new ArrayBuffer(8));
/**
* floatBits writes value into the shared buffer and returns its two 32-bit
* halves.
* @param {number} value - the floating point number to read
* @returns {{hi: number, lo: number}} the high and low 32-bit halves
*/
function floatBits(value) {
	dv.setFloat64(0, value);
	return {
		hi: dv.getUint32(0),
		lo: dv.getUint32(4)
	};
}
/**
* getNormalBase2 extracts the normalized base-2 fractional exponent.
* This returns k for the equation f x 2**k where f is
* in the range [1, 2).  Note that this function is not called for
* subnormal numbers.
* @param {number} value - the value to determine normalized base-2 fractional
*    exponent for
* @returns {number} the normalized base-2 exponent
*/
function getNormalBase2(value) {
	const { hi } = floatBits(value);
	return ((hi & EXPONENT_MASK) >> 20) - EXPONENT_BIAS;
}
/**
* isPowerOfTwo reports whether value is an exact power of two, e.g. its 52-bit
* significand is all zeros. Only valid for positive, finite values.
* @param {number} value - the floating point number to test
* @returns {boolean} true if value is an exact power of two
*/
function isPowerOfTwo(value) {
	const { hi, lo } = floatBits(value);
	return (hi & SIGNIFICAND_MASK) === 0 && lo === 0;
}
/**
* Note: other languages provide this as a built in function. This is
* a naive, but functionally correct implementation. This is used sparingly,
* when creating a new mapping in a running application.
*
* ldexp returns frac × 2**exp. With the following special cases:
*   ldexp(±0, exp) = ±0
*   ldexp(±Inf, exp) = ±Inf
*   ldexp(NaN, exp) = NaN
* @param frac
* @param exp
* @returns {number}
*/
function ldexp(frac, exp) {
	if (frac === 0 || frac === Number.POSITIVE_INFINITY || frac === Number.NEGATIVE_INFINITY || Number.isNaN(frac)) return frac;
	return frac * Math.pow(2, exp);
}
/**
* Computes the next power of two that is greater than or equal to v.
* This implementation more efficient than, but functionally equivalent
* to Math.pow(2, Math.ceil(Math.log(x)/Math.log(2))).
* @param v
* @returns {number}
*/
function nextGreaterSquare(v) {
	v--;
	v |= v >> 1;
	v |= v >> 2;
	v |= v >> 4;
	v |= v >> 8;
	v |= v >> 16;
	v++;
	return v;
}
var MappingError = class extends Error {};
/**
* ExponentMapping implements exponential mapping functions for
* scales <=0. For scales > 0 LogarithmMapping should be used.
*/
var ExponentMapping = class {
	_shift;
	constructor(scale) {
		this._shift = -scale;
	}
	/**
	* Maps positive floating point values to indexes corresponding to scale
	* @param value
	* @returns {number} index for provided value at the current scale
	*/
	mapToIndex(value) {
		if (value < MIN_VALUE) return this._minNormalLowerBoundaryIndex();
		return getNormalBase2(value) + (isPowerOfTwo(value) ? -1 : 0) >> this._shift;
	}
	/**
	* Returns the lower bucket boundary for the given index for scale
	*
	* @param index
	* @returns {number}
	*/
	lowerBoundary(index) {
		const minIndex = this._minNormalLowerBoundaryIndex();
		if (index < minIndex) throw new MappingError(`underflow: ${index} is < minimum lower boundary: ${minIndex}`);
		const maxIndex = this._maxNormalLowerBoundaryIndex();
		if (index > maxIndex) throw new MappingError(`overflow: ${index} is > maximum lower boundary: ${maxIndex}`);
		return ldexp(1, index << this._shift);
	}
	/**
	* The scale used by this mapping
	* @returns {number}
	*/
	get scale() {
		if (this._shift === 0) return 0;
		return -this._shift;
	}
	_minNormalLowerBoundaryIndex() {
		let index = MIN_NORMAL_EXPONENT >> this._shift;
		if (this._shift < 2) index--;
		return index;
	}
	_maxNormalLowerBoundaryIndex() {
		return MAX_NORMAL_EXPONENT >> this._shift;
	}
};
/**
* LogarithmMapping implements exponential mapping functions for scale > 0.
* For scales <= 0 the exponent mapping should be used.
*/
var LogarithmMapping = class {
	_scale;
	_scaleFactor;
	_inverseFactor;
	constructor(scale) {
		this._scale = scale;
		this._scaleFactor = ldexp(Math.LOG2E, scale);
		this._inverseFactor = ldexp(Math.LN2, -scale);
	}
	/**
	* Maps positive floating point values to indexes corresponding to scale
	* @param value
	* @returns {number} index for provided value at the current scale
	*/
	mapToIndex(value) {
		if (value <= MIN_VALUE) return this._minNormalLowerBoundaryIndex() - 1;
		if (isPowerOfTwo(value)) return (getNormalBase2(value) << this._scale) - 1;
		const index = Math.floor(Math.log(value) * this._scaleFactor);
		const maxIndex = this._maxNormalLowerBoundaryIndex();
		if (index >= maxIndex) return maxIndex;
		return index;
	}
	/**
	* Returns the lower bucket boundary for the given index for scale
	*
	* @param index
	* @returns {number}
	*/
	lowerBoundary(index) {
		const maxIndex = this._maxNormalLowerBoundaryIndex();
		if (index >= maxIndex) {
			if (index === maxIndex) return 2 * Math.exp((index - (1 << this._scale)) / this._scaleFactor);
			throw new MappingError(`overflow: ${index} is > maximum lower boundary: ${maxIndex}`);
		}
		const minIndex = this._minNormalLowerBoundaryIndex();
		if (index <= minIndex) {
			if (index === minIndex) return MIN_VALUE;
			else if (index === minIndex - 1) return Math.exp((index + (1 << this._scale)) / this._scaleFactor) / 2;
			throw new MappingError(`overflow: ${index} is < minimum lower boundary: ${minIndex}`);
		}
		return Math.exp(index * this._inverseFactor);
	}
	/**
	* The scale used by this mapping
	* @returns {number}
	*/
	get scale() {
		return this._scale;
	}
	_minNormalLowerBoundaryIndex() {
		return MIN_NORMAL_EXPONENT << this._scale;
	}
	_maxNormalLowerBoundaryIndex() {
		return (MAX_NORMAL_EXPONENT + 1 << this._scale) - 1;
	}
};
var MIN_SCALE = -10;
var MAX_SCALE$1 = 20;
var PREBUILT_MAPPINGS = Array.from({ length: 31 }, (_, i) => {
	if (i > 10) return new LogarithmMapping(i - 10);
	return new ExponentMapping(i - 10);
});
/**
* getMapping returns an appropriate mapping for the given scale. For scales -10
* to 0 the underlying type will be ExponentMapping. For scales 1 to 20 the
* underlying type will be LogarithmMapping.
* @param scale a number in the range [-10, 20]
* @returns {Mapping}
*/
function getMapping(scale) {
	if (scale > MAX_SCALE$1 || scale < MIN_SCALE) throw new MappingError(`expected scale >= ${MIN_SCALE} && <= ${MAX_SCALE$1}, got: ${scale}`);
	return PREBUILT_MAPPINGS[scale + 10];
}
init_esm();
var HighLow = class HighLow {
	static combine(h1, h2) {
		return new HighLow(Math.min(h1.low, h2.low), Math.max(h1.high, h2.high));
	}
	low;
	high;
	constructor(low, high) {
		this.low = low;
		this.high = high;
	}
};
var MAX_SCALE = 20;
var DEFAULT_MAX_SIZE = 160;
var MIN_MAX_SIZE = 2;
var ExponentialHistogramAccumulation = class ExponentialHistogramAccumulation {
	startTime;
	_maxSize;
	_recordMinMax;
	_sum;
	_count;
	_zeroCount;
	_min;
	_max;
	_positive;
	_negative;
	_mapping;
	constructor(startTime, maxSize = DEFAULT_MAX_SIZE, recordMinMax = true, sum = 0, count = 0, zeroCount = 0, min = Number.POSITIVE_INFINITY, max = Number.NEGATIVE_INFINITY, positive = new Buckets(), negative = new Buckets(), mapping = getMapping(MAX_SCALE)) {
		this.startTime = startTime;
		this._maxSize = maxSize;
		this._recordMinMax = recordMinMax;
		this._sum = sum;
		this._count = count;
		this._zeroCount = zeroCount;
		this._min = min;
		this._max = max;
		this._positive = positive;
		this._negative = negative;
		this._mapping = mapping;
		if (this._maxSize < MIN_MAX_SIZE) {
			diag.warn(`Exponential Histogram Max Size set to ${this._maxSize}, \
                changing to the minimum size of: ${MIN_MAX_SIZE}`);
			this._maxSize = MIN_MAX_SIZE;
		}
	}
	/**
	* record updates a histogram with a single count
	* @param {Number} value
	*/
	record(value) {
		this.updateByIncrement(value, 1);
	}
	/**
	* Sets the start time for this accumulation
	* @param {HrTime} startTime
	*/
	setStartTime(startTime) {
		this.startTime = startTime;
	}
	/**
	* Returns the datapoint representation of this accumulation
	* @param {HrTime} startTime
	*/
	toPointValue() {
		return {
			hasMinMax: this._recordMinMax,
			min: this.min,
			max: this.max,
			sum: this.sum,
			positive: {
				offset: this.positive.offset,
				bucketCounts: this.positive.counts()
			},
			negative: {
				offset: this.negative.offset,
				bucketCounts: this.negative.counts()
			},
			count: this.count,
			scale: this.scale,
			zeroCount: this.zeroCount
		};
	}
	/**
	* @returns {Number} The sum of values recorded by this accumulation
	*/
	get sum() {
		return this._sum;
	}
	/**
	* @returns {Number} The minimum value recorded by this accumulation
	*/
	get min() {
		return this._min;
	}
	/**
	* @returns {Number} The maximum value recorded by this accumulation
	*/
	get max() {
		return this._max;
	}
	/**
	* @returns {Number} The count of values recorded by this accumulation
	*/
	get count() {
		return this._count;
	}
	/**
	* @returns {Number} The number of 0 values recorded by this accumulation
	*/
	get zeroCount() {
		return this._zeroCount;
	}
	/**
	* @returns {Number} The scale used by this accumulation
	*/
	get scale() {
		if (this._count === this._zeroCount) return 0;
		return this._mapping.scale;
	}
	/**
	* positive holds the positive values
	* @returns {Buckets}
	*/
	get positive() {
		return this._positive;
	}
	/**
	* negative holds the negative values by their absolute value
	* @returns {Buckets}
	*/
	get negative() {
		return this._negative;
	}
	/**
	* updateByIncr supports updating a histogram with a non-negative
	* increment.
	* @param value
	* @param increment
	*/
	updateByIncrement(value, increment) {
		if (!Number.isFinite(value)) return;
		if (value > this._max) this._max = value;
		if (value < this._min) this._min = value;
		this._count += increment;
		if (value === 0) {
			this._zeroCount += increment;
			return;
		}
		this._sum += value * increment;
		if (value > 0) this._updateBuckets(this._positive, value, increment);
		else this._updateBuckets(this._negative, -value, increment);
	}
	/**
	* merge combines data from previous value into self
	* @param {ExponentialHistogramAccumulation} previous
	*/
	merge(previous) {
		if (this._count === 0) {
			this._min = previous.min;
			this._max = previous.max;
		} else if (previous.count !== 0) {
			if (previous.min < this.min) this._min = previous.min;
			if (previous.max > this.max) this._max = previous.max;
		}
		this.startTime = previous.startTime;
		this._sum += previous.sum;
		this._count += previous.count;
		this._zeroCount += previous.zeroCount;
		const minScale = this._minScale(previous);
		this._downscale(this.scale - minScale);
		this._mergeBuckets(this.positive, previous, previous.positive, minScale);
		this._mergeBuckets(this.negative, previous, previous.negative, minScale);
	}
	/**
	* diff subtracts other from self
	* @param {ExponentialHistogramAccumulation} other
	*/
	diff(other) {
		this._min = Infinity;
		this._max = -Infinity;
		this._sum -= other.sum;
		this._count -= other.count;
		this._zeroCount -= other.zeroCount;
		const minScale = this._minScale(other);
		this._downscale(this.scale - minScale);
		this._diffBuckets(this.positive, other, other.positive, minScale);
		this._diffBuckets(this.negative, other, other.negative, minScale);
	}
	/**
	* clone returns a deep copy of self
	* @returns {ExponentialHistogramAccumulation}
	*/
	clone() {
		return new ExponentialHistogramAccumulation(this.startTime, this._maxSize, this._recordMinMax, this._sum, this._count, this._zeroCount, this._min, this._max, this.positive.clone(), this.negative.clone(), this._mapping);
	}
	/**
	* _updateBuckets maps the incoming value to a bucket index for the current
	* scale. If the bucket index is outside of the range of the backing array,
	* it will rescale the backing array and update the mapping for the new scale.
	*/
	_updateBuckets(buckets, value, increment) {
		let index = this._mapping.mapToIndex(value);
		let rescalingNeeded = false;
		let high = 0;
		let low = 0;
		if (buckets.length === 0) {
			buckets.indexStart = index;
			buckets.indexEnd = buckets.indexStart;
			buckets.indexBase = buckets.indexStart;
		} else if (index < buckets.indexStart && buckets.indexEnd - index >= this._maxSize) {
			rescalingNeeded = true;
			low = index;
			high = buckets.indexEnd;
		} else if (index > buckets.indexEnd && index - buckets.indexStart >= this._maxSize) {
			rescalingNeeded = true;
			low = buckets.indexStart;
			high = index;
		}
		if (rescalingNeeded) {
			const change = this._changeScale(high, low);
			this._downscale(change);
			index = this._mapping.mapToIndex(value);
		}
		this._incrementIndexBy(buckets, index, increment);
	}
	/**
	* _incrementIndexBy increments the count of the bucket specified by `index`.
	* If the index is outside of the range [buckets.indexStart, buckets.indexEnd]
	* the boundaries of the backing array will be adjusted and more buckets will
	* be added if needed.
	*/
	_incrementIndexBy(buckets, index, increment) {
		if (increment === 0) return;
		if (buckets.length === 0) buckets.indexStart = buckets.indexEnd = buckets.indexBase = index;
		if (index < buckets.indexStart) {
			const span = buckets.indexEnd - index;
			if (span >= buckets.backing.length) this._grow(buckets, span + 1);
			buckets.indexStart = index;
		} else if (index > buckets.indexEnd) {
			const span = index - buckets.indexStart;
			if (span >= buckets.backing.length) this._grow(buckets, span + 1);
			buckets.indexEnd = index;
		}
		let bucketIndex = index - buckets.indexBase;
		if (bucketIndex < 0) bucketIndex += buckets.backing.length;
		buckets.incrementBucket(bucketIndex, increment);
	}
	/**
	* grow resizes the backing array by doubling in size up to maxSize.
	* This extends the array with a bunch of zeros and copies the
	* existing counts to the same position.
	*/
	_grow(buckets, needed) {
		const size = buckets.backing.length;
		const bias = buckets.indexBase - buckets.indexStart;
		const oldPositiveLimit = size - bias;
		let newSize = nextGreaterSquare(needed);
		if (newSize > this._maxSize) newSize = this._maxSize;
		const newPositiveLimit = newSize - bias;
		buckets.backing.growTo(newSize, oldPositiveLimit, newPositiveLimit);
	}
	/**
	* _changeScale computes how much downscaling is needed by shifting the
	* high and low values until they are separated by no more than size.
	*/
	_changeScale(high, low) {
		let change = 0;
		while (high - low >= this._maxSize) {
			high >>= 1;
			low >>= 1;
			change++;
		}
		return change;
	}
	/**
	* _downscale subtracts `change` from the current mapping scale.
	*/
	_downscale(change) {
		if (change === 0) return;
		if (change < 0) throw new Error(`impossible change of scale: ${this.scale}`);
		const newScale = this._mapping.scale - change;
		this._positive.downscale(change);
		this._negative.downscale(change);
		this._mapping = getMapping(newScale);
	}
	/**
	* _minScale is used by diff and merge to compute an ideal combined scale
	*/
	_minScale(other) {
		const minScale = Math.min(this.scale, other.scale);
		const highLowPos = HighLow.combine(this._highLowAtScale(this.positive, this.scale, minScale), this._highLowAtScale(other.positive, other.scale, minScale));
		const highLowNeg = HighLow.combine(this._highLowAtScale(this.negative, this.scale, minScale), this._highLowAtScale(other.negative, other.scale, minScale));
		return Math.min(minScale - this._changeScale(highLowPos.high, highLowPos.low), minScale - this._changeScale(highLowNeg.high, highLowNeg.low));
	}
	/**
	* _highLowAtScale is used by diff and merge to compute an ideal combined scale.
	*/
	_highLowAtScale(buckets, currentScale, newScale) {
		if (buckets.length === 0) return new HighLow(0, -1);
		const shift = currentScale - newScale;
		return new HighLow(buckets.indexStart >> shift, buckets.indexEnd >> shift);
	}
	/**
	* _mergeBuckets translates index values from another histogram and
	* adds the values into the corresponding buckets of this histogram.
	*/
	_mergeBuckets(ours, other, theirs, scale) {
		const theirOffset = theirs.offset;
		const theirChange = other.scale - scale;
		for (let i = 0; i < theirs.length; i++) this._incrementIndexBy(ours, theirOffset + i >> theirChange, theirs.at(i));
	}
	/**
	* _diffBuckets translates index values from another histogram and
	* subtracts the values in the corresponding buckets of this histogram.
	*/
	_diffBuckets(ours, other, theirs, scale) {
		const theirOffset = theirs.offset;
		const theirChange = other.scale - scale;
		for (let i = 0; i < theirs.length; i++) {
			let bucketIndex = (theirOffset + i >> theirChange) - ours.indexBase;
			if (bucketIndex < 0) bucketIndex += ours.backing.length;
			ours.decrementBucket(bucketIndex, theirs.at(i));
		}
		ours.trim();
	}
};
/**
* Aggregator for ExponentialHistogramAccumulations
*/
var ExponentialHistogramAggregator = class {
	kind = AggregatorKind.EXPONENTIAL_HISTOGRAM;
	_maxSize;
	_recordMinMax;
	/**
	* @param _maxSize Maximum number of buckets for each of the positive
	*    and negative ranges, exclusive of the zero-bucket.
	* @param _recordMinMax If set to true, min and max will be recorded.
	*    Otherwise, min and max will not be recorded.
	*/
	constructor(maxSize, recordMinMax) {
		this._maxSize = maxSize;
		this._recordMinMax = recordMinMax;
	}
	createAccumulation(startTime) {
		return new ExponentialHistogramAccumulation(startTime, this._maxSize, this._recordMinMax);
	}
	/**
	* Return the result of the merge of two exponential histogram accumulations.
	*/
	merge(previous, delta) {
		const result = delta.clone();
		result.merge(previous);
		return result;
	}
	/**
	* Returns a new DELTA aggregation by comparing two cumulative measurements.
	*/
	diff(previous, current) {
		const result = current.clone();
		result.diff(previous);
		return result;
	}
	toMetricData(descriptor, aggregationTemporality, accumulationByAttributes, endTime) {
		return {
			descriptor,
			aggregationTemporality,
			dataPointType: DataPointType.EXPONENTIAL_HISTOGRAM,
			dataPoints: accumulationByAttributes.map(([attributes, accumulation]) => {
				const pointValue = accumulation.toPointValue();
				const allowsNegativeValues = descriptor.type === InstrumentType.GAUGE || descriptor.type === InstrumentType.UP_DOWN_COUNTER || descriptor.type === InstrumentType.OBSERVABLE_GAUGE || descriptor.type === InstrumentType.OBSERVABLE_UP_DOWN_COUNTER;
				return {
					attributes,
					startTime: accumulation.startTime,
					endTime,
					value: {
						min: pointValue.hasMinMax ? pointValue.min : void 0,
						max: pointValue.hasMinMax ? pointValue.max : void 0,
						sum: !allowsNegativeValues ? pointValue.sum : void 0,
						positive: {
							offset: pointValue.positive.offset,
							bucketCounts: pointValue.positive.bucketCounts
						},
						negative: {
							offset: pointValue.negative.offset,
							bucketCounts: pointValue.negative.bucketCounts
						},
						count: pointValue.count,
						scale: pointValue.scale,
						zeroCount: pointValue.zeroCount
					}
				};
			})
		};
	}
};
var LastValueAccumulation = class {
	startTime;
	_current;
	sampleTime;
	constructor(startTime, current = 0, sampleTime = [0, 0]) {
		this.startTime = startTime;
		this._current = current;
		this.sampleTime = sampleTime;
	}
	record(value) {
		this._current = value;
		this.sampleTime = millisToHrTime(Date.now());
	}
	setStartTime(startTime) {
		this.startTime = startTime;
	}
	toPointValue() {
		return this._current;
	}
};
/** Basic aggregator which calculates a LastValue from individual measurements. */
var LastValueAggregator = class {
	kind = AggregatorKind.LAST_VALUE;
	createAccumulation(startTime) {
		return new LastValueAccumulation(startTime);
	}
	/**
	* Returns the result of the merge of the given accumulations.
	*
	* Return the newly captured (delta) accumulation for LastValueAggregator.
	*/
	merge(previous, delta) {
		const latestAccumulation = hrTimeToMicroseconds(delta.sampleTime) >= hrTimeToMicroseconds(previous.sampleTime) ? delta : previous;
		return new LastValueAccumulation(previous.startTime, latestAccumulation.toPointValue(), latestAccumulation.sampleTime);
	}
	/**
	* Returns a new DELTA aggregation by comparing two cumulative measurements.
	*
	* A delta aggregation is not meaningful to LastValueAggregator, just return
	* the newly captured (delta) accumulation for LastValueAggregator.
	*/
	diff(previous, current) {
		const latestAccumulation = hrTimeToMicroseconds(current.sampleTime) >= hrTimeToMicroseconds(previous.sampleTime) ? current : previous;
		return new LastValueAccumulation(current.startTime, latestAccumulation.toPointValue(), latestAccumulation.sampleTime);
	}
	toMetricData(descriptor, aggregationTemporality, accumulationByAttributes, endTime) {
		return {
			descriptor,
			aggregationTemporality,
			dataPointType: DataPointType.GAUGE,
			dataPoints: accumulationByAttributes.map(([attributes, accumulation]) => {
				return {
					attributes,
					startTime: accumulation.startTime,
					endTime,
					value: accumulation.toPointValue()
				};
			})
		};
	}
};
var SumAccumulation = class {
	startTime;
	monotonic;
	_current;
	reset;
	constructor(startTime, monotonic, current = 0, reset = false) {
		this.startTime = startTime;
		this.monotonic = monotonic;
		this._current = current;
		this.reset = reset;
	}
	record(value) {
		if (this.monotonic && value < 0) return;
		this._current += value;
	}
	setStartTime(startTime) {
		this.startTime = startTime;
	}
	toPointValue() {
		return this._current;
	}
};
/** Basic aggregator which calculates a Sum from individual measurements. */
var SumAggregator = class {
	kind = AggregatorKind.SUM;
	monotonic;
	constructor(monotonic) {
		this.monotonic = monotonic;
	}
	createAccumulation(startTime) {
		return new SumAccumulation(startTime, this.monotonic);
	}
	/**
	* Returns the result of the merge of the given accumulations.
	*/
	merge(previous, delta) {
		const prevPv = previous.toPointValue();
		const deltaPv = delta.toPointValue();
		if (delta.reset) return new SumAccumulation(delta.startTime, this.monotonic, deltaPv, delta.reset);
		return new SumAccumulation(previous.startTime, this.monotonic, prevPv + deltaPv);
	}
	/**
	* Returns a new DELTA aggregation by comparing two cumulative measurements.
	*/
	diff(previous, current) {
		const prevPv = previous.toPointValue();
		const currPv = current.toPointValue();
		/**
		* If the SumAggregator is a monotonic one and the previous point value is
		* greater than the current one, a reset is deemed to be happened.
		* Return the current point value to prevent the value from been reset.
		*/
		if (this.monotonic && prevPv > currPv) return new SumAccumulation(current.startTime, this.monotonic, currPv, true);
		return new SumAccumulation(current.startTime, this.monotonic, currPv - prevPv);
	}
	toMetricData(descriptor, aggregationTemporality, accumulationByAttributes, endTime) {
		return {
			descriptor,
			aggregationTemporality,
			dataPointType: DataPointType.SUM,
			dataPoints: accumulationByAttributes.map(([attributes, accumulation]) => {
				return {
					attributes,
					startTime: accumulation.startTime,
					endTime,
					value: accumulation.toPointValue()
				};
			}),
			isMonotonic: this.monotonic
		};
	}
};
init_esm();
/**
* The default drop aggregation.
*/
var DropAggregation = class DropAggregation {
	static DEFAULT_INSTANCE = new DropAggregator();
	createAggregator(_instrument) {
		return DropAggregation.DEFAULT_INSTANCE;
	}
};
/**
* The default sum aggregation.
*/
var SumAggregation = class SumAggregation {
	static MONOTONIC_INSTANCE = new SumAggregator(true);
	static NON_MONOTONIC_INSTANCE = new SumAggregator(false);
	createAggregator(instrument) {
		switch (instrument.type) {
			case InstrumentType.COUNTER:
			case InstrumentType.OBSERVABLE_COUNTER:
			case InstrumentType.HISTOGRAM: return SumAggregation.MONOTONIC_INSTANCE;
			default: return SumAggregation.NON_MONOTONIC_INSTANCE;
		}
	}
};
/**
* The default last value aggregation.
*/
var LastValueAggregation = class LastValueAggregation {
	static DEFAULT_INSTANCE = new LastValueAggregator();
	createAggregator(_instrument) {
		return LastValueAggregation.DEFAULT_INSTANCE;
	}
};
/**
* The default histogram aggregation.

*/
var HistogramAggregation = class HistogramAggregation {
	static DEFAULT_INSTANCE = new HistogramAggregator([
		0,
		5,
		10,
		25,
		50,
		75,
		100,
		250,
		500,
		750,
		1e3,
		2500,
		5e3,
		7500,
		1e4
	], true);
	createAggregator(_instrument) {
		return HistogramAggregation.DEFAULT_INSTANCE;
	}
};
/**
* The explicit bucket histogram aggregation.
*/
var ExplicitBucketHistogramAggregation = class {
	_boundaries;
	_recordMinMax;
	/**
	* @param boundaries the bucket boundaries of the histogram aggregation
	* @param _recordMinMax If set to true, min and max will be recorded. Otherwise, min and max will not be recorded.
	*/
	constructor(boundaries, recordMinMax = true) {
		if (boundaries == null) throw new Error("ExplicitBucketHistogramAggregation should be created with explicit boundaries, if a single bucket histogram is required, please pass an empty array");
		boundaries = boundaries.concat();
		boundaries = boundaries.sort((a, b) => a - b);
		const minusInfinityIndex = boundaries.lastIndexOf(-Infinity);
		let infinityIndex = boundaries.indexOf(Infinity);
		if (infinityIndex === -1) infinityIndex = void 0;
		this._boundaries = boundaries.slice(minusInfinityIndex + 1, infinityIndex);
		this._recordMinMax = recordMinMax;
	}
	createAggregator(_instrument) {
		return new HistogramAggregator(this._boundaries, this._recordMinMax);
	}
};
var ExponentialHistogramAggregation = class {
	_maxSize;
	_recordMinMax;
	constructor(maxSize = 160, recordMinMax = true) {
		this._maxSize = maxSize;
		this._recordMinMax = recordMinMax;
	}
	createAggregator(_instrument) {
		return new ExponentialHistogramAggregator(this._maxSize, this._recordMinMax);
	}
};
/**
* The default aggregation.
*/
var DefaultAggregation = class {
	_resolve(instrument) {
		switch (instrument.type) {
			case InstrumentType.COUNTER:
			case InstrumentType.UP_DOWN_COUNTER:
			case InstrumentType.OBSERVABLE_COUNTER:
			case InstrumentType.OBSERVABLE_UP_DOWN_COUNTER: return SUM_AGGREGATION;
			case InstrumentType.GAUGE:
			case InstrumentType.OBSERVABLE_GAUGE: return LAST_VALUE_AGGREGATION;
			case InstrumentType.HISTOGRAM:
				if (instrument.advice.explicitBucketBoundaries) return new ExplicitBucketHistogramAggregation(instrument.advice.explicitBucketBoundaries);
				return HISTOGRAM_AGGREGATION;
		}
		diag.warn(`Unable to recognize instrument type: ${instrument.type}`);
		return DROP_AGGREGATION;
	}
	createAggregator(instrument) {
		return this._resolve(instrument).createAggregator(instrument);
	}
};
var DROP_AGGREGATION = new DropAggregation();
var SUM_AGGREGATION = new SumAggregation();
var LAST_VALUE_AGGREGATION = new LastValueAggregation();
var HISTOGRAM_AGGREGATION = new HistogramAggregation();
new ExponentialHistogramAggregation();
var DEFAULT_AGGREGATION = new DefaultAggregation();
var AggregationType;
(function(AggregationType) {
	AggregationType[AggregationType["DEFAULT"] = 0] = "DEFAULT";
	AggregationType[AggregationType["DROP"] = 1] = "DROP";
	AggregationType[AggregationType["SUM"] = 2] = "SUM";
	AggregationType[AggregationType["LAST_VALUE"] = 3] = "LAST_VALUE";
	AggregationType[AggregationType["EXPLICIT_BUCKET_HISTOGRAM"] = 4] = "EXPLICIT_BUCKET_HISTOGRAM";
	AggregationType[AggregationType["EXPONENTIAL_HISTOGRAM"] = 5] = "EXPONENTIAL_HISTOGRAM";
})(AggregationType || (AggregationType = {}));
function toAggregation(option) {
	switch (option.type) {
		case AggregationType.DEFAULT: return DEFAULT_AGGREGATION;
		case AggregationType.DROP: return DROP_AGGREGATION;
		case AggregationType.SUM: return SUM_AGGREGATION;
		case AggregationType.LAST_VALUE: return LAST_VALUE_AGGREGATION;
		case AggregationType.EXPONENTIAL_HISTOGRAM: {
			const expOption = option;
			return new ExponentialHistogramAggregation(expOption.options?.maxSize, expOption.options?.recordMinMax);
		}
		case AggregationType.EXPLICIT_BUCKET_HISTOGRAM: {
			const expOption = option;
			if (expOption.options == null) return HISTOGRAM_AGGREGATION;
			else return new ExplicitBucketHistogramAggregation(expOption.options?.boundaries, expOption.options?.recordMinMax);
		}
		default: throw new Error("Unsupported Aggregation");
	}
}
var DEFAULT_AGGREGATION_SELECTOR = (_instrumentType) => {
	return { type: AggregationType.DEFAULT };
};
var DEFAULT_AGGREGATION_TEMPORALITY_SELECTOR = (_instrumentType) => AggregationTemporality.CUMULATIVE;
/**
* A name uniquely identifying the instance of the OpenTelemetry component within its containing SDK instance.
*
* @example otlp_grpc_span_exporter/0
* @example custom-name
*
* @note Implementations **SHOULD** ensure a low cardinality for this attribute, even across application or SDK restarts.
* E.g. implementations **MUST NOT** use UUIDs as values for this attribute.
*
* Implementations **MAY** achieve these goals by following a `<otel.component.type>/<instance-counter>` pattern, e.g. `batching_span_processor/0`.
* Hereby `otel.component.type` refers to the corresponding attribute value of the component.
*
* The value of `instance-counter` **MAY** be automatically assigned by the component and uniqueness within the enclosing SDK instance **MUST** be guaranteed.
* For example, `<instance-counter>` **MAY** be implemented by using a monotonically increasing counter (starting with `0`), which is incremented every time an
* instance of the given component type is started.
*
* With this implementation, for example the first Batching Span Processor would have `batching_span_processor/0`
* as `otel.component.name`, the second one `batching_span_processor/1` and so on.
* These values will therefore be reused in the case of an application restart.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_NAME$2 = "otel.component.name";
/**
* A name identifying the type of the OpenTelemetry component.
*
* @example batching_span_processor
* @example com.example.MySpanExporter
*
* @note If none of the standardized values apply, implementations **SHOULD** use the language-defined name of the type.
* E.g. for Java the fully qualified classname **SHOULD** be used in this case.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_TYPE$2 = "otel.component.type";
/**
* Enum value "periodic_metric_reader" for attribute {@link ATTR_OTEL_COMPONENT_TYPE}.
*
* The builtin SDK periodically exporting metric reader
*
* @experimental This enum value is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var OTEL_COMPONENT_TYPE_VALUE_PERIODIC_METRIC_READER = "periodic_metric_reader";
/**
* The duration of the collect operation of the metric reader.
*
* @note For successful collections, `error.type` **MUST NOT** be set. For failed collections, `error.type` **SHOULD** contain the failure cause.
* It can happen that metrics collection is successful for some MetricProducers, while others fail. In that case `error.type` **SHOULD** be set to any of the failure causes.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_METRIC_READER_COLLECTION_DURATION = "otel.sdk.metric_reader.collection.duration";
/**
* Describes a class of error the operation ended with.
*
* @example timeout
* @example java.net.UnknownHostException
* @example server_certificate_invalid
* @example 500
*
* @note The `error.type` **SHOULD** be predictable, and **SHOULD** have low cardinality.
*
* When `error.type` is set to a type (e.g., an exception type), its
* canonical class name identifying the type within the artifact **SHOULD** be used.
*
* Instrumentations **SHOULD** document the list of errors they report.
*
* The cardinality of `error.type` within one instrumentation library **SHOULD** be low.
* Telemetry consumers that aggregate data from multiple instrumentation libraries and applications
* should be prepared for `error.type` to have high cardinality at query time when no
* additional filters are applied.
*
* If the operation has completed successfully, instrumentations **SHOULD NOT** set `error.type`.
*
* If a specific domain defines its own set of error identifiers (such as HTTP or RPC status codes),
* it's **RECOMMENDED** to:
*
*   - Use a domain-specific attribute
*   - Set `error.type` to capture all errors, regardless of whether they are defined within the domain-specific set or not.
*/
var ATTR_ERROR_TYPE$1 = "error.type";
var componentCounter$2 = /* @__PURE__ */ new Map();
/**
* Generates `otel.sdk.metric_reader.*` self-observability metrics.
* https://opentelemetry.io/docs/specs/semconv/otel/sdk-metrics/#metric-otelsdkmetric_readercollectionduration
*/
var MetricReaderMetrics = class {
	collectionDuration;
	standardAttrs;
	constructor(componentType, meter) {
		const counter = componentCounter$2.get(componentType) ?? 0;
		componentCounter$2.set(componentType, counter + 1);
		this.standardAttrs = {
			[ATTR_OTEL_COMPONENT_TYPE$2]: componentType,
			[ATTR_OTEL_COMPONENT_NAME$2]: `${componentType}/${counter}`
		};
		this.collectionDuration = meter.createHistogram(METRIC_OTEL_SDK_METRIC_READER_COLLECTION_DURATION, {
			unit: "s",
			description: "The duration of the collect operation of the metric reader.",
			advice: { explicitBucketBoundaries: [] }
		});
	}
	recordCollection(durationSecs, error) {
		const attrs = error ? {
			...this.standardAttrs,
			[ATTR_ERROR_TYPE$1]: error
		} : this.standardAttrs;
		this.collectionDuration.record(durationSecs, attrs);
	}
};
var VERSION$2 = "2.11.0";
init_esm();
/**
* A registered reader of metrics that, when linked to a {@link MetricProducer}, offers global
* control over metrics.
*/
var MetricReader = class {
	_shutdown = false;
	_metricProducers;
	_sdkMetricProducer;
	_selfObsMetrics;
	_aggregationTemporalitySelector;
	_aggregationSelector;
	_cardinalitySelector;
	_otelComponentType;
	constructor(options) {
		this._aggregationSelector = options?.aggregationSelector ?? DEFAULT_AGGREGATION_SELECTOR;
		this._aggregationTemporalitySelector = options?.aggregationTemporalitySelector ?? DEFAULT_AGGREGATION_TEMPORALITY_SELECTOR;
		this._metricProducers = options?.metricProducers ?? [];
		this._cardinalitySelector = options?.cardinalitySelector;
		this._otelComponentType = options?.otelComponentType ?? this.constructor.name;
		this._selfObsMetrics = new MetricReaderMetrics(this._otelComponentType, createNoopMeter());
	}
	setMetricProducer(metricProducer) {
		if (this._sdkMetricProducer) throw new Error("MetricReader can not be bound to a MeterProvider again.");
		this._sdkMetricProducer = metricProducer;
		this.onInitialized();
	}
	_setSelfObsMeterProvider(meterProvider) {
		const meter = meterProvider.getMeter("@opentelemetry/sdk-metrics", VERSION$2);
		this._selfObsMetrics = new MetricReaderMetrics(this._otelComponentType, meter);
	}
	selectAggregation(instrumentType) {
		return this._aggregationSelector(instrumentType);
	}
	selectAggregationTemporality(instrumentType) {
		return this._aggregationTemporalitySelector(instrumentType);
	}
	selectCardinalityLimit(instrumentType) {
		return this._cardinalitySelector ? this._cardinalitySelector(instrumentType) : 2e3;
	}
	/**
	* Handle once the SDK has initialized this {@link MetricReader}
	* Overriding this method is optional.
	*/
	onInitialized() {}
	async collect(options) {
		if (this._sdkMetricProducer === void 0) throw new Error("MetricReader is not bound to a MetricProducer");
		if (this._shutdown) throw new Error("MetricReader is shutdown");
		const startTime = hrTime();
		const [sdkCollectionResults, ...additionalCollectionResults] = await Promise.all([this._sdkMetricProducer.collect({ timeoutMillis: options?.timeoutMillis }), ...this._metricProducers.map((producer) => producer.collect({ timeoutMillis: options?.timeoutMillis }))]);
		const endTime = hrTime();
		const errors = sdkCollectionResults.errors.concat(additionalCollectionResults.flatMap((result) => result.errors));
		const collectDuration = hrTimeToSeconds(hrTimeDuration(startTime, endTime));
		this._selfObsMetrics.recordCollection(collectDuration, errors.length > 0 ? errors[0].name ?? "collect_error" : void 0);
		return {
			resourceMetrics: {
				resource: sdkCollectionResults.resourceMetrics.resource,
				scopeMetrics: sdkCollectionResults.resourceMetrics.scopeMetrics.concat(additionalCollectionResults.flatMap((result) => result.resourceMetrics.scopeMetrics))
			},
			errors
		};
	}
	async shutdown(options) {
		if (this._shutdown) {
			diag.error("Cannot call shutdown twice.");
			return;
		}
		if (options?.timeoutMillis == null) await this.onShutdown();
		else await callWithTimeout(this.onShutdown(), options.timeoutMillis);
		this._shutdown = true;
	}
	async forceFlush(options) {
		if (this._shutdown) {
			diag.warn("Cannot forceFlush on already shutdown MetricReader.");
			return;
		}
		if (options?.timeoutMillis == null) {
			await this.onForceFlush();
			return;
		}
		await callWithTimeout(this.onForceFlush(), options.timeoutMillis);
	}
};
/**
* Splits a ResourceMetrics object into smaller ResourceMetrics objects
* such that no batch exceeds maxExportBatchSize data points.
* @param resourceMetrics The metrics to split.
* @param maxExportBatchSize The maximum number of data points per batch.
* @internal
*/
function splitMetricData(resourceMetrics, maxExportBatchSize) {
	if (!Number.isInteger(maxExportBatchSize) || maxExportBatchSize <= 0) throw new Error("maxExportBatchSize must be a positive integer");
	const batches = [];
	let currentBatchPoints = 0;
	let currentScopeMetrics = [];
	function flush() {
		if (currentScopeMetrics.length > 0) {
			batches.push({
				resource: resourceMetrics.resource,
				scopeMetrics: currentScopeMetrics
			});
			currentScopeMetrics = [];
			currentBatchPoints = 0;
		}
	}
	for (const scopeMetric of resourceMetrics.scopeMetrics) {
		let scopeMetricCopy = null;
		for (const metric of scopeMetric.metrics) {
			const dataPoints = metric.dataPoints;
			if (dataPoints.length === 0) {
				if (!scopeMetricCopy) {
					scopeMetricCopy = {
						scope: scopeMetric.scope,
						metrics: []
					};
					currentScopeMetrics.push(scopeMetricCopy);
				}
				scopeMetricCopy.metrics.push(metric);
				continue;
			}
			let offset = 0;
			while (offset < dataPoints.length) {
				const spaceLeft = maxExportBatchSize - currentBatchPoints;
				const take = Math.min(spaceLeft, dataPoints.length - offset);
				if (!scopeMetricCopy) {
					scopeMetricCopy = {
						scope: scopeMetric.scope,
						metrics: []
					};
					currentScopeMetrics.push(scopeMetricCopy);
				}
				const metricCopy = {
					...metric,
					dataPoints: dataPoints.slice(offset, offset + take)
				};
				scopeMetricCopy.metrics.push(metricCopy);
				offset += take;
				currentBatchPoints += take;
				if (currentBatchPoints === maxExportBatchSize) {
					flush();
					scopeMetricCopy = null;
				}
			}
		}
	}
	flush();
	return batches;
}
init_esm();
/**
* {@link MetricReader} which collects metrics based on a user-configurable time interval, and passes the metrics to
* the configured {@link PushMetricExporter}
*/
var PeriodicExportingMetricReader = class extends MetricReader {
	_interval;
	_exporter;
	_exportInterval;
	_exportTimeout;
	_maxExportBatchSize;
	_ongoingExportPromise = null;
	constructor(options) {
		const { exporter, exportIntervalMillis = 6e4, metricProducers, cardinalityLimits, maxExportBatchSize } = options;
		let { exportTimeoutMillis = 3e4 } = options;
		super({
			aggregationSelector: exporter.selectAggregation?.bind(exporter),
			aggregationTemporalitySelector: exporter.selectAggregationTemporality?.bind(exporter),
			otelComponentType: OTEL_COMPONENT_TYPE_VALUE_PERIODIC_METRIC_READER,
			metricProducers,
			cardinalitySelector: (instrumentType) => {
				const limits = {
					default: 2e3,
					...cardinalityLimits
				};
				switch (instrumentType) {
					case InstrumentType.COUNTER: return limits.counter ?? limits.default;
					case InstrumentType.GAUGE: return limits.gauge ?? limits.default;
					case InstrumentType.HISTOGRAM: return limits.histogram ?? limits.default;
					case InstrumentType.OBSERVABLE_COUNTER: return limits.observableCounter ?? limits.default;
					case InstrumentType.OBSERVABLE_UP_DOWN_COUNTER: return limits.observableUpDownCounter ?? limits.default;
					case InstrumentType.OBSERVABLE_GAUGE: return limits.observableGauge ?? limits.default;
					case InstrumentType.UP_DOWN_COUNTER: return limits.upDownCounter ?? limits.default;
					default: return limits.default;
				}
			}
		});
		if (exportIntervalMillis <= 0) throw Error("exportIntervalMillis must be greater than 0");
		if (exportTimeoutMillis <= 0) throw Error("exportTimeoutMillis must be greater than 0");
		if (maxExportBatchSize !== void 0 && (!Number.isInteger(maxExportBatchSize) || maxExportBatchSize <= 0)) throw Error("maxExportBatchSize must be a positive integer");
		if (exportIntervalMillis < exportTimeoutMillis) {
			if ("exportIntervalMillis" in options && "exportTimeoutMillis" in options) throw Error("exportIntervalMillis must be greater than or equal to exportTimeoutMillis");
			else {
				diag.info(`Timeout of ${exportTimeoutMillis} exceeds the interval of ${exportIntervalMillis}. Clamping timeout to interval duration.`);
				exportTimeoutMillis = exportIntervalMillis;
			}
		}
		this._exportInterval = exportIntervalMillis;
		this._exportTimeout = exportTimeoutMillis;
		this._exporter = exporter;
		this._maxExportBatchSize = maxExportBatchSize;
	}
	async _runOnce() {
		try {
			await this._doRun();
		} catch (err) {
			globalErrorHandler(err);
		}
	}
	async _doRun() {
		if (this._ongoingExportPromise) {
			diag.debug("PeriodicExportingMetricReader: export already in progress, skipping");
			return;
		}
		const currentRun = async () => {
			const { resourceMetrics, errors } = await this.collect({ timeoutMillis: this._exportTimeout });
			if (errors.length > 0) diag.error("PeriodicExportingMetricReader: metrics collection errors", ...errors);
			if (resourceMetrics.resource.asyncAttributesPending) try {
				await resourceMetrics.resource.waitForAsyncAttributes?.();
			} catch (e) {
				diag.debug("Error while resolving async portion of resource: ", e);
				globalErrorHandler(e);
			}
			if (resourceMetrics.scopeMetrics.length === 0) return;
			const batches = this._maxExportBatchSize ? splitMetricData(resourceMetrics, this._maxExportBatchSize) : [resourceMetrics];
			let anyErr = null;
			for (const batch of batches) try {
				const result = await callWithTimeout(internal._export(this._exporter, batch), this._exportTimeout);
				if (result.code !== ExportResultCode.SUCCESS) anyErr = /* @__PURE__ */ new Error(`PeriodicExportingMetricReader: metrics export failed (error ${result.error})`);
			} catch (e) {
				if (e instanceof TimeoutError) {
					diag.error(`PeriodicExportingMetricReader: metrics export timed out after ${this._exportTimeout}ms`);
					break;
				} else {
					diag.error("PeriodicExportingMetricReader: metrics export threw error", e);
					anyErr = e instanceof Error ? e : new Error(String(e));
				}
			}
			if (anyErr) throw anyErr;
		};
		this._ongoingExportPromise = currentRun();
		try {
			await this._ongoingExportPromise;
		} finally {
			this._ongoingExportPromise = null;
		}
	}
	onInitialized() {
		this._interval = setInterval(() => {
			this._runOnce();
		}, this._exportInterval);
		if (typeof this._interval !== "number") this._interval.unref();
	}
	async onForceFlush() {
		await this._awaitOngoingExport();
		if (this._ongoingExportPromise) await this._awaitOngoingExport();
		else await this._runOnce();
		await this._exporter.forceFlush();
	}
	/**
	* Helper function to wait for an ongoing export to complete.
	* Errors are swallowed and handled by the original _runOnce().
	*/
	async _awaitOngoingExport() {
		if (this._ongoingExportPromise) {
			diag.debug("PeriodicExportingMetricReader: export already in progress, awaiting ongoing export");
			try {
				await this._ongoingExportPromise;
			} catch {}
		}
	}
	async onShutdown() {
		if (this._interval) clearInterval(this._interval);
		await this.onForceFlush();
		await this._exporter.shutdown();
	}
};
var serviceName;
/**
* Returns the default service name for OpenTelemetry resources.
* In Node.js environments, returns "unknown_service:<process.argv0>".
* In browser/edge environments, returns "unknown_service".
*/
function defaultServiceName() {
	if (serviceName === void 0) try {
		const argv0 = globalThis.process.argv0;
		serviceName = argv0 ? `unknown_service:${argv0}` : "unknown_service";
	} catch {
		serviceName = "unknown_service";
	}
	return serviceName;
}
var isPromiseLike = (val) => {
	return val !== null && typeof val === "object" && typeof val.then === "function";
};
init_esm();
var ResourceImpl = class ResourceImpl {
	_rawAttributes;
	_asyncAttributesPending = false;
	_schemaUrl;
	_memoizedAttributes;
	static FromAttributeList(attributes, options) {
		const res = new ResourceImpl({}, options);
		res._rawAttributes = guardedRawAttributes(attributes);
		res._asyncAttributesPending = attributes.filter(([_, val]) => isPromiseLike(val)).length > 0;
		return res;
	}
	constructor(resource, options) {
		const attributes = resource.attributes ?? {};
		this._rawAttributes = Object.entries(attributes).map(([k, v]) => {
			if (isPromiseLike(v)) this._asyncAttributesPending = true;
			return [k, v];
		});
		this._rawAttributes = guardedRawAttributes(this._rawAttributes);
		this._schemaUrl = validateSchemaUrl(options?.schemaUrl);
	}
	get asyncAttributesPending() {
		return this._asyncAttributesPending;
	}
	async waitForAsyncAttributes() {
		if (!this.asyncAttributesPending) return;
		for (let i = 0; i < this._rawAttributes.length; i++) {
			const [k, v] = this._rawAttributes[i];
			this._rawAttributes[i] = [k, isPromiseLike(v) ? await v : v];
		}
		this._asyncAttributesPending = false;
	}
	get attributes() {
		if (this.asyncAttributesPending) diag.error("Accessing resource attributes before async attributes settled");
		if (this._memoizedAttributes) return this._memoizedAttributes;
		const attrs = {};
		for (const [k, v] of this._rawAttributes) {
			if (isPromiseLike(v)) {
				diag.debug(`Unsettled resource attribute ${k} skipped`);
				continue;
			}
			if (v != null) attrs[k] ??= v;
		}
		if (!this._asyncAttributesPending) this._memoizedAttributes = attrs;
		return attrs;
	}
	getRawAttributes() {
		return this._rawAttributes;
	}
	get schemaUrl() {
		return this._schemaUrl;
	}
	merge(resource) {
		if (resource == null) return this;
		const mergedSchemaUrl = mergeSchemaUrl(this, resource);
		const mergedOptions = mergedSchemaUrl ? { schemaUrl: mergedSchemaUrl } : void 0;
		return ResourceImpl.FromAttributeList([...resource.getRawAttributes(), ...this.getRawAttributes()], mergedOptions);
	}
};
function resourceFromAttributes(attributes, options) {
	return ResourceImpl.FromAttributeList(Object.entries(attributes), options);
}
function defaultResource() {
	return resourceFromAttributes({
		[ATTR_SERVICE_NAME]: defaultServiceName(),
		[ATTR_TELEMETRY_SDK_LANGUAGE]: SDK_INFO[ATTR_TELEMETRY_SDK_LANGUAGE],
		[ATTR_TELEMETRY_SDK_NAME]: SDK_INFO[ATTR_TELEMETRY_SDK_NAME],
		[ATTR_TELEMETRY_SDK_VERSION]: SDK_INFO[ATTR_TELEMETRY_SDK_VERSION]
	});
}
function guardedRawAttributes(attributes) {
	return attributes.map(([k, v]) => {
		if (isPromiseLike(v)) return [k, v.catch((err) => {
			diag.debug("promise rejection for resource attribute: %s - %s", k, err);
		})];
		return [k, v];
	});
}
function validateSchemaUrl(schemaUrl) {
	if (typeof schemaUrl === "string" || schemaUrl === void 0) return schemaUrl;
	diag.warn("Schema URL must be string or undefined, got %s. Schema URL will be ignored.", schemaUrl);
}
function mergeSchemaUrl(old, updating) {
	const oldSchemaUrl = old?.schemaUrl;
	const updatingSchemaUrl = updating?.schemaUrl;
	const isOldEmpty = oldSchemaUrl === void 0 || oldSchemaUrl === "";
	const isUpdatingEmpty = updatingSchemaUrl === void 0 || updatingSchemaUrl === "";
	if (isOldEmpty) return updatingSchemaUrl;
	if (isUpdatingEmpty) return oldSchemaUrl;
	if (oldSchemaUrl === updatingSchemaUrl) return oldSchemaUrl;
	diag.warn("Schema URL merge conflict: old resource has \"%s\", updating resource has \"%s\". Resulting resource will have undefined Schema URL.", oldSchemaUrl, updatingSchemaUrl);
}
var ViewRegistry = class {
	_registeredViews = [];
	addView(view) {
		this._registeredViews.push(view);
	}
	findViews(instrument, meter) {
		return this._registeredViews.filter((registeredView) => {
			return this._matchInstrument(registeredView.instrumentSelector, instrument) && this._matchMeter(registeredView.meterSelector, meter);
		});
	}
	_matchInstrument(selector, instrument) {
		return (selector.getType() === void 0 || instrument.type === selector.getType()) && selector.getNameFilter().match(instrument.name) && selector.getUnitFilter().match(instrument.unit);
	}
	_matchMeter(selector, meter) {
		return selector.getNameFilter().match(meter.name) && (meter.version === void 0 || selector.getVersionFilter().match(meter.version)) && (meter.schemaUrl === void 0 || selector.getSchemaUrlFilter().match(meter.schemaUrl));
	}
};
init_esm();
function createInstrumentDescriptor(name, type, options) {
	if (!isValidName(name)) diag.warn(`Invalid metric name: "${name}". The metric name should be a ASCII string with a length no greater than 255 characters.`);
	return {
		name,
		type,
		description: options?.description ?? "",
		unit: options?.unit ?? "",
		valueType: options?.valueType ?? ValueType.DOUBLE,
		advice: options?.advice ?? {}
	};
}
function createInstrumentDescriptorWithView(view, instrument) {
	return {
		name: view.name ?? instrument.name,
		description: view.description ?? instrument.description,
		type: instrument.type,
		unit: instrument.unit,
		valueType: instrument.valueType,
		advice: instrument.advice
	};
}
function isDescriptorCompatibleWith(descriptor, otherDescriptor) {
	return equalsCaseInsensitive(descriptor.name, otherDescriptor.name) && descriptor.unit === otherDescriptor.unit && descriptor.type === otherDescriptor.type && descriptor.valueType === otherDescriptor.valueType;
}
var NAME_REGEXP = /^[a-z][a-z0-9_.\-/]{0,254}$/i;
function isValidName(name) {
	return NAME_REGEXP.test(name);
}
init_esm();
var SyncInstrument = class {
	_writableMetricStorage;
	_descriptor;
	constructor(writableMetricStorage, descriptor) {
		this._writableMetricStorage = writableMetricStorage;
		this._descriptor = descriptor;
	}
	_record(value, attributes = {}, context) {
		if (typeof value !== "number") {
			diag.warn(`non-number value provided to metric ${this._descriptor.name}: ${value}`);
			return;
		}
		if (this._descriptor.valueType === ValueType.INT && !Number.isInteger(value)) {
			diag.warn(`INT value type cannot accept a floating-point value for ${this._descriptor.name}, ignoring the fractional digits.`);
			value = Math.trunc(value);
			if (!Number.isInteger(value)) return;
		}
		this._writableMetricStorage.record(value, attributes, context, Date.now());
	}
};
/**
* The class implements {@link UpDownCounter} interface.
*/
var UpDownCounterInstrument = class extends SyncInstrument {
	/**
	* Increment value of counter by the input. Inputs may be negative.
	*/
	add(value, attributes, ctx) {
		this._record(value, attributes, ctx);
	}
};
/**
* The class implements {@link Counter} interface.
*/
var CounterInstrument = class extends SyncInstrument {
	/**
	* Increment value of counter by the input. Inputs may not be negative.
	*/
	add(value, attributes, ctx) {
		if (value < 0) {
			diag.warn(`negative value provided to counter ${this._descriptor.name}: ${value}`);
			return;
		}
		this._record(value, attributes, ctx);
	}
};
/**
* The class implements {@link Gauge} interface.
*/
var GaugeInstrument = class extends SyncInstrument {
	/**
	* Records a measurement.
	*/
	record(value, attributes, ctx) {
		this._record(value, attributes, ctx);
	}
};
/**
* The class implements {@link Histogram} interface.
*/
var HistogramInstrument = class extends SyncInstrument {
	/**
	* Records a measurement. Value of the measurement must not be negative.
	*/
	record(value, attributes, ctx) {
		if (value < 0) {
			diag.warn(`negative value provided to histogram ${this._descriptor.name}: ${value}`);
			return;
		}
		this._record(value, attributes, ctx);
	}
};
var ObservableInstrument = class {
	/** @internal */
	_metricStorages;
	/** @internal */
	_descriptor;
	_observableRegistry;
	constructor(descriptor, metricStorages, observableRegistry) {
		this._descriptor = descriptor;
		this._metricStorages = metricStorages;
		this._observableRegistry = observableRegistry;
	}
	/**
	* @see {Observable.addCallback}
	*/
	addCallback(callback) {
		this._observableRegistry.addCallback(callback, this);
	}
	/**
	* @see {Observable.removeCallback}
	*/
	removeCallback(callback) {
		this._observableRegistry.removeCallback(callback, this);
	}
};
var ObservableCounterInstrument = class extends ObservableInstrument {};
var ObservableGaugeInstrument = class extends ObservableInstrument {};
var ObservableUpDownCounterInstrument = class extends ObservableInstrument {};
function isObservableInstrument(it) {
	return it instanceof ObservableInstrument;
}
/**
* This class implements the {@link IMeter} interface.
*/
var Meter = class {
	_meterSharedState;
	constructor(meterSharedState) {
		this._meterSharedState = meterSharedState;
	}
	/**
	* Create a {@link Gauge} instrument.
	*/
	createGauge(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.GAUGE, options);
		return new GaugeInstrument(this._meterSharedState.registerMetricStorage(descriptor), descriptor);
	}
	/**
	* Create a {@link Histogram} instrument.
	*/
	createHistogram(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.HISTOGRAM, options);
		return new HistogramInstrument(this._meterSharedState.registerMetricStorage(descriptor), descriptor);
	}
	/**
	* Create a {@link Counter} instrument.
	*/
	createCounter(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.COUNTER, options);
		return new CounterInstrument(this._meterSharedState.registerMetricStorage(descriptor), descriptor);
	}
	/**
	* Create a {@link UpDownCounter} instrument.
	*/
	createUpDownCounter(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.UP_DOWN_COUNTER, options);
		return new UpDownCounterInstrument(this._meterSharedState.registerMetricStorage(descriptor), descriptor);
	}
	/**
	* Create a {@link ObservableGauge} instrument.
	*/
	createObservableGauge(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.OBSERVABLE_GAUGE, options);
		return new ObservableGaugeInstrument(descriptor, this._meterSharedState.registerAsyncMetricStorage(descriptor), this._meterSharedState.observableRegistry);
	}
	/**
	* Create a {@link ObservableCounter} instrument.
	*/
	createObservableCounter(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.OBSERVABLE_COUNTER, options);
		return new ObservableCounterInstrument(descriptor, this._meterSharedState.registerAsyncMetricStorage(descriptor), this._meterSharedState.observableRegistry);
	}
	/**
	* Create a {@link ObservableUpDownCounter} instrument.
	*/
	createObservableUpDownCounter(name, options) {
		const descriptor = createInstrumentDescriptor(name, InstrumentType.OBSERVABLE_UP_DOWN_COUNTER, options);
		return new ObservableUpDownCounterInstrument(descriptor, this._meterSharedState.registerAsyncMetricStorage(descriptor), this._meterSharedState.observableRegistry);
	}
	/**
	* @see {@link Meter.addBatchObservableCallback}
	*/
	addBatchObservableCallback(callback, observables) {
		this._meterSharedState.observableRegistry.addBatchCallback(callback, observables);
	}
	/**
	* @see {@link Meter.removeBatchObservableCallback}
	*/
	removeBatchObservableCallback(callback, observables) {
		this._meterSharedState.observableRegistry.removeBatchCallback(callback, observables);
	}
};
/**
* Internal interface.
*
* Represents a storage from which we can collect metrics.
*/
var MetricStorage = class {
	_instrumentDescriptor;
	constructor(instrumentDescriptor) {
		this._instrumentDescriptor = instrumentDescriptor;
	}
	getInstrumentDescriptor() {
		return this._instrumentDescriptor;
	}
	updateDescription(description) {
		this._instrumentDescriptor = createInstrumentDescriptor(this._instrumentDescriptor.name, this._instrumentDescriptor.type, {
			description,
			valueType: this._instrumentDescriptor.valueType,
			unit: this._instrumentDescriptor.unit,
			advice: this._instrumentDescriptor.advice
		});
	}
};
var HashMap = class {
	_valueMap = /* @__PURE__ */ new Map();
	_keyMap = /* @__PURE__ */ new Map();
	_hash;
	constructor(hash) {
		this._hash = hash;
	}
	get(key, hashCode) {
		hashCode ??= this._hash(key);
		return this._valueMap.get(hashCode);
	}
	getOrDefault(key, defaultFactory) {
		const hash = this._hash(key);
		if (this._valueMap.has(hash)) return this._valueMap.get(hash);
		const val = defaultFactory();
		if (!this._keyMap.has(hash)) this._keyMap.set(hash, key);
		this._valueMap.set(hash, val);
		return val;
	}
	set(key, value, hashCode) {
		hashCode ??= this._hash(key);
		if (!this._keyMap.has(hashCode)) this._keyMap.set(hashCode, key);
		this._valueMap.set(hashCode, value);
	}
	has(key, hashCode) {
		hashCode ??= this._hash(key);
		return this._valueMap.has(hashCode);
	}
	*keys() {
		const keyIterator = this._keyMap.entries();
		let next = keyIterator.next();
		while (next.done !== true) {
			yield [next.value[1], next.value[0]];
			next = keyIterator.next();
		}
	}
	*entries() {
		const valueIterator = this._valueMap.entries();
		let next = valueIterator.next();
		while (next.done !== true) {
			yield [
				this._keyMap.get(next.value[0]),
				next.value[1],
				next.value[0]
			];
			next = valueIterator.next();
		}
	}
	get size() {
		return this._valueMap.size;
	}
};
var AttributeHashMap = class extends HashMap {
	constructor() {
		super(hashAttributes);
	}
};
/**
* Internal interface.
*
* Allows synchronous collection of metrics. This processor should allow
* allocation of new aggregation cells for metrics and convert cumulative
* recording to delta data points.
*/
var DeltaMetricProcessor = class {
	_activeCollectionStorage = new AttributeHashMap();
	_cumulativeMemoStorage = new AttributeHashMap();
	_cardinalityLimit;
	_overflowAttributes = { "otel.metric.overflow": true };
	_overflowHashCode;
	_aggregator;
	constructor(aggregator, aggregationCardinalityLimit) {
		this._aggregator = aggregator;
		this._cardinalityLimit = (aggregationCardinalityLimit ?? 2e3) - 1;
		this._overflowHashCode = hashAttributes(this._overflowAttributes);
	}
	record(value, attributes, collectionTime) {
		let accumulation = this._activeCollectionStorage.get(attributes);
		if (!accumulation) {
			const hrTime = millisToHrTime(collectionTime);
			if (this._activeCollectionStorage.size >= this._cardinalityLimit) {
				this._activeCollectionStorage.getOrDefault(this._overflowAttributes, () => this._aggregator.createAccumulation(hrTime))?.record(value);
				return;
			}
			accumulation = this._aggregator.createAccumulation(hrTime);
			this._activeCollectionStorage.set(attributes, accumulation);
		}
		accumulation?.record(value);
	}
	batchCumulate(measurements, collectionTime) {
		for (const [originalAttributes, value, originalHashCode] of measurements.entries()) {
			let attributes = originalAttributes;
			let hashCode = originalHashCode;
			const accumulation = this._aggregator.createAccumulation(collectionTime);
			accumulation?.record(value);
			let delta = accumulation;
			if (this._cumulativeMemoStorage.has(attributes, hashCode)) {
				const previous = this._cumulativeMemoStorage.get(attributes, hashCode);
				delta = this._aggregator.diff(previous, accumulation);
			} else if (this._cumulativeMemoStorage.size >= this._cardinalityLimit) {
				attributes = this._overflowAttributes;
				hashCode = this._overflowHashCode;
				if (this._cumulativeMemoStorage.has(attributes, hashCode)) {
					const previous = this._cumulativeMemoStorage.get(attributes, hashCode);
					delta = this._aggregator.diff(previous, accumulation);
				}
			}
			if (this._activeCollectionStorage.has(attributes, hashCode)) {
				const active = this._activeCollectionStorage.get(attributes, hashCode);
				delta = this._aggregator.merge(active, delta);
			}
			this._cumulativeMemoStorage.set(attributes, accumulation, hashCode);
			this._activeCollectionStorage.set(attributes, delta, hashCode);
		}
	}
	/**
	* Returns a collection of delta metrics. Start time is the when first
	* time event collected.
	*/
	collect() {
		const unreportedDelta = this._activeCollectionStorage;
		this._activeCollectionStorage = new AttributeHashMap();
		return unreportedDelta;
	}
};
/**
* Internal interface.
*
* Provides unique reporting for each collector. Allows synchronous collection
* of metrics and reports given temporality values.
*/
var TemporalMetricProcessor = class TemporalMetricProcessor {
	_aggregator;
	_unreportedAccumulations = /* @__PURE__ */ new Map();
	_reportHistory = /* @__PURE__ */ new Map();
	constructor(aggregator, collectorHandles) {
		this._aggregator = aggregator;
		collectorHandles.forEach((handle) => {
			this._unreportedAccumulations.set(handle, []);
		});
	}
	/**
	* Builds the {@link MetricData} streams to report against a specific MetricCollector.
	* @param collector The information of the MetricCollector.
	* @param collectors The registered collectors.
	* @param instrumentDescriptor The instrumentation descriptor that these metrics generated with.
	* @param currentAccumulations The current accumulation of metric data from instruments.
	* @param collectionTime The current collection timestamp.
	* @returns The {@link MetricData} points or `null`.
	*/
	buildMetrics(collector, instrumentDescriptor, currentAccumulations, collectionTime) {
		this._stashAccumulations(currentAccumulations);
		const unreportedAccumulations = this._getMergedUnreportedAccumulations(collector);
		let result = unreportedAccumulations;
		let aggregationTemporality;
		if (this._reportHistory.has(collector)) {
			const last = this._reportHistory.get(collector);
			const lastCollectionTime = last.collectionTime;
			aggregationTemporality = last.aggregationTemporality;
			if (aggregationTemporality === AggregationTemporality.CUMULATIVE) result = TemporalMetricProcessor.merge(last.accumulations, unreportedAccumulations, this._aggregator);
			else result = TemporalMetricProcessor.calibrateStartTime(last.accumulations, unreportedAccumulations, lastCollectionTime);
		} else aggregationTemporality = collector.selectAggregationTemporality(instrumentDescriptor.type);
		this._reportHistory.set(collector, {
			accumulations: result,
			collectionTime,
			aggregationTemporality
		});
		const accumulationRecords = AttributesMapToAccumulationRecords(result);
		if (accumulationRecords.length === 0) return;
		return this._aggregator.toMetricData(instrumentDescriptor, aggregationTemporality, accumulationRecords, collectionTime);
	}
	_stashAccumulations(currentAccumulation) {
		const registeredCollectors = this._unreportedAccumulations.keys();
		for (const collector of registeredCollectors) {
			let stash = this._unreportedAccumulations.get(collector);
			if (stash === void 0) {
				stash = [];
				this._unreportedAccumulations.set(collector, stash);
			}
			stash.push(currentAccumulation);
		}
	}
	_getMergedUnreportedAccumulations(collector) {
		let result = new AttributeHashMap();
		const unreportedList = this._unreportedAccumulations.get(collector);
		this._unreportedAccumulations.set(collector, []);
		if (unreportedList === void 0) return result;
		for (const it of unreportedList) result = TemporalMetricProcessor.merge(result, it, this._aggregator);
		return result;
	}
	static merge(last, current, aggregator) {
		const result = last;
		const iterator = current.entries();
		let next = iterator.next();
		while (next.done !== true) {
			const [key, record, hash] = next.value;
			if (last.has(key, hash)) {
				const lastAccumulation = last.get(key, hash);
				const accumulation = aggregator.merge(lastAccumulation, record);
				result.set(key, accumulation, hash);
			} else result.set(key, record, hash);
			next = iterator.next();
		}
		return result;
	}
	/**
	* Calibrate the reported metric streams' startTime to lastCollectionTime. Leaves
	* the new stream to be the initial observation time unchanged.
	*/
	static calibrateStartTime(last, current, lastCollectionTime) {
		for (const [key, hash] of last.keys()) current.get(key, hash)?.setStartTime(lastCollectionTime);
		return current;
	}
};
function AttributesMapToAccumulationRecords(map) {
	return Array.from(map.entries());
}
/**
* Internal interface.
*
* Stores and aggregates {@link MetricData} for asynchronous instruments.
*/
var AsyncMetricStorage = class extends MetricStorage {
	_aggregationCardinalityLimit;
	_deltaMetricStorage;
	_temporalMetricStorage;
	_attributesProcessor;
	constructor(_instrumentDescriptor, aggregator, attributesProcessor, collectorHandles, aggregationCardinalityLimit) {
		super(_instrumentDescriptor);
		this._aggregationCardinalityLimit = aggregationCardinalityLimit;
		this._deltaMetricStorage = new DeltaMetricProcessor(aggregator, this._aggregationCardinalityLimit);
		this._temporalMetricStorage = new TemporalMetricProcessor(aggregator, collectorHandles);
		this._attributesProcessor = attributesProcessor;
	}
	record(measurements, observationTime) {
		if (this._attributesProcessor === void 0) {
			this._deltaMetricStorage.batchCumulate(measurements, observationTime);
			return;
		}
		const processed = new AttributeHashMap();
		for (const [attributes, value] of measurements.entries()) processed.set(this._attributesProcessor.process(attributes), value);
		this._deltaMetricStorage.batchCumulate(processed, observationTime);
	}
	/**
	* Collects the metrics from this storage. The ObservableCallback is invoked
	* during the collection.
	*
	* Note: This is a stateful operation and may reset any interval-related
	* state for the MetricCollector.
	*/
	collect(collector, collectionTime) {
		const accumulations = this._deltaMetricStorage.collect();
		return this._temporalMetricStorage.buildMetrics(collector, this._instrumentDescriptor, accumulations, collectionTime);
	}
};
function getIncompatibilityDetails(existing, otherDescriptor) {
	let incompatibility = "";
	if (existing.unit !== otherDescriptor.unit) incompatibility += `\t- Unit '${existing.unit}' does not match '${otherDescriptor.unit}'\n`;
	if (existing.type !== otherDescriptor.type) incompatibility += `\t- Type '${existing.type}' does not match '${otherDescriptor.type}'\n`;
	if (existing.valueType !== otherDescriptor.valueType) incompatibility += `\t- Value Type '${existing.valueType}' does not match '${otherDescriptor.valueType}'\n`;
	if (existing.description !== otherDescriptor.description) incompatibility += `\t- Description '${existing.description}' does not match '${otherDescriptor.description}'\n`;
	return incompatibility;
}
function getValueTypeConflictResolutionRecipe(existing, otherDescriptor) {
	return `\t- use valueType '${existing.valueType}' on instrument creation or use an instrument name other than '${otherDescriptor.name}'`;
}
function getUnitConflictResolutionRecipe(existing, otherDescriptor) {
	return `\t- use unit '${existing.unit}' on instrument creation or use an instrument name other than '${otherDescriptor.name}'`;
}
function getTypeConflictResolutionRecipe(existing, otherDescriptor) {
	const selector = {
		name: otherDescriptor.name,
		type: otherDescriptor.type,
		unit: otherDescriptor.unit
	};
	const selectorString = JSON.stringify(selector);
	return `\t- create a new view with a name other than '${existing.name}' and InstrumentSelector '${selectorString}'`;
}
function getDescriptionResolutionRecipe(existing, otherDescriptor) {
	const selector = {
		name: otherDescriptor.name,
		type: otherDescriptor.type,
		unit: otherDescriptor.unit
	};
	const selectorString = JSON.stringify(selector);
	return `\t- create a new view with a name other than '${existing.name}' and InstrumentSelector '${selectorString}'
    \t- OR - create a new view with the name ${existing.name} and description '${existing.description}' and InstrumentSelector ${selectorString}
    \t- OR - create a new view with the name ${otherDescriptor.name} and description '${existing.description}' and InstrumentSelector ${selectorString}`;
}
function getConflictResolutionRecipe(existing, otherDescriptor) {
	if (existing.valueType !== otherDescriptor.valueType) return getValueTypeConflictResolutionRecipe(existing, otherDescriptor);
	if (existing.unit !== otherDescriptor.unit) return getUnitConflictResolutionRecipe(existing, otherDescriptor);
	if (existing.type !== otherDescriptor.type) return getTypeConflictResolutionRecipe(existing, otherDescriptor);
	if (existing.description !== otherDescriptor.description) return getDescriptionResolutionRecipe(existing, otherDescriptor);
	return "";
}
init_esm();
/**
* Internal class for storing {@link MetricStorage}
*/
var MetricStorageRegistry = class MetricStorageRegistry {
	_sharedRegistry = /* @__PURE__ */ new Map();
	_perCollectorRegistry = /* @__PURE__ */ new Map();
	static create() {
		return new MetricStorageRegistry();
	}
	getStorages(collector) {
		let storages = [];
		for (const metricStorages of this._sharedRegistry.values()) storages = storages.concat(metricStorages);
		const perCollectorStorages = this._perCollectorRegistry.get(collector);
		if (perCollectorStorages != null) for (const metricStorages of perCollectorStorages.values()) storages = storages.concat(metricStorages);
		return storages;
	}
	register(storage) {
		this._registerStorage(storage, this._sharedRegistry);
	}
	registerForCollector(collector, storage) {
		let storageMap = this._perCollectorRegistry.get(collector);
		if (storageMap == null) {
			storageMap = /* @__PURE__ */ new Map();
			this._perCollectorRegistry.set(collector, storageMap);
		}
		this._registerStorage(storage, storageMap);
	}
	findOrUpdateCompatibleStorage(expectedDescriptor) {
		const storages = this._sharedRegistry.get(expectedDescriptor.name);
		if (storages === void 0) return null;
		return this._findOrUpdateCompatibleStorage(expectedDescriptor, storages);
	}
	findOrUpdateCompatibleCollectorStorage(collector, expectedDescriptor) {
		const storageMap = this._perCollectorRegistry.get(collector);
		if (storageMap === void 0) return null;
		const storages = storageMap.get(expectedDescriptor.name);
		if (storages === void 0) return null;
		return this._findOrUpdateCompatibleStorage(expectedDescriptor, storages);
	}
	_registerStorage(storage, storageMap) {
		const descriptor = storage.getInstrumentDescriptor();
		const storages = storageMap.get(descriptor.name);
		if (storages === void 0) {
			storageMap.set(descriptor.name, [storage]);
			return;
		}
		storages.push(storage);
	}
	_findOrUpdateCompatibleStorage(expectedDescriptor, existingStorages) {
		let compatibleStorage = null;
		for (const existingStorage of existingStorages) {
			const existingDescriptor = existingStorage.getInstrumentDescriptor();
			if (isDescriptorCompatibleWith(existingDescriptor, expectedDescriptor)) {
				if (existingDescriptor.description !== expectedDescriptor.description) {
					if (expectedDescriptor.description.length > existingDescriptor.description.length) existingStorage.updateDescription(expectedDescriptor.description);
					diag.warn("A view or instrument with the name ", expectedDescriptor.name, " has already been registered, but has a different description and is incompatible with another registered view.\n", "Details:\n", getIncompatibilityDetails(existingDescriptor, expectedDescriptor), "The longer description will be used.\nTo resolve the conflict:", getConflictResolutionRecipe(existingDescriptor, expectedDescriptor));
				}
				compatibleStorage = existingStorage;
			} else diag.warn("A view or instrument with the name ", expectedDescriptor.name, " has already been registered and is incompatible with another registered view.\n", "Details:\n", getIncompatibilityDetails(existingDescriptor, expectedDescriptor), "To resolve the conflict:\n", getConflictResolutionRecipe(existingDescriptor, expectedDescriptor));
		}
		return compatibleStorage;
	}
};
init_esm();
/**
* Internal interface.
*/
var MultiMetricStorage = class {
	_backingStorages;
	hasAttributeProcessor;
	constructor(backingStorages) {
		this._backingStorages = backingStorages;
		this.hasAttributeProcessor = backingStorages.some((s) => s.hasAttributeProcessor);
	}
	record(value, attributes, context$3, recordTime) {
		if (this.hasAttributeProcessor && context$3 === void 0) context$3 = context.active();
		const storages = this._backingStorages;
		for (let i = 0; i < storages.length; i++) storages[i].record(value, attributes, context$3, recordTime);
	}
};
init_esm();
/**
* The class implements {@link ObservableResult} interface.
*/
var ObservableResultImpl = class {
	/**
	* @internal
	*/
	_buffer = new AttributeHashMap();
	_instrumentName;
	_valueType;
	constructor(instrumentName, valueType) {
		this._instrumentName = instrumentName;
		this._valueType = valueType;
	}
	/**
	* Observe a measurement of the value associated with the given attributes.
	*/
	observe(value, attributes = {}) {
		if (typeof value !== "number") {
			diag.warn(`non-number value provided to metric ${this._instrumentName}: ${value}`);
			return;
		}
		if (this._valueType === ValueType.INT && !Number.isInteger(value)) {
			diag.warn(`INT value type cannot accept a floating-point value for ${this._instrumentName}, ignoring the fractional digits.`);
			value = Math.trunc(value);
			if (!Number.isInteger(value)) return;
		}
		this._buffer.set(attributes, value);
	}
};
/**
* The class implements {@link BatchObservableCallback} interface.
*/
var BatchObservableResultImpl = class {
	/**
	* @internal
	*/
	_buffer = /* @__PURE__ */ new Map();
	/**
	* Observe a measurement of the value associated with the given attributes.
	*/
	observe(metric, value, attributes = {}) {
		if (!isObservableInstrument(metric)) return;
		let map = this._buffer.get(metric);
		if (map == null) {
			map = new AttributeHashMap();
			this._buffer.set(metric, map);
		}
		if (typeof value !== "number") {
			diag.warn(`non-number value provided to metric ${metric._descriptor.name}: ${value}`);
			return;
		}
		if (metric._descriptor.valueType === ValueType.INT && !Number.isInteger(value)) {
			diag.warn(`INT value type cannot accept a floating-point value for ${metric._descriptor.name}, ignoring the fractional digits.`);
			value = Math.trunc(value);
			if (!Number.isInteger(value)) return;
		}
		map.set(attributes, value);
	}
};
init_esm();
/**
* An internal interface for managing ObservableCallbacks.
*
* Every registered callback associated with a set of instruments are be evaluated
* exactly once during collection prior to reading data for that instrument.
*/
var ObservableRegistry = class {
	_callbacks = [];
	_batchCallbacks = [];
	addCallback(callback, instrument) {
		if (this._findCallback(callback, instrument) >= 0) return;
		this._callbacks.push({
			callback,
			instrument
		});
	}
	removeCallback(callback, instrument) {
		const idx = this._findCallback(callback, instrument);
		if (idx < 0) return;
		this._callbacks.splice(idx, 1);
	}
	addBatchCallback(callback, instruments) {
		const observableInstruments = new Set(instruments.filter(isObservableInstrument));
		if (observableInstruments.size === 0) {
			diag.error("BatchObservableCallback is not associated with valid instruments", instruments);
			return;
		}
		if (this._findBatchCallback(callback, observableInstruments) >= 0) return;
		this._batchCallbacks.push({
			callback,
			instruments: observableInstruments
		});
	}
	removeBatchCallback(callback, instruments) {
		const observableInstruments = new Set(instruments.filter(isObservableInstrument));
		const idx = this._findBatchCallback(callback, observableInstruments);
		if (idx < 0) return;
		this._batchCallbacks.splice(idx, 1);
	}
	/**
	* @returns a promise of rejected reasons for invoking callbacks.
	*/
	async observe(collectionTime, timeoutMillis) {
		const callbackFutures = this._observeCallbacks(collectionTime, timeoutMillis);
		const batchCallbackFutures = this._observeBatchCallbacks(collectionTime, timeoutMillis);
		return (await Promise.allSettled([...callbackFutures, ...batchCallbackFutures])).filter((result) => result.status === "rejected").map((result) => result.reason);
	}
	_observeCallbacks(observationTime, timeoutMillis) {
		return this._callbacks.map(async ({ callback, instrument }) => {
			const observableResult = new ObservableResultImpl(instrument._descriptor.name, instrument._descriptor.valueType);
			let callPromise = Promise.resolve(callback(observableResult));
			if (timeoutMillis != null) callPromise = callWithTimeout(callPromise, timeoutMillis);
			await callPromise;
			instrument._metricStorages.forEach((metricStorage) => {
				metricStorage.record(observableResult._buffer, observationTime);
			});
		});
	}
	_observeBatchCallbacks(observationTime, timeoutMillis) {
		return this._batchCallbacks.map(async ({ callback, instruments }) => {
			const observableResult = new BatchObservableResultImpl();
			let callPromise = Promise.resolve(callback(observableResult));
			if (timeoutMillis != null) callPromise = callWithTimeout(callPromise, timeoutMillis);
			await callPromise;
			instruments.forEach((instrument) => {
				const buffer = observableResult._buffer.get(instrument);
				if (buffer == null) return;
				instrument._metricStorages.forEach((metricStorage) => {
					metricStorage.record(buffer, observationTime);
				});
			});
		});
	}
	_findCallback(callback, instrument) {
		return this._callbacks.findIndex((record) => {
			return record.callback === callback && record.instrument === instrument;
		});
	}
	_findBatchCallback(callback, instruments) {
		return this._batchCallbacks.findIndex((record) => {
			return record.callback === callback && setEquals(record.instruments, instruments);
		});
	}
};
init_esm();
/**
* Internal interface.
*
* Stores and aggregates {@link MetricData} for synchronous instruments.
*/
var SyncMetricStorage = class extends MetricStorage {
	_aggregationCardinalityLimit;
	_deltaMetricStorage;
	_temporalMetricStorage;
	_attributesProcessor;
	constructor(instrumentDescriptor, aggregator, attributesProcessor, collectorHandles, aggregationCardinalityLimit) {
		super(instrumentDescriptor);
		this._aggregationCardinalityLimit = aggregationCardinalityLimit;
		this._deltaMetricStorage = new DeltaMetricProcessor(aggregator, this._aggregationCardinalityLimit);
		this._temporalMetricStorage = new TemporalMetricProcessor(aggregator, collectorHandles);
		this._attributesProcessor = attributesProcessor;
		this.hasAttributeProcessor = attributesProcessor !== void 0;
	}
	hasAttributeProcessor;
	record(value, attributes, context$2, recordTime) {
		if (this._attributesProcessor !== void 0) attributes = this._attributesProcessor.process(attributes, context$2 ?? context.active());
		this._deltaMetricStorage.record(value, attributes, recordTime);
	}
	/**
	* Collects the metrics from this storage.
	*
	* Note: This is a stateful operation and may reset any interval-related
	* state for the MetricCollector.
	*/
	collect(collector, collectionTime) {
		const accumulations = this._deltaMetricStorage.collect();
		return this._temporalMetricStorage.buildMetrics(collector, this._instrumentDescriptor, accumulations, collectionTime);
	}
};
/**
* An internal record for shared meter provider states.
*/
var MeterSharedState = class {
	metricStorageRegistry = new MetricStorageRegistry();
	observableRegistry = new ObservableRegistry();
	meter;
	_meterProviderSharedState;
	_instrumentationScope;
	constructor(meterProviderSharedState, instrumentationScope) {
		this.meter = new Meter(this);
		this._meterProviderSharedState = meterProviderSharedState;
		this._instrumentationScope = instrumentationScope;
	}
	registerMetricStorage(descriptor) {
		const storages = this._registerMetricStorage(descriptor, SyncMetricStorage);
		if (storages.length === 1) return storages[0];
		return new MultiMetricStorage(storages);
	}
	registerAsyncMetricStorage(descriptor) {
		return this._registerMetricStorage(descriptor, AsyncMetricStorage);
	}
	/**
	* @param collector opaque handle of {@link MetricCollector} which initiated the collection.
	* @param collectionTime the HrTime at which the collection was initiated.
	* @param options options for collection.
	* @returns the list of metric data collected.
	*/
	async collect(collector, collectionTime, options) {
		/**
		* 1. Call all observable callbacks first.
		* 2. Collect metric result for the collector.
		*/
		const errors = await this.observableRegistry.observe(collectionTime, options?.timeoutMillis);
		const storages = this.metricStorageRegistry.getStorages(collector);
		if (storages.length === 0) return null;
		const metricDataList = [];
		storages.forEach((metricStorage) => {
			const metricData = metricStorage.collect(collector, collectionTime);
			if (metricData != null) metricDataList.push(metricData);
		});
		if (metricDataList.length === 0) return { errors };
		return {
			scopeMetrics: {
				scope: this._instrumentationScope,
				metrics: metricDataList
			},
			errors
		};
	}
	_registerMetricStorage(descriptor, MetricStorageType) {
		let storages = this._meterProviderSharedState.viewRegistry.findViews(descriptor, this._instrumentationScope).map((view) => {
			const viewDescriptor = createInstrumentDescriptorWithView(view, descriptor);
			const compatibleStorage = this.metricStorageRegistry.findOrUpdateCompatibleStorage(viewDescriptor);
			if (compatibleStorage != null) return compatibleStorage;
			const viewStorage = new MetricStorageType(viewDescriptor, view.aggregation.createAggregator(viewDescriptor), view.attributesProcessor, this._meterProviderSharedState.metricCollectors, view.aggregationCardinalityLimit);
			this.metricStorageRegistry.register(viewStorage);
			return viewStorage;
		});
		if (storages.length === 0) {
			const collectorStorages = this._meterProviderSharedState.selectAggregations(descriptor.type).map(([collector, aggregation]) => {
				const compatibleStorage = this.metricStorageRegistry.findOrUpdateCompatibleCollectorStorage(collector, descriptor);
				if (compatibleStorage != null) return compatibleStorage;
				const aggregator = aggregation.createAggregator(descriptor);
				const cardinalityLimit = collector.selectCardinalityLimit(descriptor.type);
				const storage = new MetricStorageType(descriptor, aggregator, void 0, [collector], cardinalityLimit);
				this.metricStorageRegistry.registerForCollector(collector, storage);
				return storage;
			});
			storages = storages.concat(collectorStorages);
		}
		return storages;
	}
};
/**
* An internal record for shared meter provider states.
*/
var MeterProviderSharedState = class {
	viewRegistry = new ViewRegistry();
	metricCollectors = [];
	meterSharedStates = /* @__PURE__ */ new Map();
	resource;
	constructor(resource) {
		this.resource = resource;
	}
	getMeterSharedState(instrumentationScope) {
		const id = instrumentationScopeId(instrumentationScope);
		let meterSharedState = this.meterSharedStates.get(id);
		if (meterSharedState == null) {
			meterSharedState = new MeterSharedState(this, instrumentationScope);
			this.meterSharedStates.set(id, meterSharedState);
		}
		return meterSharedState;
	}
	selectAggregations(instrumentType) {
		const result = [];
		for (const collector of this.metricCollectors) result.push([collector, toAggregation(collector.selectAggregation(instrumentType))]);
		return result;
	}
};
/**
* An internal opaque interface that the MetricReader receives as
* MetricProducer. It acts as the storage key to the internal metric stream
* state for each MetricReader.
*/
var MetricCollector = class {
	_sharedState;
	_metricReader;
	constructor(sharedState, metricReader) {
		this._sharedState = sharedState;
		this._metricReader = metricReader;
	}
	async collect(options) {
		const collectionTime = millisToHrTime(Date.now());
		const scopeMetrics = [];
		const errors = [];
		const meterCollectionPromises = Array.from(this._sharedState.meterSharedStates.values()).map(async (meterSharedState) => {
			const current = await meterSharedState.collect(this, collectionTime, options);
			if (current?.scopeMetrics != null) scopeMetrics.push(current.scopeMetrics);
			if (current?.errors != null) errors.push(...current.errors);
		});
		await Promise.all(meterCollectionPromises);
		return {
			resourceMetrics: {
				resource: this._sharedState.resource,
				scopeMetrics
			},
			errors
		};
	}
	/**
	* Delegates for MetricReader.forceFlush.
	*/
	async forceFlush(options) {
		await this._metricReader.forceFlush(options);
	}
	/**
	* Delegates for MetricReader.shutdown.
	*/
	async shutdown(options) {
		await this._metricReader.shutdown(options);
	}
	selectAggregationTemporality(instrumentType) {
		return this._metricReader.selectAggregationTemporality(instrumentType);
	}
	selectAggregation(instrumentType) {
		return this._metricReader.selectAggregation(instrumentType);
	}
	/**
	* Select the cardinality limit for the given {@link InstrumentType} for this
	* collector.
	*/
	selectCardinalityLimit(instrumentType) {
		return this._metricReader.selectCardinalityLimit?.(instrumentType) ?? 2e3;
	}
};
var ESCAPE = /[\^$\\.+?()[\]{}|]/g;
/**
* Wildcard pattern predicate, supports patterns like `*`, `foo*`, `*bar`.
*/
var PatternPredicate = class PatternPredicate {
	_matchAll;
	_regexp;
	constructor(pattern) {
		if (pattern === "*") {
			this._matchAll = true;
			this._regexp = /.*/;
		} else {
			this._matchAll = false;
			this._regexp = new RegExp(PatternPredicate.escapePattern(pattern));
		}
	}
	match(str) {
		if (this._matchAll) return true;
		return this._regexp.test(str);
	}
	static escapePattern(pattern) {
		return `^${pattern.replace(ESCAPE, "\\$&").replace("*", ".*")}$`;
	}
	static hasWildcard(pattern) {
		return pattern.includes("*");
	}
};
var ExactPredicate = class {
	_matchAll;
	_pattern;
	constructor(pattern) {
		this._matchAll = pattern === void 0;
		this._pattern = pattern;
	}
	match(str) {
		if (this._matchAll) return true;
		if (str === this._pattern) return true;
		return false;
	}
};
var NoopAttributesProcessor = class {
	process(incoming, _context) {
		return incoming;
	}
};
var MultiAttributesProcessor = class {
	_processors;
	constructor(processors) {
		this._processors = processors;
	}
	process(incoming, context) {
		let filteredAttributes = incoming;
		for (const processor of this._processors) filteredAttributes = processor.process(filteredAttributes, context);
		return filteredAttributes;
	}
};
/**
* @internal
*
* Create an {@link IAttributesProcessor} that acts as a simple pass-through for attributes.
*/
function createNoopAttributesProcessor() {
	return NOOP;
}
/**
* @internal
*
* Create an {@link IAttributesProcessor} that applies all processors from the provided list in order.
*
* @param processors Processors to apply in order.
*/
function createMultiAttributesProcessor(processors) {
	return new MultiAttributesProcessor(processors);
}
var NOOP = new NoopAttributesProcessor();
var InstrumentSelector = class {
	_nameFilter;
	_type;
	_unitFilter;
	constructor(criteria) {
		this._nameFilter = new PatternPredicate(criteria?.name ?? "*");
		this._type = criteria?.type;
		this._unitFilter = new ExactPredicate(criteria?.unit);
	}
	getType() {
		return this._type;
	}
	getNameFilter() {
		return this._nameFilter;
	}
	getUnitFilter() {
		return this._unitFilter;
	}
};
var MeterSelector = class {
	_nameFilter;
	_versionFilter;
	_schemaUrlFilter;
	constructor(criteria) {
		this._nameFilter = new ExactPredicate(criteria?.name);
		this._versionFilter = new ExactPredicate(criteria?.version);
		this._schemaUrlFilter = new ExactPredicate(criteria?.schemaUrl);
	}
	getNameFilter() {
		return this._nameFilter;
	}
	/**
	* TODO: semver filter? no spec yet.
	*/
	getVersionFilter() {
		return this._versionFilter;
	}
	getSchemaUrlFilter() {
		return this._schemaUrlFilter;
	}
};
function isSelectorNotProvided(options) {
	return options.instrumentName == null && options.instrumentType == null && options.instrumentUnit == null && options.meterName == null && options.meterVersion == null && options.meterSchemaUrl == null;
}
function validateViewOptions(viewOptions) {
	if (isSelectorNotProvided(viewOptions)) throw new Error("Cannot create view with no selector arguments supplied");
	if (viewOptions.name != null && (viewOptions?.instrumentName == null || PatternPredicate.hasWildcard(viewOptions.instrumentName))) throw new Error("Views with a specified name must be declared with an instrument selector that selects at most one instrument per meter.");
}
/**
* Can be passed to a {@link MeterProvider} to select instruments and alter their metric stream.
*/
var View = class {
	name;
	description;
	aggregation;
	attributesProcessor;
	instrumentSelector;
	meterSelector;
	aggregationCardinalityLimit;
	/**
	* Create a new {@link View} instance.
	*
	* Parameters can be categorized as two types:
	*  Instrument selection criteria: Used to describe the instrument(s) this view will be applied to.
	*  Will be treated as additive (the Instrument has to meet all the provided criteria to be selected).
	*
	*  Metric stream altering: Alter the metric stream of instruments selected by instrument selection criteria.
	*
	* @param viewOptions {@link ViewOptions} for altering the metric stream and instrument selection.
	* @param viewOptions.name
	* Alters the metric stream:
	*  This will be used as the name of the metrics stream.
	*  If not provided, the original Instrument name will be used.
	* @param viewOptions.description
	* Alters the metric stream:
	*  This will be used as the description of the metrics stream.
	*  If not provided, the original Instrument description will be used by default.
	* @param viewOptions.attributesProcessors
	* Alters the metric stream:
	*  If provided, the attributes will be modified as defined by the added processors.
	*  If not provided, all attribute keys will be used by default.
	* @param viewOptions.aggregationCardinalityLimit
	* Alters the metric stream:
	*  Sets a limit on the number of unique attribute combinations (cardinality) that can be aggregated.
	*  If not provided, the default limit of 2000 will be used.
	* @param viewOptions.aggregation
	* Alters the metric stream:
	*  Alters the {@link Aggregation} of the metric stream.
	* @param viewOptions.instrumentName
	* Instrument selection criteria:
	*  Original name of the Instrument(s) with wildcard support.
	* @param viewOptions.instrumentType
	* Instrument selection criteria:
	*  The original type of the Instrument(s).
	* @param viewOptions.instrumentUnit
	* Instrument selection criteria:
	*  The unit of the Instrument(s).
	* @param viewOptions.meterName
	* Instrument selection criteria:
	*  The name of the Meter. No wildcard support, name must match the meter exactly.
	* @param viewOptions.meterVersion
	* Instrument selection criteria:
	*  The version of the Meter. No wildcard support, version must match exactly.
	* @param viewOptions.meterSchemaUrl
	* Instrument selection criteria:
	*  The schema URL of the Meter. No wildcard support, schema URL must match exactly.
	*
	* @example
	* // Create a view that changes the Instrument 'my.instrument' to use to an
	* // ExplicitBucketHistogramAggregation with the boundaries [20, 30, 40]
	* new View({
	*   aggregation: new ExplicitBucketHistogramAggregation([20, 30, 40]),
	*   instrumentName: 'my.instrument'
	* })
	*/
	constructor(viewOptions) {
		validateViewOptions(viewOptions);
		if (viewOptions.attributesProcessors != null) this.attributesProcessor = createMultiAttributesProcessor(viewOptions.attributesProcessors);
		else this.attributesProcessor = createNoopAttributesProcessor();
		this.name = viewOptions.name;
		this.description = viewOptions.description;
		this.aggregation = toAggregation(viewOptions.aggregation ?? { type: AggregationType.DEFAULT });
		this.instrumentSelector = new InstrumentSelector({
			name: viewOptions.instrumentName,
			type: viewOptions.instrumentType,
			unit: viewOptions.instrumentUnit
		});
		this.meterSelector = new MeterSelector({
			name: viewOptions.meterName,
			version: viewOptions.meterVersion,
			schemaUrl: viewOptions.meterSchemaUrl
		});
		this.aggregationCardinalityLimit = viewOptions.aggregationCardinalityLimit;
	}
};
init_esm();
/**
* This class implements the {@link MeterProvider} interface.
*/
var MeterProvider = class {
	_sharedState;
	_shutdown = false;
	constructor(options) {
		this._sharedState = new MeterProviderSharedState(options?.resource ?? defaultResource());
		if (options?.views != null && options.views.length > 0) for (const viewOption of options.views) this._sharedState.viewRegistry.addView(new View(viewOption));
		if (options?.readers != null && options.readers.length > 0) for (const metricReader of options.readers) {
			const collector = new MetricCollector(this._sharedState, metricReader);
			metricReader.setMetricProducer(collector);
			this._sharedState.metricCollectors.push(collector);
			if (options.sdkMetricsEnabled && metricReader instanceof MetricReader) metricReader._setSelfObsMeterProvider(this);
		}
	}
	/**
	* Get a meter with the configuration of the MeterProvider.
	*/
	getMeter(name, version = "", options = {}) {
		if (this._shutdown) {
			diag.warn("A shutdown MeterProvider cannot provide a Meter");
			return createNoopMeter();
		}
		return this._sharedState.getMeterSharedState({
			name,
			version,
			schemaUrl: options.schemaUrl
		}).meter;
	}
	/**
	* Shut down the MeterProvider and all registered
	* MetricReaders.
	*
	* Returns a promise which is resolved when all flushes are complete.
	*/
	async shutdown(options) {
		if (this._shutdown) {
			diag.warn("shutdown may only be called once per MeterProvider");
			return;
		}
		this._shutdown = true;
		await Promise.all(this._sharedState.metricCollectors.map((collector) => {
			return collector.shutdown(options);
		}));
	}
	/**
	* Notifies all registered MetricReaders to flush any buffered data.
	*
	* Returns a promise which is resolved when all flushes are complete.
	*/
	async forceFlush(options) {
		if (this._shutdown) {
			diag.warn("invalid attempt to force flush after MeterProvider shutdown");
			return;
		}
		await Promise.all(this._sharedState.metricCollectors.map((collector) => {
			return collector.forceFlush(options);
		}));
	}
};
function createResource(resource, encoder) {
	const result = {
		attributes: toAttributes(resource.attributes, encoder),
		droppedAttributesCount: 0
	};
	const schemaUrl = resource.schemaUrl;
	if (schemaUrl && schemaUrl !== "") result.schemaUrl = schemaUrl;
	return result;
}
function createInstrumentationScope(scope, encoder) {
	const result = {
		name: scope.name,
		version: scope.version
	};
	if (scope.attributes && Object.keys(scope.attributes).length > 0) {
		result.attributes = toAttributes(scope.attributes, encoder);
		result.droppedAttributesCount = scope.droppedAttributesCount ?? 0;
	}
	return result;
}
function toAttributes(attributes, encoder) {
	return Object.keys(attributes).map((key) => toKeyValue(key, attributes[key], encoder));
}
function toKeyValue(key, value, encoder) {
	return {
		key,
		value: toAnyValue(value, encoder)
	};
}
function toAnyValue(value, encoder) {
	const t = typeof value;
	if (t === "string") return { stringValue: value };
	if (t === "number") {
		if (!Number.isInteger(value)) return { doubleValue: value };
		return { intValue: value };
	}
	if (t === "boolean") return { boolValue: value };
	if (value instanceof Uint8Array) return { bytesValue: encoder.encodeUint8Array(value) };
	if (Array.isArray(value)) {
		const values = new Array(value.length);
		for (let i = 0; i < value.length; i++) values[i] = toAnyValue(value[i], encoder);
		return { arrayValue: { values } };
	}
	if (t === "object" && value != null) {
		const keys = Object.keys(value);
		const values = new Array(keys.length);
		for (let i = 0; i < keys.length; i++) values[i] = {
			key: keys[i],
			value: toAnyValue(value[keys[i]], encoder)
		};
		return { kvlistValue: { values } };
	}
	return {};
}
function createExportLogsServiceRequest(logRecords, encoder) {
	return { resourceLogs: logRecordsToResourceLogs(logRecords, encoder) };
}
function createResourceMap$1(logRecords) {
	const resourceMap = /* @__PURE__ */ new Map();
	for (const record of logRecords) {
		const { resource, instrumentationScope } = record;
		let ismMap = resourceMap.get(resource);
		if (!ismMap) {
			ismMap = /* @__PURE__ */ new Map();
			resourceMap.set(resource, ismMap);
		}
		let records = ismMap.get(instrumentationScope);
		if (!records) {
			records = [];
			ismMap.set(instrumentationScope, records);
		}
		records.push(record);
	}
	return resourceMap;
}
function logRecordsToResourceLogs(logRecords, encoder) {
	const resourceMap = createResourceMap$1(logRecords);
	return Array.from(resourceMap, ([resource, ismMap]) => {
		const processedResource = createResource(resource, encoder);
		return {
			resource: processedResource,
			scopeLogs: Array.from(ismMap, ([, scopeLogs]) => {
				return {
					scope: createInstrumentationScope(scopeLogs[0].instrumentationScope, encoder),
					logRecords: scopeLogs.map((log) => toLogRecord(log, encoder)),
					schemaUrl: scopeLogs[0].instrumentationScope.schemaUrl
				};
			}),
			schemaUrl: processedResource.schemaUrl
		};
	});
}
function toLogRecord(log, encoder) {
	return {
		timeUnixNano: encoder.encodeHrTime(log.hrTime),
		observedTimeUnixNano: encoder.encodeHrTime(log.hrTimeObserved),
		severityNumber: toSeverityNumber(log.severityNumber),
		severityText: log.severityText,
		body: toAnyValue(log.body, encoder),
		eventName: log.eventName,
		attributes: toAttributes(log.attributes, encoder),
		droppedAttributesCount: log.droppedAttributesCount,
		flags: log.spanContext?.traceFlags,
		traceId: encoder.encodeOptionalSpanContext(log.spanContext?.traceId),
		spanId: encoder.encodeOptionalSpanContext(log.spanContext?.spanId)
	};
}
function toSeverityNumber(severityNumber) {
	return severityNumber;
}
function hrTimeToNanos(hrTime) {
	const NANOSECONDS = BigInt(1e9);
	return BigInt(Math.trunc(hrTime[0])) * NANOSECONDS + BigInt(Math.trunc(hrTime[1]));
}
function encodeAsString(hrTime) {
	return hrTimeToNanos(hrTime).toString();
}
var encodeTimestamp = typeof BigInt !== "undefined" ? encodeAsString : hrTimeToNanoseconds;
function identity(value) {
	return value;
}
/**
* Encoder for JSON format.
* Uses string timestamps, hex for span/trace IDs, and base64 for Uint8Array.
*/
var JSON_ENCODER = {
	encodeHrTime: encodeTimestamp,
	encodeSpanContext: identity,
	encodeOptionalSpanContext: identity,
	encodeUint8Array: (bytes) => {
		if (typeof Buffer !== "undefined") return Buffer.from(bytes).toString("base64");
		const chars = new Array(bytes.length);
		for (let i = 0; i < bytes.length; i++) chars[i] = String.fromCharCode(bytes[i]);
		return btoa(chars.join(""));
	}
};
init_esm();
/**
* @experimental this serializer may receive breaking changes in minor versions, pin this package's version when using this constant
*/
var JsonLogsSerializer = {
	serializeRequest: (arg) => {
		const request = createExportLogsServiceRequest(arg, JSON_ENCODER);
		return new TextEncoder().encode(JSON.stringify(request));
	},
	deserializeResponse: (arg) => {
		if (arg.length === 0) return {};
		const decoder = new TextDecoder();
		try {
			return JSON.parse(decoder.decode(arg));
		} catch (err) {
			diag.warn(`Failed to parse logs export response: ${err.message}. Returning empty response`);
			return {};
		}
	}
};
/**
* AggregationTemporality defines how a metric aggregator reports aggregated
* values. It describes how those values relate to the time interval over
* which they are aggregated.
*/
var EAggregationTemporality;
(function(EAggregationTemporality) {
	EAggregationTemporality[EAggregationTemporality["AGGREGATION_TEMPORALITY_UNSPECIFIED"] = 0] = "AGGREGATION_TEMPORALITY_UNSPECIFIED";
	/** DELTA is an AggregationTemporality for a metric aggregator which reports
	changes since last report time. Successive metrics contain aggregation of
	values from continuous and non-overlapping intervals.
	
	The values for a DELTA metric are based only on the time interval
	associated with one measurement cycle. There is no dependency on
	previous measurements like is the case for CUMULATIVE metrics.
	
	For example, consider a system measuring the number of requests that
	it receives and reports the sum of these requests every second as a
	DELTA metric:
	
	1. The system starts receiving at time=t_0.
	2. A request is received, the system measures 1 request.
	3. A request is received, the system measures 1 request.
	4. A request is received, the system measures 1 request.
	5. The 1 second collection cycle ends. A metric is exported for the
	number of requests received over the interval of time t_0 to
	t_0+1 with a value of 3.
	6. A request is received, the system measures 1 request.
	7. A request is received, the system measures 1 request.
	8. The 1 second collection cycle ends. A metric is exported for the
	number of requests received over the interval of time t_0+1 to
	t_0+2 with a value of 2. */
	EAggregationTemporality[EAggregationTemporality["AGGREGATION_TEMPORALITY_DELTA"] = 1] = "AGGREGATION_TEMPORALITY_DELTA";
	/** CUMULATIVE is an AggregationTemporality for a metric aggregator which
	reports changes since a fixed start time. This means that current values
	of a CUMULATIVE metric depend on all previous measurements since the
	start time. Because of this, the sender is required to retain this state
	in some form. If this state is lost or invalidated, the CUMULATIVE metric
	values MUST be reset and a new fixed start time following the last
	reported measurement time sent MUST be used.
	
	For example, consider a system measuring the number of requests that
	it receives and reports the sum of these requests every second as a
	CUMULATIVE metric:
	
	1. The system starts receiving at time=t_0.
	2. A request is received, the system measures 1 request.
	3. A request is received, the system measures 1 request.
	4. A request is received, the system measures 1 request.
	5. The 1 second collection cycle ends. A metric is exported for the
	number of requests received over the interval of time t_0 to
	t_0+1 with a value of 3.
	6. A request is received, the system measures 1 request.
	7. A request is received, the system measures 1 request.
	8. The 1 second collection cycle ends. A metric is exported for the
	number of requests received over the interval of time t_0 to
	t_0+2 with a value of 5.
	9. The system experiences a fault and loses state.
	10. The system recovers and resumes receiving at time=t_1.
	11. A request is received, the system measures 1 request.
	12. The 1 second collection cycle ends. A metric is exported for the
	number of requests received over the interval of time t_1 to
	t_0+1 with a value of 1.
	
	Note: Even though, when reporting changes since last report time, using
	CUMULATIVE is valid, it is not recommended. This may cause problems for
	systems that do not use start_time to determine when the aggregation
	value was reset (e.g. Prometheus). */
	EAggregationTemporality[EAggregationTemporality["AGGREGATION_TEMPORALITY_CUMULATIVE"] = 2] = "AGGREGATION_TEMPORALITY_CUMULATIVE";
})(EAggregationTemporality || (EAggregationTemporality = {}));
init_esm();
function toResourceMetrics(resourceMetrics, encoder) {
	const processedResource = createResource(resourceMetrics.resource, encoder);
	return {
		resource: processedResource,
		schemaUrl: processedResource.schemaUrl,
		scopeMetrics: toScopeMetrics(resourceMetrics.scopeMetrics, encoder)
	};
}
function toScopeMetrics(scopeMetrics, encoder) {
	return Array.from(scopeMetrics.map((metrics) => ({
		scope: createInstrumentationScope(metrics.scope, encoder),
		metrics: metrics.metrics.map((metricData) => toMetric(metricData, encoder)),
		schemaUrl: metrics.scope.schemaUrl
	})));
}
function toMetric(metricData, encoder) {
	const out = {
		name: metricData.descriptor.name,
		description: metricData.descriptor.description,
		unit: metricData.descriptor.unit
	};
	const aggregationTemporality = toAggregationTemporality(metricData.aggregationTemporality);
	switch (metricData.dataPointType) {
		case DataPointType.SUM:
			out.sum = {
				aggregationTemporality,
				isMonotonic: metricData.isMonotonic,
				dataPoints: toSingularDataPoints(metricData, encoder)
			};
			break;
		case DataPointType.GAUGE:
			out.gauge = { dataPoints: toSingularDataPoints(metricData, encoder) };
			break;
		case DataPointType.HISTOGRAM:
			out.histogram = {
				aggregationTemporality,
				dataPoints: toHistogramDataPoints(metricData, encoder)
			};
			break;
		case DataPointType.EXPONENTIAL_HISTOGRAM: out.exponentialHistogram = {
			aggregationTemporality,
			dataPoints: toExponentialHistogramDataPoints(metricData, encoder)
		};
	}
	return out;
}
function toSingularDataPoint(dataPoint, valueType, encoder) {
	const out = {
		attributes: toAttributes(dataPoint.attributes, encoder),
		startTimeUnixNano: encoder.encodeHrTime(dataPoint.startTime),
		timeUnixNano: encoder.encodeHrTime(dataPoint.endTime)
	};
	switch (valueType) {
		case ValueType.INT:
			out.asInt = dataPoint.value;
			break;
		case ValueType.DOUBLE: out.asDouble = dataPoint.value;
	}
	return out;
}
function toSingularDataPoints(metricData, encoder) {
	return metricData.dataPoints.map((dataPoint) => {
		return toSingularDataPoint(dataPoint, metricData.descriptor.valueType, encoder);
	});
}
function toHistogramDataPoints(metricData, encoder) {
	return metricData.dataPoints.map((dataPoint) => {
		const histogram = dataPoint.value;
		return {
			attributes: toAttributes(dataPoint.attributes, encoder),
			bucketCounts: histogram.buckets.counts,
			explicitBounds: histogram.buckets.boundaries,
			count: histogram.count,
			sum: histogram.sum,
			min: histogram.min,
			max: histogram.max,
			startTimeUnixNano: encoder.encodeHrTime(dataPoint.startTime),
			timeUnixNano: encoder.encodeHrTime(dataPoint.endTime)
		};
	});
}
function toExponentialHistogramDataPoints(metricData, encoder) {
	return metricData.dataPoints.map((dataPoint) => {
		const histogram = dataPoint.value;
		return {
			attributes: toAttributes(dataPoint.attributes, encoder),
			count: histogram.count,
			min: histogram.min,
			max: histogram.max,
			sum: histogram.sum,
			positive: {
				offset: histogram.positive.offset,
				bucketCounts: histogram.positive.bucketCounts
			},
			negative: {
				offset: histogram.negative.offset,
				bucketCounts: histogram.negative.bucketCounts
			},
			scale: histogram.scale,
			zeroCount: histogram.zeroCount,
			startTimeUnixNano: encoder.encodeHrTime(dataPoint.startTime),
			timeUnixNano: encoder.encodeHrTime(dataPoint.endTime)
		};
	});
}
function toAggregationTemporality(temporality) {
	switch (temporality) {
		case AggregationTemporality.DELTA: return EAggregationTemporality.AGGREGATION_TEMPORALITY_DELTA;
		case AggregationTemporality.CUMULATIVE: return EAggregationTemporality.AGGREGATION_TEMPORALITY_CUMULATIVE;
	}
}
function createExportMetricsServiceRequest(resourceMetrics, encoder) {
	return { resourceMetrics: resourceMetrics.map((metrics) => toResourceMetrics(metrics, encoder)) };
}
init_esm();
var JsonMetricsSerializer = {
	serializeRequest: (arg) => {
		const request = createExportMetricsServiceRequest([arg], JSON_ENCODER);
		return new TextEncoder().encode(JSON.stringify(request));
	},
	deserializeResponse: (arg) => {
		if (arg.length === 0) return {};
		const decoder = new TextDecoder();
		try {
			return JSON.parse(decoder.decode(arg));
		} catch (err) {
			diag.warn(`Failed to parse metrics export response: ${err.message}. Returning empty response`);
			return {};
		}
	}
};
var SPAN_FLAGS_CONTEXT_HAS_IS_REMOTE_MASK = 256;
var SPAN_FLAGS_CONTEXT_IS_REMOTE_MASK = 512;
/**
* Builds the 32-bit span flags value combining the low 8-bit W3C TraceFlags
* with the HAS_IS_REMOTE and IS_REMOTE bits according to the OTLP spec.
*/
function buildSpanFlagsFrom(traceFlags, isRemote) {
	let flags = traceFlags & 255 | SPAN_FLAGS_CONTEXT_HAS_IS_REMOTE_MASK;
	if (isRemote) flags |= SPAN_FLAGS_CONTEXT_IS_REMOTE_MASK;
	return flags;
}
function sdkSpanToOtlpSpan(span, encoder) {
	const ctx = span.spanContext();
	const status = span.status;
	const parentSpanId = span.parentSpanContext?.spanId ? encoder.encodeSpanContext(span.parentSpanContext?.spanId) : void 0;
	return {
		traceId: encoder.encodeSpanContext(ctx.traceId),
		spanId: encoder.encodeSpanContext(ctx.spanId),
		parentSpanId,
		traceState: ctx.traceState?.serialize(),
		name: span.name,
		kind: span.kind == null ? 0 : span.kind + 1,
		startTimeUnixNano: encoder.encodeHrTime(span.startTime),
		endTimeUnixNano: encoder.encodeHrTime(span.endTime),
		attributes: toAttributes(span.attributes, encoder),
		droppedAttributesCount: span.droppedAttributesCount,
		events: span.events.map((event) => toOtlpSpanEvent(event, encoder)),
		droppedEventsCount: span.droppedEventsCount,
		status: {
			code: status.code,
			message: status.message
		},
		links: span.links.map((link) => toOtlpLink(link, encoder)),
		droppedLinksCount: span.droppedLinksCount,
		flags: buildSpanFlagsFrom(ctx.traceFlags, span.parentSpanContext?.isRemote)
	};
}
function toOtlpLink(link, encoder) {
	return {
		attributes: link.attributes ? toAttributes(link.attributes, encoder) : [],
		spanId: encoder.encodeSpanContext(link.context.spanId),
		traceId: encoder.encodeSpanContext(link.context.traceId),
		traceState: link.context.traceState?.serialize(),
		droppedAttributesCount: link.droppedAttributesCount || 0,
		flags: buildSpanFlagsFrom(link.context.traceFlags, link.context.isRemote)
	};
}
function toOtlpSpanEvent(timedEvent, encoder) {
	return {
		attributes: timedEvent.attributes ? toAttributes(timedEvent.attributes, encoder) : [],
		name: timedEvent.name,
		timeUnixNano: encoder.encodeHrTime(timedEvent.time),
		droppedAttributesCount: timedEvent.droppedAttributesCount || 0
	};
}
function createExportTraceServiceRequest(spans, encoder) {
	return { resourceSpans: spanRecordsToResourceSpans(spans, encoder) };
}
function createResourceMap(readableSpans) {
	const resourceMap = /* @__PURE__ */ new Map();
	for (const record of readableSpans) {
		let ilsMap = resourceMap.get(record.resource);
		if (!ilsMap) {
			ilsMap = /* @__PURE__ */ new Map();
			resourceMap.set(record.resource, ilsMap);
		}
		const instrumentationScopeKey = `${record.instrumentationScope.name}@${record.instrumentationScope.version || ""}:${record.instrumentationScope.schemaUrl || ""}`;
		let records = ilsMap.get(instrumentationScopeKey);
		if (!records) {
			records = [];
			ilsMap.set(instrumentationScopeKey, records);
		}
		records.push(record);
	}
	return resourceMap;
}
function spanRecordsToResourceSpans(readableSpans, encoder) {
	const resourceMap = createResourceMap(readableSpans);
	const out = [];
	const entryIterator = resourceMap.entries();
	let entry = entryIterator.next();
	while (!entry.done) {
		const [resource, ilmMap] = entry.value;
		const scopeResourceSpans = [];
		const ilmIterator = ilmMap.values();
		let ilmEntry = ilmIterator.next();
		while (!ilmEntry.done) {
			const scopeSpans = ilmEntry.value;
			if (scopeSpans.length > 0) {
				const spans = scopeSpans.map((readableSpan) => sdkSpanToOtlpSpan(readableSpan, encoder));
				scopeResourceSpans.push({
					scope: createInstrumentationScope(scopeSpans[0].instrumentationScope, encoder),
					spans,
					schemaUrl: scopeSpans[0].instrumentationScope.schemaUrl
				});
			}
			ilmEntry = ilmIterator.next();
		}
		const processedResource = createResource(resource, encoder);
		const transformedSpans = {
			resource: processedResource,
			scopeSpans: scopeResourceSpans,
			schemaUrl: processedResource.schemaUrl
		};
		out.push(transformedSpans);
		entry = entryIterator.next();
	}
	return out;
}
init_esm();
var JsonTraceSerializer = {
	serializeRequest: (arg) => {
		const request = createExportTraceServiceRequest(arg, JSON_ENCODER);
		return new TextEncoder().encode(JSON.stringify(request));
	},
	deserializeResponse: (arg) => {
		if (arg.length === 0) return {};
		const decoder = new TextDecoder();
		try {
			return JSON.parse(decoder.decode(arg));
		} catch (err) {
			diag.warn(`Failed to parse trace export response: ${err.message}. Returning empty response`);
			return {};
		}
	}
};
init_esm();
/**
* Validates if a value is a valid AnyValue for Log Attributes according to OpenTelemetry spec.
* Log Attributes support a superset of standard Attributes and must support:
* - Scalar values: string, boolean, signed 64-bit integer, or double precision floating point
* - Byte arrays (Uint8Array)
* - Arrays of any values (heterogeneous arrays allowed)
* - Maps from string to any value (nested objects)
* - Empty values (null/undefined)
*
* @param val - The value to validate
* @returns true if the value is a valid AnyValue, false otherwise
*/
function isLogAttributeValue(val) {
	return isLogAttributeValueInternal(val, /* @__PURE__ */ new WeakSet());
}
function isLogAttributeValueInternal(val, visited) {
	if (val == null) return true;
	if (typeof val === "string" || typeof val === "number" || typeof val === "boolean") return true;
	if (val instanceof Uint8Array) return true;
	if (typeof val === "object") {
		if (visited.has(val)) return false;
		visited.add(val);
		if (Array.isArray(val)) {
			for (const item of val) if (!isLogAttributeValueInternal(item, visited)) return false;
			return true;
		}
		const obj = val;
		if (obj.constructor !== Object && obj.constructor !== void 0) return false;
		for (const key in obj) if (Object.prototype.hasOwnProperty.call(obj, key) && !isLogAttributeValueInternal(obj[key], visited)) return false;
		return true;
	}
	return false;
}
var AddAttributeDecision;
(function(AddAttributeDecision) {
	AddAttributeDecision[AddAttributeDecision["DROP_INVALID"] = 0] = "DROP_INVALID";
	AddAttributeDecision[AddAttributeDecision["DROP_LIMIT_REACHED"] = 1] = "DROP_LIMIT_REACHED";
	AddAttributeDecision[AddAttributeDecision["ADD_NEW"] = 2] = "ADD_NEW";
	AddAttributeDecision[AddAttributeDecision["ADD_OVERWRITE_EXISTING"] = 3] = "ADD_OVERWRITE_EXISTING";
})(AddAttributeDecision || (AddAttributeDecision = {}));
function addAttribute(attributes, limits, currentAttributesCount, key, value) {
	if (key.length === 0) {
		diag.warn(`Invalid attribute key: ${key}`);
		return AddAttributeDecision.DROP_INVALID;
	}
	if (!isLogAttributeValue(value)) {
		diag.warn(`Invalid attribute value set for key: ${key}`);
		return AddAttributeDecision.DROP_INVALID;
	}
	const isNewKey = !Object.prototype.hasOwnProperty.call(attributes, key);
	if (isNewKey && currentAttributesCount >= limits.attributeCountLimit) return AddAttributeDecision.DROP_LIMIT_REACHED;
	attributes[key] = truncateToSize(value, limits.attributeValueLengthLimit);
	if (isNewKey) return AddAttributeDecision.ADD_NEW;
	return AddAttributeDecision.ADD_OVERWRITE_EXISTING;
}
function truncateToSize(value, limit) {
	if (limit <= 0) {
		diag.warn(`Attribute value limit must be positive, got ${limit}`);
		return value;
	}
	if (value == null) return value;
	if (typeof value === "string") {
		if (value.length <= limit) return value;
		return value.substring(0, limit);
	}
	if (value instanceof Uint8Array) return value;
	if (Array.isArray(value)) return value.map((val) => truncateToSize(val, limit));
	if (typeof value === "object") {
		const truncatedObj = {};
		for (const [k, v] of Object.entries(value)) truncatedObj[k] = truncateToSize(v, limit);
		return truncatedObj;
	}
	return value;
}
/**
* Normalize attributes for use on the instrumentation scope. Drops invalid attributes and keeps track of
* how many were dropped.
*
* @param limits
* @param attributes
*/
function normalizeScopeAttributes(limits, attributes) {
	if (attributes == null) return {};
	const normalizedAttributes = {};
	let currentAttributesCount = 0;
	let droppedAttributesCount = 0;
	for (const [key, value] of Object.entries(attributes)) {
		const decision = addAttribute(normalizedAttributes, limits, currentAttributesCount, key, value);
		if (decision === AddAttributeDecision.ADD_NEW) currentAttributesCount += 1;
		else if (decision === AddAttributeDecision.DROP_INVALID) droppedAttributesCount += 1;
		else if (decision === AddAttributeDecision.DROP_LIMIT_REACHED) droppedAttributesCount += 1;
	}
	return {
		attributes: currentAttributesCount > 0 ? normalizedAttributes : void 0,
		droppedAttributesCount
	};
}
init_esm();
var LogRecordImpl = class {
	resource;
	instrumentationScope;
	attributes = {};
	_hrTime;
	_hrTimeObserved;
	_spanContext;
	_severityText;
	_severityNumber;
	_body;
	_eventName;
	_attributesCount = 0;
	_droppedAttributesCount = 0;
	_isReadonly = false;
	_logRecordLimits;
	get hrTime() {
		return this._hrTime;
	}
	set hrTime(hrTime) {
		if (this._isLogRecordReadonly()) return;
		this._hrTime = hrTime;
	}
	get hrTimeObserved() {
		return this._hrTimeObserved;
	}
	set hrTimeObserved(hrTimeObserved) {
		if (this._isLogRecordReadonly()) return;
		this._hrTimeObserved = hrTimeObserved;
	}
	get spanContext() {
		return this._spanContext;
	}
	set spanContext(spanContext) {
		if (this._isLogRecordReadonly()) return;
		this._spanContext = spanContext;
	}
	set severityText(severityText) {
		if (this._isLogRecordReadonly()) return;
		this._severityText = severityText;
	}
	get severityText() {
		return this._severityText;
	}
	set severityNumber(severityNumber) {
		if (this._isLogRecordReadonly()) return;
		this._severityNumber = severityNumber;
	}
	get severityNumber() {
		return this._severityNumber;
	}
	set body(body) {
		if (this._isLogRecordReadonly()) return;
		this._body = body;
	}
	get body() {
		return this._body;
	}
	get eventName() {
		return this._eventName;
	}
	set eventName(eventName) {
		if (this._isLogRecordReadonly()) return;
		this._eventName = eventName;
	}
	get droppedAttributesCount() {
		return this._droppedAttributesCount;
	}
	constructor(_sharedState, instrumentationScope, logRecord) {
		const { timestamp, observedTimestamp, eventName, severityNumber, severityText, body, attributes = {}, exception, context } = logRecord;
		const now = Date.now();
		this._hrTime = timeInputToHrTime(timestamp ?? now);
		this._hrTimeObserved = timeInputToHrTime(observedTimestamp ?? now);
		if (context) {
			const spanContext = trace.getSpanContext(context);
			if (spanContext && isSpanContextValid(spanContext)) this._spanContext = spanContext;
		}
		this.severityNumber = severityNumber;
		this.severityText = severityText;
		this.body = body;
		this.resource = _sharedState.resource;
		this.instrumentationScope = instrumentationScope;
		this._logRecordLimits = _sharedState.logRecordLimits;
		this._eventName = eventName;
		this.setAttributes(attributes);
		if (exception != null) this._setException(exception);
	}
	setAttribute(key, value) {
		if (this._isLogRecordReadonly()) return this;
		const decision = addAttribute(this.attributes, this._logRecordLimits, this._attributesCount, key, value);
		if (decision === AddAttributeDecision.DROP_LIMIT_REACHED) {
			this._droppedAttributesCount++;
			if (this._droppedAttributesCount === 1) diag.warn("Dropping extra attributes.");
		} else if (decision === AddAttributeDecision.ADD_NEW) this._attributesCount++;
		return this;
	}
	setAttributes(attributes) {
		for (const [k, v] of Object.entries(attributes)) this.setAttribute(k, v);
		return this;
	}
	setBody(body) {
		this.body = body;
		return this;
	}
	setEventName(eventName) {
		this.eventName = eventName;
		return this;
	}
	setSeverityNumber(severityNumber) {
		this.severityNumber = severityNumber;
		return this;
	}
	setSeverityText(severityText) {
		this.severityText = severityText;
		return this;
	}
	/**
	* @internal
	* A LogRecordProcessor may freely modify logRecord for the duration of the OnEmit call.
	* If logRecord is needed after OnEmit returns (i.e. for asynchronous processing) only reads are permitted.
	*/
	_makeReadonly() {
		this._isReadonly = true;
	}
	_setException(exception) {
		let hasMinimumAttributes = false;
		if (typeof exception === "string" || typeof exception === "number") {
			if (!Object.hasOwn(this.attributes, "exception.message")) this.setAttribute(ATTR_EXCEPTION_MESSAGE, String(exception));
			hasMinimumAttributes = true;
		} else if (exception && typeof exception === "object") {
			const exceptionObj = exception;
			if (exceptionObj.code) {
				if (!Object.hasOwn(this.attributes, "exception.type")) this.setAttribute(ATTR_EXCEPTION_TYPE, exceptionObj.code.toString());
				hasMinimumAttributes = true;
			} else if (exceptionObj.name) {
				if (!Object.hasOwn(this.attributes, "exception.type")) this.setAttribute(ATTR_EXCEPTION_TYPE, exceptionObj.name);
				hasMinimumAttributes = true;
			}
			if (exceptionObj.message) {
				if (!Object.hasOwn(this.attributes, "exception.message")) this.setAttribute(ATTR_EXCEPTION_MESSAGE, exceptionObj.message);
				hasMinimumAttributes = true;
			}
			if (exceptionObj.stack) {
				if (!Object.hasOwn(this.attributes, "exception.stacktrace")) this.setAttribute(ATTR_EXCEPTION_STACKTRACE, exceptionObj.stack);
				hasMinimumAttributes = true;
			}
		}
		if (!hasMinimumAttributes) diag.warn(`Failed to record an exception ${exception}`);
	}
	_isLogRecordReadonly() {
		if (this._isReadonly) diag.warn("Can not execute the operation on emitted log record");
		return this._isReadonly;
	}
};
init_esm();
var Logger = class {
	_instrumentationScope;
	_sharedState;
	_loggerConfig;
	constructor(instrumentationScope, sharedState) {
		this._instrumentationScope = instrumentationScope;
		this._sharedState = sharedState;
		this._loggerConfig = this._sharedState.getLoggerConfig(this._instrumentationScope);
	}
	emit(logRecord) {
		const currentContext = logRecord.context || context.active();
		if (!this.enabled(logRecord)) return;
		/**
		* If a Logger was obtained with include_trace_context=true,
		* the LogRecords it emits MUST automatically include the Trace Context from the active Context,
		* if Context has not been explicitly set.
		*/
		const logRecordInstance = new LogRecordImpl(this._sharedState, this._instrumentationScope, {
			context: currentContext,
			...logRecord
		});
		this._sharedState.loggerMetrics.emitLog();
		/**
		* the explicitly passed Context,
		* the current Context, or an empty Context if the Logger was obtained with include_trace_context=false
		*/
		this._sharedState.activeProcessor.onEmit(logRecordInstance, currentContext);
		/**
		* A LogRecordProcessor may freely modify logRecord for the duration of the OnEmit call.
		* If logRecord is needed after OnEmit returns (i.e. for asynchronous processing) only reads are permitted.
		*/
		logRecordInstance._makeReadonly();
	}
	enabled(options) {
		if (this._sharedState.hasShutdown) return false;
		const loggerConfig = this._loggerConfig;
		if (loggerConfig.disabled) return false;
		const severityNumber = options?.severityNumber;
		if (typeof severityNumber === "number" && severityNumber !== SeverityNumber.UNSPECIFIED && severityNumber < loggerConfig.minimumSeverity) return false;
		const currentContext = options?.context || context.active();
		if (loggerConfig.traceBased) {
			const spanContext = trace.getSpanContext(currentContext);
			if (spanContext && isSpanContextValid(spanContext)) {
				if (!((spanContext.traceFlags & TraceFlags.SAMPLED) === TraceFlags.SAMPLED)) return false;
			}
		}
		const enabledOpts = {
			context: currentContext,
			instrumentationScope: this._instrumentationScope,
			severityNumber: options?.severityNumber,
			eventName: options?.eventName
		};
		for (const processor of this._sharedState.processors) if (!processor.enabled || processor.enabled(enabledOpts)) return true;
		return false;
	}
};
var NoopLogRecordProcessor = class {
	forceFlush() {
		return Promise.resolve();
	}
	onEmit(_logRecord, _context) {}
	shutdown() {
		return Promise.resolve();
	}
	enabled(_options) {
		return false;
	}
};
/**
* Implementation of the {@link LogRecordProcessor} that simply forwards all
* received events to a list of {@link LogRecordProcessor}s.
*/
var MultiLogRecordProcessor = class {
	processors;
	constructor(processors) {
		this.processors = processors;
	}
	async forceFlush(options) {
		const timeout = options?.timeoutMillis ?? 3e4;
		await Promise.all(this.processors.map((processor) => callWithTimeout$1(processor.forceFlush(), timeout)));
	}
	onEmit(logRecord, context) {
		this.processors.forEach((processors) => processors.onEmit(logRecord, context));
	}
	async shutdown() {
		await Promise.all(this.processors.map((processor) => processor.shutdown()));
	}
	enabled(options) {
		for (const processor of this.processors) if (!processor.enabled || processor.enabled(options)) return true;
		return false;
	}
};
/**
* Normalizes an AnyValue to a JSON-serializable [typeTag, payload] tuple.
*
* Using a type tag as the first element guarantees that two values can only
* produce the same tuple when they have the same type AND the same data,
* avoiding cross-type collisions such as:
*   - null vs NaN vs Infinity (all become JSON `null` via JSON.stringify)
*   - -0 vs 0 (both become JSON `0` via JSON.stringify)
*   - string "null" vs the value null
*
* Object keys are sorted so that attribute maps with the same entries but
* different insertion orders produce the same key.
*/
function normalizeAnyValue(value) {
	if (value === void 0) return ["u", null];
	if (value === null) return ["n", null];
	const valueType = typeof value;
	if (valueType === "string") return ["s", value];
	if (valueType === "boolean") return ["b", value];
	if (valueType === "number") {
		if (Number.isNaN(value)) return ["nan", null];
		if (value === Infinity) return ["inf", null];
		if (value === -Infinity) return ["-inf", null];
		if (Object.is(value, -0)) return ["n0", null];
		return ["d", value];
	}
	if (value instanceof Uint8Array) return ["bytes", Array.from(value)];
	if (Array.isArray(value)) return ["arr", value.map(normalizeAnyValue)];
	return ["map", Object.entries(value).sort(([a], [b]) => a.localeCompare(b)).map(([k, v]) => [k, normalizeAnyValue(v)])];
}
/**
* Converting the instrumentation scope object to a unique identifier string.
* @param scope - The instrumentation scope to convert
* @returns A unique string identifier for the scope
*/
function getInstrumentationScopeKey(scope) {
	return JSON.stringify([
		scope.name,
		scope.version || "",
		scope.schemaUrl || "",
		normalizeAnyValue(scope.attributes),
		scope.droppedAttributesCount ?? 0
	]);
}
/**
* The number of logs submitted to enabled SDK Loggers.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_LOG_CREATED = "otel.sdk.log.created";
/**
* The number of log records for which the processing has finished, either successful or failed.
*
* @note For successful processing, `error.type` **MUST NOT** be set. For failed processing, `error.type` **MUST** contain the failure cause.
* For the SDK Simple and Batching Log Record Processor a log record is considered to be processed already when it has been submitted to the exporter,
* not when the corresponding export call has finished.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_LOG_PROCESSED = "otel.sdk.processor.log.processed";
/**
* The maximum number of log records the queue of a given instance of an SDK Log Record processor can hold.
*
* @note Only applies to Log Record processors which use a queue, e.g. the SDK Batching Log Record Processor.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_LOG_QUEUE_CAPACITY = "otel.sdk.processor.log.queue.capacity";
/**
* The number of log records in the queue of a given instance of an SDK log processor.
*
* @note Only applies to log record processors which use a queue, e.g. the SDK Batching Log Record Processor.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_LOG_QUEUE_SIZE = "otel.sdk.processor.log.queue.size";
/**
* A name uniquely identifying the instance of the OpenTelemetry component within its containing SDK instance.
*
* @example otlp_grpc_span_exporter/0
* @example custom-name
*
* @note Implementations **SHOULD** ensure a low cardinality for this attribute, even across application or SDK restarts.
* E.g. implementations **MUST NOT** use UUIDs as values for this attribute.
*
* Implementations **MAY** achieve these goals by following a `<otel.component.type>/<instance-counter>` pattern, e.g. `batching_span_processor/0`.
* Hereby `otel.component.type` refers to the corresponding attribute value of the component.
*
* The value of `instance-counter` **MAY** be automatically assigned by the component and uniqueness within the enclosing SDK instance **MUST** be guaranteed.
* For example, `<instance-counter>` **MAY** be implemented by using a monotonically increasing counter (starting with `0`), which is incremented every time an
* instance of the given component type is started.
*
* With this implementation, for example the first Batching Span Processor would have `batching_span_processor/0`
* as `otel.component.name`, the second one `batching_span_processor/1` and so on.
* These values will therefore be reused in the case of an application restart.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_NAME$1 = "otel.component.name";
/**
* A name identifying the type of the OpenTelemetry component.
*
* @example batching_span_processor
* @example com.example.MySpanExporter
*
* @note If none of the standardized values apply, implementations **SHOULD** use the language-defined name of the type.
* E.g. for Java the fully qualified classname **SHOULD** be used in this case.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_TYPE$1 = "otel.component.type";
/**
* Enum value "batching_log_processor" for attribute {@link ATTR_OTEL_COMPONENT_TYPE}.
*
* The builtin SDK batching log record processor
*
* @experimental This enum value is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var OTEL_COMPONENT_TYPE_VALUE_BATCHING_LOG_PROCESSOR = "batching_log_processor";
/**
* Describes a class of error the operation ended with.
*
* @example timeout
* @example java.net.UnknownHostException
* @example server_certificate_invalid
* @example 500
*
* @note The `error.type` **SHOULD** be predictable, and **SHOULD** have low cardinality.
*
* When `error.type` is set to a type (e.g., an exception type), its
* canonical class name identifying the type within the artifact **SHOULD** be used.
*
* Instrumentations **SHOULD** document the list of errors they report.
*
* The cardinality of `error.type` within one instrumentation library **SHOULD** be low.
* Telemetry consumers that aggregate data from multiple instrumentation libraries and applications
* should be prepared for `error.type` to have high cardinality at query time when no
* additional filters are applied.
*
* If the operation has completed successfully, instrumentations **SHOULD NOT** set `error.type`.
*
* If a specific domain defines its own set of error identifiers (such as HTTP or RPC status codes),
* it's **RECOMMENDED** to:
*
*   - Use a domain-specific attribute
*   - Set `error.type` to capture all errors, regardless of whether they are defined within the domain-specific set or not.
*/
var ATTR_ERROR_TYPE = "error.type";
/**
* Generates `otel.sdk.log.*` metrics.
* https://opentelemetry.io/docs/specs/semconv/otel/sdk-metrics/#log-metrics
*/
var LoggerMetrics = class {
	createdLogs;
	constructor(meter) {
		this.createdLogs = meter.createCounter(METRIC_OTEL_SDK_LOG_CREATED, {
			unit: "{log_record}",
			description: "The number of logs submitted to enabled SDK Loggers."
		});
	}
	emitLog() {
		this.createdLogs.add(1);
	}
};
var VERSION$1 = "0.222.0";
init_esm();
var DEFAULT_LOGGER_CONFIG = {
	disabled: false,
	minimumSeverity: SeverityNumber.UNSPECIFIED,
	traceBased: false
};
/**
* Default LoggerConfigurator that returns the default config for all loggers
*/
var DEFAULT_LOGGER_CONFIGURATOR = () => ({ ...DEFAULT_LOGGER_CONFIG });
var LoggerProviderSharedState = class {
	loggers = /* @__PURE__ */ new Map();
	activeProcessor;
	registeredLogRecordProcessors = [];
	resource;
	logRecordLimits;
	processors;
	loggerMetrics;
	hasShutdown = false;
	_loggerConfigurator;
	_loggerConfigs = /* @__PURE__ */ new Map();
	constructor(resource, logRecordLimits, processors, loggerConfigurator, meterProvider) {
		this.resource = resource;
		this.logRecordLimits = logRecordLimits;
		this.processors = processors;
		if (processors.length > 0) {
			this.registeredLogRecordProcessors = processors;
			this.activeProcessor = new MultiLogRecordProcessor(this.registeredLogRecordProcessors);
		} else this.activeProcessor = new NoopLogRecordProcessor();
		this._loggerConfigurator = loggerConfigurator ?? DEFAULT_LOGGER_CONFIGURATOR;
		const meter = meterProvider ? meterProvider.getMeter("@opentelemetry/sdk-logs", VERSION$1) : createNoopMeter();
		this.loggerMetrics = new LoggerMetrics(meter);
	}
	/**
	* Get the LoggerConfig for a given instrumentation scope.
	* Uses the LoggerConfigurator function to compute the config on first access
	* and caches the result.
	*
	* @experimental This feature is in development as per the OpenTelemetry specification.
	*/
	getLoggerConfig(instrumentationScope) {
		const key = getInstrumentationScopeKey(instrumentationScope);
		let config = this._loggerConfigs.get(key);
		if (config) return config;
		config = this._loggerConfigurator(instrumentationScope);
		this._loggerConfigs.set(key, config);
		return config;
	}
};
init_esm();
var LoggerProvider = class {
	_shutdownOnce;
	_sharedState;
	constructor(config = {}) {
		const mergedConfig = {
			resource: config.resource ?? defaultResource(),
			logRecordLimits: {
				attributeCountLimit: config.logRecordLimits?.attributeCountLimit ?? 128,
				attributeValueLengthLimit: config.logRecordLimits?.attributeValueLengthLimit ?? Infinity
			},
			loggerConfigurator: config.loggerConfigurator ?? DEFAULT_LOGGER_CONFIGURATOR,
			processors: config.processors ?? [],
			meterProvider: config.meterProvider
		};
		this._sharedState = new LoggerProviderSharedState(mergedConfig.resource, mergedConfig.logRecordLimits, mergedConfig.processors, mergedConfig.loggerConfigurator, mergedConfig.meterProvider);
		this._shutdownOnce = new BindOnceFuture(this._shutdown, this);
	}
	/**
	* Get a logger with the configuration of the LoggerProvider.
	*/
	getLogger(name, version, options) {
		if (this._shutdownOnce.isCalled) {
			diag.warn("A shutdown LoggerProvider cannot provide a Logger");
			return createNoopLogger();
		}
		if (!name) diag.warn("Logger requested without instrumentation scope name.");
		const instrumentationScope = {
			name: name || "unknown",
			version,
			schemaUrl: options?.schemaUrl,
			...normalizeScopeAttributes(this._sharedState.logRecordLimits, options?.attributes)
		};
		const key = getInstrumentationScopeKey(instrumentationScope);
		if (!this._sharedState.loggers.has(key)) this._sharedState.loggers.set(key, new Logger(instrumentationScope, this._sharedState));
		return this._sharedState.loggers.get(key);
	}
	/**
	* Notifies all registered LogRecordProcessor to flush any buffered data.
	*
	* Returns a promise which is resolved when all flushes are complete.
	*/
	forceFlush(options) {
		if (this._shutdownOnce.isCalled) {
			diag.warn("invalid attempt to force flush after LoggerProvider shutdown");
			return this._shutdownOnce.promise;
		}
		return this._sharedState.activeProcessor.forceFlush(options);
	}
	/**
	* Flush all buffered data and shut down the LoggerProvider and all registered
	* LogRecordProcessor.
	*
	* Returns a promise which is resolved when all flushes are complete.
	*/
	shutdown() {
		if (this._shutdownOnce.isCalled) {
			diag.warn("shutdown may only be called once per LoggerProvider");
			return this._shutdownOnce.promise;
		}
		return this._shutdownOnce.call();
	}
	_shutdown() {
		this._sharedState.hasShutdown = true;
		return this._sharedState.activeProcessor.shutdown();
	}
};
var componentCounter$1 = /* @__PURE__ */ new Map();
var LogRecordProcessorMetrics = class {
	processedLogs;
	queueSize;
	queueSizeCallback;
	standardAttrs;
	droppedAttrs;
	constructor(componentType, meter, queueConfig) {
		const counter = componentCounter$1.get(componentType) ?? 0;
		componentCounter$1.set(componentType, counter + 1);
		this.standardAttrs = {
			[ATTR_OTEL_COMPONENT_TYPE$1]: componentType,
			[ATTR_OTEL_COMPONENT_NAME$1]: `${componentType}/${counter}`
		};
		this.droppedAttrs = {
			...this.standardAttrs,
			[ATTR_ERROR_TYPE]: "queue_full"
		};
		this.processedLogs = meter.createCounter(METRIC_OTEL_SDK_PROCESSOR_LOG_PROCESSED, {
			unit: "{log_record}",
			description: "The number of log records for which the processing has finished, either successful or failed."
		});
		if (queueConfig) {
			const { capacity, getQueueSize } = queueConfig;
			meter.createUpDownCounter(METRIC_OTEL_SDK_PROCESSOR_LOG_QUEUE_CAPACITY, {
				unit: "{log_record}",
				description: "The maximum number of log records the queue of a given instance of an SDK log processor can hold."
			}).add(capacity, this.standardAttrs);
			this.queueSize = meter.createObservableUpDownCounter(METRIC_OTEL_SDK_PROCESSOR_LOG_QUEUE_SIZE, {
				unit: "{log_record}",
				description: "The number of log records in the queue of a given instance of an SDK log processor."
			});
			this.queueSizeCallback = (result) => result.observe(getQueueSize(), this.standardAttrs);
			this.queueSize.addCallback(this.queueSizeCallback);
		}
	}
	dropLogs(count) {
		this.processedLogs.add(count, this.droppedAttrs);
	}
	finishLogs(count, error) {
		if (!error) {
			this.processedLogs.add(count, this.standardAttrs);
			return;
		}
		const attrs = {
			...this.standardAttrs,
			[ATTR_ERROR_TYPE]: error.name
		};
		this.processedLogs.add(count, attrs);
	}
	shutdown() {
		if (this.queueSize && this.queueSizeCallback) this.queueSize.removeCallback(this.queueSizeCallback);
	}
};
init_esm();
/**
* Waits for all pending async resources in the log records to be resolved.
*/
async function waitForResources(logRecords) {
	const pendingResources = [];
	for (let i = 0, len = logRecords.length; i < len; i++) {
		const logRecord = logRecords[i];
		if (logRecord.resource.asyncAttributesPending && logRecord.resource.waitForAsyncAttributes) pendingResources.push(logRecord.resource.waitForAsyncAttributes());
	}
	if (pendingResources != null && pendingResources.length > 0) await Promise.all(pendingResources);
}
/**
* Represents an export operation that handles the entire export workflow.
*/
var ExportOperation = class {
	_exportCompleted;
	_exportScheduledPromise;
	_metrics;
	_exportScheduledResolve;
	constructor(exporter, logRecords, exportTimeoutMillis, metrics) {
		this._exportScheduledPromise = new Promise((resolve) => {
			this._exportScheduledResolve = resolve;
		});
		this._exportCompleted = this._executeExport(exporter, logRecords, exportTimeoutMillis);
		this._metrics = metrics;
	}
	/** Get the promise that resolves when the export completes */
	get exportCompleted() {
		return this._exportCompleted;
	}
	/** Get the promise that resolves when exporter.export() has been called */
	get exportScheduled() {
		return this._exportScheduledPromise;
	}
	async _executeExport(exporter, logRecords, exportTimeoutMillis) {
		try {
			await waitForResources(logRecords);
			await context.with(suppressTracing(context.active()), async () => {
				return this._exportWithTimeout(exporter, logRecords, exportTimeoutMillis);
			});
		} catch (e) {
			globalErrorHandler(e);
			this._exportScheduledResolve();
		}
	}
	async _exportWithTimeout(exporter, logRecords, exportTimeoutMillis) {
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				reject(/* @__PURE__ */ new Error("Timeout"));
			}, exportTimeoutMillis);
			exporter.export(logRecords, (result) => {
				this._metrics.finishLogs(logRecords.length, result.error);
				clearTimeout(timer);
				if (result.code === ExportResultCode.SUCCESS) resolve();
				else reject(result.error ?? /* @__PURE__ */ new Error("BatchLogRecordProcessor: log record export failed"));
			});
			this._exportScheduledResolve();
		});
	}
};
var BatchLogRecordProcessorBase = class {
	_maxExportBatchSize;
	_maxQueueSize;
	_scheduledDelayMillis;
	_exportTimeoutMillis;
	_exporter;
	_metrics;
	_currentExport = null;
	_finishedLogRecords = [];
	_timer;
	_shutdownOnce;
	_flushing = false;
	constructor(options) {
		this._exporter = options.exporter;
		this._maxExportBatchSize = options.maxExportBatchSize ?? 512;
		this._maxQueueSize = options.maxQueueSize ?? 2048;
		this._scheduledDelayMillis = options.scheduledDelayMillis ?? 1e3;
		this._exportTimeoutMillis = options.exportTimeoutMillis ?? 3e4;
		this._shutdownOnce = new BindOnceFuture(this._shutdown, this);
		if (this._maxExportBatchSize > this._maxQueueSize) {
			diag.warn("BatchLogRecordProcessor: maxExportBatchSize must be smaller or equal to maxQueueSize, setting maxExportBatchSize to match maxQueueSize");
			this._maxExportBatchSize = this._maxQueueSize;
		}
		const meter = options?.selfObsMeterProvider ? options.selfObsMeterProvider.getMeter("@opentelemetry/sdk-logs") : createNoopMeter();
		this._metrics = new LogRecordProcessorMetrics(OTEL_COMPONENT_TYPE_VALUE_BATCHING_LOG_PROCESSOR, meter, {
			capacity: this._maxQueueSize,
			getQueueSize: () => this._finishedLogRecords.length
		});
	}
	onEmit(logRecord) {
		if (this._shutdownOnce.isCalled) return;
		this._addToBuffer(logRecord);
	}
	forceFlush() {
		if (this._shutdownOnce.isCalled) return this._shutdownOnce.promise;
		return this._flushAll();
	}
	/** Add a LogRecord in the buffer. */
	_addToBuffer(logRecord) {
		if (this._finishedLogRecords.length >= this._maxQueueSize) {
			this._metrics.dropLogs(1);
			return;
		}
		this._finishedLogRecords.push(logRecord);
		this._maybeStartTimer();
	}
	shutdown() {
		return this._shutdownOnce.call();
	}
	async _shutdown() {
		this.onShutdown();
		await this._flushAll();
		this._metrics.shutdown();
		await this._exporter.shutdown();
	}
	/**
	* Send all LogRecords to the exporter respecting the batch size limit
	* This function is used only on forceFlush or shutdown,
	* for all other cases _exportOneBatch should be used
	* */
	async _flushAll() {
		if (this._flushing) return;
		this._flushing = true;
		let toFlush = this._finishedLogRecords;
		this._finishedLogRecords = [];
		this._clearTimer();
		const inFlight = this._currentExport;
		if (inFlight !== null) {
			await this._exporter.forceFlush();
			await inFlight.exportCompleted;
			this._currentExport = null;
		}
		while (toFlush.length > 0) {
			let batch;
			if (toFlush.length <= this._maxExportBatchSize) {
				batch = toFlush;
				toFlush = [];
			} else batch = toFlush.splice(0, this._maxExportBatchSize);
			const exportOp = new ExportOperation(this._exporter, batch, this._exportTimeoutMillis, this._metrics);
			this._currentExport = exportOp;
			try {
				await exportOp.exportScheduled;
				await this._exporter.forceFlush();
				await exportOp.exportCompleted;
			} catch (e) {
				globalErrorHandler(e);
			} finally {
				this._currentExport = null;
			}
		}
		this._flushing = false;
		this._maybeStartTimer();
	}
	/**
	* Extracts one batch from the buffer.
	* Returns null if buffer is empty.
	*/
	_extractBatch() {
		if (this._finishedLogRecords.length === 0) return null;
		if (this._finishedLogRecords.length <= this._maxExportBatchSize) {
			const batch = this._finishedLogRecords;
			this._finishedLogRecords = [];
			return batch;
		} else return this._finishedLogRecords.splice(0, this._maxExportBatchSize);
	}
	_exportOneBatch() {
		this._clearTimer();
		const logRecords = this._extractBatch();
		if (logRecords === null) return;
		const exportOp = new ExportOperation(this._exporter, logRecords, this._exportTimeoutMillis, this._metrics);
		this._currentExport = exportOp;
		exportOp.exportCompleted.then(() => {
			this._currentExport = null;
			this._maybeStartTimer();
		}).catch((error) => {
			this._currentExport = null;
			globalErrorHandler(error);
			this._maybeStartTimer();
		});
	}
	_maybeStartTimer() {
		if (this._shutdownOnce.isCalled) return;
		if (this._flushing) return;
		if (this._finishedLogRecords.length === 0) return;
		if (this._currentExport !== null) return;
		if (this._finishedLogRecords.length >= this._maxExportBatchSize) {
			this._exportOneBatch();
			return;
		}
		if (this._timer !== void 0) return;
		this._timer = setTimeout(() => {
			this._timer = void 0;
			this._exportOneBatch();
		}, this._scheduledDelayMillis);
		if (typeof this._timer !== "number") this._timer.unref();
	}
	_clearTimer() {
		if (this._timer !== void 0) {
			clearTimeout(this._timer);
			this._timer = void 0;
		}
	}
};
var BatchLogRecordProcessor = class extends BatchLogRecordProcessorBase {
	onShutdown() {}
};
var ExceptionEventName = "exception";
/**
* Well-known symbol used by Node.js `util.inspect` (and `console.*`) to
* render an object via a custom representation. Defined as a global Symbol
* so it works without importing from `node:util`, keeping this module safe
* for browser builds (where the symbol is simply never looked up).
*/
var inspectCustom = Symbol.for("nodejs.util.inspect.custom");
/**
* Collect a Resource's settled attributes without touching the
* `attributes` getter, which emits diag.error/debug entries when async
* attribute detectors are still pending. Promise-like (unsettled)
* entries are silently skipped so logging a Span/Tracer/Provider during
* startup doesn't recurse through the diag pipeline.
*/
function settledResourceAttributes(resource) {
	const attrs = {};
	for (const [k, v] of resource.getRawAttributes()) {
		if (typeof v?.then === "function") continue;
		if (v != null) attrs[k] ??= v;
	}
	return attrs;
}
/**
* Build a class-tagged inspect representation. Returns a stub like
* `[ClassName]` once the recursion budget is exhausted, otherwise returns
* `ClassName <inspected payload>` so nested fields keep proper coloring,
* indentation, and depth handling. In environments that don't supply an
* `inspect` callback (e.g. browsers), falls back to returning the raw
* payload object.
*/
function formatInspect(className, payload, depth, options, inspect) {
	if (typeof depth === "number" && depth < 0) {
		const tag = `[${className}]`;
		return options?.stylize ? options.stylize(tag, "special") : tag;
	}
	if (typeof inspect !== "function" || !options) return payload;
	return `${className} ${inspect(payload, {
		...options,
		depth: options.depth == null ? options.depth : options.depth - 1
	})}`;
}
init_esm();
/**
* This class represents a span.
*/
var SpanImpl = class {
	_spanContext;
	kind;
	parentSpanContext;
	attributes = {};
	links = [];
	events = [];
	startTime;
	resource;
	instrumentationScope;
	_droppedAttributesCount = 0;
	_droppedEventsCount = 0;
	_droppedLinksCount = 0;
	_attributesCount = 0;
	name;
	status = { code: SpanStatusCode.UNSET };
	endTime = [0, 0];
	_ended = false;
	_duration = [-1, -1];
	_spanProcessor;
	_spanLimits;
	_attributeValueLengthLimit;
	_recordEndMetrics;
	_performanceStartTime;
	_performanceOffset;
	_startTimeProvided;
	/**
	* Constructs a new SpanImpl instance.
	*/
	constructor(opts) {
		const now = Date.now();
		this._spanContext = opts.spanContext;
		this._performanceStartTime = otperformance.now();
		this._performanceOffset = now - (this._performanceStartTime + otperformance.timeOrigin);
		this._startTimeProvided = opts.startTime != null;
		this._spanLimits = opts.spanLimits;
		this._attributeValueLengthLimit = this._spanLimits.attributeValueLengthLimit ?? 0;
		this._spanProcessor = opts.spanProcessor;
		this.name = opts.name;
		this.parentSpanContext = opts.parentSpanContext;
		this.kind = opts.kind;
		if (opts.links) for (const link of opts.links) this.addLink(link);
		this.startTime = this._getTime(opts.startTime ?? now);
		this.resource = opts.resource;
		this.instrumentationScope = opts.scope;
		this._recordEndMetrics = opts.recordEndMetrics;
		if (opts.attributes != null) this.setAttributes(opts.attributes);
		this._spanProcessor.onStart(this, opts.context);
	}
	spanContext() {
		return this._spanContext;
	}
	setAttribute(key, value) {
		if (value == null || this._isSpanEnded()) return this;
		if (key.length === 0) {
			diag.warn(`Invalid attribute key: ${key}`);
			return this;
		}
		if (!isAttributeValue(value)) {
			diag.warn(`Invalid attribute value set for key: ${key}`);
			return this;
		}
		const { attributeCountLimit } = this._spanLimits;
		const isNewKey = !Object.prototype.hasOwnProperty.call(this.attributes, key);
		if (attributeCountLimit !== void 0 && this._attributesCount >= attributeCountLimit && isNewKey) {
			this._droppedAttributesCount++;
			return this;
		}
		this.attributes[key] = this._truncateToSize(value);
		if (isNewKey) this._attributesCount++;
		return this;
	}
	setAttributes(attributes) {
		for (const key in attributes) if (Object.prototype.hasOwnProperty.call(attributes, key)) this.setAttribute(key, attributes[key]);
		return this;
	}
	/**
	*
	* @param name Span Name
	* @param [attributesOrStartTime] Span attributes or start time
	*     if type is {@type TimeInput} and 3rd param is undefined
	* @param [timeStamp] Specified time stamp for the event
	*/
	addEvent(name, attributesOrStartTime, timeStamp) {
		if (this._isSpanEnded()) return this;
		const { eventCountLimit } = this._spanLimits;
		if (eventCountLimit === 0) {
			diag.warn("No events allowed.");
			this._droppedEventsCount++;
			return this;
		}
		if (eventCountLimit !== void 0 && this.events.length >= eventCountLimit) {
			if (this._droppedEventsCount === 0) diag.debug("Dropping extra events.");
			this.events.shift();
			this._droppedEventsCount++;
		}
		if (isTimeInput(attributesOrStartTime)) {
			if (!isTimeInput(timeStamp)) timeStamp = attributesOrStartTime;
			attributesOrStartTime = void 0;
		}
		const sanitized = sanitizeAttributes(attributesOrStartTime);
		const { attributePerEventCountLimit } = this._spanLimits;
		const attributes = {};
		let droppedAttributesCount = 0;
		let eventAttributesCount = 0;
		for (const attr in sanitized) {
			if (!Object.prototype.hasOwnProperty.call(sanitized, attr)) continue;
			const attrVal = sanitized[attr];
			if (attributePerEventCountLimit !== void 0 && eventAttributesCount >= attributePerEventCountLimit) {
				droppedAttributesCount++;
				continue;
			}
			attributes[attr] = this._truncateToSize(attrVal);
			eventAttributesCount++;
		}
		this.events.push({
			name,
			attributes,
			time: this._getTime(timeStamp),
			droppedAttributesCount
		});
		return this;
	}
	addLink(link) {
		if (this._isSpanEnded()) return this;
		const { linkCountLimit } = this._spanLimits;
		if (linkCountLimit === 0) {
			this._droppedLinksCount++;
			return this;
		}
		if (linkCountLimit !== void 0 && this.links.length >= linkCountLimit) {
			if (this._droppedLinksCount === 0) diag.debug("Dropping extra links.");
			this.links.shift();
			this._droppedLinksCount++;
		}
		const { attributePerLinkCountLimit } = this._spanLimits;
		const sanitized = sanitizeAttributes(link.attributes);
		const attributes = {};
		let droppedAttributesCount = 0;
		let linkAttributesCount = 0;
		for (const attr in sanitized) {
			if (!Object.prototype.hasOwnProperty.call(sanitized, attr)) continue;
			const attrVal = sanitized[attr];
			if (attributePerLinkCountLimit !== void 0 && linkAttributesCount >= attributePerLinkCountLimit) {
				droppedAttributesCount++;
				continue;
			}
			attributes[attr] = this._truncateToSize(attrVal);
			linkAttributesCount++;
		}
		const processedLink = { context: link.context };
		if (linkAttributesCount > 0) processedLink.attributes = attributes;
		if (droppedAttributesCount > 0) processedLink.droppedAttributesCount = droppedAttributesCount;
		this.links.push(processedLink);
		return this;
	}
	addLinks(links) {
		for (const link of links) this.addLink(link);
		return this;
	}
	setStatus(status) {
		if (this._isSpanEnded()) return this;
		if (status.code === SpanStatusCode.UNSET) return this;
		if (this.status.code === SpanStatusCode.OK) return this;
		const newStatus = { code: status.code };
		if (status.code === SpanStatusCode.ERROR) {
			if (typeof status.message === "string") newStatus.message = status.message;
			else if (status.message != null) diag.warn(`Dropping invalid status.message of type '${typeof status.message}', expected 'string'`);
		}
		this.status = newStatus;
		return this;
	}
	updateName(name) {
		if (this._isSpanEnded()) return this;
		this.name = name;
		return this;
	}
	end(endTime) {
		if (this._isSpanEnded()) {
			diag.error(`${this.name} ${this._spanContext.traceId}-${this._spanContext.spanId} - You can only call end() on a span once.`);
			return;
		}
		this.endTime = this._getTime(endTime);
		this._duration = hrTimeDuration(this.startTime, this.endTime);
		if (this._duration[0] < 0) {
			diag.warn("Inconsistent start and end time, startTime > endTime. Setting span duration to 0ms.", this.startTime, this.endTime);
			this.endTime = this.startTime.slice();
			this._duration = [0, 0];
		}
		if (this._droppedEventsCount > 0) diag.warn(`Dropped ${this._droppedEventsCount} events because eventCountLimit reached`);
		if (this._droppedLinksCount > 0) diag.warn(`Dropped ${this._droppedLinksCount} links because linkCountLimit reached`);
		if (this._spanProcessor.onEnding) this._spanProcessor.onEnding(this);
		this._recordEndMetrics?.();
		this._ended = true;
		this._spanProcessor.onEnd(this);
	}
	_getTime(inp) {
		if (typeof inp === "number" && inp <= otperformance.now()) return hrTime(inp + this._performanceOffset);
		if (typeof inp === "number") return millisToHrTime(inp);
		if (inp instanceof Date) return millisToHrTime(inp.getTime());
		if (isTimeInputHrTime(inp)) return inp;
		if (this._startTimeProvided) return millisToHrTime(Date.now());
		const msDuration = otperformance.now() - this._performanceStartTime;
		return addHrTimes(this.startTime, millisToHrTime(msDuration));
	}
	isRecording() {
		return this._ended === false;
	}
	recordException(exception, time) {
		const attributes = {};
		if (typeof exception === "string") attributes[ATTR_EXCEPTION_MESSAGE] = exception;
		else if (exception) {
			if (exception.code) attributes[ATTR_EXCEPTION_TYPE] = exception.code.toString();
			else if (exception.name) attributes[ATTR_EXCEPTION_TYPE] = exception.name;
			if (exception.message) attributes[ATTR_EXCEPTION_MESSAGE] = exception.message;
			if (exception.stack) attributes[ATTR_EXCEPTION_STACKTRACE] = exception.stack;
		}
		if (attributes["exception.type"] || attributes["exception.message"]) this.addEvent(ExceptionEventName, attributes, time);
		else diag.warn(`Failed to record an exception ${exception}`);
	}
	get duration() {
		return this._duration;
	}
	get ended() {
		return this._ended;
	}
	get droppedAttributesCount() {
		return this._droppedAttributesCount;
	}
	get droppedEventsCount() {
		return this._droppedEventsCount;
	}
	get droppedLinksCount() {
		return this._droppedLinksCount;
	}
	_isSpanEnded() {
		if (this._ended) {
			const error = /* @__PURE__ */ new Error(`Operation attempted on ended Span {traceId: ${this._spanContext.traceId}, spanId: ${this._spanContext.spanId}}`);
			diag.warn(`Cannot execute the operation on ended Span {traceId: ${this._spanContext.traceId}, spanId: ${this._spanContext.spanId}}`, error);
		}
		return this._ended;
	}
	_truncateToLimitUtil(value, limit) {
		if (value.length <= limit) return value;
		return value.substring(0, limit);
	}
	/**
	* If the given attribute value is of type string and has more characters than given {@code attributeValueLengthLimit} then
	* return string with truncated to {@code attributeValueLengthLimit} characters
	*
	* If the given attribute value is array of strings then
	* return new array of strings with each element truncated to {@code attributeValueLengthLimit} characters
	*
	* Otherwise return same Attribute {@code value}
	*
	* @param value Attribute value
	* @returns truncated attribute value if required, otherwise same value
	*/
	_truncateToSize(value) {
		const limit = this._attributeValueLengthLimit;
		if (limit <= 0) {
			diag.warn(`Attribute value limit must be positive, got ${limit}`);
			return value;
		}
		if (typeof value === "string") return this._truncateToLimitUtil(value, limit);
		if (Array.isArray(value)) return value.map((val) => typeof val === "string" ? this._truncateToLimitUtil(val, limit) : val);
		return value;
	}
	[inspectCustom](depth, options, inspect) {
		return formatInspect("SpanImpl", {
			name: this.name,
			kind: this.kind,
			spanContext: this._spanContext,
			parentSpanContext: this.parentSpanContext,
			status: this.status,
			startTime: this.startTime,
			endTime: this.endTime,
			duration: this._duration,
			ended: this._ended,
			attributes: this.attributes,
			events: this.events,
			links: this.links,
			droppedAttributesCount: this._droppedAttributesCount,
			droppedEventsCount: this._droppedEventsCount,
			droppedLinksCount: this._droppedLinksCount,
			instrumentationScope: this.instrumentationScope,
			resource: { attributes: settledResourceAttributes(this.resource) }
		}, depth, options, inspect);
	}
};
/**
* A sampling decision that determines how a {@link Span} will be recorded
* and collected.
*/
var SamplingDecision;
(function(SamplingDecision) {
	/**
	* `Span.isRecording() === false`, span will not be recorded and all events
	* and attributes will be dropped.
	*/
	SamplingDecision[SamplingDecision["NOT_RECORD"] = 0] = "NOT_RECORD";
	/**
	* `Span.isRecording() === true`, but `Sampled` flag in {@link TraceFlags}
	* MUST NOT be set.
	*/
	SamplingDecision[SamplingDecision["RECORD"] = 1] = "RECORD";
	/**
	* `Span.isRecording() === true` AND `Sampled` flag in {@link TraceFlags}
	* MUST be set.
	*/
	SamplingDecision[SamplingDecision["RECORD_AND_SAMPLED"] = 2] = "RECORD_AND_SAMPLED";
})(SamplingDecision || (SamplingDecision = {}));
/**
* A name uniquely identifying the instance of the OpenTelemetry component within its containing SDK instance.
*
* @example otlp_grpc_span_exporter/0
* @example custom-name
*
* @note Implementations **SHOULD** ensure a low cardinality for this attribute, even across application or SDK restarts.
* E.g. implementations **MUST NOT** use UUIDs as values for this attribute.
*
* Implementations **MAY** achieve these goals by following a `<otel.component.type>/<instance-counter>` pattern, e.g. `batching_span_processor/0`.
* Hereby `otel.component.type` refers to the corresponding attribute value of the component.
*
* The value of `instance-counter` **MAY** be automatically assigned by the component and uniqueness within the enclosing SDK instance **MUST** be guaranteed.
* For example, `<instance-counter>` **MAY** be implemented by using a monotonically increasing counter (starting with `0`), which is incremented every time an
* instance of the given component type is started.
*
* With this implementation, for example the first Batching Span Processor would have `batching_span_processor/0`
* as `otel.component.name`, the second one `batching_span_processor/1` and so on.
* These values will therefore be reused in the case of an application restart.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_NAME = "otel.component.name";
/**
* A name identifying the type of the OpenTelemetry component.
*
* @example batching_span_processor
* @example com.example.MySpanExporter
*
* @note If none of the standardized values apply, implementations **SHOULD** use the language-defined name of the type.
* E.g. for Java the fully qualified classname **SHOULD** be used in this case.
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_COMPONENT_TYPE = "otel.component.type";
/**
* Determines whether the span has a parent span, and if so, [whether it is a remote parent](https://opentelemetry.io/docs/specs/otel/trace/api/#isremote)
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_SPAN_PARENT_ORIGIN = "otel.span.parent.origin";
/**
* The result value of the sampler for this span
*
* @experimental This attribute is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var ATTR_OTEL_SPAN_SAMPLING_RESULT = "otel.span.sampling_result";
/**
* The number of spans for which the processing has finished, either successful or failed.
*
* @note For successful processing, `error.type` **MUST NOT** be set. For failed processing, `error.type` **MUST** contain the failure cause.
* For the SDK Simple and Batching Span Processor a span is considered to be processed already when it has been submitted to the exporter, not when the corresponding export call has finished.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_SPAN_PROCESSED = "otel.sdk.processor.span.processed";
/**
* The maximum number of spans the queue of a given instance of an SDK span processor can hold.
*
* @note Only applies to span processors which use a queue, e.g. the SDK Batching Span Processor.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_SPAN_QUEUE_CAPACITY = "otel.sdk.processor.span.queue.capacity";
/**
* The number of spans in the queue of a given instance of an SDK span processor.
*
* @note Only applies to span processors which use a queue, e.g. the SDK Batching Span Processor.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_PROCESSOR_SPAN_QUEUE_SIZE = "otel.sdk.processor.span.queue.size";
/**
* The number of created spans with `recording=true` for which the end operation has not been called yet.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_SPAN_LIVE = "otel.sdk.span.live";
/**
* The number of created spans.
*
* @note Implementations **MUST** record this metric for all spans, even for non-recording ones.
*
* @experimental This metric is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var METRIC_OTEL_SDK_SPAN_STARTED = "otel.sdk.span.started";
/**
* Enum value "batching_span_processor" for attribute {@link ATTR_OTEL_COMPONENT_TYPE}.
*
* The builtin SDK batching span processor
*
* @experimental This enum value is experimental and is subject to breaking changes in minor releases of `@opentelemetry/semantic-conventions`.
*/
var OTEL_COMPONENT_TYPE_VALUE_BATCHING_SPAN_PROCESSOR = "batching_span_processor";
/**
* Generates `otel.sdk.span.*` metrics.
* https://opentelemetry.io/docs/specs/semconv/otel/sdk-metrics/#span-metrics
*/
var TracerMetrics = class {
	startedSpans;
	liveSpans;
	constructor(meter) {
		this.startedSpans = meter.createCounter(METRIC_OTEL_SDK_SPAN_STARTED, {
			unit: "{span}",
			description: "The number of created spans."
		});
		this.liveSpans = meter.createUpDownCounter(METRIC_OTEL_SDK_SPAN_LIVE, {
			unit: "{span}",
			description: "The number of currently live spans."
		});
	}
	startSpan(parentSpanCtx, samplingDecision) {
		const samplingDecisionStr = samplingDecisionToString(samplingDecision);
		this.startedSpans.add(1, {
			[ATTR_OTEL_SPAN_PARENT_ORIGIN]: parentOrigin(parentSpanCtx),
			[ATTR_OTEL_SPAN_SAMPLING_RESULT]: samplingDecisionStr
		});
		if (samplingDecision === SamplingDecision.NOT_RECORD) return () => {};
		const liveSpanAttributes = { [ATTR_OTEL_SPAN_SAMPLING_RESULT]: samplingDecisionStr };
		this.liveSpans.add(1, liveSpanAttributes);
		return () => {
			this.liveSpans.add(-1, liveSpanAttributes);
		};
	}
};
function parentOrigin(parentSpanContext) {
	if (!parentSpanContext) return "none";
	if (parentSpanContext.isRemote) return "remote";
	return "local";
}
function samplingDecisionToString(decision) {
	switch (decision) {
		case SamplingDecision.RECORD_AND_SAMPLED: return "RECORD_AND_SAMPLE";
		case SamplingDecision.RECORD: return "RECORD_ONLY";
		case SamplingDecision.NOT_RECORD: return "DROP";
	}
}
var VERSION = "2.11.0";
init_esm();
/**
* This class represents a basic tracer.
*/
var Tracer = class {
	_sampler;
	_spanLimits;
	_idGenerator;
	instrumentationScope;
	_resource;
	_spanProcessor;
	_tracerMetrics;
	/**
	* Constructs a new Tracer instance.
	*/
	constructor(instrumentationScope, options) {
		this.instrumentationScope = instrumentationScope;
		this._sampler = options.sampler;
		this._spanLimits = options.spanLimits;
		this._resource = options.resource;
		this._idGenerator = options.idGenerator;
		this._spanProcessor = options.spanProcessor;
		const meter = options.meterProvider.getMeter("@opentelemetry/sdk-trace", VERSION);
		this._tracerMetrics = new TracerMetrics(meter);
	}
	/**
	* Starts a new Span or returns the default NoopSpan based on the sampling
	* decision.
	*/
	startSpan(name, options = {}, context$1 = context.active()) {
		if (options.root) context$1 = trace.deleteSpan(context$1);
		const parentSpan = trace.getSpan(context$1);
		if (isTracingSuppressed(context$1)) {
			diag.debug("Instrumentation suppressed, returning Noop Span");
			return trace.wrapSpanContext(INVALID_SPAN_CONTEXT);
		}
		const parentSpanContext = parentSpan?.spanContext();
		const spanId = this._idGenerator.generateSpanId();
		let validParentSpanContext;
		let traceId;
		let traceState;
		if (!parentSpanContext || !trace.isSpanContextValid(parentSpanContext)) traceId = this._idGenerator.generateTraceId();
		else {
			traceId = parentSpanContext.traceId;
			traceState = parentSpanContext.traceState;
			validParentSpanContext = parentSpanContext;
		}
		const spanKind = options.kind ?? SpanKind.INTERNAL;
		const links = (options.links ?? []).map((link) => {
			return {
				context: link.context,
				attributes: sanitizeAttributes(link.attributes)
			};
		});
		const attributes = sanitizeAttributes(options.attributes);
		const samplingResult = this._sampler.shouldSample(context$1, traceId, name, spanKind, attributes, links);
		const recordEndMetrics = this._tracerMetrics.startSpan(parentSpanContext, samplingResult.decision);
		traceState = samplingResult.traceState ?? traceState;
		const traceFlags = samplingResult.decision === SamplingDecision$1.RECORD_AND_SAMPLED ? TraceFlags.SAMPLED : TraceFlags.NONE;
		const spanContext = {
			traceId,
			spanId,
			traceFlags,
			traceState
		};
		if (samplingResult.decision === SamplingDecision$1.NOT_RECORD) {
			diag.debug("Recording is off, propagating context in a non-recording span");
			return trace.wrapSpanContext(spanContext);
		}
		const initAttributes = sanitizeAttributes(Object.assign(attributes, samplingResult.attributes));
		return new SpanImpl({
			resource: this._resource,
			scope: this.instrumentationScope,
			context: context$1,
			spanContext,
			name,
			kind: spanKind,
			links,
			parentSpanContext: validParentSpanContext,
			attributes: initAttributes,
			startTime: options.startTime,
			spanProcessor: this._spanProcessor,
			spanLimits: this._spanLimits,
			recordEndMetrics
		});
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
		const parentContext = ctx ?? context.active();
		const span = this.startSpan(name, opts, parentContext);
		const contextWithSpanSet = trace.setSpan(parentContext, span);
		return context.with(contextWithSpanSet, fn, void 0, span);
	}
	[inspectCustom](depth, options, inspect) {
		return formatInspect("Tracer", {
			instrumentationScope: this.instrumentationScope,
			resource: { attributes: settledResourceAttributes(this._resource) },
			spanLimits: this._spanLimits
		}, depth, options, inspect);
	}
};
/**
* Implementation of the {@link SpanProcessor} that simply forwards all
* received events to a list of {@link SpanProcessor}s.
*/
var MultiSpanProcessor = class {
	_spanProcessors;
	constructor(spanProcessors) {
		this._spanProcessors = spanProcessors;
	}
	forceFlush() {
		const promises = [];
		for (const spanProcessor of this._spanProcessors) promises.push(spanProcessor.forceFlush());
		return new Promise((resolve) => {
			Promise.all(promises).then(() => {
				resolve();
			}).catch((error) => {
				globalErrorHandler(error || /* @__PURE__ */ new Error("MultiSpanProcessor: forceFlush failed"));
				resolve();
			});
		});
	}
	onStart(span, context) {
		for (const spanProcessor of this._spanProcessors) spanProcessor.onStart(span, context);
	}
	onEnding(span) {
		for (const spanProcessor of this._spanProcessors) if (spanProcessor.onEnding) spanProcessor.onEnding(span);
	}
	onEnd(span) {
		for (const spanProcessor of this._spanProcessors) spanProcessor.onEnd(span);
	}
	shutdown() {
		const promises = [];
		for (const spanProcessor of this._spanProcessors) promises.push(spanProcessor.shutdown());
		return new Promise((resolve, reject) => {
			Promise.all(promises).then(() => {
				resolve();
			}, reject);
		});
	}
};
/** Sampler that samples no traces. */
var AlwaysOffSampler = class {
	shouldSample() {
		return { decision: SamplingDecision.NOT_RECORD };
	}
	toString() {
		return "AlwaysOffSampler";
	}
};
/** Sampler that samples all traces. */
var AlwaysOnSampler = class {
	shouldSample() {
		return { decision: SamplingDecision.RECORD_AND_SAMPLED };
	}
	toString() {
		return "AlwaysOnSampler";
	}
};
init_esm();
/**
* A composite sampler that either respects the parent span's sampling decision
* or delegates to `delegateSampler` for root spans.
*/
var ParentBasedSampler = class {
	_root;
	_remoteParentSampled;
	_remoteParentNotSampled;
	_localParentSampled;
	_localParentNotSampled;
	constructor(config) {
		this._root = config.root;
		if (!this._root) {
			globalErrorHandler(/* @__PURE__ */ new Error("ParentBasedSampler must have a root sampler configured"));
			this._root = new AlwaysOnSampler();
		}
		this._remoteParentSampled = config.remoteParentSampled ?? new AlwaysOnSampler();
		this._remoteParentNotSampled = config.remoteParentNotSampled ?? new AlwaysOffSampler();
		this._localParentSampled = config.localParentSampled ?? new AlwaysOnSampler();
		this._localParentNotSampled = config.localParentNotSampled ?? new AlwaysOffSampler();
	}
	shouldSample(context, traceId, spanName, spanKind, attributes, links) {
		const parentContext = trace.getSpanContext(context);
		if (!parentContext || !isSpanContextValid(parentContext)) return this._root.shouldSample(context, traceId, spanName, spanKind, attributes, links);
		if (parentContext.isRemote) {
			if (parentContext.traceFlags & TraceFlags.SAMPLED) return this._remoteParentSampled.shouldSample(context, traceId, spanName, spanKind, attributes, links);
			return this._remoteParentNotSampled.shouldSample(context, traceId, spanName, spanKind, attributes, links);
		}
		if (parentContext.traceFlags & TraceFlags.SAMPLED) return this._localParentSampled.shouldSample(context, traceId, spanName, spanKind, attributes, links);
		return this._localParentNotSampled.shouldSample(context, traceId, spanName, spanKind, attributes, links);
	}
	toString() {
		return `ParentBased{root=${this._root.toString()}, remoteParentSampled=${this._remoteParentSampled.toString()}, remoteParentNotSampled=${this._remoteParentNotSampled.toString()}, localParentSampled=${this._localParentSampled.toString()}, localParentNotSampled=${this._localParentNotSampled.toString()}}`;
	}
};
var componentCounter = /* @__PURE__ */ new Map();
var SpanProcessorMetrics = class {
	processedSpans;
	queueSize;
	queueSizeCallback;
	standardAttrs;
	droppedAttrs;
	constructor(componentType, meter, queueConfig) {
		const counter = componentCounter.get(componentType) ?? 0;
		componentCounter.set(componentType, counter + 1);
		this.standardAttrs = {
			[ATTR_OTEL_COMPONENT_TYPE]: componentType,
			[ATTR_OTEL_COMPONENT_NAME]: `${componentType}/${counter}`
		};
		this.droppedAttrs = {
			...this.standardAttrs,
			[ATTR_ERROR_TYPE$2]: "queue_full"
		};
		this.processedSpans = meter.createCounter(METRIC_OTEL_SDK_PROCESSOR_SPAN_PROCESSED, {
			unit: "{span}",
			description: "The number of spans for which the processing has finished, either successful or failed."
		});
		if (queueConfig) {
			const { capacity, getQueueSize } = queueConfig;
			meter.createUpDownCounter(METRIC_OTEL_SDK_PROCESSOR_SPAN_QUEUE_CAPACITY, {
				unit: "{span}",
				description: "The maximum number of spans the queue of a given instance of an SDK span processor can hold."
			}).add(capacity, this.standardAttrs);
			this.queueSize = meter.createObservableUpDownCounter(METRIC_OTEL_SDK_PROCESSOR_SPAN_QUEUE_SIZE, {
				unit: "{span}",
				description: "The number of spans in the queue of a given instance of an SDK span processor."
			});
			this.queueSizeCallback = (result) => result.observe(getQueueSize(), this.standardAttrs);
			this.queueSize.addCallback(this.queueSizeCallback);
		}
	}
	dropSpans(count) {
		this.processedSpans.add(count, this.droppedAttrs);
	}
	finishSpans(count, error) {
		if (!error) {
			this.processedSpans.add(count, this.standardAttrs);
			return;
		}
		const attrs = {
			...this.standardAttrs,
			[ATTR_ERROR_TYPE$2]: error.name
		};
		this.processedSpans.add(count, attrs);
	}
	shutdown() {
		if (this.queueSize && this.queueSizeCallback) this.queueSize.removeCallback(this.queueSizeCallback);
	}
};
init_esm();
/**
* Implementation of the {@link SpanProcessor} that batches spans exported by
* the SDK then pushes them to the exporter pipeline.
*/
var BatchSpanProcessorBase = class {
	_maxExportBatchSize;
	_maxQueueSize;
	_scheduledDelayMillis;
	_exportTimeoutMillis;
	_exporter;
	_metrics;
	_isExporting = false;
	_finishedSpans = [];
	_timer;
	_shutdownOnce;
	_droppedSpansCount = 0;
	constructor(options) {
		this._exporter = options.exporter;
		this._maxExportBatchSize = options.maxExportBatchSize ?? 512;
		this._maxQueueSize = options.maxQueueSize ?? 2048;
		this._scheduledDelayMillis = options.scheduledDelayMillis ?? 5e3;
		this._exportTimeoutMillis = options.exportTimeoutMillis ?? 3e4;
		this._shutdownOnce = new BindOnceFuture(this._shutdown, this);
		if (this._maxExportBatchSize > this._maxQueueSize) {
			diag.warn("BatchSpanProcessor: maxExportBatchSize must be smaller or equal to maxQueueSize, setting maxExportBatchSize to match maxQueueSize");
			this._maxExportBatchSize = this._maxQueueSize;
		}
		const meter = options.selfObsMeterProvider ? options.selfObsMeterProvider.getMeter("@opentelemetry/sdk-trace") : createNoopMeter();
		this._metrics = new SpanProcessorMetrics(OTEL_COMPONENT_TYPE_VALUE_BATCHING_SPAN_PROCESSOR, meter, {
			capacity: this._maxQueueSize,
			getQueueSize: () => this._finishedSpans.length
		});
	}
	forceFlush() {
		if (this._shutdownOnce.isCalled) return this._shutdownOnce.promise;
		return this._flushAll();
	}
	onStart(_span, _parentContext) {}
	onEnd(span) {
		if (this._shutdownOnce.isCalled) return;
		if ((span.spanContext().traceFlags & TraceFlags.SAMPLED) === 0) return;
		this._addToBuffer(span);
	}
	shutdown() {
		return this._shutdownOnce.call();
	}
	_shutdown() {
		return Promise.resolve().then(() => {
			return this.onShutdown();
		}).then(() => {
			return this._flushAll();
		}).then(() => {
			this._metrics.shutdown();
			return this._exporter.shutdown();
		});
	}
	/** Add a span in the buffer. */
	_addToBuffer(span) {
		if (this._finishedSpans.length >= this._maxQueueSize) {
			if (this._droppedSpansCount === 0) diag.debug("maxQueueSize reached, dropping spans");
			this._droppedSpansCount++;
			this._metrics.dropSpans(1);
			return;
		}
		if (this._droppedSpansCount > 0) {
			diag.warn(`Dropped ${this._droppedSpansCount} spans because maxQueueSize reached`);
			this._droppedSpansCount = 0;
		}
		this._finishedSpans.push(span);
		this._maybeStartTimer();
	}
	/**
	* Send all spans to the exporter respecting the batch size limit
	* This function is used only on forceFlush or shutdown,
	* for all other cases _flush should be used
	* */
	_flushAll() {
		return new Promise((resolve, reject) => {
			const promises = [];
			const count = Math.ceil(this._finishedSpans.length / this._maxExportBatchSize);
			for (let i = 0, j = count; i < j; i++) promises.push(this._flushOneBatch());
			Promise.all(promises).then(() => {
				resolve();
			}).catch(reject);
		});
	}
	_flushOneBatch() {
		this._clearTimer();
		if (this._finishedSpans.length === 0) return Promise.resolve();
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				reject(/* @__PURE__ */ new Error("Timeout"));
			}, this._exportTimeoutMillis);
			context.with(suppressTracing(context.active()), () => {
				let spans;
				if (this._finishedSpans.length <= this._maxExportBatchSize) {
					spans = this._finishedSpans;
					this._finishedSpans = [];
				} else spans = this._finishedSpans.splice(0, this._maxExportBatchSize);
				const doExport = () => this._exporter.export(spans, (result) => {
					clearTimeout(timer);
					this._metrics.finishSpans(spans.length, result.error);
					if (result.code === ExportResultCode.SUCCESS) resolve();
					else reject(result.error ?? /* @__PURE__ */ new Error("BatchSpanProcessor: span export failed"));
				});
				let pendingResources = null;
				for (let i = 0, len = spans.length; i < len; i++) {
					const span = spans[i];
					if (span.resource.asyncAttributesPending && span.resource.waitForAsyncAttributes) {
						pendingResources ??= [];
						pendingResources.push(span.resource.waitForAsyncAttributes());
					}
				}
				if (pendingResources === null) doExport();
				else Promise.all(pendingResources).then(doExport, (err) => {
					globalErrorHandler(err);
					reject(err);
				});
			});
		});
	}
	_maybeStartTimer() {
		if (this._isExporting) return;
		const flush = () => {
			this._isExporting = true;
			this._flushOneBatch().finally(() => {
				this._isExporting = false;
				if (this._finishedSpans.length > 0) {
					this._clearTimer();
					this._maybeStartTimer();
				}
			}).catch((e) => {
				this._isExporting = false;
				globalErrorHandler(e);
			});
		};
		if (this._finishedSpans.length >= this._maxExportBatchSize) return flush();
		if (this._timer !== void 0) return;
		this._timer = setTimeout(() => flush(), this._scheduledDelayMillis);
		if (typeof this._timer !== "number") this._timer.unref();
	}
	_clearTimer() {
		if (this._timer !== void 0) {
			clearTimeout(this._timer);
			this._timer = void 0;
		}
	}
};
var BatchSpanProcessor = class extends BatchSpanProcessorBase {
	onShutdown() {}
};
var SPAN_ID_BYTES = 8;
var TRACE_ID_BYTES = 16;
var RandomIdGenerator = class {
	/**
	* Returns a random 16-byte trace ID formatted/encoded as a 32 lowercase hex
	* characters corresponding to 128 bits.
	*/
	generateTraceId = getIdGenerator(TRACE_ID_BYTES);
	/**
	* Returns a random 8-byte span ID formatted/encoded as a 16 lowercase hex
	* characters corresponding to 64 bits.
	*/
	generateSpanId = getIdGenerator(SPAN_ID_BYTES);
};
var SHARED_BUFFER = Buffer.allocUnsafe(TRACE_ID_BYTES);
function getIdGenerator(bytes) {
	return function generateId() {
		for (let i = 0; i < bytes / 4; i++) SHARED_BUFFER.writeUInt32BE(Math.random() * 2 ** 32 >>> 0, i * 4);
		for (let i = 0; i < bytes; i++) if (SHARED_BUFFER[i] > 0) break;
		else if (i === bytes - 1) SHARED_BUFFER[bytes - 1] = 1;
		return SHARED_BUFFER.toString("hex", 0, bytes);
	};
}
init_esm();
var ForceFlushState;
(function(ForceFlushState) {
	ForceFlushState[ForceFlushState["resolved"] = 0] = "resolved";
	ForceFlushState[ForceFlushState["timeout"] = 1] = "timeout";
	ForceFlushState[ForceFlushState["error"] = 2] = "error";
	ForceFlushState[ForceFlushState["unresolved"] = 3] = "unresolved";
})(ForceFlushState || (ForceFlushState = {}));
/**
* This class represents a basic tracer provider which platform libraries can extend
*/
var TracerProvider = class {
	_resource;
	_activeSpanProcessor;
	_forceFlushTimeoutMillis;
	_tracerOptions;
	_tracers = /* @__PURE__ */ new Map();
	constructor(options = {}) {
		this._forceFlushTimeoutMillis = options.forceFlushTimeoutMillis ?? 3e4;
		this._resource = options.resource ?? defaultResource();
		const spanProcessors = options.spanProcessors ?? [];
		this._activeSpanProcessor = new MultiSpanProcessor(spanProcessors);
		this._tracerOptions = {
			resource: this._resource,
			sampler: options.sampler ?? new ParentBasedSampler({ root: new AlwaysOnSampler() }),
			spanLimits: {
				attributeCountLimit: options.spanLimits?.attributeCountLimit ?? 128,
				attributeValueLengthLimit: options.spanLimits?.attributeValueLengthLimit ?? Infinity,
				eventCountLimit: options.spanLimits?.eventCountLimit ?? 128,
				linkCountLimit: options.spanLimits?.linkCountLimit ?? 128,
				attributePerEventCountLimit: options.spanLimits?.attributePerEventCountLimit ?? 128,
				attributePerLinkCountLimit: options.spanLimits?.attributePerLinkCountLimit ?? 128
			},
			idGenerator: options.idGenerator || new RandomIdGenerator(),
			spanProcessor: this._activeSpanProcessor,
			meterProvider: options.meterProvider ?? { getMeter() {
				return createNoopMeter();
			} }
		};
	}
	getTracer(name, version, options) {
		const key = `${name}@${version || ""}:${options?.schemaUrl || ""}`;
		if (!this._tracers.has(key)) this._tracers.set(key, new Tracer({
			name,
			version,
			schemaUrl: options?.schemaUrl
		}, this._tracerOptions));
		return this._tracers.get(key);
	}
	forceFlush(options) {
		const timeout = options?.timeoutMillis ?? this._forceFlushTimeoutMillis;
		const promises = this._activeSpanProcessor["_spanProcessors"].map((spanProcessor) => {
			return new Promise((resolve) => {
				let state;
				const timeoutInterval = setTimeout(() => {
					resolve(/* @__PURE__ */ new Error(`Span processor did not completed within timeout period of ${timeout} ms`));
					state = ForceFlushState.timeout;
				}, timeout);
				spanProcessor.forceFlush().then(() => {
					clearTimeout(timeoutInterval);
					if (state !== ForceFlushState.timeout) {
						state = ForceFlushState.resolved;
						resolve(state);
					}
				}).catch((error) => {
					clearTimeout(timeoutInterval);
					state = ForceFlushState.error;
					resolve(error);
				});
			});
		});
		return new Promise((resolve, reject) => {
			Promise.all(promises).then((results) => {
				const errors = results.filter((result) => result !== ForceFlushState.resolved);
				if (errors.length > 0) reject(errors);
				else resolve();
			}).catch((error) => reject([error]));
		});
	}
	shutdown() {
		return this._activeSpanProcessor.shutdown();
	}
	[inspectCustom](depth, options, inspect) {
		const processors = this._activeSpanProcessor["_spanProcessors"];
		return formatInspect("TracerProvider", {
			resource: { attributes: settledResourceAttributes(this._resource) },
			tracers: Array.from(this._tracers.keys()),
			spanProcessors: processors.map((p) => p.constructor?.name ?? "SpanProcessor")
		}, depth, options, inspect);
	}
};
init_esm();
/** The trailing hexadecimal digits of a trace identifier the decision reads, all random. */
var DECISION_DIGITS = 8;
/** Samples a ratio of root traces, children following their parent, and records the rest for tail sampling. */
var RatioSampler = class {
	/** The sampled ratio of root traces, from 0 to 1. */
	ratio;
	/** Sample a ratio of root traces. */
	constructor(ratio) {
		this.ratio = ratio;
	}
	/** Follow a parent's decision, or sample a root whose random bits fall below the ratio. */
	shouldSample(context, traceId) {
		const parent = trace.getSpanContext(context);
		if (parent !== void 0) return { decision: (parent.traceFlags & 1) === 1 ? SamplingDecision.RECORD_AND_SAMPLED : SamplingDecision.RECORD };
		return { decision: Number.parseInt(traceId.slice(-8), 16) < this.ratio * 16 ** DECISION_DIGITS ? SamplingDecision.RECORD_AND_SAMPLED : SamplingDecision.RECORD };
	}
	/** Describe the sampler. */
	toString() {
		return `RatioSampler{${this.ratio}}`;
	}
};
init_esm();
var __destackModule$31 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ef-ba8b-b65e3579f3d0",
	"name": "@destack/telemetry",
	"version": "2026.9.0"
} });
/** How long a local root runs before its trace counts as slow and is kept: a second. */
var SLOW_MILLISECONDS = 1e3;
/** The most unsampled local traces held at once: older ones are dropped beyond it. */
var HELD_TRACES = 1e3;
/** The unsampled traces dropped undecided because too many were held. */
var evicted = scope(__destackModule$31.package).metric.counter("telemetry.tail.evicted", {
	unit: "{trace}",
	description: "Unsampled traces dropped undecided because too many were held.",
	attributes: {}
});
/** Keeps unsampled local traces that failed or ran slow, and drops the rest when their root ends. */
var TailSampler = class {
	/** The processor receiving kept spans. */
	#spans;
	/** The processor receiving kept log records. */
	#logs;
	/** How long a local root runs before its trace is kept, in milliseconds. */
	#slowMilliseconds;
	/** The held traces, by trace identifier, oldest first. */
	#held = /* @__PURE__ */ new Map();
	/** The span processor: sampled spans pass, unsampled ones wait for their local root. */
	spans = {
		onStart: (span, context) => this.#start(span, context),
		onEnd: (span) => this.#end(span),
		forceFlush: () => this.#spans.forceFlush(),
		shutdown: () => this.#spans.shutdown()
	};
	/** The log record processor: warnings and sampled records pass, the rest wait with their trace. */
	logs = {
		onEmit: (record, context) => this.#emit(record, context),
		forceFlush: () => this.#logs.forceFlush(),
		shutdown: () => this.#logs.shutdown()
	};
	/** Pass kept spans and log records to downstream processors. */
	constructor(spans, logs, slowMilliseconds = SLOW_MILLISECONDS) {
		this.#spans = spans;
		this.#logs = logs;
		this.#slowMilliseconds = slowMilliseconds;
	}
	/** Start holding an unsampled local root's trace, and pass sampled spans on. */
	#start(span, context) {
		const { traceId, traceFlags } = span.spanContext();
		if ((traceFlags & TraceFlags.SAMPLED) !== 0) this.#spans.onStart(span, context);
		else if (!this.#held.has(traceId)) {
			this.#held.set(traceId, {
				spans: [],
				logs: [],
				isFailed: false
			});
			this.#evict();
		}
	}
	/** Pass a sampled span on, or hold an unsampled one and decide its trace when its root ends. */
	#end(span) {
		const { traceId, traceFlags } = span.spanContext();
		if ((traceFlags & TraceFlags.SAMPLED) !== 0) {
			this.#spans.onEnd(span);
			return;
		}
		const held = this.#held.get(traceId);
		if (held === void 0) return;
		held.spans.push(span);
		held.isFailed ||= span.status.code === SpanStatusCode.ERROR;
		if (span.parentSpanContext === void 0 || span.parentSpanContext.isRemote === true) {
			this.#held.delete(traceId);
			const milliseconds = span.duration[0] * 1e3 + span.duration[1] / 1e6;
			if (held.isFailed || milliseconds >= this.#slowMilliseconds) this.#keep(held);
		}
	}
	/** Pass warnings and records outside held traces on, and hold the rest with their trace. */
	#emit(record, context) {
		const span = record.spanContext ?? (context === void 0 ? void 0 : trace.getSpanContext(context));
		const isWarning = (record.severityNumber ?? 0) >= SeverityNumber.WARN;
		const held = span === void 0 ? void 0 : this.#held.get(span.traceId);
		if (isWarning || span === void 0 || (span.traceFlags & TraceFlags.SAMPLED) !== 0) this.#logs.onEmit(record, context);
		else if (held !== void 0) held.logs.push(record);
	}
	/** Pass a kept trace's spans on as sampled, then its log records. */
	#keep(held) {
		for (const span of held.spans) this.#spans.onEnd(sampled(span));
		for (const record of held.logs) this.#logs.onEmit(record);
	}
	/** Drop the oldest held traces beyond the bound, counting each. */
	#evict() {
		for (const traceId of this.#held.keys()) {
			if (this.#held.size <= HELD_TRACES) break;
			this.#held.delete(traceId);
			evicted.add(1);
		}
	}
};
/** View an ended span as sampled, so downstream processors export it. */
function sampled(span) {
	const context = {
		...span.spanContext(),
		traceFlags: span.spanContext().traceFlags | TraceFlags.SAMPLED
	};
	return Object.create(span, { spanContext: { value: () => context } });
}
/** How often metrics export: once a minute, the resolution monitors store points at. */
var METRIC_INTERVAL_MILLISECONDS = 6e4;
/** Exports every signal as OTLP/JSON through one delivery, such as HTTP to a monitor. */
var OtlpExporter = class OtlpExporter {
	/** Deliver one request. */
	#send;
	/** Report a failed delivery, which the SDK otherwise drops silently. */
	#report;
	/** The deliveries in flight. */
	#pending = /* @__PURE__ */ new Set();
	/** The log record exporter. */
	logs = {
		export: (records, done) => this.#export("logs", JsonLogsSerializer.serializeRequest(records), done),
		shutdown: () => this.#drain(),
		forceFlush: () => this.#drain()
	};
	/** The span exporter. */
	traces = {
		export: (spans, done) => this.#export("traces", JsonTraceSerializer.serializeRequest(spans), done),
		shutdown: () => this.#drain(),
		forceFlush: () => this.#drain()
	};
	/** The metric exporter, exporting deltas and histograms as base-2 exponential histograms. */
	metrics = {
		export: (metrics, done) => this.#export("metrics", JsonMetricsSerializer.serializeRequest(metrics), done),
		shutdown: () => this.#drain(),
		forceFlush: () => this.#drain(),
		selectAggregationTemporality: () => AggregationTemporality.DELTA,
		selectAggregation: (instrument) => instrument === InstrumentType.HISTOGRAM ? { type: AggregationType.EXPONENTIAL_HISTOGRAM } : { type: AggregationType.DEFAULT }
	};
	/** Export through a delivery, reporting its failures. */
	constructor(send, report) {
		this.#send = send;
		this.#report = report;
	}
	/** Export over OTLP/HTTP to an endpoint with a current authorization header. */
	static http(endpoint, authorization, report) {
		return new OtlpExporter(async (signal, body) => {
			const response = await fetch(`${endpoint}/v1/${signal}`, {
				method: "POST",
				headers: {
					"content-type": "application/json",
					authorization: authorization()
				},
				body
			});
			const text = await response.text();
			if (!response.ok) throw new Error(`otlp ${signal} export failed with ${response.status}: ${text}`);
			const partial = text === "" ? void 0 : JSON.parse(text).partialSuccess;
			if (partial?.errorMessage) throw new Error(`otlp ${signal} export partly rejected: ${partial.errorMessage}`);
		}, report);
	}
	/** Build telemetry options sampling an owner's traces and batching its signals through this exporter. */
	options(owner, options = {}) {
		const tail = new TailSampler(new BatchSpanProcessor({ exporter: this.traces }), new BatchLogRecordProcessor({ exporter: this.logs }));
		return {
			name: owner.name,
			version: owner.version,
			attributes: options.attributes ?? {},
			report: this.#report,
			traces: {
				sampler: new RatioSampler(options.ratio ?? 1),
				spanProcessors: [tail.spans]
			},
			logs: { processors: [tail.logs] },
			metrics: { readers: [new PeriodicExportingMetricReader({
				exporter: this.metrics,
				exportIntervalMillis: METRIC_INTERVAL_MILLISECONDS
			})] }
		};
	}
	/** Deliver one serialized request and complete the export with its outcome. */
	#export(signal, body, done) {
		if (body === void 0) {
			const error = /* @__PURE__ */ new Error(`otlp ${signal} request could not be serialized`);
			this.#report(error);
			done({
				code: ExportResultCode.FAILED,
				error
			});
			return;
		}
		const delivery = this.#send(signal, body).then(() => done({ code: ExportResultCode.SUCCESS }), (cause) => {
			const error = cause instanceof Error ? cause : new Error(String(cause));
			this.#report(error);
			done({
				code: ExportResultCode.FAILED,
				error
			});
		});
		this.#pending.add(delivery);
		delivery.finally(() => this.#pending.delete(delivery));
	}
	/** Wait for the deliveries in flight. */
	async #drain() {
		await Promise.all(this.#pending);
	}
};
/** The maximum age of verified identity and membership records. */
var CALLER_LIFETIME_MILLISECONDS = 6e4;
/** A workload identity authenticated within one deployment. */
var CallerDeployment = strictObject({
	/** The workload identity. */
	subject: Subject,
	/** The deployment. */
	id: identifier("deployment")
});
/** A permission a credential or delegation step keeps. */
var KeptPermission = PermissionReference.extend({
	/** The authority scope. */
	scope: string().min(1),
	/** The object restriction. */
	objectId: string().optional()
});
/** A verified caller in transit between hosts, runners and services. */
var CallerAuthentication = strictObject({
	/** The authority scope of a scoped credential. */
	scope: string().min(1).optional(),
	/** The credential's permission restrictions. */
	permissions: array(KeptPermission).optional(),
	/** The credential reference. */
	credential: strictObject({
		/** The credential kind. */
		kind: string().min(1),
		/** The credential identifier. */
		id: string().min(1)
	}),
	/** The receiving package. */
	audience: PackageId,
	/** The verification time, in Unix milliseconds. */
	verifiedAt: number(),
	/** The exclusive expiry, in Unix milliseconds. */
	expiresAt: number(),
	/** The represented identity. */
	subject: Subject,
	/** How strongly and how recently the subject authenticated. */
	assurance: AuthenticationAssurance.optional(),
	/** The identifiers the subject proved control of. */
	identifiers: array(VerifiedIdentifier).optional(),
	/** The verified principals and the subject sets the caller belongs to. */
	subjects: array(Subject),
	/** The acting principals in order, the last sending the request. */
	delegates: array(Delegate).optional(),
	/** The verified deployments of workload identities. */
	deployments: array(CallerDeployment).optional(),
	/** The trusted attributes access policies read. */
	attributes: record$1(string(), Attribute).optional()
});
/** The header carrying the caller a host forwards to a runner. */
var CALLER_HEADER = "x-destack-caller";
/** A verified caller. */
var Caller = class Caller {
	/** The host-verified authentication. */
	authentication;
	/** Create the caller from its verified authentication. */
	constructor(authentication) {
		this.authentication = structuredClone(authentication);
	}
	/** Read the caller a host forwarded with a request, or null for a request without one. */
	static forwarded(request) {
		const forwarded = request.headers.get(CALLER_HEADER);
		return forwarded === null ? null : new Caller(CallerAuthentication.parse(JSON.parse(forwarded)));
	}
	/** Forward the caller to a runner in a request's headers. */
	forward(headers) {
		headers.set(CALLER_HEADER, JSON.stringify(this.authentication));
	}
	/** The credential reference. */
	get credential() {
		return this.authentication.credential;
	}
	/** The time the caller's authentication lapses, in Unix milliseconds. */
	get lapsesAt() {
		const { expiresAt, verifiedAt } = this.authentication;
		return Math.min(expiresAt, verifiedAt + CALLER_LIFETIME_MILLISECONDS);
	}
	/** Hold a time within the caller's lifetime: the time itself, or just before the lapse once passed. */
	within(now) {
		return Math.min(now, this.lapsesAt - 1);
	}
	/** The retry identity of the subject and the sending principal. */
	get id() {
		const { delegates, subject } = this.authentication;
		const sending = delegates?.at(-1)?.subject ?? subject;
		return JSON.stringify([subjectKey(subject), subjectKey(sending)]);
	}
	/** Require current authentication for the audience and scope. */
	requireCurrent(audience, now, scope) {
		const authentication = this.authentication;
		if (authentication.audience !== audience || authentication.scope !== void 0 && authentication.scope !== scope || !Number.isFinite(authentication.verifiedAt) || !Number.isFinite(authentication.expiresAt) || authentication.verifiedAt > now + 5e3 || now >= this.lapsesAt) throw new ORPCError("UNAUTHORIZED", { message: "caller authentication is expired or has a different audience or scope" });
		if (!authentication.subjects.some((subject) => sameSubject(subject, authentication.subject))) throw new ORPCError("UNAUTHORIZED", { message: "caller subject is missing from verified identities" });
	}
	/** Build an access context for the audience after checking freshness. */
	context(audience, now = Date.now(), scope) {
		this.requireCurrent(audience, now, scope);
		const authentication = this.authentication;
		return {
			subject: authentication.subject,
			subjects: authentication.subjects,
			...authentication.assurance === void 0 ? {} : { assurance: authentication.assurance },
			...authentication.identifiers === void 0 ? {} : { identifiers: authentication.identifiers },
			session: SessionKey.of(authentication.credential),
			delegates: authentication.delegates,
			attributes: authentication.attributes ?? {},
			permissions: authentication.permissions,
			now
		};
	}
	/** Require identity claims within an issuer's authority. */
	requireAuthority(authority) {
		const authentication = this.authentication;
		const subjects = [authentication.subject, ...authentication.subjects];
		if (authority.kind === "space" && (authentication.identifiers?.length || authentication.delegates?.length || subjects.some((subject) => !principal.installation.is(subject) || subject.scope !== authority.spaceId))) throw new ORPCError("UNAUTHORIZED", { message: "token exceeds issuer authority" });
		const workloads = [...subjects, ...(authentication.delegates ?? []).map((delegate) => delegate.subject)].filter((subject) => principal.installation.is(subject));
		const deployments = authentication.deployments ?? [];
		for (const subject of workloads) if (deployments.filter((deployment) => sameSubject(deployment.subject, subject)).length !== 1) throw new ORPCError("UNAUTHORIZED", { message: "invalid workload token identity" });
		if (deployments.some((deployment) => !workloads.some((subject) => sameSubject(subject, deployment.subject)))) throw new ORPCError("UNAUTHORIZED", { message: "unexpected workload token identity" });
		const delegates = authentication.delegates ?? [];
		const chain = [authentication.subject, ...delegates.map((delegate) => delegate.subject)];
		if (delegates.some((delegate, position) => !isPrincipal(delegate.subject) || delegate.authority === "full" && (position !== 0 || !principal.user.is(delegate.subject))) || chain.some((entry, position) => chain.slice(position + 1).some((other) => sameSubject(entry, other)))) throw new ORPCError("UNAUTHORIZED", { message: "invalid token delegation" });
	}
};
/** A caller's session key in access decisions: its credential's kind and identifier. */
var SessionKey = {
	/** Build a credential's session key. */
	of(credential) {
		return `${credential.kind}:${credential.id}`;
	},
	/** Read the credential identifier of a session key of one kind, absent for another kind. */
	id(key, kind) {
		const prefix = `${kind}:`;
		return key?.startsWith(prefix) === true ? key.slice(prefix.length) : void 0;
	}
};
var __destackModule$30 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} });
/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
var DRAIN_MILLISECONDS = 1e4;
/** The address of the audit service of a workload's space. */
var AUDIT_ADDRESS = "@destack/audit";
/** The address of the monitor service of a workload's space, receiving its telemetry over OTLP. */
var MONITOR_ADDRESS = "@destack/monitor";
/** The address of the space service of a workload's holder, relaying the space's access. */
var SPACE_ADDRESS = "@destack/space";
/** The runner's log records. */
var { log } = scope(__destackModule$30.package);
/** A workload instance a host started, serving its package below its mount on any runtime. */
var WorkloadRunner = class WorkloadRunner {
	/** The host's start message. */
	start;
	/** The running instance. */
	instance;
	/** The package's one service. */
	#service;
	/** The running package with the mount that serves the service. */
	#packageId;
	/** The telemetry exporting to the space's monitor. */
	#telemetry;
	/** Hold a started instance and its telemetry. */
	constructor(start, instance, service, running, packageId) {
		this.start = start;
		this.instance = instance;
		this.#service = service;
		this.#packageId = packageId;
		this.#telemetry = running;
	}
	/** Start a workload as a host's start asks, with the runtime's telemetry and failure reporting. */
	static async start(runner, start, startTelemetry, report) {
		const bearer = () => `Bearer ${start.secret}`;
		const monitor = Egress.url(start.egress, MONITOR_ADDRESS);
		const running = await startTelemetry(OtlpExporter.http(monitor, bearer, report).options(runner.workload.package, {
			attributes: { "service.instance.id": start.instance },
			ratio: start.sampling
		}));
		try {
			const resources = await WorkloadRunner.#connect(runner, start);
			const instance = await WorkloadInstance.start(runner.workload, {
				resources,
				history: runner.history(Egress.url(start.egress, AUDIT_ADDRESS), start.secret),
				replicas: {
					scope: start.scope,
					source: runner.replicas(Egress.url(start.egress, SPACE_ADDRESS), start.secret)
				},
				service: () => WorkloadRunner.#serve(runner, start)
			});
			const [service, ...others] = instance.services;
			if (service === void 0 || others.length > 0) {
				await instance.close();
				const found = instance.services.length;
				throw new ORPCError("PRECONDITION_FAILED", { message: `a runner serves one service per package, found ${found}` });
			}
			log.info("workload.started", {
				workload: runner.workload.name,
				instance: start.instance
			});
			return new WorkloadRunner(start, instance, service, running, runner.workload.package.id);
		} catch (error) {
			await running.shutdown();
			throw error;
		}
	}
	/** Aborts once the instance shuts down. */
	get signal() {
		return this.instance.signal;
	}
	/** Serve a forwarded request below the package's mount and refuse any other. */
	fetch(request) {
		if (request.headers.get("authorization") !== `Bearer ${this.start.secret}`) return Promise.resolve(Response.json({
			code: "UNAUTHORIZED",
			message: "invalid host secret"
		}, { status: 401 }));
		const routed = ServiceMount.route(request);
		if (routed?.packageId !== this.#packageId) return Promise.resolve(new Response(null, { status: 404 }));
		return this.instance.fetch(this.#service, routed.request);
	}
	/** Request shutdown. */
	shutdown() {
		this.instance.shutdown();
	}
	/** Drain the instance, then export the telemetry left and stop it. */
	async close() {
		try {
			await this.instance.close();
		} finally {
			await this.#telemetry.shutdown();
		}
	}
	/** Close the runner at the end of its scope. */
	async [Symbol.asyncDispose]() {
		await this.close();
	}
	/** Connect each bound resource through its declaration's connector for the resource's provider. */
	static async #connect(runner, start) {
		const resources = new ResourceContext();
		for (const [name, binding] of Object.entries(start.bindings)) {
			const declaration = runner.resources[name];
			const connector = declaration?.connectors[binding.provider];
			if (declaration === void 0 || declaration.kind !== binding.kind || connector === void 0) throw new ORPCError("NOT_FOUND", { message: `no ${binding.kind} ${name} connecting to provider ${binding.provider}` });
			resources.bind(declaration, await connector.connect(binding, declaration));
		}
		return resources;
	}
	/** Serve the package's service to the callers the host forwards. */
	static #serve(runner, start) {
		const audience = runner.workload.package.id;
		return {
			audience,
			scope: start.scope,
			instance: start.instance,
			drainTimeout: DRAIN_MILLISECONDS,
			authenticate: async (request) => {
				const caller = Caller.forwarded(request);
				caller?.requireCurrent(audience, Date.now(), start.scope);
				return caller;
			},
			authorizeHost: async ({ access }) => {
				if (access.authentication === "host") throw new ORPCError("FORBIDDEN", { message: "a workload serves no host procedures" });
			}
		};
	}
};
/** An affected object. */
var AuditTarget = defineSchema(strictObject({
	/** The object type. */
	type: string().min(1),
	/** The object identifier. */
	id: string().min(1),
	/** The display name at the time of the event. */
	name: string().optional(),
	/** The object version. */
	version: string().optional()
}));
/** The authenticated identity captured when an action occurs. */
var AuditActor = defineSchema(discriminatedUnion("type", [
	strictObject({
		/** A verified subject acted. */
		type: literal("subject"),
		/** The principal that acted. */
		subject: Subject,
		/** The display name captured when recording. */
		name: string().optional()
	}),
	strictObject({
		/** The platform acted on its own. */
		type: literal("system"),
		/** The component that acted. */
		name: string().min(1)
	}),
	strictObject({ 
	/** An unauthenticated caller acted. */
type: literal("anonymous") })
]));
/** The host-supplied authority and origin of an event. */
var AuditContext = defineSchema(strictObject({
	/** The authenticated actor performing the action. */
	actor: AuditActor,
	/** The subject the actor acts for. */
	subject: Subject.optional(),
	/** The delegators, from the initiator to the immediate delegator. */
	delegation: array(AuditActor),
	/** The scope whose history receives the event. */
	scope: string().min(1),
	/** The package producing the event. */
	package: Package,
	/** The service producing the event. */
	service: string().min(1),
	/** The deployment executing the service. */
	deploymentId: identifier("deployment").optional(),
	/** The instance executing the service. */
	instanceId: identifier("instance").optional(),
	/** The host executing the service. */
	hostId: identifier("host").optional(),
	/** The device the caller authenticated on. */
	deviceId: identifier("device").optional(),
	/** The caller's session, without its credential. */
	sessionId: identifier("session").optional(),
	/** The caller's token, without its secret. */
	tokenId: identifier("token").optional(),
	/** The request the event belongs to. */
	requestId: string().min(1).optional(),
	/** The operation the event belongs to. */
	operationId: string().min(1).optional(),
	/** The event that caused this one. */
	causeId: identifier("audit-event").optional(),
	/** The trace the event belongs to, as 32 lowercase hexadecimal digits. */
	traceId: string().regex(/^[0-9a-f]{32}$/).optional(),
	/** The caller's network address, as the host saw it. */
	address: string().optional(),
	/** The caller's user agent, as the host saw it. */
	userAgent: string().optional()
}));
/** How an action ended. */
var AuditOutcome = defineSchema(_enum([
	"success",
	"failure",
	"denied",
	"cancelled"
]));
/** The observed result of an action. */
var AuditResult = defineSchema(discriminatedUnion("outcome", [strictObject({ 
/** The action succeeded. */
outcome: AuditOutcome.extract(["success"]) }), strictObject({
	/** The action failed, was denied or was cancelled. */
	outcome: AuditOutcome.exclude(["success"]),
	/** The stable code of the failure. */
	errorCode: string().min(1)
})]));
/** A recorded event. */
var AuditEvent = defineSchema(strictObject({
	/** The event identifier. */
	id: identifier("audit-event"),
	/** The attempt this result completes. */
	attemptId: identifier("audit-event").optional(),
	/** The action and the package release recording it. */
	action: AuditActionReference,
	/** A committed write, a read of data, or a refused call. */
	category: _enum([
		"activity",
		"access",
		"denial"
	]),
	/** The producer's timestamp, in UTC epoch milliseconds. */
	occurredAt: number().int().nonnegative(),
	/** The origin and authority of the event. */
	context: AuditContext,
	/** The named affected objects. */
	targets: record$1(string().min(1), AuditTarget),
	/** The details the action schema accepts. */
	details: json(),
	/** The attempt, or the result with its outcome. */
	result: union([strictObject({ 
	/** The event records an attempt. */
stage: literal("attempt") }), ...AuditResult.options.map((result) => result.extend({ 
	/** The event records a result. */
stage: literal("result") }))])
}));
/** Events delivered together, oldest first. */
var AuditBatch = defineSchema(strictObject({ 
/** The events, oldest first. */
events: array(AuditEvent).min(1).max(100) }));
/** The most events one page returns or one prune removes. */
var MAX_AUDIT_BATCH = 1e3;
/** The scope whose history a query reads. */
var AuditScope = defineSchema(string().min(1));
/** The position after an event in acceptance order. */
var AuditCursor = defineSchema(strictObject({
	/** The acceptance time of the last returned event, in UTC milliseconds. */
	recordedAt: number().int().nonnegative(),
	/** The last returned event. */
	id: identifier("audit-event")
}));
/** A history query. */
var AuditQuery = defineSchema(strictObject({
	/** The scope whose history the query reads. */
	scope: AuditScope,
	/** The action name the events carry. */
	action: AuditActionName.optional(),
	/** The package declaring the events' action. */
	packageId: PackageId.optional(),
	/** The actor the events record. */
	actor: AuditActor.optional(),
	/** An object the events name. */
	target: strictObject({
		type: string().min(1),
		id: string().min(1)
	}).optional(),
	/** The attempt the events are or complete. */
	attemptId: identifier("audit-event").optional(),
	/** The category of the events. */
	category: AuditEvent.shape.category.optional(),
	/** The outcome the results report. */
	outcome: AuditOutcome.optional(),
	/** Whether to return only attempts without a result. */
	unresolved: boolean().optional(),
	/** The inclusive earliest acceptance time, in UTC milliseconds. */
	from: number().int().nonnegative().optional(),
	/** The exclusive latest acceptance time, in UTC milliseconds. */
	before: number().int().nonnegative().optional(),
	/** The position after which the page continues. */
	cursor: AuditCursor.optional(),
	/** The maximum number of events in the page. */
	limit: number().int().min(1).max(MAX_AUDIT_BATCH)
}));
/** An event and its acceptance time. */
var AuditRecord = defineSchema(strictObject({
	/** The event. */
	event: AuditEvent,
	/** The time the history accepted the event, in UTC milliseconds. */
	recordedAt: number().int().nonnegative()
}));
defineSchema(strictObject({
	/** The records of the page, in acceptance order. */
	items: array(AuditRecord),
	/** The position after the last record, null when the history ends. */
	cursor: AuditCursor.nullable()
}));
/** A removal of a scope's events accepted before a time. */
var AuditPrune = defineSchema(strictObject({
	/** The scope whose history loses the events. */
	scope: AuditScope,
	/** The exclusive latest acceptance time, in UTC milliseconds. */
	before: number().int().nonnegative(),
	/** The most events the prune removes. */
	limit: number().int().min(1).max(MAX_AUDIT_BATCH)
}));
var __destackModule$29 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739d-8c3c-55f69a40525a",
	"name": "@destack/audit",
	"version": "2026.9.0"
} });
/** A procedure that records its call and checks the history permission in its handler. */
var procedure$1 = defineProcedure({
	authentication: "identity",
	permission: null,
	audit: false
});
/** The audit service definition. */
var auditService = defineService("audit", {
	ingest: procedure$1.route({
		method: "POST",
		path: "/audit/events"
	}).input(AuditBatch).output(strictObject({ events: number().int().nonnegative() })),
	export: procedure$1.route({
		method: "POST",
		path: "/audit/export"
	}).input(AuditQuery).output(eventIterator(AuditRecord)),
	prune: procedure$1.route({
		method: "POST",
		path: "/audit/prune"
	}).input(AuditPrune).output(strictObject({ events: number().int().nonnegative() }))
}, __destackModule$29);
/** Connect to a local or regional audit service. */
function createAuditClient(options) {
	return createClient(auditService, options);
}
var require_AbstractAsyncHooksContextManager = /* @__PURE__ */ __commonJSMin(((exports) => {
	Object.defineProperty(exports, "__esModule", { value: true });
	exports.AbstractAsyncHooksContextManager = void 0;
	var events_1 = __require("events");
	var ADD_LISTENER_METHODS = [
		"addListener",
		"on",
		"once",
		"prependListener",
		"prependOnceListener"
	];
	var AbstractAsyncHooksContextManager = class {
		/**
		* Binds a the certain context or the active one to the target function and then returns the target
		* @param context A context (span) to be bind to target
		* @param target a function or event emitter. When target or one of its callbacks is called,
		*  the provided context will be used as the active context for the duration of the call.
		*/
		bind(context, target) {
			if (target instanceof events_1.EventEmitter) return this._bindEventEmitter(context, target);
			if (typeof target === "function") return this._bindFunction(context, target);
			return target;
		}
		_bindFunction(context, target) {
			const manager = this;
			const contextWrapper = function(...args) {
				return manager.with(context, () => target.apply(this, args));
			};
			Object.defineProperty(contextWrapper, "length", {
				enumerable: false,
				configurable: true,
				writable: false,
				value: target.length
			});
			/**
			* It isn't possible to tell Typescript that contextWrapper is the same as T
			* so we forced to cast as any here.
			*/
			return contextWrapper;
		}
		/**
		* By default, EventEmitter call their callback with their context, which we do
		* not want, instead we will bind a specific context to all callbacks that
		* go through it.
		* @param context the context we want to bind
		* @param ee EventEmitter an instance of EventEmitter to patch
		*/
		_bindEventEmitter(context, ee) {
			if (this._getPatchMap(ee) !== void 0) return ee;
			this._createPatchMap(ee);
			ADD_LISTENER_METHODS.forEach((methodName) => {
				if (ee[methodName] === void 0) return;
				ee[methodName] = this._patchAddListener(ee, ee[methodName], context);
			});
			if (typeof ee.removeListener === "function") ee.removeListener = this._patchRemoveListener(ee, ee.removeListener);
			if (typeof ee.off === "function") ee.off = this._patchRemoveListener(ee, ee.off);
			if (typeof ee.removeAllListeners === "function") ee.removeAllListeners = this._patchRemoveAllListeners(ee, ee.removeAllListeners);
			return ee;
		}
		/**
		* Patch methods that remove a given listener so that we match the "patched"
		* version of that listener (the one that propagate context).
		* @param ee EventEmitter instance
		* @param original reference to the patched method
		*/
		_patchRemoveListener(ee, original) {
			const contextManager = this;
			return function(event, listener) {
				const events = contextManager._getPatchMap(ee)?.[event];
				if (events === void 0) return original.call(this, event, listener);
				const patchedListener = events.get(listener);
				return original.call(this, event, patchedListener || listener);
			};
		}
		/**
		* Patch methods that remove all listeners so we remove our
		* internal references for a given event.
		* @param ee EventEmitter instance
		* @param original reference to the patched method
		*/
		_patchRemoveAllListeners(ee, original) {
			const contextManager = this;
			return function(event) {
				const map = contextManager._getPatchMap(ee);
				if (map !== void 0) {
					if (arguments.length === 0) contextManager._createPatchMap(ee);
					else if (map[event] !== void 0) delete map[event];
				}
				return original.apply(this, arguments);
			};
		}
		/**
		* Patch methods on an event emitter instance that can add listeners so we
		* can force them to propagate a given context.
		* @param ee EventEmitter instance
		* @param original reference to the patched method
		* @param [context] context to propagate when calling listeners
		*/
		_patchAddListener(ee, original, context) {
			const contextManager = this;
			return function(event, listener) {
				/**
				* This check is required to prevent double-wrapping the listener.
				* The implementation for ee.once wraps the listener and calls ee.on.
				* Without this check, we would wrap that wrapped listener.
				* This causes an issue because ee.removeListener depends on the onceWrapper
				* to properly remove the listener. If we wrap their wrapper, we break
				* that detection.
				*/
				if (contextManager._wrapped) return original.call(this, event, listener);
				let map = contextManager._getPatchMap(ee);
				if (map === void 0) map = contextManager._createPatchMap(ee);
				let listeners = map[event];
				if (listeners === void 0) {
					listeners = /* @__PURE__ */ new WeakMap();
					map[event] = listeners;
				}
				const patchedListener = contextManager.bind(context, listener);
				listeners.set(listener, patchedListener);
				/**
				* See comment at the start of this function for the explanation of this property.
				*/
				contextManager._wrapped = true;
				try {
					return original.call(this, event, patchedListener);
				} finally {
					contextManager._wrapped = false;
				}
			};
		}
		_createPatchMap(ee) {
			const map = Object.create(null);
			ee[this._kOtListeners] = map;
			return map;
		}
		_getPatchMap(ee) {
			return ee[this._kOtListeners];
		}
		_kOtListeners = Symbol("OtListeners");
		_wrapped = false;
	};
	exports.AbstractAsyncHooksContextManager = AbstractAsyncHooksContextManager;
}));
var require_AsyncHooksContextManager = /* @__PURE__ */ __commonJSMin(((exports) => {
	Object.defineProperty(exports, "__esModule", { value: true });
	exports.AsyncHooksContextManager = void 0;
	var api_1 = (init_esm(), __toCommonJS(esm_exports));
	var asyncHooks = __require("async_hooks");
	var AbstractAsyncHooksContextManager_1 = require_AbstractAsyncHooksContextManager();
	/**
	* @deprecated Use AsyncLocalStorageContextManager instead.
	*/
	var AsyncHooksContextManager = class extends AbstractAsyncHooksContextManager_1.AbstractAsyncHooksContextManager {
		_asyncHook;
		_contexts = /* @__PURE__ */ new Map();
		_stack = [];
		constructor() {
			super();
			this._asyncHook = asyncHooks.createHook({
				init: this._init.bind(this),
				before: this._before.bind(this),
				after: this._after.bind(this),
				destroy: this._destroy.bind(this),
				promiseResolve: this._destroy.bind(this)
			});
		}
		active() {
			return this._stack[this._stack.length - 1] ?? api_1.ROOT_CONTEXT;
		}
		with(context, fn, thisArg, ...args) {
			this._enterContext(context);
			try {
				return fn.call(thisArg, ...args);
			} finally {
				this._exitContext();
			}
		}
		enable() {
			this._asyncHook.enable();
			return this;
		}
		disable() {
			this._asyncHook.disable();
			this._contexts.clear();
			this._stack = [];
			return this;
		}
		/**
		* Init hook will be called when userland create a async context, setting the
		* context as the current one if it exist.
		* @param uid id of the async context
		* @param type the resource type
		*/
		_init(uid, type) {
			if (type === "TIMERWRAP") return;
			const context = this._stack[this._stack.length - 1];
			if (context !== void 0) this._contexts.set(uid, context);
		}
		/**
		* Destroy hook will be called when a given context is no longer used so we can
		* remove its attached context.
		* @param uid uid of the async context
		*/
		_destroy(uid) {
			this._contexts.delete(uid);
		}
		/**
		* Before hook is called just before executing a async context.
		* @param uid uid of the async context
		*/
		_before(uid) {
			const context = this._contexts.get(uid);
			if (context !== void 0) this._enterContext(context);
		}
		/**
		* After hook is called just after completing the execution of a async context.
		*/
		_after() {
			this._exitContext();
		}
		/**
		* Set the given context as active
		*/
		_enterContext(context) {
			this._stack.push(context);
		}
		/**
		* Remove the context at the root of the stack
		*/
		_exitContext() {
			this._stack.pop();
		}
	};
	exports.AsyncHooksContextManager = AsyncHooksContextManager;
}));
var require_AsyncLocalStorageContextManager = /* @__PURE__ */ __commonJSMin(((exports) => {
	Object.defineProperty(exports, "__esModule", { value: true });
	exports.AsyncLocalStorageContextManager = void 0;
	var api_1 = (init_esm(), __toCommonJS(esm_exports));
	var async_hooks_1 = __require("async_hooks");
	var AbstractAsyncHooksContextManager_1 = require_AbstractAsyncHooksContextManager();
	/**
	* Wrapper around a token and _asyncLocalStorage to mirror the behavior of
	* a Node.js RunScope
	*
	* @internal not intended for direct public consumption. Will be removed once
	* withScope is available on all supported Node.js versions
	*/
	var DisposeOnceToken = class {
		_isDisposed = false;
		_previousContext;
		_asyncLocalStorage;
		constructor(previousContext, asyncLocalStorage) {
			this._previousContext = previousContext;
			this._asyncLocalStorage = asyncLocalStorage;
		}
		dispose() {
			if (this._isDisposed) return;
			this._asyncLocalStorage.enterWith(this._previousContext);
			this._isDisposed = true;
		}
	};
	var AsyncLocalStorageContextManager = class extends AbstractAsyncHooksContextManager_1.AbstractAsyncHooksContextManager {
		_asyncLocalStorage;
		constructor() {
			super();
			this._asyncLocalStorage = new async_hooks_1.AsyncLocalStorage();
		}
		active() {
			return this._asyncLocalStorage.getStore() ?? api_1.ROOT_CONTEXT;
		}
		with(context, fn, thisArg, ...args) {
			const cb = thisArg == null ? fn : fn.bind(thisArg);
			return this._asyncLocalStorage.run(context, cb, ...args);
		}
		enable() {
			return this;
		}
		disable() {
			this._asyncLocalStorage.disable();
			return this;
		}
		/**
		* Imperatively sets `context` as active for the current async execution chain
		* and operations spawned from it. Returns a {@link ContextManagementToken} whose `dispose()`
		* restores the previous context (see {@link ContextManager.attach}).
		*
		* On Node.js 25.9+, delegates to `AsyncLocalStorage.withScope()` which returns
		* a native `RunScope`. On older Node.js versions, falls back to `enterWith()` with
		* a manual token.
		*
		* **Caveat for async functions:** Both `withScope()` and `enterWith()` affect the
		* entire current async execution chain. If `attach()` is called inside an async
		* function before the first `await`, the context change will leak into the caller's
		* context and remain active there until something else restores it. Prefer `with()`
		* for async code.
		*
		* @experimental This API is experimental and may change in minor releases without prior notice.
		*/
		attach(context) {
			const withScope = this._asyncLocalStorage.withScope;
			if (withScope) return withScope.call(this._asyncLocalStorage, context);
			const previousContext = this.active();
			this._asyncLocalStorage.enterWith(context);
			return new DisposeOnceToken(previousContext, this._asyncLocalStorage);
		}
	};
	exports.AsyncLocalStorageContextManager = AsyncLocalStorageContextManager;
}));
var require_src = /* @__PURE__ */ __commonJSMin(((exports) => {
	Object.defineProperty(exports, "__esModule", { value: true });
	exports.AsyncLocalStorageContextManager = exports.AsyncHooksContextManager = void 0;
	var AsyncHooksContextManager_1 = require_AsyncHooksContextManager();
	Object.defineProperty(exports, "AsyncHooksContextManager", {
		enumerable: true,
		get: function() {
			return AsyncHooksContextManager_1.AsyncHooksContextManager;
		}
	});
	var AsyncLocalStorageContextManager_1 = require_AsyncLocalStorageContextManager();
	Object.defineProperty(exports, "AsyncLocalStorageContextManager", {
		enumerable: true,
		get: function() {
			return AsyncLocalStorageContextManager_1.AsyncLocalStorageContextManager;
		}
	});
}));
/** Generates trace identifiers starting with their creation millisecond, so lookups know the time. */
var TraceIdGenerator = class {
	/** Generate a 16-byte trace identifier: 6 bytes of Unix milliseconds, then 10 random bytes. */
	generateTraceId() {
		return `${Date.now().toString(16).padStart(12, "0")}${crypto.getRandomValues(/* @__PURE__ */ new Uint8Array(10)).toHex()}`;
	}
	/** Generate a random 8-byte span identifier. */
	generateSpanId() {
		return crypto.getRandomValues(/* @__PURE__ */ new Uint8Array(8)).toHex();
	}
};
init_esm();
/** Manage one application's trace, metric, and log providers. */
var Telemetry = class Telemetry {
	/** The HTTP trace context and baggage propagator. */
	propagator;
	/** The trace provider. */
	traces;
	/** The metric provider. */
	metrics;
	/** The structured log provider. */
	logs;
	/** Remove only registrations made by this instance. */
	unregister = [];
	/** The shared shutdown operation. */
	shutdownPromise;
	/** Construct providers without changing process globals. */
	constructor(options) {
		this.propagator = options.propagator ?? new CompositePropagator({ propagators: [new W3CTraceContextPropagator(), new W3CBaggagePropagator()] });
		const resource = resourceFromAttributes({
			...options.attributes,
			"service.name": options.name,
			"service.version": options.version
		});
		this.traces = new TracerProvider({
			idGenerator: new TraceIdGenerator(),
			...options.traces,
			resource
		});
		this.metrics = new MeterProvider({
			...options.metrics,
			resource
		});
		this.logs = new LoggerProvider({
			...options.logs,
			resource
		});
	}
	/** Attribute instrumentation to a package using this instance's providers. */
	scope(source) {
		return instrument(this.traces.getTracer(source.name, source.version), this.metrics.getMeter(source.name, source.version), this.logs.getLogger(source.name, source.version));
	}
	/** Register providers and the host's context manager once per application. */
	static async start(options, manager) {
		const telemetry = new Telemetry(options);
		try {
			const report = (message, ...details) => options.report(new Error([message, ...details.map(String)].join(" ")));
			const logger = {
				error: report,
				warn: report,
				info: () => {},
				debug: () => {},
				verbose: () => {}
			};
			telemetry.register(diag.setLogger(logger, DiagLogLevel.WARN), () => diag.disable());
			telemetry.register(context.setGlobalContextManager(manager), () => context.disable());
			manager.enable();
			telemetry.register(propagation.setGlobalPropagator(telemetry.propagator), () => propagation.disable());
			telemetry.register(trace.setGlobalTracerProvider(telemetry.traces), () => trace.disable());
			telemetry.register(metrics.setGlobalMeterProvider(telemetry.metrics), () => metrics.disable());
			telemetry.register(logs.setGlobalLoggerProvider(telemetry.logs) === telemetry.logs, () => logs.disable());
		} catch (error) {
			try {
				await telemetry.shutdown();
			} catch (cleanupError) {
				throw new AggregateError([error, cleanupError], "telemetry initialization failed");
			}
			throw error;
		}
		return telemetry;
	}
	/** Export buffered signals before a request or application finishes. */
	async flush() {
		if (this.shutdownPromise) throw new Error("telemetry is shut down");
		await complete$1([
			this.traces.forceFlush(),
			this.metrics.forceFlush(),
			this.logs.forceFlush()
		]);
	}
	/** Remove global registrations, export buffered signals, and stop providers. */
	shutdown() {
		if (this.shutdownPromise) return this.shutdownPromise;
		for (const unregister of this.unregister.splice(0).reverse()) unregister();
		this.shutdownPromise = complete$1([
			this.traces.shutdown(),
			this.metrics.shutdown(),
			this.logs.shutdown()
		]);
		return this.shutdownPromise;
	}
	/** Track a successful registration or fail without replacing another provider. */
	register(isRegistered, unregister) {
		if (!isRegistered) throw new Error("OpenTelemetry is already initialized in this application");
		this.unregister.push(unregister);
	}
};
/** Wait for every provider and report every failure. */
async function complete$1(operations) {
	const errors = (await Promise.allSettled(operations)).filter((result) => result.status === "rejected").map((result) => result.reason);
	if (errors.length > 0) throw new AggregateError(errors, "telemetry export failed");
}
var import_src = require_src();
/** Start host telemetry with asynchronous context propagation. */
function startTelemetry(options) {
	return Telemetry.start(options, new import_src.AsyncLocalStorageContextManager());
}
function _usingCtx() {
	var r = "function" == typeof SuppressedError ? SuppressedError : function(r, e) {
		var n = Error();
		return n.name = "SuppressedError", n.error = r, n.suppressed = e, n;
	}, e = {}, n = [];
	function using(r, e) {
		if (null != e) {
			if (Object(e) !== e) throw new TypeError("using declarations can only be used with objects, functions, null, or undefined.");
			if (r) var o = e[Symbol.asyncDispose || Symbol["for"]("Symbol.asyncDispose")];
			if (void 0 === o && (o = e[Symbol.dispose || Symbol["for"]("Symbol.dispose")], r)) var t = o;
			if ("function" != typeof o) throw new TypeError("Object is not disposable.");
			t && (o = function o() {
				try {
					t.call(e);
				} catch (r) {
					return Promise.reject(r);
				}
			}), n.push({
				v: e,
				d: o,
				a: r
			});
		} else r && n.push({
			d: e,
			a: r
		});
		return e;
	}
	return {
		e,
		u: using.bind(null, !1),
		a: using.bind(null, !0),
		d: function d() {
			var o, t = this.e, s = 0;
			function next() {
				for (; o = n.pop();) try {
					if (!o.a && 1 === s) return s = 0, n.push(o), Promise.resolve().then(next);
					if (o.d) {
						var r = o.d.call(o.v);
						if (o.a) return s |= 2, Promise.resolve(r).then(next, err);
					} else s |= 1;
				} catch (r) {
					return err(r);
				}
				if (1 === s) return t !== e ? Promise.reject(t) : Promise.resolve();
				if (t !== e) throw t;
			}
			function err(n) {
				return t = t !== e ? new r(n, t) : n, next();
			}
			return next();
		}
	};
}
/** Serve each endpoint until shutdown, then drain and close. */
async function serveProcess(options) {
	const listeners = [];
	const stopped = Promise.withResolvers();
	const stop = () => stopped.resolve();
	const shutdown = () => options.shutdown();
	options.signal.addEventListener("abort", stop, { once: true });
	process.on("SIGINT", shutdown);
	process.on("SIGTERM", shutdown);
	try {
		for (const endpoint of options.endpoints) listeners.push(serve({
			hostname: endpoint.hostname,
			port: endpoint.port,
			fetch: (request) => endpoint.fetch(request)
		}));
		await options.ready?.(listeners.map((listener) => listener.url));
		if (!options.signal.aborted) await stopped.promise;
	} finally {
		try {
			await options.close();
		} finally {
			try {
				await Promise.all(listeners.map((listener) => listener.stop(true)));
			} finally {
				options.signal.removeEventListener("abort", stop);
				process.off("SIGINT", shutdown);
				process.off("SIGTERM", shutdown);
			}
		}
	}
}
/** Run a workload as a host's first input line starts it, on a loopback port, until shutdown. */
async function runWorkload(runner, lines, ready) {
	try {
		var _usingCtx$1 = _usingCtx();
		const first = await lines.next();
		if (first.done === true) throw new ORPCError("BAD_REQUEST", { message: "the host sent no start" });
		const start = WorkloadStart.parse(JSON.parse(first.value));
		const workload = _usingCtx$1.a(await WorkloadRunner.start(runner, start, startTelemetry, (error) => process.stderr.write(`telemetry export failed: ${error.message}\n`)));
		await serveProcess({
			endpoints: [{
				hostname: "127.0.0.1",
				port: 0,
				fetch: (request) => workload.fetch(request)
			}],
			signal: workload.signal,
			shutdown: () => workload.shutdown(),
			close: () => workload.close(),
			ready: (addresses) => ready({ port: Number(addresses[0].port) })
		});
	} catch (_) {
		_usingCtx$1.e = _;
	} finally {
		await _usingCtx$1.d();
	}
}
/** One call of a method inside its transaction, served or predicted. */
var Call = class Call {
	/** The object type. */
	object;
	/** The method's name. */
	name;
	/** The method. */
	method;
	/** The scope containing the object. */
	scope;
	/** The scope and the scopes containing it, nearest first. */
	chain;
	/** The method's own input fields. */
	input;
	/** The target's or created object's identifier. */
	id;
	/** The target at the start of the call. */
	target;
	/** The transaction: the server's, or the client's over its replica. */
	database;
	/** The calling principal. */
	caller;
	/** The time of the call in UTC epoch milliseconds. */
	now;
	/** Whether a client predicts the call. */
	isPredicted;
	/** The server's authorization, absent in a prediction. */
	authorization;
	/** The object types served together. */
	objects;
	/** The value the prepare phase produced. */
	prepared;
	/** The idempotency key of the call's external work. */
	key;
	/** The client writing an ephemeral object. */
	client;
	/** Run another method in the call's transaction as its caller. */
	run;
	/** Hold one call's fields. */
	constructor(fields) {
		this.object = fields.object;
		this.name = fields.name;
		this.method = fields.method;
		this.scope = fields.scope;
		this.chain = fields.chain;
		this.input = fields.input;
		this.database = fields.database;
		this.now = fields.now;
		this.objects = fields.objects;
		this.isPredicted = fields.isPredicted;
		if (!fields.isPredicted && fields.authorization === void 0) throw new TypeError(`a served call of ${fields.name} needs an authorization`);
		if (fields.id !== void 0) this.id = fields.id;
		if (fields.target !== void 0) this.target = fields.target;
		if (fields.caller !== void 0) this.caller = fields.caller;
		if (fields.authorization !== void 0) this.authorization = fields.authorization;
		if (fields.prepared !== void 0) this.prepared = fields.prepared;
		if (fields.key !== void 0) this.key = fields.key;
		if (fields.client !== void 0) this.client = fields.client;
		if (fields.run !== void 0) this.run = fields.run;
	}
	/** Record a call of an object's method, against the release of the object's package. */
	static record(object, name, input) {
		return {
			method: `${object.name}.${name}`,
			input: record$1(string(), json()).parse(input),
			release: object.package.version
		};
	}
	/** Convert a recorded call's input of an earlier release to the object's release, dropping the fields a shape no longer declares. */
	static upgrade(object, name, call, shape) {
		const served = object.package.version;
		if (Version.compare(call.release, served) > 0) throw new ORPCError("BAD_REQUEST", { message: `${call.method} was made against release ${call.release} of ${object.name}, which is at ${served}` });
		const conversions = object.conversions(name);
		if (Version.between(Object.keys(conversions), call.release, served).length === 0) return call.input;
		const converted = Expression$1.upgrade(conversions, call.input, call.release, served);
		return shape === void 0 ? converted : Object.fromEntries(Object.entries(converted).filter(([field]) => field in shape));
	}
	/** Read the identifier of the object a method returned. */
	static resultId(result) {
		const id = result?.id;
		return typeof id === "string" ? id : void 0;
	}
	/** Call another object's method in this call's scope and transaction. */
	async invoke(object, name, input) {
		const method = object.methods[name];
		if (method === void 0) throw new TypeError(`object ${object.name} has no method ${name}`);
		else if (method.prepare !== void 0) throw new TypeError(`${object.name}.${name} does external work, which an invoked call cannot`);
		else if (object.storage !== this.object.storage) throw new TypeError(`${this.object.name}.${this.name} cannot invoke ${object.name}, stored apart`);
		else if (this.run === void 0) throw new TypeError(`${this.object.name}.${this.name} runs no invoked calls`);
		return this.run(object, name, input);
	}
	/** Copy the call with some fields changed. */
	with(changes) {
		return new Call({
			...this,
			...changes
		});
	}
	/** Read the server's authorization and refuse a prediction. */
	served() {
		if (this.isPredicted) throw new ORPCError("FORBIDDEN", { message: `${this.object.name} changes on the server` });
		return this.authorization;
	}
	/** Reference the object the call acts on, in the call's scope. */
	reference() {
		return this.object.reference(this.scope, this.id);
	}
	/** Read the parent the call's input names. */
	parent() {
		const declared = this.object.parent;
		const named = declared.object === "any" ? this.input.parent : this.input.parentId;
		if (named === void 0 || named === null) return;
		return declared.object === "any" ? ObjectReference.parse({
			...ParentReference.parse(named),
			scope: this.scope
		}) : declared.object.reference(this.scope, string().parse(named));
	}
	/** Read the parent columns the call's input writes. */
	parentColumns() {
		if (this.object.parent.object !== "any") return { parentId: this.input.parentId };
		const parent = this.parent();
		return parent === void 0 ? {
			parentPackageId: null,
			parentType: null,
			parentId: null
		} : {
			parentPackageId: parent.packageId,
			parentType: parent.type,
			parentId: parent.id
		};
	}
	/** Require the receive permission on a parent. */
	async requireReceiving(parent, reader) {
		const declared = this.object.parent;
		const authorizer = this.authorization.authorizer;
		if (declared.object === "any") {
			if (!authorizer.relation(this.object.policy, "parent").subjects.some((host) => host.packageId === parent.packageId && host.type === parent.type)) throw new ORPCError("BAD_REQUEST", { message: `${parent.type} takes no ${this.object.plural}` });
			const mapping = authorizer.mapping(parent);
			const table = mapping.table;
			const [held] = await this.authorization.database.select({ id: table[mapping.id] }).from(table).where(and(eq(table[mapping.id], parent.id), mapping.scope === void 0 ? void 0 : eq(table[mapping.scope], parent.scope)));
			if (held === void 0) throw new ORPCError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
		}
		const receive = authorizer.policy(parent).permission(declared.receive);
		if ((await this.authorization.check(receive, parent, reader)).isAllowed) return;
		const reading = (declared.object === "any" ? this.objects.find((object) => object.policy.is(parent)) : declared.object)?.reading;
		if (reading === void 0 || !(await this.authorization.check(reading, parent)).isAllowed) throw new ORPCError("NOT_FOUND", { message: `no ${parent.type} ${parent.id}` });
		await this.authorization.require(receive, parent);
	}
	/** Update the target's desired state at the loaded revision. */
	async revise(changes) {
		const target = this.target;
		const generation = this.object.isControlled ? { generation: target.generation + 1 } : {};
		return this.#write({
			...changes,
			...generation
		});
	}
	/** Update the target's observed state at the loaded revision and generation. */
	async observe(changes) {
		return this.#write(changes);
	}
	/** Delete the target at the loaded revision. */
	async remove() {
		const table = this.object.table;
		const target = this.target;
		if ((await this.database.delete(table).where(and(eq(table.id, target.id), eq(table.revision, target.revision))).returning({ id: table.id })).length === 0) throw new ORPCError("CONFLICT", { message: `${this.object.name} revision has changed` });
	}
	/** Write changes to the target at the loaded revision. */
	async #write(changes) {
		const table = this.object.table;
		const target = this.target;
		const [row] = await this.database.update(table).set({
			...changes,
			revision: target.revision + 1,
			updatedAt: this.now
		}).where(and(eq(table.id, target.id), eq(table.revision, target.revision))).returning();
		if (!row) throw new ORPCError("CONFLICT", { message: `${this.object.name} revision has changed` });
		return row;
	}
};
/** The operations inverses use on steps. */
var Step = {
	/** Record a call to the step object's method of a kind. */
	call(step, kind, input) {
		const found = Object.entries(step.object.methods).find(([, declared]) => declared.kind === kind);
		return found === void 0 ? void 0 : Step.record(step, found[0], input);
	},
	/** Record a call to a named method of the step's object. */
	record(step, name, input) {
		return Call.record(step.object, name, input);
	},
	/** Build the input naming the step's object. */
	target(step, id) {
		const { field } = step.object.route;
		return field === void 0 ? { id } : {
			[field]: step.input[field],
			id
		};
	},
	/** Decide whether two field values have the same JSON form. */
	same(left, right) {
		return canonicalize(left ?? null) === canonicalize(right ?? null);
	}
};
/** The default page limit. */
var DEFAULT_PAGE_LIMIT = 50;
/** The most records one page returns. */
var MAX_PAGE_LIMIT = 1e3;
strictObject({
	/** The cursor of the preceding page. */
	cursor: string().min(1).optional(),
	/** The most records the page returns, 50 when absent. */
	limit: number().int().min(1).max(MAX_PAGE_LIMIT).optional()
});
/** A page position within a collection scope. */
var Page = class {
	/** The most records the page returns. */
	limit;
	/** The exclusive position after the preceding page. */
	after;
	/** The collection and parent identifiers every cursor keeps. */
	scope;
	/** Create the page from a request. */
	constructor(input, scope, position) {
		this.scope = scope;
		this.limit = input.limit ?? DEFAULT_PAGE_LIMIT;
		if (!Number.isInteger(this.limit) || this.limit < 1 || this.limit > MAX_PAGE_LIMIT) throw new ORPCError("BAD_REQUEST", { message: `page limit must be between 1 and ${MAX_PAGE_LIMIT}` });
		if (input.cursor !== void 0) {
			let cursor;
			try {
				cursor = strictObject({
					scope: array(string()),
					after: json()
				}).parse(JSON.parse(input.cursor));
				this.after = position.parse(cursor.after);
			} catch {
				throw new ORPCError("BAD_REQUEST", { message: "invalid collection cursor" });
			}
			if (JSON.stringify(cursor.scope) !== JSON.stringify(scope)) throw new ORPCError("BAD_REQUEST", { message: "cursor belongs to another collection" });
		}
	}
	/** Return the page's records and a cursor when more exist. */
	result(rows, position) {
		const items = rows.slice(0, this.limit);
		const last = items.at(-1);
		return {
			items,
			cursor: rows.length > this.limit && last !== void 0 ? JSON.stringify({
				scope: this.scope,
				after: position(last)
			}) : null
		};
	}
};
/** Describe a page of records. */
function page(item) {
	return strictObject({
		/** The records. */
		items: array(item),
		/** The cursor, or null at the end. */
		cursor: string().min(1).nullable()
	});
}
/** Write a camel-case or kebab-case name in kebab case. */
function kebabCase(name) {
	return name.replace(/([a-z0-9])([A-Z])/g, "$1-$2").replace(/([A-Z])([A-Z][a-z])/g, "$1-$2").toLowerCase();
}
/** Write a camel-case or kebab-case name in snake case. */
function snakeCase(name) {
	return kebabCase(name).replaceAll("-", "_");
}
/** Write a kebab-case name in camel case. */
function camelCase(name) {
	return name.replace(/-([a-z0-9])/g, (_match, letter) => letter.toUpperCase());
}
/** Write a kebab-case name in Pascal case. */
function pascalCase(name) {
	const camel = camelCase(name);
	return camel.charAt(0).toUpperCase() + camel.slice(1);
}
/** The rows one call's caller may list, decided once. */
var Listing = class {
	/** The tables whose changes decide visibility, none. */
	watches = [];
	/** The name of the decisions, one per call. */
	key = "call";
	/** The call listing the objects. */
	#call;
	/** The object types the call lists, by table. */
	#objects;
	/** List objects as a call's caller may list them. */
	constructor(call) {
		this.#call = call;
		this.#objects = new ListedObjects(call.objects);
	}
	/** Match the rows the caller may list. */
	where(table) {
		const { object, permission } = this.#objects.listed(table);
		return this.#call.isPredicted ? sql`true` : this.#call.served().listable(object, permission);
	}
	/** Decide which rows of a table the caller may list, at once. */
	async admits(table, rows) {
		const { object, permission } = this.#objects.listed(table);
		return this.#call.isPredicted ? new Set(rows.keys()) : (await this.#call.served().admitRows(object, permission, this.#call.scope, rows)).held;
	}
	/** List an object type's guarded fields. */
	concealable(table) {
		return this.#objects.concealable(table);
	}
	/** List the guarded fields the caller may not read on each row. */
	async conceals(table, rows) {
		const { object } = this.#objects.listed(table);
		return this.#call.isPredicted ? rows.map(() => []) : (await this.#call.served().concealed(object, rows)).hidden;
	}
	/** Read no expiry. */
	async until() {}
	/** Decide nothing again. */
	async refresh() {}
	/** List no dependents. */
	async dependents() {
		return [];
	}
};
/** Object types by their tables. */
var ListedObjects = class {
	/** The object types, by table. */
	#objects;
	/** Index object types by their tables. */
	constructor(objects) {
		this.#objects = new Map(objects.map((object) => [object.table, object]));
	}
	/** Read the object type holding a table, absent for another table. */
	get(table) {
		return this.#objects.get(table);
	}
	/** Read a table's listed object type and listing permission. */
	listed(table) {
		const object = this.#objects.get(table);
		if (object?.listing === void 0) throw new TypeError(`no listed object lives in ${table[TABLE].name}`);
		return {
			object,
			permission: object.listing
		};
	}
	/** List an object type's guarded fields. */
	concealable(table) {
		return this.#objects.get(table)?.guarded ?? [];
	}
};
/** A final failure of a request. */
var Failure = strictObject({
	/** The error code, such as CONFLICT. */
	code: string(),
	/** The HTTP status. */
	status: number().int(),
	/** The readable message. */
	message: string(),
	/** The structured details. */
	data: json().optional()
});
/** The outcome of a request. */
var Outcome = union([strictObject({ 
/** The result. */
value: json() }), strictObject({ 
/** The failure. */
error: Failure })]);
/** The milliseconds in each unit a duration names. */
var UNIT_MILLISECONDS = {
	days: 864e5,
	hours: 36e5,
	minutes: 6e4,
	seconds: 1e3,
	milliseconds: 1
};
/** Measure, check and describe spans of time. */
var Duration = {
	/** The schema of a duration. */
	schema: strictObject({
		/** Whole or partial days. */
		days: number().nonnegative().optional(),
		/** Hours. */
		hours: number().nonnegative().optional(),
		/** Minutes. */
		minutes: number().nonnegative().optional(),
		/** Seconds. */
		seconds: number().nonnegative().optional(),
		/** Milliseconds. */
		milliseconds: number().nonnegative().optional()
	}),
	/** Measure a duration in milliseconds. */
	milliseconds(duration) {
		return Object.entries(duration).reduce((total, [unit, count]) => total + count * UNIT_MILLISECONDS[unit], 0);
	},
	/** Require a duration to name at least one unit. */
	require(duration, name) {
		const entries = Object.entries(duration);
		if (!(entries.length > 0 && entries.every(([unit, count]) => Object.hasOwn(UNIT_MILLISECONDS, unit) && Number.isFinite(count) && count >= 0))) throw new TypeError(`${name} is no duration: ${JSON.stringify(duration)}`);
	}
};
/** The fields of a query of one object type's rows. */
var QueryShape = {
	/** Values computed from each row's fields, read like fields. */
	compute: record$1(string().min(1), Expression$1.schema).optional(),
	/** The condition the rows meet, over their logged fields. */
	where: Condition.schema.optional(),
	/** How the rows sort, completed by their identifier. */
	order: Order.schema.optional(),
	/** The most rows held, per held row for an include. */
	limit: number().int().positive().optional(),
	/** The includes of the rows, by relation. */
	include: lazy(() => record$1(string(), ObjectInclude)).optional(),
	/** Aggregates of the rows, held instead of the rows. */
	aggregate: Aggregate.optional(),
	/** Which trashed rows of a recoverable type to hold, none by default. */
	deleted: _enum([
		"exclude",
		"include",
		"only"
	]).optional()
};
/** One aggregate group a list measures. */
var GroupSchema = strictObject({
	/** The group's values in their JSON form, by field. */
	group: record$1(string(), Scalar),
	/** The group's measures by name. */
	values: record$1(string(), Scalar)
});
/** What a list returns beside its rows. */
var ListedShape = {
	/** Each row's computed values, by row identifier. */
	computed: record$1(string(), record$1(string(), Scalar)).optional(),
	/** What each row includes, in its JSON form, by include name and row identifier. */
	included: record$1(string(), record$1(string(), json())).optional(),
	/** The groups of an aggregate query. */
	groups: array(GroupSchema).optional()
};
/** Rows an include adds, or their aggregates. */
var ObjectInclude = lazy(() => strictObject({
	via: string().min(1).optional(),
	...QueryShape
}));
/** The identifier an object client mints for itself. */
var ClientId = string().min(1).max(64);
/** A query of one object type's rows in the scope a replica follows. */
var ObjectQuery = strictObject({
	object: string().min(1),
	...QueryShape
});
/** The outcomes of a push and the watermark holding their changes. */
var PushResult = strictObject({
	/** Each mutation's outcome, in the order pushed. */
	outcomes: array(strictObject({
		/** The mutation's request identifier. */
		id: string(),
		/** The results of its calls in order, or its final failure. */
		outcome: Outcome
	})),
	/** The server log sequence holding every executed mutation's changes. */
	watermark: Watermark
});
/** The procedures a client replica or a database copying the served one uses. */
var replicaProcedures = {
	push: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/replica/push"
	}).input(strictObject({
		/** The scope the mutations change. */
		scope: string().min(1),
		/** The mutations, in the order the client committed them. */
		mutations: array(Mutation).min(1).max(100),
		/** The client writing ephemeral objects. */
		client: ClientId.optional()
	})).output(PushResult),
	sync: defineProcedure({
		authentication: "public",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/replica/sync"
	}).input(strictObject({
		/** The scope to follow. */
		scope: string().min(1),
		/** The queries to follow by name, every listed object type when absent. */
		queries: record$1(string(), ObjectQuery).optional(),
		/** The queries the subscriber followed before. */
		previous: record$1(string(), ObjectQuery).optional(),
		/** The log position the subscriber holds, absent before its first snapshot. */
		after: LogPosition.optional(),
		/** How often merged pages arrive. */
		refresh: strictObject({ every: Duration.schema }).optional(),
		/** The client following ephemeral objects, absent for durable ones. */
		client: ClientId.optional()
	})).output(eventIterator(QueryPage)),
	call: defineProcedure({
		authentication: "public",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/replica/call"
	}).input(strictObject({
		/** The scope the call reads. */
		scope: string().min(1),
		/** The call of a reading method. */
		call: Call$1
	})).output(json()),
	broadcast: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/replica/broadcast"
	}).input(strictObject({
		/** The scope holding the object. */
		scope: string().min(1),
		/** The object whose readers receive the event. */
		object: ObjectReference.omit({ scope: true }),
		/** The unstored event. */
		event: json()
	})).output(strictObject({})),
	stream: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/replica/stream"
	}).input(ReplicaRequest).output(eventIterator(QueryPage))
};
/** Immutable versions of a parent, numbered within it. */
var versioned = {
	key: "versioned",
	isDurable: true,
	options: (definition) => definition.versioned,
	columns: () => ({ number: integer("number").notNull() }),
	constraints: (_options, table, columns) => [unique(`${table}_number`).on(columns.parentId, columns.number)],
	methods: () => ({}),
	validate: (_options, object, definition) => {
		if (!definition.nested || definition.nested.in === "any") throw new TypeError(`versions of ${object.name} need a parent of one type`);
		if (definition.methods && "update" in definition.methods) throw new TypeError(`versions of ${object.name} are immutable and cannot be updated`);
	},
	async next(object, parentId, database) {
		const table = object.table;
		const [latest] = await database.select({ number: table.number }).from(table).where(eq(table.parentId, parentId)).orderBy(desc(table.number)).limit(1);
		return latest === void 0 ? 1 : latest.number + 1;
	}
};
/** The schema of the stack declaration managing a record. */
var ManagerSchema = defineSchema(strictObject({
	/** The installation that applied the declaration. */
	installationId: identifier("installation"),
	/** The immutable identity of the declaring package. */
	packageId: PackageId,
	/** The declaration's path within the package, such as installations/notes. */
	name: string().regex(/^[a-z][a-z0-9-]*(?:\/[a-z][a-z0-9-]*)*$/)
}));
/** The stack declaration managing a record. */
var Manager = {
	/** The schema of a manager. */
	schema: ManagerSchema,
	/** Read a record's manager, or null for records managed at runtime. */
	read(row) {
		return row.managerInstallationId === null ? null : ManagerSchema.parse({
			installationId: row.managerInstallationId,
			packageId: row.managerPackageId,
			name: row.managerName
		});
	},
	/** Decide whether a declaration still manages a record. */
	isManaging(row) {
		return row?.managerInstallationId !== void 0 && row.managerInstallationId !== null && row.detachedAt === null;
	},
	/** Write a record's manager into its columns. */
	values(manager) {
		return {
			managerInstallationId: manager?.installationId ?? null,
			managerPackageId: manager?.packageId ?? null,
			managerName: manager?.name ?? null
		};
	}
};
/** Declare the columns naming a record's managing declaration. */
function managedColumns() {
	return {
		/** The installation that applied the declaration. */
		managerInstallationId: identifier$1("manager_installation_id", "installation"),
		/** The declaring package. */
		managerPackageId: identifier$1("manager_package_id", "package"),
		/** The declaration's name within the declaring package. */
		managerName: text$1("manager_name"),
		/** The time the declaration stopped managing the record, null while managed. */
		detachedAt: integer("detached_at")
	};
}
/** Records that stacks declare, managed by their declaration until detached. */
var declarable = {
	key: "declarable",
	isDurable: true,
	options: (definition) => definition.declarable,
	columns: () => managedColumns(),
	constraints: (_options, table, columns) => managerChecks(table, columns),
	methods: () => ({})
};
/** Declared records callers detach from their declaration. */
var detachable = {
	key: "detachable",
	isDurable: true,
	options: (definition) => definition.detachable,
	columns: () => ({}),
	constraints: () => [],
	methods: (options) => ({ detach: detach(options.by) }),
	validate: (_options, object, definition) => {
		if (definition.declarable === void 0) throw new TypeError(`object ${object.name} is detachable but not declarable`);
	},
	detach: (permission) => detach(permission)
};
/** Detach an object from its stack declaration. */
function detach(permission) {
	return defineMethod({
		kind: "detach",
		permission,
		mutates: true,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/{id}/detach"
			},
			input: shapes.target.extend({
				...shapes.replay,
				...RevisionShape
			}),
			output: shapes.row
		}),
		effect: (call) => call.revise({ detachedAt: call.now }),
		async execute(call) {
			if (!Manager.isManaging(call.target)) throw new ORPCError("CONFLICT", { message: `${call.object.name} is not managed by a declaration` });
			return this.effect(call);
		}
	});
}
/** User-defined labels indexed by name. */
var TagMap = record$1(string().min(1).max(128), string().max(256));
/** Declare a label map with an empty database default. */
function tags(name = "tags") {
	return json$1(name, TagMap).notNull().default(sql`'{}'`);
}
/** Declare the columns every record holds. */
function recordColumns(prefix) {
	return {
		/** The immutable record identifier. */
		id: identifier$1("id", prefix).primaryKey(),
		/** Creation time in UTC epoch milliseconds. */
		createdAt: integer("created_at").notNull(),
		/** Last modification time in UTC epoch milliseconds. */
		updatedAt: integer("updated_at").notNull(),
		/** The revision used by conditional updates. */
		revision: integer("revision").notNull().default(1),
		/** User-defined labels. */
		tags: tags()
	};
}
/** Declare the column naming an ephemeral object's writing client. */
function clientColumns() {
	return { 
	/** The identifier of the client that wrote the object. */
client: text$1("client").notNull() };
}
/** The record trait every object takes. */
var record = {
	options: () => true,
	columns: (_options, object) => ({
		...recordColumns(object.identity),
		...object.storage === "ephemeral" ? clientColumns() : {}
	}),
	constraints: () => [],
	table: (_options, object) => object.storage === "ephemeral" ? {} : { dependents: [{
		from: () => accessRelationship,
		key: "objectId",
		where: {
			packageId: object.packageId,
			type: object.name
		},
		onDelete: "cascade"
	}] },
	methods: () => ({})
};
/** Read one object. */
function get(permission) {
	return defineMethod({
		kind: "get",
		permission,
		mutates: false,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "GET",
				path: "/{id}"
			},
			input: shapes.target,
			output: shapes.row
		}),
		effect: async (call) => call.target
	});
}
/** Read a page of a scope's objects or aggregate groups. */
function list(permission) {
	return defineMethod({
		kind: "list",
		permission,
		mutates: false,
		target: false,
		result: "page",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/query"
			},
			input: shapes.scope.extend({
				...QueryShape,
				...PageShape
			}),
			output: page(shapes.row).extend(ListedShape)
		}),
		effect: listObjects
	});
}
/** Create an object from the fields the caller writes. */
function create(permission, options = {}) {
	const { creator, creation: declaredCreation, ...declared } = options;
	if (creator !== void 0 && declaredCreation !== void 0) throw new TypeError("a creation names a creator relation or a creation, not both");
	const creation = creator === void 0 ? declaredCreation : relateCreator(creator);
	return defineMethod({
		kind: "create",
		permission,
		mutates: true,
		...declared,
		target: false,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: ""
			},
			input: shapes.scope.extend({
				...shapes.created,
				...shapes.parent,
				...shapes.replay,
				...shapes.written(options.fields, false),
				...fields(options.input)
			}),
			output: shapes.row
		}),
		effect: createObject,
		authorize: async (call) => {
			if (call.method.permission === null) return;
			const reader = await call.authorization.reader(call.scope);
			const parent = call.object.parent && call.parent();
			if (parent) await call.requireReceiving(parent, reader);
			await requireCreatable(call, reader, creation);
		},
		inverse: (step) => {
			const call = Step.call(step, "delete", Step.target(step, step.after?.id));
			return call === void 0 ? void 0 : [call];
		},
		async execute(call) {
			if (call.isPredicted) {
				if (call.id === void 0) throw new TypeError(`predicted ${call.object.name} creations need an identifier`);
				return this.effect(call);
			}
			const authorization = call.served();
			const { created, row } = await insertCreated(call, (inserted) => this.effect(inserted));
			const id = Call.resultId(row);
			const object = call.object.reference(call.scope, id);
			if (call.object.policy.definition.scope === true || creation !== void 0) await authorization.create(object, await creation?.(created, object) ?? {});
			await authorization.requireWritable(created, id);
			return row;
		}
	});
}
/** Update an object's written fields. */
function update(permission, options = {}) {
	return defineMethod({
		kind: "update",
		permission,
		mutates: true,
		...options,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "PATCH",
				path: "/{id}"
			},
			input: shapes.target.extend({
				...shapes.replay,
				...RevisionShape,
				...shapes.written(options.fields, true),
				...fields(options.input)
			}),
			output: shapes.row
		}),
		effect: (call) => call.revise(decodeRow(call.object.table, call.input)),
		inverse: (step) => {
			const current = step.current;
			if (current === void 0) return;
			const restored = Object.fromEntries(step.object.written.filter((name) => Object.hasOwn(step.input, name)).filter((name) => Step.same(current[name], step.after?.[name])).map((name) => [name, step.before?.[name] ?? null]));
			if (Object.keys(restored).length === 0) return;
			return [Step.record(step, step.name, {
				...Step.target(step, step.input.id),
				...restored
			})];
		},
		async execute(call) {
			requireUnmanaged(call);
			if (!call.isPredicted) await call.served().requireWritable(call, call.id);
			return this.effect(call);
		}
	});
}
/** Update every matching object the caller may change, returning the count. */
function updateMany(permission, options) {
	return defineMethod({
		kind: "updateMany",
		permission,
		mutates: true,
		...options.convert === void 0 ? {} : { convert: options.convert },
		target: false,
		result: "value",
		procedure: (_name, shapes) => ({
			route: {
				method: "PATCH",
				path: ""
			},
			input: shapes.scope.extend({
				...shapes.replay,
				where: strictObject(shapes.written(options.match, true)),
				...shapes.written(options.fields, true)
			}),
			output: strictObject({ count: number().int().nonnegative() })
		}),
		validate: (object) => {
			if (object.tracked !== void 0 || object.declarationSchema !== void 0 || object.versioned) throw new TypeError(`object ${object.name} keeps a record of each change, which many updates at once skip`);
			const guarded = options.fields.find((name) => object.fields[name]?.access?.write !== void 0);
			if (guarded !== void 0) throw new TypeError(`object ${object.name} guards field ${guarded}, which many updates at once skip`);
		},
		effect: async (call) => {
			const table = call.object.table;
			const input = call.input;
			const values = decodeRow(table, Object.fromEntries(options.fields.flatMap((name) => Object.hasOwn(input, name) ? [[name, input[name]]] : [])));
			const matched = decodeRow(table, input.where);
			const hasGeneration = Object.hasOwn(table[TABLE].columns, "generation");
			const matching = and(call.object.inScope(call.scope), ...Object.entries(matched).map(([name, value]) => value === null ? isNull(table[name]) : eq(table[name], value)));
			let held;
			if (!call.isPredicted) {
				const served = call.served();
				const permission = call.object.permission(call.method.permission);
				const listable = served.listable(call.object, permission);
				if (listable === "memory") {
					const rows = await call.database.select().from(table).where(matching);
					const kept = await served.keep(rows, permission);
					held = inArray(table.id, kept.map((row) => row.id));
				} else held = listable;
			}
			return { count: (await call.database.update(table).set({
				...values,
				revision: sql`${table.revision} + 1`,
				...hasGeneration ? { generation: sql`${table.generation} + 1` } : {},
				updatedAt: call.now
			}).where(and(matching, held)).returning({ id: table.id })).length };
		}
	});
}
/** Delete an object. */
function remove(permission, options = {}) {
	return defineMethod({
		kind: "delete",
		permission,
		mutates: true,
		...options,
		target: true,
		result: "value",
		procedure: (_name, shapes) => ({
			route: {
				method: "DELETE",
				path: "/{id}"
			},
			input: shapes.target.extend({
				...shapes.replay,
				...RevisionShape
			}),
			output: Empty
		}),
		effect: deleteObject,
		inverse: (step) => {
			const call = step.object.recoverable === void 0 ? void 0 : Step.call(step, "restore", Step.target(step, step.input.id));
			return call === void 0 ? void 0 : [call];
		},
		async execute(call) {
			requireUnmanaged(call);
			return this.effect(call);
		}
	});
}
/** Declare a custom method on one object, mutating by default. */
function custom(definition) {
	const mutates = definition.mutates ?? true;
	const { inverse, ...declared } = definition;
	return defineMethod({
		kind: "custom",
		...declared,
		...inverse === void 0 ? {} : { inverse: typeof inverse === "string" ? (step) => [Step.record(step, inverse, step.input)] : inverse },
		mutates,
		isPredicted: false,
		target: true,
		result: definition.output === void 0 ? "object" : "value",
		procedure: (name, shapes) => ({
			route: {
				method: "POST",
				path: `/{id}/${kebabCase(name)}`
			},
			input: shapes.target.extend({
				...mutates ? shapes.replay : {},
				...fields(definition.input)
			}),
			output: definition.output ?? shapes.row
		}),
		effect: missingHandler
	});
}
/** Read a method's own input fields. */
function fields(input) {
	return input?.shape ?? {};
}
/** Insert a created object. */
async function createObject(call) {
	const table = call.object.table;
	const [row] = await call.database.insert(table).values(await createdValues(call)).returning();
	return row;
}
/** Build the columns a created object is inserted with. */
async function createdValues(call) {
	const { object, caller } = call;
	const isSystem = call.method.isSystem === true;
	const callers = Object.entries(object.fields).filter(([, declared]) => declared.isCaller && !isSystem);
	for (const [, declared] of callers) {
		const kinds = declared.principals;
		if (caller === void 0 || !(kinds === void 0 ? isPrincipal(caller) : kinds.some((kind) => kind.is(caller)))) {
			const named = kinds?.map((kind) => kind.name).join(" or ") ?? "principal";
			throw new ORPCError("FORBIDDEN", { message: `${object.name} is created by a ${named}` });
		}
	}
	const version = object.versioned ? await versioned.next(object, call.parent().id, call.database) : void 0;
	if (object.storage === "ephemeral" && call.client === void 0) throw new TypeError(`ephemeral ${object.name} objects are created by a client`);
	const table = object.table;
	return {
		...decodeRow(table, call.input),
		...object.parent === void 0 ? {} : call.parentColumns(),
		id: call.id,
		scope: call.scope,
		...version === void 0 ? {} : { number: version },
		...object.storage === "ephemeral" ? { client: call.client } : {},
		...Object.fromEntries(callers.map(([name, declared]) => [name, declared.type === "subject" ? subjectKey(caller) : caller.id])),
		createdAt: call.now,
		updatedAt: call.now
	};
}
/** Insert a created object under an unused identifier within a savepoint. */
async function insertCreated(call, effect) {
	if (call.isPredicted) {
		const created = call.with({ id: await createdId(call) });
		return {
			created,
			row: await effect(created)
		};
	}
	try {
		return await call.database.transaction(async (transaction) => {
			const inserting = call.with({ database: transaction });
			const created = inserting.with({ id: await createdId(inserting) });
			const row = await effect(created);
			return {
				created: created.with({ database: call.database }),
				row
			};
		});
	} catch (error) {
		if ((error instanceof ORPCError && error.code === "CONFLICT" || error instanceof DatabaseError && (error.code === "DUPLICATE" || error.code === "BROKEN_REFERENCE")) && call.method.permission !== null) await requireCreatable(call);
		throw error;
	}
}
/** Require the create permission on the object as written. */
async function requireCreatable(call, reader, creation) {
	const columns = call.object.table[TABLE].columns;
	const values = await createdValues(call.with({ id: call.id ?? `${call.object.identity}-${v7()}` }));
	const defaults = Object.entries(columns).flatMap(([name, column]) => {
		const value = column.definition.default;
		const isExpression = typeof value === "object" && value !== null && "getSQL" in value;
		return Object.hasOwn(values, name) || value === void 0 || isExpression ? [] : [[name, value]];
	});
	const row = {
		...Object.fromEntries(defaults),
		...values
	};
	const authorization = call.authorization;
	const { authorizer, snapshot } = authorization;
	const grants = reader ?? await authorization.reader(call.scope);
	const created = call.object.reference(call.scope, String(row.id));
	const initial = await creation?.(call.with({ id: String(row.id) }), created) ?? {};
	grants.creating(created, authorizer.initialRelationships(created, initial, call.now));
	const permission = call.object.permission(call.method.permission);
	const admits = async (access) => (await authorizer.checkRows(snapshot, permission, access, [row], grants)).held.has(0);
	const access = await authorization.in(call.scope);
	if (await admits(access)) return;
	const stepUp = await authorizer.challenge(snapshot, permission, access, admits);
	if (stepUp !== void 0) throw new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again at the required assurance", { stepUp });
	const reading = call.object.reading;
	const isReadable = reading !== void 0 && (await authorizer.checkRows(snapshot, reading, access, [row])).held.has(0);
	const denial = new ORPCError("FORBIDDEN", { message: `permission denied: ${permission.name}` });
	throw isReadable ? denial : conceal(denial, `${call.object.name} not found`);
}
/** Delete an object, or request deletion when a trash or controller finishes it. */
async function deleteObject(call) {
	const { object } = call;
	const table = object.table;
	const target = call.target;
	if (target.deletionRequestedAt !== void 0 && target.deletionRequestedAt !== null) throw new ORPCError("CONFLICT", { message: `${object.name} is already deleted` });
	if (Object.hasOwn(table[TABLE].columns, "deletionRequestedAt")) {
		await call.revise({ deletionRequestedAt: call.now });
		return {};
	}
	await call.remove();
	return {};
}
/** Accept a caller's unused identifier for a created object, or mint one. */
async function createdId(call) {
	const { object } = call;
	const chosen = call.id;
	if (chosen === void 0) return `${object.identity}-${v7()}`;
	const table = object.table;
	const { packageId, name } = object.policy.definition;
	const related = sql`EXISTS (
        SELECT 1 FROM ${accessRelationship}
        WHERE ${and(eq(accessRelationship.packageId, packageId), eq(accessRelationship.type, name), eq(accessRelationship.objectId, chosen))}
    )`;
	const held = sql`EXISTS (SELECT 1 FROM ${table} WHERE ${eq(table.id, chosen)})`;
	const isTaken = async (database, exists) => {
		const [row] = await database.execute(sql`SELECT ${exists} AS "isTaken"`);
		return Boolean(row.isTaken);
	};
	if (object.storage === "durable" ? await isTaken(call.database, sql`(${held} OR ${related})`) : await isTaken(call.database, held) || await isTaken(call.authorization.database, related)) throw new ORPCError("CONFLICT", { message: `${object.name} identifier is taken` });
	return chosen;
}
/** Read a page of a query's rows or its aggregate groups. */
async function listObjects(call) {
	const { cursor, limit, ...shape } = call.input;
	const listing = new Page({
		...cursor === void 0 ? {} : { cursor },
		...limit === void 0 ? {} : { limit }
	}, [
		call.object.name,
		call.scope,
		canonicalize(shape)
	], record$1(string(), json()));
	const dataflow = new Dataflow({ list: {
		...call.object.query(shape.aggregate === void 0 ? {
			...shape,
			limit: listing.limit + 1
		} : shape, call.objects),
		scopes: call.object.scopesOf(call.chain)
	} }, {
		audience: new Listing(call),
		database: call.database,
		changesThrough: changesThroughLog(call.database),
		isMaterialized: true
	});
	const node = dataflow.roots[0];
	const after = listing.after === void 0 ? void 0 : decodeRow(node.table, listing.after);
	await dataflow.fill(await View$1.latest(call.database), after);
	if (node.aggregate !== void 0) return {
		items: [],
		cursor: null,
		groups: await dataflow.read("list")
	};
	const rows = await dataflow.read("list");
	const ordered = node.order.flatMap((key) => node.columnsOf(key.column));
	const listed = listing.result(rows, (row) => encodeRow(node.table, Object.fromEntries(ordered.map((name) => [name, row[name]]))));
	const names = node.children.filter((child) => child.kind === "include").map((child) => child.name.slice(node.name.length + 1));
	const computed = Object.keys(node.computed);
	const values = Object.fromEntries(listed.items.map((row) => [String(row.id), Object.fromEntries(computed.map((name) => [name, node.json(name, row[name])]))]));
	const included = Object.fromEntries(names.map((name) => [name, Object.fromEntries(listed.items.map((row) => [String(row.id), json().parse(row[name])]))]));
	const items = listed.items.map((row) => Object.fromEntries(Object.entries(row).filter(([name]) => !names.includes(name) && !computed.includes(name))));
	return {
		...listed,
		items,
		...names.length === 0 ? {} : { included },
		...computed.length === 0 ? {} : { computed: values }
	};
}
/** Refuse standard changes to managed objects. */
function requireUnmanaged(call) {
	if (Manager.isManaging(call.target)) throw new ORPCError("MANAGED", {
		status: 409,
		message: `${call.object.name} is managed by its stack; detach it before changing it`
	});
}
/** Reject a method without a handler. */
function missingHandler(call) {
	throw new TypeError(`object ${call.object.name} implements no handler for ${call.name}`);
}
/** Relate the calling principal to a new object. */
function relateCreator(relation) {
	return (call, object) => {
		if (call.caller === void 0) throw new ORPCError("FORBIDDEN", { message: `${call.object.name} needs a creator` });
		const isScope = call.object.policy.definition.scope === true;
		return {
			relationships: [{
				relation,
				subject: call.caller
			}],
			...isScope ? { owner: {
				...object,
				relation
			} } : {}
		};
	};
}
/** Declare custom methods with `method(...)`, standard ones with `method.get(...)` and siblings. */
var method = Object.assign(custom, {
	get,
	list,
	create,
	update,
	updateMany,
	delete: remove
});
/** Complete a method a trait declares. */
function defineMethod(definition) {
	return {
		...definition,
		isPredicted: definition.isPredicted ?? true,
		execute: definition.execute ?? function(call) {
			return this.effect(call);
		},
		handle(handler) {
			const handling = typeof handler === "function" ? { effect: handler } : handler;
			if (handling.settle !== void 0 && handling.prepare === void 0) throw new TypeError("a method settles only the work its prepare phase does");
			const effect = this.effect.bind(this);
			const wrapped = handling.effect;
			return {
				...this,
				...wrapped === void 0 ? {} : {
					effect: (call) => wrapped(call, (changed = call) => effect(changed)),
					...this.kind === "custom" ? { isPredicted: true } : {}
				},
				...handling.prepare === void 0 ? {} : {
					prepare: handling.prepare,
					isPredicted: false
				},
				...handling.settle === void 0 ? {} : { settle: handling.settle },
				...handling.key === void 0 ? {} : { key: handling.key },
				...handling.authorize === void 0 ? {} : { authorize: async (call) => {
					await this.authorize?.(call);
					await handling.authorize(call);
				} }
			};
		}
	};
}
/** A parent of any type, in the scope of the object naming it. */
var ParentReference = defineSchema(ObjectReference.omit({ scope: true }));
/** Objects nested under a parent, or under an object of their type in a tree. */
var nested = {
	key: "nested",
	options: (definition) => definition.nested,
	columns: (options, object) => {
		if (options.in === "any") {
			const columns = {
				parentPackageId: identifier$1("parent_package_id", "package"),
				parentType: text$1("parent_type"),
				parentId: text$1("parent_id")
			};
			return options.optional ? columns : {
				parentPackageId: columns.parentPackageId.notNull(),
				parentType: columns.parentType.notNull(),
				parentId: columns.parentId.notNull()
			};
		}
		const owner = options.in === "self" ? void 0 : options.in;
		const column = identifier$1("parent_id", owner?.identity ?? object.identity).references(() => (owner?.table ?? object.table())[TABLE].columns.id, { onDelete: options.delete ?? "cascade" });
		return { parentId: options.optional ? column : column.notNull() };
	},
	constraints: (options, table, columns) => options.in === "any" ? [index(`${table}_parent`).on(columns.parentPackageId, columns.parentType, columns.parentId)] : [],
	table: (options) => options.in === "self" ? { tree: {
		id: "id",
		scope: "scope",
		parent: "parentId"
	} } : {},
	policy: (options, object) => {
		if (options.in === "any") return { relations: { parent: {
			grantedBy: null,
			subjects: [],
			open: true
		} } };
		return { relations: { parent: {
			grantedBy: null,
			subjects: [options.in === "self" ? {
				packageId: object.packageId,
				type: object.name
			} : options.in.policy]
		} } };
	},
	mapping: (options, object) => {
		if (options.in === "any") return { relations: { parent: {
			column: "parentId",
			subject: {
				packageId: "parentPackageId",
				type: "parentType",
				scope: "scope"
			}
		} } };
		const tree = object.table()[TABLE].tree;
		return {
			relations: { parent: { column: "parentId" } },
			parent: "parent",
			...tree === void 0 ? {} : { trees: { parent: tree } }
		};
	},
	methods: (options) => options.move === void 0 ? {} : { move: move(options.move) },
	validate: (options, object) => {
		if (object.storage === "ephemeral" && options.in !== "any") throw new TypeError(`ephemeral object ${object.name} attaches to its parent as an attachment of any type`);
		if (options.in === "self" && !options.optional) throw new TypeError(`${object.name} is a tree, so its parent must be optional`);
		const parent = options.in === "self" ? object : options.in;
		if (parent !== "any" && !parent.permissions.includes(options.receive)) throw new TypeError(`object ${object.name} has parents lacking permission ${options.receive}`);
	}
};
/** Move objects to another parent and keep trees acyclic. */
function move(permission) {
	return defineMethod({
		kind: "move",
		permission,
		mutates: true,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/{id}/move"
			},
			input: shapes.target.extend({
				...shapes.replay,
				...RevisionShape,
				...shapes.destination
			}),
			output: shapes.row
		}),
		effect: (call) => call.revise(call.parentColumns()),
		inverse: (step) => {
			const before = step.before;
			if (before === void 0) return;
			const destination = step.object.parent.object !== "any" ? { parentId: before.parentId ?? null } : { parent: before.parentId === null || before.parentId === void 0 ? null : {
				packageId: before.parentPackageId,
				type: before.parentType,
				id: before.parentId
			} };
			return [Step.record(step, step.name, {
				...Step.target(step, step.input.id),
				...destination
			})];
		},
		async execute(call) {
			const { object } = call;
			const parent = call.parent();
			if (parent === void 0 && !object.parent.optional) throw new ORPCError("BAD_REQUEST", { message: `${object.name} needs a parent` });
			if (parent !== void 0) {
				if (!call.isPredicted) await call.requireReceiving(parent);
				await requireOutside(call, parent.id);
			}
			return this.effect(call);
		}
	});
}
/** Refuse moving a node of a tree into its own subtree. */
async function requireOutside(call, parentId) {
	if (!call.object.tree) return;
	const moved = String(call.target.id);
	if (!call.isPredicted) {
		const ancestors = call.object.tree.ancestors;
		const [below] = await call.database.select({ depth: ancestors.depth }).from(ancestors).where(and(eq(ancestors.scope, call.scope), eq(ancestors.ancestor, moved), eq(ancestors.descendant, parentId)));
		if (below !== void 0) throw new ORPCError("CONFLICT", { message: `${call.object.name} cannot move into its own subtree` });
		return;
	}
	const table = call.object.table;
	const visited = /* @__PURE__ */ new Set();
	for (let node = parentId; node !== null && !visited.has(node);) {
		if (node === moved) throw new ORPCError("CONFLICT", { message: `${call.object.name} cannot move into its own subtree` });
		visited.add(node);
		const [row] = await call.database.select({ parentId: table.parentId }).from(table).where(eq(table.id, node));
		node = row?.parentId ?? null;
	}
}
/** The fields paging a list. */
var PageShape = {
	/** The continuation from the previous page. */
	cursor: string().min(1).optional(),
	/** The largest page to return. */
	limit: number().int().min(1).max(1e3).optional()
};
/** The revision a change requires. */
var RevisionShape = { revision: number().int().positive().optional() };
/** The empty result of methods returning nothing. */
var Empty = strictObject({});
/** Derive one procedure per method of an object type. */
function objectProcedures(object) {
	const collection = `${scopeRoute(object).prefix}/${kebabCase(object.plural)}`;
	const shapes = objectSchema(object);
	const procedures = {};
	for (const [name, method] of Object.entries(object.methods)) {
		if (method.isSystem) continue;
		const { route, input, output } = method.procedure(name, shapes);
		procedures[name] = procedure(defineProcedure({
			authentication: method.mutates ? "identity" : "public",
			permission: null,
			audit: false,
			convert: object.conversions(name)
		}), {
			method: route.method,
			path: `${collection}${route.path}`
		}, input, output);
	}
	return procedures;
}
/** Build the schemas an object's procedures compose. */
function objectSchema(object) {
	const idColumn = object.table[TABLE].columns.id;
	if (idColumn === void 0) throw new TypeError(`object ${object.name} is held in a table without an id column`);
	const id = idColumn.definition.schema;
	const columns = createSelectSchema(object.table, "json");
	const selected = strictObject(Object.fromEntries(Object.entries(columns.shape).filter(([name]) => !object.sensitive.includes(name))));
	const parent = object.parent && parentSchema(object.parent.object);
	const field = scopeRoute(object).field;
	const selection = strictObject(field === void 0 ? {} : { [field]: string().min(1) });
	const row = selected.extend({
		...Object.fromEntries(object.guarded.map((name) => [name, selected.shape[name].optional()])),
		...Object.fromEntries(object.text.map((name) => [name, string()]))
	});
	const inserted = createInsertSchema(object.table, "json").shape;
	return {
		row,
		written: (names, isPartial) => Object.fromEntries((names ?? object.written).map((name) => {
			const field = object.fields[name];
			const column = object.table[TABLE].columns[name]?.definition;
			const value = column === void 0 ? void 0 : column.json ?? column.schema;
			const declared = !(field?.required === true && field.access?.read !== void 0) ? inserted[name] : field.initial === void 0 ? value : value?.optional();
			if (declared === void 0) throw new TypeError(`${object.name} writes no column ${name}`);
			return [name, isPartial ? declared.optional() : declared];
		})),
		scope: selection,
		target: selection.extend({ id }),
		created: { id: id.optional() },
		parent: Object.fromEntries(Object.entries(parent ?? {}).map(([name, column]) => [name, object.parent.optional ? column.optional() : column])),
		destination: Object.fromEntries(Object.entries(parent ?? {}).map(([name, column]) => [name, object.parent.optional ? column.nullable() : column])),
		replay: object.storage === "ephemeral" ? { client: ClientId } : { requestId: RequestId.schema }
	};
}
/** Build the schema naming an object's parent. */
function parentSchema(parent) {
	return parent === "any" ? { parent: ParentReference } : { parentId: identifier(parent.identity) };
}
/** Derive an object's scope route. */
function scopeRoute(object) {
	const scope = object.scope;
	if (scope === Scope.universe.id) return { prefix: "" };
	else if (Array.isArray(scope)) return {
		prefix: "",
		field: "scope"
	};
	const single = scope;
	return {
		prefix: `/${kebabCase(single.plural)}/{${single.identity}Id}`,
		field: `${single.identity}Id`
	};
}
/** Declare one derived procedure with its access, route, input and output. */
function procedure(access, route, input, output) {
	return access.route(route).input(input).output(output);
}
/** The digits of minted positions in ascending order, sorting alike in every collation. */
var POSITION_DIGITS = "0123456789abcdefghijklmnopqrstuvwxyz";
/** The shape of a position. */
var POSITION = /^[0-9a-z]+$/;
/** A field of an object. */
var Field = class Field {
	/** The field's meaning. */
	type;
	/** The referenced object type, for references to objects. */
	target;
	/** The principal kinds a principal reference or subject field holds. */
	principals;
	/** Whether a reference holds the scope beside the identifier. */
	qualified;
	/** Whether every object must hold a value. */
	required;
	/** The value creation supplies when the caller omits the field. */
	initial;
	/** How the value is protected. */
	classification;
	/** The permissions reading or writing the value requires. */
	access;
	/** Whether creation fills the value with the calling principal. */
	isCaller;
	/** The aggregate function keeping the value current. */
	aggregate;
	/** The state machine the value follows. */
	machine;
	/** Create the nullable column storing the field. */
	#build;
	/** Retain the field's meaning and storage. */
	constructor(definition) {
		this.type = definition.type;
		this.target = definition.target;
		this.principals = definition.principals;
		this.qualified = definition.qualified ?? false;
		this.required = definition.required;
		this.initial = definition.initial;
		this.classification = definition.classification;
		this.access = definition.access;
		this.isCaller = definition.isCaller ?? false;
		this.aggregate = definition.aggregate;
		this.machine = definition.machine;
		this.#build = definition.build;
	}
	/** Create the column storing the field. */
	column(name, owner, storage) {
		let column = this.#build(name, owner, storage);
		if (this.classification === "sensitive") {
			const { definition } = column.sensitive();
			column = new ColumnBuilder({
				...definition,
				schema: sensitive(definition.schema.clone()),
				...definition.json === void 0 ? {} : { json: sensitive(definition.json.clone()) }
			});
		} else if (this.classification === "personal") column = column.personal();
		if (this.initial) column = column.default(this.initial.value);
		if (this.required && this.access?.read === void 0) column = column.notNull();
		return column;
	}
	/** Allow the field to be absent. */
	optional() {
		return this.#with({ required: false });
	}
	/** Supply a value when creation omits the field. */
	default(value) {
		return this.#with({ initial: { value } });
	}
	/** Keep the value out of logs, audit details, sync and request fingerprints. */
	sensitive() {
		return this.#with({ classification: "sensitive" });
	}
	/** Mark the value as personal data, exported and erased with its subject. */
	personal() {
		return this.#with({ classification: "personal" });
	}
	/** Fill the field with the principal creating the object. */
	caller() {
		if (this.principals === void 0 && this.type !== "subject") throw new TypeError("only principal references and subject fields hold the calling principal");
		return this.#with({ isCaller: true });
	}
	/**
	* Require extra permissions to read or write the value.
	*
	* A field guarded to read stays required to write, while its column is nullable: readers without the permission and copies in other databases hold it concealed.
	*/
	guard(access) {
		return this.#with({ access: {
			...this.access,
			...access
		} });
	}
	/** Derive a field with changed properties. */
	#with(change) {
		return new Field({
			type: this.type,
			build: this.#build,
			target: this.target,
			...this.principals === void 0 ? {} : { principals: this.principals },
			qualified: this.qualified,
			required: change.required ?? this.required,
			initial: change.initial ?? this.initial,
			classification: change.classification ?? this.classification,
			access: change.access ?? this.access,
			...this.aggregate === void 0 ? {} : { aggregate: this.aggregate },
			...this.machine === void 0 ? {} : { machine: this.machine },
			isCaller: change.isCaller ?? this.isCaller
		});
	}
};
/** Build a required field of one type from a nullable column. */
function required(type, build, target) {
	return new Field({
		type,
		build,
		required: true,
		target
	});
}
/** Declare a field an aggregate keeps, starting at a value or absent. */
function aggregated(aggregate, type, build, initial) {
	return new Field({
		type,
		build,
		required: initial !== void 0,
		...initial === void 0 ? {} : { initial: { value: initial } },
		aggregate
	});
}
/** Reference another object, or a principal of a kind. */
function reference(target, options = {}) {
	if (target instanceof Policy) {
		if (target.definition.isGlobal !== true) throw new TypeError(`principal kind ${target.name} lives in scopes; hold it with field.subject(principal.${target.name})`);
		else if (options.qualified !== void 0 || options.delete !== void 0) throw new TypeError(`a reference to principal kind ${target.name} holds a global identifier, without a scope or deletion`);
		return new Field({
			type: "reference",
			required: true,
			principals: [target],
			build: (name) => text$1(name).validate(string().min(1))
		});
	}
	if (target === "self" && !options.qualified) throw new TypeError("a reference to its own type must be qualified; use a parent for trees");
	const resolve = target === "self" ? void 0 : typeof target === "function" ? target : () => target;
	return new Field({
		type: "reference",
		required: true,
		...resolve === void 0 ? {} : { target: resolve },
		qualified: options.qualified ?? false,
		build: (name, owner, storage) => {
			if (options.qualified) return json$1(name, strictObject({
				/** The scope containing the referenced object. */
				scope: string().min(1),
				/** The referenced object's identifier. */
				id: lazy(() => identifier(resolve?.().identity ?? owner))
			}));
			else if (storage === "ephemeral") return identifier$1(name, () => resolve().identity);
			else return identifier$1(name, () => resolve().identity).references(() => resolve().table[TABLE].columns.id, { onDelete: options.delete === "null" ? "set null" : options.delete ?? "restrict" });
		}
	});
}
/** Declare the fields of objects. */
var field = {
	/** Text a string schema validates. */
	string(validator = string()) {
		return required("string", (name) => text$1(name).validate(validator));
	},
	/** A whole number within the exactly representable range. */
	integer() {
		return required("integer", integer);
	},
	/** A double-precision number. */
	number() {
		return required("number", real);
	},
	/** The count of objects belonging to each object. */
	count() {
		return aggregated("count", "integer", integer, 0);
	},
	/** The sum of a field over each object's belonging objects. */
	sum() {
		return aggregated("sum", "number", real, 0);
	},
	/** The smallest value of a field over each object's belonging objects. */
	min() {
		return aggregated("min", "number", real);
	},
	/** The largest value of a field over each object's belonging objects. */
	max() {
		return aggregated("max", "number", real);
	},
	/** True or false. */
	boolean() {
		return required("boolean", boolean$1);
	},
	/** An instant in UTC epoch milliseconds. */
	time() {
		return required("time", integer);
	},
	/** One of a fixed set of strings. */
	enum(values) {
		return required("enum", (name) => text$1(name, { enum: values }));
	},
	/** A state machine changed by its transition methods. */
	state(machine) {
		const names = /* @__PURE__ */ new Set([machine.initial]);
		for (const transition of Object.values(machine.transitions)) {
			transition.from.forEach((state) => names.add(state));
			names.add(transition.to);
		}
		return new Field({
			type: "state",
			required: true,
			initial: { value: machine.initial },
			machine,
			build: (name) => text$1(name, { enum: [...names] })
		});
	},
	/** Structured data validated by a schema. */
	json(validator) {
		return required("json", (name) => json$1(name, validator));
	},
	/** A principal of the given kinds, stored as its subject key. */
	subject(...kinds) {
		const prefixes = kinds.map((kind) => JSON.stringify([kind.definition.packageId, kind.name]).slice(0, -1));
		const key = prefixes.length === 0 ? string().min(1) : string().regex(new RegExp(`^(${prefixes.map(literally).join("|")}),`));
		return new Field({
			type: "subject",
			required: true,
			...kinds.length === 0 ? {} : { principals: kinds },
			build: (name) => text$1(name).validate(key)
		});
	},
	/** Another object by identifier, or a principal by its global identifier. */
	reference,
	/** A fractional index ordering objects among their siblings. */
	position() {
		return required("position", (name) => text$1(name).validate(string().regex(POSITION)));
	},
	/** A text held in chunks and changed by edits, empty at first. */
	text() {
		return new Field({
			type: "text",
			required: true,
			initial: { value: "" },
			build: () => {
				throw new TypeError("a text field holds its characters in chunks, not a column");
			}
		});
	}
};
/** Fractional indexes ordering siblings. */
var Position = { 
/** Mint a position between two others, either end open when absent. */
between(before, after) {
	let minted = "";
	let upper = after;
	for (let index = 0;; index++) {
		const low = before !== void 0 && index < before.length ? digit(before[index]) : 0;
		const high = upper !== void 0 && index < upper.length ? digit(upper[index]) : 36;
		if (high - low > 1) return minted + POSITION_DIGITS[Math.floor((low + high) / 2)];
		minted += POSITION_DIGITS[low];
		if (high > low) upper = void 0;
	}
} };
/** Read the value of a position digit. */
function digit(character) {
	return POSITION_DIGITS.indexOf(character);
}
/** Match a text literally within a regular expression. */
function literally(text) {
	return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
/** A stable element of a sequence. */
var Element = strictObject({
	/** The run the element was inserted in. */
	run: string().min(1),
	/** The element's offset within its run. */
	offset: number().int().nonnegative()
});
/** Consecutive elements of one run in sequence order. */
var Run = strictObject({
	/** The run the elements were inserted in. */
	run: string().min(1),
	/** The offset of the piece's first element within its run. */
	start: number().int().nonnegative(),
	/** The piece's characters, or its length once deleted. */
	text: union([string().min(1), number().int().positive()])
});
/** One change to a sequence. */
var SequenceEdit = union([
	strictObject({
		/** Insert a new run's characters right after an element, or at the start. */
		insert: string().min(1),
		/** The new run's identifier. */
		run: string().min(1),
		/** The element the run follows, absent at the start. */
		after: Element.optional()
	}),
	strictObject({ 
	/** Delete the elements from one element through another, in sequence order. */
delete: strictObject({
		from: Element,
		to: Element
	}) }),
	strictObject({ 
	/** Restore the deleted elements from one element through another with their characters. */
restore: strictObject({
		from: Element,
		to: Element,
		text: string().min(1)
	}) })
]);
/** A text as ordered runs of stable elements, deleted ones kept as tombstone lengths. */
var Sequence = class Sequence {
	/** The runs in sequence order. */
	runs;
	/** Hold runs in sequence order. */
	constructor(runs = []) {
		this.runs = runs;
	}
	/** Read the visible text. */
	text() {
		return this.runs.flatMap((piece) => typeof piece.text === "string" ? [piece.text] : []).join("");
	}
	/** Apply one edit, returning the sequence it makes. */
	apply(edit) {
		if ("insert" in edit) return this.#insert(edit.insert, edit.run, edit.after);
		else if ("delete" in edit) return this.#mark(edit.delete.from, edit.delete.to, void 0);
		else return this.#mark(edit.restore.from, edit.restore.to, edit.restore.text);
	}
	/** Derive the edits undoing one edit applied to a sequence. */
	static inverse(edit, before) {
		if ("insert" in edit) {
			const last = {
				run: edit.run,
				offset: edit.insert.length - 1
			};
			return [{ delete: {
				from: {
					run: edit.run,
					offset: 0
				},
				to: last
			} }];
		}
		const range = "delete" in edit ? edit.delete : edit.restore;
		const cut = before.#cut(range.from, "before").#cut(range.to, "after");
		return cut.runs.slice(cut.#find(range.from), cut.#find(range.to) + 1).flatMap((piece) => {
			const from = {
				run: piece.run,
				offset: piece.start
			};
			const to = {
				run: piece.run,
				offset: piece.start + Sequence.length(piece) - 1
			};
			if ("delete" in edit) return typeof piece.text === "string" ? [{ restore: {
				from,
				to,
				text: piece.text
			} }] : [];
			return typeof piece.text === "number" ? [{ delete: {
				from,
				to
			} }] : [];
		});
	}
	/** Translate a text change at offsets into edits naming elements. */
	change(change, run) {
		const deleted = change.to > change.from ? [{ delete: {
			from: this.element(change.from),
			to: this.element(change.to - 1)
		} }] : [];
		const after = change.from === 0 ? void 0 : this.element(change.from - 1);
		const inserted = change.insert.length > 0 ? [{
			insert: change.insert,
			run,
			...after === void 0 ? {} : { after }
		}] : [];
		return [...deleted, ...inserted];
	}
	/** Translate an edit into text changes at offsets. */
	changesOf(edit) {
		if ("insert" in edit) {
			const from = edit.after === void 0 ? 0 : this.#after(edit.after);
			return [{
				from,
				to: from,
				insert: edit.insert
			}];
		}
		const range = "delete" in edit ? edit.delete : edit.restore;
		const cut = this.#cut(range.from, "before").#cut(range.to, "after");
		const pieces = cut.runs.slice(cut.#find(range.from), cut.#find(range.to) + 1);
		const from = this.offset(range.from);
		const changes = [];
		let cursor = from;
		let restored = "restore" in edit ? edit.restore.text : "";
		for (const piece of pieces) {
			const length = Sequence.length(piece);
			if ("restore" in edit && typeof piece.text === "number") {
				changes.push({
					from: cursor,
					to: cursor,
					insert: restored.slice(0, length)
				});
				restored = restored.slice(length);
				cursor += length;
			} else if (typeof piece.text === "string") cursor += length;
		}
		return "delete" in edit ? cursor > from ? [{
			from,
			to: cursor,
			insert: ""
		}] : [] : changes;
	}
	/** Split the visible text into stretches each covered by one set of annotations. */
	spans(annotations) {
		const order = /* @__PURE__ */ new Map();
		let ordinal = 0;
		for (const piece of this.runs) for (let index = 0; index < Sequence.length(piece); index++) {
			order.set(`${piece.run}:${piece.start + index}`, ordinal);
			ordinal += 1;
		}
		const place = (anchor) => {
			const at = order.get(`${anchor.element.run}:${anchor.element.offset}`);
			if (at === void 0) throw new RangeError(`sequence holds no element ${anchor.element.run}:${anchor.element.offset}`);
			return 2 * at + (anchor.side === "before" ? 0 : 2);
		};
		const ranges = annotations.map((annotation) => ({
			annotation,
			start: place(annotation.start),
			end: place(annotation.end)
		}));
		const spans = [];
		for (const piece of this.runs) {
			if (typeof piece.text !== "string") continue;
			const text = piece.text;
			for (let index = 0; index < text.length; index++) {
				const position = 2 * order.get(`${piece.run}:${piece.start + index}`) + 1;
				const covering = ranges.filter((range) => range.start < position && position < range.end).map((range) => range.annotation);
				const last = spans.at(-1);
				if (last !== void 0 && Sequence.#same(last.annotations, covering)) last.text += text[index];
				else spans.push({
					text: text[index],
					annotations: covering
				});
			}
		}
		return spans;
	}
	/** Find the element at an offset of the visible text, absent at the text's end. */
	element(offset) {
		let remaining = offset;
		for (const piece of this.runs) {
			const length = Sequence.length(piece);
			if (typeof piece.text === "number") continue;
			else if (remaining < length) return {
				run: piece.run,
				offset: piece.start + remaining
			};
			remaining -= length;
		}
	}
	/** Find the text offset right after an element. */
	#after(element) {
		const index = this.#find(element);
		const piece = this.runs[index];
		return this.offset(element) + (typeof piece.text === "number" ? 0 : 1);
	}
	/** Find the text offset of an element. */
	offset(element) {
		let before = 0;
		for (const piece of this.runs) {
			const length = Sequence.length(piece);
			if (piece.run === element.run && element.offset >= piece.start && element.offset < piece.start + length) return before + (typeof piece.text === "number" ? 0 : element.offset - piece.start);
			else if (typeof piece.text === "string") before += length;
		}
		throw new RangeError(`sequence holds no element ${element.run}:${element.offset}`);
	}
	/** Insert a run's characters right after an element, splitting the piece holding it. */
	#insert(text, run, after) {
		const inserted = {
			run,
			start: 0,
			text
		};
		if (after === void 0) return new Sequence([inserted, ...this.runs]);
		const index = this.#find(after);
		const piece = this.runs[index];
		const cut = after.offset - piece.start + 1;
		const [head, tail] = Sequence.split(piece, cut);
		return new Sequence([
			...this.runs.slice(0, index),
			head,
			inserted,
			...tail === void 0 ? [] : [tail],
			...this.runs.slice(index + 1)
		]);
	}
	/** Delete a range of elements, or restore its tombstones with the given characters. */
	#mark(from, to, restored) {
		const cut = this.#cut(from, "before").#cut(to, "after");
		const first = cut.#find(from);
		const last = cut.#find(to);
		if (last < first) throw new RangeError(`sequence range ends before it starts: ${from.run}:${from.offset}`);
		const deleted = cut.runs.slice(first, last + 1).reduce((sum, piece) => sum + (typeof piece.text === "number" ? piece.text : 0), 0);
		if (restored !== void 0 && restored.length !== deleted) throw new RangeError(`sequence restores ${restored.length} characters into ${deleted} deleted ones`);
		let remaining = restored ?? "";
		const marked = cut.runs.map((piece, index) => {
			const length = Sequence.length(piece);
			if (index < first || index > last) return piece;
			else if (restored === void 0) return {
				run: piece.run,
				start: piece.start,
				text: length
			};
			else if (typeof piece.text === "string") return piece;
			const text = remaining.slice(0, length);
			remaining = remaining.slice(length);
			return {
				run: piece.run,
				start: piece.start,
				text
			};
		});
		return new Sequence(Sequence.#merge(marked));
	}
	/** Split the piece holding an element at one of its edges. */
	#cut(element, edge) {
		const index = this.#find(element);
		const piece = this.runs[index];
		const count = element.offset - piece.start + (edge === "after" ? 1 : 0);
		const [head, tail] = Sequence.split(piece, count);
		return tail === void 0 ? this : new Sequence([
			...this.runs.slice(0, index),
			head,
			tail,
			...this.runs.slice(index + 1)
		]);
	}
	/** Find the index of the piece holding an element. */
	#find(element) {
		const index = this.runs.findIndex((piece) => piece.run === element.run && element.offset >= piece.start && element.offset < piece.start + Sequence.length(piece));
		if (index === -1) throw new RangeError(`sequence holds no element ${element.run}:${element.offset}`);
		return index;
	}
	/** Split a piece before a count of its elements. */
	static split(piece, cut) {
		const length = Sequence.length(piece);
		if (cut <= 0 || cut >= length) return [piece, void 0];
		const [head, tail] = typeof piece.text === "number" ? [cut, length - cut] : [piece.text.slice(0, cut), piece.text.slice(cut)];
		return [{
			run: piece.run,
			start: piece.start,
			text: head
		}, {
			run: piece.run,
			start: piece.start + cut,
			text: tail
		}];
	}
	/** Merge neighbouring pieces continuing one run in the same state. */
	static #merge(runs) {
		const merged = [];
		for (const piece of runs) {
			const previous = merged.at(-1);
			if (previous !== void 0 && previous.run === piece.run && previous.start + Sequence.length(previous) === piece.start && typeof previous.text === typeof piece.text) merged[merged.length - 1] = {
				...previous,
				text: typeof previous.text === "number" ? previous.text + piece.text : previous.text + piece.text
			};
			else merged.push(piece);
		}
		return merged;
	}
	/** Decide whether two lists name the same annotations in the same order. */
	static #same(left, right) {
		return left.length === right.length && left.every((annotation, index) => annotation === right[index]);
	}
	/** Count a piece's elements. */
	static length(piece) {
		return typeof piece.text === "number" ? piece.text : piece.text.length;
	}
};
var __destackModule$28 = Object.freeze({ "package": {
	"id": "package-01a0d462-1b26-7401-be27-6258e0a32141",
	"name": "@destack/object",
	"version": "2026.9.0"
} });
/** The pieces of owners' texts, in position order per field. */
var chunk = defineTable("chunk", {
	/** The chunk's identifier. */
	id: identifier$1("id", "chunk").primaryKey(),
	/** The scope holding the owner. */
	scope: text$1("scope").notNull(),
	/** The package declaring the owner's type. */
	parentPackageId: identifier$1("parent_package_id", "package").notNull(),
	/** The owner's type. */
	parentType: text$1("parent_type").notNull(),
	/** The owner's identifier. */
	parentId: text$1("parent_id").notNull(),
	/** The owner's text field. */
	field: text$1("field").notNull(),
	/** The chunk's place among the field's chunks. */
	position: text$1("position").notNull(),
	/** The chunk's runs in sequence order. */
	runs: json$1("runs", array(Run)).notNull()
}, {
	log: {},
	constraints: (held) => [index("chunk_parent").on(held.parentPackageId, held.parentType, held.parentId, held.field, held.position)]
}, __destackModule$28);
/** The server's index of which chunk holds each run's elements, unsynced. */
var chunkRun = defineTable("chunk_run", {
	/** The chunk holding the elements. */
	chunk: identifier$1("chunk", "chunk").notNull().references(() => chunk.id, { onDelete: "cascade" }),
	/** The scope holding the owner. */
	scope: text$1("scope").notNull(),
	/** The package declaring the owner's type. */
	parentPackageId: identifier$1("parent_package_id", "package").notNull(),
	/** The owner's type. */
	parentType: text$1("parent_type").notNull(),
	/** The owner's identifier. */
	parentId: text$1("parent_id").notNull(),
	/** The owner's text field. */
	field: text$1("field").notNull(),
	/** The run the elements belong to. */
	run: text$1("run").notNull(),
	/** The first element's offset. */
	start: integer("start").notNull(),
	/** The offset after the last element. */
	end: integer("end").notNull()
}, { constraints: (held) => [index("chunk_run_parent").on(held.parentPackageId, held.parentType, held.parentId, held.field, held.run), index("chunk_run_chunk").on(held.chunk)] }, __destackModule$28);
var __destackModule$27 = Object.freeze({ "package": {
	"id": "package-01a0d462-1b26-7401-be27-6258e0a32141",
	"name": "@destack/object",
	"version": "2026.9.0"
} });
/** The characters a split leaves in each chunk: three quarters full, with room to type into one row. */
var FILL_CHARACTERS = 384;
/** The owner permission whose holders read its text. */
var TEXT_READ = "text";
/** The include holding each owner's chunks. */
var CHUNKS = "chunks";
/** The result of an edit: the edits undoing it. */
var Edited = strictObject({ 
/** The edits undoing the edit, in order. */
inverse: array(SequenceEdit) });
/** Pieces of the texts owners hold, read through their owner. */
var Chunk = {
	/** Declare the chunk type of the owners served together. */
	object(owners) {
		const [first] = owners;
		if (!owners.every((owner) => owner.scopes.length === first.scopes.length && owner.scopes.every((scope, index) => scope.same(first.scopes[index])))) throw new TypeError("objects holding text live in one kind of scope per database");
		return new ObjectType(__destackModule$27.package, {
			name: "chunk",
			plural: CHUNKS,
			scope: first.scope,
			table: chunk,
			nested: {
				in: "any",
				receive: TEXT_READ
			},
			relations: { parent: {
				subjects: [...owners],
				grantedBy: null
			} },
			permissions: { read: through("parent", TEXT_READ) },
			methods: { list: method.list("read") }
		}, [], {
			table: chunk,
			id: "id",
			scope: "scope",
			attributes: {},
			relations: { parent: {
				column: "parentId",
				subject: {
					packageId: "parentPackageId",
					type: "parentType",
					scope: "scope"
				}
			} }
		});
	},
	/** Report whether an owner's text field holds every element. */
	async holds(database, owner, field, elements) {
		const parent = {
			scope: owner.scope,
			parentPackageId: owner.packageId,
			parentType: owner.type,
			parentId: owner.id,
			field
		};
		const held = await database.select({
			run: chunkRun.run,
			start: chunkRun.start,
			end: chunkRun.end
		}).from(chunkRun).where(and(scoped(chunkRun, parent), inArray(chunkRun.run, elements.map((element) => element.run))));
		return elements.every((element) => held.some((entry) => entry.run === element.run && entry.start <= element.offset && element.offset < entry.end));
	},
	/** Read the texts of some owners, by owner identifier. */
	async texts(database, owner, scope, ids) {
		const rows = ids.length === 0 ? [] : await database.select().from(chunk).where(and(eq(chunk.scope, scope), eq(chunk.parentPackageId, owner.policy.definition.packageId), eq(chunk.parentType, owner.name), inArray(chunk.parentId, [...ids])));
		return new Map(ids.map((id) => [id, Chunk.text(owner, rows.filter((row) => row.parentId === id))]));
	},
	/** Assemble an owner's visible texts from its chunks, by field. */
	text(owner, rows) {
		return Object.fromEntries(owner.text.map((field) => [field, new Sequence(ordered(rows.filter((row) => row.field === field)).flatMap((row) => row.runs)).text()]));
	},
	/** Replace each row's chunks by its texts, through a query's includes. */
	present(rows, query, objects) {
		const owner = objects.find((object) => object.table === query.table);
		return rows.map((row) => {
			const presented = { ...row };
			for (const [name, include] of Object.entries(query.include ?? {})) {
				const value = row[name];
				if (name === "chunks" || include.aggregate !== void 0 || value === null) continue;
				presented[name] = Array.isArray(value) ? Chunk.present(value, include, objects) : Chunk.present([value], include, objects)[0];
			}
			if (owner === void 0 || owner.text.length === 0) return presented;
			const { [CHUNKS]: chunks, ...rest } = presented;
			return {
				...rest,
				...Chunk.text(owner, chunks)
			};
		});
	},
	/** Apply a call's edits to one text field, writing the changed chunks. */
	async edit(call) {
		const { field, edits } = call.input;
		const owner = call.object;
		const parent = {
			scope: call.scope,
			parentPackageId: owner.policy.definition.packageId,
			parentType: owner.name,
			parentId: call.id,
			field
		};
		const rows = call.isPredicted ? await read(call.database, parent, void 0) : await touched(call.database, parent, edits);
		let sequence = new Sequence(rows.flatMap((row) => row.runs));
		const inverses = [];
		for (const edit of edits) {
			if ("insert" in edit && sequence.runs.some((piece) => piece.run === edit.run)) throw new ORPCError("BAD_REQUEST", { message: `${field} already holds run ${edit.run}` });
			try {
				inverses.unshift(Sequence.inverse(edit, sequence));
				sequence = sequence.apply(edit);
			} catch (error) {
				if (error instanceof RangeError) throw new ORPCError("BAD_REQUEST", { message: error.message });
				throw error;
			}
		}
		const parts = cut(sequence.runs, rows.slice(1).map((row) => ({
			run: row.runs[0].run,
			offset: row.runs[0].start
		})));
		const changed = [];
		const created = [];
		for (const [index, part] of parts.entries()) {
			const held = rows[index];
			const [first, ...rest] = fill(part);
			if (held !== void 0 && canonicalize(first) !== canonicalize(held.runs)) changed.push({
				...held,
				runs: first
			});
			const groups = held === void 0 ? [first, ...rest] : rest;
			let before = held?.position;
			const after = groups.length === 0 ? void 0 : index + 1 < rows.length || call.isPredicted ? rows[index + 1]?.position : await next(call.database, parent, held?.position);
			for (const runs of groups) {
				before = Position.between(before, after);
				created.push({
					...parent,
					id: `chunk-${v7()}`,
					position: before,
					runs
				});
			}
		}
		for (const row of changed) await call.database.update(chunk).set({ runs: row.runs }).where(eq(chunk.id, row.id));
		if (created.length > 0) await call.database.insert(chunk).values(created);
		if (!call.isPredicted) {
			const indexed = [...changed.filter((row) => canonicalize(ranges(row.runs)) !== canonicalize(ranges(rows.find((held) => held.id === row.id).runs))), ...created];
			await reindex(call.database, indexed);
		}
		return { inverse: inverses.flat() };
	}
};
/** Match one text field's rows of a chunk or index table. */
function scoped(table, parent) {
	return and(eq(table.scope, parent.scope), eq(table.parentPackageId, parent.parentPackageId), eq(table.parentType, parent.parentType), eq(table.parentId, parent.parentId), eq(table.field, parent.field));
}
/** Read a text field's chunks in position order, within positions when given. */
async function read(database, parent, within) {
	return ordered(await database.select().from(chunk).where(and(scoped(chunk, parent), within === void 0 ? void 0 : gte(chunk.position, within.from), within === void 0 ? void 0 : lte(chunk.position, within.to))));
}
/** Read the contiguous chunks holding the elements some edits name. */
async function touched(database, parent, edits) {
	const inserted = /* @__PURE__ */ new Set();
	const named = [];
	let isAtStart = false;
	for (const edit of edits) {
		const elements = "insert" in edit ? edit.after === void 0 ? [] : [edit.after] : "delete" in edit ? [edit.delete.from, edit.delete.to] : [edit.restore.from, edit.restore.to];
		named.push(...elements.filter((element) => !inserted.has(element.run)));
		if ("insert" in edit) {
			isAtStart ||= edit.after === void 0;
			inserted.add(edit.run);
		}
	}
	const held = named.length === 0 && inserted.size === 0 ? [] : await database.select({
		run: chunkRun.run,
		start: chunkRun.start,
		end: chunkRun.end,
		position: chunk.position
	}).from(chunkRun).innerJoin(chunk, eq(chunk.id, chunkRun.chunk)).where(and(scoped(chunkRun, parent), or(...named.map((element) => and(eq(chunkRun.run, element.run), lte(chunkRun.start, element.offset), gt(chunkRun.end, element.offset))), inserted.size === 0 ? void 0 : inArray(chunkRun.run, [...inserted]))));
	const duplicate = held.find((entry) => inserted.has(entry.run));
	if (duplicate !== void 0) throw new ORPCError("BAD_REQUEST", { message: `${parent.field} already holds run ${duplicate.run}` });
	const missing = named.find((element) => !held.some((entry) => entry.run === element.run && entry.start <= element.offset && element.offset < entry.end));
	if (missing !== void 0) throw new ORPCError("BAD_REQUEST", { message: `no chunk holds element ${missing.run}:${missing.offset}` });
	const positions = held.map((entry) => entry.position);
	if (isAtStart) {
		const [first] = await database.select({ position: chunk.position }).from(chunk).where(scoped(chunk, parent)).orderBy(asc(chunk.position)).limit(1);
		positions.push(...first === void 0 ? [] : [first.position]);
	}
	const sorted = [...positions].sort();
	return sorted.length === 0 ? [] : read(database, parent, {
		from: sorted[0],
		to: sorted.at(-1)
	});
}
/** Read the position of the chunk after one, absent after the last. */
async function next(database, parent, position) {
	if (position === void 0) return;
	const [after] = await database.select({ position: chunk.position }).from(chunk).where(and(scoped(chunk, parent), gt(chunk.position, position))).orderBy(asc(chunk.position)).limit(1);
	return after?.position;
}
/** Replace the index rows of some chunks with their runs' ranges. */
async function reindex(database, rows) {
	if (rows.length === 0) return;
	await database.delete(chunkRun).where(inArray(chunkRun.chunk, rows.map((row) => row.id)));
	await database.insert(chunkRun).values(rows.flatMap((row) => ranges(row.runs).map((range) => ({
		chunk: row.id,
		scope: row.scope,
		parentPackageId: row.parentPackageId,
		parentType: row.parentType,
		parentId: row.parentId,
		field: row.field,
		...range
	}))));
}
/** List the element ranges of runs, merging continued pieces of one run. */
function ranges(runs) {
	const merged = [];
	for (const piece of runs) {
		const previous = merged.at(-1);
		const end = piece.start + Sequence.length(piece);
		if (previous !== void 0 && previous.run === piece.run && previous.end === piece.start) previous.end = end;
		else merged.push({
			run: piece.run,
			start: piece.start,
			end
		});
	}
	return merged;
}
/** Order chunks by position, as binary collations sort them. */
function ordered(rows) {
	return [...rows].sort((left, right) => left.position < right.position ? -1 : 1);
}
/** Split runs into parts starting at each boundary element. */
function cut(runs, boundaries) {
	const parts = [[]];
	let next = 0;
	for (let piece of runs) {
		while (next < boundaries.length && holds(piece, boundaries[next])) {
			const [head, tail] = Sequence.split(piece, boundaries[next].offset - piece.start);
			if (tail !== void 0) {
				parts.at(-1).push(head);
				piece = tail;
			}
			parts.push([]);
			next += 1;
		}
		parts.at(-1).push(piece);
	}
	return parts;
}
/** Split a part's runs into even groups of at most a chunk's characters. */
function fill(runs) {
	const total = runs.reduce((sum, piece) => sum + characters(piece), 0);
	const count = total <= 512 ? 1 : Math.ceil(total / FILL_CHARACTERS);
	const capacity = Math.ceil(total / count);
	const groups = [[]];
	let held = 0;
	for (let piece of runs) {
		while (characters(piece) > capacity - held) {
			const room = capacity - held;
			if (room > 0) {
				const [head, tail] = Sequence.split(piece, room);
				groups.at(-1).push(head);
				piece = tail;
			}
			groups.push([]);
			held = 0;
		}
		groups.at(-1).push(piece);
		held += characters(piece);
	}
	return groups;
}
/** Count the characters a piece stores. */
function characters(piece) {
	return typeof piece.text === "string" ? piece.text.length : 0;
}
/** Decide whether a piece holds an element. */
function holds(piece, element) {
	return piece.run === element.run && element.offset >= piece.start && element.offset < piece.start + Sequence.length(piece);
}
/** Compile object queries of a scope chain, nearest first, into queries of their tables. */
function compileQueries(objects, queries, chain) {
	const scope = chain[0];
	const isGlobal = scope === Scope.universe.id;
	const named = queries ?? Object.fromEntries(objects.filter((object) => object.listing !== void 0 && object.scope === Scope.universe.id === isGlobal).map((object) => [object.name, {
		object: object.name,
		...object.recoverable === void 0 ? {} : { deleted: "include" }
	}]));
	return Object.fromEntries(Object.entries(named).map(([name, query]) => {
		const { object: objectName, ...shape } = query;
		const object = objects.find((entry) => entry.name === objectName);
		if (!object) throw new ORPCError("BAD_REQUEST", { message: `no object ${objectName}` });
		else if (object.scope === Scope.universe.id !== isGlobal) throw new ORPCError("BAD_REQUEST", { message: `object ${objectName} is not in scope ${scope}` });
		return [name, {
			...compile(object, shape, objects),
			scopes: object.scopesOf(chain)
		}];
	}));
}
/** Compile one object query or include, requiring a listed object type. */
function compile(object, shape, objects) {
	if (object.listing === void 0) throw new ORPCError("BAD_REQUEST", { message: `object ${object.name} is not listed` });
	const includes = object.text.length === 0 || shape.aggregate !== void 0 ? shape.include : {
		...shape.include,
		[CHUNKS]: {}
	};
	const include = Object.fromEntries(Object.entries(includes ?? {}).map(([name, nested]) => {
		const { object: target, on, where } = join(object, nested.via ?? name, objects);
		const compiled = compile(target, nested, objects);
		return [name, {
			...compiled,
			on,
			...where === void 0 ? {} : { where: compiled.where === void 0 ? where : Condition.all(compiled.where, where) }
		}];
	}));
	if (shape.deleted !== void 0 && object.recoverable === void 0) throw new ORPCError("BAD_REQUEST", { message: `object ${object.name} has no trash` });
	const kept = Condition.missing("deletionRequestedAt");
	const deleted = object.recoverable === void 0 ? "include" : shape.deleted ?? "exclude";
	const where = [...shape.where === void 0 ? [] : [shape.where], ...deleted === "exclude" ? [kept] : deleted === "only" ? [Condition.not(kept)] : []];
	const relations = relationsOf(object, shape.where, Object.values(shape.compute ?? {}), objects);
	return {
		table: object.table,
		...Object.keys(relations).length === 0 ? {} : { relations },
		...shape.compute === void 0 ? {} : { compute: shape.compute },
		...where.length === 0 ? {} : { where: where.length === 1 ? where[0] : Condition.all(...where) },
		...shape.order === void 0 ? {} : { order: shape.order },
		...shape.limit === void 0 ? {} : { limit: shape.limit },
		...shape.aggregate === void 0 ? {} : { aggregate: shape.aggregate },
		...Object.keys(include).length === 0 ? {} : { include }
	};
}
/** Resolve the relations a condition's terms, lookups and rollups follow. */
function relationsOf(object, condition, computed, objects) {
	const terms = [...Condition.relations(condition ?? Condition.all()), ...computed.flatMap((expression) => [...Expression$1.lookups(expression).map(({ via }) => ({
		via,
		where: void 0
	})), ...Expression$1.rollups(expression)])];
	const relations = {};
	for (const via of new Set(terms.map((term) => term.via))) {
		const { object: target, on, where } = join(object, via, objects);
		if (target.listing === void 0) throw new ORPCError("BAD_REQUEST", { message: `object ${target.name} is not listed` });
		const wheres = terms.flatMap((term) => term.via === via && term.where !== void 0 ? [term.where] : []);
		const nested = relationsOf(target, Condition.all(...wheres), [], objects);
		const kept = [...where === void 0 ? [] : [where], ...target.recoverable === void 0 ? [] : [Condition.missing("deletionRequestedAt")]];
		relations[via] = {
			table: target.table,
			on,
			...kept.length === 0 ? {} : { where: kept.length === 1 ? kept[0] : Condition.all(...kept) },
			...Object.keys(nested).length === 0 ? {} : { relations: nested }
		};
	}
	return relations;
}
/** Resolve the name an include follows into its join. */
function join(object, name, objects) {
	const hops = name.split(".");
	if (hops.length === 2) {
		const first = join(object, hops[0], objects);
		const second = join(first.object, hops[1], objects);
		if (first.on.kind === "key" && first.on.parent === "id" && second.on.kind === "key" && second.on.column === "id") return {
			object: second.object,
			on: {
				kind: "junction",
				table: first.object.table,
				from: {
					column: first.on.column,
					key: "id"
				},
				to: {
					column: second.on.parent,
					key: "id"
				}
			}
		};
	} else if ((name === "descendants" || name === "ancestors") && object.same(object.parent?.object)) return {
		object,
		on: {
			kind: name,
			column: "parentId"
		}
	};
	else if (hops.length === 1) {
		const field = object.fields[name];
		const parent = object.parent?.object;
		const target = name === "parent" && parent !== void 0 && parent !== "any" ? {
			object: parent,
			column: "parentId"
		} : field?.type === "reference" && field.target && !field.qualified ? {
			object: field.target(),
			column: name
		} : void 0;
		const served = target && objects.find((entry) => entry.same(target.object));
		if (target !== void 0 && served !== void 0) return {
			object: served,
			on: {
				kind: "key",
				column: "id",
				parent: target.column
			}
		};
		const children = objects.find((entry) => entry.plural === name && !entry.same(object));
		const isAttached = children !== void 0 && object.holds(children);
		const column = children === void 0 ? void 0 : object.same(children.parent?.object) || isAttached ? "parentId" : referenceTo(children, object);
		if (children !== void 0 && column !== void 0) return {
			object: children,
			on: {
				kind: "key",
				column,
				parent: "id"
			},
			...isAttached ? { where: Condition.all(Condition.eq("parentPackageId", object.policy.definition.packageId), Condition.eq("parentType", object.name)) } : {}
		};
	}
	throw new ORPCError("BAD_REQUEST", { message: `object ${object.name} includes nothing named ${name}` });
}
/** Name a type's only unqualified reference field to another type. */
function referenceTo(from, to) {
	const fields = Object.entries(from.fields).filter(([, field]) => field.type === "reference" && !field.qualified && to.same(field.target?.()));
	return fields.length === 1 ? fields[0][0] : void 0;
}
var __destackModule$26 = Object.freeze({ "package": {
	"id": "package-01a0ed2a-b697-77bb-bce0-6b61abb3ed08",
	"name": "@destack/directory",
	"version": "2026.9.0"
} });
defineSchema(strictObject({
	/** The scope the databases belong to. */
	id: string().min(1),
	/** The scope containing it, such as its account. */
	scope: string().min(1),
	/** The cell serving the databases. */
	cell: string().min(1),
	/** The placement epoch the cell serves the zone at. */
	epoch: number().int().min(1)
}));
defineSchema(strictObject({
	/** The region or host. */
	id: string().min(1),
	/** The scope it belongs to. */
	scope: string().min(1),
	/** The URL its services answer at. */
	endpoint: url()
}));
/** The zones placed in the cells that serve their databases. */
var zoneTable = defineTable("zone", {
	/** The scope the databases belong to. */
	id: text$1("id").primaryKey(),
	/** The universe, where every zone row lives. */
	scope: text$1("scope").notNull(),
	/** The scope containing the zone's scope, such as its account. */
	parent: text$1("parent").notNull(),
	/** The cell serving the databases. */
	cell: text$1("cell").notNull(),
	/** The placement epoch the cell serves the zone at. */
	epoch: integer("epoch").notNull(),
	/** The cell a pending transfer moves the zone to. */
	target: text$1("target")
}, {
	tier: "global",
	log: {},
	constraints: (zone) => [
		check("zone_epoch", sql`${zone.epoch} > 0`),
		check("zone_target", sql`${zone.target} IS NULL OR ${zone.target} <> ${zone.cell}`),
		index("zone_target").on(zone.target),
		index("zone_parent").on(zone.parent)
	]
}, __destackModule$26);
defineTable("cell", {
	/** The region or host. */
	id: text$1("id").primaryKey(),
	/** The scope it belongs to. */
	scope: text$1("scope").notNull(),
	/** The URL its services answer at. */
	endpoint: text$1("endpoint").notNull(),
	/** When the cell last published its endpoint, in UTC epoch milliseconds. */
	publishedAt: integer("published_at").notNull()
}, {
	tier: "global",
	log: {}
}, __destackModule$26);
/** The answer a cell gives for a scope that moved to another cell. */
var Moved = {
	/** The schema of a moved scope. */
	schema: strictObject({
		/** The moved scope. */
		scope: string().min(1),
		/** The cell now serving it. */
		cell: string().min(1)
	}),
	/** Build the 421 failure that points to the cell a scope moved to. */
	error(moved) {
		return new ORPCError("MOVED", {
			status: 421,
			message: `${moved.scope} moves to ${moved.cell}`,
			data: moved
		});
	},
	/** Read the cell a MOVED failure points to, absent for other failures. */
	of(error) {
		return error instanceof ORPCError && error.code === "MOVED" ? Moved.schema.parse(error.data) : void 0;
	}
};
/** A caller's authorization in one call's transaction. */
var Authorization = class extends Authorization$1 {
	/** The caller's resolved access in the call's scope. */
	access;
	/** Authorize a caller in one transaction. */
	constructor(authorizer, database, bind, resolved) {
		super(authorizer, database, bind, resolved);
		this.access = resolved;
	}
	/** Match the rows the caller may list. */
	listable(object, permission) {
		if (object.storage === "ephemeral") return "memory";
		const table = object.table;
		const held = this.authorizer.where(permission, this.access, table);
		const columns = table[TABLE].columns;
		const isCopy = ne(columns.scope, this.access.scope);
		const included = Replica.includes(this.access.scope, table);
		return object.inherited === void 0 ? or(and(isCopy, included), held) : or(isCopy, held);
	}
	/**
	* Decide which rows the caller may list.
	*
	* The copies an enclosing scope hands down and the copies kept for the scope are admitted.
	* The scope's own rows are decided by the caller's access in it, and the rows of a scope it encloses by the caller's access there.
	*/
	async admitRows(object, permission, scope, rows, reader) {
		const table = object.table;
		const others = [...rows.keys()].filter((position) => rows[position].scope !== scope);
		let copies;
		let below = /* @__PURE__ */ new Map();
		if (object.inherited !== void 0) copies = new Set(others);
		else if (object.storage === "durable" && others.length > 0) {
			const included = await Replica.keysIncluded(this.database, scope, table, others.map((position) => rows[position]));
			copies = new Set(others.filter((position) => included.has(Key.name(table, rows[position]))));
			const scopes = others.filter((position) => !copies.has(position)).map((position) => String(rows[position].scope));
			below = await this.descend(scope, [...new Set(scopes)]);
		} else copies = /* @__PURE__ */ new Set();
		const own = [...rows.keys()].filter((position) => !copies.has(position) && (rows[position].scope === scope || below.has(String(rows[position].scope))));
		const admission = await this.checkRows(permission, scope, own.map((position) => rows[position]), reader, below);
		return {
			...admission,
			held: /* @__PURE__ */ new Set([...copies, ...[...admission.held].map((position) => own[position])]),
			below: [...below.keys()]
		};
	}
	/** The call's scope and the scopes containing it, nearest first. */
	get chain() {
		return [.../* @__PURE__ */ new Set([this.access.scope, ...this.access.scopes.map((entry) => entry.id)])];
	}
	/** Read a call's target where the caller holds the method's permission. */
	async read(call, id) {
		const { object, scope } = call;
		const permission = object.permission(call.method.permission);
		const table = object.table;
		const target = object.reference(scope, id);
		const select = async (evaluated) => object.storage === "ephemeral" ? await this.#decide(call, id, permission, evaluated) : await this.database.select().from(table).where(and(eq(table.id, id), object.inScope(scope), this.authorizer.holds(permission, target, evaluated)));
		const governing = await this.in(this.authorizer.governingScope(target));
		const [row] = await select(governing);
		if (row) return row;
		const stepUp = await this.authorizer.challenge(this.snapshot, permission, governing, async (stepped) => (await select(stepped)).length > 0);
		if (stepUp !== void 0) throw new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again at the required assurance", { stepUp });
		const context = governing.context;
		const delegates = context.delegates ?? [];
		const chain = delegationChain(context);
		for (let position = 0; position < delegates.length; position++) {
			if (delegates[position].authority === "full") continue;
			const lending = await this.authorizer.resolve(this.snapshot, this.authorizer.governingScope(target), {
				...context,
				delegates: delegates.slice(0, position)
			});
			const lent = await this.authorizer.resolve(this.snapshot, this.authorizer.governingScope(target), {
				...context,
				delegates: delegates.slice(0, position + 1)
			});
			if ((await select(lending)).length > 0 && (await select(lent)).length === 0) throw new ORPCError("INSUFFICIENT_GRANT", {
				status: 403,
				message: "propose the missing delegation to the principal the delegate acts for",
				data: {
					permission,
					object: object.reference(scope, id),
					delegate: chain[position].delegate,
					onBehalfOf: chain[position].delegator
				}
			});
		}
		const isReadable = object.reading !== void 0 && await this.#holds(call, id, object.reading);
		const denial = new ORPCError("FORBIDDEN", { message: `permission denied: ${permission.name}` });
		throw isReadable ? denial : conceal(denial, `no ${object.name} ${id}`);
	}
	/** Require the call's scope to be visible to the caller. */
	async requireVisible(objects) {
		const scope = this.access.scope;
		if (scope === Scope.universe.id) return;
		const own = this.access.scopes[0];
		const type = own?.id === scope ? objects.flatMap((object) => object.scopes).find((candidate) => candidate.policy.is(own)) : void 0;
		if (type === void 0) throw new ORPCError("NOT_FOUND", { message: `no scope ${scope}` });
		const { permissions, ...person } = this.access.context;
		const isNamed = permissions?.some((restriction) => restriction.scope === scope) === true;
		const context = isNamed ? person : this.access.context;
		const caller = (bound) => isNamed ? this.authorizer.resolve(this.snapshot, bound, person) : this.in(bound);
		const delegates = context.delegates ?? [];
		const lent = delegates.findIndex((delegate) => delegate.authority === "lent");
		const principal = {
			...context,
			delegates: delegates.slice(0, lent)
		};
		if (!(await this.#sees(type, own, objects, caller) || lent !== -1 && await this.#sees(type, own, objects, (bound) => this.authorizer.resolve(this.snapshot, bound, principal)))) throw conceal(new ORPCError("FORBIDDEN", { message: `scope ${scope} is not visible` }), `no scope ${scope}`);
	}
	/** Decide whether an access sees a scope. */
	async #sees(type, own, objects, resolve) {
		const container = await resolve(this.authorizer.governingScope(own));
		const permission = type.permission(SCOPE_READ);
		if ((await this.authorizer.check(this.snapshot, permission, own, container)).isAllowed) return true;
		const scope = own.id;
		const evaluated = await resolve(scope);
		const inside = objects.filter((object) => object.storage === "durable" && object.scopes.includes(type));
		for (const object of inside) {
			const table = object.table;
			const held = object.permissions.map((name) => this.authorizer.where(object.permission(name), evaluated, table));
			const [row] = await this.database.select({ id: table.id }).from(table).where(and(object.inScope(scope), or(...held))).limit(1);
			if (row !== void 0) return true;
		}
		return false;
	}
	/** Decide whether an object type's rows live in scopes of the admitted scope's type. */
	isScopeOf(object) {
		const own = this.access.scopes[0];
		return this.access.scope === Scope.universe.id ? object.scopes.length === 0 : own?.id === this.access.scope && object.scopes.some((type) => type.policy.is(own));
	}
	/** Refuse an object type living outside the admitted scope's type. */
	requireScopeOf(object) {
		if (!this.isScopeOf(object)) throw new ORPCError("NOT_FOUND", { message: `no ${object.plural} in scope ${this.access.scope}` });
	}
	/** Refuse a moved scope with its new holder. */
	requireUnmoved() {
		const moved = this.access.moved;
		if (moved !== void 0) throw Moved.error(moved);
	}
	/** Require the write permission of every guarded field a call sets. */
	async requireWritable(call, id) {
		for (const [name, declared] of Object.entries(call.object.fields)) {
			const permission = declared.access?.write;
			if (permission !== void 0 && Object.hasOwn(call.input, name) && !await this.#holds(call, id, call.object.permission(permission))) throw new ORPCError("FORBIDDEN", { message: `field ${name} is not writable` });
		}
	}
	/** Decide whether the caller holds a permission on one object of a call's type. */
	async #holds(call, id, permission) {
		const { object, scope } = call;
		if (object.storage === "ephemeral") {
			const access = await this.in(this.authorizer.governingScope(object.reference(scope, id)));
			return (await this.#decide(call, id, permission, access)).length > 0;
		}
		return (await this.check(permission, object.reference(scope, id))).isAllowed;
	}
	/** Read an ephemeral object the access holds a permission on, decided in memory. */
	async #decide(call, id, permission, evaluated) {
		const table = call.object.table;
		const rows = await call.database.select().from(table).where(and(eq(table.id, id), call.object.inScope(call.scope)));
		return this.keep(rows, permission, evaluated);
	}
	/** Keep the rows an access holds a permission on, decided in memory. */
	async keep(rows, permission, evaluated = this.access) {
		const { held } = await this.authorizer.checkRows(this.snapshot, permission, evaluated, rows);
		return rows.filter((_, position) => held.has(position));
	}
	/** List each row's guarded fields the caller may not read, and until when. */
	async concealed(object, rows, reader) {
		const hidden = rows.map(() => []);
		const permissions = /* @__PURE__ */ new Map();
		for (const name of object.guarded) {
			const permission = object.fields[name].access.read;
			permissions.set(permission, [...permissions.get(permission) ?? [], name]);
		}
		const scope = this.access.scope;
		const others = rows.map((row) => String(row.scope)).filter((other) => other !== scope);
		const below = permissions.size === 0 || others.length === 0 ? void 0 : await this.descend(scope, [...new Set(others)]);
		const moments = [];
		for (const [permission, names] of permissions) {
			const readable = await this.checkRows(object.permission(permission), scope, rows, reader, below);
			moments.push(readable.until);
			for (const [position, fields] of hidden.entries()) if (!readable.held.has(position)) fields.push(...names);
		}
		const until = earliest(moments);
		return {
			hidden,
			...until === void 0 ? {} : { until }
		};
	}
	/** Omit from each row the guarded fields the caller may not read on it. */
	async redact(object, rows) {
		const hidden = object.guarded.length === 0 ? rows.map(() => []) : (await this.concealed(object, rows)).hidden;
		return rows.map((row, position) => omit(row, [...object.sensitive, ...hidden[position]]));
	}
};
/** Copy a row without some of its fields. */
function omit(row, names) {
	return names.length === 0 ? row : Object.fromEntries(Object.entries(row).filter(([name]) => !names.includes(name)));
}
/** How long a caller's changes continue one activity after their last. */
var SESSION = { minutes: 10 };
/** Objects keeping their history, grouped into activities. */
var tracked = {
	key: "tracked",
	isDurable: true,
	options: (definition) => definition.tracked,
	columns: () => ({}),
	constraints: () => [],
	methods: (options, declared) => {
		const reading = ["get", "list"].map((kind) => Object.values(declared).find((method) => method.kind === kind)).find((method) => method !== void 0)?.permission;
		if (reading === void 0 || reading === null) throw new TypeError("an object keeping history needs a get or list method");
		return {
			history: historyMethod(reading),
			revert: revertMethod(options.by)
		};
	},
	validate: (options, object) => {
		if (!object.attachments.some((attachment) => attachment.object === options.activity)) throw new TypeError(`object ${object.name} keeps history but takes no ${options.activity.plural}`);
		if (options.session !== void 0) Duration.require(options.session, `history session of ${object.name}`);
	},
	require(objects, authorizer) {
		const followed = /* @__PURE__ */ new Set();
		for (const object of objects.filter((served) => served.tracked !== void 0)) requireKept(object, object, object.reading.name, {
			objects,
			authorizer,
			followed
		});
	},
	async record(call, before, after, from) {
		const { object } = call;
		const options = object.tracked;
		const activity = options.activity;
		const table = activity.table;
		const caller = subjectKey(call.caller);
		const [latest] = await call.database.select().from(table).where(and(eq(table.parentPackageId, object.policy.definition.packageId), eq(table.parentType, object.name), eq(table.parentId, call.id), eq(table.caller, caller))).orderBy(desc(table.endedAt)).limit(1);
		const changed = Object.keys(object.fields).filter((name) => !Step.same(before?.[name], after[name]));
		const session = Duration.milliseconds(options.session ?? SESSION);
		if (latest !== void 0 && latest.endedAt >= call.now - session) {
			const fields = [.../* @__PURE__ */ new Set([...latest.fields, ...changed])];
			await call.database.update(table).set({
				endedAt: call.now,
				fields,
				changes: latest.changes + 1,
				revision: latest.revision + 1,
				updatedAt: call.now
			}).where(eq(table.id, latest.id));
		} else await call.database.insert(table).values({
			id: `${activity.identity}-${v7()}`,
			scope: call.scope,
			parentPackageId: object.policy.definition.packageId,
			parentType: object.name,
			parentId: call.id,
			caller,
			startedAt: call.now,
			endedAt: call.now,
			from,
			fields: changed,
			changes: 1,
			createdAt: call.now,
			updatedAt: call.now
		});
	}
};
/** Read an object as it was at a position. */
function historyMethod(permission) {
	return defineMethod({
		kind: "history",
		permission,
		mutates: false,
		isPredicted: false,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/{id}/history"
			},
			input: shapes.target.extend({ at: LogPosition }),
			output: shapes.row
		}),
		effect: (call) => earlier(call, call.object.permission(call.method.permission))
	});
}
/** Revert an object's written fields to a position. */
function revertMethod(permission) {
	return defineMethod({
		kind: "revert",
		permission,
		mutates: true,
		isPredicted: false,
		target: true,
		result: "object",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/{id}/revert"
			},
			input: shapes.target.extend({
				...shapes.replay,
				at: LogPosition
			}),
			output: shapes.row
		}),
		effect: async (call) => {
			const row = await earlier(call, call.object.reading);
			const target = call.target;
			const changes = Object.fromEntries(call.object.written.filter((name) => !Step.same(row[name], target[name])).map((name) => [name, row[name]]));
			return Object.keys(changes).length === 0 ? target : call.revise(changes);
		},
		inverse: (step) => {
			const current = step.current;
			if (current === void 0) return;
			const restored = Object.fromEntries(step.object.written.filter((name) => !Step.same(step.before?.[name], step.after?.[name])).filter((name) => Step.same(current[name], step.after?.[name])).map((name) => [name, step.before?.[name] ?? null]));
			const update = Step.call(step, "update", {
				...Step.target(step, step.input.id),
				...restored
			});
			return Object.keys(restored).length === 0 || update === void 0 ? void 0 : [update];
		}
	});
}
/** Read the call's object at the input's position. */
async function earlier(call, permission) {
	const at = LogPosition.parse(call.input.at);
	const authorization = call.served();
	const snapshot = call.database.log.at(at);
	const row = await snapshot.row(call.object.table, { id: call.id });
	if (row === void 0) throw new ORPCError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
	const scope = authorization.authorizer.governingScope(call.reference());
	const access = await authorization.authorizer.resolve(snapshot, scope, authorization.context(scope));
	if (!(await authorization.authorizer.check(snapshot, permission, call.reference(), access)).isAllowed) throw new ORPCError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
	return row;
}
/** Require the objects a permission reads through to keep their history. */
function requireKept(keeper, object, permission, served) {
	const key = `${object.name}.${permission}`;
	if (served.followed.has(key)) return;
	served.followed.add(key);
	const types = [...served.objects, ...served.objects.flatMap((candidate) => candidate.scopes)];
	for (const part of parts(object.policy.definition.permissions[permission])) if (part.kind === "permission") requireKept(keeper, object, part.name, served);
	else if (part.kind === "through") {
		const subjects = served.authorizer.relation(object.policy, part.relation).subjects;
		const reached = types.filter((candidate) => subjects.some((subject) => candidate.policy.definition.packageId === subject.packageId && candidate.policy.definition.name === subject.type));
		for (const target of new Set(reached)) {
			if (target.table[TABLE].retention !== "history") throw new TypeError(`object ${keeper.name} keeps history but reads through ${target.name}, which keeps none`);
			requireKept(keeper, target, part.permission, served);
		}
	}
}
/** List the permissions and relations an expression reads. */
function parts(expression) {
	return expression === void 0 ? [] : expression.kind === "union" || expression.kind === "intersection" ? expression.expressions.flatMap(parts) : expression.kind === "exclusion" ? [...parts(expression.include), ...parts(expression.exclude)] : [expression];
}
/** The most rows one expiry removes per transaction: about a millisecond of deletes. */
var EXPIRE_ROWS = 100;
/** Objects the system removes once a rule's window passes. */
var expiring = {
	key: "expiring",
	isDurable: true,
	options: (definition) => definition.expiring,
	columns: () => ({}),
	constraints: () => [],
	methods: () => ({ expire: expiry }),
	validate: (rules, object) => requireRules(object, rules),
	after(object, rules) {
		if (object.expiring === void 0) throw new TypeError(`object ${object.name} does not expire`);
		requireRules(object, rules);
		const traits = object.traits.map((applied) => applied.trait === expiring ? {
			trait: applied.trait,
			options: rules
		} : applied);
		return object.with({
			expiring: rules,
			traits
		});
	},
	async expire(server, now) {
		let removed = 0;
		for (const object of server.objects) for (const rule of object.expiring ?? []) for (let batch = EXPIRE_ROWS; batch === EXPIRE_ROWS;) {
			const rows = await expired$1(server.database, object, rule, now);
			if (rows.length > 0) await server.executeAsSystem(object, "expire", rows.map((row) => SystemCall.of(row)), now);
			batch = rows.length;
			removed += batch;
		}
		return removed;
	},
	controller(server) {
		const types = server.objects.filter((object) => object.expiring !== void 0);
		return {
			name: "expiry",
			watches: types.map((object) => object.table),
			keys: (change) => {
				const object = types.find((type) => type.table === change.table);
				const after = change.after;
				const before = change.before;
				return (object?.expiring ?? []).some((rule) => after?.[rule.from] !== void 0 && after[rule.from] !== null && before?.[rule.from] !== after[rule.from]) ? ["expiry"] : [];
			},
			list: async () => ["expiry"],
			reconcile: async () => {
				const now = Date.now();
				await expiring.expire(server, now);
				const next = earliest(await Promise.all(types.flatMap((object) => object.expiring.map((rule) => passing(server.database, object, rule)))));
				return next === void 0 ? void 0 : Math.max(0, next - now);
			}
		};
	}
};
/** Remove an expired object at any revision, bypassing the trash. */
var expiry = defineMethod({
	kind: "delete",
	permission: null,
	isSystem: true,
	mutates: true,
	target: true,
	result: "value",
	procedure: () => ({
		route: {
			method: "DELETE",
			path: "/{id}"
		},
		input: Empty,
		output: Empty
	}),
	effect: async (call) => {
		await call.remove();
		return {};
	}
});
/** Require at least one valid rule. */
function requireRules(object, rules) {
	if (rules.length === 0) throw new TypeError(`object ${object.name} expires by no rule`);
	const columns = object.table[TABLE].columns;
	for (const rule of rules) {
		Duration.require(rule.after, `expiry window of ${object.name}`);
		if (!Object.hasOwn(columns, rule.from)) throw new TypeError(`object ${object.name} expires from unknown field ${rule.from}`);
	}
}
/** Read a batch of a type's rows a rule's window released. */
async function expired$1(database, object, rule, now) {
	const table = object.table;
	return await database.select().from(table).where(and(lte(table[rule.from], now - Duration.milliseconds(rule.after)), matching(table, rule))).limit(EXPIRE_ROWS);
}
/** Read when a rule's window next passes for a type. */
async function passing(database, object, rule) {
	const table = object.table;
	const [row] = await database.select({ from: min(table[rule.from]) }).from(table).where(and(isNotNull(table[rule.from]), matching(table, rule)));
	const from = row?.from;
	return from == null ? void 0 : from + Duration.milliseconds(rule.after);
}
/** Render a rule's condition on its table. */
function matching(table, rule) {
	return rule.where === void 0 ? void 0 : Condition.render(rule.where, Condition.bind(table));
}
var __destackModule$25 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} });
/** The longest delivery, in milliseconds. */
var DELIVERY_TIMEOUT_MILLISECONDS = 3e4;
/** The messages waiting for their destination. */
var outbox = defineTable("outbox", {
	/** The message key. */
	id: text$1("id").primaryKey().notNull(),
	/** The destination's name. */
	destination: text$1("destination").notNull(),
	/** The message as canonical JSON. */
	message: text$1("message").notNull(),
	/** The commit time. */
	recordedAt: integer("recorded_at").notNull()
}, { constraints: (entry) => [index("outbox_order").on(entry.destination, entry.recordedAt, entry.id)] }, __destackModule$25);
/** Messages sent with their transactions and delivered at least once, in order. */
var Outbox = class {
	/** The database holding the outbox. */
	database;
	/** Bind the outbox to its database. */
	constructor(database) {
		this.database = database;
	}
	/** Append a message inside a transaction or in its own, once per key. */
	async append(destination, id, message, transaction) {
		if (transaction !== void 0) {
			if (!transaction.driver.transaction || transaction.state !== this.database.state) throw new TypeError("outbox appends need a transaction on the outbox's database");
			await this.#insert(destination, id, message, transaction);
		} else await this.database.transaction((own) => this.#insert(destination, id, message, own));
	}
	/** Read a destination's pending messages, oldest first. */
	async read(destination, limit) {
		return (await this.#pending(destination, limit)).map((row) => row.message);
	}
	/** Deliver a destination's oldest pending messages as one batch, returning how many. */
	async deliver(destination, signal) {
		const rows = await this.#pending(destination, destination.batch);
		if (rows.length === 0) return 0;
		const timeout = AbortSignal.timeout(DELIVERY_TIMEOUT_MILLISECONDS);
		await destination.accept(rows.map((row) => row.message), { signal: signal === void 0 ? timeout : AbortSignal.any([signal, timeout]) });
		const keys = Condition.oneOf("id", rows.map((row) => row.id));
		await this.database.delete(outbox).where(Condition.render(keys, Condition.bind(outbox)));
		return rows.length;
	}
	/** Deliver a destination's messages as they commit. */
	controller(destination) {
		return {
			name: destination.name,
			watches: [outbox],
			keys: (change) => change.after?.destination === destination.name ? [destination.name] : [],
			list: async () => (await this.#pending(destination, 1)).length > 0 ? [destination.name] : [],
			reconcile: async () => {
				return await this.deliver(destination) === destination.batch ? 0 : void 0;
			}
		};
	}
	/** Read a destination's pending messages, oldest first. */
	async #pending(destination, limit) {
		return (await this.database.select({
			id: outbox.id,
			message: outbox.message
		}).from(outbox).where(eq(outbox.destination, destination.name)).orderBy(asc(outbox.recordedAt), asc(outbox.id)).limit(limit)).map((row) => ({
			id: row.id,
			message: destination.message.parse(JSON.parse(row.message))
		}));
	}
	/** Insert a message once per key. */
	async #insert(destination, id, message, transaction) {
		const content = canonicalize(destination.message.parse(message));
		if ((await transaction.insert(outbox).values({
			id,
			destination: destination.name,
			message: content,
			recordedAt: Date.now()
		}).onConflictDoNothing().returning({ id: outbox.id })).length > 0) return;
		const [held] = await transaction.select({ message: outbox.message }).from(outbox).where(and(eq(outbox.id, id), eq(outbox.destination, destination.name)));
		if (held === void 0 || held.message !== content) throw new ORPCError("CONFLICT", { message: `${destination.name} message ${id} has conflicting contents` });
	}
};
var __destackModule$24 = Object.freeze({ "package": {
	"id": "package-01a0d462-1b26-7401-be27-6258e0a32141",
	"name": "@destack/object",
	"version": "2026.9.0"
} });
/** The key under which access changes reconsider every row of an addressed type. */
var ACCESS_KEY = "*";
/** The revision of each addressed row its recipient's home holds. */
var copy = defineTable("copy", {
	/** The addressed object type, by name. */
	type: text$1("type").notNull(),
	/** The addressed row. */
	id: text$1("id").notNull(),
	/** The space holding the addressed row. */
	space: text$1("space").notNull(),
	/** The recipient's subject key. */
	recipient: text$1("recipient").notNull(),
	/** The revision sent to the recipient's home, null once withdrawn. */
	revision: integer("revision")
}, { constraints: (held) => [primaryKey({
	name: "copy_key",
	columns: [held.type, held.id]
})] }, __destackModule$24);
/** The outbox address of recipients' homes. */
var INBOX = {
	name: "inbox",
	message: defineSchema(strictObject({
		/** The addressed object type, by name. */
		type: string().min(1),
		/** The space holding the row. */
		space: string().min(1),
		/** The row's identifier. */
		id: string().min(1),
		/** The recipient's subject key. */
		recipient: string().min(1),
		/** The row as its columns encode it, null to withdraw the copy. */
		row: record$1(string(), json()).nullable()
	}))
};
/** Objects copied into their recipient's home while the recipient may read them. */
var addressed = {
	key: "addressed",
	isDurable: true,
	options: (definition) => definition.addressed,
	columns: () => ({
		/** The space holding the source row, null on the source row. */
		origin: text$1("origin"),
		/** The source row's identifier, null on the source row. */
		originId: text$1("origin_id")
	}),
	constraints: (_options, table, columns) => [uniqueIndex(`${table}_origin`).on(columns.origin, columns.originId)],
	policy: () => ({ attributes: { origin: "string" } }),
	methods: () => ({}),
	validate: (options, object) => {
		if (object.fields[options.recipient]?.type !== "subject") throw new TypeError(`object ${object.name} addresses no subject field ${options.recipient}`);
	},
	controller(server) {
		const types = server.objects.filter((object) => object.addressed !== void 0);
		const outbox = new Outbox(server.database);
		return {
			name: "addressed",
			watches: [
				...types.map((object) => object.table),
				accessRelationship,
				accessRole
			],
			keys: (change) => {
				const object = types.find((type) => type.table === change.table);
				const row = change.after ?? change.before;
				return object === void 0 ? types.map((type) => `${type.name} ${ACCESS_KEY}`) : row.origin === null || row.origin === void 0 ? [`${object.name} ${String(row.id)}`] : [];
			},
			list: async () => types.map((type) => `${type.name} ${ACCESS_KEY}`),
			reconcile: async (key) => {
				const [name, id] = key.split(" ");
				const object = types.find((type) => type.name === name);
				const table = object.table;
				const ids = id === ACCESS_KEY ? (await server.database.select({ id: table.id }).from(table).where(isNull(table.origin))).map((row) => String(row.id)) : [id];
				const untils = [];
				for (const each of ids) {
					const until = await send(server, outbox, object, each);
					if (until !== void 0) untils.push(until);
				}
				const next = untils.length === 0 ? void 0 : Math.min(...untils);
				return next === void 0 ? void 0 : Math.max(0, next - Date.now());
			}
		};
	},
	async accept(server, copies, home) {
		await server.database.transaction(async (transaction) => {
			for (const entry of copies) {
				const object = server.objects.find((type) => type.name === entry.type);
				if (object?.addressed === void 0) throw new TypeError(`object ${entry.type} is not addressed here`);
				const table = object.table;
				const scope = await home(keySubject(entry.recipient));
				const source = and(eq(table.origin, entry.space), eq(table.originId, entry.id));
				if (entry.row === null) {
					await transaction.delete(table).where(source);
					continue;
				}
				const [held] = await transaction.select({ id: table.id }).from(table).where(source);
				const { id: _id, ...row } = decodeRow(table, entry.row);
				const values = {
					...row,
					scope,
					origin: entry.space,
					originId: entry.id
				};
				if (held === void 0) await transaction.insert(table).values({
					...values,
					id: `${object.identity}-${v7()}`
				});
				else await transaction.update(table).set(values).where(eq(table.id, held.id));
			}
		});
	}
};
/** Send or withdraw one row's copy, returning when time next changes the recipient's access. */
async function send(server, outbox, object, id) {
	return server.database.transaction(async (transaction) => {
		const table = object.table;
		const [row] = await transaction.select().from(table).where(and(eq(table.id, id), isNull(table.origin)));
		const [held] = await transaction.select().from(copy).where(and(eq(copy.type, object.name), eq(copy.id, id)));
		if (row === void 0) {
			if (held !== void 0 && held.revision !== null) await withdraw(transaction, outbox, object, held, "gone");
			return;
		}
		const now = Date.now();
		const recipient = String(row[object.addressed.recipient]);
		const snapshot = Snapshot.live(transaction);
		const scope = String(row.scope);
		const access = await server.authorizer.resolveAssured(snapshot, scope, keySubject(recipient), now);
		const admission = await server.authorizer.checkRows(snapshot, object.permission("read"), access, [row]);
		const revision = Number(row.revision);
		if (admission.held.has(0) && held?.revision !== revision) {
			const [read] = await new Authorization(server.authorizer, transaction, () => access.context, access).redact(object, [row]);
			const message = {
				type: object.name,
				space: scope,
				id,
				recipient,
				row: encodeRow(table, read)
			};
			await outbox.append(INBOX, `${object.name}/${id}/${revision}`, message, transaction);
			await transaction.insert(copy).values({
				type: object.name,
				id,
				space: scope,
				recipient,
				revision
			}).onConflictDoUpdate({
				target: [copy.type, copy.id],
				set: {
					space: scope,
					recipient,
					revision
				}
			});
		} else if (!admission.held.has(0) && held !== void 0 && held.revision !== null) await withdraw(transaction, outbox, object, held, String(revision));
		return admission.until;
	});
}
/** Withdraw a recipient's copy through the outbox. */
async function withdraw(transaction, outbox, object, held, reason) {
	await outbox.append(INBOX, `${object.name}/${held.id}/withdrawn/${reason}`, {
		type: object.name,
		space: held.space,
		id: held.id,
		recipient: held.recipient,
		row: null
	}, transaction);
	await transaction.update(copy).set({ revision: null }).where(and(eq(copy.type, held.type), eq(copy.id, held.id)));
}
var __destackModule$23 = Object.freeze({ "package": {
	"id": "package-01a0d462-1b26-7401-be27-6258e0a32141",
	"name": "@destack/object",
	"version": "2026.9.0"
} });
/** The calls whose external work settles once. */
var settlement = defineTable("settlement", {
	/** The call's idempotency key. */
	id: text$1("id").primaryKey(),
	/** The object type the call acts on, by name. */
	object: text$1("object").notNull(),
	/** The method's name. */
	method: text$1("method").notNull(),
	/** The scope the call acts in. */
	scope: text$1("scope").notNull(),
	/** The object the call acts on or creates. */
	target: text$1("target"),
	/** The prepared value, null until the transaction commits. */
	prepared: json$1("prepared", strictObject({ value: json() })),
	/** When the call reserved the settlement, in UTC epoch milliseconds. */
	createdAt: integer("created_at").notNull(),
	/** When a settler last claimed it, null while unclaimed. */
	claimedAt: integer("claimed_at")
}, { log: {} }, __destackModule$23);
var __destackModule$22 = Object.freeze({ "package": {
	"id": "package-01a0d462-1b26-7401-be27-6258e0a32141",
	"name": "@destack/object",
	"version": "2026.9.0"
} });
/** The object call spans. */
var { span } = scope(__destackModule$22.package);
/** Build system calls. */
var SystemCall = { 
/** Call on an existing object's row, in its scope. */
of(row, input) {
	return {
		scope: String(row.scope),
		target: row,
		...input === void 0 ? {} : { input }
	};
} };
/** The most rows one purge removes per transaction: about a millisecond of deletes. */
var PURGE_ROWS = 100;
/** Declare the time a record's deletion was requested. */
function deletionColumns() {
	return { 
	/** The time deletion was requested, null otherwise. */
deletionRequestedAt: integer("deletion_requested_at") };
}
/** Objects deleted to a trash, restorable within a window and purged after it. */
var recoverable = {
	key: "recoverable",
	isDurable: true,
	options: (definition) => definition.recoverable,
	columns: (options) => ({
		...deletionColumns(),
		...options.keep === "record" ? { 
		/** The time a purge destroyed the kept record's content, null before. */
purgedAt: integer("purged_at") } : {}
	}),
	constraints: () => [],
	methods: (options) => ({
		delete: remove(options.by),
		restore: restoration$2("restore", options.by, true),
		purge: restoration$2("purge", options.purge ?? options.by, options.keep !== "record"),
		...options.keep === "record" ? { discard } : {}
	}),
	validate: (options, object, definition) => {
		if (definition.controlled) throw new TypeError(`object ${object.name} is controlled, so its controller finishes its deletion`);
		Duration.require(options.within, `recovery window of ${object.name}`);
		if (options.keep !== void 0 && options.keep !== "record") throw new TypeError(`purges of ${object.name} keep the record or nothing`);
	},
	within(object, within) {
		const declared = object.recoverable;
		if (declared === void 0) throw new TypeError(`object ${object.name} is not recoverable`);
		Duration.require(within, `recovery window of ${object.name}`);
		const options = {
			...declared,
			within
		};
		const traits = object.traits.map((applied) => applied.trait === recoverable ? {
			trait: applied.trait,
			options
		} : applied);
		return object.with({
			recoverable: options,
			traits
		});
	},
	async purge(server, now) {
		let purged = 0;
		for (const object of server.objects) for (let batch = PURGE_ROWS; object.recoverable !== void 0 && batch === PURGE_ROWS;) {
			const rows = await expired(server.database, object, now);
			if (rows.length > 0) await server.executeAsSystem(object, purgeMethod(object), rows.map((row) => SystemCall.of(row)), now);
			batch = rows.length;
			purged += batch;
		}
		return purged;
	},
	controller(server) {
		const types = server.objects.filter((object) => object.recoverable !== void 0);
		return {
			name: "purge",
			watches: types.map((object) => object.table),
			keys: (change) => {
				const after = change.after;
				const before = change.before;
				const deleted = after?.deletionRequestedAt ?? null;
				return deleted !== null && before?.deletionRequestedAt !== deleted ? ["trash"] : [];
			},
			list: async () => ["trash"],
			reconcile: async () => {
				const now = Date.now();
				await recoverable.purge(server, now);
				const next = earliest(await Promise.all(types.map((object) => ending(server.database, object))));
				return next === void 0 ? void 0 : Math.max(0, next - now);
			}
		};
	}
};
/** Read a batch of a type's expired, unpurged deleted rows. */
async function expired(database, object, now) {
	const table = object.table;
	const window = Duration.milliseconds(object.recoverable.within);
	const isKept = object.recoverable.keep === "record";
	return await database.select().from(table).where(and(lte(table.deletionRequestedAt, now - window), isKept ? isNull(table.purgedAt) : void 0)).limit(PURGE_ROWS);
}
/** Read when a type's earliest pending deletion leaves its window. */
async function ending(database, object) {
	const table = object.table;
	const isKept = object.recoverable.keep === "record";
	const [row] = await database.select({ requestedAt: min(table.deletionRequestedAt) }).from(table).where(and(isNotNull(table.deletionRequestedAt), isKept ? isNull(table.purgedAt) : void 0));
	const requestedAt = row?.requestedAt;
	return requestedAt == null ? void 0 : requestedAt + Duration.milliseconds(object.recoverable.within);
}
/** Name the purge method a recoverable type takes. */
function purgeMethod(object) {
	return Object.entries(object.methods).find(([, declared]) => declared.kind === "purge")[0];
}
/** Restore a deleted object, or purge it for good. */
function restoration$2(kind, permission, isPredicted) {
	const isRestore = kind === "restore";
	return defineMethod({
		kind,
		permission,
		mutates: true,
		isPredicted,
		target: true,
		result: isRestore ? "object" : "value",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: `/{id}/${kind}`
			},
			input: shapes.target.extend(shapes.replay),
			output: isRestore ? shapes.row : Empty
		}),
		effect: isRestore ? restore : purge,
		...isRestore ? { inverse: (step) => {
			const call = Step.call(step, "delete", Step.target(step, step.input.id));
			return call === void 0 ? void 0 : [call];
		} } : { async execute(call) {
			if (call.object.recoverable.keep === "record" && this.effect === purge) throw new TypeError(`object ${call.object.name} keeps purged records, so a purge handler of its own destroys their content`);
			return this.effect(call);
		} }
	});
}
/** Clear a deletion that is still recoverable. */
async function restore(call) {
	const { object } = call;
	const target = call.target;
	const requested = target.deletionRequestedAt;
	if (requested === null) throw new ORPCError("CONFLICT", { message: `${object.name} is not deleted` });
	else if (isPurged(target)) throw new ORPCError("CONFLICT", { message: `${object.name} is purged` });
	else if (call.now - requested >= Duration.milliseconds(object.recoverable.within)) throw new ORPCError("CONFLICT", { message: `${object.name} is past its recovery window` });
	return call.revise({ deletionRequestedAt: null });
}
/** Purge an object in the trash: mark a kept record purged, else remove it. */
async function purge(call) {
	const { object } = call;
	const target = call.target;
	if (target.deletionRequestedAt === null) throw new ORPCError("CONFLICT", { message: `${object.name} is not deleted` });
	else if (isPurged(target)) throw new ORPCError("CONFLICT", { message: `${object.name} is purged` });
	if (object.recoverable.keep === "record") await call.revise({ purgedAt: call.now });
	else await call.remove();
	return {};
}
/** Remove a purged record the type keeps, at the loaded revision. */
var discard = method({
	permission: null,
	isSystem: true,
	output: Empty
}).handle(async (call) => {
	if (!isPurged(call.target)) throw new ORPCError("CONFLICT", { message: `${call.object.name} is not purged` });
	await call.remove();
	return {};
});
/** Decide whether a purge already destroyed a kept record's content. */
function isPurged(row) {
	return row.purgedAt !== void 0 && row.purgedAt !== null;
}
/** A controller's observation of a record. */
var StatusCondition = defineSchema(strictObject({
	/** Whether the condition holds, or has not been established. */
	status: _enum([
		"true",
		"false",
		"unknown"
	]),
	/** The desired generation the controller evaluated. */
	observedGeneration: number().int().min(1),
	/** The machine-readable explanation. */
	reason: string().min(1),
	/** The human-readable explanation. */
	message: string(),
	/** The last change of condition status, in UTC epoch milliseconds. */
	lastTransitionAt: number().int().min(0)
}));
/** Status conditions keyed by their unique domain-specific names. */
var ConditionMap = defineSchema(record$1(string().min(1), StatusCondition));
/** Record a controller's observation of its target at its generation. */
var observe = method({
	permission: null,
	isSystem: true,
	input: defineSchema(strictObject({
		/** The desired generation the controller evaluated. */
		observedGeneration: number().int().min(0),
		/** The conditions by name, their transition times kept while their status holds. */
		conditions: record$1(string().min(1), StatusCondition.omit({
			observedGeneration: true,
			lastTransitionAt: true
		})),
		/** The observed fields the controller writes. */
		fields: record$1(string(), json()).optional()
	}))
}).handle(async (call) => {
	const target = call.target;
	const { observedGeneration, conditions, fields } = call.input;
	const merged = Object.fromEntries(Object.entries(conditions).map(([name, condition]) => [name, controlled.observe(target.conditions[name], {
		...condition,
		observedGeneration
	}, call.now)]));
	return call.observe({
		...decodeRow(call.object.table, fields ?? {}),
		observedGeneration,
		conditions: {
			...target.conditions,
			...merged
		}
	});
});
/** Remove a record with a requested deletion after its controller finishes. */
var finalize = method({
	permission: null,
	isSystem: true,
	output: strictObject({})
}).handle(async (call) => {
	if (call.target.deletionRequestedAt === null) throw new ORPCError("CONFLICT", { message: `${call.object.name} is not being deleted` });
	await call.remove();
	return {};
});
/** Declare the desired generation and a controller's observations. */
function controlledColumns() {
	return {
		/** The desired state's generation. */
		generation: integer("generation").notNull().default(1),
		/** The latest generation the controller evaluated. */
		observedGeneration: integer("observed_generation").notNull().default(0),
		/** The conditions the controller reports. */
		conditions: json$1("conditions", ConditionMap).notNull().default(sql`'{}'`)
	};
}
/** Records a controller reconciles. */
var controlled = {
	key: "controlled",
	isDurable: true,
	options: (definition) => definition.controlled ? true : void 0,
	columns: () => ({
		...controlledColumns(),
		...deletionColumns()
	}),
	constraints: (_options, table, columns) => controlledChecks(table, columns),
	methods: () => ({
		observe,
		finalize
	}),
	observe: (previous, observation, now) => ({
		...observation,
		lastTransitionAt: previous?.status === observation.status ? previous.lastTransitionAt : now
	})
};
/** Require valid revisions and generations, and refuse observations of future generations. */
function controlledChecks(name, columns) {
	return [
		check(`${name}_revision`, sql`${columns.revision} >= 1`),
		check(`${name}_generation`, sql`${columns.generation} >= 1`),
		check(`${name}_observed_generation`, sql`${columns.observedGeneration} BETWEEN 0 AND ${columns.generation}`)
	];
}
/** Transitions of state fields. */
var transitions = {
	options: (definition) => {
		const fields = Object.entries(definition.fields ?? {}).flatMap(([name, declared]) => declared.machine === void 0 ? [] : [[name, declared.machine]]);
		return fields.length === 0 ? void 0 : fields;
	},
	columns: () => ({}),
	constraints: () => [],
	methods: (fields) => {
		const methods = {};
		for (const [field, machine] of fields) for (const [name, transition] of Object.entries(machine.transitions)) {
			if (Object.hasOwn(methods, name)) throw new TypeError(`transition ${name} is declared by more than one state field`);
			methods[name] = transitionMethod(field, name, transition);
		}
		return methods;
	}
};
/** Change one state field through one transition. */
function transitionMethod(field, name, transition) {
	return defineMethod({
		kind: "transition",
		permission: transition.permission,
		mutates: true,
		transition: {
			field,
			from: transition.from,
			to: transition.to
		},
		inverse: (step) => {
			const left = step.before?.[field];
			const back = Object.entries(step.object.methods).find(([, declared]) => declared.transition?.field === field && declared.transition.to === left && declared.transition.from.includes(transition.to));
			return back === void 0 ? void 0 : [Step.record(step, back[0], Step.target(step, step.input.id))];
		},
		target: true,
		result: "object",
		procedure: (method, shapes) => ({
			route: {
				method: "POST",
				path: `/{id}/${kebabCase(method)}`
			},
			input: shapes.target.extend(shapes.replay),
			output: shapes.row
		}),
		effect: (call) => call.revise({ [field]: transition.to }),
		async execute(call) {
			const state = call.target[field];
			if (!transition.from.includes(state)) throw new ORPCError("CONFLICT", { message: `${call.object.name} cannot ${name} while ${field} is ${state}` });
			return this.effect(call);
		}
	});
}
/** The fields relating a subject to an object through exactly one relation or role. */
var GrantShape = {
	/** The declared relation to grant. */
	relation: AccessName.optional(),
	/** The role to bind. */
	role: string().min(1).optional(),
	/** The subject, subject set or wildcard. */
	subject: Subject,
	/** Optional expiry in UTC epoch milliseconds. */
	expiresAt: number().int().optional(),
	/** What a request must satisfy for the relationship to apply. */
	conditions: RelationshipCondition.optional()
};
/** A relationship an object's sharing methods grant. */
var GrantInput = strictObject(GrantShape);
/** The fields proposing a relationship to exactly one subject or recipient. */
var ProposeShape = {
	/** The relationship to propose. */
	relationship: strictObject({
		...GrantShape,
		subject: Subject.optional()
	}),
	/** The identifier whose owner may accept an offer, such as `email:bob@acme.com`. */
	recipient: VerifiedIdentifier.optional(),
	/** Why the caller asks for or offers the relationship. */
	purpose: string().min(1).max(1e3).optional(),
	/** When the proposal lapses in UTC epoch milliseconds, a week from now by default. */
	expiresAt: number().int().optional()
};
/** A relationship an object's sharing methods propose. */
var ProposeInput = strictObject(ProposeShape);
/** The field selecting one relationship. */
var RelationshipShape = { 
/** The relationship's identifier. */
relationshipId: string().min(1) };
/** The fields selecting what an explanation covers. */
var ExplainShape = {
	/** The permission to explain, by default reading. */
	permission: AccessName.optional(),
	/** The subject with the access to explain, by default the caller. */
	subject: Subject.optional()
};
/** The field selecting one proposal. */
var ProposalShape = { 
/** The proposal's identifier. */
proposalId: string().min(1) };
/** Objects callers share through relationships and proposals. */
var shareable = {
	key: "shareable",
	isDurable: true,
	options: (definition) => definition.shareable?.by,
	columns: () => ({}),
	constraints: () => [],
	methods: (grant) => ({
		relationships: sharingMethod("relationships", grant, {
			method: "GET",
			path: "/{id}/relationships"
		}, (shapes) => shapes.target.extend(PageShape), page(Relationship.schema), async (call) => {
			const listing = sharingPage(call, "relationships");
			const relationships = await call.authorization.authorizer.relationships(call.authorization.snapshot, call.reference(), {
				...listing.after === void 0 ? {} : { after: listing.after },
				limit: listing.limit + 1
			});
			return listing.result(relationships, (relationship) => relationship.id);
		}),
		grant: sharingMethod("grant", null, {
			method: "POST",
			path: "/{id}/relationships"
		}, (shapes) => shapes.target.extend({
			...shapes.replay,
			...GrantShape
		}), Relationship.schema, async (call) => {
			await requirePresent(call, "live");
			return call.authorization.grant({
				...GrantInput.parse(call.input),
				object: call.reference()
			});
		}, (step) => {
			const granted = step.result?.id;
			const call = granted === void 0 ? void 0 : Step.call(step, "revoke", {
				...Step.target(step, step.input.id),
				relationshipId: granted
			});
			return call === void 0 ? void 0 : [call];
		}),
		revoke: sharingMethod("revoke", null, {
			method: "DELETE",
			path: "/{id}/relationships/{relationshipId}"
		}, (shapes) => shapes.target.extend({
			...shapes.replay,
			...RelationshipShape
		}), Empty, async (call) => {
			await requirePresent(call, "held");
			await call.authorization.revoke(call.reference(), string().parse(call.input.relationshipId));
			return {};
		}),
		proposals: sharingMethod("proposals", grant, {
			method: "GET",
			path: "/{id}/proposals"
		}, (shapes) => shapes.target.extend(PageShape), page(Proposal.schema), async (call) => {
			const listing = sharingPage(call, "proposals");
			const proposals = await call.authorization.proposals({ object: call.reference() }, {
				...listing.after === void 0 ? {} : { after: listing.after },
				limit: listing.limit + 1
			});
			return listing.result(proposals, (proposal) => proposal.id);
		}),
		propose: sharingMethod("propose", null, {
			method: "POST",
			path: "/{id}/proposals"
		}, (shapes) => shapes.target.extend({
			...shapes.replay,
			...ProposeShape
		}), Proposal.schema, async (call) => {
			await requirePresent(call, "live");
			const { relationship, ...proposal } = ProposeInput.parse(call.input);
			return call.authorization.propose({
				...proposal,
				relationship: {
					...relationship,
					object: call.reference()
				}
			});
		}),
		accept: sharingMethod("accept", null, {
			method: "POST",
			path: "/{id}/proposals/{proposalId}/accept"
		}, (shapes) => shapes.target.extend({
			...shapes.replay,
			...ProposalShape
		}), Relationship.schema, async (call) => {
			await requirePresent(call, "live");
			return call.authorization.accept(call.reference(), string().parse(call.input.proposalId));
		}),
		explain: defineMethod({
			kind: "explain",
			permission: null,
			mutates: false,
			target: false,
			result: "value",
			isPredicted: false,
			procedure: (_name, shapes) => ({
				route: {
					method: "GET",
					path: "/{id}/access"
				},
				input: shapes.target.extend(ExplainShape),
				output: Explanation
			}),
			effect: explain
		}),
		decline: sharingMethod("decline", null, {
			method: "DELETE",
			path: "/{id}/proposals/{proposalId}"
		}, (shapes) => shapes.target.extend({
			...shapes.replay,
			...ProposalShape
		}), Empty, async (call) => {
			await requirePresent(call, "held");
			await call.authorization.decline(call.reference(), string().parse(call.input.proposalId));
			return {};
		})
	})
};
/** Declare a sharing method. */
function sharingMethod(kind, permission, route, input, output, effect, inverse) {
	return defineMethod({
		kind,
		permission,
		mutates: permission === null,
		target: permission !== null,
		result: "value",
		isPredicted: false,
		procedure: (_name, shapes) => ({
			route,
			input: input(shapes),
			output
		}),
		effect,
		...inverse === void 0 ? {} : { inverse }
	});
}
/** Explain a subject's permission on an object. */
async function explain(call) {
	const { authorizer, snapshot } = call.authorization;
	const { permission, subject } = strictObject(ExplainShape).parse(call.input);
	const object = call.object;
	const target = call.reference();
	const scope = authorizer.governingScope(target);
	const access = await call.authorization.in(scope);
	if (subject !== void 0) {
		const grant = object.policy.definition.grantedBy;
		if (grant === void 0 || !(await authorizer.check(snapshot, object.permission(grant), target, access)).isAllowed) throw new ORPCError("FORBIDDEN", { message: `explaining access to ${object.name} needs ${grant}` });
		const explained = await authorizer.resolve(snapshot, scope, {
			subjects: [subject],
			now: call.now,
			attributes: {}
		});
		return authorizer.explain(snapshot, permission === void 0 ? requireReading(object) : object.permission(permission), target, explained);
	}
	if (!(await authorizer.check(snapshot, requireReading(object), target, access)).isAllowed) throw new ORPCError("NOT_FOUND", { message: `no ${object.name} ${target.id}` });
	return authorizer.explain(snapshot, permission === void 0 ? requireReading(object) : object.permission(permission), target, access);
}
/** Read the permission reading an object of a type needs. */
function requireReading(object) {
	if (object.reading === void 0) throw new ORPCError("BAD_REQUEST", { message: `${object.name} has no objects to read` });
	return object.reading;
}
/** Require the named object to exist, outside the trash when live. */
async function requirePresent(call, state) {
	const table = call.object.table;
	const [row] = await call.database.select().from(table).where(and(eq(table.id, call.id), call.object.inScope(call.scope)));
	if (row === void 0) throw new ORPCError("NOT_FOUND", { message: `no ${call.object.name} ${call.id}` });
	else if (state === "live" && row.deletionRequestedAt !== void 0 && row.deletionRequestedAt !== null) throw new ORPCError("CONFLICT", { message: `${call.object.name} is in the trash` });
}
/** Read the page a sharing listing asks for, bound to its object. */
function sharingPage(call, list) {
	return new Page(call.input, [
		call.object.name,
		call.scope,
		call.id,
		list
	], string());
}
/** Scope objects that can be suspended. */
var suspendable = {
	key: "suspendable",
	isDurable: true,
	options: (definition) => definition.suspendable,
	columns: () => ({}),
	constraints: () => [],
	methods: (options) => ({
		suspend: suspendable.suspend(options.by),
		resume: suspendable.resume(options.by)
	}),
	validate: (_options, object, definition) => {
		if (definition.isScope !== true) throw new TypeError(`object ${object.name} is suspendable but no scope`);
	},
	suspend: (permission) => change(permission, "suspend", "resume"),
	resume: (permission) => change(permission, "resume", "suspend")
};
/** Suspend or resume a scope object on the server. */
function change(permission, change, inverse) {
	return {
		...custom({
			permission,
			inverse
		}).handle(async (call) => {
			await call.served()[change](call.reference());
			return call.target;
		}),
		isPredicted: false
	};
}
/** Objects taking attachments of other types. */
var attachments = {
	key: "attachments",
	isDurable: true,
	options: (definition) => definition.attachments === void 0 || definition.attachments.length === 0 ? void 0 : definition.attachments,
	columns: () => ({}),
	constraints: () => [],
	table: (options, object) => {
		const hosted = {
			parentPackageId: object.packageId,
			parentType: object.name
		};
		const durable = options.filter((attachment) => attachment.object.storage === "durable");
		return {
			aggregates: durable.flatMap((attachment) => Object.entries(attachment.object.aggregates).filter(([, aggregate]) => aggregate.via === void 0).map(([name, aggregate]) => ({
				from: () => attachment.object.table,
				column: name,
				key: "parentId",
				function: aggregate.function,
				...aggregate.value === void 0 ? {} : { value: aggregate.value },
				where: {
					...aggregate.where,
					...hosted
				}
			}))),
			dependents: durable.map((attachment) => ({
				from: () => attachment.object.table,
				key: "parentId",
				where: hosted,
				onDelete: attachment.object.parent.delete
			}))
		};
	},
	policy: (options, object, permissions) => {
		const attaching = {};
		for (const attachment of options) {
			const receive = attachment.object.parent.receive;
			const plural = attachment.object.plural;
			if (!permissions.includes(attachment.by)) throw new TypeError(`object ${object.name} attaches ${plural} by missing permission ${attachment.by}`);
			else if (permissions.includes(receive)) {
				if (attachment.by !== receive) throw new TypeError(`object ${object.name} declares permission ${receive}, so it attaches ${plural} by ${receive}`);
			} else if (Object.hasOwn(attaching, receive)) throw new TypeError(`object ${object.name} already derives permission ${receive}, which attaching ${plural} derives`);
			else attaching[receive] = permission(attachment.by);
		}
		return {
			permissions: attaching,
			contributes: options.map((attachment) => ({
				policy: attachment.object.policy,
				relation: "parent"
			}))
		};
	},
	methods: () => ({}),
	validate: (options, object) => {
		for (const attachment of options) for (const [name, aggregate] of Object.entries(attachment.object.aggregates)) if (aggregate.via === void 0) object.requireAggregate(attachment.object, name, aggregate);
	}
};
/** Text fields held in chunks and changed only by edits. */
var text = {
	options: (definition) => {
		const fields = Object.entries(definition.fields ?? {}).filter(([, declared]) => declared.type === "text").map(([name]) => name);
		const methods = Object.values(definition.methods ?? {});
		const permissionOf = (kind) => methods.find((declared) => declared.kind === kind)?.permission ?? void 0;
		return fields.length === 0 ? void 0 : {
			fields,
			permission: permissionOf("update"),
			reading: permissionOf("list") ?? permissionOf("get")
		};
	},
	columns: () => ({}),
	constraints: () => [],
	table: (_options, object) => ({ dependents: [{
		from: () => chunk,
		key: "parentId",
		where: {
			parentPackageId: object.packageId,
			parentType: object.name
		},
		onDelete: "cascade"
	}] }),
	policy: (options, object, permissions) => {
		if (permissions.includes("text")) throw new TypeError(`object ${object.name} declares permission ${TEXT_READ}, which its text fields derive`);
		return options.reading === void 0 ? {} : { permissions: { [TEXT_READ]: permission(options.reading) } };
	},
	methods: (options) => options.permission === void 0 ? {} : { edit: edit(options.permission, options.fields) },
	validate: (options, object) => {
		if (object.storage === "ephemeral") throw new TypeError(`ephemeral object ${object.name} holds no text`);
		else if (options.permission === void 0) throw new TypeError(`object ${object.name} holds text but no update method to edit it`);
		else if (options.reading === void 0) throw new TypeError(`object ${object.name} holds text but no get or list method reads it`);
		for (const [name, declared] of Object.entries(object.methods)) {
			const written = declared.fields?.find((field) => options.fields.includes(field));
			if (written !== void 0) throw new TypeError(`method ${name} of ${object.name} writes text field ${written}, which only edit changes`);
		}
	}
};
/** Edit one text field of an object. */
function edit(permission, fields) {
	return defineMethod({
		kind: "edit",
		permission,
		mutates: true,
		target: true,
		result: "value",
		procedure: (_name, shapes) => ({
			route: {
				method: "POST",
				path: "/{id}/edit"
			},
			input: shapes.target.extend({
				...shapes.replay,
				field: _enum(fields),
				edits: array(SequenceEdit).min(1)
			}),
			output: Edited
		}),
		effect: (call) => Chunk.edit(call),
		inverse: (step) => {
			const result = step.result;
			if (result === void 0) return;
			return result.inverse.length === 0 ? [] : [Step.record(step, step.name, {
				...Step.target(step, step.input.id),
				field: step.input.field,
				edits: result.inverse
			})];
		},
		async execute(call) {
			const guard = call.object.fields[String(call.input.field)].access?.write;
			if (!call.isPredicted && guard !== void 0) {
				if (!(await call.served().check(call.object.permission(guard), call.reference())).isAllowed) throw new ORPCError("FORBIDDEN", { message: `field ${String(call.input.field)} is not writable` });
			}
			return this.effect(call);
		}
	});
}
/** The definition key holding an intrinsic object's table. */
var INTRINSIC = Symbol("intrinsic");
/** The tables every durable object type's database holds beside its own: access, copies, settlements, controller leases and the outbox. */
var serverTables = [
	...accessTables,
	settlement,
	controllerLease,
	outbox
];
/** Derive an object's table from its traits, scope and fields. */
function deriveTable(definition, applied, packageId, module) {
	const name = definition.name;
	const scoped = { scope: definition.scope === Scope.universe.id ? text$1("scope").notNull().default(Scope.universe.id) : Array.isArray(definition.scope) ? text$1("scope").notNull() : identifier$1("scope", definition.scope.identity).notNull() };
	let derived;
	const identity = definition.identity ?? name;
	const object = {
		name,
		identity,
		packageId,
		table: () => derived,
		storage: definition.storage ?? "durable"
	};
	const [record, ...others] = applied;
	const fields = Object.fromEntries(Object.entries(definition.fields ?? {}).filter(([, declared]) => declared.type !== "text").map(([property, declared]) => [property, declared.column(snakeCase(property), identity, object.storage)]));
	const isAttachment = definition.nested?.in === "any";
	const kept = applied.map(({ trait, options }) => trait.table?.(options, object) ?? {});
	const tree = kept.find((table) => table.tree !== void 0)?.tree;
	const tableName = snakeCase(name);
	derived = defineTable(tableName, {
		...record.trait.columns(record.options, object),
		...scoped,
		...Object.assign({}, ...others.map(({ trait, options }) => trait.columns(options, object))),
		...fields
	}, {
		constraints: ((built) => [...applied.flatMap(({ trait, options }) => trait.constraints(options, tableName, built)), ...definition.constraints?.(built) ?? []]),
		aggregates: [...Object.entries(definition.aggregates ?? {}).filter(([, aggregate]) => !(aggregate.via === void 0 && isAttachment)).map(([name, aggregate]) => ({
			into: () => holderOf(definition, aggregate, derived),
			column: name,
			key: aggregate.via ?? "parentId",
			function: aggregate.function,
			...aggregate.value === void 0 ? {} : { value: aggregate.value },
			...aggregate.where === void 0 ? {} : { where: aggregate.where }
		})), ...kept.flatMap((table) => table.aggregates ?? [])],
		dependents: kept.flatMap((table) => table.dependents ?? []),
		moved: { columns: Object.fromEntries(Object.entries(definition.moved?.fields ?? {}).map(([field, previous]) => [field, snakeCase(previous)])) },
		...definition.convert === void 0 ? {} : { convert: definition.convert },
		...definition.tier === void 0 ? {} : { tier: definition.tier },
		log: definition.tracked === void 0 ? {} : { retention: "history" },
		...tree === void 0 ? {} : { tree }
	}, module);
	return derived;
}
/** Find the table holding an aggregate. */
function holderOf(definition, aggregate, own) {
	const field = aggregate.via === void 0 ? void 0 : definition.fields?.[aggregate.via];
	const parent = definition.nested?.in;
	const holder = field?.target?.() ?? (typeof parent === "string" ? void 0 : parent);
	if (aggregate.via !== void 0 && field?.target === void 0) throw new TypeError(`aggregate of ${definition.name} follows no reference ${aggregate.via}`);
	if (aggregate.via === void 0 && definition.nested === void 0) throw new TypeError(`aggregate of ${definition.name} needs a parent or a reference`);
	return holder?.table ?? own;
}
/** The default ephemeral linger, in milliseconds: 10 s spans two 1–3 s reconnects. */
var LINGER_MILLISECONDS = 1e4;
/** The method kinds writing records. */
var RECORD_KINDS = /* @__PURE__ */ new Set([
	"create",
	"update",
	"delete",
	"updateMany"
]);
/** The permission on a scope's own object that shows the scope. */
var SCOPE_READ = "read";
/** Every trait, in application order. */
var TRAITS = [
	record,
	nested,
	transitions,
	recoverable,
	expiring,
	addressed,
	versioned,
	controlled,
	declarable,
	detachable,
	shareable,
	suspendable,
	attachments,
	tracked,
	text
];
/** A declared object type with its storage, permissions, methods and declaration schema. */
var ObjectType = class ObjectType {
	/** The declaring package, supplied by the module transform. */
	package;
	/** The singular name. */
	name;
	/** The prefix of the objects' identifiers. */
	identity;
	/** The plural name. */
	plural;
	/** The table holding one record per object. */
	table;
	/** The scope type containing each object, or the types when objects live in several. */
	scope;
	/** The scope types containing the objects, none for objects outside every scope. */
	scopes;
	/** The object's relations and permissions, evaluated by access. */
	policy;
	/** The fields each object holds, for objects declared by fields. */
	fields;
	/** The fields needing their own read permission. */
	guarded;
	/** The fields callers write. */
	written;
	/** The columns holding sensitive values. */
	sensitive;
	/** The text fields, held in chunks. */
	text;
	/** Whether reads of the objects are audited. */
	isReadAudited;
	/** The rows the scopes inside each object's scope copy, for inherited objects. */
	inherited;
	/** The ancestor index of a tree of this object type. */
	tree;
	/** The tables a database holding the object needs, access tables included. */
	tables;
	/** The permission names callers may hold on the object. */
	permissions;
	/** The schema of one declaration in a stack, when stacks may declare the object. */
	declarationSchema;
	/** The operations callers may execute, keyed by method name. */
	methods;
	/** How deleted objects stay restorable, for recoverable objects. */
	recoverable;
	/** When the system removes the objects, for expiring objects. */
	expiring;
	/** The system controller reconciling the objects with work waiting. */
	controller;
	/** The relations of the objects to the scope they live in, by the name of the scope's type, that permissions read through. */
	enclosing;
	/** How a stack's declarations of the objects become their managed records. */
	declaration;
	/** How addressed objects name their recipient. */
	addressed;
	/** Whether the objects are numbered versions of their parent. */
	versioned;
	/** How tracked objects keep their history. */
	tracked;
	/** The parent of nested objects. */
	parent;
	/** The aggregates of these objects their holder keeps, by field name. */
	aggregates;
	/** The attachments the objects take. */
	attachments;
	/** The traits the objects take, with their options. */
	traits;
	/** The unique indexes the key index keeps across databases, by name. */
	indexes;
	/** Where the objects live. */
	storage;
	/** How long ephemeral objects outlive their session, in milliseconds. */
	linger;
	/** Where access finds objects stored in a table another package owns. */
	intrinsic;
	/** The previous names of renamed fields, by current field. */
	moved;
	/** The fields each release computes from stored rows and earlier callers' inputs. */
	convert;
	/** Retain a definition with its table, methods and traits, and assemble its policy. */
	constructor(owner, definition, traits, intrinsic) {
		if (definition.table[TABLE].retention === "none") throw new TypeError(`object table is not logged: ${definition.table[TABLE].name}`);
		this.package = owner;
		this.name = definition.name;
		this.identity = definition.identity ?? definition.name;
		this.plural = definition.plural;
		this.table = definition.table;
		this.scope = definition.scope;
		const scopes = [definition.scope].flat();
		this.scopes = scopes.filter((scope) => scope !== Scope.universe.id);
		this.declarationSchema = definition.declarable?.schema;
		this.recoverable = definition.recoverable;
		this.expiring = definition.expiring;
		this.controller = definition.controller;
		this.addressed = definition.addressed;
		this.versioned = definition.versioned;
		this.tracked = definition.tracked;
		for (const [name, method] of Object.entries(definition.methods ?? {})) Version.requireUpTo(method.convert ?? {}, owner.version, `${definition.name}.${name}`);
		this.moved = definition.moved?.fields ?? {};
		this.convert = definition.convert ?? {};
		for (const [field, previous] of Object.entries(this.moved)) if (previous in (definition.fields ?? {})) throw new TypeError(`field ${definition.name}.${field} moved from ${previous}, which names a current field`);
		Version.requireUpTo(this.convert, owner.version, definition.name);
		this.isReadAudited = definition.audited?.reads === true;
		this.inherited = definition.inherited;
		this.methods = definition.methods ?? {};
		this.parent = definition.nested && {
			object: definition.nested.in === "self" ? this : definition.nested.in,
			optional: definition.nested.optional ?? false,
			receive: definition.nested.receive,
			delete: definition.nested.delete ?? "cascade"
		};
		this.aggregates = definition.aggregates ?? {};
		this.attachments = definition.attachments ?? [];
		this.traits = traits;
		this.intrinsic = intrinsic;
		this.indexes = definition.indexes ?? {};
		this.storage = definition.storage ?? "durable";
		if (this.storage === "ephemeral") {
			if (definition.linger !== void 0) Duration.require(definition.linger, `linger of ${definition.name}`);
			this.linger = definition.linger === void 0 ? LINGER_MILLISECONDS : Duration.milliseconds(definition.linger);
		}
		const relations = {};
		for (const [name, relation] of Object.entries(definition.relations ?? {})) relations[name] = {
			subjects: relation.subjects.map((subject) => subject instanceof ObjectType ? subject.policy : typeof subject === "string" ? subjectType(owner.id, subject) : subject),
			...relation.grantedBy === void 0 ? {} : { grantedBy: relation.grantedBy }
		};
		const permissions = definition.permissions ?? [];
		const decided = new Set(Array.isArray(permissions) ? [] : Object.values(permissions).flatMap(relationsOf$1));
		for (const [property, field] of Object.entries(definition.fields ?? {})) {
			const name = kebabCase(property);
			if (!decided.has(name)) continue;
			if (field.type === "reference" && field.principals !== void 0) relations[name] = {
				subjects: field.principals,
				grantedBy: null
			};
			else if (field.type === "reference" && field.target && !field.qualified) relations[name] = {
				subjects: [field.target().policy],
				grantedBy: null
			};
			else if (field.type === "subject") relations[name] = {
				subjects: field.principals ?? [
					principal.user,
					principal.host,
					principal.installation
				],
				grantedBy: null
			};
		}
		const enclosing = this.scopes.filter((scope) => decided.has(scope.name) && relations[scope.name] === void 0);
		for (const scope of enclosing) {
			if (scope.scopes.length > 0) throw new TypeError(`object ${definition.name} reads through its scope ${scope.name}, which is no object of the universe`);
			relations[scope.name] = {
				subjects: [scope.policy],
				grantedBy: null
			};
		}
		this.enclosing = enclosing.map((scope) => scope.name);
		this.fields = definition.fields ?? {};
		this.text = Object.entries(this.fields).filter(([, declared]) => declared.type === "text").map(([name]) => name);
		const names = Array.isArray(permissions) ? permissions : Object.keys(permissions);
		const object = traitObject(this, (definition.represents?.package ?? owner).id);
		const policies = traits.flatMap(({ trait, options }) => trait.policy === void 0 ? [] : [trait.policy(options, object, names)]);
		const derived = Object.assign({}, ...policies.map((policy) => policy.permissions ?? {}));
		Object.assign(relations, ...policies.map((policy) => policy.relations ?? {}));
		this.permissions = [...names, ...Object.keys(derived)];
		const represented = definition.represents;
		if (represented && represented.name !== definition.name) throw new TypeError(`object ${definition.name} cannot represent ${represented.name}`);
		this.policy = new Policy(represented?.package ?? owner, {
			name: definition.name,
			attributes: Object.assign({}, ...policies.map((policy) => policy.attributes ?? {}), definition.attributes ?? {}),
			relations: {
				...representedRelations(represented),
				...relations
			},
			permissions: {
				...represented?.definition.permissions,
				...Array.isArray(permissions) ? Object.fromEntries(permissions.map((name) => [name, none()])) : permissions,
				...derived
			},
			...definition.shareable === void 0 ? {} : { grantedBy: definition.shareable.by },
			...definition.reserved === void 0 ? {} : { reserved: definition.reserved },
			...definition.elevated === void 0 ? {} : { elevated: definition.elevated },
			...definition.administration === void 0 ? {} : { administration: definition.administration },
			...definition.isScope || represented?.definition.scope === true ? { scope: true } : {},
			contributes: policies.flatMap((policy) => policy.contributes ?? [])
		});
		this.tree = this.table[TABLE].tree;
		this.tables = this.storage === "durable" ? [
			this.table,
			...serverTables,
			...this.addressed === void 0 ? [] : [copy],
			...this.text.length === 0 ? [] : [chunk, chunkRun]
		] : [this.table];
		const shadowed = Object.keys(this.methods).find((name) => Object.getOwnPropertyNames(Function.prototype).includes(name));
		if (shadowed !== void 0) throw new TypeError(`object ${this.name} names a method ${shadowed}, which functions hold`);
		for (const declared of Object.values(this.methods)) {
			if (declared.permission !== null && !this.permissions.includes(declared.permission)) throw new TypeError(`object ${definition.name} has no permission ${declared.permission}`);
			else if (declared.permission === null && RECORD_KINDS.has(declared.kind) && declared.isSystem !== true) throw new TypeError(`object ${definition.name} ${declared.kind}s without a permission outside the system`);
			declared.validate?.(this);
		}
		for (const [name, declared] of Object.entries(this.fields)) for (const permission of [declared.access?.read, declared.access?.write]) if (permission !== void 0 && !this.permissions.includes(permission)) throw new TypeError(`object ${definition.name} field ${name} has no permission ${permission}`);
		for (const [name, aggregate] of Object.entries(this.aggregates)) {
			const holding = aggregate.via === void 0 ? this.parent?.object : this.fields[aggregate.via]?.target?.();
			if (holding === void 0) throw new TypeError(`aggregate ${name} of ${definition.name} fills no ${aggregate.function} field of its holder`);
			else if (holding !== "any") holding.requireAggregate(this, name, aggregate);
		}
		for (const [name, declared] of Object.entries(this.methods)) {
			const shape = declared.input?.shape;
			for (const field of Object.keys(shape ?? {})) if (this.fields[field]?.aggregate !== void 0) throw new TypeError(`method ${name} of ${definition.name} writes aggregate field ${field}`);
		}
		this.written = Object.entries(this.fields).filter(([, declared]) => !declared.isCaller && declared.aggregate === void 0 && declared.machine === void 0 && declared.type !== "text").map(([name]) => name);
		this.sensitive = Object.entries(this.table[TABLE].columns).filter(([, column]) => column.definition.classification === "sensitive").map(([name]) => name);
		this.guarded = Object.entries(this.fields).filter(([, declared]) => declared.access?.read !== void 0).map(([name]) => name);
		for (const scope of this.scopes) if (scope.policy.definition.scope !== true) throw new TypeError(`object ${this.name} lives in ${scope.name}, which is no scope`);
		else if (!scope.permissions.includes("read")) throw new TypeError(`object ${this.name} lives in ${scope.name}, which declares no ${SCOPE_READ} permission`);
		const { logged } = this.table[TABLE];
		for (const [name, declared] of Object.entries(this.indexes)) {
			const missing = declared.on.find((field) => !Object.hasOwn(this.fields, field));
			const unlogged = declared.on.find((field) => !Object.hasOwn(logged, field));
			if (missing !== void 0) throw new TypeError(`index ${name} of ${this.name} names no field ${missing}`);
			else if (unlogged !== void 0) throw new TypeError(`index ${name} of ${this.name} names unlogged field ${unlogged}`);
			else if (declared.across !== Scope.universe.id && !this.ancestors.some((ancestor) => ancestor.same(declared.across))) throw new TypeError(`index ${name} of ${this.name} is unique within a scope enclosing its objects or across every scope`);
		}
		if (this.storage === "durable" && definition.linger !== void 0) throw new TypeError(`object ${this.name} lingers without being ephemeral`);
		else if (this.storage === "ephemeral") {
			const durable = [...traits.filter(({ trait }) => trait.isDurable).map(({ trait }) => trait.key), ...[
				"audited",
				"inherited",
				"controller",
				"isScope",
				"aggregates",
				"indexes"
			].filter((key) => definition[key] !== void 0 && definition[key] !== false)];
			if (durable.length > 0) throw new TypeError(`ephemeral object ${this.name} takes no ${durable[0]}`);
			const external = Object.entries(this.methods).find(([, declared]) => declared.prepare !== void 0 || declared.settle !== void 0);
			if (external !== void 0) throw new TypeError(`ephemeral object ${this.name} does no external work in method ${external[0]}`);
		}
		for (const { trait, options } of traits) trait.validate?.(options, this, definition);
	}
	/** The input conversions of a method: renamed fields, the object's conversions, then the method's own. */
	conversions(name) {
		const renames = Object.fromEntries(Object.entries(this.moved).map(([field, previous]) => [field, Expression$1.column(previous)]));
		const releases = Object.keys(renames).length === 0 ? {} : { [this.package.version]: renames };
		const method = this.methods[name];
		for (const conversions of [this.convert, method?.convert ?? {}]) for (const [release, assignments] of Object.entries(conversions)) releases[release] = {
			...releases[release],
			...assignments
		};
		return releases;
	}
	/** The audit target name of the objects' method events. */
	get auditTarget() {
		return camelCase(this.name);
	}
	/** Derive the audit action `Noun.method` recording one method. */
	audit(method, target = this.auditTarget) {
		const declared = this.methods[method]?.audit;
		return {
			package: this.package,
			name: `${pascalCase(this.name)}.${method}`,
			targets: strictObject({ [target]: AuditTarget }),
			details: declared === void 0 ? strictObject({}) : declared.details.partial()
		};
	}
	/** Build the audit action and values of a call: its object for a targeted method, else the scope's collection. */
	auditCall(method, input, scope) {
		if (this.methods[method]?.target === true) {
			const id = string().parse(input.id);
			const targets = { [this.auditTarget]: {
				type: this.name,
				id
			} };
			return {
				action: this.audit(method),
				values: {
					targets,
					details: {}
				}
			};
		}
		const targets = { collection: {
			type: this.plural,
			id: scope
		} };
		return {
			action: this.audit(method, "collection"),
			values: {
				targets,
				details: {}
			}
		};
	}
	/** Accept the members of one of this type's relations as subjects. */
	members(relation) {
		return this.policy.members(relation);
	}
	/** Accept every object of this type through one relationship. */
	all() {
		return this.policy.all();
	}
	/** Determine whether a subject is one object of this type. */
	is(subject) {
		return this.policy.is(subject);
	}
	/** Reference one object in its scope. */
	reference(scope, id) {
		return this.policy.reference(scope, id);
	}
	/** The table mapping access reads the objects through. */
	get mapping() {
		const inherited = this.inherited === void 0 ? {} : { inherited: this.inherited.where ?? Condition.all() };
		if (this.intrinsic !== void 0) return {
			...this.intrinsic,
			policy: this.policy,
			...inherited
		};
		const relations = {};
		for (const [property, field] of Object.entries(this.fields)) {
			const name = kebabCase(property);
			const isRelation = Object.hasOwn(this.policy.definition.relations, name);
			if (isRelation && field.type === "subject") relations[name] = {
				column: property,
				isKey: true
			};
			else if (isRelation && field.type === "reference" && field.principals !== void 0) relations[name] = {
				column: property,
				scope: Scope.universe.id
			};
			else if (isRelation && field.type === "reference" && field.target && !field.qualified) relations[name] = field.target().scope === Scope.universe.id ? {
				column: property,
				scope: Scope.universe.id
			} : { column: property };
		}
		for (const name of this.enclosing) relations[name] = {
			column: "scope",
			scope: Scope.universe.id
		};
		const object = traitObject(this, this.policy.package.id);
		const located = this.traits.flatMap(({ trait, options }) => trait.mapping === void 0 ? [] : [trait.mapping(options, object)]);
		Object.assign(relations, ...located.map((mapping) => mapping.relations));
		if (this.isPrincipal && Object.hasOwn(this.policy.definition.relations, "self")) relations.self = { column: "id" };
		return {
			policy: this.policy,
			table: this.table,
			id: "id",
			scope: "scope",
			attributes: Object.fromEntries(Object.keys(this.policy.definition.attributes).map((name) => [name, camelCase(name)])),
			relations,
			...inherited,
			...Object.assign({}, ...located.map(({ relations: _relations, ...placed }) => placed))
		};
	}
	/** Require a matching aggregate field with readers that may list every measured row. */
	requireAggregate(measured, name, aggregate) {
		if (this.fields[name]?.aggregate !== aggregate.function) throw new TypeError(`aggregate ${name} of ${measured.name} fills no ${aggregate.function} field of ${this.name}`);
		const guard = this.fields[name].access?.read;
		const readers = new Set(guard === void 0 ? Object.values(this.methods).filter((method) => method.kind === "get" || method.kind === "list").flatMap((method) => method.permission === null ? [] : [method.permission]) : [guard]);
		const listing = measured.listing?.name;
		const link = aggregate.via ?? "parent";
		const permissions = measured.policy.definition.permissions;
		const uncovered = [...readers].filter((reader) => listing === void 0 || !covers(permissions, listing, link, reader, /* @__PURE__ */ new Set()));
		if (uncovered.length > 0) throw new TypeError(`aggregate ${name} of ${measured.name} measures rows that readers of ${this.name} may not list: list them through ${link} ${uncovered.join(", ")}`);
	}
	/** The permission listing the objects needs, absent without a list method. */
	get listing() {
		return methodPermission(this, "list");
	}
	/** The permission reading one object needs, the listing's without a get method, or none. */
	get reading() {
		return methodPermission(this, "get") ?? this.listing;
	}
	/** Whether the objects represent a principal kind. */
	get isPrincipal() {
		return Object.values(principal).some((kind) => kind.definition.packageId === this.policy.definition.packageId && kind.name === this.policy.name);
	}
	/** Match the objects living in one scope, as SQL. */
	inScope(scope) {
		return eq(this.table[TABLE].columns.scope, scope);
	}
	/** Take these objects as attachments of a host. */
	attach(options) {
		if (this.parent?.object !== "any") throw new TypeError(`object ${this.name} is not nested in any parent`);
		return {
			object: this,
			by: options.by
		};
	}
	/** Wrap methods' effects in handlers. */
	handle(handlers) {
		const methods = { ...this.methods };
		for (const [name, handler] of Object.entries(handlers)) {
			const declared = methods[name];
			if (declared === void 0) throw new TypeError(`object ${this.name} has no method ${name}`);
			methods[name] = declared.handle(handler);
		}
		return this.with({ methods });
	}
	/** Set how a stack's declarations of the objects become their managed records. */
	declare(declaration) {
		return this.with({ declaration });
	}
	/** Reconcile or follow the objects with work waiting as the system. */
	control(controller) {
		return this.with({ controller });
	}
	/** Copy the object type with some members changed, sharing its table. */
	with(changes) {
		return Object.assign(Object.create(Object.getPrototypeOf(this)), this, changes);
	}
	/** Reference one of the object's declared permissions. */
	permission(name) {
		if (!this.permissions.includes(name)) throw new TypeError(`object ${this.name} has no permission ${name}`);
		return this.policy.permission(name);
	}
	/** Where the objects' routes name their scope. */
	get route() {
		return scopeRoute(this);
	}
	/** The schemas of the objects' rows and of the inputs their methods take. */
	get schema() {
		return objectSchema(this);
	}
	/** Whether a controller reconciles the objects to their desired generation. */
	get isControlled() {
		return this.traits.some((applied) => applied.trait === controlled);
	}
	/** The scope types enclosing each object, nearest first, for objects in one scope type. */
	get ancestors() {
		const ancestors = [];
		for (let scope = this.scope; scope instanceof ObjectType; scope = scope.scope) ancestors.push(scope);
		return ancestors;
	}
	/** Read the directory's identity of one of the type's unique indexes. */
	index(name) {
		return `${this.policy.definition.packageId}/${this.name}/${name}`;
	}
	/** Key values in one of the type's indexes, within the scope the index is unique across. */
	claim(name, values, scope) {
		const declared = this.indexes[name];
		if (declared === void 0) throw new TypeError(`object ${this.name} has no index ${name}`);
		const within = declared.across === Scope.universe.id ? null : scope;
		if (within === void 0) throw new TypeError(`index ${name} of ${this.name} keys values within a scope`);
		return {
			index: this.index(name),
			key: canonicalize([within, ...values])
		};
	}
	/** List the names a row claims in the type's indexes with enclosing scopes from a snapshot. */
	async claims(row, snapshot) {
		const scope = String(row.scope);
		const claims = [];
		for (const [name, declared] of Object.entries(this.indexes)) {
			const values = declared.on.map((field) => row[field]);
			if (values.some((value) => value === null || value === void 0)) continue;
			const within = await this.within(declared.across, scope, snapshot);
			claims.push({
				index: this.index(name),
				key: canonicalize([within, ...values]),
				objectId: String(row.id),
				scope
			});
		}
		return claims;
	}
	/** Describe the names an object claims after a write, none after deletion. */
	async owned(objectId, row, snapshot) {
		return {
			indexes: Object.keys(this.indexes).map((name) => this.index(name)),
			objectId,
			claims: row === void 0 ? [] : await this.claims(row, snapshot)
		};
	}
	/** Look up the object owning values in one of the type's indexes. */
	async lookup(directory, name, values, scope) {
		const { index, key } = this.claim(name, values, scope);
		const found = await directory.owner(index, key);
		return found === void 0 ? void 0 : this.reference(found.scope, found.objectId);
	}
	/** Read the names the indexed rows an open transaction wrote claim, one entry per written object. */
	static async written(transaction, objects) {
		const indexed = objects.filter((object) => Object.keys(object.indexes).length > 0);
		const byTable = new Map(indexed.map((object) => [object.table, object]));
		const owned = /* @__PURE__ */ new Map();
		const snapshot = Snapshot.live(transaction);
		if (indexed.length > 0) for (const change of await transaction.log.written([...byTable.keys()])) {
			const object = byTable.get(change.table);
			const objectId = String(change.key.id);
			const row = change.after;
			owned.set(`${object.name}/${objectId}`, await object.owned(objectId, row, snapshot));
		}
		return [...owned.values()];
	}
	/** Find the scope an index is unique across, from the scope an object lives in. */
	async within(across, scope, snapshot) {
		if (across === Scope.universe.id) return null;
		else if (across.same(this.ancestors[0])) return scope;
		const { packageId, name } = across.policy.definition;
		const enclosing = (await Scope.chain(snapshot, scope)).find((link) => link.object.packageId === packageId && link.object.type === name);
		if (enclosing === void 0) throw new ORPCError("PRECONDITION_FAILED", { message: `scope ${scope} has no enclosing ${name}` });
		return enclosing.object.id;
	}
	/** Read the scopes a query of the objects reads from a scope chain, nearest first. */
	scopesOf(chain) {
		return this.inherited === void 0 ? [chain[0]] : [...chain];
	}
	/** Decide whether another type shares this one's table. */
	same(other) {
		return other instanceof ObjectType && other.table === this.table;
	}
	/** Compile a query or include of the objects. */
	query(shape, objects) {
		return compile(this, shape, objects);
	}
	/** Resolve the join an include name makes. */
	join(name, objects) {
		return join(this, name, objects);
	}
	/** Compile a scope's named object queries. */
	static queries(objects, queries, chain) {
		return compileQueries(objects, queries, chain);
	}
	/** The procedures of the object's methods. */
	get procedures() {
		return objectProcedures(this);
	}
	/** List object types with the chunk type their text fields need. */
	static served(objects) {
		const owners = objects.filter((object) => object.text.length > 0);
		return owners.length === 0 ? [...objects] : [...objects, Chunk.object(owners)];
	}
	/** Decide whether objects of another type attach to these objects. */
	holds(child) {
		return this.attachments.some((attachment) => child.same(attachment.object)) || child.table === chunk && this.text.length > 0;
	}
	/** The procedures every object type shares. */
	get shared() {
		return { replica: replicaProcedures };
	}
};
/** Declare an object type. */
function defineObject(definition, module) {
	const owner = declaringModule(module, "defineObject").package;
	const intrinsic = definition[INTRINSIC];
	const traits = TRAITS.flatMap((trait) => {
		const options = intrinsic === void 0 ? trait.options(definition) : void 0;
		return options === void 0 ? [] : [{
			trait,
			options
		}];
	});
	const packageId = (definition.represents?.package ?? owner).id;
	const table = intrinsic?.table ?? deriveTable(definition, traits, packageId, module);
	const declared = definition.methods ?? {};
	const derived = traits.map(({ trait, options }) => trait.methods(options, declared));
	const clash = derived.flatMap(Object.keys).find((name) => Object.hasOwn(declared, name));
	if (clash !== void 0) throw new TypeError(`object ${definition.name} declares method ${clash}, which a trait derives`);
	const methods = Object.assign({ ...declared }, ...derived);
	return new ObjectType(owner, {
		...definition,
		table,
		methods
	}, traits, intrinsic?.mapping);
}
/** Decide whether a permission admits the holders of a related object's permission. */
function covers(permissions, name, relation, permission, seen) {
	if (seen.has(name)) return false;
	seen.add(name);
	const admits = (expression) => expression.kind === "union" ? expression.expressions.some(admits) : expression.kind === "permission" ? covers(permissions, expression.name, relation, permission, seen) : expression.kind === "through" && expression.relation === relation && expression.permission === permission;
	return permissions[name] !== void 0 && admits(permissions[name]);
}
/** Read a represented policy's relations as input. */
function representedRelations(represented) {
	return Object.fromEntries(Object.entries(represented?.definition.relations ?? {}).map(([name, relation]) => [name, {
		subjects: relation.subjects,
		grantedBy: relation.grantedBy ?? null
	}]));
}
/** Read the permission a type's standard method of a kind needs, absent without one. */
function methodPermission(object, kind) {
	const method = Object.values(object.methods).find((declared) => declared.kind === kind);
	return method?.permission === void 0 || method.permission === null ? void 0 : object.permission(method.permission);
}
/** Describe an object type to its traits. */
function traitObject(type, packageId) {
	return {
		name: type.name,
		identity: type.identity,
		packageId,
		table: () => type.table,
		storage: type.storage
	};
}
/** An account's handle, a DNS label. */
var AccountHandle = defineSchema(string().regex(/^(?!-)[a-z0-9-]{1,63}(?<!-)$/));
/** Whether a handle may be taken. */
var HandleAvailability = defineSchema(_enum([
	"available",
	"taken",
	"invalid"
]));
var __destackModule$21 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** The supported storage and processing jurisdictions. */
var RESIDENCIES = ["eu", "us"];
/** The permitted jurisdiction for storage and processing. */
var Residency = defineSchema(_enum(RESIDENCIES));
/** A Destack regional deployment. */
var region = defineObject({
	name: "region",
	tier: "global",
	plural: "regions",
	scope: "universe",
	represents: principal.region,
	fields: {
		/** The region's short code, such as eu-central. */
		code: field.string(),
		/** The display name. */
		name: field.string(),
		/** The jurisdiction the region keeps data in. */
		residency: field.enum(RESIDENCIES)
	},
	permissions: ["serve"],
	constraints: (region) => [unique("region_code").on(region.code), unique("region_residency_id").on(region.id, region.residency)]
}, __destackModule$21);
/** The authentication sensitive changes ask for, sudo's fifteen minutes. */
var sudo = {
	assurance: 2,
	maxAge: 9e5
};
var __destackModule$20 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** The longest text a handle check accepts, beyond which nothing is a handle. */
var HANDLE_INPUT = 256;
/** The longest IANA time zone name, America/Argentina/ComodRivadavia with room to spare. */
var TIME_ZONE_LENGTH = 64;
/** A BCP 47 language tag, such as en-US. */
var Locale = defineSchema(string().regex(/^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|\d{3}))?$/));
/** An IANA time zone name, such as Europe/Vienna. */
var TimeZone = defineSchema(string().max(TIME_ZONE_LENGTH).regex(/^[A-Za-z][A-Za-z0-9_+-]*(?:\/[A-Za-z0-9_+-]+)*$/));
/** A person, the user principal. */
var user = defineObject({
	name: "user",
	tier: "global",
	plural: "users",
	scope: "universe",
	isScope: true,
	represents: principal.user,
	fields: {
		/** The display name. */
		name: field.string(string().max(200)),
		/** The primary email address, unique across users. */
		email: field.string().guard({ read: "update" }),
		/** Whether the user proved control of the email address. */
		emailVerified: field.boolean().default(false).guard({ read: "update" }),
		/** The profile image URL. */
		image: field.string().optional(),
		/** The login the sign-in provider knows the user by. */
		login: field.string().optional().guard({ read: "update" }),
		/** The language and region the user reads, the OpenID Connect locale claim. */
		locale: field.string(Locale).optional().guard({ read: "update" }),
		/** The time zone the user lives in, the OpenID Connect zoneinfo claim. */
		timeZone: field.string(TimeZone).optional().guard({ read: "update" }),
		/** The jurisdiction the user's home space keeps their data in, chosen with their handle. */
		residency: field.enum(RESIDENCIES).optional().guard({ read: "update" }),
		/** The user's home space. */
		home: field.string(identifier("space")).optional().guard({ read: "update" }),
		/** Whether sign-in requires a second factor. */
		twoFactorEnabled: field.boolean().default(false).guard({ read: "update" }),
		/** When the platform suspended the user, in UTC epoch milliseconds. */
		suspendedAt: field.time().optional().guard({ read: "update" })
	},
	recoverable: {
		within: { days: 30 },
		by: "delete"
	},
	relations: {
		self: { subjects: [principal.user] },
		delegate: {
			subjects: [principal.user],
			grantedBy: "lend"
		},
		joined: {
			subjects: [principal.space],
			grantedBy: null
		}
	},
	permissions: {
		read: union$1(relation("self"), relation("joined")),
		update: relation("self"),
		lend: relation("self"),
		impersonate: relation("delegate"),
		delete: relation("self")
	},
	shareable: { by: "lend" },
	elevated: {
		lend: sudo,
		impersonate: sudo,
		delete: sudo
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		update: method.update("update", { fields: [
			"name",
			"image",
			"locale",
			"timeZone",
			"residency"
		] }),
		suggestHandle: method({
			permission: "read",
			mutates: false,
			output: strictObject({ handle: AccountHandle })
		}),
		checkHandle: method({
			permission: "read",
			mutates: false,
			input: strictObject({ 
			/** The handle as the user typed it. */
handle: string().max(HANDLE_INPUT) }),
			output: strictObject({
				/** The handle checked. */
				handle: string(),
				/** Whether the handle is free, taken, or not a handle at all. */
				availability: HandleAvailability
			})
		})
	},
	constraints: (user) => [unique("user_email").on(user.email)]
}, __destackModule$20);
var __destackModule$19 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** An organisation owning accounts. */
var organisation = defineObject({
	name: "organisation",
	tier: "global",
	plural: "organisations",
	scope: "universe",
	isScope: true,
	fields: {
		/** The display name. */
		name: field.string(string().min(1).max(200)),
		/** The logo image URL. */
		image: field.string(string().url()).optional(),
		/** The jurisdiction the organisation's home space keeps its data in, chosen at creation. */
		residency: field.enum(RESIDENCIES),
		/** The organisation's home space. */
		home: field.string(identifier("space")).optional()
	},
	recoverable: {
		within: { days: 30 },
		by: "delete"
	},
	relations: {
		owner: {
			subjects: [user],
			grantedBy: "own"
		},
		member: { subjects: [user] }
	},
	permissions: {
		read: union$1(relation("owner"), relation("member")),
		update: relation("owner"),
		delete: relation("owner"),
		share: relation("owner"),
		own: relation("owner")
	},
	shareable: { by: "share" },
	reserved: ["own"],
	elevated: {
		delete: sudo,
		own: sudo
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("update", {
			fields: [
				"name",
				"image",
				"residency"
			],
			creator: "owner"
		}),
		update: method.update("update", { fields: ["name", "image"] })
	}
}, __destackModule$19);
var __destackModule$18 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** An account, the scope of its spaces. */
var account = defineObject({
	name: "account",
	tier: "global",
	plural: "accounts",
	scope: [user, organisation],
	isScope: true,
	fields: {
		/** The globally unique handle scoping the account's package names. */
		handle: field.string(AccountHandle),
		/** The display name. */
		name: field.string(string().min(1).max(200)),
		/** The residency new spaces take. */
		defaultResidency: field.enum(RESIDENCIES).guard({ read: "use" }),
		/** Whether the account is its user's own or shared. */
		kind: field.enum(["personal", "shared"]).default("shared"),
		/** The package admission policy every space of the account inherits. */
		packagePolicyId: field.string(identifier("package-policy")).optional().guard({ read: "use" }),
		/** The region holding the package policy. */
		packagePolicyRegion: field.reference(region, { delete: "restrict" }).optional().guard({ read: "use" }),
		/** The network policy every space of the account inherits. */
		networkPolicyId: field.string(identifier("network-policy")).optional().guard({ read: "use" }),
		/** The region holding the network policy. */
		networkPolicyRegion: field.reference(region, { delete: "restrict" }).optional().guard({ read: "use" })
	},
	attributes: { kind: "string" },
	recoverable: {
		within: { days: 30 },
		by: "delete"
	},
	relations: {
		root: {
			subjects: [user, organisation.members("owner")],
			grantedBy: "own"
		},
		member: { subjects: [user] },
		host: {
			subjects: [principal.host.all()],
			grantedBy: null
		}
	},
	permissions: {
		read: union$1(relation("root"), relation("member"), relation("host"), intersection(condition(Condition.eq("kind", "personal")), through("user", "read"))),
		use: union$1(relation("root"), relation("member"), relation("host")),
		replicate: relation("host"),
		update: relation("root"),
		delete: relation("root"),
		share: relation("root"),
		own: relation("root"),
		impersonate: relation("root"),
		verify: relation("root")
	},
	shareable: { by: "share" },
	reserved: ["own", "replicate"],
	elevated: {
		delete: sudo,
		own: sudo,
		impersonate: sudo
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("update", {
			fields: [
				"handle",
				"name",
				"kind"
			],
			input: strictObject({ 
			/** The residency new spaces take, its owner's when absent. */
defaultResidency: Residency.optional() }),
			creation: rootAccount,
			isPredicted: false
		}),
		update: method.update("update", { fields: [
			"handle",
			"name",
			"defaultResidency"
		] })
	},
	constraints: (account) => [
		unique("account_handle_unique").on(account.handle),
		index("account_scope").on(account.scope),
		uniqueIndex("account_personal").on(account.scope).where(sql`${account.kind} = 'personal'`),
		check("account_kind", sql`${account.kind} = 'shared' OR ${account.scope} LIKE 'user-%'`),
		check("account_network_policy", sql`(${account.networkPolicyId} IS NULL) = (${account.networkPolicyRegion} IS NULL)`),
		check("account_package_policy", sql`(${account.packagePolicyId} IS NULL) = (${account.packagePolicyRegion} IS NULL)`),
		check("account_handle", dialectSQL({
			sqlite: sql`length(${account.handle}) BETWEEN 1 AND 63 AND ${account.handle} NOT GLOB '*[^a-z0-9-]*' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
			postgresql: sql`length(${account.handle}) BETWEEN 1 AND 63 AND (${account.handle} COLLATE "C") !~ '[^a-z0-9-]' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`
		}))
	]
}, __destackModule$18);
/** Root a new account at its creator or its organisation's owners. */
async function rootAccount(call, object) {
	const creator = call.caller;
	if (creator === void 0) throw new ORPCError("FORBIDDEN", { message: "account needs a creator" });
	const owner = await Scope.object(Snapshot.live(call.database), call.scope);
	return {
		relationships: [{
			relation: "root",
			subject: owner.type === organisation.name ? {
				...owner,
				relation: "owner"
			} : creator
		}, {
			relation: "host",
			subject: principal.host.reference(object.id, "*")
		}],
		owner: {
			...object,
			relation: "root"
		}
	};
}
/** Refuse a withdrawn object, reading its noun in the message. */
function requireUnrevoked(target, noun) {
	if (target.revokedAt !== null) throw new ORPCError("CONFLICT", { message: `${noun} is revoked` });
}
/** Withdraw a call's target now, refusing to withdraw it twice. */
function revokeOnce(call, noun) {
	requireUnrevoked(call.target, noun);
	return call.revise({ revokedAt: call.now });
}
/** SHA-256 digests of text, as secrets, codes, PKCE verifiers and key thumbprints keep them. */
var Digest = class Digest {
	/** Hash text with SHA-256 into lowercase hex. */
	static async hex(text) {
		return (await Digest.#bytes(text)).toHex();
	}
	/** Hash text with SHA-256 into unpadded base64url, as PKCE S256 and JWK thumbprints encode it. */
	static async base64url(text) {
		return (await Digest.#bytes(text)).toBase64({
			alphabet: "base64url",
			omitPadding: true
		});
	}
	/** Hash the UTF-8 bytes of text with SHA-256. */
	static async #bytes(text) {
		return new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
	}
};
var __destackModule$17 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
defineTable("signing_key", {
	/** The key identifier published in the JWKS document. */
	id: identifier$1("id", "signing-key").primaryKey(),
	/** The serialized public JWK. */
	publicKey: text$1("public_key").notNull(),
	/** The encrypted private JWK. */
	privateKey: text$1("private_key").notNull(),
	/** The creation time in epoch milliseconds. */
	createdAt: integer("created_at").notNull(),
	/** The key expiry in epoch milliseconds. */
	expiresAt: integer("expires_at"),
	/** The JOSE signing algorithm. */
	alg: text$1("alg"),
	/** The elliptic curve name. */
	crv: text$1("crv")
}, {
	tier: "global",
	constraints: (key) => [index("signing_key_created").on(key.createdAt)]
}, __destackModule$17);
/** A consumed device key proof retained for replay protection across service instances. */
var authenticationReplay = defineTable("authentication_replay", {
	/** The digest of the proof's key thumbprint and identifier. */
	id: text$1("id").primaryKey(),
	/** The earliest safe deletion time in epoch milliseconds. */
	expiresAt: integer("expires_at").notNull()
}, {
	tier: "global",
	constraints: (replay) => [index("authentication_replay_expiry").on(replay.expiresAt)]
}, __destackModule$17);
var __destackModule$16 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** The type a device key proof declares in its header. */
var PROOF_TYPE = "device-key+jwt";
/** How far a proof's issue time may lie from now, a round trip plus skew in seconds. */
var PROOF_SKEW_SECONDS = 60;
/** A device's P-256 public key as a JSON Web Key. */
var DevicePublicKey = defineSchema(strictObject({
	/** The key type. */
	kty: literal("EC"),
	/** The curve. */
	crv: literal("P-256"),
	/** The x coordinate, base64url encoded. */
	x: string().regex(/^[A-Za-z0-9_-]{43}$/),
	/** The y coordinate, base64url encoded. */
	y: string().regex(/^[A-Za-z0-9_-]{43}$/)
}).strict());
/** The protected header of a device key proof. */
var ProofHeader = strictObject({
	/** The signing algorithm, ECDSA over P-256 with SHA-256. */
	alg: literal("ES256"),
	/** The proof type. */
	typ: literal(PROOF_TYPE),
	/** The RFC 7638 thumbprint of the signing key. */
	kid: string().min(1)
});
/** The claims of a device key proof, named as in RFC 9449. */
var ProofClaims = strictObject({
	/** The device or host the proof speaks for. */
	sub: string().min(1),
	/** The issue time in seconds since the epoch. */
	iat: number().int(),
	/** The proof's unique identifier. */
	jti: string().min(16).max(64),
	/** The HTTP method of the request a request proof authenticates. */
	htm: string().min(1).optional(),
	/** The URL, without query and fragment, of the request a request proof authenticates. */
	htu: url().optional()
});
/** A device's proof that it holds a key's private half, as a compact ES256 JWS. */
var DeviceProof = class DeviceProof {
	/** The RFC 7638 thumbprint of the signing key. */
	thumbprint;
	/** The device the proof speaks for. */
	deviceId;
	/** The issue time in seconds since the epoch. */
	issuedAt;
	/** The proof's unique identifier. */
	id;
	/** The request the proof authenticates, absent for a possession proof. */
	request;
	/** The signed header and claims. */
	signed;
	/** The signature over them. */
	signature;
	/** Hold a proof's parts. */
	constructor(parts) {
		this.thumbprint = parts.thumbprint;
		this.deviceId = parts.deviceId;
		this.issuedAt = parts.issuedAt;
		this.id = parts.id;
		this.request = parts.request;
		this.signed = parts.signed;
		this.signature = parts.signature;
	}
	/** Read a proof's parts and refuse any other serialization, type or algorithm. */
	static read(proof) {
		const parts = proof.split(".");
		if (parts.length !== 3) throw invalid("device proof is no compact JWS");
		const [encodedHeader, encodedClaims, encodedSignature] = parts;
		const header = ProofHeader.safeParse(decodeJson(encodedHeader));
		const claims = ProofClaims.safeParse(decodeJson(encodedClaims));
		if (!header.success || !claims.success) throw invalid("device proof has an invalid header or claims");
		const { htm, htu } = claims.data;
		if (htm === void 0 !== (htu === void 0)) throw invalid("device proof binds a method or a URL alone");
		return new DeviceProof({
			thumbprint: header.data.kid,
			deviceId: claims.data.sub,
			issuedAt: claims.data.iat,
			id: claims.data.jti,
			request: htm === void 0 ? void 0 : {
				method: htm,
				url: htu
			},
			signed: new TextEncoder().encode(`${encodedHeader}.${encodedClaims}`),
			signature: decodeBytes(encodedSignature)
		});
	}
	/** Sign a proof for a subject. */
	static async sign(privateKey, publicKey, subject, now, request) {
		const header = {
			alg: "ES256",
			typ: PROOF_TYPE,
			kid: await DeviceProof.thumbprint(publicKey)
		};
		const claims = {
			sub: subject,
			iat: Math.floor(now / 1e3),
			jti: crypto.randomUUID(),
			...request === void 0 ? {} : {
				htm: request.method,
				htu: requestTarget(request.url)
			}
		};
		const signed = `${encodeJson(header)}.${encodeJson(claims)}`;
		const signature = await crypto.subtle.sign({
			name: "ECDSA",
			hash: "SHA-256"
		}, privateKey, new TextEncoder().encode(signed));
		return `${signed}.${new Uint8Array(signature).toBase64({
			alphabet: "base64url",
			omitPadding: true
		})}`;
	}
	/** Compute a key's RFC 7638 thumbprint. */
	static async thumbprint(key) {
		const { crv, kty, x, y } = key;
		return Digest.base64url(JSON.stringify({
			crv,
			kty,
			x,
			y
		}));
	}
	/** Verify the proof with its signing key. */
	async verify(key, deviceId, now, request) {
		const imported = await crypto.subtle.importKey("jwk", key, {
			name: "ECDSA",
			namedCurve: "P-256"
		}, false, ["verify"]);
		const isSigned = await crypto.subtle.verify({
			name: "ECDSA",
			hash: "SHA-256"
		}, imported, this.signature, this.signed);
		if (this.thumbprint !== await DeviceProof.thumbprint(key) || !isSigned) throw invalid("device proof signature is invalid");
		const skew = Math.abs(now / 1e3 - this.issuedAt);
		if (this.deviceId !== deviceId || skew > PROOF_SKEW_SECONDS) throw invalid("device proof is for another device or time");
		if (!(request === void 0 ? this.request === void 0 : this.request?.method === request.method && this.request.url === requestTarget(request.url))) throw invalid("device proof is for another request");
	}
	/** Use the proof once. */
	async consume(database, now) {
		await database.delete(authenticationReplay).where(lte(authenticationReplay.expiresAt, now));
		const digest = await Digest.hex(JSON.stringify([this.thumbprint, this.id]));
		const [recorded] = await database.insert(authenticationReplay).values({
			id: digest,
			expiresAt: (this.issuedAt + PROOF_SKEW_SECONDS) * 1e3
		}).onConflictDoNothing().returning({ id: authenticationReplay.id });
		if (recorded === void 0) throw invalid("device proof was used before");
	}
};
/** Attach the calling session to a device. */
var attach = method({
	permission: "update",
	input: strictObject({ proof: string().min(1).max(4096) })
});
/** Withdraw a device with its keys and sessions. */
var revoke$1 = method({ permission: "revoke" });
/** A device a user registered. */
var device = defineObject({
	name: "device",
	tier: "global",
	plural: "devices",
	scope: user,
	fields: {
		/** The user's name for the device. */
		name: field.string(string().min(1).max(200)),
		/** The form of the device. */
		category: field.enum([
			"desktop",
			"laptop",
			"phone",
			"tablet",
			"server",
			"unknown"
		]).default("unknown"),
		/** The operating system, such as macos. */
		operatingSystem: field.string().optional(),
		/** The operating system's version. */
		operatingSystemVersion: field.string().optional(),
		/** The processor architecture, such as arm64. */
		architecture: field.string().optional(),
		/** The hardware model. */
		model: field.string().optional(),
		/** The version of the Destack client that registered the device. */
		clientVersion: field.string().optional(),
		/** The last time the device proved itself. */
		lastSeenAt: field.time().optional(),
		/** The time the user withdrew the device. */
		revokedAt: field.time().optional()
	},
	permissions: [
		"read",
		"create",
		"update",
		"revoke"
	],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("create", {
			fields: [
				"name",
				"category",
				"operatingSystem",
				"operatingSystemVersion",
				"architecture",
				"model",
				"clientVersion"
			],
			isPredicted: false
		}),
		update: method.update("update", { fields: ["name"] }),
		attach,
		revoke: revoke$1,
		see: method({
			permission: null,
			isSystem: true
		}).handle((call) => call.revise({ lastSeenAt: call.now }))
	},
	constraints: (device) => [index("device_scope").on(device.scope)]
}, __destackModule$16);
defineObject({
	name: "device-key",
	tier: "global",
	plural: "deviceKeys",
	scope: user,
	nested: {
		in: device,
		receive: "update",
		delete: "cascade"
	},
	fields: {
		/** The public key. */
		publicKey: field.json(DevicePublicKey),
		/** The RFC 7638 JWK thumbprint binding device proofs and tokens to the key. */
		thumbprint: field.string(string().min(1)),
		/** The time the device proved possession of the private key. */
		verifiedAt: field.time(),
		/** The time the key stops authenticating. */
		expiresAt: field.time(),
		/** The time the user withdrew the key. */
		revokedAt: field.time().optional()
	},
	permissions: [
		"read",
		"register",
		"revoke"
	],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("register", {
			fields: ["publicKey"],
			input: strictObject({ proof: string().min(1).max(4096) }),
			isPredicted: false
		}),
		revoke: method({ permission: "revoke" }).handle((call) => revokeOnce(call, "device key"))
	},
	constraints: (key) => [
		unique("device_key_thumbprint").on(key.thumbprint),
		unique("device_key_device_id").on(key.parentId, key.id),
		check("device_key_expiry", sql`${key.expiresAt} > ${key.createdAt}`),
		check("device_key_verification", sql`${key.verifiedAt} >= ${key.createdAt} AND ${key.verifiedAt} < ${key.expiresAt}`)
	]
}, __destackModule$16);
/** Refuse a malformed or unverifiable device proof. */
function invalid(message, cause) {
	return new ORPCError("BAD_REQUEST", {
		message,
		...cause === void 0 ? {} : { cause }
	});
}
/** Read a request's URL without its query and fragment. */
function requestTarget(url) {
	const parsed = new URL(url);
	return `${parsed.origin}${parsed.pathname}`;
}
/** Encode a value as a base64url JSON part of a JWS. */
function encodeJson(value) {
	return new TextEncoder().encode(JSON.stringify(value)).toBase64({
		alphabet: "base64url",
		omitPadding: true
	});
}
/** Decode a base64url part of a JWS into JSON and refuse a part without JSON. */
function decodeJson(part) {
	const text = new TextDecoder().decode(decodeBytes(part));
	try {
		return JSON.parse(text);
	} catch (error) {
		throw invalid("device proof part is no JSON", error);
	}
}
/** Decode a base64url part of a JWS into bytes and refuse any other encoding. */
function decodeBytes(part) {
	try {
		return Uint8Array.fromBase64(part, { alphabet: "base64url" });
	} catch (error) {
		throw invalid("device proof part is no base64url", error);
	}
}
var __destackModule$15 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614c-76c1-90c6-f83bdcd72b32",
	"name": "@destack/account",
	"version": "2026.9.0"
} });
/** A space's zone as the directory places it: the cell serving it represents the space, and the cell a transfer moves it to reads it. */
var zone = defineObject({
	name: "zone",
	plural: "zones",
	[INTRINSIC]: {
		table: zoneTable,
		mapping: {
			table: zoneTable,
			id: "id",
			scope: "scope",
			attributes: {},
			relations: {
				space: {
					column: "id",
					scope: Scope.universe.id
				},
				cell: {
					column: "cell",
					scope: Scope.universe.id
				},
				target: {
					column: "target",
					scope: Scope.universe.id
				}
			}
		}
	},
	scope: "universe",
	relations: {
		space: {
			subjects: [principal.space],
			grantedBy: null
		},
		cell: {
			subjects: [principal.cell],
			grantedBy: null
		},
		target: {
			subjects: [principal.cell],
			grantedBy: null
		}
	},
	permissions: {
		read: relation("target"),
		represent: union$1(relation("space"), relation("cell"))
	},
	methods: {
		get: method.get("read"),
		list: method.list("read")
	}
}, __destackModule$15);
/** A package-local setting name, kept across releases. */
var SettingName = defineSchema(string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/));
/** The identity of a setting across package renames and releases. */
var SettingReference = Object.assign(defineSchema(strictObject({
	/** The package declaring the setting. */
	packageId: PackageId,
	/** The package-local name. */
	name: SettingName
})), { 
/** Key a setting by its package and name. */
key(reference) {
	return `${reference.packageId}/${reference.name}`;
} });
/** The ways a value applies: set in its scope, or recommended or required by an enclosing one. */
var SETTING_MODES = [
	"set",
	"recommend",
	"require"
];
/** How a value applies. */
var SettingMode = defineSchema(_enum(SETTING_MODES));
/** A setting value a stack places in its space. */
var SpaceSetting = defineSchema(strictObject({
	/** The setting. */
	setting: SettingReference,
	/** The value. */
	value: json(),
	/** How the value applies. */
	mode: SettingMode
}));
var __destackModule$14 = Object.freeze({ "package": {
	"id": "package-01a0d394-bf2b-7191-926c-8d1fee31f728",
	"name": "@destack/host",
	"version": "2026.9.0"
} });
/** How long a host key authenticates, a year in milliseconds. */
var KEY_LIFETIME_MILLISECONDS = 31536e6;
/** The longest proof a host presents, far above the few hundred bytes of an ES256 compact JWS. */
var PROOF_LENGTH = 4096;
/** The longest host name: a DNS label (RFC 1035 2.3.4). */
var HOST_NAME_LENGTH = 63;
/** The characters of a host name, as a character class: lowercase letters, digits and hyphens. */
var HOST_NAME_CHARACTERS = "a-z0-9-";
/** Whether a host accepts new work, drains its existing work, or refuses work. */
var HOST_STATUSES = [
	"enabled",
	"draining",
	"disabled"
];
/** A proof by a host key. */
var Proof = string().min(1).max(PROOF_LENGTH);
/** A host's name within its account: a DNS label without leading, trailing or doubled hyphens. */
var HostName = string().regex(new RegExp(`^(?!-)(?!.*--)[${HOST_NAME_CHARACTERS}]{1,${HOST_NAME_LENGTH}}(?<!-)$`));
/** Rename a host within its account. */
var renaming = method({
	permission: "rename",
	input: strictObject({ 
	/** The new name. */
name: HostName })
});
/** Report the host's version and runtimes at a proven contact. */
var seeing = method({
	permission: "rotate",
	input: strictObject({
		/** The Destack version the host runs. */
		version: string().min(1).nullable(),
		/** The runtimes the host can execute. */
		runtimes: array(ServerRuntime)
	})
});
/** A host key and the proof that the host holds its private half. */
var KeyInput = strictObject({
	/** The public key. */
	publicKey: DevicePublicKey,
	/** A proof of the host by the key's private half. */
	proof: Proof
});
/** A machine running Destack for an account. */
var host = defineObject({
	name: "host",
	tier: "global",
	plural: "hosts",
	scope: account,
	represents: principal.host,
	fields: {
		/** The name within the account. */
		name: field.string(HostName),
		/** Whether the host is an account's device or a provider's cloud machine. */
		kind: field.enum(["device", "cloud"]),
		/** The device running a device host. */
		device: field.reference(device, { delete: "restrict" }).optional(),
		/** The region a cloud host serves. */
		region: field.reference(region, { delete: "restrict" }).optional(),
		/** The provider running a cloud host. */
		providerCode: field.string().optional(),
		/** The provider location of a cloud host. */
		location: field.string().optional(),
		/** Whether the host accepts new work, drains existing work, or neither. */
		status: field.enum(HOST_STATUSES).default("enabled"),
		/** The Destack version the host last reported. */
		version: field.string().optional(),
		/** The runtimes the host can execute. */
		runtimes: field.json(array(ServerRuntime)).default([]),
		/** The host's last proven contact, in UTC epoch milliseconds. */
		lastSeenAt: field.time().optional(),
		/** The time the account withdrew the host and its keys. */
		revokedAt: field.time().optional()
	},
	relations: {
		tenant: {
			subjects: [account.members("member")],
			grantedBy: "share"
		},
		self: {
			subjects: [principal.host],
			grantedBy: null
		},
		peer: {
			subjects: [principal.host.all()],
			grantedBy: null
		}
	},
	permissions: {
		read: union$1(relation("tenant"), relation("self")),
		verify: union$1(relation("tenant"), relation("self"), relation("peer")),
		enroll: none(),
		rotate: relation("self"),
		rename: relation("self"),
		revoke: relation("self"),
		update: none(),
		share: none()
	},
	reserved: ["rotate"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		enroll: method.create("enroll", {
			fields: [
				"name",
				"kind",
				"device",
				"region",
				"providerCode",
				"location",
				"version",
				"runtimes"
			],
			input: KeyInput,
			creation: () => ({ relationships: [{
				relation: "peer",
				subject: principal.host.reference("*", "*")
			}] })
		}).handle(enroll),
		revoke: method({ permission: "revoke" }).handle(revoke),
		enable: method({ permission: "revoke" }).handle((call) => Host.transition(call, "enabled")),
		drain: method({ permission: "revoke" }).handle((call) => Host.transition(call, "draining")),
		disable: method({ permission: "revoke" }).handle((call) => Host.transition(call, "disabled")),
		see: seeing.handle((call) => call.revise({
			lastSeenAt: call.now,
			version: call.input.version,
			runtimes: call.input.runtimes
		})),
		rename: renaming.handle(async (call) => {
			Host.requireActive(call.target);
			return call.revise({ name: HostName.parse(call.input.name) });
		})
	},
	constraints: (host) => [
		unique("host_device_id").on(host.device, host.id),
		unique("host_scope_id").on(host.scope, host.id),
		uniqueIndex("host_scope_name").on(host.scope, host.name).where(sql`${host.revokedAt} IS NULL`),
		check("host_name", hostNameCheck(host.name)),
		check("host_location", sql`${host.location} IS NULL OR (${host.providerCode} IS NOT NULL AND length(${host.location}) > 0)`),
		check("host_kind", sql`(${host.kind} = 'device' AND ${host.device} IS NOT NULL AND ${host.region} IS NULL) OR (${host.kind} = 'cloud' AND ${host.device} IS NULL)`)
	]
}, __destackModule$14);
/** A host's revocable authentication key, registered with a proof by its private half. */
var hostKey = defineObject({
	name: "host-key",
	tier: "global",
	plural: "hostKeys",
	scope: account,
	nested: {
		in: host,
		receive: "rotate",
		delete: "cascade"
	},
	fields: {
		/** The public key. */
		publicKey: field.json(DevicePublicKey),
		/** The RFC 7638 JWK thumbprint that identifies the key in the host's proofs and assertions. */
		thumbprint: field.string(string().min(1)),
		/** The time the host proved possession of the private key. */
		verifiedAt: field.time(),
		/** The time the key stops authenticating. */
		expiresAt: field.time(),
		/** The time the key was withdrawn. */
		revokedAt: field.time().optional(),
		/** The time disabling the host suspended the key. */
		suspendedAt: field.time().optional()
	},
	permissions: {
		read: through("parent", "verify"),
		rotate: through("parent", "rotate"),
		revoke: through("parent", "revoke")
	},
	reserved: ["rotate"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("rotate", {
			fields: ["publicKey"],
			input: KeyInput.pick({ proof: true })
		}).handle(register),
		enroll: method.create(null, { isSystem: true }),
		suspend: method({
			permission: null,
			isSystem: true
		}).handle((call) => call.revise({ suspendedAt: call.now })),
		resume: method({
			permission: null,
			isSystem: true
		}).handle((call) => call.revise({ suspendedAt: null })),
		revoke: method({ permission: "revoke" }).handle(async (call) => {
			if (call.target.revokedAt !== null) throw new ORPCError("CONFLICT", { message: "host key is revoked" });
			return call.revise({ revokedAt: call.now });
		})
	},
	constraints: (key) => [
		unique("host_key_thumbprint").on(key.thumbprint),
		unique("host_key_host_id").on(key.parentId, key.id),
		check("host_key_expiry", sql`${key.expiresAt} > ${key.createdAt}`),
		check("host_key_verification", sql`${key.verifiedAt} >= ${key.createdAt} AND ${key.verifiedAt} < ${key.expiresAt}`)
	]
}, __destackModule$14);
/** Enroll a host under the identifier its proof carries, with the key it proves. */
async function enroll(call, next) {
	if (call.input.region !== void 0 && call.input.region !== null) {
		const served = region.reference(Scope.universe.id, string().parse(call.input.region));
		const authorization = call.served();
		await authorization.require(authorization.authorizer.policy(served).permission("serve"), served);
	}
	const input = KeyInput.parse({
		publicKey: call.input.publicKey,
		proof: call.input.proof
	});
	if (call.id === void 0) throw new ORPCError("BAD_REQUEST", { message: "a host enrolls under the identifier its proof carries" });
	const proof = DeviceProof.read(input.proof);
	await proof.verify(input.publicKey, call.id, call.now);
	await proof.consume(call.database, call.now);
	const name = await freeName(call.database, call.scope, String(call.input.name));
	const created = await next(call.with({ input: {
		...call.input,
		name,
		lastSeenAt: call.now
	} }));
	await call.invoke(hostKey, "enroll", {
		parentId: created.id,
		...await keyRow(input.publicKey, call.now)
	});
	return created;
}
/** Take a preferred host name, or its first free numbered variant in the account, as `laptop-2`. */
async function freeName(database, scope, preferred) {
	const rows = await database.select({ name: host.table.name }).from(host.table).where(and(eq(host.table.scope, scope), isNull(host.table.revokedAt)));
	const taken = new Set(rows.map((row) => row.name));
	let name = preferred;
	for (let number = 2; taken.has(name); number++) {
		const suffix = `-${number}`;
		name = `${preferred.slice(0, HOST_NAME_LENGTH - suffix.length)}${suffix}`;
	}
	return name;
}
/** Register another key a host proves it holds. */
async function register(call, next) {
	const [owner] = await call.database.select().from(host.table).where(and(eq(host.table.id, call.parent().id), host.inScope(call.scope)));
	if (owner === void 0) throw new ORPCError("NOT_FOUND", { message: "host not found" });
	Host.requireActive(owner);
	const publicKey = DevicePublicKey.parse(call.input.publicKey);
	const proof = DeviceProof.read(Proof.parse(call.input.proof));
	await proof.verify(publicKey, owner.id, call.now);
	await proof.consume(call.database, call.now);
	await call.invoke(host, "see", {
		id: owner.id,
		version: owner.version,
		runtimes: owner.runtimes
	});
	await revokeKeys(call, owner.id);
	return next(call.with({ input: {
		...call.input,
		...await keyRow(publicKey, call.now)
	} }));
}
/** Withdraw a host, ending its keys with it. */
async function revoke(call) {
	const target = call.target;
	Host.requireActive(target);
	await revokeKeys(call, target.id);
	return call.revise({ revokedAt: call.now });
}
/** Revoke a host's active keys through the key's own method, recording each revocation. */
function revokeKeys(call, hostId) {
	return invokeKeys(call, hostId, "revoke");
}
/** Invoke a method on each of a host's unrevoked keys, recording each call. */
async function invokeKeys(call, hostId, name) {
	const keys = await call.database.select({ id: hostKey.table.id }).from(hostKey.table).where(and(eq(hostKey.table.parentId, hostId), isNull(hostKey.table.revokedAt)));
	for (const key of keys) await call.invoke(hostKey, name, { id: key.id });
}
/** Require a host name to be a DNS label in both dialects' check syntax. */
function hostNameCheck(name) {
	const length = sql.raw(String(HOST_NAME_LENGTH));
	const characters = sql.raw(`'[^${HOST_NAME_CHARACTERS}]'`);
	const glob = sql.raw(`'*[^${HOST_NAME_CHARACTERS}]*'`);
	const shape = sql`length(${name}) BETWEEN 1 AND ${length} AND ${name} NOT LIKE '-%' AND ${name} NOT LIKE '%-' AND ${name} NOT LIKE '%--%'`;
	return dialectSQL({
		sqlite: sql`${shape} AND ${name} NOT GLOB ${glob}`,
		postgresql: sql`${shape} AND (${name} COLLATE "C") !~ ${characters}`
	});
}
/** Describe a proven key's row. */
async function keyRow(publicKey, now) {
	return {
		publicKey,
		thumbprint: await DeviceProof.thumbprint(publicKey),
		verifiedAt: now,
		expiresAt: now + KEY_LIFETIME_MILLISECONDS
	};
}
/** The checks and reads of hosts that methods and other packages route by. */
var Host = {
	/** Refuse a procedure requiring a host to a caller acting as no host. */
	async authorize({ context, access }) {
		const subject = context.caller?.authentication.subject;
		if (access.authentication === "host" && (subject === void 0 || !principal.host.is(subject))) throw new ORPCError("FORBIDDEN", { message: "the procedure requires a host" });
	},
	/** Refuse changing a withdrawn host. */
	requireActive(target) {
		if (target.revokedAt !== null) throw new ORPCError("CONFLICT", { message: "host is revoked" });
	},
	/** Move an active host to a status, suspending its keys while it is disabled. */
	async transition(call, status) {
		const target = call.target;
		Host.requireActive(target);
		if (status === "disabled" && target.status !== "disabled") await invokeKeys(call, target.id, "suspend");
		else if (status !== "disabled" && target.status === "disabled") await invokeKeys(call, target.id, "resume");
		return call.revise({ status });
	},
	/** Find an account's standing host by its name. */
	async find(database, accountId, name) {
		const [found] = await database.select({ id: host.table.id }).from(host.table).where(and(host.inScope(accountId), eq(host.table.name, name), isNull(host.table.revokedAt)));
		return found?.id;
	}
};
/** A space's address within its account: lowercase letters, digits and single inner hyphens, at most 63 characters. */
var SpaceName = string().regex(/^(?!-)(?!.*--)[a-z0-9-]{1,63}(?<!-)$/);
templateLiteral([
	SpaceName,
	".",
	AccountHandle
]);
/** A resource created by the configuration or explicitly adopted into its administration. */
var SpaceResource = defineSchema(strictObject({
	/** The resource declaration imported from its declaring package. */
	declaration: ResourceDescription.extend({ 
	/** The package declaring the resource. */
package: Package }),
	/** An existing resource to adopt, subject to ownership and residency checks. */
	adopt: ResourceReference.optional(),
	/** The reference of a resource no host provisions, such as a service's address, connected by the placement's provider. */
	reference: string().min(1).optional(),
	/** Whether authorised removal retains or destroys the resource's contents. */
	retention: _enum(["retain", "delete"]),
	/** Provider placement, absent when selected by the host. */
	placement: strictObject({
		/** The provider adapter. */
		provider: string().min(1),
		/** The location code accepted by the provider adapter. */
		location: string().min(1).optional(),
		/** The host administering the resource. */
		host: identifier("host").optional()
	}).optional(),
	/** User-defined labels. */
	tags: record$1(string().min(1), string())
}));
/** A bindable object a binding targets: one the stack declares by key, or an existing one in the destination space. */
var SpaceBindingTarget = defineSchema(union([strictObject({
	/** The bindable object type, such as resource or secret. */
	type: AccessName,
	/** The object's key in this configuration. */
	name: DeclarationName
}), strictObject({
	/** The bindable object type, such as resource or secret. */
	type: AccessName,
	/** The existing object; application verifies it lives in the destination space. */
	id: string().min(1)
})]));
/** Bind one of a package's declarations to an object the space holds: a resource, a secret, or any other bindable object. */
var SpaceBinding = defineSchema(strictObject({
	/** The bound object. */
	target: SpaceBindingTarget,
	/** An exact version or generation to run with, else the target's current one. */
	version: number().int().positive().optional(),
	/** The state the declaration requires of the target, such as a database's tables. */
	state: record$1(string(), json())
}));
/** The scope a setting belongs to and the overrides it permits. */
var SettingScope = defineSchema(discriminatedUnion("scope", [
	strictObject({
		/** A value of each user. */
		scope: literal("user"),
		/** The overrides the setting permits. */
		overrides: array(_enum([
			"package",
			"space",
			"installation",
			"device"
		]))
	}),
	strictObject({
		/** A value of each space. */
		scope: literal("space"),
		/** The overrides the setting permits. */
		overrides: array(literal("installation"))
	}),
	strictObject({
		/** A value of each host. */
		scope: literal("host"),
		/** The overrides the setting permits: none. */
		overrides: tuple([])
	})
]));
/** The serializable fields of a setting declaration. */
var SettingMetadata = defineSchema(strictObject({
	/** The package-local name. */
	name: SettingName,
	/** The label settings views show. */
	title: string().min(1),
	/** The behavior the value controls. */
	description: string().min(1),
	/** The group settings views show it in. */
	group: string().min(1).optional(),
	/** When a consumer applies a changed value. */
	apply: _enum(["immediate", "restart"]),
	/** The migration guidance of a deprecated setting. */
	deprecated: string().min(1).optional()
}));
Object.freeze({ "package": {
	"id": "package-019f5530-8000-7000-8000-000000000001",
	"name": "@destack/setting",
	"version": "2026.9.0"
} });
/** The serializable fields of a setting description beside its scope. */
var description = SettingMetadata.extend({
	/** The declaring package. */
	package: Package,
	/** The value's JSON Schema. */
	schema: record$1(string(), json()),
	/** The default value. */
	default: json(),
	/** The value each release computes from a value of an earlier release, by the release introducing it. */
	convert: record$1(Version, Expression$1.schema).optional()
});
/** A setting declaration as manifests describe it. */
var SettingDescription = defineSchema(discriminatedUnion("scope", [
	SettingScope.options[0].extend(description.shape),
	SettingScope.options[1].extend(description.shape),
	SettingScope.options[2].extend(description.shape)
]));
/** The decision applied to a matching package. */
var PackageDecision = defineSchema(_enum(["allow", "deny"]));
/** A selection of actual packages, independent of dependency import aliases. */
var PackageSelector = defineSchema(strictObject({
	/** The package format, including source packages authored for Destack. */
	kind: _enum(["npm", "destack"]),
	/** The canonical registry URL; omission matches any registry of this format. */
	registry: string().regex(/^https?:\/\/[^\s?#@]+\/$/).optional(),
	/** An exact package name; omission matches every name. */
	name: DependencyName.optional(),
	/** An npm version range; omission matches every version. */
	version: string().min(1).optional(),
	/** Exact content integrity; omission permits any content matching the other fields. */
	integrity: string().min(1).optional()
}));
/** A named allow or deny rule. */
var PackageRule = defineSchema(strictObject({
	/** All specified selector fields must match. */
	package: PackageSelector,
	/** Deny takes precedence over allow within this policy. */
	decision: PackageDecision
}));
/** Decisions for a complete resolved package graph. */
var PackageAdmission = defineSchema(strictObject({
	/** The decision when no rule matches. */
	default: PackageDecision,
	/** Rules keyed by stable names used in diagnostics and audit records. */
	rules: record$1(DeclarationName, PackageRule)
}));
/** Package admission for a space and its installations. */
var PackagePolicyDefinition = defineSchema(strictObject({ 
/** Rules for the root package and every resolved dependency. */
admission: PackageAdmission }));
/** A destination selected independently of its protocol and port. */
var NetworkDestination = defineSchema(union([
	strictObject({ 
	/** Globally routable destinations, excluding host and provider control endpoints. */
kind: literal("public") }),
	strictObject({
		/** Match a canonical DNS hostname. */
		kind: literal("hostname"),
		/** Lowercase ASCII hostname, without a trailing dot or wildcard. */
		hostname: string().max(253).regex(/^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/),
		/** Include descendants of this hostname. */
		subdomains: boolean()
	}),
	strictObject({
		/** Match the actual destination address against an IP network. */
		kind: literal("network"),
		/** An IPv4 or IPv6 address range; use /32 or /128 for a single address. */
		cidr: union([cidrv4(), cidrv6()])
	})
]));
/** An outbound connection rule. */
var NetworkRule = defineSchema(strictObject({
	/** The destination to match, checked again after redirects and DNS resolution. */
	destination: NetworkDestination,
	/** The application protocol or raw transport. */
	protocol: _enum([
		"http",
		"https",
		"ws",
		"wss",
		"tcp",
		"udp"
	]),
	/** Matching ports; omission uses protocol defaults, or any raw transport port. */
	ports: array(int().min(1).max(65535)).min(1).optional(),
	/** Deny takes precedence over allow within this policy. */
	decision: _enum(["allow", "deny"])
}));
/** Outbound network restrictions for an account, space, installation, or workload. */
var NetworkPolicyDefinition = defineSchema(strictObject({
	/** The decision for unmatched public destinations. */
	default: _enum(["allow", "deny"]),
	/** Named rules; nonpublic destinations require an explicit IP network allow rule. */
	rules: record$1(DeclarationName, NetworkRule)
}));
defineSchema(strictObject({
	/** Package admission rules. */
	packages: PackagePolicyDefinition.optional(),
	/** Outbound access intersected with account and host restrictions. */
	network: NetworkPolicyDefinition.optional()
}));
/** Network restrictions applied to an installation and its named workloads. */
var InstallationPolicy = defineSchema(strictObject({
	/** Restrictions shared by every workload in the installation. */
	network: NetworkPolicyDefinition.optional(),
	/** Additional restrictions for individual package workloads. */
	workloads: record$1(DeclarationName, strictObject({ 
	/** Outbound access intersected with installation, space, account, and host restrictions. */
network: NetworkPolicyDefinition })).optional()
}));
/** The account or space administering a policy, independent of its current location. */
var PolicyOwner = defineSchema(discriminatedUnion("kind", [strictObject({
	/** An account-wide policy. */
	kind: literal("account"),
	/** The account whose regional policy service retains the revision. */
	accountId: identifier("account")
}), strictObject({
	/** A space, installation or workload policy. */
	kind: literal("space"),
	/** The space whose administrator retains the revision. */
	spaceId: identifier("space")
})]));
/** Exact policy versions selected for a deployment's installation and workload. */
var PolicySelection = defineSchema(strictObject({
	/** Each package policy must permit the selected package graph. */
	packages: array(strictObject({
		/** The owner through which the version is resolved. */
		owner: PolicyOwner,
		/** The immutable policy version, resolved through its policy authority. */
		versionId: identifier("package-policy-version"),
		/** The canonical definition digest. */
		digest: Digest$1,
		/** The captured definition used to evaluate this deployment. */
		definition: PackagePolicyDefinition
	})),
	/** Each network policy must permit the connection; hosts enforce their intersection. */
	network: array(strictObject({
		/** The owner through which the version is resolved. */
		owner: PolicyOwner,
		/** The immutable policy version, resolved through its policy authority. */
		versionId: identifier("network-policy-version"),
		/** The canonical definition digest. */
		digest: Digest$1,
		/** The captured definition supplied to the execution host. */
		definition: NetworkPolicyDefinition
	}))
}));
/** A package directory within a repository or checkout, or its root. */
var PackageDirectory = union([literal("."), PackagePath]);
/** A committed Git object identifier. */
var Commit = string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/);
/** The release, repository reference or checkout an installation follows. */
var InstallationSelection = defineSchema(discriminatedUnion("kind", [
	strictObject({
		/** Follow one immutable registry release. */
		kind: literal("release"),
		/** The selected package version. */
		version: string().min(1)
	}),
	strictObject({
		/** Follow committed repository history. */
		kind: literal("repository"),
		/** The registered repository. */
		repository: identifier("repository"),
		/** The full branch or tag reference. */
		reference: string().regex(/^refs\/(heads|tags)\/.+$/),
		/** The package directory within the repository. */
		directory: PackageDirectory
	}),
	strictObject({
		/** Follow a local working directory, including unpublished changes. */
		kind: literal("checkout"),
		/** The host administering the checkout. */
		host: identifier("host"),
		/** The checkout registered on that host. */
		checkout: identifier("checkout"),
		/** The package directory within the checkout. */
		directory: PackageDirectory
	})
]));
/** The exact build a revision evaluated. */
var InstallationBuild = defineSchema(discriminatedUnion("kind", [
	strictObject({
		/** A published registry release. */
		kind: literal("release"),
		/** The released package version. */
		version: string().min(1),
		/** The digest of the release manifest, absent before a host resolved the release. */
		manifest: Digest$1.optional()
	}),
	strictObject({
		/** A build of committed repository history. */
		kind: literal("commit"),
		/** The registered repository retaining the commit. */
		repository: identifier("repository"),
		/** The complete Git commit object identifier. */
		commit: Commit,
		/** The package directory within the repository. */
		directory: PackageDirectory,
		/** The digest of the build manifest. */
		manifest: Digest$1
	}),
	strictObject({
		/** A build of a local checkout, including unpublished changes. */
		kind: literal("checkout"),
		/** The host retaining the checkout and build. */
		host: identifier("host"),
		/** The registered working directory. */
		checkout: identifier("checkout"),
		/** The parent commit, absent for an unborn branch. */
		commit: Commit.optional(),
		/** The package directory within the checkout. */
		directory: PackageDirectory,
		/** The digest of the build manifest, absent when evaluated from source. */
		manifest: Digest$1.optional()
	})
]));
defineSchema(union([DeclarationName, strictObject({ 
/** The existing installation checked when applying the configuration. */
id: identifier("installation") })]));
/** A package installation declared in a configuration. */
var SpaceInstallation = defineSchema(strictObject({
	/** Additional restrictions for this installation and its workloads. */
	policies: InstallationPolicy.optional(),
	/** The package release selected by this configuration. */
	package: Package,
	/** Whether this installation should serve requests. */
	status: _enum(["enabled", "suspended"]),
	/** Space-local URL alias, independent of the stable installation key. */
	alias: DeclarationName,
	/** The bindings of the package's declarations, keyed by immutable package ID and declaration name. */
	bindings: record$1(PackageId, record$1(DeclarationName, SpaceBinding)),
	/** Workload compute settings checked against package and host policy. */
	compute: record$1(DeclarationName, ComputeDefinition),
	/** User-defined labels. */
	tags: record$1(string().min(1), string())
}));
/** A stack's declarations as JSON, keyed by collection and validated by each object when applied. */
var SpaceDefinition = defineSchema(record$1(string().min(1), json()));
var __destackModule$12 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Move a space to another address within its account. */
var rename = method({
	permission: "update",
	input: strictObject({ 
	/** The new address. */
name: SpaceName })
}).handle((call) => call.revise({ name: call.input.name }));
/** An isolated place for installations and their data, held by the host or region of its zone. */
var space = defineObject({
	name: "space",
	plural: "spaces",
	scope: account,
	isScope: true,
	controlled: true,
	fields: {
		/** The unique address within the account. */
		name: field.string(SpaceName),
		/** The display text, absent to show the name. */
		title: field.string(string().min(1).max(128)).optional(),
		/** The least risk a resource plan waits for approval at. */
		approval: field.enum(Risk.options).default("backward-incompatible")
	},
	constraints: (space) => [unique("space_scope_id").on(space.scope, space.id), check("space_name", dialectSQL({
		sqlite: sql`length(${space.name}) BETWEEN 1 AND 63 AND ${space.name} NOT GLOB '*[^a-z0-9-]*' AND ${space.name} NOT LIKE '-%' AND ${space.name} NOT LIKE '%-' AND ${space.name} NOT LIKE '%--%'`,
		postgresql: sql`length(${space.name}) BETWEEN 1 AND 63 AND (${space.name} COLLATE "C") !~ '[^a-z0-9-]' AND ${space.name} NOT LIKE '-%' AND ${space.name} NOT LIKE '%-' AND ${space.name} NOT LIKE '%--%'`
	}))],
	indexes: { name: {
		on: ["name"],
		unique: true,
		across: account
	} },
	permissions: [
		"read",
		"list",
		"create",
		"update",
		"delete",
		"share"
	],
	shareable: { by: "share" },
	suspendable: { by: "update" },
	elevated: { delete: sudo },
	administration: [
		"read",
		"list",
		"update",
		"share"
	],
	methods: {
		get: method.get("read"),
		list: method.list("list"),
		create: method.create("create", { fields: ["name", "title"] }),
		update: method.update("update", { fields: ["title"] }),
		rename,
		delete: method.delete("delete")
	}
}, __destackModule$12);
/** A build submitted to the installation following it: an application's build, or a stack's evaluated with its space definition. */
var Submission = strictObject({
	/** The installed package. */
	packageId: PackageId,
	/** The release, repository reference or checkout the installation follows. */
	selection: InstallationSelection,
	/** The exact build evaluated. */
	build: InstallationBuild,
	/** The exported definition or parameterised function, for stacks. */
	export: string().min(1).optional(),
	/** Arguments for a parameterised definition, for stacks. */
	parameters: record$1(string(), json()).optional(),
	/** The evaluated space definition, for stacks. */
	definition: SpaceDefinition.optional()
});
_enum(["Applied", "WaitingForResources"]);
var __destackModule$11 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** A view a build's output mounts. */
var BuildView = ViewDescription.extend({ 
/** The output mounting the view. */
output: DeclarationName });
/** A location within a view, with its query, excluding another origin. */
var ViewPath = string().regex(/^\/(?!\/)[^\\\r\n]*$/);
/** A view of an installation's revision, as a client opens it: the build serving it, the view and the location within it. */
var InstallationView = strictObject({
	/** The installation. */
	installationId: identifier("installation"),
	/** The revision whose build serves the view. */
	revisionId: identifier("installation-revision"),
	/** The build serving the view. */
	build: InstallationBuild,
	/** The view's name. */
	view: DeclarationName,
	/** The view as the build's output mounts it. */
	definition: BuildView,
	/** The location within the view. */
	path: ViewPath
});
/** Plan a submitted build without changing anything. */
var plan = method({
	permission: "read",
	mutates: false,
	input: Submission,
	output: Plan
});
/** Follow a submitted build, with its plan's digest when the plan needs approval. */
var submit = method({
	permission: "apply",
	input: Submission.extend({ 
	/** The digest of the approved plan. */
approved: Digest$1.optional() }),
	output: Plan
});
/** Read the build and view a client opens at a location. */
var open = method({
	permission: "open",
	mutates: false,
	audited: true,
	input: strictObject({
		/** The view the revision declares. */
		view: DeclarationName,
		/** The location within the view. */
		path: ViewPath
	}),
	output: InstallationView
}).handle(async (call) => {
	const target = call.target;
	const { view, path } = call.input;
	const [revision] = target.revisionId === null ? [] : await call.database.select().from(installationRevision.table).where(eq(installationRevision.table.id, target.revisionId));
	const definition = revision?.views[view];
	if (revision === void 0 || definition === void 0) throw new ORPCError("NOT_FOUND", { message: `${target.alias} declares no view ${view}` });
	return InstallationView.parse({
		installationId: target.id,
		revisionId: revision.id,
		build: revision.build,
		view,
		definition,
		path
	});
});
/** Point an installation at the revision it follows as the system. */
var follow = method({
	permission: null,
	isSystem: true,
	input: strictObject({
		/** The package the revision evaluated. */
		packageId: PackageId.optional(),
		/** The release, repository reference or checkout the revision evaluated. */
		selection: InstallationSelection.optional(),
		/** The exported definition or parameterised function, for stacks. */
		export: string().min(1).optional(),
		/** Arguments for a parameterised definition, for stacks. */
		parameters: record$1(string(), json()).optional(),
		/** The revision to follow. */
		revisionId: identifier("installation-revision")
	})
}).handle((call) => call.revise(call.input));
/** A package installed into a space: its stack or one of its applications. */
var installation = defineObject({
	name: "installation",
	plural: "installations",
	scope: space,
	represents: principal.installation,
	controlled: true,
	declarable: { schema: SpaceInstallation },
	fields: {
		/** The installed package. */
		packageId: field.string(PackageId),
		/** Whether the installation declares the space's contents or serves as an application. */
		role: field.enum(["stack", "application"]).default("application"),
		/** The release, repository reference or checkout the installation follows. */
		selection: field.json(InstallationSelection),
		/** The exported space definition or parameterised function, for stacks. */
		export: field.string().optional(),
		/** Arguments supplied to the exported definition function, for stacks. */
		parameters: field.json(record$1(string(), json())).optional(),
		/** The revision to apply, absent before the first submission. */
		revisionId: field.reference(() => installationRevision).optional(),
		/** The revision fully applied, absent before the first application. */
		appliedRevisionId: field.reference(() => installationRevision).optional(),
		/** Whether the installation should serve requests. */
		status: field.enum(["enabled", "suspended"]).default("enabled"),
		/** The space-local address for this installation. */
		alias: field.string(DeclarationName),
		/** Workload compute settings checked against package and host policy. */
		compute: field.json(record$1(DeclarationName, ComputeDefinition)).default({})
	},
	constraints: (installation) => [
		foreignKey({
			columns: [installation.scope],
			foreignColumns: [space.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [installation.scope, installation.managerInstallationId],
			foreignColumns: [installation.scope, installation.id]
		}),
		unique("installation_scope_alias").on(installation.scope, installation.alias),
		unique("installation_scope_id").on(installation.scope, installation.id),
		unique("installation_space_package").on(installation.scope, installation.id, installation.packageId),
		uniqueIndex("installation_stack").on(installation.scope).where(sql`${installation.role} = 'stack'`),
		index("installation_package").on(installation.packageId),
		foreignKey({
			columns: [installation.id, installation.revisionId],
			foreignColumns: [installationRevision.table.installationId, installationRevision.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [installation.id, installation.appliedRevisionId],
			foreignColumns: [installationRevision.table.installationId, installationRevision.table.id]
		}).onDelete("restrict"),
		check("installation_stack_export", sql`(${installation.role} = 'stack') = (${installation.export} IS NOT NULL AND ${installation.parameters} IS NOT NULL)`)
	],
	relations: { self: {
		subjects: [principal.installation],
		grantedBy: null
	} },
	permissions: {
		read: none(),
		list: none(),
		create: none(),
		update: none(),
		delete: none(),
		apply: none(),
		open: none(),
		replicate: relation("self")
	},
	methods: {
		get: method.get("read"),
		list: method.list("list"),
		create: method.create("create", { fields: [
			"packageId",
			"role",
			"selection",
			"export",
			"parameters",
			"alias"
		] }),
		update: method.update("update", { fields: [
			"selection",
			"alias",
			"status"
		] }),
		delete: method.delete("delete"),
		plan,
		submit,
		open,
		follow
	}
}, __destackModule$11);
/** An immutable evaluation of an installation's selected build: the build, its views and settings, and a stack's space definition. */
var installationRevision = defineObject({
	name: "installation-revision",
	plural: "installationRevisions",
	scope: space,
	fields: {
		/** The evaluated installation. */
		installationId: field.reference(() => installation, { delete: "cascade" }),
		/** The exact build evaluated. */
		build: field.json(InstallationBuild),
		/** The views the build's outputs mount, keyed by name. */
		views: field.json(record$1(DeclarationName, BuildView)),
		/** The settings the build declares. */
		settings: field.json(array(SettingDescription)),
		/** The evaluated space definition, for stacks. */
		definition: field.json(SpaceDefinition).optional(),
		/** The SHA-256 digest of the canonical build, parameters and definition. */
		digest: field.string()
	},
	constraints: (revision) => [
		unique("installation_revision_installation_id").on(revision.installationId, revision.id),
		unique("installation_revision_digest").on(revision.installationId, revision.digest),
		check("installation_revision_digest", dialectSQL({
			sqlite: sql`length(${revision.digest}) = 64 AND ${revision.digest} NOT GLOB '*[^a-f0-9]*'`,
			postgresql: sql`(${revision.digest} COLLATE "C") ~ '^[a-f0-9]{64}$'`
		}))
	],
	permissions: ["read"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, {
			isSystem: true,
			input: strictObject({ 
			/** The package whose build the revision evaluates. */
packageId: PackageId })
		})
	}
}, __destackModule$11);
var __destackModule$10 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** The desired states a resource applied, and the plan waiting to apply them. */
var ResourceStatus = strictObject({ 
/** The applied desired states, with the plan waiting for approval or blocked until applied. */
state: strictObject({
	/** The digest of the bound desired states. */
	digest: string(),
	/** The plan waiting to apply them, absent once applied. */
	plan: Plan.optional()
}).optional() });
/** The plan an approval names. */
var approval = strictObject({ 
/** The digest of the reviewed plan. */
plan: string().min(1) });
/** Approve a plan digest for the resource's controller to apply once it plans the same steps. */
var approve = method({
	permission: "approve",
	input: approval
}).handle((call) => call.revise({ approvedPlan: approval.parse(call.input).plan }));
/** Make an installation the owner of a retained resource it declares again, cancelling its deletion. */
var own = method({
	permission: null,
	isSystem: true,
	input: strictObject({ 
	/** The installation owning the resource. */
ownerInstallationId: identifier("installation") })
}).handle((call) => {
	const { ownerInstallationId } = call.input;
	return call.revise({
		ownerInstallationId,
		deletionRequestedAt: null
	});
});
/** Infrastructure a space holds: databases, buckets and vaults, provisioned by providers. */
var resource = defineObject({
	name: "resource",
	plural: "resources",
	scope: space,
	controlled: true,
	declarable: { schema: SpaceResource },
	fields: {
		/** The space-local resource name. */
		name: field.string(),
		/** The immutable package release defining this resource kind. */
		definitionPackageId: field.string(PackageId),
		/** The defining package's calendar version. */
		definitionVersion: field.string(),
		/** The definition's exported name in the release manifest. */
		definitionName: field.string(),
		/** The desired specification validated against the definition schema. */
		spec: field.json(record$1(string(), json())),
		/** The desired states the controller applied, and the plan waiting to apply them. */
		status: field.json(ResourceStatus).default({}),
		/** The resource kind, such as database or files. */
		kind: field.string(),
		/** The requested provider, absent for automatic selection. */
		requestedProviderCode: field.string().optional(),
		/** The requested provider location within the space's residency. */
		requestedLocation: field.string().optional(),
		/** The requested host for host-managed storage. */
		requestedHostId: field.string(identifier("host")).optional(),
		/** Whether a host's provider provisions the resource, or its stack declares its reference. */
		origin: field.enum(["provisioned", "declared"]).default("provisioned"),
		/** The provider supplying the resource, absent before provisioning. */
		providerCode: field.string().optional(),
		/** The provider-assigned resource reference. */
		reference: field.string().optional(),
		/** The actual provider location, absent before provisioning. */
		location: field.string().optional(),
		/** The host holding the resource, absent for provider-managed storage. */
		hostId: field.string(identifier("host")).optional(),
		/** The digest of the plan an approver accepted, applied once the controller plans it again. */
		approvedPlan: field.string().optional(),
		/** Whether explicit resource deletion retains or destroys stored content. */
		retention: field.enum(["retain", "delete"]).default("retain"),
		/** The application installation that owns the resource, absent for stack resources and after removal. */
		ownerInstallationId: field.reference(() => installation, { delete: "null" }).optional()
	},
	constraints: (resource) => [
		foreignKey({
			columns: [resource.scope],
			foreignColumns: [space.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [resource.scope, resource.managerInstallationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}),
		check("resource_requested_location", sql`${resource.requestedLocation} IS NULL OR (${resource.requestedProviderCode} IS NOT NULL AND length(${resource.requestedLocation}) > 0)`),
		unique("resource_scope_name").on(resource.scope, resource.name),
		unique("resource_scope_id").on(resource.scope, resource.id),
		unique("resource_scope_kind").on(resource.scope, resource.id, resource.kind),
		check("resource_provider", sql`(${resource.providerCode} IS NULL) = (${resource.reference} IS NULL)`),
		check("resource_location", sql`${resource.location} IS NULL OR (${resource.providerCode} IS NOT NULL AND length(${resource.location}) > 0)`),
		check("resource_host", sql`${resource.hostId} IS NULL OR ${resource.providerCode} IS NOT NULL`)
	],
	relations: { 
	/** Installations whose live deployments captured the resource. */
reader: {
		subjects: [principal.installation],
		grantedBy: null
	} },
	permissions: {
		read: relation("reader"),
		approve: none()
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, {
			isSystem: true,
			fields: [
				"name",
				"kind",
				"definitionPackageId",
				"definitionVersion",
				"definitionName",
				"spec",
				"requestedProviderCode",
				"requestedLocation",
				"requestedHostId",
				"retention",
				"ownerInstallationId"
			]
		}),
		update: method.update(null, {
			isSystem: true,
			fields: [
				"definitionVersion",
				"spec",
				"retention"
			]
		}),
		delete: method.delete(null, { isSystem: true }),
		approve,
		own
	}
}, __destackModule$10);
var __destackModule$9 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** A host authorized to execute installations in a space. */
var spaceHost = defineObject({
	name: "space-host",
	plural: "spaceHosts",
	scope: space,
	fields: {
		/** The authorized host. */
		hostId: field.string(identifier("host")),
		/** Whether this host accepts new work or drains existing instances. */
		status: field.enum([
			"enabled",
			"draining",
			"disabled"
		]).default("enabled"),
		/** The authorization epoch used to fence revoked host credentials. */
		epoch: field.integer().default(1)
	},
	constraints: (entry) => [
		unique("space_host_scope_host").on(entry.scope, entry.hostId),
		foreignKey({
			columns: [entry.scope],
			foreignColumns: [space.table.id]
		}).onDelete("restrict"),
		check("space_host_epoch", sql`${entry.epoch} >= 1`)
	],
	permissions: ["read"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true })
	}
}, __destackModule$9);
var __destackModule$8 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Record a new instance of a deployment starting on a host, as the cell's instance controller decides. */
var start = method({
	permission: null,
	isSystem: true,
	input: strictObject({ 
	/** The host starting the instance. */
hostId: identifier("host") })
});
/** One rollout of a workload of an installation revision from one build output; at most one per workload is prepared or active. */
var deployment = defineObject({
	name: "deployment",
	plural: "deployments",
	scope: space,
	fields: {
		/** The persistent package installation. */
		installationId: field.reference(() => installation),
		/** The installed package. */
		packageId: field.string(PackageId),
		/** The installation's revision whose build the deployment runs. */
		revisionId: field.reference(() => installationRevision),
		/** The named build output. */
		output: field.string(),
		/** The package-local workload name. */
		workload: field.string(),
		/** The runtime of the selected build output. */
		runtime: field.enum(ServerRuntime.options),
		/** The package release the build runs. */
		release: field.string(Version),
		/** The oldest caller release the package's service serves, every earlier one when absent. */
		since: field.string(Version).optional(),
		/** The workload description and effective compute settings captured at preparation. */
		description: field.json(WorkloadDescription),
		/** Exact policies checked at preparation and rechecked against current revisions at activation. */
		policies: field.json(PolicySelection),
		/** An explicitly selected execution host, absent for scheduling across space hosts. */
		hostId: field.string(identifier("host")).optional(),
		/** The desired deployment availability. */
		status: field.enum([
			"prepared",
			"active",
			"draining",
			"stopping",
			"retired"
		]).default("prepared"),
		/** Controller observations, including readiness and deployment failures. */
		conditions: field.json(ConditionMap).default({}),
		/** Activation time in UTC epoch milliseconds. */
		activatedAt: field.time().optional(),
		/** Retirement time in UTC epoch milliseconds. */
		retiredAt: field.time().optional()
	},
	constraints: (entry) => [
		unique("deployment_scope_id").on(entry.scope, entry.id),
		uniqueIndex("deployment_workload").on(entry.installationId, entry.workload).where(sql`${entry.status} IN ('prepared', 'active')`),
		foreignKey({
			columns: [
				entry.scope,
				entry.installationId,
				entry.packageId
			],
			foreignColumns: [
				installation.table.scope,
				installation.table.id,
				installation.table.packageId
			]
		}).onDelete("restrict"),
		foreignKey({
			columns: [entry.scope, entry.hostId],
			foreignColumns: [spaceHost.table.scope, spaceHost.table.hostId]
		}).onDelete("restrict"),
		index("deployment_installation_status").on(entry.installationId, entry.status),
		check("deployment_workload", sql`length(${entry.workload}) > 0`),
		check("deployment_times", sql`${entry.retiredAt} IS NULL OR ${entry.activatedAt} IS NULL OR ${entry.retiredAt} >= ${entry.activatedAt}`)
	],
	permissions: ["list", "read"],
	methods: {
		get: method.get("read"),
		list: method.list("list"),
		create: method.create(null, { isSystem: true }),
		update: method.update(null, { isSystem: true }),
		delete: method.delete(null, { isSystem: true }),
		start
	}
}, __destackModule$8);
var __destackModule$7 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** An installation's binding of a package declaration to a bindable object in the space. */
var binding = defineObject({
	name: "binding",
	plural: "bindings",
	scope: space,
	declarable: { schema: SpaceBinding },
	fields: {
		/** The installation whose package declares the need. */
		installationId: field.reference(() => installation, { delete: "cascade" }),
		/** The package declaring the need. */
		packageId: field.string(PackageId),
		/** The declaration name within the package. */
		name: field.string(),
		/** The bound object's identifier with its type as prefix. */
		target: field.string(),
		/** An exact version or generation to run with; absence follows the target's current one at capture. */
		version: field.integer().optional(),
		/** The state the declaration requires of the target. */
		state: field.json(record$1(string(), json()))
	},
	constraints: (entry) => [
		foreignKey({
			columns: [entry.scope, entry.managerInstallationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}),
		unique("binding_declaration").on(entry.installationId, entry.packageId, entry.name),
		foreignKey({
			columns: [entry.scope, entry.installationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}).onDelete("cascade"),
		index("binding_target").on(entry.scope, entry.target),
		check("binding_name", sql`length(${entry.name}) > 0`),
		check("binding_version", sql`${entry.version} IS NULL OR ${entry.version} > 0`)
	],
	methods: {
		create: method.create(null, { isSystem: true }),
		update: method.update(null, {
			isSystem: true,
			fields: ["state"]
		}),
		delete: method.delete(null, { isSystem: true })
	}
}, __destackModule$7);
/** What a deployment runs with for one of its declarations: the bound object at the version or generation captured when it was prepared. */
var capture = defineObject({
	name: "capture",
	plural: "captures",
	scope: space,
	fields: {
		/** The deployment running with the capture. */
		deploymentId: field.reference(() => deployment),
		/** The package declaring the need. */
		packageId: field.string(PackageId),
		/** The declaration name within the package. */
		name: field.string(),
		/** The bound object's identifier with its type as prefix. */
		target: field.string(),
		/** The target's version or generation the deployment runs with. */
		version: field.integer(),
		/** The state the deployment's declaration requires of the target, kept until the deployment retires. */
		state: field.json(record$1(string(), json()))
	},
	constraints: (entry) => [
		unique("capture_declaration").on(entry.deploymentId, entry.packageId, entry.name),
		foreignKey({
			columns: [entry.scope, entry.deploymentId],
			foreignColumns: [deployment.table.scope, deployment.table.id]
		}).onDelete("restrict"),
		index("capture_target").on(entry.scope, entry.target),
		check("capture_version", sql`${entry.version} > 0`)
	],
	permissions: ["read"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true }),
		delete: method.delete(null, { isSystem: true })
	}
}, __destackModule$7);
var __destackModule$6 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Point a policy at the version in force, or at none, as its stack does. */
var point = method({
	permission: null,
	isSystem: true,
	input: strictObject({ 
	/** The version in force, null for none. */
currentVersionId: string().nullable() })
}).handle((call) => call.observe({ currentVersionId: call.input.currentVersionId }));
/** Outbound network rules for a space, an installation or one of its workloads. */
var networkPolicy = defineObject({
	name: "network-policy",
	plural: "networkPolicies",
	scope: [account, space],
	declarable: { schema: NetworkPolicyDefinition },
	fields: {
		/** What the policy restricts: its account, its space, one installation of the space, or one workload of it. */
		level: field.enum([
			"account",
			"space",
			"installation",
			"workload"
		]),
		/** The restricted installation, for installation and workload policies. */
		installationId: field.reference(() => installation).optional(),
		/** The package-local workload, for workload policies. */
		workload: field.string().optional(),
		/** The desired generation, the number of the version it makes current. */
		generation: field.integer().default(1),
		/** The version in force, absent while the policy has none. */
		currentVersionId: field.reference(() => networkPolicyVersion, { delete: "null" }).optional()
	},
	constraints: (policy) => [
		foreignKey({
			columns: [policy.scope, policy.managerInstallationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}),
		foreignKey({
			columns: [policy.scope, policy.installationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}).onDelete("restrict"),
		uniqueIndex("network_policy_installation").on(policy.installationId).where(sql`${policy.level} = 'installation'`),
		uniqueIndex("network_policy_workload").on(policy.installationId, policy.workload).where(sql`${policy.level} = 'workload'`),
		uniqueIndex("network_policy_scope").on(policy.scope).where(sql`${policy.level} IN ('account', 'space')`),
		check("network_policy_level", sql`
            (${policy.level} IN ('account', 'space') AND ${policy.installationId} IS NULL AND ${policy.workload} IS NULL) OR
            (${policy.level} = 'installation' AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NULL) OR
            (${policy.level} = 'workload' AND ${policy.installationId} IS NOT NULL AND ${policy.workload} IS NOT NULL AND length(${policy.workload}) > 0)
        `),
		check("network_policy_generation", sql`${policy.generation} > 0`)
	],
	permissions: ["read", "update"],
	methods: { point }
}, __destackModule$6);
/** An immutable version of a network policy. */
var networkPolicyVersion = defineObject({
	name: "network-policy-version",
	plural: "networkPolicyVersions",
	scope: [account, space],
	nested: {
		in: networkPolicy,
		delete: "cascade",
		receive: "update"
	},
	versioned: true,
	fields: {
		/** The policy as this version defines it. */
		definition: field.json(NetworkPolicyDefinition),
		/** The stack that declared this version, absent for versions written directly. */
		source: field.json(Manager.schema).optional(),
		/** The SHA-256 digest of the canonical definition. */
		digest: field.string(string().min(64).max(64))
	},
	permissions: {
		read: through("parent", "read"),
		update: through("parent", "update")
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true })
	}
}, __destackModule$6);
/** The packages a space admits. */
var packagePolicy = defineObject({
	name: "package-policy",
	plural: "packagePolicies",
	scope: [account, space],
	declarable: { schema: PackagePolicyDefinition },
	fields: {
		/** What the policy restricts: its account or its space. */
		level: field.enum(["account", "space"]),
		/** The desired generation, the number of the version it makes current. */
		generation: field.integer().default(1),
		/** The version in force, absent while the policy has none. */
		currentVersionId: field.reference(() => packagePolicyVersion, { delete: "null" }).optional()
	},
	constraints: (policy) => [
		foreignKey({
			columns: [policy.scope, policy.managerInstallationId],
			foreignColumns: [installation.table.scope, installation.table.id]
		}),
		uniqueIndex("package_policy_scope").on(policy.scope),
		check("package_policy_generation", sql`${policy.generation} > 0`)
	],
	permissions: ["read", "update"],
	methods: { point }
}, __destackModule$6);
/** An immutable version of a package policy. */
var packagePolicyVersion = defineObject({
	name: "package-policy-version",
	plural: "packagePolicyVersions",
	scope: [account, space],
	nested: {
		in: packagePolicy,
		delete: "cascade",
		receive: "update"
	},
	versioned: true,
	fields: {
		/** The policy as this version defines it. */
		definition: field.json(PackagePolicyDefinition),
		/** The stack that declared this version, absent for versions written directly. */
		source: field.json(Manager.schema).optional(),
		/** The SHA-256 digest of the canonical definition. */
		digest: field.string(string().min(64).max(64))
	},
	permissions: {
		read: through("parent", "read"),
		update: through("parent", "update")
	},
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true })
	}
}, __destackModule$6);
/** The values of a webhook route's parameters, by name. */
var WebhookParameters = defineSchema(record$1(string(), string().min(1)));
/** One verified webhook delivery. */
var WebhookDelivery = defineSchema(strictObject({
	/** The sender's delivery identifier. */
	id: string().min(1),
	/** The event type, such as push. */
	event: string().min(1),
	/** The decoded body. */
	payload: json(),
	/** The route's parameters. */
	parameters: WebhookParameters,
	/** The receiving time, in UTC epoch milliseconds. */
	receivedAt: number().int()
}));
var __destackModule$5 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Where a run stands: waiting for an attempt, attempting, or finished one of three ways. */
var RUN_STATES = [
	"pending",
	"running",
	"succeeded",
	"failed",
	"skipped"
];
/** Why a run failed or was skipped. */
var RunError = defineSchema(strictObject({
	/** The stable failure code, such as the handler error's code. */
	code: string().min(1),
	/** The failure message. */
	message: string()
}));
/** One delivery of a trigger's cause to an installation's workload. */
var run$1 = defineObject({
	name: "run",
	plural: "runs",
	scope: space,
	fields: {
		/** The installation whose workload runs. */
		installation: field.reference(installation, { delete: "cascade" }),
		/** The kind of trigger delivered. */
		kind: field.enum(TRIGGER_KINDS),
		/** The package declaring the trigger. */
		packageId: field.string(PackageId),
		/** The trigger's name within its package. */
		trigger: field.string(string().min(1)),
		/** The occurrence a schedule delivers. */
		scheduledAt: field.time().optional(),
		/** The webhook delivery. */
		deliveryId: field.string(string().min(1)).optional(),
		/** The webhook delivery's event and body. */
		delivery: field.json(WebhookDelivery.omit({ id: true })).optional(),
		/** The log epoch of a watch's change. */
		epoch: field.string(string().min(1)).optional(),
		/** The log sequence of a watch's change. */
		sequence: field.integer().optional(),
		/** The row a snapshot delivers at one log position. */
		key: field.string(string().min(1)).optional(),
		/** The shape of the watched table's changes when the watch read this run's change. */
		shape: field.string().optional(),
		/** Where the run stands. */
		state: field.enum(RUN_STATES),
		/** Whether runs of the trigger may overlap. */
		concurrency: field.enum(CONCURRENCIES),
		/** The attempts started so far. */
		attempts: field.integer(),
		/** The first attempt's start, in UTC epoch milliseconds. */
		startedAt: field.time().optional(),
		/** The finish, in UTC epoch milliseconds. */
		finishedAt: field.time().optional(),
		/** Why the run failed or was skipped. */
		error: field.json(RunError).optional(),
		/** The trace of the run's attempts. */
		traceId: field.string(string().min(1)).optional()
	},
	constraints: (run) => [
		uniqueIndex("run_schedule_cause").on(run.installation, run.packageId, run.trigger, run.scheduledAt).where(sql`${run.kind} = 'schedule'`),
		uniqueIndex("run_webhook_cause").on(run.installation, run.packageId, run.trigger, run.deliveryId).where(sql`${run.kind} = 'webhook'`),
		uniqueIndex("run_watch_cause").on(run.installation, run.packageId, run.trigger, run.epoch, run.sequence).where(sql`${run.kind} = 'watch' AND ${run.key} IS NULL`),
		uniqueIndex("run_snapshot_cause").on(run.installation, run.packageId, run.trigger, run.epoch, run.sequence, run.key).where(sql`${run.kind} = 'watch' AND ${run.key} IS NOT NULL`),
		uniqueIndex("run_claim").on(run.installation, run.kind, run.packageId, run.trigger).where(sql`${run.state} IN ('pending', 'running') AND ${run.concurrency} <> 'allow'`),
		check("run_cause", sql`(${run.kind} = 'schedule' AND ${run.scheduledAt} IS NOT NULL AND ${run.deliveryId} IS NULL AND ${run.delivery} IS NULL AND ${run.epoch} IS NULL AND ${run.sequence} IS NULL AND ${run.key} IS NULL) OR (${run.kind} = 'webhook' AND ${run.deliveryId} IS NOT NULL AND ${run.delivery} IS NOT NULL AND ${run.scheduledAt} IS NULL AND ${run.epoch} IS NULL AND ${run.sequence} IS NULL AND ${run.key} IS NULL) OR (${run.kind} = 'watch' AND ${run.epoch} IS NOT NULL AND ${run.sequence} IS NOT NULL AND ${run.scheduledAt} IS NULL AND ${run.deliveryId} IS NULL AND ${run.delivery} IS NULL)`),
		check("run_attempts", sql`${run.attempts} >= 0`),
		check("run_finish", sql`(${run.state} IN ('succeeded', 'failed', 'skipped')) = (${run.finishedAt} IS NOT NULL)`)
	],
	permissions: { read: through("installation", "read") },
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true }),
		update: method.update(null, { isSystem: true })
	}
}, __destackModule$5);
var __destackModule$4 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Record an instance stopped, as its cell's instance controller decides. */
var stop = method({
	permission: null,
	isSystem: true
});
/** An observed running occurrence of a deployment on one host, restarted in place after failures. */
var instance = defineObject({
	name: "instance",
	plural: "instances",
	scope: space,
	fields: {
		/** The immutable deployment being executed. */
		deploymentId: field.reference(() => deployment),
		/** The host administering this instance. */
		hostId: field.string(identifier("host")),
		/** The space-host authorization epoch captured when execution started. */
		hostEpoch: field.integer(),
		/** The provider's instance reference, when one is available. */
		reference: field.string().optional(),
		/** The latest observed execution status. */
		status: field.enum([
			"starting",
			"running",
			"draining",
			"stopped",
			"failed"
		]),
		/** Host observations and failure details. */
		conditions: field.json(ConditionMap).default({}),
		/** The last authenticated observation in UTC epoch milliseconds. */
		observedAt: field.time(),
		/** The expiry of the host's execution lease, when execution uses leases. */
		leaseExpiresAt: field.time().optional(),
		/** The execution start time in UTC epoch milliseconds. */
		startedAt: field.time().optional(),
		/** The execution finish time in UTC epoch milliseconds, and of the last failure while failed. */
		stoppedAt: field.time().optional(),
		/** The restarts after failures since the instance was recorded. */
		restarts: field.integer().default(0)
	},
	constraints: (entry) => [
		foreignKey({
			columns: [entry.scope, entry.deploymentId],
			foreignColumns: [deployment.table.scope, deployment.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [entry.scope, entry.hostId],
			foreignColumns: [spaceHost.table.scope, spaceHost.table.hostId]
		}).onDelete("restrict"),
		index("instance_deployment_status").on(entry.deploymentId, entry.status),
		index("instance_host_status").on(entry.hostId, entry.status),
		check("instance_epoch", sql`${entry.hostEpoch} > 0`),
		check("instance_times", sql`${entry.stoppedAt} IS NULL OR ${entry.startedAt} IS NULL OR ${entry.stoppedAt} >= ${entry.startedAt}`)
	],
	permissions: ["read"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true }),
		update: method.update(null, { isSystem: true }),
		delete: method.delete(null, { isSystem: true }),
		stop
	}
}, __destackModule$4);
var __destackModule$3 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** Record a transfer's success once its target took the space over, as the target does. */
var complete = method({
	permission: null,
	isSystem: true
}).handle((call) => call.revise({
	outcome: "succeeded",
	completedAt: call.now
}));
/** A handoff of a space with its rows and resources from the host or region holding it to another. */
var transfer = defineObject({
	name: "transfer",
	plural: "transfers",
	scope: space,
	fields: {
		/** The host or region holding the space until the transfer activates. */
		source: field.string(),
		/** The host or region receiving the space. */
		target: field.string(),
		/** The epoch the source holds the space at. */
		sourceEpoch: field.integer(),
		/** The verified identity requesting the transfer. */
		requestedBy: field.json(Subject),
		/** The terminal outcome; an interrupted transfer keeps its state for retry. */
		outcome: field.enum(["succeeded", "cancelled"]).optional(),
		/** The completion time in UTC epoch milliseconds. */
		completedAt: field.time().optional(),
		/** Progress and failures retained while recovery remains required. */
		conditions: field.json(ConditionMap).default({})
	},
	constraints: (transfer) => [
		foreignKey({
			columns: [transfer.scope],
			foreignColumns: [space.table.id]
		}).onDelete("restrict"),
		unique("transfer_scope_id").on(transfer.scope, transfer.id),
		check("transfer_holder", sql`${transfer.source} <> ${transfer.target}`),
		uniqueIndex("transfer_active").on(transfer.scope).where(sql`${transfer.completedAt} IS NULL`),
		check("transfer_epoch", sql`${transfer.sourceEpoch} > 0`),
		check("transfer_completion", sql`(${transfer.outcome} IS NULL) = (${transfer.completedAt} IS NULL)`),
		check("transfer_times", sql`${transfer.completedAt} IS NULL OR ${transfer.completedAt} >= ${transfer.createdAt}`)
	],
	permissions: ["read", "create"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("create", { fields: ["target"] }),
		complete
	}
}, __destackModule$3);
/** The part of a transfer moving one resource: where its provider held it on the source, and where the target's provider holds it now. */
var resourceTransfer = defineObject({
	name: "resource-transfer",
	plural: "resourceTransfers",
	scope: space,
	fields: {
		/** The transfer moving the resource. */
		transferId: field.reference(() => transfer),
		/** The resource. */
		resourceId: field.reference(() => resource),
		/** The provider holding the resource on the source. */
		sourceProviderCode: field.string(),
		/** The source provider's reference. */
		sourceReference: field.string(),
		/** The source host, absent for provider-managed storage. */
		sourceHostId: field.string(identifier("host")).optional(),
		/** The provider holding the resource on the target. */
		targetProviderCode: field.string(),
		/** The target provider's reference. */
		targetReference: field.string(),
		/** The target host, absent for provider-managed storage. */
		targetHostId: field.string(identifier("host")).optional(),
		/** The time the target's provider held all of the fenced source's content. */
		copiedAt: field.time()
	},
	constraints: (entry) => [
		unique("resource_transfer_resource").on(entry.transferId, entry.resourceId),
		foreignKey({
			columns: [entry.scope],
			foreignColumns: [space.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [entry.scope, entry.transferId],
			foreignColumns: [transfer.table.scope, transfer.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [entry.scope, entry.resourceId],
			foreignColumns: [resource.table.scope, resource.table.id]
		}).onDelete("cascade")
	],
	permissions: ["read"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create(null, { isSystem: true })
	}
}, __destackModule$3);
var __destackModule$2 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** A retained recovery point of a resource's committed state. */
var snapshot$1 = defineObject({
	name: "snapshot",
	plural: "snapshots",
	scope: space,
	controlled: true,
	fields: {
		/** The resource the snapshot captures. */
		resourceId: field.reference(() => resource),
		/** The earliest time retention permits removal. */
		retainUntil: field.time(),
		/** The time the recovery point became usable, absent while the controller takes it. */
		readyAt: field.time().optional(),
		/** The provider retaining the recovery point. */
		providerCode: field.string().optional(),
		/** The provider location retaining the snapshot. */
		location: field.string().optional(),
		/** The provider's immutable recovery reference. */
		reference: field.string().optional(),
		/** The recovery format interpreted by the controller. */
		format: field.string().optional(),
		/** The committed source position represented by the snapshot. */
		position: field.string().optional(),
		/** The integrity digest, when supplied by the format. */
		digest: field.string().optional(),
		/** Completion of the integrity and completeness checks. */
		verifiedAt: field.time().optional()
	},
	constraints: (snapshot) => [
		unique("snapshot_scope_id").on(snapshot.scope, snapshot.id),
		foreignKey({
			columns: [snapshot.scope, snapshot.resourceId],
			foreignColumns: [resource.table.scope, resource.table.id]
		}).onDelete("restrict"),
		unique("snapshot_resource_id").on(snapshot.resourceId, snapshot.id),
		check("snapshot_retention", sql`${snapshot.retainUntil} >= ${snapshot.createdAt} AND (${snapshot.deletionRequestedAt} IS NULL OR ${snapshot.deletionRequestedAt} >= ${snapshot.retainUntil})`),
		check("snapshot_ready", sql`(${snapshot.readyAt} IS NULL) = (${snapshot.providerCode} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.reference} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.format} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.position} IS NULL) AND (${snapshot.readyAt} IS NOT NULL OR (${snapshot.location} IS NULL AND ${snapshot.digest} IS NULL)) AND (${snapshot.readyAt} IS NULL OR ${snapshot.readyAt} >= ${snapshot.createdAt})`),
		check("snapshot_verification", sql`${snapshot.verifiedAt} IS NULL OR (${snapshot.readyAt} IS NOT NULL AND ${snapshot.verifiedAt} >= ${snapshot.readyAt})`)
	],
	permissions: [
		"read",
		"create",
		"delete"
	],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("create", { fields: ["resourceId", "retainUntil"] }),
		delete: method.delete("delete")
	}
}, __destackModule$2);
/** A restoration of a snapshot into a resource. */
var restoration$1 = defineObject({
	name: "restoration",
	plural: "restorations",
	scope: space,
	fields: {
		/** The recovery point selected for restoration. */
		snapshotId: field.reference(() => snapshot$1),
		/** The destination resource; the controller verifies it is safe to initialize. */
		destinationResourceId: field.reference(() => resource),
		/** The first restoration attempt. */
		startedAt: field.time().optional(),
		/** Completion time, absent while pending. */
		completedAt: field.time().optional(),
		/** The terminal outcome. */
		outcome: field.enum([
			"succeeded",
			"failed",
			"cancelled"
		]).optional(),
		/** The last reported failure. */
		error: field.string().optional()
	},
	constraints: (restoration) => [
		foreignKey({
			columns: [restoration.scope, restoration.snapshotId],
			foreignColumns: [snapshot$1.table.scope, snapshot$1.table.id]
		}).onDelete("restrict"),
		foreignKey({
			columns: [restoration.scope, restoration.destinationResourceId],
			foreignColumns: [resource.table.scope, resource.table.id]
		}).onDelete("restrict"),
		check("restoration_completion", sql`(${restoration.completedAt} IS NULL) = (${restoration.outcome} IS NULL)`),
		check("restoration_success", sql`${restoration.outcome} IS NULL OR ${restoration.outcome} <> 'succeeded' OR (${restoration.startedAt} IS NOT NULL AND ${restoration.error} IS NULL)`),
		check("restoration_finish", sql`${restoration.completedAt} IS NULL OR ${restoration.startedAt} IS NULL OR ${restoration.completedAt} >= ${restoration.startedAt}`),
		check("restoration_time", sql`(${restoration.startedAt} IS NULL OR ${restoration.startedAt} >= ${restoration.createdAt}) AND (${restoration.completedAt} IS NULL OR ${restoration.completedAt} >= ${restoration.createdAt})`)
	],
	permissions: ["read", "create"],
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("create", { fields: ["snapshotId", "destinationResourceId"] })
	}
}, __destackModule$2);
/** Snapshots on the server. */
var snapshot = snapshot$1.handle({
	create: async () => {
		throw new ORPCError("NOT_IMPLEMENTED", { message: "no controller takes snapshots yet" });
	},
	delete: async () => {
		throw new ORPCError("NOT_IMPLEMENTED", { message: "no controller removes snapshots yet" });
	}
});
/** Restorations on the server. */
var restoration = restoration$1.handle({ create: async () => {
	throw new ORPCError("NOT_IMPLEMENTED", { message: "no controller runs restorations yet" });
} });
var __destackModule$1 = Object.freeze({ "package": {
	"id": "package-019f5530-8000-7000-8000-000000000001",
	"name": "@destack/setting",
	"version": "2026.9.0"
} });
/** A setting value placed in a scope. */
var setting = defineObject({
	name: "setting",
	plural: "settings",
	scope: [
		user,
		space,
		account,
		organisation,
		host
	],
	declarable: { schema: SpaceSetting },
	inherited: { where: Condition.ne("mode", "set") },
	fields: {
		/** The package declaring the setting. */
		packageId: field.string(PackageId),
		/** The setting's name in its declaring package. */
		name: field.string(SettingName),
		/** The consuming package the value applies to. */
		package: field.string(PackageId).optional(),
		/** The space the value applies in. */
		space: field.string(identifier("space")).optional(),
		/** The installation the value applies to. */
		installation: field.string(identifier("installation")).optional(),
		/** The user's device the value applies on. */
		device: field.reference(device, { delete: "cascade" }).optional(),
		/** How the value applies. */
		mode: field.enum(SETTING_MODES),
		/** The value. */
		value: field.json(json()),
		/** The release of the setting's package the value was written against. */
		release: field.string(Version)
	},
	permissions: ["read", "write"],
	detachable: { by: "write" },
	methods: {
		get: method.get("read"),
		list: method.list("read"),
		create: method.create("write"),
		update: method.update("write", { fields: [
			"mode",
			"value",
			"release"
		] }),
		delete: method.delete("write")
	},
	constraints: (value) => [
		uniqueIndex("setting_placement").on(value.packageId, value.name, value.scope, sql`coalesce(${value.package}, '')`, sql`coalesce(${value.space}, '')`, sql`coalesce(${value.installation}, '')`, sql`coalesce(${value.device}, '')`),
		index("setting_scope").on(value.scope, value.id),
		check("setting_override", sql`(${value.installation} IS NULL OR ${value.space} IS NULL) AND (${value.mode} = 'set' OR (${value.package} IS NULL AND ${value.space} IS NULL AND ${value.device} IS NULL))`)
	]
}, __destackModule$1);
/** Identify a space. */
var SpaceKey = strictObject({ spaceId: identifier("space") });
SpaceKey.extend({ installationId: identifier("installation") });
/** Where a source stood once fenced: the log position the target's copy reaches, and the digest of the space's rows at it. */
var Fence = strictObject({
	/** The source's log position once fenced. */
	position: LogPosition,
	/** The SHA-256 digest of the space's rows at the position, as hexadecimal. */
	digest: string().min(1)
});
/** One record of a resource's export: a chunk of its content, or the end once every chunk was sent. */
var ExportRecord = union([strictObject({ 
/** The next chunk. */
chunk: Chunk$1 }), strictObject({ 
/** Whether the export sent every chunk. */
end: literal(true) })]);
/** The relay of a space's zone from a transfer's source to its target. */
var relay = {
	/** Follow the space's rows from a position, or from a snapshot without one. */
	watch: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/spaces/{spaceId}/relay/watch"
	}).input(SpaceKey.extend({ 
	/** The log position the target's copy holds, absent before its first snapshot. */
after: LogPosition.optional() })).output(eventIterator(QueryPage)),
	/** Stop serving the space's reads and writes, recording where the source stood; a repeat answers the same. */
	fence: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: "activity"
	}).route({
		method: "POST",
		path: "/spaces/{spaceId}/relay/fence"
	}).input(SpaceKey).output(Fence),
	/** Read a resource's content after a cursor, sealing secrets to the target's recipient key, and mark the end. */
	export: defineProcedure({
		authentication: "identity",
		permission: null,
		audit: false
	}).route({
		method: "POST",
		path: "/spaces/{spaceId}/relay/resources/{resourceId}/export"
	}).input(SpaceKey.extend({
		/** The resource. */
		resourceId: identifier("resource"),
		/** How far the copy goes. */
		stage: CopyStage,
		/** The cursor of the last chunk the target imported, absent at the start. */
		after: string().min(1).optional(),
		/** The target's recipient key for this copy, as base64 of its P-256 point. */
		recipient: base64()
	})).output(eventIterator(ExportRecord))
};
var __destackModule = Object.freeze({ "package": {
	"id": "package-01a0c80b-614e-739c-9b50-70e5119010b1",
	"name": "@destack/space",
	"version": "2026.9.0"
} });
/** HTTP procedures exposed by space administration. */
var spaceService = defineService("space", {
	/** Spaces, their installations, and the records their controllers write. */
	objects: {
		space,
		resource,
		installation,
		installationRevision,
		binding,
		networkPolicy,
		networkPolicyVersion,
		packagePolicy,
		packagePolicyVersion,
		deployment,
		capture,
		instance,
		spaceHost,
		transfer,
		resourceTransfer,
		snapshot,
		restoration,
		run: run$1,
		setting,
		user,
		account,
		zone
	},
	/** The relay of the space's zone to the target of its transfer. */
	relay
}, __destackModule);
/** Connect to space administration. */
function connect(options) {
	return createClient(spaceService, options);
}
/** Run a workload in this process as its host starts it, with its package's resources. */
async function run(workload, resources) {
	await runWorkload({
		workload,
		resources,
		history: (url, secret) => {
			const client = createAuditClient({
				url,
				headers: () => ({ authorization: `Bearer ${secret}` })
			});
			return { ingest: (batch, options) => client.ingest({ events: batch.events.map((event) => AuditEvent.parse(event)) }, options) };
		},
		replicas: (url, secret) => {
			const client = connect({
				url,
				headers: () => ({ authorization: `Bearer ${secret}` })
			});
			return { async *stream(request, signal) {
				yield* await client.replica.stream(request, { signal });
			} };
		}
	}, console[Symbol.asyncIterator](), async (ready) => {
		process.stdout.write(`${JSON.stringify(ready)}\n`);
	});
	process.exit(0);
}
await run(web, {
	"main": database,
	"credentials": vault,
	"notes": notes
});
export {};

//# sourceMappingURL=workload-web-DwV9oTB9.js.map