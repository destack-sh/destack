import type { Clock } from "@destack/environment";
import type { Example } from "@destack/package/declare";
import { RequestId, type Subject } from "@destack/sync";
import { type ClientOptions, type ClientTarget, createClient } from "../client/package.ts";
import type { PackageHandle } from "../declare/package.ts";
import type { ObjectType } from "../object/object.ts";
import type { Client, ProcedureRouter } from "../procedure/index.ts";
import type { Server } from "../server/http.ts";
import { TestBearer } from "./bearer.ts";

/** The origin test clients address the servers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://objects.test";

/** A scope served to test callers: its package servers and the object types they serve. */
export interface TestServed {
    /** The served scope. */
    readonly scope: string;
    /** The clock the calls read. */
    readonly clock: Clock;
    /** Read the server of a served package, refusing another. */
    readonly server: (handle: PackageHandle) => Server;
    /** Read a served object type with its package, absent for a type served nowhere. */
    readonly type: (
        name: string,
    ) => { readonly type: ObjectType; readonly handle: PackageHandle } | undefined;
}

/** A caller of a served scope, calling through typed clients and making examples' calls. */
export class TestCaller {
    /** The caller. */
    readonly subject: Subject;
    /** The scope it calls. */
    readonly #served: TestServed;
    /** Whether the caller authenticated with a second factor just before each call. */
    readonly #isSteppedUp: boolean;

    /** Call a served scope as a subject, stepped up when asked. */
    constructor(served: TestServed, subject: Subject, isSteppedUp = false) {
        this.#served = served;
        this.subject = subject;
        this.#isSteppedUp = isSteppedUp;
    }

    /** Call as the same subject having just authenticated with a second factor (RFC 9470). */
    stepUp(): TestCaller {
        return new TestCaller(this.#served, this.subject, true);
    }

    /** Call a served package through its typed client. */
    client<Definition extends ProcedureRouter>(
        handle: PackageHandle & ClientTarget<Definition>,
    ): Client<Definition> {
        return createClient(handle, this.endpoint(handle));
    }

    /** Address a served package. */
    endpoint(handle: PackageHandle): ClientOptions {
        const server = this.#served.server(handle);

        return {
            url: TEST_ORIGIN,
            headers: TestBearer.headers(this.subject, this.#isSteppedUp),
            fetch: (request: Request) => server.fetch(request),
        };
    }

    /** Make an example's calls in the served space, each through the caller's client. */
    async given(example: Pick<Example, "objects">): Promise<void> {
        for (const [scope, calls] of Object.entries(example.objects)) {
            // refuse calls in a scope the store does not serve
            if (scope !== "space") {
                throw new TypeError(
                    `the test store serves its space alone, not the example's ${scope}`,
                );
            }

            // call each method in the served scope through the package's client
            for (const { object, method, input } of calls) {
                const served = this.#served.type(object.name);
                if (served === undefined) {
                    throw new TypeError(`the test store serves no object type ${object.name}`);
                }
                const { type, handle } = served;
                const procedure: unknown = Reflect.get(
                    Reflect.get(this.client(handle), type.name),
                    method,
                );
                if (typeof procedure !== "function") {
                    throw new TypeError(`${type.name} has no method ${method}`);
                }
                await Reflect.apply(procedure, undefined, [
                    {
                        ...type.in(this.#served.scope),
                        ...(type.method(method).mutates
                            ? { requestId: RequestId.create(this.#served.clock.now()) }
                            : {}),
                        ...input,
                    },
                ]);
            }
        }
    }
}
