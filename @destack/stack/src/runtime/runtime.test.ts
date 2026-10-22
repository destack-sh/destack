import { PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { ResourceId } from "@destack/resource";
import { schema } from "@destack/schema";
import { testKeys } from "@destack/object/test";
import { expect, refusal, test } from "@destack/test";
import { InstanceRegistry, InstanceSpec } from "./runtime.ts";

/** The egress the instances call addresses below. */
const EGRESS = "http://127.0.0.1:7470/.destack/egress";

/** The package the instance runs. */
const notes = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e7"),
    name: "@example/notes",
    version: "2026.9.0",
};

/** Another package, whose resources the instance does not bind. */
const other = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e8");

/** An instance as the registry keeps it, with its spec and the secret its requests prove. */
interface Running {
    /** The instance's spec, as far as the registry reads it. */
    readonly spec: Pick<InstanceSpec, "instanceId">;
    /** The secret its requests prove. */
    readonly secret: string;
}

/** The instance the registry starts. */
const spec = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000f2"),
};

test("start an instance once for concurrent starts, identifying its secret while it starts", async () => {
    // launch once for two starts at once, reading the secret before the launch resolves
    const registry = new InstanceRegistry<Running>();
    const launched: string[] = [];
    const ready = Promise.withResolvers<void>();
    const launch = async (identify: (secret: string) => void) => {
        launched.push("launch");
        identify("first");
        await ready.promise;

        return { spec, secret: "first" };
    };
    const starts = [registry.start(spec, launch), registry.start(spec, launch)];
    const starting = [registry.identify("first"), registry.isRunning(spec.instanceId)];
    ready.resolve();
    await Promise.all(starts);

    // leave the running instance as it is for a later start
    await registry.start(spec, launch);

    expect([launched, starting, registry.isRunning(spec.instanceId)]).toEqual([
        ["launch"],
        [spec, false],
        true,
    ]);
});

test("drop a failed start's secret, and forget a stopped instance and its secret", async () => {
    // fail a start after identifying its secret
    const registry = new InstanceRegistry<Running>();
    const failed = await registry
        .start(spec, async (identify) => {
            identify("failed");
            throw new TypeError("the launch failed");
        })
        .then(
            () => "started",
            (error: Error) => error.message,
        );
    const afterFailure = registry.identify("failed");

    // start, stop and look the instance up again
    await registry.start(spec, async (identify) => {
        identify("second");

        return { spec, secret: "second" };
    });
    const stopped = await registry.stop(spec.instanceId);

    expect([failed, afterFailure, stopped, registry.identify("second")]).toEqual([
        "the launch failed",
        undefined,
        { spec, secret: "second" },
        undefined,
    ]);
});

test("forget only the instance still recorded, leaving one a later start replaced", async () => {
    // start, stop and start again
    const registry = new InstanceRegistry<Running>();
    const first = { spec, secret: "first" };
    const second = { spec, secret: "second" };
    await registry.start(spec, async () => first);
    await registry.stop(spec.instanceId);
    await registry.start(spec, async () => second);

    expect([registry.forget(first), registry.list(), registry.forget(second)]).toEqual([
        false,
        [second],
        true,
    ]);
});

test("bind a workload's own provisioned resources by name, refusing another package's", async () => {
    // describe an instance binding a database and an installation its package calls
    const build = await MemoryBuild.write(new Map(), { package: notes });
    const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f4");
    const database = {
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000e5"),
        scope,
        kind: "database",
        spec: {},
        reference: "file:///spaces/main.db",
        packageId: notes.id,
        name: "main",
        provider: "sqlite",
    };
    const dependency = {
        ...database,
        id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000e9"),
        kind: "installation",
        spec: { package: notes.id },
        reference: "notes",
        name: "notes",
        provider: "http",
    };
    const bound: InstanceSpec = {
        ...spec,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000f1"),
        name: "notes",
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000f3"),
        output: "bun",
        workload: "main",
        scope,
        build,
        settings: {},
        sandbox: {},
        enabled: [],
        directories: [],
        served: [],
        installations: [],
        resources: [database, dependency],
    };
    const options = {
        egress: EGRESS,
        url: "https://notes--home.test",
        sampling: 1,
        identity: await testKeys(),
    };

    // start it, then refuse a spec binding a resource of another package
    const started = await InstanceSpec.start(bound, options);
    const foreign = await refusal(
        InstanceSpec.start({ ...bound, resources: [{ ...database, packageId: other }] }, options),
    );

    // bind the database at its URL and the installation at the egress, proven by the instance's secret
    expect([started.bindings, foreign]).toEqual([
        {
            main: {
                resource: database.id,
                kind: "database",
                provider: "sqlite",
                reference: "file:///spaces/main.db",
            },
            notes: {
                resource: dependency.id,
                kind: "installation",
                provider: "http",
                reference: `${EGRESS}/notes`,
                credential: started.secret,
                scope,
            },
        },
        ["PRECONDITION_FAILED", `resource ${database.id} binds no resource of ${notes.id}`],
    ]);
});
