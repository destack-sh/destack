import { $n as Package, Ar as __esmMin, C as init_diag, D as init_global_utils, Dn as sql, Dr as uuidv7, Dt as Expression, E as getGlobal, Et as dialectSQL, Jn as ResourceHandle, K as DatabaseTier, O as registerGlobal, P as Condition, Qn as Entrypoint, S as DiagAPI, Sr as string, T as init_types, Tr as union$1, Wn as Resource, X as declareState, Xn as DeclarationName, Yn as declaringModule, _ as ContextAPI, _r as lazy$1, _t as TABLE, a as ProxyTracer, ar as defineSchema, b as createContextKey, br as record, bt as expandTrees, c as isSpanContextValid, cr as _enum, d as INVALID_SPANID, dr as boolean, er as PackageId, f as INVALID_SPAN_CONTEXT, ft as v7, g as init_trace_flags, h as TraceFlags, i as init_ProxyTracerProvider, ir as identifier, j as isAfter, jr as __exportAll, k as unregisterGlobal, kt as check, l as isValidSpanId, lr as array, m as init_invalid_span_constants, n as trace, o as init_ProxyTracer, p as INVALID_TRACEID, q as DatabaseState, qn as defineResourceKind, r as ProxyTracerProvider, rr as Version, s as init_spancontext_utils, t as init_trace_api, u as isValidTraceId, v as init_context, vr as literal, w as DiagLogLevel, x as init_context$1, xr as strictObject, y as ROOT_CONTEXT, yr as number } from "./trace-api-DJ-_ZhlJ.js";
/** CPU and memory capacity assigned to one running instance. */
var ComputeCapacity = defineSchema(strictObject({
	/** CPU capacity in cores. */
	cpu: number().positive().optional(),
	/** Memory capacity in MiB. */
	memory: number().int().positive().optional()
}));
/** Capacity and lifecycle policy for a workload. */
var ComputeDefinition = defineSchema(strictObject({
	/** Minimum capacity requested when scheduling an instance. */
	requests: ComputeCapacity.optional(),
	/** Maximum capacity allowed for an instance. */
	limits: ComputeCapacity.optional(),
	/** Instance scaling bounds. */
	scaling: strictObject({
		/** Minimum warm instances, zero to stop every idle instance. */
		minInstances: number().int().nonnegative().optional(),
		/** Maximum simultaneous instances. */
		maxInstances: number().int().positive().optional()
	}).optional(),
	/** Time in milliseconds to retain an idle instance. */
	idleTimeout: number().int().nonnegative().optional(),
	/** Time in milliseconds allowed for graceful shutdown. */
	shutdownTimeout: number().int().positive().optional(),
	/** CPU time allowed per invocation in milliseconds. */
	cpuTime: number().int().positive().optional()
}));
/** A declaration qualified by its declaring package, as bindings, permissions and events store it. */
var DeclarationReference = defineSchema(strictObject({
	/** The immutable identity of the declaring package. */
	packageId: PackageId,
	/** The package-local declaration name. */
	name: DeclarationName
}));
/** Reference a declaration by its package identity and name. */
function reference(declaration) {
	return {
		packageId: declaration.package.id,
		name: declaration.name
	};
}
/** A unit of deployment, as its declaration defines it. */
var WorkloadDefinition = defineSchema(strictObject({
	/** The package-local workload name. */
	name: DeclarationName,
	/** Capacity and lifecycle policy for each instance. */
	compute: ComputeDefinition.optional()
}).strict());
/** A workload located in a compiled output with the declarations its code reaches. */
var WorkloadDescription = defineSchema(strictObject({
	/** The package entrypoint running the workload. */
	entrypoint: Entrypoint,
	/** Service declarations of this package reachable from the workload. */
	services: array(DeclarationReference),
	/** Trigger declarations (schedules, webhooks, subscriptions) of this package reachable from the workload. */
	triggers: array(DeclarationReference),
	/** Resource declarations reachable from the workload. */
	resources: array(DeclarationReference),
	/** Secret declarations reachable from the workload. */
	secrets: array(DeclarationReference),
	/** Service connection declarations reachable from the workload. */
	connections: array(DeclarationReference),
	/** Capacity and lifecycle policy for each instance. */
	compute: ComputeDefinition
}));
/** The connectors opening databases on Bun: SQLite files, loaded on first connect. */
var connectors = { sqlite: {
	code: "sqlite",
	connect: async (bound, declaration) => {
		const { sqliteConnector } = await import("./connector-DYjyYlo7.js");
		return sqliteConnector.connect(bound, declaration);
	}
} };
/** A database's resource settings. */
var DatabaseSpec = defineSchema(strictObject({ tier: DatabaseTier }));
/** The database resource kind: tables planned toward the union of their declared states. */
var DatabaseKind = defineResourceKind("database", {
	spec: DatabaseSpec,
	state: DatabaseState
});
/** A database declaration. */
var Database = class extends Resource {
	/** The tables the database holds, referencing tables held elsewhere without foreign keys. */
	tables;
	/** Create the declaration. */
	constructor(owner, description, tables) {
		super(owner, description);
		this.tables = tables;
	}
	/** The connectors opening databases on the running runtime. */
	get connectors() {
		return connectors;
	}
	/** Describe the required tables. */
	state() {
		return { tables: {
			sqlite: declareState(this.tables, "sqlite"),
			postgresql: declareState(this.tables, "postgresql")
		} };
	}
	/** Read the connection. */
	get(context) {
		return context.get(this);
	}
	/** Name the required tables a connected database has not applied. */
	check(connection) {
		return connection.unapplied(this.tables);
	}
};
/**
* Declare a database.
*
* A database holds tables of its own tier and of wider ones, whose rows it replicates from their home.
*/
function defineDatabase(definition, module) {
	const owner = declaringModule(module, "defineDatabase").package;
	const tier = definition.tier ?? "zonal";
	const tiers = DatabaseTier.options;
	const names = /* @__PURE__ */ new Map();
	for (const table of expandTrees(definition.tables)) {
		const { sqlName, tier: declared } = table[TABLE];
		const existing = names.get(sqlName);
		if (existing && existing !== table) throw new TypeError(`duplicate SQL table: ${sqlName}`);
		if (declared !== void 0 && tiers.indexOf(declared) > tiers.indexOf(tier)) throw new TypeError(`${declared} table ${sqlName} in a ${tier} database`);
		names.set(sqlName, table);
	}
	return new Database(owner, DatabaseKind.description.parse({
		name: definition.name,
		kind: "database",
		spec: { tier }
	}), [...names.values()]);
}
/** A managed collection of encrypted secrets. */
var VaultSpec = defineSchema(strictObject({}));
/** The vault resource kind: encrypted secrets. */
var VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** Declare a vault resource. */
function defineVault(declaration, module) {
	const owner = declaringModule(module, "defineVault").package;
	return new Resource(owner, VaultKind.description.parse({
		...declaration,
		kind: "vault"
	}));
}
/** A secret selected when installing a package. */
var SecretDescription = defineSchema(strictObject({ 
/** The package-local secret name. */
name: DeclarationName }));
/** A package's declaration of a secret it reads. */
var SecretDeclaration = class extends ResourceHandle {
	/** Retain validated metadata without acquiring credentials. */
	constructor(owner, declaration) {
		super(owner, declaration.name);
	}
};
/** Declare a secret without embedding its value. */
function defineSecret(declaration, module) {
	const owner = declaringModule(module, "defineSecret").package;
	return new SecretDeclaration(owner, SecretDescription.parse(declaration));
}
/** The trigger kinds. */
var TRIGGER_KINDS = [
	"schedule",
	"webhook",
	"watch"
];
/** Pair a trigger with its handler. */
function handleTrigger(handle) {
	return {
		trigger: this,
		handle
	};
}
/** How a trigger's runs may overlap. */
var CONCURRENCIES = [
	"allow",
	"forbid",
	"replace"
];
/** The common schedule fields. */
var SCHEDULE = strictObject({
	/** The package-local schedule name. */
	name: DeclarationName,
	/** Whether occurrences may overlap. */
	concurrency: _enum(CONCURRENCIES),
	/** How late an occurrence may start, in milliseconds. */
	deadline: number().int().nonnegative()
});
/** A schedule declaration. */
var ScheduleDescription = defineSchema(union$1([
	SCHEDULE.extend({
		/** A calendar schedule. */
		timing: literal("cron"),
		/** A five-field cron expression. */
		cron: string().regex(/^\S+\s+\S+\s+\S+\s+\S+\s+\S+$/),
		/** The IANA time zone. */
		timezone: string().min(1),
		/** The earliest occurrence time, in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative().optional(),
		/** The exclusive end time, in UTC epoch milliseconds. */
		endsAt: number().int().nonnegative().optional()
	}),
	SCHEDULE.extend({
		/** A repeating interval schedule. */
		timing: literal("interval"),
		/** The interval, in milliseconds. */
		interval: number().int().positive(),
		/** The first occurrence time, in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative(),
		/** The exclusive end time, in UTC epoch milliseconds. */
		endsAt: number().int().nonnegative().optional()
	}),
	SCHEDULE.extend({
		/** A one-off schedule. */
		timing: literal("once"),
		/** The occurrence time, in UTC epoch milliseconds. */
		startsAt: number().int().nonnegative()
	})
]));
defineSchema(strictObject({ 
/** The due time, in UTC epoch milliseconds. */
scheduledAt: number().int().nonnegative() }));
/** Declare a schedule. */
function defineSchedule(definition, module) {
	const owner = declaringModule(module, "defineSchedule").package;
	const description = ScheduleDescription.parse(definition);
	return Object.freeze({
		...description,
		kind: "schedule",
		package: owner,
		handle: handleTrigger
	});
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
function get(object, path) {
	let current = object;
	for (const key of path) {
		if (!isTypescriptObject(current)) return;
		current = current[key];
	}
	return current;
}
var NullProtoObj = /* @__PURE__ */ (() => {
	const e = function() {};
	e.prototype = /* @__PURE__ */ Object.create(null);
	Object.freeze(e.prototype);
	return e;
})();
function value(value2, ...args) {
	if (typeof value2 === "function") return value2(...args);
	return value2;
}
function preventNativeAwait(target) {
	return new Proxy(target, { get(target2, prop, receiver) {
		const value2 = Reflect.get(target2, prop, receiver);
		if (prop !== "then" || typeof value2 !== "function") return value2;
		return new Proxy(value2, { apply(targetFn, thisArg, args) {
			if (args.length !== 2 || args.some((arg) => !isNativeFunction(arg))) return Reflect.apply(targetFn, thisArg, args);
			let shouldOmit = true;
			args[0].call(thisArg, preventNativeAwait(new Proxy(target2, { get: (target3, prop2, receiver2) => {
				if (shouldOmit && prop2 === "then") {
					shouldOmit = false;
					return;
				}
				return Reflect.get(target3, prop2, receiver2);
			} })));
		} });
	} });
}
var NATIVE_FUNCTION_REGEX = /^\s*function\s*\(\)\s*\{\s*\[native code\]\s*\}\s*$/;
function isNativeFunction(fn) {
	return typeof fn === "function" && NATIVE_FUNCTION_REGEX.test(fn.toString());
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
var RECURSIVE_CLIENT_UNWRAP_KEYS = /* @__PURE__ */ new Set([
	"bind",
	"valueOf",
	"toString",
	"toJSON"
]);
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
function mergeStandardHeaders(a, b) {
	const merged = { ...a };
	for (const key in b) if (Array.isArray(b[key])) merged[key] = [...toArray(merged[key]), ...b[key]];
	else if (b[key] !== void 0) {
		if (Array.isArray(merged[key])) merged[key] = [...merged[key], b[key]];
		else if (merged[key] !== void 0) merged[key] = [merged[key], b[key]];
		else merged[key] = b[key];
	}
	return merged;
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
function resolveFriendlyClientOptions(options) {
	return {
		...options,
		context: options.context ?? {}
	};
}
function createORPCClient(link, options = {}) {
	const path = options.path ?? [];
	const procedureClient = async (...[input, options2 = {}]) => {
		return await link.call(path, input, resolveFriendlyClientOptions(options2));
	};
	return preventNativeAwait(new Proxy(procedureClient, { get(target, key) {
		if (typeof key !== "string" || RECURSIVE_CLIENT_UNWRAP_KEYS.has(key)) return Reflect.get(target, key);
		return createORPCClient(link, {
			...options,
			path: [...path, key]
		});
	} }));
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
function toStandardHeaders$1(headers, standardHeaders = {}) {
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
			const headers = toStandardHeaders$1(request.headers);
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
function toFetchRequest(request, options = {}) {
	const headers = toFetchHeaders(request.headers);
	const body = toFetchBody(request.body, headers, options);
	return new Request(request.url, {
		signal: request.signal,
		method: request.method,
		headers,
		body
	});
}
function toFetchResponse(response, options = {}) {
	const headers = toFetchHeaders(response.headers);
	const body = toFetchBody(response.body, headers, options);
	return new Response(body, {
		headers,
		status: response.status
	});
}
function toStandardLazyResponse(response, options = {}) {
	return {
		body: once(() => toStandardBody(response, options)),
		status: response.status,
		get headers() {
			const headers = toStandardHeaders$1(response.headers);
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
var CompositeStandardLinkPlugin = class {
	plugins;
	constructor(plugins = []) {
		this.plugins = [...plugins].sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
	}
	init(options) {
		for (const plugin of this.plugins) plugin.init?.(options);
	}
};
var StandardLink = class {
	constructor(codec, sender, options = {}) {
		this.codec = codec;
		this.sender = sender;
		new CompositeStandardLinkPlugin(options.plugins).init(options);
		this.interceptors = toArray(options.interceptors);
		this.clientInterceptors = toArray(options.clientInterceptors);
	}
	interceptors;
	clientInterceptors;
	call(path, input, options) {
		return runWithSpan({
			name: `${ORPC_NAME}.${path.join("/")}`,
			signal: options.signal
		}, (span) => {
			span?.setAttribute("rpc.system", ORPC_NAME);
			span?.setAttribute("rpc.method", path.join("."));
			if (isAsyncIteratorObject(input)) input = asyncIteratorWithSpan({
				name: "consume_event_iterator_input",
				signal: options.signal
			}, input);
			return intercept(this.interceptors, {
				...options,
				path,
				input
			}, async ({ path: path2, input: input2, ...options2 }) => {
				const otelConfig = getGlobalOtelConfig();
				let otelContext;
				const currentSpan = otelConfig?.trace.getActiveSpan() ?? span;
				if (currentSpan && otelConfig) otelContext = otelConfig?.trace.setSpan(otelConfig.context.active(), currentSpan);
				const request = await runWithSpan({
					name: "encode_request",
					context: otelContext
				}, () => this.codec.encode(path2, input2, options2));
				const response = await intercept(this.clientInterceptors, {
					...options2,
					input: input2,
					path: path2,
					request
				}, ({ input: input3, path: path3, request: request2, ...options3 }) => {
					return runWithSpan({
						name: "send_request",
						signal: options3.signal,
						context: otelContext
					}, () => this.sender.call(request2, options3, path3, input3));
				});
				const output = await runWithSpan({
					name: "decode_response",
					context: otelContext
				}, () => this.codec.decode(response, options2, path2, input2));
				if (isAsyncIteratorObject(output)) return asyncIteratorWithSpan({
					name: "consume_event_iterator_output",
					signal: options2.signal
				}, output);
				return output;
			});
		});
	}
};
function toHttpPath(path) {
	return `/${path.map(encodeURIComponent).join("/")}`;
}
function toStandardHeaders(headers) {
	if (typeof headers.forEach === "function") return toStandardHeaders$1(headers);
	return headers;
}
function getMalformedResponseErrorCode(status) {
	return Object.entries(COMMON_ORPC_ERROR_DEFS).find(([, def]) => def.status === status)?.[0] ?? "MALFORMED_ORPC_ERROR_RESPONSE";
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
var EVENT_ITERATOR_DETAILS_SYMBOL = Symbol("ORPC_EVENT_ITERATOR_DETAILS");
function eventIterator(yields, returns) {
	return { "~standard": {
		[EVENT_ITERATOR_DETAILS_SYMBOL]: {
			yields,
			returns
		},
		vendor: "orpc",
		version: 1,
		validate(iterator) {
			if (!isAsyncIteratorObject(iterator)) return { issues: [{
				message: "Expect event iterator",
				path: []
			}] };
			return { value: mapEventIterator(iterator, {
				async value(value, done) {
					const schema = done ? returns : yields;
					if (!schema) return value;
					const result = await schema["~standard"].validate(value);
					if (result.issues) throw new ORPCError("EVENT_ITERATOR_VALIDATION_FAILED", {
						message: "Event iterator validation failed",
						cause: new ValidationError({
							issues: result.issues,
							message: "Event iterator validation failed",
							data: value
						})
					});
					return result.value;
				},
				error: async (error) => error
			}) };
		}
	} };
}
/** An object: its type, the scope containing it, and its identifier there. */
var ObjectReference = defineSchema(strictObject({
	/** The package that declares the object type. */
	packageId: identifier("package"),
	/** The declaration-local object type name. */
	type: string().regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/),
	/** The scope containing the object, the container for a scope object. */
	scope: string().min(1),
	/** The stable application record identity. */
	id: string().min(1)
}));
/** A package-qualified object type. */
var ObjectTypeReference = defineSchema(ObjectReference.pick({
	packageId: true,
	type: true
}));
/** A scalar request attribute that permission conditions compare. */
var Attribute = union$1([
	string(),
	number().finite(),
	boolean()
]);
/** A stable declaration-local name used by access rules. */
var AccessName = string().regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/);
/** Constrain a column holding access names to their lowercase kebab case. */
function nameCheck(name, column) {
	return check(name, dialectSQL({
		sqlite: sql`${column} IS NULL OR (length(${column}) > 0 AND substr(${column}, 1, 1) GLOB '[a-z]' AND ${column} NOT GLOB '*[^a-z0-9-]*' AND ${column} NOT LIKE '%--%' AND ${column} NOT LIKE '%-')`,
		postgresql: sql`${column} IS NULL OR (${column} COLLATE "C") ~ '^[a-z][a-z0-9]*(-[a-z0-9]+)*$'`
	}));
}
/** Match no relation, so only roles grant the permission. */
function none() {
	return { kind: "none" };
}
/** Construct a relation membership expression. */
function relation(name) {
	return {
		kind: "relation",
		name: AccessName.parse(name)
	};
}
/** Reference another permission on the same object. */
function permission(name) {
	return {
		kind: "permission",
		name: AccessName.parse(name)
	};
}
/** Permit subjects accepted by any expression. */
function union(...expressions) {
	return {
		kind: "union",
		expressions
	};
}
/** Require every expression to permit access. */
function intersection(...expressions) {
	return {
		kind: "intersection",
		expressions
	};
}
/** Follow an object relation once or through its ancestor closure. */
function through(relation, permission, transitive = false) {
	return {
		kind: "through",
		relation: AccessName.parse(relation),
		permission: AccessName.parse(permission),
		transitive
	};
}
/** Permit whoever may grant on the object a row references, through the grant permission of that object's own type. */
function grants(reference) {
	return {
		kind: "grants",
		reference: AccessName.parse(reference)
	};
}
/** Permit rows with attributes that meet a condition over request attributes. */
function condition(condition) {
	return {
		kind: "condition",
		condition
	};
}
/** A type a relation accepts as subject: its objects, a subject set of them, or all of them. */
var SubjectType = defineSchema(strictObject({
	/** The package declaring the subject type. */
	packageId: PackageId,
	/** The declaration-local subject type name. */
	type: AccessName,
	/** The relation whose members are the subjects, for a subject set. */
	relation: AccessName.optional(),
	/** Whether one relationship relates every object of the type. */
	wildcard: literal(true).optional()
}));
/** A subject: a principal or other object, a subject set of its relation's members, or every object of a type. */
var Subject = defineSchema(strictObject({
	/** The package declaring the subject type. */
	packageId: PackageId,
	/** The declaration-local subject type name. */
	type: AccessName,
	/** The scope containing the subject, or `*` for every scope. */
	scope: string().min(1),
	/** The subject identifier, or `*` for every object of the type in the scope. */
	id: string().min(1),
	/** The relation whose members are the subjects, for a subject set. */
	relation: AccessName.optional()
}));
/** Construct a collision-free subject key. */
function subjectKey(subject) {
	return JSON.stringify([
		subject.packageId,
		subject.type,
		subject.scope,
		subject.id,
		subject.relation ?? null
	]);
}
/** Read a subject back from its key. */
function keySubject(key) {
	const [packageId, type, scope, id, relation] = JSON.parse(key);
	return Subject.parse({
		packageId,
		type,
		scope,
		id,
		...relation === null ? {} : { relation }
	});
}
/** Compare two subjects completely. */
function sameSubject(left, right) {
	return subjectKey(left) === subjectKey(right);
}
/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
var AccessError = class extends Error {
	/** The stable failure classification. */
	code;
	/** The authentication that would admit the caller, for insufficient authentication. */
	stepUp;
	/** Retain the error classification and cause, and the authentication that would admit the caller. */
	constructor(code, message, options) {
		super(message, options);
		this.name = "AccessError";
		this.code = code;
		if (options?.stepUp !== void 0) this.stepUp = options.stepUp;
	}
};
/** The authentication a sensitive permission requires, like OpenID Connect's `acr_values` and `max_age`. */
var Elevation = {
	/** The schema of an elevation. */
	schema: defineSchema(strictObject({
		/** The lowest assurance level: 1 for one factor, 2 for several, 3 for phishing-resistant factors. */
		assurance: number().int().min(1).max(3),
		/** The longest time since the caller authenticated at that level, in milliseconds. */
		maxAge: number().int().positive()
	})),
	admits,
	until
};
/** Report whether a request authenticated at the elevation's level within its age. */
function admits(elevation, context) {
	const assurance = context.assurance;
	return assurance !== void 0 && assurance.level >= elevation.assurance && context.now - assurance.authenticatedAt <= elevation.maxAge;
}
/** Read the moment an admitted request's authentication grows too old for the elevation. */
function until(elevation, context) {
	return admits(elevation, context) ? context.assurance.authenticatedAt + elevation.maxAge : void 0;
}
/** Serialized permission expressions with no executable callbacks. */
var AccessExpressionDescription = lazy$1(() => union$1([
	strictObject({ kind: literal("none") }),
	strictObject({
		kind: _enum(["relation", "permission"]),
		name: AccessName
	}),
	strictObject({
		kind: _enum(["union", "intersection"]),
		expressions: array(AccessExpressionDescription).min(1)
	}),
	strictObject({
		kind: literal("exclusion"),
		include: AccessExpressionDescription,
		exclude: AccessExpressionDescription
	}),
	strictObject({
		kind: literal("condition"),
		condition: Condition.schema
	}),
	strictObject({
		kind: literal("through"),
		relation: AccessName,
		permission: AccessName,
		transitive: boolean()
	}),
	strictObject({
		kind: literal("grants"),
		reference: AccessName
	})
]));
/** A type's relations and permissions as data. */
var PolicyDescription = defineSchema(strictObject({
	/** The stable declaring package identity. */
	packageId: PackageId,
	/** The declaration-local object type name. */
	name: AccessName,
	/** The scalar type of each object attribute that permission expressions read. */
	attributes: record(AccessName, _enum([
		"string",
		"number",
		"boolean"
	])),
	/** The relations to subjects and to other objects. */
	relations: record(AccessName, strictObject({
		/** The subject types the relation accepts itself, beside those other types contribute to an open relation. */
		subjects: array(SubjectType),
		/** The permission whose holders grant and revoke the relation. */
		grantedBy: AccessName.optional(),
		/** Whether other types contribute themselves as subject types. */
		open: literal(true).optional()
	})),
	/** The open relations of other types this type's objects may be subjects of. */
	contributes: array(strictObject({
		/** The package declaring the type with the open relation. */
		packageId: PackageId,
		/** The type with the open relation. */
		type: AccessName,
		/** The open relation. */
		relation: AccessName
	})).optional(),
	/** The named permission expressions. */
	permissions: record(AccessName, AccessExpressionDescription),
	/** The permission required to bind roles on an object. */
	grantedBy: AccessName.optional(),
	/** The permissions only their expressions grant. */
	reserved: array(AccessName).optional(),
	/** The permissions that apply only after the authentication each names. */
	elevated: record(AccessName, Elevation.schema).optional(),
	/** Permissions that stay available while the scope is suspended. */
	administration: array(AccessName).optional(),
	/** Whether the objects are scopes. */
	scope: literal(true).optional(),
	/** Whether the type's objects live in the universe and an identifier alone refers to each. */
	isGlobal: literal(true).optional()
}));
/** The policy of one type of object, with typed references to its permissions and relations. */
var Policy = class Policy {
	/** The declaring package, supplied by the module transform. */
	package;
	/** The package-local object type name. */
	name;
	/** The immutable serializable definition. */
	definition;
	/** The other policies the relations name as subjects. */
	references;
	/** Qualify the declared subject types by package and freeze a copy of the rules. */
	constructor(owner, input) {
		const relations = Object.fromEntries(Object.entries(input.relations ?? {}).map(([name, relation]) => {
			const grantedBy = relation.grantedBy === void 0 ? input.grantedBy : relation.grantedBy;
			return [name, {
				subjects: relation.subjects.map((subject) => subjectType(owner.id, subject)),
				...grantedBy === void 0 || grantedBy === null ? {} : { grantedBy },
				...relation.open ? { open: true } : {}
			}];
		}));
		for (const [name, relation] of Object.entries(relations)) if (relation.subjects.length === 0 && !relation.open) throw new AccessError("INVALID_DECLARATION", `relation ${name} accepts no subject type and is not open`);
		this.references = [.../* @__PURE__ */ new Set([...Object.values(input.relations ?? {}).flatMap((relation) => relation.subjects).flatMap((subject) => subject instanceof Policy ? [subject] : subject instanceof PolicySubject ? [subject.policy] : []), ...(input.contributes ?? []).map((entry) => entry.policy)])];
		this.package = owner;
		this.name = input.name;
		this.definition = freeze(PolicyDescription.parse({
			packageId: owner.id,
			name: input.name,
			attributes: input.attributes ?? {},
			relations,
			permissions: input.permissions,
			...input.grantedBy === void 0 ? {} : { grantedBy: input.grantedBy },
			...input.reserved === void 0 ? {} : { reserved: [...input.reserved] },
			...input.elevated === void 0 ? {} : { elevated: { ...input.elevated } },
			...input.administration === void 0 ? {} : { administration: [...input.administration] },
			...input.scope ? { scope: true } : {},
			...input.isGlobal ? { isGlobal: true } : {},
			...input.contributes === void 0 || input.contributes.length === 0 ? {} : { contributes: input.contributes.map((entry) => ({
				packageId: entry.policy.definition.packageId,
				type: entry.policy.definition.name,
				relation: entry.relation
			})) }
		}));
	}
	/** Identify an object of this type in the scope containing it. */
	reference(scope, id) {
		return ObjectReference.parse({
			packageId: this.definition.packageId,
			type: this.definition.name,
			scope,
			id
		});
	}
	/** Reference one of this type's declared permissions. */
	permission(name) {
		if (!Object.hasOwn(this.definition.permissions, name)) throw new AccessError("INVALID_DECLARATION", `unknown permission: ${name}`);
		return {
			packageId: this.definition.packageId,
			type: this.definition.name,
			name
		};
	}
	/** Determine whether a subject is one object of this type, rather than a set or another type's object. */
	is(subject) {
		return subject.relation === void 0 && subject.packageId === this.definition.packageId && subject.type === this.definition.name;
	}
	/** Accept the members of one of this type's relations as subjects. */
	members(relation) {
		return new PolicySubject(this, AccessName.parse(relation), false);
	}
	/** Accept every object of this type through one relationship. */
	all() {
		return new PolicySubject(this, void 0, true);
	}
};
/** A subject type taken from a policy: the members of one of its relations, or all of its objects. */
var PolicySubject = class {
	/** The policy whose objects are the subjects. */
	policy;
	/** The relation whose members are the subjects, for a subject set. */
	relation;
	/** Whether one relationship relates every object of the type. */
	isWildcard;
	/** Retain the policy and the form of its subjects. */
	constructor(policy, relation, isWildcard) {
		this.policy = policy;
		this.relation = relation;
		this.isWildcard = isWildcard;
	}
	/** Qualify the subject type by the policy's package. */
	type() {
		return {
			packageId: this.policy.definition.packageId,
			type: this.policy.definition.name,
			...this.relation === void 0 ? {} : { relation: this.relation },
			...this.isWildcard ? { wildcard: true } : {}
		};
	}
};
/** A stable reference to a declared permission, independent of a package version. */
var PermissionReference = defineSchema(strictObject({
	/** The package that declares the permission. */
	packageId: PackageId,
	/** The declaration-local object type name. */
	type: AccessName,
	/** The permission name within that object type. */
	name: AccessName
}));
/** Qualify a declared subject type with the package declaring it. */
function subjectType(packageId, subject) {
	if (subject instanceof Policy) return {
		packageId: subject.definition.packageId,
		type: subject.definition.name
	};
	else if (subject instanceof PolicySubject) return subject.type();
	else if (typeof subject !== "string") return SubjectType.parse(subject);
	const match = /^([a-z][a-z0-9-]*)(?:#([a-z][a-z0-9-]*)|(:\*))?$(?![\s\S])/.exec(subject);
	if (!match) throw new AccessError("INVALID_DECLARATION", `invalid subject type: ${subject}`);
	return SubjectType.parse({
		packageId,
		type: match[1],
		...match[2] === void 0 ? {} : { relation: match[2] },
		...match[3] === void 0 ? {} : { wildcard: true }
	});
}
/** Construct a collision-free permission key. */
function permissionKey(permission) {
	return JSON.stringify([
		permission.packageId,
		permission.type,
		permission.name
	]);
}
/** Construct a collision-free reference key. */
function objectKey(reference) {
	return JSON.stringify([
		reference.packageId,
		reference.type,
		reference.scope,
		reference.id
	]);
}
/** Freeze every declaration node after copying it. */
function freeze(value) {
	if (value !== null && typeof value === "object") {
		for (const child of Object.values(value)) freeze(child);
		Object.freeze(value);
	}
	return value;
}
/** List the relations an expression reads, directly or through an arrow. */
function relationsOf(expression) {
	switch (expression.kind) {
		case "relation": return [expression.name];
		case "through": return [expression.relation];
		case "union":
		case "intersection": return expression.expressions.flatMap(relationsOf);
		case "exclusion": return [...relationsOf(expression.include), ...relationsOf(expression.exclude)];
		default: return [];
	}
}
/** The access and audit requirements of a procedure. */
var ProcedureAccess = strictObject({
	/** The required credentials. */
	authentication: _enum([
		"public",
		"identity",
		"host"
	]),
	/** The permission checked on the call's target, null when the handler checks itself. */
	permission: PermissionReference.nullable(),
	/** The category of the event a call records when it ends, or false to record only denials. */
	audit: union$1([literal(false), _enum(["activity", "access"])])
});
/** The declared requirements and conversions of each procedure, parsed once. */
var METAS = /* @__PURE__ */ new WeakMap();
/** A procedure's declared requirements and the conversions of inputs from earlier releases. */
var ProcedureMeta = Object.assign(ProcedureAccess.extend({ 
/** The input fields each release computes from an earlier input's fields, by the release introducing them. */
convert: record(Version, record(string().min(1), Expression.schema)).optional() }), { 
/** Read a procedure's declared requirements and conversions, parsing them once. */
of(procedure) {
	let meta = METAS.get(procedure);
	if (meta === void 0) {
		meta = ProcedureMeta.parse(procedure["~orpc"].meta);
		METAS.set(procedure, meta);
	}
	return meta;
} });
/** Declare a procedure with its access requirements and the conversions of earlier inputs. */
function defineProcedure(meta) {
	return oc.$meta(meta).errors({
		NOT_IMPLEMENTED: { status: 501 },
		UNAUTHORIZED: { status: 401 },
		INSUFFICIENT_AUTHENTICATION: { status: 401 },
		FORBIDDEN: { status: 403 },
		NOT_FOUND: { status: 404 },
		CONFLICT: { status: 409 },
		PRECONDITION_FAILED: { status: 412 },
		MANAGED: { status: 409 },
		MOVED: { status: 421 },
		UNSUPPORTED: { status: 422 },
		RATE_LIMITED: { status: 429 },
		UNAVAILABLE: { status: 503 }
	});
}
/** Declare an HTTP service. */
function defineService(name, input, module) {
	const owner = Package.parse(declaringModule(module, "defineService").package);
	const { objects = {}, since, ...router } = input;
	const derived = { ...router };
	const shared = {};
	for (const [key, routed] of Object.entries(objects)) {
		derived[key] = routed.procedures;
		Object.assign(shared, routed.shared);
	}
	const taken = [
		...Object.keys(router),
		...Object.keys(objects),
		...Object.keys(shared)
	];
	const collision = taken.find((key, position) => taken.indexOf(key) !== position);
	if (collision !== void 0) throw new TypeError(`service ${name} routes two procedures under ${collision}`);
	Object.assign(derived, shared);
	return Object.freeze({
		package: owner,
		name: DeclarationName.parse(name),
		...since === void 0 ? {} : { since: Version.parse(since) },
		protocol: "http",
		router: derived,
		objects
	});
}
var BaggageImpl;
var init_baggage_impl = __esmMin((() => {
	BaggageImpl = class BaggageImpl {
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
}));
var baggageEntryMetadataSymbol;
var init_symbol = __esmMin((() => {
	baggageEntryMetadataSymbol = Symbol("BaggageEntryMetadata");
}));
/**
* Create a new Baggage with optional entries
*
* @param entries An array of baggage entries the new baggage should contain
*/
function createBaggage(entries = {}) {
	return new BaggageImpl(new Map(Object.entries(entries)));
}
/**
* Create a serializable BaggageEntryMetadata object from a string.
*
* @param str string metadata. Format is currently not defined by the spec and has no special meaning.
*
* @since 1.0.0
*/
function baggageEntryMetadataFromString(str) {
	if (typeof str !== "string") {
		diag$1.error(`Cannot create baggage metadata from unknown type: ${typeof str}`);
		str = "";
	}
	return {
		__TYPE__: baggageEntryMetadataSymbol,
		toString() {
			return str;
		}
	};
}
var diag$1;
var init_utils$1 = __esmMin((() => {
	init_diag();
	init_baggage_impl();
	init_symbol();
	diag$1 = DiagAPI.instance();
}));
var consoleMap, _originalConsoleMethods, DiagConsoleLogger;
var init_consoleLogger = __esmMin((() => {
	consoleMap = [
		{
			n: "error",
			c: "error"
		},
		{
			n: "warn",
			c: "warn"
		},
		{
			n: "info",
			c: "info"
		},
		{
			n: "debug",
			c: "debug"
		},
		{
			n: "verbose",
			c: "trace"
		}
	];
	_originalConsoleMethods = {};
	if (typeof console !== "undefined") {
		for (const key of [
			"error",
			"warn",
			"info",
			"debug",
			"trace",
			"log"
		]) if (typeof console[key] === "function") _originalConsoleMethods[key] = console[key];
	}
	DiagConsoleLogger = class {
		constructor() {
			function _consoleFunc(funcName) {
				return function(...args) {
					let theFunc = _originalConsoleMethods[funcName];
					if (typeof theFunc !== "function") theFunc = _originalConsoleMethods["log"];
					if (typeof theFunc !== "function" && console) {
						theFunc = console[funcName];
						if (typeof theFunc !== "function") theFunc = console.log;
					}
					if (typeof theFunc === "function") return theFunc.apply(console, args);
				};
			}
			for (let i = 0; i < consoleMap.length; i++) this[consoleMap[i].n] = _consoleFunc(consoleMap[i].c);
		}
	};
}));
/**
* Create a no-op Meter
*
* @since 1.3.0
*/
function createNoopMeter() {
	return NOOP_METER;
}
var NoopMeter, NoopMetric, NoopCounterMetric, NoopUpDownCounterMetric, NoopGaugeMetric, NoopHistogramMetric, NoopObservableMetric, NoopObservableCounterMetric, NoopObservableGaugeMetric, NoopObservableUpDownCounterMetric, NOOP_METER, NOOP_COUNTER_METRIC, NOOP_GAUGE_METRIC, NOOP_HISTOGRAM_METRIC, NOOP_UP_DOWN_COUNTER_METRIC, NOOP_OBSERVABLE_COUNTER_METRIC, NOOP_OBSERVABLE_GAUGE_METRIC, NOOP_OBSERVABLE_UP_DOWN_COUNTER_METRIC;
var init_NoopMeter = __esmMin((() => {
	NoopMeter = class {
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
	NoopMetric = class {};
	NoopCounterMetric = class extends NoopMetric {
		add(_value, _attributes) {}
	};
	NoopUpDownCounterMetric = class extends NoopMetric {
		add(_value, _attributes) {}
	};
	NoopGaugeMetric = class extends NoopMetric {
		record(_value, _attributes) {}
	};
	NoopHistogramMetric = class extends NoopMetric {
		record(_value, _attributes) {}
	};
	NoopObservableMetric = class {
		addCallback(_callback) {}
		removeCallback(_callback) {}
	};
	NoopObservableCounterMetric = class extends NoopObservableMetric {};
	NoopObservableGaugeMetric = class extends NoopObservableMetric {};
	NoopObservableUpDownCounterMetric = class extends NoopObservableMetric {};
	NOOP_METER = new NoopMeter();
	NOOP_COUNTER_METRIC = new NoopCounterMetric();
	NOOP_GAUGE_METRIC = new NoopGaugeMetric();
	NOOP_HISTOGRAM_METRIC = new NoopHistogramMetric();
	NOOP_UP_DOWN_COUNTER_METRIC = new NoopUpDownCounterMetric();
	NOOP_OBSERVABLE_COUNTER_METRIC = new NoopObservableCounterMetric();
	NOOP_OBSERVABLE_GAUGE_METRIC = new NoopObservableGaugeMetric();
	NOOP_OBSERVABLE_UP_DOWN_COUNTER_METRIC = new NoopObservableUpDownCounterMetric();
}));
var ValueType;
var init_Metric = __esmMin((() => {
	(function(ValueType) {
		ValueType[ValueType["INT"] = 0] = "INT";
		ValueType[ValueType["DOUBLE"] = 1] = "DOUBLE";
	})(ValueType || (ValueType = {}));
}));
var defaultTextMapGetter, defaultTextMapSetter;
var init_TextMapPropagator = __esmMin((() => {
	defaultTextMapGetter = {
		get(carrier, key) {
			if (carrier == null) return;
			return carrier[key];
		},
		keys(carrier) {
			if (carrier == null) return [];
			return Object.keys(carrier);
		}
	};
	defaultTextMapSetter = { set(carrier, key, value) {
		if (carrier == null) return;
		carrier[key] = value;
	} };
}));
var SamplingDecision;
var init_SamplingResult = __esmMin((() => {
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
}));
var SpanKind;
var init_span_kind = __esmMin((() => {
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
}));
var SpanStatusCode;
var init_status = __esmMin((() => {
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
}));
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
var VALID_KEY_CHAR_RANGE, VALID_KEY_REGEX, VALID_VALUE_BASE_REGEX, INVALID_VALUE_COMMA_EQUAL_REGEX;
var init_tracestate_validators = __esmMin((() => {
	VALID_KEY_CHAR_RANGE = "[_0-9a-z-*/]";
	VALID_KEY_REGEX = new RegExp(`^(?:${`[a-z]${VALID_KEY_CHAR_RANGE}{0,255}`}|${`[a-z0-9]${VALID_KEY_CHAR_RANGE}{0,240}@[a-z]${VALID_KEY_CHAR_RANGE}{0,13}`})$`);
	VALID_VALUE_BASE_REGEX = /^[ -~]{0,255}[!-~]$/;
	INVALID_VALUE_COMMA_EQUAL_REGEX = /,|=/;
}));
var MAX_TRACE_STATE_ITEMS, MAX_TRACE_STATE_LEN, LIST_MEMBERS_SEPARATOR, LIST_MEMBER_KEY_VALUE_SPLITTER, TraceStateImpl;
var init_tracestate_impl = __esmMin((() => {
	init_tracestate_validators();
	MAX_TRACE_STATE_ITEMS = 32;
	MAX_TRACE_STATE_LEN = 512;
	LIST_MEMBERS_SEPARATOR = ",";
	LIST_MEMBER_KEY_VALUE_SPLITTER = "=";
	TraceStateImpl = class TraceStateImpl {
		constructor(rawTraceState) {
			this._internalState = /* @__PURE__ */ new Map();
			if (rawTraceState) this._parse(rawTraceState);
		}
		set(key, value) {
			const traceState = this._clone();
			if (traceState._internalState.has(key)) traceState._internalState.delete(key);
			traceState._internalState.set(key, value);
			return traceState;
		}
		unset(key) {
			const traceState = this._clone();
			traceState._internalState.delete(key);
			return traceState;
		}
		get(key) {
			return this._internalState.get(key);
		}
		serialize() {
			return Array.from(this._internalState.keys()).reduceRight((agg, key) => {
				agg.push(key + LIST_MEMBER_KEY_VALUE_SPLITTER + this.get(key));
				return agg;
			}, []).join(LIST_MEMBERS_SEPARATOR);
		}
		_parse(rawTraceState) {
			if (rawTraceState.length > MAX_TRACE_STATE_LEN) return;
			this._internalState = rawTraceState.split(LIST_MEMBERS_SEPARATOR).reduceRight((agg, part) => {
				const listMember = part.trim();
				const i = listMember.indexOf(LIST_MEMBER_KEY_VALUE_SPLITTER);
				if (i !== -1) {
					const key = listMember.slice(0, i);
					const value = listMember.slice(i + 1, part.length);
					if (validateKey(key) && validateValue(value)) agg.set(key, value);
				}
				return agg;
			}, /* @__PURE__ */ new Map());
			if (this._internalState.size > MAX_TRACE_STATE_ITEMS) this._internalState = new Map(Array.from(this._internalState.entries()).reverse().slice(0, MAX_TRACE_STATE_ITEMS));
		}
		_keys() {
			return Array.from(this._internalState.keys()).reverse();
		}
		_clone() {
			const traceState = new TraceStateImpl();
			traceState._internalState = new Map(this._internalState);
			return traceState;
		}
	};
}));
/**
* @since 1.1.0
*/
function createTraceState(rawTraceState) {
	return new TraceStateImpl(rawTraceState);
}
var init_utils = __esmMin((() => {
	init_tracestate_impl();
}));
var context;
var init_context_api = __esmMin((() => {
	init_context();
	context = ContextAPI.getInstance();
}));
var diag;
var init_diag_api = __esmMin((() => {
	init_diag();
	diag = DiagAPI.instance();
}));
var NoopMeterProvider, NOOP_METER_PROVIDER;
var init_NoopMeterProvider = __esmMin((() => {
	init_NoopMeter();
	NoopMeterProvider = class {
		getMeter(_name, _version, _options) {
			return NOOP_METER;
		}
	};
	NOOP_METER_PROVIDER = new NoopMeterProvider();
}));
var API_NAME$1, MetricsAPI;
var init_metrics = __esmMin((() => {
	init_NoopMeterProvider();
	init_global_utils();
	init_diag();
	API_NAME$1 = "metrics";
	MetricsAPI = class MetricsAPI {
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
			return registerGlobal(API_NAME$1, provider, DiagAPI.instance());
		}
		/**
		* Returns the global meter provider.
		*/
		getMeterProvider() {
			return getGlobal(API_NAME$1) || NOOP_METER_PROVIDER;
		}
		/**
		* Returns a meter from the global meter provider.
		*/
		getMeter(name, version, options) {
			return this.getMeterProvider().getMeter(name, version, options);
		}
		/** Remove the global meter provider */
		disable() {
			unregisterGlobal(API_NAME$1, DiagAPI.instance());
		}
	};
}));
var metrics;
var init_metrics_api = __esmMin((() => {
	init_metrics();
	metrics = MetricsAPI.getInstance();
}));
var NoopTextMapPropagator;
var init_NoopTextMapPropagator = __esmMin((() => {
	NoopTextMapPropagator = class {
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
}));
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
var BAGGAGE_KEY;
var init_context_helpers = __esmMin((() => {
	init_context();
	init_context$1();
	BAGGAGE_KEY = createContextKey("OpenTelemetry Baggage Key");
}));
var API_NAME, NOOP_TEXT_MAP_PROPAGATOR, PropagationAPI;
var init_propagation = __esmMin((() => {
	init_global_utils();
	init_NoopTextMapPropagator();
	init_TextMapPropagator();
	init_context_helpers();
	init_utils$1();
	init_diag();
	API_NAME = "propagation";
	NOOP_TEXT_MAP_PROPAGATOR = new NoopTextMapPropagator();
	PropagationAPI = class PropagationAPI {
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
			return registerGlobal(API_NAME, propagator, DiagAPI.instance());
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
			unregisterGlobal(API_NAME, DiagAPI.instance());
		}
		_getGlobalPropagator() {
			return getGlobal(API_NAME) || NOOP_TEXT_MAP_PROPAGATOR;
		}
	};
}));
var propagation;
var init_propagation_api = __esmMin((() => {
	init_propagation();
	propagation = PropagationAPI.getInstance();
}));
var esm_exports = /* @__PURE__ */ __exportAll({
	DiagConsoleLogger: () => DiagConsoleLogger,
	DiagLogLevel: () => DiagLogLevel,
	INVALID_SPANID: () => INVALID_SPANID,
	INVALID_SPAN_CONTEXT: () => INVALID_SPAN_CONTEXT,
	INVALID_TRACEID: () => INVALID_TRACEID,
	ProxyTracer: () => ProxyTracer,
	ProxyTracerProvider: () => ProxyTracerProvider,
	ROOT_CONTEXT: () => ROOT_CONTEXT,
	SamplingDecision: () => SamplingDecision,
	SpanKind: () => SpanKind,
	SpanStatusCode: () => SpanStatusCode,
	TraceFlags: () => TraceFlags,
	ValueType: () => ValueType,
	baggageEntryMetadataFromString: () => baggageEntryMetadataFromString,
	context: () => context,
	createContextKey: () => createContextKey,
	createNoopMeter: () => createNoopMeter,
	createTraceState: () => createTraceState,
	default: () => esm_default,
	defaultTextMapGetter: () => defaultTextMapGetter,
	defaultTextMapSetter: () => defaultTextMapSetter,
	diag: () => diag,
	isSpanContextValid: () => isSpanContextValid,
	isValidSpanId: () => isValidSpanId,
	isValidTraceId: () => isValidTraceId,
	metrics: () => metrics,
	propagation: () => propagation,
	trace: () => trace
});
var esm_default;
var init_esm = __esmMin((() => {
	init_utils$1();
	init_context$1();
	init_consoleLogger();
	init_types();
	init_NoopMeter();
	init_Metric();
	init_TextMapPropagator();
	init_ProxyTracer();
	init_ProxyTracerProvider();
	init_SamplingResult();
	init_span_kind();
	init_status();
	init_trace_flags();
	init_utils();
	init_spancontext_utils();
	init_invalid_span_constants();
	init_context_api();
	init_diag_api();
	init_metrics_api();
	init_propagation_api();
	init_trace_api();
	esm_default = {
		context,
		diag,
		metrics,
		propagation,
		trace
	};
}));
init_esm();
var SeverityNumber;
(function(SeverityNumber) {
	SeverityNumber[SeverityNumber["UNSPECIFIED"] = 0] = "UNSPECIFIED";
	SeverityNumber[SeverityNumber["TRACE"] = 1] = "TRACE";
	SeverityNumber[SeverityNumber["TRACE2"] = 2] = "TRACE2";
	SeverityNumber[SeverityNumber["TRACE3"] = 3] = "TRACE3";
	SeverityNumber[SeverityNumber["TRACE4"] = 4] = "TRACE4";
	SeverityNumber[SeverityNumber["DEBUG"] = 5] = "DEBUG";
	SeverityNumber[SeverityNumber["DEBUG2"] = 6] = "DEBUG2";
	SeverityNumber[SeverityNumber["DEBUG3"] = 7] = "DEBUG3";
	SeverityNumber[SeverityNumber["DEBUG4"] = 8] = "DEBUG4";
	SeverityNumber[SeverityNumber["INFO"] = 9] = "INFO";
	SeverityNumber[SeverityNumber["INFO2"] = 10] = "INFO2";
	SeverityNumber[SeverityNumber["INFO3"] = 11] = "INFO3";
	SeverityNumber[SeverityNumber["INFO4"] = 12] = "INFO4";
	SeverityNumber[SeverityNumber["WARN"] = 13] = "WARN";
	SeverityNumber[SeverityNumber["WARN2"] = 14] = "WARN2";
	SeverityNumber[SeverityNumber["WARN3"] = 15] = "WARN3";
	SeverityNumber[SeverityNumber["WARN4"] = 16] = "WARN4";
	SeverityNumber[SeverityNumber["ERROR"] = 17] = "ERROR";
	SeverityNumber[SeverityNumber["ERROR2"] = 18] = "ERROR2";
	SeverityNumber[SeverityNumber["ERROR3"] = 19] = "ERROR3";
	SeverityNumber[SeverityNumber["ERROR4"] = 20] = "ERROR4";
	SeverityNumber[SeverityNumber["FATAL"] = 21] = "FATAL";
	SeverityNumber[SeverityNumber["FATAL2"] = 22] = "FATAL2";
	SeverityNumber[SeverityNumber["FATAL3"] = 23] = "FATAL3";
	SeverityNumber[SeverityNumber["FATAL4"] = 24] = "FATAL4";
})(SeverityNumber || (SeverityNumber = {}));
var NoopLogger = class {
	emit(_logRecord) {}
	enabled() {
		return false;
	}
};
var NOOP_LOGGER = new NoopLogger();
/**
* Create a no-op Logger
*/
function createNoopLogger() {
	return NOOP_LOGGER;
}
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
/** Inject the active trace into an outgoing HTTP request. */
function injectContext(headers, parent = context.active(), propagator = propagation) {
	propagator.inject(parent, headers, { set: (headers, name, value) => headers.set(name, value) });
}
/** Extract a remote trace without inheriting another request's active context. */
function extractContext(headers, propagator = propagation) {
	return propagator.extract(ROOT_CONTEXT, headers, {
		keys: (headers) => [...headers.keys()],
		get: (headers, name) => headers.get(name) ?? void 0
	});
}
init_esm();
/** Declares a package's metric instruments with bounded attributes, checked by the compiler. */
var MetricFactory = class {
	/** The package's meter. */
	#meter;
	/** Declare instruments on a package's meter. */
	constructor(meter) {
		this.#meter = meter;
	}
	/** Declare a counter of increments. */
	counter(name, definition) {
		return this.#meter.createCounter(name, options(definition));
	}
	/** Declare a histogram of measurements. */
	histogram(name, definition) {
		return this.#meter.createHistogram(name, options(definition));
	}
	/** Declare a gauge of current values. */
	gauge(name, definition) {
		return this.#meter.createGauge(name, options(definition));
	}
};
/** Structured log records of one package, each named by a literal event name. */
var Log = class {
	/** The package's logger. */
	#logger;
	/** Emit through a package's logger. */
	constructor(logger) {
		this.#logger = logger;
	}
	/** Emit a debug record. */
	debug(name, attributes = {}) {
		this.#emit(SeverityNumber.DEBUG, "DEBUG", name, attributes);
	}
	/** Emit an informational record. */
	info(name, attributes = {}) {
		this.#emit(SeverityNumber.INFO, "INFO", name, attributes);
	}
	/** Emit a warning record. */
	warn(name, attributes = {}) {
		this.#emit(SeverityNumber.WARN, "WARN", name, attributes);
	}
	/** Emit an error record. */
	error(name, attributes = {}) {
		this.#emit(SeverityNumber.ERROR, "ERROR", name, attributes);
	}
	/** Emit one record in the active span's context. */
	#emit(severityNumber, severityText, eventName, attributes) {
		this.#logger.emit({
			eventName,
			severityNumber,
			severityText,
			attributes
		});
	}
};
/** Obtain package instruments from the providers registered by the host. */
function scope(source) {
	return instrument(trace.getTracer(source.name, source.version), metrics.getMeter(source.name, source.version), logs.getLogger(source.name, source.version));
}
/** Bundle a package's tracer, meter and logger with its log records and spans. */
function instrument(tracer, meter, logger) {
	return {
		tracer,
		meter,
		logger,
		log: new Log(logger),
		span: runner(tracer),
		metric: new MetricFactory(meter)
	};
}
/** Run work in child spans of a tracer. */
function runner(tracer) {
	const run = (name, attributes, work) => tracer.startActiveSpan(name, { attributes }, async (span) => {
		try {
			return await work(span);
		} catch (error) {
			span.recordException(error instanceof Error ? error : String(error));
			span.setStatus({ code: SpanStatusCode.ERROR });
			throw error;
		} finally {
			span.end();
		}
	});
	return Object.assign(run, { current: () => trace.getActiveSpan() });
}
/** Read the instrument options of a metric definition. */
function options(definition) {
	return {
		...definition.unit === void 0 ? {} : { unit: definition.unit },
		...definition.description === void 0 ? {} : { description: definition.description }
	};
}
/** Describe a failure by the OpenTelemetry exception attributes. */
function exceptionAttributes(error) {
	const exception = error instanceof Error ? error : new Error(String(error));
	return {
		"exception.type": exception.name,
		"exception.message": exception.message,
		...exception.stack === void 0 ? {} : { "exception.stacktrace": exception.stack }
	};
}
var CompositeLinkFetchPlugin = class extends CompositeStandardLinkPlugin {
	initRuntimeAdapter(options) {
		for (const plugin of this.plugins) plugin.initRuntimeAdapter?.(options);
	}
};
var LinkFetchClient = class {
	fetch;
	toFetchRequestOptions;
	adapterInterceptors;
	constructor(options) {
		new CompositeLinkFetchPlugin(options.plugins).initRuntimeAdapter(options);
		this.fetch = options.fetch ?? globalThis.fetch.bind(globalThis);
		this.toFetchRequestOptions = options;
		this.adapterInterceptors = toArray(options.adapterInterceptors);
	}
	async call(standardRequest, options, path, input) {
		const request = toFetchRequest(standardRequest, this.toFetchRequestOptions);
		return toStandardLazyResponse(await intercept(this.adapterInterceptors, {
			...options,
			request,
			path,
			input,
			init: { redirect: "manual" }
		}, ({ request: request2, path: path2, input: input2, init, ...options2 }) => this.fetch(request2, init, options2, path2, input2)), { signal: request.signal });
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
	const obj = new NullProtoObj();
	array.forEach((item, i) => {
		obj[i] = item;
	});
	return obj;
}
function pushStyleArrayToObject(array) {
	const obj = new NullProtoObj();
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
function getDynamicParams(path) {
	return path ? standardizeHTTPPath(path).match(/\/\{[^}]+\}/g)?.map((v) => ({
		raw: v,
		name: v.match(/\{\+?([^}]+)\}/)[1]
	})) : void 0;
}
var StandardOpenapiLinkCodec = class {
	constructor(contract, serializer, options) {
		this.contract = contract;
		this.serializer = serializer;
		this.baseUrl = options.url;
		this.headers = options.headers ?? {};
		this.customErrorResponseBodyDecoder = options.customErrorResponseBodyDecoder;
	}
	baseUrl;
	headers;
	customErrorResponseBodyDecoder;
	async encode(path, input, options) {
		let headers = toStandardHeaders(await value(this.headers, options, path, input));
		if (options.lastEventId !== void 0) headers = mergeStandardHeaders(headers, { "last-event-id": options.lastEventId });
		const baseUrl = await value(this.baseUrl, options, path, input);
		const procedure = get(this.contract, path);
		if (!isContractProcedure(procedure)) throw new Error(`[StandardOpenapiLinkCodec] expect a contract procedure at ${path.join(".")}`);
		return fallbackContractConfig("defaultInputStructure", procedure["~orpc"].route.inputStructure) === "compact" ? this.#encodeCompact(procedure, path, input, options, baseUrl, headers) : this.#encodeDetailed(procedure, path, input, options, baseUrl, headers);
	}
	#encodeCompact(procedure, path, input, options, baseUrl, headers) {
		let httpPath = standardizeHTTPPath(procedure["~orpc"].route.path ?? toHttpPath(path));
		let httpBody = input;
		const dynamicParams = getDynamicParams(httpPath);
		if (dynamicParams?.length) {
			if (!isObject(input)) throw new TypeError(`[StandardOpenapiLinkCodec] Invalid input shape for "compact" structure when has dynamic params at ${path.join(".")}.`);
			const body = { ...input };
			for (const param of dynamicParams) {
				const value2 = input[param.name];
				httpPath = httpPath.replace(param.raw, `/${encodeURIComponent(`${this.serializer.serialize(value2)}`)}`);
				delete body[param.name];
			}
			httpBody = Object.keys(body).length ? body : void 0;
		}
		const method = fallbackContractConfig("defaultMethod", procedure["~orpc"].route.method);
		const url = new URL(baseUrl);
		url.pathname = `${url.pathname.replace(/\/$/, "")}${httpPath}`;
		if (method === "GET") {
			const serialized = this.serializer.serialize(httpBody, { outputFormat: "URLSearchParams" });
			for (const [key, value2] of serialized) url.searchParams.append(key, value2);
			return {
				url,
				method,
				headers,
				body: void 0,
				signal: options.signal
			};
		}
		return {
			url,
			method,
			headers,
			body: this.serializer.serialize(httpBody),
			signal: options.signal
		};
	}
	#encodeDetailed(procedure, path, input, options, baseUrl, headers) {
		let httpPath = standardizeHTTPPath(procedure["~orpc"].route.path ?? toHttpPath(path));
		const dynamicParams = getDynamicParams(httpPath);
		if (!isObject(input) && input !== void 0) throw new TypeError(`[StandardOpenapiLinkCodec] Invalid input shape for "detailed" structure at ${path.join(".")}.`);
		if (dynamicParams?.length) {
			if (!isObject(input?.params)) throw new TypeError(`[StandardOpenapiLinkCodec] Invalid input.params shape for "detailed" structure when has dynamic params at ${path.join(".")}.`);
			for (const param of dynamicParams) {
				const value2 = input.params[param.name];
				httpPath = httpPath.replace(param.raw, `/${encodeURIComponent(`${this.serializer.serialize(value2)}`)}`);
			}
		}
		let mergedHeaders = headers;
		if (input?.headers !== void 0) {
			if (!isObject(input.headers)) throw new TypeError(`[StandardOpenapiLinkCodec] Invalid input.headers shape for "detailed" structure at ${path.join(".")}.`);
			mergedHeaders = mergeStandardHeaders(input.headers, headers);
		}
		const method = fallbackContractConfig("defaultMethod", procedure["~orpc"].route.method);
		const url = new URL(baseUrl);
		url.pathname = `${url.pathname.replace(/\/$/, "")}${httpPath}`;
		if (input?.query !== void 0) {
			const query = this.serializer.serialize(input.query, { outputFormat: "URLSearchParams" });
			for (const [key, value2] of query) url.searchParams.append(key, value2);
		}
		if (method === "GET") return {
			url,
			method,
			headers: mergedHeaders,
			body: void 0,
			signal: options.signal
		};
		return {
			url,
			method,
			headers: mergedHeaders,
			body: this.serializer.serialize(input?.body),
			signal: options.signal
		};
	}
	async decode(response, _options, path) {
		const isOk = !isORPCErrorStatus(response.status);
		const deserialized = await (async () => {
			let isBodyOk = false;
			try {
				const body = await response.body();
				isBodyOk = true;
				return this.serializer.deserialize(body);
			} catch (error) {
				if (!isBodyOk) throw new Error("Cannot parse response body, please check the response body and content-type.", { cause: error });
				throw new Error("Invalid OpenAPI response format.", { cause: error });
			}
		})();
		if (!isOk) {
			const error = this.customErrorResponseBodyDecoder?.(deserialized, response);
			if (error !== null && error !== void 0) throw error;
			if (isORPCErrorJson(deserialized)) throw createORPCErrorFromJson(deserialized);
			throw new ORPCError(getMalformedResponseErrorCode(response.status), {
				status: response.status,
				data: {
					...response,
					body: deserialized
				}
			});
		}
		const procedure = get(this.contract, path);
		if (!isContractProcedure(procedure)) throw new Error(`[StandardOpenapiLinkCodec] expect a contract procedure at ${path.join(".")}`);
		if (fallbackContractConfig("defaultOutputStructure", procedure["~orpc"].route.outputStructure) === "compact") return deserialized;
		return {
			status: response.status,
			headers: response.headers,
			body: deserialized
		};
	}
};
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
var StandardOpenAPILink = class extends StandardLink {
	constructor(contract, linkClient, options) {
		const linkCodec = new StandardOpenapiLinkCodec(contract, new StandardOpenAPISerializer(new StandardOpenAPIJsonSerializer(options), new StandardBracketNotationSerializer({ maxBracketNotationArrayIndex: 4294967294 })), options);
		super(linkCodec, linkClient, options);
	}
};
var OpenAPILink = class extends StandardOpenAPILink {
	constructor(contract, options) {
		const linkClient = new LinkFetchClient(options);
		super(contract, linkClient, options);
	}
};
var ClientRetryPluginInvalidEventIteratorRetryResponse = class extends Error {};
var ClientRetryPlugin = class {
	defaultRetry;
	defaultRetryDelay;
	defaultShouldRetry;
	defaultOnRetry;
	order = 18e5;
	constructor(options = {}) {
		this.defaultRetry = options.default?.retry ?? 0;
		this.defaultRetryDelay = options.default?.retryDelay ?? ((o) => o.lastEventRetry ?? 2e3);
		this.defaultShouldRetry = options.default?.shouldRetry ?? true;
		this.defaultOnRetry = options.default?.onRetry;
	}
	init(options) {
		options.interceptors ??= [];
		options.interceptors.push(async (interceptorOptions) => {
			const maxAttempts = await value(interceptorOptions.context.retry ?? this.defaultRetry, interceptorOptions);
			const retryDelay = interceptorOptions.context.retryDelay ?? this.defaultRetryDelay;
			const shouldRetry = interceptorOptions.context.shouldRetry ?? this.defaultShouldRetry;
			const onRetry = interceptorOptions.context.onRetry ?? this.defaultOnRetry;
			if (maxAttempts <= 0) return interceptorOptions.next();
			let lastEventId = interceptorOptions.lastEventId;
			let lastEventRetry;
			let callback;
			let attemptIndex = 0;
			const next = async (initialError) => {
				let currentError = initialError;
				while (true) {
					const updatedInterceptorOptions = {
						...interceptorOptions,
						lastEventId
					};
					if (currentError) {
						if (attemptIndex >= maxAttempts) throw currentError.error;
						const attemptOptions = {
							...updatedInterceptorOptions,
							attemptIndex,
							error: currentError.error,
							lastEventRetry
						};
						if (!await value(shouldRetry, attemptOptions)) throw currentError.error;
						callback = onRetry?.(attemptOptions);
						const retryDelayMs = await value(retryDelay, attemptOptions);
						await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
						attemptIndex++;
					}
					try {
						currentError = void 0;
						return await interceptorOptions.next(updatedInterceptorOptions);
					} catch (error) {
						currentError = { error };
						if (updatedInterceptorOptions.signal?.aborted) throw error;
					} finally {
						callback?.(!currentError);
						callback = void 0;
					}
				}
			};
			const output = await next();
			if (!isAsyncIteratorObject(output)) return output;
			let current = output;
			let isIteratorAborted = false;
			return overlayProxy(() => current, new AsyncIteratorClass(async () => {
				while (true) try {
					const item = await current.next();
					const meta = getEventMeta(item.value);
					lastEventId = meta?.id ?? lastEventId;
					lastEventRetry = meta?.retry ?? lastEventRetry;
					return item;
				} catch (error) {
					const meta = getEventMeta(error);
					lastEventId = meta?.id ?? lastEventId;
					lastEventRetry = meta?.retry ?? lastEventRetry;
					const maybeEventIterator = await next({ error });
					if (!isAsyncIteratorObject(maybeEventIterator)) throw new ClientRetryPluginInvalidEventIteratorRetryResponse("RetryPlugin: Expected an Event Iterator, got a non-Event Iterator");
					current = maybeEventIterator;
					if (isIteratorAborted) {
						await current.return?.();
						throw error;
					}
				}
			}, async (reason) => {
				isIteratorAborted = true;
				if (reason !== "next") await current.return?.();
			}));
		});
	}
};
/** The header carrying the release of the callee's package a caller was built against, its API version. */
var VERSION_HEADER = "Destack-Version";
/** The retry lifetime of a request identifier. */
var REQUEST_LIFETIME_MILLISECONDS = 6048e5;
/** The clock tolerance for new request identifiers. */
var CLOCK_TOLERANCE_MILLISECONDS = 3e5;
/** The originals of copied requests. */
var ORIGINALS = /* @__PURE__ */ new WeakMap();
/** Copy a request with changes and an optional URL and hold the client's original while the copy lives. */
function copyRequest(request, changes, url = request.url) {
	const copy = new Request(url, new Request(request, changes));
	ORIGINALS.set(copy, originalRequest(request));
	return copy;
}
/** Find a request as its client sent it, before any router copied it. */
function originalRequest(request) {
	return ORIGINALS.get(request) ?? request;
}
/** A timestamped request identifier kept across retries. */
var RequestId = {
	/** The schema of a request identifier, a UUIDv7. */
	schema: uuidv7(),
	/** Create a request identifier. */
	create() {
		return v7();
	},
	/** Read the retry deadline of a request identifier. */
	expiry(requestId, now = Date.now()) {
		const key = RequestId.schema.parse(requestId);
		const createdAt = Number.parseInt(key.slice(0, 13).replaceAll("-", ""), 16);
		const expiresAt = createdAt + REQUEST_LIFETIME_MILLISECONDS;
		if (createdAt > now + CLOCK_TOLERANCE_MILLISECONDS || expiresAt <= now) throw new ORPCError("PRECONDITION_FAILED", { message: "request identifier is outside its retry period" });
		return expiresAt;
	}
};
init_context_api(), init_propagation_api(), init_trace_api(), init_span_kind(), init_status();
/** The instrumenting package. */
var manifest = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73ee-8687-f9c041e21f17",
	"name": "@destack/service",
	"version": "2026.9.0"
} }).package;
/** Record RPC calls, including streams. */
var ServiceTelemetry = class {
	/** The call durations, in seconds. */
	#duration;
	/** The RPC span kind. */
	#kind;
	/** Create the telemetry. */
	constructor(kind) {
		instrumentService();
		this.#kind = kind === "client" ? SpanKind.CLIENT : SpanKind.SERVER;
		this.#duration = scope(manifest).meter.createHistogram(`rpc.${kind}.call.duration`, { unit: "s" });
	}
	/** Measure a call. */
	invoke(path, next, labels = {}) {
		const attributes = {
			"rpc.system.name": "orpc",
			"rpc.method": path.join("/"),
			...labels
		};
		return scope(manifest).tracer.startActiveSpan(path.join("/"), {
			kind: this.#kind,
			attributes
		}, (span) => this.#invoke(next, attributes, span));
	}
	/** Trace a call until its value or stream completes. */
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
	/** Trace a stream until it completes or is cancelled. */
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
	/** Record a call's outcome and duration. */
	#record(started, attributes, span, error) {
		if (error !== void 0) attributes["error.type"] = error instanceof ORPCError ? String(error.status) : "internal";
		if (attributes["error.type"]) span.setStatus({ code: SpanStatusCode.ERROR });
		span.setAttributes(attributes);
		span.end();
		this.#duration.record((performance.now() - started) / 1e3, attributes);
	}
};
/** Enable procedure and HTTP tracing. */
function instrumentService() {
	setGlobalOtelConfig({
		tracer: trace.getTracer("@destack/service"),
		trace,
		context,
		propagation
	});
}
/** The header carrying watermarks on requests and responses. */
var BOOKMARK_HEADER = "destack-bookmark";
/** A position in one scope's log. */
var Watermark = defineSchema(strictObject({
	/** The scope whose database holds the log. */
	scope: string().min(1),
	/** The log's epoch, renewed when the database is restored. */
	epoch: string().min(1),
	/** The log sequence reached within the epoch. */
	sequence: number().int().nonnegative()
}));
/** The latest watermark per scope. */
var Bookmark = class Bookmark {
	/** The latest watermark per scope. */
	#watermarks = [];
	/** Parse a header value. */
	static parse(header) {
		const bookmark = new Bookmark();
		if (header) for (const watermark of array(Watermark).parse(JSON.parse(header))) bookmark.observe(watermark);
		return bookmark;
	}
	/** The latest watermark per scope. */
	get watermarks() {
		return this.#watermarks;
	}
	/** Merge a watermark and keep the newest per scope. */
	observe(watermark) {
		const index = this.#watermarks.findIndex((known) => known.scope === watermark.scope);
		if (index === -1) this.#watermarks.push(watermark);
		else if (isAfter(watermark, this.#watermarks[index])) this.#watermarks[index] = watermark;
	}
	/** Write the watermarks as a header value. */
	format() {
		return JSON.stringify(this.#watermarks);
	}
};
/** Create a typed HTTP client of a service, speaking the release of the service it was built against. */
function createClient(service, options) {
	const telemetry = new ServiceTelemetry("client");
	const link = new OpenAPILink(service.router, {
		...options,
		plugins: [new ClientRetryPlugin(), ...options.plugins ?? []],
		adapterInterceptors: [async ({ request, next }) => {
			injectContext(request.headers);
			request.headers.set(VERSION_HEADER, service.package.version);
			const bookmark = options.bookmark;
			if (bookmark && bookmark.watermarks.length > 0) request.headers.set(BOOKMARK_HEADER, bookmark.format());
			const response = await next();
			const observed = response.headers.get(BOOKMARK_HEADER);
			if (bookmark && observed) for (const watermark of Bookmark.parse(observed).watermarks) bookmark.observe(watermark);
			return response;
		}, ...options.adapterInterceptors ?? []]
	});
	return createORPCClient({ call: (path, input, options) => telemetry.invoke(path, () => link.call(path, input, options)) });
}
/** A service binding's specification: the service it calls. */
var ServiceBindingSpec = defineSchema(strictObject({ service: DeclarationReference }));
defineSchema(strictObject({ release: Version }));
/** The service resource kind: typed clients of a package's service, bound to its endpoint. */
var ServiceKind = defineResourceKind("service", { spec: ServiceBindingSpec });
/** A dependency on a package's service, bound to a typed client of its endpoint. */
var ServiceBinding = class extends Resource {
	/** The service the client calls. */
	service;
	/** Create the binding. */
	constructor(owner, description, service) {
		super(owner, description);
		this.service = service;
	}
	/** The HTTP connector, calling the service at the bound address through the host's egress with the bound credential. */
	get connectors() {
		return { http: {
			code: "http",
			connect: async (binding) => {
				const credential = binding.credential;
				if (credential === void 0) throw new TypeError(`service binding ${this.name} holds no credential`);
				return createClient(this.service, {
					url: binding.reference,
					headers: () => ({ authorization: `Bearer ${credential}` })
				});
			}
		} };
	}
};
/** Declare a dependency on a service. */
function defineServiceBinding(name, service, module) {
	const owner = declaringModule(module, "defineServiceBinding").package;
	return new ServiceBinding(owner, ServiceKind.description.parse({
		name,
		kind: ServiceKind.name,
		spec: { service: reference(service) }
	}), service);
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
/** Declare a workload. */
function defineWorkload(definition, module) {
	const owner = declaringModule(module, "defineWorkload").package;
	const { start, ...fields } = definition;
	return Object.freeze({
		...WorkloadDefinition.parse(fields),
		start,
		package: owner
	});
}
/** A package-local action name of the form Noun.verb. */
var AuditActionName = defineSchema(string().regex(/^[A-Z][A-Za-z0-9]*(?:\.[A-Z][A-Za-z0-9]*)*\.[a-z][A-Za-z0-9]*$/));
/** A reference to an action of a package release, whose schemas read its targets and details. */
var AuditActionReference = defineSchema(strictObject({
	/** The declaring package at the release that recorded the event. */
	package: Package,
	/** The package-local Noun.verb action name. */
	name: AuditActionName
}));
/** Declare an action. */
function defineAuditAction(definition, module) {
	AuditActionName.parse(definition.name);
	return Object.freeze({
		...definition,
		package: Package.parse(declaringModule(module, "defineAuditAction").package)
	});
}
var __destackModule = Object.freeze({ "package": {
	"id": "package-01a0c80b-6150-71b1-a0c5-78117553227d",
	"name": "@destack/build-service-fixture",
	"version": "2026.9.0"
} });
/** Record a published note under its declaring package. */
var publishNote = defineAuditAction({
	name: "Note.publish",
	targets: strictObject({ note: strictObject({
		type: literal("note"),
		id: string()
	}) }),
	details: strictObject({ revision: number().int() })
}, __destackModule);
/** Package instruments initialized from build-injected metadata. */
var instruments = scope(__destackModule.package);
/** The shared application database. */
var database = defineDatabase({
	name: "main",
	tables: []
}, __destackModule);
/** The application's secret collection. */
var vault = defineVault({
	name: "credentials",
	spec: {}
}, __destackModule);
/** The secret selected during installation. */
var token = defineSecret({ name: "mail-token" }, __destackModule);
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
var service = defineService("notes", router, __destackModule);
/** A dependency on the installation's notes service. */
var notes = defineServiceBinding("notes", service, __destackModule);
/** Implement the public notes procedures. */
function implementService() {
	const implementation = implement(router);
	return {
		service,
		router: implementation.router({ list: implementation.list.handler(() => ({ path: "/notes" })) }),
		authorize: async () => {}
	};
}
/** The web workload hosting notes and reminders. */
var web = defineWorkload({
	name: "web",
	compute: { cpuTime: 1e3 },
	start: async () => {
		instruments.logger.emit({ body: "Workload started" });
		return {
			services: [implementService()],
			triggers: [
				reminders,
				refresh,
				appointment
			].map(implementSchedule)
		};
	}
}, __destackModule);
/** The daily reminder schedule. */
var reminders = defineSchedule({
	name: "reminders",
	timing: "cron",
	cron: "0 9 * * *",
	timezone: "UTC",
	concurrency: "forbid",
	deadline: 6e4
}, __destackModule);
/** Repeat from a fixed first occurrence. */
var refresh = defineSchedule({
	name: "refresh",
	timing: "interval",
	interval: 3e5,
	startsAt: 18e11,
	endsAt: 18000864e5,
	concurrency: "forbid",
	deadline: 6e4
}, __destackModule);
/** Send a reminder at one specified time. */
var appointment = defineSchedule({
	name: "appointment",
	timing: "once",
	startsAt: 18e11,
	concurrency: "allow",
	deadline: 6e4
}, __destackModule);
/** Log each occurrence of a reminder schedule. */
function implementSchedule(schedule) {
	return schedule.handle(async (_occurrence, signal) => {
		signal.throwIfAborted();
		instruments.logger.emit({ body: `Reminder ${schedule.name}` });
	});
}
export { init_status as $, isObject as $t, copyRequest as A, none as At, createNoopLogger as B, toFetchResponse as Bt, createClient as C, subjectKey as Ct, ServiceTelemetry as D, grants as Dt, Watermark as E, condition as Et, exceptionAttributes as F, ObjectReference as Ft, propagation as G, isORPCErrorStatus as Gt, esm_exports as H, ValidationError as Ht, instrument as I, ObjectTypeReference as It, diag as J, NullProtoObj as Jt, init_metrics_api as K, toORPCError as Kt, scope as L, eventIterator as Lt, StandardOpenAPISerializer as M, relation as Mt, standardizeHTTPPath as N, through as Nt, RequestId as O, intersection as Ot, StandardBracketNotationSerializer as P, union as Pt, SpanStatusCode as Q, isAsyncIteratorObject as Qt, extractContext as R, fallbackContractConfig as Rt, unlazy as S, sameSubject as St, Bookmark as T, Attribute as Tt, init_esm as U, flattenHeader as Ut, SeverityNumber as V, toStandardLazyRequest as Vt, init_propagation_api as W, ORPCError as Wt, context as X, asyncIteratorWithSpan as Xt, init_diag_api as Y, ORPC_NAME as Yt, init_context_api as Z, intercept as Zt, getLazyMeta as _, relationsOf as _t, publishNote as a, tryDecodeURIComponent as an, init_Metric as at, isProcedure as b, Subject as bt, router as c, TRIGGER_KINDS as cn, baggageEntryMetadataFromString as ct, vault as d, ComputeDefinition as dn, ProcedureMeta as dt, resolveMaybeOptionalOptions as en, SpanKind as et, web as f, defineProcedure as ft, createProcedureClient as g, permissionKey as gt, createContractedProcedure as h, objectKey as ht, notes as i, toArray as in, ValueType as it, StandardOpenAPIJsonSerializer as j, permission as jt, VERSION_HEADER as k, nameCheck as kt, service as l, WorkloadDescription as ln, init_utils$1 as lt, AuditActionReference as m, Policy as mt, database as n, setSpanError as nn, SamplingDecision as nt, refresh as o, value as on, createNoopMeter as ot, AuditActionName as p, PermissionReference as pt, metrics as q, AsyncIteratorClass as qt, implementService as r, stringifyJSON as rn, init_SamplingResult as rt, reminders as s, CONCURRENCIES as sn, init_NoopMeter as st, appointment as t, runWithSpan as tn, init_span_kind as tt, token as u, reference as un, defineService as ut, getRouter as v, subjectType as vt, BOOKMARK_HEADER as w, AccessName as wt, traverseContractProcedures as x, keySubject as xt, isLazy as y, AccessError as yt, logs as z, toHttpPath as zt };

//# sourceMappingURL=server-DVf9XgjR.js.map