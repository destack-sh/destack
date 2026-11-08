import type { RowImage, Select, Snapshot } from "@destack/db";
import type { MethodContext } from "@destack/object";
import { Binding } from "@destack/stack/object";
import { VaultError } from "../error/error.ts";
import { secret, secretVersion } from "../object/secret.ts";
import { MAX_VALUE_BYTES, type SecretReading, type SecretValue } from "@destack/resource";

/** A persisted secret version. */
export type SecretVersion = Select<typeof secretVersion.table>;

/** Find, check and read secret versions. */
export const SecretVersion = {
    /** Read a secret's selected version, its current one by default, as a caller may. */
    async read(
        target: RowImage<typeof secret.table>,
        version: number | undefined,
        context: MethodContext,
    ): Promise<SecretReading> {
        // refuse a secret without a selected version
        const number = version ?? target.currentVersion;
        if (number === null) {
            throw VaultError.of("CURRENT_VERSION_MISSING", "secret has no current version");
        }

        // let software bound to the secret, its vault or the selected version read it
        const selected = await SecretVersion.find(context, target.id, number);
        const consumes = await context.check(
            secret.permission("consume"),
            secret.reference(context.scope, target.id),
        );
        const isConsumer =
            consumes.isAllowed ||
            (selected !== undefined &&
                (
                    await context.check(
                        secretVersion.permission("read"),
                        secretVersion.reference(context.scope, selected.id),
                    )
                ).isAllowed);

        // require anyone else to reveal through the secret, challenging for a step-up
        if (!isConsumer) {
            await context.require(
                secret.permission("reveal"),
                secret.reference(context.scope, target.id),
            );
        }
        // answer a missing version only to those revealing the secret
        if (selected === undefined) {
            throw VaultError.of("VERSION_NOT_FOUND", "secret version not found");
        }

        return {
            version: selected.number,
            value: SecretVersion.requireReadable(selected, context.now),
        };
    },

    /** Read a secret's selected version and the earlier ones still retiring, newest first. */
    async versions(
        target: RowImage<typeof secret.table>,
        version: number | undefined,
        context: MethodContext,
    ): Promise<[SecretReading, ...SecretReading[]]> {
        // read the selected version as the caller may read it
        const selected = await SecretVersion.read(target, version, context);

        // read the earlier versions still retiring, which a caller reading the secret or each version reads
        const earlier = await context.reveal(secretVersion).findMany({
            where: { parentId: target.id, number: { lt: selected.version } },
        });
        const retiring = earlier.filter((row) => SecretVersion.isRetiring(row, context.now));
        const isWhole = (
            await context.check(
                secret.permission("read"),
                secret.reference(context.scope, target.id),
            )
        ).isAllowed;
        const readable = isWhole
            ? retiring.map((row) => [row])
            : await Promise.all(
                  retiring.map(async (row) => {
                      const check = await context.check(
                          secretVersion.permission("read"),
                          secretVersion.reference(context.scope, row.id),
                      );

                      return check.isAllowed ? [row] : [];
                  }),
              );

        // answer them newest first
        const readings = readable
            .flat()
            .toSorted((left, right) => right.number - left.number)
            .map((row) => ({
                version: row.number,
                value: SecretVersion.requireReadable(row, context.now),
            }));

        return [selected, ...readings];
    },

    /** Report whether a version is retiring: intact, enabled and readable until a later expiry. */
    isRetiring(version: SecretVersion, now: number): boolean {
        return (
            version.content !== null &&
            version.state === "enabled" &&
            version.expiresAt !== null &&
            version.expiresAt > now
        );
    },

    /** Refuse destroying a version already destroyed or one a live deployment runs with. */
    async requireReleased(reader: Pick<Snapshot, "read">, version: SecretVersion): Promise<void> {
        SecretVersion.requireIntact(version);
        await Binding.requireReleased(reader, {
            type: secret.name,
            scope: version.scope,
            id: version.parentId,
            version: version.number,
        });
    },

    /** The fields of a version whose value is destroyed. */
    erasure: { state: "destroyed", content: null } as const,

    /** Find one numbered version of a secret, absent when it has none. */
    async find(
        context: Pick<MethodContext, "reveal">,
        secretId: Select<typeof secret.table>["id"],
        version: number,
    ): Promise<SecretVersion | undefined> {
        const [row] = await context.reveal(secretVersion).findMany({
            where: {
                parentId: secretId,
                number: version,
            },
        });

        return row;
    },

    /** Refuse changing a version with a destroyed value. */
    requireIntact(version: SecretVersion): void {
        if (version.state === "destroyed") {
            throw VaultError.of("VERSION_DESTROYED", "secret version is destroyed");
        }
    },

    /** Refuse reading a destroyed, disabled, expired or foreign version, answering its value. */
    requireReadable(version: SecretVersion, now: number): SecretValue {
        const { content } = version;
        // refuse a value copied from another version or secret
        if (
            content !== null &&
            (content.version !== version.id || content.secret !== version.parentId)
        ) {
            throw VaultError.of(
                "VERSION_MISMATCH",
                `secret version ${version.id} contains the value of another version`,
            );
        }
        // refuse a destroyed, disabled or expired version
        else if (
            content === null ||
            version.state === "disabled" ||
            (version.expiresAt !== null && version.expiresAt <= now)
        ) {
            throw VaultError.of("VERSION_UNAVAILABLE", "secret version is unavailable");
        }

        return content.value;
    },

    /** Refuse a value beyond the decoded byte limit. */
    requireSize(value: SecretValue): void {
        // measure the decoded bytes, wiping them once counted
        const bytes =
            value.encoding === "text"
                ? new TextEncoder().encode(value.value)
                : Uint8Array.fromBase64(value.value);
        const size = bytes.byteLength;
        bytes.fill(0);
        if (size > MAX_VALUE_BYTES) {
            throw VaultError.of("VALUE_TOO_LARGE", `secret value exceeds ${MAX_VALUE_BYTES} bytes`);
        }
    },
};
