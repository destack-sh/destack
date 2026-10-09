import type { Select } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { declaredSpace } from "@destack/space/server";
import { Checkout } from "../checkout/index.ts";
import { checkout, type StackSource, type CheckoutSubmission } from "../object/index.ts";
import type { BuildServiceOptions } from "./server.ts";
import { SpaceSession } from "./session.ts";

/** Give checkouts their Git working tree registration and the stacks they submit, built in their spaces. */
export function serveCheckouts(options: BuildServiceOptions) {
    const { builders } = options;

    return checkout.handle({
        create: {
            prepare: (call) => Checkout.open(call.input.directory, call.input.source),
            handler: (call, next) =>
                next(
                    call.with({
                        input: { directory: call.prepared.directory, root: call.prepared.root },
                    }),
                ),
        },
        relocate: {
            prepare: (call) => {
                // require the directory the working tree moved to
                const { directory } = call.input;
                if (directory === undefined) {
                    throw new ServiceError("BAD_REQUEST", {
                        message: "relocate names the directory the working tree moved to",
                    });
                }

                return Checkout.relocated(call.requireTarget().directory, directory);
            },
            handler: (call) => call.update({ ...call.prepared }),
        },
        delete: {
            prepare: async (call) => call.target.root,
            commit: async (_call, prepared) => {
                // close the warm compilers of the unregistered working tree
                await builders.release(prepared);
            },
        },
        members: {
            prepare: (call) => builders.members(call.target.root, call.target.directory),
            handler: (call) => Promise.resolve(call.prepared),
        },
        submit: {
            prepare: (call) => submission(call.target, call.input, options),
            handler: (call) => Promise.resolve(call.prepared),
        },
    });
}

/** Build a checkout's stack package in its space, and read its exported space into its stack submission. */
async function submission(
    registered: Select<typeof checkout.table>,
    source: StackSource,
    options: BuildServiceOptions,
): Promise<CheckoutSubmission> {
    // require a package directory inside the working tree
    const located = Checkout.package(registered.root, source.directory);
    const followed = {
        kind: "checkout" as const,
        machine: options.machine,
        checkout: registered.id,
        directory: located.directory,
    };

    // build every output the package implies in the space as the person
    const session = await SpaceSession.open(options, source.login);
    const built = await session.build(
        source.space,
        followed,
        await options.builders.outputs(located.path),
    );
    const reader = await options.read(built.space, built.manifest);
    const packageId = reader.manifest.package.id;

    return {
        packageId,
        submission: {
            selection: followed,
            build: { kind: "build", build: built.id },
            evaluation: {
                export: source.export,
                parameters: { ...source.parameters },
                definition: await declaredSpace(reader, source.export),
            },
        },
    };
}
