import type { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { createRequestId } from "@destack/service/request";
import { Watch } from "@destack/service/watch";
import type { SettingContext, SettingBatch, SettingResult } from "../setting/context.ts";
import type { Setting } from "../setting/setting.ts";
import type { SettingTarget } from "../setting/target.ts";
import type { SettingResolution } from "../setting/resolution.ts";
import type { createSettingClient } from "./client.ts";
import { SettingError } from "../error/index.ts";

/** Typed client bound to the verified user and consuming package selected by its host. */
export class SettingClient implements SettingContext {
    /** Authenticated service connection. */
    readonly client: ReturnType<typeof createSettingClient>;
    /** Consumer whose release the host verifies. */
    readonly packageId: PackageId;
    /** Personal or runtime selection presented for host verification. */
    readonly target: SettingTarget;
    /** Request cancellation when this client belongs to a backend invocation. */
    readonly signal?: AbortSignal;

    /** Bind a caller-selected target without granting authority to it. */
    constructor(
        client: ReturnType<typeof createSettingClient>,
        packageId: PackageId,
        target: SettingTarget,
        signal?: AbortSignal,
    ) {
        this.client = client;
        this.packageId = packageId;
        this.target = target;
        this.signal = signal;
    }

    /** Resolve named declarations in one request using their imported schemas. */
    async resolve<Batch extends SettingBatch>(settings: Batch): Promise<SettingResult<Batch>> {
        const result = await this.client.setting.resolve(
            {
                packageId: this.packageId,
                settings: references(settings),
                target: this.target,
            },
            { signal: this.signal },
        );

        return decode(settings, result);
    }

    /** Observe authorized values until cancellation or a reported authority failure. */
    async *watch<Batch extends SettingBatch>(
        settings: Batch,
        signal: AbortSignal,
    ): AsyncIterable<SettingResult<Batch>> {
        const cancellation = this.signal ? AbortSignal.any([signal, this.signal]) : signal;
        const query = {
            packageId: this.packageId,
            settings: references(settings),
            target: this.target,
        };

        // reconnect a normally completed subscription through fresh server authentication
        const stream = Watch.observe(
            (signal) => this.client.setting.watch(query, { signal }),
            cancellation,
        );
        for await (const value of stream) {
            yield decode(settings, value);
        }
    }

    /** Replace an exact target only if its observed revision remains current. */
    async set<Value extends schema.Schema>(
        setting: Setting<Value>,
        value: schema.Infer<Value>,
        expectedRevision: string | null,
        target: SettingTarget = this.target,
    ) {
        setting.assertTarget(target);

        return this.client.assignment.set(
            {
                packageId: this.packageId,
                requestId: createRequestId(),
                setting: setting.reference,
                target,
                expectedRevision,
                value: schema.json().parse(setting.declaration.schema.parse(value)),
            },
            { signal: this.signal },
        );
    }

    /** Reset an exact target at the revision last observed by the editor. */
    reset(setting: Setting, expectedRevision: string | null, target: SettingTarget = this.target) {
        return this.client.assignment.reset(
            {
                packageId: this.packageId,
                requestId: createRequestId(),
                setting: setting.reference,
                target,
                expectedRevision,
            },
            { signal: this.signal },
        );
    }
}

/** Deduplicate declarations while retaining caller-defined result names. */
function references(settings: SettingBatch) {
    const unique = new Map(
        Object.values(settings).map((setting) => [
            `${setting.reference.packageId}/${setting.reference.name}`,
            setting.reference,
        ]),
    );

    return [...unique.values()];
}

/** Require a complete batch and validate every value against its imported declaration. */
function decode<Batch extends SettingBatch>(
    settings: Batch,
    results: readonly SettingResolution[],
): SettingResult<Batch> {
    const expected = references(settings);
    const values = new Map(
        results.map((result) => [`${result.setting.packageId}/${result.setting.name}`, result]),
    );
    if (values.size !== expected.length || results.length !== expected.length) {
        throw new SettingError(
            "INVALID_VALUE",
            "setting service returned an incomplete or duplicate batch",
        );
    }

    // retain the caller's property names without duplicating transport requests
    return Object.fromEntries(
        Object.entries(settings).map(([name, setting]) => {
            const result = values.get(`${setting.reference.packageId}/${setting.reference.name}`);
            if (!result) {
                throw new SettingError(
                    "INVALID_VALUE",
                    "setting service omitted a requested declaration",
                );
            }

            return [name, { ...result, value: setting.declaration.schema.parse(result.value) }];
        }),
    ) as SettingResult<Batch>;
}
