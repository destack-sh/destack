import type { DatabaseConnection } from "@destack/db";
import { schema, type Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import type { Server } from "@destack/service/server";
import { build, Build } from "@destack/space/object";
import type { CheckoutSource } from "@destack/space/server";
import { connect } from "@destack/space/client";

/** How long a checkout's build caller waits: past the compiler's 60 s limit and about 10 s storing 2000 files 8 at a time. */
const BUILD_WAIT_MILLISECONDS = 5 * 60_000;

/** What opens space sessions as a person: the people's sessions and the space service of the machine's spaces. */
export interface SpaceSessionOptions {
    /** The machine's database with the copies of the builds it runs, whose changes a build's wait follows. */
    readonly database: DatabaseConnection;
    /** Read the session of a person signed in on the machine's native client, refusing one signed out. */
    session(login: Identifier<"login">): Promise<string>;
    /** The space service while the machine serves its spaces. */
    spaces(): Pick<Server, "fetch"> | undefined;
}

/** A signed-in person's session with the spaces a machine serves. */
export class SpaceSession {
    /** The client of the space service, calling as the person. */
    readonly client: ReturnType<typeof connect>;
    /** The database whose commits a build's progress follows. */
    readonly #database: DatabaseConnection;

    /** Keep a client calling as a person. */
    private constructor(client: ReturnType<typeof connect>, database: DatabaseConnection) {
        this.client = client;
        this.#database = database;
    }

    /** Connect to the space service as the person a login signed in, refusing before the machine serves spaces and after sign-out. */
    static async open(
        options: SpaceSessionOptions,
        loginId: Identifier<"login">,
    ): Promise<SpaceSession> {
        // require the space service
        const server = options.spaces();
        if (server === undefined) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "this machine serves no spaces before it enrolls",
            });
        }

        // call it with the person's session
        const token = await options.session(loginId);
        const client = connect({
            url: "http://127.0.0.1/",
            headers: { authorization: `Bearer ${token}` },
            fetch: (request: Request) => server.fetch(request),
        });

        return new SpaceSession(client, options.database);
    }

    /** Build a checkout's package directory in a space, and wait until it succeeds, refusing one that failed or was cancelled. */
    async build(spaceId: string, source: CheckoutSource, outputs: readonly string[]) {
        // note the log position before the build exists
        const space = schema.identifier("space").parse(spaceId);
        const { sequence } = await this.#database.log.position();

        // create the build for the machine to run
        const created = await this.client.build.create({
            spaceId: space,
            requestId: RequestId.create(),
            source,
            outputs: [...outputs],
        });

        // read the build again at each of its changes until it finished
        let current = created;
        const changes = this.#database.log.follow(
            { tables: [build.table], after: sequence },
            AbortSignal.timeout(BUILD_WAIT_MILLISECONDS),
        );
        for await (const page of changes) {
            if (page.changes.some((change) => change.key.id === created.id)) {
                current = await this.client.build.get({ spaceId: space, id: created.id });
            }
            if (Build.isFinished(current)) {
                break;
            }
        }

        // refuse a build still running after the wait or one that did not succeed
        if (!Build.isFinished(current)) {
            throw new ServiceError("TIMEOUT", {
                message: `build ${created.id} did not finish in ${BUILD_WAIT_MILLISECONDS} ms`,
            });
        }

        return { id: created.id, space, manifest: Build.manifest(current) };
    }
}
