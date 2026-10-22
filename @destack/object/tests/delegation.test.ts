import { AccessError, principal, through } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Clock } from "@destack/environment";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { definePackage } from "@destack/object";
import { testCallKey, testKeys } from "@destack/object/test";
import type { ServerContext } from "@destack/object/workload";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject } from "../src/index.ts";
import { implementPackage, ObjectStore } from "../src/server/index.ts";
import { testEvents } from "../src/test/index.ts";
import { spaceId } from "./fixture/device.ts";
import { openSpace, space } from "./fixture/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The installation serving the probes in the space. */
const INSTALLATION = "installation-01996ab0-0000-7000-8000-000000000051";

/** Probes answering whom their code acts as when it acts for its space. */
const probe = defineObject(
    {
        name: "probe",
        plural: "probes",
        scope: space,
        fields: {},
        permissions: { read: through("scope", "read") },
    },
    (method) => ({
        acting: method.query({
            permission: "read",
            target: "scope",
            output: schema.object({
                subject: schema.string(),
                actors: schema.array(schema.string()),
                grant: schema.string().nullable(),
                capabilities: schema.array(schema.boolean()),
            }),
        }),
    }),
).implement({
    methods: {
        acting: async (context) => {
            // act for the space, and answer the caller's subject, actors and grant, and two capabilities
            const acting = await context.space();
            const capabilities = [context.enabled("push"), context.enabled("sms")];

            return {
                subject: acting.subject.id,
                actors: acting.actors.map((actor) => actor.id),
                grant: acting.grants.at(-1)?.id ?? null,
                capabilities,
            };
        },
    },
});

/** The package of the probes. */
const probes = definePackage({ objects: { probe } });

test("act for the space as its installation within the space's grant, refusing without one, and read its capabilities", async () => {
    // serve the probes in a public space as its installation
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { space, probe }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const server: ServerContext = {
        installation: INSTALLATION,
        name: "probes",
        space: spaceId,
        build: await MemoryBuild.write(new Map(), { package: probe.package }),
        resources: new ResourceContext(),
        fetch: (sent) => Promise.reject(new Error(`the probes reach no ${sent.url}`)),
        url: "https://probes--home.test",
        publisher: {
            confirm: async () => {},
            stream: () => {
                throw new TypeError("the fixture streams no replicas");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
            call: () => Promise.reject(new TypeError("the fixture answers no queries")),
        },
        settings: {},
        capabilities: ["push"],
        keys: await testKeys(),
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        at: () => {
            throw new TypeError("the fixture fetches no address");
        },
    };
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        server,
        servers: [implementPackage(probes, { objects: [probe] })],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: probe.package },
    });

    // ask whom the code acts as, before and after the space grants its installation reading
    const ask = () =>
        store
            .query(store.serving(probe), probe.call("acting", { spaceId }))
            .catch((error: unknown) =>
                error instanceof AccessError
                    ? `${error.code}: ${error.message}`
                    : Promise.reject(error),
            );
    const before = await ask();
    const installation = principal.installation.reference(spaceId, INSTALLATION);
    const grant = await new AccessFixture(database).replicateGrant(
        principal.space.reference(Scope.universe.id, spaceId),
        installation,
        [probe.permission("read")],
    );

    // refuse acting for the space without a grant, then act as the space with the installation acting
    expect([before, await ask()]).toEqual([
        `FORBIDDEN: installation ${INSTALLATION} acts for space ${spaceId} under no grant`,
        { subject: spaceId, actors: [INSTALLATION], grant, capabilities: [true, false] },
    ]);
});
