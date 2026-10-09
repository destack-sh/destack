import type { DatabaseConnection } from "@destack/db";
import type { Extension } from "@destack/object/server";
import { PackageInspection } from "@destack/package/code";
import type { BuildReader } from "@destack/package/manifest";
import { type Identifier, schema } from "@destack/schema";
import { implement, type Server, type ServiceContext } from "@destack/service/server";
import { Checkout } from "../checkout/index.ts";
import { checkout, preview } from "../object/index.ts";
import { buildService } from "../service/index.ts";
import type { PackageBuilders } from "./builders.ts";
import { serveCheckouts } from "./checkout.ts";
import { PreviewController, SourceController } from "./preview.ts";

/** What a machine runs its development workflow with: its checkouts' database, its warm builders, its spaces and the sessions of its people. */
export interface BuildServiceOptions {
    /** The machine registering the checkouts. */
    readonly machine: Identifier<"machine">;
    /** The machine's database with its checkouts and previews and the copies of the builds it runs. */
    readonly database: DatabaseConnection;
    /** The machine's warm package builders. */
    readonly builders: PackageBuilders;
    /** Read a space's build from the machine serving the space. */
    read(spaceId: Identifier<"space">, manifest: string): Promise<BuildReader>;
    /** The space service while the machine serves its spaces. */
    spaces(): Pick<Server, "fetch"> | undefined;
    /** Read the session of a person signed in on the machine's native client, refusing one signed out. */
    session(login: Identifier<"login">): Promise<string>;
}

/** The checkout an inspection reads, which its permission is decided on. */
const InspectTarget = schema.object({
    /** The checkout with the package. */
    checkout: schema.identifier("checkout"),
});

/** Serve a machine's development workflow in its own scope: its checkouts, the previews running their packages, and inspections with its warm builders. */
export function implementBuild(options: BuildServiceOptions): Extension {
    return {
        service: buildService,
        objects: { checkout: serveCheckouts(options), preview },
        serve: (server) => ({
            access: {
                ...server.access,
                target: async (call) =>
                    checkout.reference(options.machine, InspectTarget.parse(call.input).checkout),
            },
            procedures: route(options),
            controllers: [
                new PreviewController(server, options),
                new SourceController(server, options),
            ],
        }),
    };
}

/** Route the inspections beside the objects. */
function route(options: BuildServiceOptions) {
    const implementation = implement(buildService.router).$context<ServiceContext>();

    return {
        inspect: implementation.inspect.handler(async ({ input, signal }) => {
            // inspect a package directory of a checkout the machine registers
            const located = await Checkout.locate(options.database, options.machine, input);

            return PackageInspection.parse(
                await options.builders.inspect(located.path, input.output, signal),
            );
        }),
    };
}
