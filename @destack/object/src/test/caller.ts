import type { Environment } from "@destack/environment";
import type { Example } from "@destack/package/declare";
import { type Call, type ObjectTypeReference, RequestId, type Subject } from "@destack/sync";
import { type ClientOptions, type ClientTarget, createClient } from "../client/package.ts";
import type { PackageHandle } from "../declare/package.ts";
import type { ObjectType } from "../object/object.ts";
import { MethodContext } from "../method/context.ts";
import type { Client, ProcedureRouter } from "../procedure/index.ts";
import type { Server } from "../server/http.ts";
import type { TestSigner } from "./store.ts";

/** The origin test clients address the servers at, which no request leaves the process for. */
const TEST_ORIGIN = "https://objects.test";

/** A scope served to test callers: its package servers and the object types they serve. */
export interface TestServed {
    /** The served scope. */
    readonly scope: string;
    /** The world the calls run in. */
    readonly environment: Environment;
    /** Read the server of a served package, refusing another. */
    readonly server: (handle: PackageHandle) => Pick<Server, "fetch">;
    /** Read a served object type with its package, absent for a type served nowhere. */
    readonly type: (
        reference: ObjectTypeReference,
    ) => { readonly type: ObjectType; readonly handle: PackageHandle } | undefined;
    /** Read the one served object type of a name with its package, absent if served nowhere. */
    readonly named: (
        name: string,
    ) => { readonly type: ObjectType; readonly handle: PackageHandle } | undefined;
    /** Sign a request as a caller of a package. */
    readonly sign: (request: Request, signer: TestSigner) => Promise<Request>;
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

    /** Call as the same subject, each request's work run by a driver such as a simulation. */
    driven(drive: <Value>(work: Promise<Value>) => Promise<Value>): TestCaller {
        // sign and serve each request under the driver
        const served = this.#served;
        const driving: TestServed = {
            scope: served.scope,
            environment: served.environment,
            type: (reference) => served.type(reference),
            named: (name) => served.named(name),
            server: (handle) => {
                const server = served.server(handle);

                return { fetch: (request, ...context) => drive(server.fetch(request, ...context)) };
            },
            sign: (request, signer) => drive(served.sign(request, signer)),
        };

        return new TestCaller(driving, this.subject, this.#isSteppedUp);
    }

    /** Call as the same subject having just authenticated with a second factor (RFC 9470). */
    stepUp(): TestCaller {
        return new TestCaller(this.#served, this.subject, true);
    }

    /** Call a served package through its typed client. */
    client<Definition extends ProcedureRouter>(
        handle: PackageHandle & ClientTarget<Definition>,
    ): Client<Definition> {
        return createClient(handle, this.endpoint(handle), this.#served.environment);
    }

    /** Address a served package through a fetch that signs each request. */
    endpoint(handle: PackageHandle): ClientOptions & {
        readonly fetch: (request: Request) => Promise<Response>;
    } {
        const server = this.#served.server(handle);

        return {
            url: TEST_ORIGIN,
            fetch: async (request: Request) => {
                const signer = {
                    subject: this.subject,
                    audience: handle.id,
                    isSteppedUp: this.#isSteppedUp,
                };

                return server.fetch(await this.#served.sign(request, signer));
            },
        };
    }

    /** Make a call through the client of its type's package, answering its parsed result. */
    async call<Result>(
        called: Call<Result>,
        options: { readonly writerId?: string } = {},
    ): Promise<Result> {
        // find the called procedure on the client of the type's package
        const served = this.#served.type(called.type);
        if (served === undefined) {
            throw new TypeError(`the test store serves no object type ${called.type.type}`);
        }
        const { type, handle } = served;
        const client: unknown = Reflect.get(this.client(handle), type.name);
        const procedure: unknown = Reflect.get(Object(client), called.method);
        if (typeof procedure !== "function") {
            throw new TypeError(`${type.name} has no method ${called.method}`);
        }

        // name a mutation's request, or its writer for an ephemeral type
        const named = !type.method(called.method).mutates
            ? {}
            : type.storage === "ephemeral"
              ? { writerId: options.writerId ?? `writer-${this.subject.id}` }
              : { requestId: RequestId.create(this.#served.environment) };
        const answered: unknown = await Reflect.apply(procedure, undefined, [
            { ...called.input, ...named },
        ]);

        // read the answer as the method's result, parsed by its output schema
        const [result] = MethodContext.results([type], [called], [answered]);

        return result;
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
                const served = this.#served.named(object.name);
                if (served === undefined) {
                    throw new TypeError(`the test store serves no object type ${object.name}`);
                }
                const { type } = served;
                await this.call(type.call(method, { ...type.in(this.#served.scope), ...input }));
            }
        }
    }
}
