import { edit, EditOutput } from "@destack/stack/object";
import { principal } from "@destack/access";
import { Scope, Subject } from "@destack/sync";
import { recorded } from "@destack/workflow";
import type { Space } from "../space/index.ts";

/** The function a runner pushes an edit with, which the edit workflow places. */
export const EDIT_PUSH = { name: "edit.push", workflow: edit } as const;

/** Serve edits: each run commits on its branch through a step placed on the space's runner. */
export function implementEdit(space: Space) {
    return edit.implement({
        run: async (step, input) => {
            // push the edit from the space's runner
            const runner = await step.run("runner", Subject, async () =>
                principal.machine.reference(Scope.universe.id, space.runner()),
            );

            return step.place(
                "push",
                EditOutput,
                { function: EDIT_PUSH.name, input: recorded(input) },
                runner,
            );
        },
    });
}
