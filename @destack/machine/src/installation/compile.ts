import { principal } from "@destack/access";
import { present, schema } from "@destack/schema";
import { build, compile, CompileOutput } from "@destack/stack/object";
import { StackError } from "@destack/stack/error";
import { Scope, Subject } from "@destack/sync";
import { recorded } from "@destack/workflow";
import { Failure } from "@destack/error";
import type { Space } from "../space/index.ts";

/** The function a runner compiles a build with, which the compile workflow places. */
export const BUILD_COMPILE = { name: "build.compile", workflow: compile } as const;

/** Serve compiles: each run places its build's compile on a runner and records the outcome. */
export function implementCompile(space: Space) {
    return compile.implement({
        run: async (step, input) => {
            // compile it on a checkout's own machine, else on the space's runner, refusing a release
            const target = build.target({ scope: space.id, id: input.build });
            try {
                const runner = await step.run("runner", Subject, async () => {
                    // read the build's source
                    const row = present(
                        await space.database
                            .snapshot()
                            .read(build.table)
                            .findFirst({ where: { id: input.build } }),
                        `build '${input.build}'`,
                    );
                    const { source } = row;

                    // place a checkout's compile on its machine, a commit's on the space's runner
                    if (source.kind === "checkout") {
                        return principal.machine.reference(Scope.universe.id, source.machine);
                    } else if (source.kind === "commit") {
                        return space.runner();
                    } else {
                        throw StackError.of(
                            "RELEASE_BLOCKED",
                            "cannot compile a release: it is built",
                        );
                    }
                });
                const compiled = await step.place(
                    "compile",
                    CompileOutput,
                    { function: BUILD_COMPILE.name, input: recorded(input) },
                    runner,
                );
                const finished = await step.run("finished", schema.number(), async () =>
                    space.machine.clock.now(),
                );
                await step.call(
                    "succeed",
                    build.call("update", {
                        ...target,
                        state: "succeeded",
                        finishedAt: finished,
                        image: compiled.image,
                        outputs: compiled.outputs,
                        ...(compiled.commit === undefined ? {} : { commit: compiled.commit }),
                    }),
                );

                return compiled;
            } catch (error) {
                // record the failure on the build, then end the run with it
                const failed = await step.run("failed", schema.number(), async () =>
                    space.machine.clock.now(),
                );
                await step.call(
                    "fail",
                    build.call("update", {
                        ...target,
                        state: "failed",
                        failure: Failure.of(error),
                        finishedAt: failed,
                    }),
                );
                throw error;
            }
        },
    });
}
