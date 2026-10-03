import { expect, test } from "@destack/test";
import { channelFiles } from "@destack/db/channel/socket";
import { Capabilities, CapabilityError, PackageId } from "@destack/package";
import { ResourceId } from "@destack/resource";
import { schema } from "@destack/schema";
import type { InstanceSpec } from "../src/runtime/index.ts";
import { type SandboxPlaces, WorkloadSandbox } from "../src/runtime/sandbox.ts";
import { memoryBuild } from "../src/test/build.ts";

/** The package the workload belongs to. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000f1");

/** The space of the installation. */
const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f2");

/** An instance of a workload binding one SQLite database, without capabilities. */
const spec: InstanceSpec = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000f3"),
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-0000000000f4"),
    scope,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000f5"),
    build: await memoryBuild(
        { id: PACKAGE, name: "@example/indexer", version: "2026.10.0" },
        {},
        new Map(),
    ),
    output: "bun",
    workload: "indexer",
    capabilities: {},
    directories: [],
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

/** Where the host runs the workload. */
const places: SandboxPlaces = {
    executable: "/usr/local/bin/bun",
    directory: "/workers/instance-1",
    runner: "bun/workload.js",
    data: "/workers/installation-1/data",
    cache: "/workers/installation-1/cache",
    runtime: "/run/user/501",
    egress: "http://127.0.0.1:7470/.destack/egress",
    environment: { PATH: "/usr/bin:/bin", GITHUB_TOKEN: "token", HOME: "/Users/person" },
    which: (command) => (command === "git" ? "/opt/homebrew/bin/git" : undefined),
};

test("confine a workload without capabilities to its files, folders, resources and the egress", () => {
    expect(WorkloadSandbox.options(spec, places)).toEqual({
        executable: "/usr/local/bin/bun",
        arguments: ["--no-env-file", "/workers/instance-1/bun/workload.js"],
        directory: "/workers/instance-1",
        environment: {
            PATH: "/usr/bin:/bin",
            HOME: "/workers/installation-1/data",
            XDG_DATA_HOME: "/workers/installation-1/data",
            XDG_CACHE_HOME: "/workers/installation-1/cache",
            XDG_RUNTIME_DIR: "/run/user/501",
        },
        read: ["/workers/instance-1"],
        write: [
            "/workers/installation-1/data",
            "/workers/installation-1/cache",
            "/spaces/space-1/main.db",
            "/spaces/space-1/main.db-wal",
            "/spaces/space-1/main.db-shm",
            "/spaces/space-1/main.db-journal",
            ...Object.values(
                channelFiles("/spaces/space-1/main.db", { XDG_RUNTIME_DIR: "/run/user/501" }),
            ),
        ],
        network: ["127.0.0.1:7470"],
        allowsListening: true,
    });
});

test("open a workload's sandbox to the hosts, directories, variables and commands it is granted, to directories only with fs, and to nothing for browser capabilities", () => {
    // grant hosts, a read and a written directory, a variable the host has and one it lacks, and a command the host has and one it lacks
    const capabilities = Capabilities.parse({
        network: { connect: ["api.github.com", "*.githubusercontent.com:443"], reason: "fetches" },
        listen: { reason: "serves the editor" },
        fs: { access: "write", reason: "indexes notes and saves exports" },
        env: { names: ["GITHUB_TOKEN", "GITLAB_TOKEN"], reason: "authenticates" },
        run: { commands: ["git", "hg"], reason: "reads history" },
    });
    const directories = [
        { path: "/Users/person/Notes", access: "read" as const },
        { path: "/Users/person/Exports", access: "write" as const },
    ];
    const options = WorkloadSandbox.options({ ...spec, capabilities, directories }, places);
    const { fs: _fs, ...withoutFs } = capabilities;
    const withheld = WorkloadSandbox.options(
        { ...spec, capabilities: withoutFs, directories },
        places,
    );

    // ignore the capabilities browsers enforce
    const browser = WorkloadSandbox.options(
        {
            ...spec,
            capabilities: {
                camera: { reason: "scans receipts" },
                "clipboard-read": { reason: "pastes notes" },
            },
        },
        places,
    );

    expect({
        environment: options.environment,
        read: options.read,
        write: options.write.slice(-1),
        network: options.network,
        withheld: [withheld.read, withheld.write],
        browser,
    }).toEqual({
        environment: {
            GITHUB_TOKEN: "token",
            PATH: "/usr/bin:/bin",
            HOME: "/workers/installation-1/data",
            XDG_DATA_HOME: "/workers/installation-1/data",
            XDG_CACHE_HOME: "/workers/installation-1/cache",
            XDG_RUNTIME_DIR: "/run/user/501",
        },
        read: ["/workers/instance-1", "/Users/person/Notes", "/opt/homebrew/bin/git"],
        write: ["/Users/person/Exports"],
        network: ["127.0.0.1:7470", "api.github.com", "*.githubusercontent.com:443"],
        withheld: [["/workers/instance-1", "/opt/homebrew/bin/git"], options.write.slice(0, -1)],
        browser: WorkloadSandbox.options(spec, places),
    });
});

test("refuse connecting to any host, which the sandbox cannot enforce", () => {
    const capabilities = Capabilities.parse({
        network: { connect: ["*"], reason: "crawls" },
    });

    expect(() => WorkloadSandbox.options({ ...spec, capabilities }, places)).toThrow(
        new CapabilityError(
            "UNENFORCEABLE",
            "network",
            "connecting to any host is not enforced by this host's sandbox, which allows listed hosts only",
        ),
    );
});
