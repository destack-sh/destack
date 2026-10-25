import { schema } from "@destack/schema";
import { expectTypeOf, test } from "@destack/test";
import { defineWorkflow, NO_OUTPUT } from "../src/index.ts";
import { note } from "./fixture.ts";
import { NOTES } from "./package.ts";

/** A workflow waiting for nothing beside change requests. */
const quiet = defineWorkflow({ name: "quiet", input: schema.object({}), output: NO_OUTPUT }, NOTES);

/** The steps quiet's code takes. */
type QuietStep = Parameters<Parameters<typeof quiet.implement>[0]>[0];

test("refuse at compile time a wait, an approval or an invocation of a type the workflow does not wait for", () => {
    // refuse a wait for notes, which quiet does not wait for
    const listing = note.call("list", { spaceId: "space", limit: 1 });
    expectTypeOf(listing).not.toExtend<Parameters<QuietStep["waitFor"]>[1]>();

    // refuse an approval, its last parameter never takes a value
    expectTypeOf<Parameters<QuietStep["approve"]>[3]>().toEqualTypeOf<never>();

    // refuse invoking quiet itself, whose runs it does not wait for
    expectTypeOf(quiet).not.toExtend<Parameters<QuietStep["invoke"]>[1]>();
});
