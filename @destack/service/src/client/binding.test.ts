import { reference } from "@destack/package/declare";
import { expect, test } from "@destack/test";
import { ResourceContext } from "@destack/resource/context";
import { schema } from "@destack/schema";
import { defineService, defineServiceConnection } from "../declare/index.ts";
import { defineProcedure } from "../procedure/index.ts";
import { Health } from "../health/index.ts";
import { implement, Server, type ServiceContext } from "../server/index.ts";
import { hosting } from "../server/tests/fixture.ts";
import { bindServiceConnection } from "./binding.ts";
import { ClientContext } from "./context.ts";

test("route independent connections through typed clients and retain verified caller identity", async () => {
    // provide one API used through independently selected connections
    const router = {
        read: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "GET", path: "/value" })
            .output(schema.string()),
    };
    const module = {
        package: { id: hosting.audience, name: "@example/notes", version: "2026.9.0" },
    };
    const service = defineService("notes", router, module);
    const personal = defineServiceConnection("personal", service, module);
    const work = defineServiceConnection("work", service, module);
    const implementation = implement(router).$context<ServiceContext>();
    const server = Server.start({
        ...hosting,
        drainTimeout: 1000,
        health: new Health("binding"),
        router: implementation.router({
            read: implementation.read.handler(
                ({ context }) => context.requireCaller().authentication.subject.id,
            ),
        }),
    });

    try {
        // bind each declared dependency once without sharing caller credentials
        const context = new ResourceContext();
        for (const [connection, user] of [
            [personal, "alice"],
            [work, "bob"],
        ] as const) {
            bindServiceConnection(
                connection,
                {
                    declaration: reference(connection),
                    url: "https://service.test",
                },
                {
                    headers: { authorization: user },
                    fetch: (request) => server.fetch(request),
                },
                context,
            );
        }
        expect(await Promise.all([personal.get(context).read(), work.get(context).read()])).toEqual(
            ["alice", "bob"],
        );

        // reject mismatched, missing and duplicate bindings before sending a request
        expect(() => personal.get(new ResourceContext())).toThrow(
            "Resource is not bound: personal",
        );
        expect(() =>
            bindServiceConnection(
                personal,
                {
                    declaration: reference(work),
                    url: "https://service.test",
                },
                {},
                new ResourceContext(),
            ),
        ).toThrow("service connection binding does not match its declaration");
        expect(() =>
            bindServiceConnection(
                personal,
                {
                    declaration: reference(personal),
                    url: "https://service.test",
                },
                {},
                context,
            ),
        ).toThrow("Resource already bound: personal");

        // reject absent or ambiguous host configuration before constructing a client
        const binding = {
            declaration: reference(personal),
            url: "https://service.test",
        };
        for (const services of [[], [binding, binding]]) {
            const client = new ClientContext({ packageId: hosting.audience, services }, {});
            expect(() => client.bind(personal)).toThrow(
                `client requires one binding for ${personal.package.id}/${personal.name}`,
            );
        }
    } finally {
        await server.close();
    }
});
