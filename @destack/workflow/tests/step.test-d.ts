import { schema } from "@destack/schema";
import { expectTypeOf, test } from "@destack/test";
import { defineWorkflow, NO_OUTPUT, type Workflow, type WorkflowCode } from "../src/index.ts";
import { note } from "./workflow.ts";
import { NOTES } from "./package.ts";
import { t } from "@destack/locale";

/** A workflow waiting for nothing beside change requests. */
const quiet = defineWorkflow(
    { name: "quiet", title: t`Quiet`, input: schema.object({}), output: NO_OUTPUT },
    NOTES,
);

/** The steps quiet's code takes. */
type QuietStep =
    typeof quiet extends Workflow<infer Input, infer Output, infer Waits>
        ? Parameters<WorkflowCode<Input, Output, Waits>["run"]>[0]
        : never;

test("refuse at compile time a wait, an approval or an invocation of a type the workflow does not wait for", () => {
    // refuse a wait for notes, which quiet does not wait for
    const listing = note.call("list", { spaceId: "space", limit: 1 });
    expectTypeOf(listing).not.toExtend<Parameters<QuietStep["waitFor"]>[1]>();

    // refuse an approval, its last parameter never takes a value
    expectTypeOf<Parameters<QuietStep["approve"]>[3]>().toEqualTypeOf<never>();

    // refuse invoking quiet itself, whose runs it does not wait for
    expectTypeOf(quiet).not.toExtend<Parameters<QuietStep["invoke"]>[1]>();
});
