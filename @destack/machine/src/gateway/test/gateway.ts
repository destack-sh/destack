import { installation, deployment, instance, installationRevision } from "@destack/stack/object";
import { Identity } from "@destack/account/object";
import { TokenVerifier, TokenIssuer } from "@destack/access/token";
import { UniverseOrigin } from "@destack/account/address";
import { Directory } from "@destack/account/client";
import type { S3Server } from "@destack/bucket/s3";
import type { Package } from "@destack/object";
import { ids, type TestSpace } from "../../test/space.ts";
import { insertInstallation } from "../../test/install.ts";
import { workload } from "../../test/build.ts";
import { eq } from "@destack/db";
import { Digest, Identifier, present } from "@destack/schema";
import { type GatewaySpace, Gateway, type TokenExchange } from "../gateway.ts";
import type { Machine } from "../../machine/index.ts";
import type { Runtime } from "../../runtime/runtime.ts";
import type { JwtClaims } from "@destack/identity";
import { Traffic } from "../traffic.ts";

/** The universe the fixture gateway's space enrolled at. */
const UNIVERSE = new UniverseOrigin("https://destack.cloud", {
    space: "destack.space",
    machine: "destack.computer",
    site: "destack.sh",
});

/** Refuse a call a test leaves to no fixture. */
function refuse(verb: string): () => Promise<never> {
    return () => Promise.reject(new Error(`the fixture cannot '${verb}'`));
}

/** Open the fixture space's gateway over its database, refusing what a test leaves out. */
export function openGateway(
    space: TestSpace,
    options: {
        /** The directory replica locating spaces, the hosting machine's by default. */
        readonly directory?: Directory;
        /** The packages the space's store serves. */
        readonly packages?: readonly Package[];
        /** Append an instance's telemetry export to its space. */
        readonly observe?: (signal: string, body: unknown) => Promise<void>;
        /** Borrow a connection's credential as a consumer. */
        readonly borrow?: GatewaySpace["borrow"];
        /** The S3 API over the space's buckets. */
        readonly s3?: Pick<S3Server, "fetch" | "serve">;
        /** The runtimes running the space's instances. */
        readonly runtimes?: readonly Runtime[];
        /** How the space acts for its installations' subjects, at the universe. */
        readonly tokens?: TokenExchange;
        /** Send a request to another origin, as the machine sends its installations' calls. */
        readonly send?: Machine["send"];
        /** Hand a request to one of the space's packages as a verified caller. */
        readonly call?: GatewaySpace["call"];
        /** The space's traffic, which its gateway counts. */
        readonly traffic?: Traffic;
        /** The mounts the space's installations declare, none by default. */
        readonly mounts?: GatewaySpace["mounts"];
        /** Sign an installation's token leaving the machine, its claims readable. */
        readonly sign?: (claims: JwtClaims) => string;
    } = {},
): Gateway {
    // locate spaces through the given directory, or the hosting machine's replica of it
    const { database } = space;
    const directory = options.directory ?? space.machine.directory;
    const verifier = new TokenVerifier({
        issuers: (issuer) => Identity.trust(directory.snapshot, issuer, UNIVERSE.issuer),
        environment: space.machine.environment,
    });
    const borrow = options.borrow ?? refuse("lend connections");

    return new Gateway(
        {
            id: ids.space,
            database,
            call: options.call ?? refuse("serve calls"),
            installation: refuse("find installations"),
            // route without holding, as these routes reach installations served already
            waitServing: (resolve) => resolve(),
            observe: options.observe ?? refuse("receive telemetry"),
            borrow,
            mounts: options.mounts ?? (() => []),
            instance: { implements: () => false },
            universe: UNIVERSE,
            identityOf: () => {
                throw new Error("the fixture cannot sign as a space");
            },
            contain: refuse("contain callers"),
            keystore: {
                issuer: (_database, issuer) => {
                    const { sign } = options;
                    if (sign === undefined) {
                        throw new Error("the fixture cannot sign as a space");
                    }

                    return new TokenIssuer({
                        issuer,
                        authority: { kind: "space", space: issuer },
                        sign: async (claims) => sign(claims),
                    });
                },
                deriver: refuse("derive the space's secrets"),
            },
            root: refuse("pull builds"),
            imports: refuse("map builds' imports"),
            builds: {
                contents: refuse("read builds"),
                attachment: refuse("read graphs"),
                graph: refuse("read graphs"),
                file: refuse("read graphs"),
                get: refuse("read builds"),
            },
            machine: {
                directory,
                accounts: () => space.machine.accounts(),
                homes: new Map([[ids.account, Promise.resolve(ids.residencyHome)]]),
                traffic: options.traffic ?? new Traffic(),
                id: space.machine.id,
                clock: space.machine.clock,
                domain: "localhost",
                serve: refuse("serve the machine's own spaces"),
                locate: refuse("locate spaces"),
                environment: space.machine.environment,
                scope: space.machine.scope,
                verifier,
                runtimes: options.runtimes ?? [],
                hibernation: undefined,
                bridge: undefined,
                send: options.send ?? refuse("call other origins"),
                // verify no credential, answering none for a call without one as the machine does
                authenticate: async (request) => {
                    if (request.headers.has("authorization")) {
                        throw new Error("the fixture cannot verify callers");
                    }

                    return undefined;
                },
            },
        },
        {
            s3: options.s3 ?? { fetch: refuse("serve S3"), serve: refuse("serve S3") },
        },
        { packages: (options.packages ?? []).map((handle) => ({ package: handle })) },
        options.tokens,
    );
}

