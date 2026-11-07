import { testSimulation } from "@destack/object/test";
import { MemoryBuild } from "@destack/package/test";
import { expect, refusal, test } from "@destack/test";
import { channelFiles } from "@destack/db/channel/socket";
import { PackageId, Sandbox } from "@destack/package";
import { ResourceId } from "@destack/resource";
import { schema } from "@destack/schema";
import type { InstanceSpec } from "@destack/stack/runtime";
import { type BunWorkloadSandboxOptions, BunWorkloadSandbox } from "./sandbox.ts";

/** The world the fixture's builds are hashed in. */
const fixtureWorld = testSimulation().environment("fixture");

/** The package the workload belongs to. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** The space of the installation. */
const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f2");

/** An instance of a workload binding one SQLite database, without capabilities. */
const spec: InstanceSpec = {
    settings: {},
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000f3"),
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-0000000000f4"),
    name: "notes",
    scope,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000f5"),
    build: await MemoryBuild.write(
        new Map(),
        { role: "app",
            package: {
                id: PACKAGE,
                name: "@example/indexer",
                version: "2026.10.0",
            },
        },
        fixtureWorld.crypto,
    ),
    output: "bun",
    workload: "indexer",
    enabled: [],
    sandbox: {},
    directories: [],
    served: [],
    installations: [],
    resources: [
        {
            id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000f6"),
            scope,
            kind: "database",
            spec: {},
            reference: "file:///spaces/space-1/main.db",
            packageId: PACKAGE,
            name: "main",
            provider: "sqlite",
        },
        {
            id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000f7"),
            scope,
            kind: "service",
            spec: {},
            reference: "notes",
            packageId: PACKAGE,
            name: "notes",
            provider: "http",
        },
    ],
};

/** What the machine gives the workload's sandbox. */
const host: BunWorkloadSandboxOptions = {
    executable: "/usr/local/bin/bun",
    directories: {
        build: "/workers/instance-1",
        roots: ["/store/trees/0a1b.bun", "/store/trees/2c3d.bun"],
        data: "/workers/installation-1/data",
        cache: "/workers/installation-1/cache",
    },
    runner: "bun/workload.js",
    runtime: "/run/user/501",
    egress: "http://127.0.0.1:7470/.destack/egress",
    socket: "/run/user/501/destack-instance-1.sock",
    environment: { PATH: "/usr/bin:/bin", GITHUB_TOKEN: "token", HOME: "/Users/person" },
    which: (command) => (command === "git" ? "/opt/homebrew/bin/git" : undefined),
};

test("confine a workload without capabilities to its build, resources and egress", () => {
    const channels = channelFiles("/spaces/space-1/main.db#log", "/run/user/501");
    expect(new BunWorkloadSandbox(host).options(spec)).toEqual({
        executable: "/usr/local/bin/bun",
        arguments: [
            "--no-env-file",
            "/workers/instance-1/bun/workload.js",
            "/run/user/501/destack-instance-1.sock",
        ],
        directory: "/workers/instance-1",
        environment: {
            PATH: "/usr/bin:/bin",
            HOME: "/workers/installation-1/data",
            XDG_DATA_HOME: "/workers/installation-1/data",
            XDG_CACHE_HOME: "/workers/installation-1/cache",
            XDG_RUNTIME_DIR: "/run/user/501",
        },
        read: ["/workers/instance-1", "/store/trees/0a1b.bun", "/store/trees/2c3d.bun"],
        write: [
            "/workers/installation-1/data",
            "/workers/installation-1/cache",
            "/run/user/501/destack-instance-1.sock",
            "/spaces/space-1/main.db",
            "/spaces/space-1/main.db-wal",
            "/spaces/space-1/main.db-shm",
            "/spaces/space-1/main.db-journal",
            ...Object.values(channels),
        ],
        network: ["127.0.0.1:7470"],
        sockets: ["/run/user/501/destack-instance-1.sock", channels.socket],
        listening: "denied",
    });
});

test("open a workload's sandbox to exactly what its capabilities grant, refusing a command the machine lacks", async () => {
    // grant hosts, directories, variables and a command the machine has
    const sandbox = Sandbox.parse({
        network: { connect: ["api.github.com", "*.githubusercontent.com:443"] },
        server: {},
        files: { access: "write" },
        env: { names: ["GITHUB_TOKEN", "GITLAB_TOKEN"] },
        run: { commands: ["git"] },
    });
    const directories = [
        { path: "/Users/person/Notes", access: "read" as const },
        { path: "/Users/person/Exports", access: "write" as const },
    ];
    const options = new BunWorkloadSandbox(host).options({ ...spec, sandbox, directories });
    const { files: _files, ...withoutFiles } = sandbox;
    const withheld = new BunWorkloadSandbox(host).options({
        ...spec,
        sandbox: withoutFiles,
        directories,
    });

    // ignore the capabilities browsers enforce
    const browser = new BunWorkloadSandbox(host).options({
        ...spec,
        enabled: [],
        sandbox: { camera: {}, "clipboard-read": {} },
    });

    // refuse a command the machine lacks
    const lacking = { ...sandbox, run: { commands: ["git", "hg"] } };
    const refused = await refusal(
        Promise.try(() =>
            new BunWorkloadSandbox(host).options({ ...spec, sandbox: lacking, directories }),
        ),
    );

    expect({
        environment: options.environment,
        read: options.read,
        write: options.write.slice(-1),
        network: options.network,
        listening: options.listening,
        withheld: [withheld.read, withheld.write],
        browser,
        refused,
    }).toEqual({
        environment: {
            GITHUB_TOKEN: "token",
            PATH: "/usr/bin:/bin",
            HOME: "/workers/installation-1/data",
            XDG_DATA_HOME: "/workers/installation-1/data",
            XDG_CACHE_HOME: "/workers/installation-1/cache",
            XDG_RUNTIME_DIR: "/run/user/501",
        },
        read: [
            "/workers/instance-1",
            "/store/trees/0a1b.bun",
            "/store/trees/2c3d.bun",
            "/Users/person/Notes",
            "/opt/homebrew/bin/git",
        ],
        write: ["/Users/person/Exports"],
        network: ["127.0.0.1:7470", "api.github.com", "*.githubusercontent.com:443"],
        listening: "loopback",
        withheld: [
            [
                "/workers/instance-1",
                "/store/trees/0a1b.bun",
                "/store/trees/2c3d.bun",
                "/opt/homebrew/bin/git",
            ],
            options.write.slice(0, -1),
        ],
        browser: new BunWorkloadSandbox(host).options(spec),
        refused: [
            "INTERNAL_SERVER_ERROR",
            "COMMAND_MISSING",
            "notes runs hg, which this machine lacks",
        ],
    });
});

test("refuse connecting to any host, which the sandbox cannot enforce", async () => {
    const sandbox = Sandbox.parse({
        network: { connect: ["*"] },
    });

    expect(await refusal(() => new BunWorkloadSandbox(host).options({ ...spec, sandbox }))).toEqual(
        [
            "UNPROCESSABLE_CONTENT",
            "ABILITY_UNENFORCEABLE",
            "connecting to any host is not enforced by this machine's sandbox, which allows listed hosts only",
        ],
    );
});
