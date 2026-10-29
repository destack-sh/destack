import { testSimulation } from "../src/test/index.ts";
import { MemoryBlobStorage } from "@destack/resource";
import { defineDatabase } from "@destack/db";
import { AccessError, principal, through } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { TestDatabase } from "@destack/db/test";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { definePackage } from "@destack/object";
import { testCallKey, testKeys } from "@destack/object/test";
import type { InstalledServerContext } from "@destack/object/workload";
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
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-000000000051");

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
            // act with the space's delegation, and answer the caller's subject, actors and grant, and two capabilities
            const acting = await context.delegated();
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

test("act with the space's delegation to its installation, refusing without one, and read its capabilities", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

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
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, world, spaceId);
    const server: InstalledServerContext = {
        installation: INSTALLATION,
        name: "probes",
        scope: spaceId,
        build: await MemoryBuild.write(new Map(), { package: probe.package }, world.crypto),
        environment: world,
        resources: new ResourceContext(),
        fetch: (sent) => Promise.reject(new Error(`the probes reach no ${sent.url}`)),
        url: "https://probes--home.test",
        publisher: {
            request: async () => {},
            stream: () => {
                throw new TypeError("the fixture streams no replicas");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
            call: () => Promise.reject(new TypeError("the fixture answers no queries")),
        },
        settings: {},
        capabilities: ["push"],
        keys: await testKeys(world),
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        at: () => {
            throw new TypeError("the fixture fetches no address");
        },
    };
    const store = new ObjectStore({
        chain: async () => undefined,
        environment: world,
        blobs: new MemoryBlobStorage(world.crypto),
        server,
        servers: [implementPackage(probes, { objects: [probe] })],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: probe.package },
    });

    // ask whom the code acts as, before and after the space grants its installation reading
    const ask = () =>
        store
            .query(store.as(probe), probe.call("acting", { spaceId }))
            .catch((error: unknown) =>
                AccessError.is(error) ? `${error.reason}: ${error.message}` : Promise.reject(error),
            );
    const before = await ask();
    const installation = principal.installation.reference(spaceId, INSTALLATION);
    const grant = await new AccessFixture(database, world).replicateGrant(
        principal.space.reference(Scope.universe.id, spaceId),
        installation,
        [probe.permission("read")],
    );

    // refuse acting for the space without a grant, then act as the space with the installation acting
    expect([before, await ask()]).toEqual([
        `GRANT_REQUIRED: installation ${INSTALLATION} has no grant to act for space ${spaceId}`,
        { subject: spaceId, actors: [INSTALLATION], grant, capabilities: [true, false] },
    ]);
});
