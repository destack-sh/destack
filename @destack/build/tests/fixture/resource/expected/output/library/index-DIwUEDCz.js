import { B as integer, D as Resource, M as expandTrees, O as defineResourceKind, R as declaringModule, U as defineSchema, V as text, X as strictObject, a as declareState, j as defineTable, k as TABLE, n as DatabaseState, t as DatabaseTier } from "./tier-JRpQKvAN.js";
/** The connectors opening databases on Bun: SQLite files, loaded on first connect. */
var connectors = { sqlite: {
	code: "sqlite",
	connect: async (bound, declaration) => {
		const { sqliteConnector } = await import("./connector-DE5F-lsu.js");
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
var __destackModule = Object.freeze({ "package": {
	"id": "package-01a0c80b-614f-73f0-b2a5-37fb7fb295b5",
	"name": "@example/resources",
	"version": "2026.9.0"
} });
/** Notes stored in the destination database. */
var note = defineTable("note", {
	id: integer("id").primaryKey(),
	title: text("title").notNull()
}, void 0, __destackModule);
/** The database selected by the destination space. */
var database = defineDatabase({
	name: "main",
	tables: [note]
}, __destackModule);
export { database, note };

//# sourceMappingURL=index-DIwUEDCz.js.map