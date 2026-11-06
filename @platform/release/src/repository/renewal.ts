import type { schema } from "@destack/schema";
import { Metadata, MetadataKind, type Root, type Timestamp } from "@tufjs/models";
import type { Environment } from "@destack/environment";
import { TrustedRoot } from "@destack/update/publish";
import { TargetCustom } from "@destack/update/release";
import { MetadataDocument } from "./document.ts";

/** Maximum release metadata document size, in bytes. */
const METADATA_SIZE = 1024 * 1024;

/** How long reading one metadata document may take, in milliseconds. */
const READ_TIMEOUT_MILLISECONDS = 15_000;

/** Consecutive authenticated roots and the latest among them. */
export interface RootHistory {
    /** Consecutive roots authenticated from the embedded bootstrap document. */
    readonly history: Metadata<Root>[];
    /** Latest root authenticated by every intervening quorum. */
    readonly root: Metadata<Root>;
}

/** Consecutive authenticated roots and the latest among them. */
export const RootHistory = {
    /** Follow the complete authenticated root history and require an unexpired final root. */
    async read(url: URL, root: Metadata<Root>, environment: Environment): Promise<RootHistory> {
        // allow expired historical authorization only while following consecutive signed replacements
        TrustedRoot.authenticate(root);
        const history = [root];
        for (;;) {
            const name = `${root.signed.version + 1}.root.json`;
            const bytes = await readOptionalDocument(url, name, environment);
            if (!bytes) {
                break;
            }
            const next = Metadata.fromJSON(
                MetadataKind.Root,
                MetadataDocument.parse(bytes.toString()),
            );
            TrustedRoot.authenticate(next, root);
            history.push(next);
            root = next;
        }
        if (root.signed.isExpired(new Date(environment.clock.now()))) {
            throw new Error("release root must be renewed by its offline quorum");
        }

        return { history, root };
    },
};

/** Authenticated publication state used to renew expired freshness metadata. */
export class RepositoryRenewal {
    /** Consecutive roots authenticated from the embedded bootstrap document. */
    readonly history: Metadata<Root>[];
    /** Latest root authenticated by every intervening quorum. */
    readonly root: Metadata<Root>;
    /** Exact targets bytes retained by freshness renewal. */
    readonly targets: Buffer;
    /** Authenticated current timestamp, including its publication revision. */
    readonly timestamp: Metadata<Timestamp>;
    /** Revision greater than the previously signed timestamp and snapshot. */
    readonly revision: number;

    /** Retain authenticated documents and the next publication revision. */
    constructor(
        { history, root }: RootHistory,
        targets: Buffer,
        timestamp: Metadata<Timestamp>,
        revision: number,
    ) {
        // retain the authenticated publication and next revision together
        this.history = history;
        this.root = root;
        this.targets = targets;
        this.timestamp = timestamp;
        this.revision = revision;
    }

    /** Read the authenticated published state, absent before the first publication. */
    static async read(
        url: URL,
        root: Metadata<Root>,
        environment: Environment,
    ): Promise<RepositoryRenewal | undefined> {
        // follow consecutive quorum-authorized rotations before accepting online signatures
        const history = await RootHistory.read(url, root, environment);
        root = history.root;

        // authenticate the timestamp and its exact snapshot bytes, also after freshness expires
        const timestampBytes = await readOptionalDocument(url, "timestamp.json", environment);
        if (timestampBytes === undefined) {
            return undefined;
        }
        const timestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            MetadataDocument.parse(timestampBytes.toString()),
        );
        root.verifyDelegate("timestamp", timestamp);
        const snapshotReference = timestamp.signed.snapshotMeta;
        const snapshotBytes = await readDocument(
            url,
            `${snapshotReference.version}.snapshot.json`,
            environment,
        );
        snapshotReference.verify(snapshotBytes);
        const snapshot = Metadata.fromJSON(
            MetadataKind.Snapshot,
            MetadataDocument.parse(snapshotBytes.toString()),
        );
        root.verifyDelegate("snapshot", snapshot);
        if (snapshot.signed.version !== snapshotReference.version) {
            throw new Error("snapshot does not match its signed reference");
        }

        // retain exact target authorization for the publication service's storage comparison
        const reference = snapshot.signed.meta["targets.json"];
        if (!reference || Object.keys(snapshot.signed.meta).length !== 1) {
            throw new Error("release snapshot must reference only targets.json");
        }
        const targets = await readDocument(url, `${reference.version}.targets.json`, environment);
        reference.verify(targets);
        const authorization = Metadata.fromJSON(
            MetadataKind.Targets,
            MetadataDocument.parse(targets.toString()),
        );
        root.verifyDelegate("targets", authorization);
        if (authorization.signed.version !== reference.version) {
            throw new Error("targets do not match their signed reference");
        }

        // choose a monotonic safe integer revision for the next publication
        const revision = Math.max(
            environment.clock.now(),
            timestamp.signed.version + 1,
            snapshot.signed.version + 1,
        );
        if (!Number.isSafeInteger(revision)) {
            throw new Error("release metadata revision exceeds the supported range");
        }

        return new RepositoryRenewal(history, targets, timestamp, revision);
    }

    /** Report whether every published target was built from a commit. */
    isBuiltFrom(commit: string): boolean {
        return this.#customs().every((custom) => custom.commit === commit);
    }

    /** Read the version every published target reports, refusing targets of several versions. */
    get version(): string {
        // read the one version of the published targets
        const versions = new Set(this.#customs().map((custom) => custom.version));
        const [version] = versions;
        if (versions.size !== 1 || version === undefined) {
            throw new Error(
                `the published targets report ${versions.size} versions instead of one`,
            );
        }

        return version;
    }

    /** Read the release fields of each target the published targets authorize. */
    #customs(): schema.Infer<typeof TargetCustom>[] {
        const targets = Metadata.fromJSON(
            MetadataKind.Targets,
            MetadataDocument.parse(this.targets.toString()),
        );

        return Object.values(targets.signed.targets).map((target) =>
            TargetCustom.parse(target.custom),
        );
    }
}

/** Fetch a required metadata document. */
async function readDocument(url: URL, name: string, environment: Environment): Promise<Buffer> {
    // refuse a document the origin does not have
    const document = await readOptionalDocument(url, name, environment);
    if (document === undefined) {
        throw new Error(`cannot read release metadata ${name}: 404`);
    }

    return document;
}

/** Read bounded signed bytes from the selected origin, absent when the origin has none. */
async function readOptionalDocument(
    url: URL,
    name: string,
    environment: Environment,
): Promise<Buffer | undefined> {
    // read without redirects or intermediary caches and bound the request duration
    const response = await environment.network.fetch(
        new Request(new URL(`metadata/${name}`, url), {
            cache: "no-store",
            redirect: "error",
            signal: environment.clock.timeout(READ_TIMEOUT_MILLISECONDS),
        }),
    );
    if (response.status === 404) {
        await response.body?.cancel();

        return undefined;
    }
    if (!response.ok || !response.body) {
        await response.body?.cancel();
        throw new Error(`cannot read release metadata ${name}: ${response.status}`);
    }

    // consume bounded chunks and cancel oversized responses through iterator cleanup
    const buffers: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of response.body) {
        const bytes: unknown = chunk;
        if (!(bytes instanceof Uint8Array)) {
            throw new TypeError(`a chunk of release metadata ${name} is not bytes`);
        }
        size += bytes.length;
        if (size > METADATA_SIZE) {
            throw new Error(`oversized release metadata: ${name}`);
        }
        buffers.push(bytes);
    }

    return Buffer.concat(buffers);
}