/** A deployment of the fixture's notes installation, running one instance. */
interface NotesDeployment {
    /** The deployment's one running instance. */
    readonly instanceId: Identifier<"instance">;
    /** The release the deployment runs. */
    readonly release: string;
    /** The release its callers speak at least, any by default. */
    readonly since?: string;
    /** Whether it takes new work or drains, an earlier release's draining beside the active one. */
    readonly status: "active" | "draining";
}

/** Install notes in the fixture space, and run one instance of each of its deployments. */
export async function deployNotes(
    space: TestSpace,
    deployments: readonly NotesDeployment[],
): Promise<void> {
    // install notes following a release of it, as the owner
    const { database } = space;
    const { environment } = space.machine;
    const now = environment.clock.now();
    const release = await space.release(ids.package, "2026.9.0");
    const notes = { id: ids.notes, scope: ids.space, packageId: ids.package, name: "notes" };
    await insertInstallation(
        database,
        {
            ...notes,
            follows: await space.followed(ids.package, "2026.9.0"),
            installer: space.owner.subject,
        },
        now,
    );

    // follow the release's revision, applied, served and deployed as the machine's start leaves it
    const record = { scope: ids.space, createdAt: now, updatedAt: now };
    const revisionId = Identifier.create("installation-revision", environment);
    await database.transaction(async (transaction) => {
        // record the revision
        await transaction.insert(installationRevision.table).values({
            ...record,
            id: revisionId,
            installationId: ids.notes,
            buildId: release.id,
            dependencies: [],
            states: {},
            ...release.recorded,
            bindings: [],
            digest: Digest.parse("e".repeat(64)),
            generation: 1,
        });

        // follow it, observed as deployed at the installation's generation
        const [installed] = await transaction
            .select({ generation: installation.table.generation })
            .from(installation.table)
            .where(eq(installation.table.id, ids.notes));
        const observed = {
            status: "true" as const,
            observedGeneration: present(installed, "the notes installation").generation,
            message: "",
            lastTransitionAt: now,
        };
        await transaction
            .update(installation.table)
            .set({
                conditions: {
                    Synced: { ...observed, reason: "ReconcileSuccess" },
                    Ready: { ...observed, reason: "Deployed" },
                },
            })
            .where(eq(installation.table.id, ids.notes));
        await transaction
            .update(installation.table)
            .set({
                targetRevisionId: revisionId,
                appliedRevisionId: revisionId,
                servedRevisionId: revisionId,
            })
            .where(eq(installation.table.id, ids.notes));
    });

    // deploy each release and run one instance of it on the fixture's machine
    for (const deployed of deployments) {
        const deploymentId = Identifier.create("deployment", environment);
        await database.transaction(async (transaction) => {
            await transaction.insert(deployment.table).values({
                ...record,
                id: deploymentId,
                installationId: ids.notes,
                packageId: ids.package,
                revisionId,
                output: "server",
                workload: "main",
                runtime: "bun",
                release: deployed.release,
                ...(deployed.since === undefined ? {} : { since: deployed.since }),
                description: workload,
                status: deployed.status,
                activatedAt: now,
            });
            await transaction.insert(instance.table).values({
                ...record,
                id: deployed.instanceId,
                deploymentId,
                machineId: space.machine.id,
                status: "running",
                observedAt: now,
                startedAt: now,
            });
        });
    }
}
