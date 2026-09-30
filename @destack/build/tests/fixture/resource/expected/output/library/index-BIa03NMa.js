import { J as text, P as defineTable, W as integer, n as defineDatabase } from "./database-DFrC3fct.js";
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

//# sourceMappingURL=index-BIa03NMa.js.map