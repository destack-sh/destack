import { $t as identifier, A as expandTrees, Bt as Resource, Gt as DeclarationName, H as Expression, Ht as ResourceReference, Jt as PackageId, Kt as DependencyName, O as TABLE, Qt as Version, Rt as canonicalize, T as Condition, Ut as defineResourceKind, Vt as ResourceDescription, Wt as declaringModule, Xt as PackagePath, Yt as Digest, Zt as PackageError, a as declareState, an as cidrv4, cn as int, dn as number, en as defineSchema, fn as record, gn as union, hn as tuple, in as boolean, ln as json, mn as string, n as DatabaseState, nn as _enum, on as cidrv6, pn as strictObject, qt as Package, rn as array, sn as discriminatedUnion, t as DatabaseTier, un as literal } from "./tier-D1tCxCZ_.js";
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
/** A stack's declarations as JSON, keyed by collection and validated by each object when applied. */
var SpaceDefinition = defineSchema(record(string().min(1), json()));
/** Define a stack's declarations, serialising each declared value to its JSON form. */
function defineSpace(collections, module) {
	return {
		package: declaringModule(module, "defineSpace").package,
		definition: SpaceDefinition.parse(JSON.parse(JSON.stringify(collections)))
	};
}
/** The connectors opening databases on Bun: SQLite files, loaded on first connect. */
var connectors = { sqlite: {
	code: "sqlite",
	connect: async (bound, declaration) => {
		const { sqliteConnector } = await import("./connector-TTjT2AhC.js");
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
union([
	string(),
	number().finite(),
	boolean()
]);
/** A stable declaration-local name used by access rules. */
var AccessName = string().regex(/^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$(?![\s\S])/);
/** A stable reference to a declared permission, independent of a package version. */
var PermissionReference = defineSchema(strictObject({
	/** The package that declares the permission. */
	packageId: PackageId,
	/** The declaration-local object type name. */
	type: AccessName,
	/** The permission name within that object type. */
	name: AccessName
}));
defineSchema(strictObject({
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
	tags: record(string().min(1), string())
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
	state: record(string(), json())
}));
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
	rules: record(DeclarationName, PackageRule)
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
	rules: record(DeclarationName, NetworkRule)
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
	workloads: record(DeclarationName, strictObject({ 
	/** Outbound access intersected with installation, space, account, and host restrictions. */
network: NetworkPolicyDefinition })).optional()
}));
/** An invalid or unsupported space configuration, or a write its cell may no longer make. */
var SpaceError = class extends Error {
	/** Stable failure code. */
	code;
	/** Create a configuration failure. */
	constructor(code, message, options) {
		super(message, options);
		this.name = "SpaceError";
		this.code = code;
	}
};
/** A package directory within a repository or checkout, or its root. */
var PackageDirectory = union([literal("."), PackagePath]);
/** A committed Git object identifier. */
var Commit = string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/);
defineSchema(discriminatedUnion("kind", [
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
defineSchema(discriminatedUnion("kind", [
	strictObject({
		/** A published registry release. */
		kind: literal("release"),
		/** The released package version. */
		version: string().min(1),
		/** The digest of the release manifest, absent before a host resolved the release. */
		manifest: Digest.optional()
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
		manifest: Digest
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
		manifest: Digest.optional()
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
	bindings: record(PackageId, record(DeclarationName, SpaceBinding)),
	/** Workload compute settings checked against package and host policy. */
	compute: record(DeclarationName, ComputeDefinition),
	/** User-defined labels. */
	tags: record(string().min(1), string())
}));
/** Install a package release, binding its declarations to this space's resources and secrets. */
function install(handle, bindings, options = {}) {
	const bound = {};
	for (const [name, selection] of Object.entries(bindings)) {
		const resource = handle.resources[name];
		const secret = handle.secrets[name];
		if (resource && typeof selection === "string") {
			bound[resource.package.id] ??= {};
			bound[resource.package.id][resource.name] = {
				target: {
					type: "resource",
					name: selection
				},
				state: resource.state()
			};
		} else if (secret) {
			const target = typeof selection === "string" ? { secret: selection } : selection;
			bound[secret.package.id] ??= {};
			bound[secret.package.id][secret.name] = {
				target: {
					type: "secret",
					name: target.secret
				},
				..."version" in target ? { version: target.version } : {},
				state: {}
			};
		} else throw new SpaceError("INVALID_DEFINITION", `${handle.name} declares no ${name} to bind`);
	}
	return SpaceInstallation.parse({
		package: {
			id: handle.id,
			name: handle.name,
			version: handle.version
		},
		status: options.status ?? "enabled",
		alias: options.alias ?? handle.name.split("/")[1],
		bindings: bound,
		compute: options.compute ?? {},
		tags: options.tags ?? {},
		...options.policies ? { policies: options.policies } : {}
	});
}
/** Declare the package handle stacks import to install this package. */
function definePackage(declarations, module) {
	const owner = Package.parse(declaringModule(module, "definePackage").package);
	const resources = declarations.resources ?? {};
	const secrets = declarations.secrets ?? {};
	const names = /* @__PURE__ */ new Set();
	for (const [key, declaration] of [...Object.entries(resources), ...Object.entries(secrets)]) {
		if (key !== declaration.name) throw new PackageError("INVALID_DEFINITION", `declaration ${declaration.name} is listed as ${key}`);
		if (names.has(key)) throw new PackageError("INVALID_DEFINITION", `duplicate declaration: ${key}`);
		names.add(key);
	}
	return Object.freeze({
		...owner,
		resources,
		secrets
	});
}
/** The space's shared application database. */
var database = defineDatabase({
	name: "main",
	tables: []
}, Object.freeze({ "package": {
	"id": "package-01a0c80b-6148-71fe-af7d-35098929d5dd",
	"name": "@template/stack",
	"version": "2026.9.0"
} }));
function resolveMaybeOptionalOptions(rest) {
	return rest[0] ?? {};
}
function getConstructor(value) {
	if (!isTypescriptObject(value)) return null;
	return Object.getPrototypeOf(value)?.constructor;
}
function isTypescriptObject(value) {
	return !!value && (typeof value === "object" || typeof value === "function");
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
function isORPCErrorStatus(status) {
	return status < 200 || status >= 400;
}
/** User-defined labels indexed by name. */
var TagMap = record(string().min(1).max(128), string().max(256));
/** An invalid account declaration. */
var AccountError = class extends Error {
	/** The stable failure code. */
	code;
	/** Create an account declaration failure. */
	constructor(code, message, options) {
		super(message, options);
		this.name = "AccountError";
		this.code = code;
	}
};
/** An account-defined environment. */
var EnvironmentDefinition = defineSchema(strictObject({ 
/** Additional account-defined metadata. */
tags: TagMap.optional() }));
/** An account role declared in source. */
var AccountRole = defineSchema(strictObject({
	/** The role's purpose. */
	description: string().min(1),
	/** Permissions granted wherever the role is bound. */
	permissions: array(PermissionReference)
}));
/** A role granted to a user, a group or a service account of the account. */
var AccountRoleBinding = defineSchema(strictObject({
	/** A declared role name or an existing role in this account. */
	role: union([DeclarationName, strictObject({ id: identifier("role") })]),
	/** The user, group or service account receiving the role. */
	subject: union([
		strictObject({ user: identifier("user") }),
		strictObject({ group: identifier("group") }),
		strictObject({ service: identifier("service-account") })
	]),
	/** The space the role is bound on. */
	spaceId: identifier("space").optional(),
	/** Optional expiry in UTC epoch milliseconds. */
	expiresAt: number().int().nonnegative().optional()
}));
/** Account records declared by a repository export. */
var AccountDefinition = defineSchema(strictObject({
	/** Account-local environment names. */
	environments: record(DeclarationName, EnvironmentDefinition).optional(),
	/** Account roles available to bindings. */
	roles: record(DeclarationName, AccountRole).optional(),
	/** Grants applied using the caller's existing authority. */
	bindings: record(DeclarationName, AccountRoleBinding).optional()
}));
/** Declare account records without creating or changing an account. */
function defineAccount(definition, module) {
	const owner = declaringModule(module, "defineAccount").package;
	const account = AccountDefinition.parse(definition);
	for (const binding of Object.values(account.bindings ?? {})) if (typeof binding.role === "string" && !Object.hasOwn(account.roles ?? {}, binding.role)) throw new AccountError("INVALID_DEFINITION", `unknown role: ${binding.role}`);
	return {
		package: owner,
		definition: account
	};
}
var __destackModule$1 = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73f2-a9cd-9cb63f4d528d",
	"name": "@example/stack",
	"version": "2026.9.0"
} });
/** The handle stacks import to install this package. */
var package_default = definePackage({ resources: { main: database } }, __destackModule$1);
/** Where a value is placed: its scope and the overrides narrowing where it applies. */
var SettingPlacement = Object.assign(defineSchema(strictObject({
	/** The user, space, account, organisation or host holding the value. */
	scope: string().min(1),
	/** The consuming package the value applies to. */
	package: PackageId.optional(),
	/** The space the value applies in. */
	space: identifier("space").optional(),
	/** The installation the value applies to. */
	installation: identifier("installation").optional(),
	/** The device the value applies on. */
	device: identifier("device").optional()
})), { 
/** Read the placement of a value, source or selection, dropping absent overrides. */
of(placed) {
	return {
		scope: placed.scope,
		...placed.package == null ? {} : { package: placed.package },
		...placed.space == null ? {} : { space: placed.space },
		...placed.installation == null ? {} : { installation: placed.installation },
		...placed.device == null ? {} : { device: placed.device }
	};
} });
defineSchema(SettingPlacement.extend({ 
/** The selected user, space or host, or null for an anonymous visitor. */
scope: string().min(1).nullable() }));
/** The service error code of each setting failure. */
var SERVICE_CODES = {
	UNDECLARED: "NOT_FOUND",
	INVALID_VALUE: "BAD_REQUEST",
	INVALID_PLACEMENT: "BAD_REQUEST",
	CONFLICT: "CONFLICT"
};
/** A setting declaration, placement or resolution failure. */
var SettingError = class extends Error {
	/** The failure category. */
	code;
	/** Report a setting failure. */
	constructor(code, message) {
		super(message);
		this.name = "SettingError";
		this.code = code;
	}
	/** Convert the failure to the service error a caller receives. */
	toServiceError() {
		return new ORPCError(SERVICE_CODES[this.code], {
			message: this.message,
			cause: this
		});
	}
};
/** The declaration default ranks below every placed value. */
var DEFAULT_TIER = 0;
/** Recommendations of enclosing scopes follow the declaration default. */
var ENCLOSING_TIER = 1;
/** Values set in the selected scope follow every recommendation. */
var SET_TIER = 2;
/** An enclosing recommendation one scope nearer follows a farther one, whatever its installation. */
var DEPTH_WEIGHT = 2;
/** A recommendation for the selected installation follows one for every installation of its scope. */
var ENCLOSING_INSTALLATION_WEIGHT = 1;
/** A set value for the consuming package follows one applying to every package. */
var PACKAGE_WEIGHT = 1;
/** A set value for the space follows the package override alone. */
var SPACE_WEIGHT = 2;
/** A set value for the installation follows one for its space. */
var INSTALLATION_WEIGHT = 4;
/** A device-specific value follows every device-independent one. */
var DEVICE_WEIGHT = 8;
/** A package-local setting name, kept across releases. */
var SettingName = defineSchema(string().regex(/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)*$(?![\s\S])/));
Object.assign(defineSchema(strictObject({
	/** The package declaring the setting. */
	packageId: PackageId,
	/** The package-local name. */
	name: SettingName
})), { 
/** Key a setting by its package and name. */
key(reference) {
	return `${reference.packageId}/${reference.name}`;
} });
/** A typed setting declaration. */
var Setting = class {
	/** The declaring package. */
	package;
	/** The name, schema, default, scopes and conversions. */
	definition;
	/** Hold a checked declaration. */
	constructor(owner, definition) {
		this.package = owner;
		this.definition = definition;
	}
	/** The identity its values refer to. */
	get reference() {
		return {
			packageId: this.package.id,
			name: this.definition.name
		};
	}
	/** The package-local name. */
	get name() {
		return this.definition.name;
	}
	/** Serialise the setting as its reference. */
	toJSON() {
		return this.reference;
	}
	/** Match the values a resolution reads: those set in the selected scope, and the recommendations and requirements above it. */
	condition(selection) {
		const policy = Condition.ne("mode", "set");
		return Condition.all(Condition.eq("packageId", this.reference.packageId), Condition.eq("name", this.reference.name), selection.scope === null ? policy : Condition.any(Condition.eq("scope", selection.scope), policy));
	}
	/**
	* Resolve the effective value for a selection from the values placed along a scope chain, nearest scope first.
	*
	* A stored value the declaration no longer accepts is skipped and listed as an invalid source.
	*/
	resolve(selection, values, chain) {
		if (!(selection.scope === null ? this.definition.scope === "user" : identifier(this.definition.scope).safeParse(selection.scope).success)) throw new SettingError("INVALID_PLACEMENT", "setting scope does not match the selected scope");
		const ordinary = [{
			rank: [DEFAULT_TIER, 0],
			source: {
				kind: "default",
				package: this.package
			},
			value: this.definition.default,
			mode: "set"
		}];
		const required = [];
		const invalid = [];
		for (const value of values) {
			const candidate = this.#candidate(value, selection, chain);
			if (candidate === void 0) continue;
			else if (candidate.source.kind === "invalid") invalid.push(candidate.source);
			else (candidate.mode === "require" ? required : ordinary).push(candidate);
		}
		ordinary.sort((left, right) => compareRanks(left, right) || compareSources(left, right));
		required.sort(compareSources);
		invalid.sort((left, right) => compareCanonical(left, right));
		for (let index = 1; index < ordinary.length; index++) {
			const previous = ordinary[index - 1];
			const current = ordinary[index];
			const isShared = previous.mode === "recommend" && current.mode === "recommend" && equalValue(previous.value, current.value);
			if (compareRanks(previous, current) === 0 && !isShared) throw new SettingError("CONFLICT", "multiple setting values have the same precedence");
		}
		if (required.some((candidate) => !equalValue(candidate.value, required[0].value))) throw new SettingError("CONFLICT", "required setting values disagree");
		const winner = required[0] ?? ordinary[ordinary.length - 1];
		const winners = required.length > 0 ? required : ordinary.filter((candidate) => compareRanks(candidate, winner) === 0);
		const overridden = required.length > 0 ? ordinary : ordinary.filter((candidate) => compareRanks(candidate, winner) < 0);
		return {
			setting: this.reference,
			selection,
			value: winner.value,
			sources: [...winners.map((candidate) => candidate.source), ...invalid],
			overridden: overridden.map((candidate) => candidate.source),
			enforcement: required.length > 0 ? "required" : "ordinary"
		};
	}
	/** Require a value the setting's schema accepts. */
	requireValue(value) {
		const parsed = this.definition.schema.safeParse(value);
		if (!parsed.success) throw new SettingError("INVALID_VALUE", "setting value does not match its declaration");
		return parsed.data;
	}
	/** Require a value written in a scope to convert to a valid value of this release, at a permitted placement. */
	requireWrite(write, scope) {
		const converted = this.#convert(write.value, write.release);
		if (!converted.isConverted) throw new SettingError("INVALID_VALUE", `setting value is at release ${write.release}, its declaration at ${this.package.version}`);
		this.requireValue(converted.value);
		const position = identifier(this.definition.scope).safeParse(scope).success ? "own" : "enclosing";
		this.requirePlacement(write, position);
	}
	/** Require a value to carry the mode and overrides its scope permits. */
	requirePlacement(value, position) {
		if (position === "own" && value.mode !== "set") throw new SettingError("INVALID_PLACEMENT", "setting value in its declared scope must be set");
		else if (position === "enclosing" && value.mode === "set") throw new SettingError("INVALID_PLACEMENT", "setting value outside its declared scope must recommend or require");
		if (value.installation !== void 0 && value.space !== void 0) throw new SettingError("INVALID_PLACEMENT", "setting value carries both an installation and its space");
		const permitted = position === "own" ? this.definition.overrides : ["installation"];
		if ([
			"package",
			"space",
			"installation",
			"device"
		].filter((override) => value[override] !== void 0).some((override) => !permitted.includes(override))) throw new SettingError("INVALID_PLACEMENT", "setting value uses an unsupported setting override");
	}
	/** Rank a value of this setting that applies to a selection, or none for another value. */
	#candidate(value, selection, chain) {
		if (value.packageId !== this.reference.packageId || value.name !== this.reference.name) return;
		const placement = SettingPlacement.of(value);
		let rank;
		if (value.scope === selection.scope) {
			this.requirePlacement({
				...placement,
				mode: value.mode
			}, "own");
			const weight = setWeight(placement, selection);
			rank = weight === void 0 ? void 0 : [SET_TIER, weight];
		} else if (value.mode === "set") throw new SettingError("INVALID_PLACEMENT", "a value set in another scope reached the resolution");
		else {
			this.requirePlacement({
				...placement,
				mode: value.mode
			}, "enclosing");
			const depth = chain.indexOf(value.scope);
			if (depth === -1) throw new SettingError("INVALID_PLACEMENT", "a value from outside the selection's scope chain reached the resolution");
			const nearness = (chain.length - depth) * DEPTH_WEIGHT;
			rank = placement.installation === void 0 ? [ENCLOSING_TIER, nearness] : placement.installation === selection.installation ? [ENCLOSING_TIER, nearness + ENCLOSING_INSTALLATION_WEIGHT] : void 0;
		}
		if (rank === void 0) return;
		const placed = {
			id: value.id,
			revision: value.revision,
			...placement
		};
		const converted = this.#convert(value.value, value.release);
		const parsed = converted.isConverted ? this.definition.schema.safeParse(converted.value) : void 0;
		if (parsed?.success !== true) return {
			rank,
			source: {
				kind: "invalid",
				...placed
			},
			value: value.value,
			mode: value.mode
		};
		return {
			rank,
			source: {
				kind: "value",
				...placed
			},
			value: parsed.data,
			mode: value.mode
		};
	}
	/** Convert a value of an earlier release to this one, refusing a value of a later release. */
	#convert(value, release) {
		const current = this.package.version;
		if (Version.compare(release, current) > 0) return { isConverted: false };
		const convert = this.definition.convert ?? {};
		return {
			isConverted: true,
			value: Version.between(Object.keys(convert), release, current).reduce((earlier, target) => Expression.evaluate(convert[target], { value: earlier }), value)
		};
	}
};
/** Match a value set in the selected scope and weigh its overrides, or none when one differs. */
function setWeight(placement, selection) {
	const matches = [
		{
			override: placement.package,
			selected: selection.package,
			weight: PACKAGE_WEIGHT
		},
		{
			override: placement.installation,
			selected: selection.installation,
			weight: INSTALLATION_WEIGHT
		},
		{
			override: placement.space,
			selected: selection.space,
			weight: SPACE_WEIGHT
		},
		{
			override: placement.device,
			selected: selection.device,
			weight: DEVICE_WEIGHT
		}
	].filter((match) => match.override !== void 0);
	return matches.every((match) => match.override === match.selected) ? matches.reduce((weight, match) => weight + match.weight, 0) : void 0;
}
/** Order candidates by their ranks. */
function compareRanks(left, right) {
	return left.rank[0] - right.rank[0] || left.rank[1] - right.rank[1];
}
/** Order candidates by their sources. */
function compareSources(left, right) {
	return compareCanonical(left.source, right.source);
}
/** Order JSON values by their canonical form. */
function compareCanonical(left, right) {
	const first = canonicalize(left);
	const second = canonicalize(right);
	return first < second ? -1 : first > second ? 1 : 0;
}
/** Compare JSON values independently of key order. */
function equalValue(left, right) {
	return canonicalize(left) === canonicalize(right);
}
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
/** Declare a typed setting. */
function defineSetting(definition, module) {
	const owner = Package.parse(declaringModule(module, "defineSetting").package);
	const { schema: valueSchema, default: defaultValue, scope, overrides, convert, ...metadata } = definition;
	SettingMetadata.parse(metadata);
	SettingScope.parse({
		scope,
		overrides
	});
	Version.requireUpTo(convert ?? {}, owner.version, definition.name);
	defineSchema(valueSchema);
	valueSchema.parse(defaultValue);
	json().parse(defaultValue);
	return new Setting(owner, definition);
}
var __destackModule = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73f2-a9cd-9cb63f4d528d",
	"name": "@example/stack",
	"version": "2026.9.0"
} });
/** Describe a shared setting without reading runtime values. */
var language = defineSetting({
	name: "language",
	title: "Language",
	description: "Language used for shared documents.",
	schema: _enum(["en", "de"]),
	default: "en",
	scope: "space",
	overrides: ["installation"],
	apply: "immediate"
}, __destackModule);
/** Declare environments independently of the destination space. */
var account = defineAccount({ environments: {
	development: {},
	production: {}
} }, __destackModule);
/** Configure a space without provisioning resources during compilation. */
var personal = defineSpace({
	resources: { main: {
		declaration: database,
		retention: "retain",
		tags: {}
	} },
	installations: { notes: install(package_default, { main: "main" }) },
	settings: {
		language: {
			setting: language,
			value: "de",
			mode: "set"
		},
		"language-policy": {
			setting: language,
			value: "en",
			mode: "recommend"
		}
	}
}, __destackModule);
export { account, database, language, personal, package_default as stack };

//# sourceMappingURL=index-Ci4mocPI.js.map