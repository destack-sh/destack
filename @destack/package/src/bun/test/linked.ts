import { BunEnvironment } from "@destack/environment/bun";
import * as schema from "@destack/schema";
import { Program } from "../program.ts";

/** The running process, whose arguments name the module to load. */
const { process } = BunEnvironment.system;

/** The program's own @destack/schema, shared as a compiled program's generated table does. */
const program = Program.of({ "@destack/schema": async () => schema }, process.loader);

// link it, and nothing for a package the program does not share
await program.link(["@destack/schema", "@destack/absent"]);

/** The module the first argument names. */
const loaded: unknown = await import(schema.nth(process.arguments, 0));
/** Whether the module shares the program's instance. */
const isShared =
    schema.schema.object({ schema: schema.schema.unknown() }).parse(loaded).schema ===
    schema.schema;

// report it with what the program shares
process.stdout.writeSync(
    `${JSON.stringify({ isShared, shares: [program.shares("@destack/schema"), program.shares("@destack/absent")] })}\n`,
);
