import { implement, type ServiceContext } from "@destack/service/server";
import { ServiceError } from "@destack/service/error";
import { Page } from "@destack/service/page";
import { CALLER_LIFETIME_MS } from "@destack/service/authentication";
import { schema } from "@destack/schema";
import type { PackageId } from "@destack/package";
import {
    type Setting,
    type SettingReference,
    type SettingSelection,
    type SettingTarget,
    resolveSetting,
} from "../setting/index.ts";
import { settingService, type SettingQuery } from "../service/index.ts";
import { describeSetting } from "../inspect/index.ts";
import { readAssignments, type SettingStore } from "../database/index.ts";
import type { SettingServerOptions } from "./server.ts";
import { reportSettingError } from "./error.ts";

/** Connect declaration discovery and effective-value resolution to the verified host. */
export function settingRouter(store: SettingStore, options: SettingServerOptions) {
    const implementation = implement(settingService.router).$context<ServiceContext>();

    return {
        list: implementation.setting.list.handler(async ({ input, context }) => {
            // bind the page to the consumer and sort its declarations by identity
            const declarations = await options.declarations(
                context,
                input.packageId,
                input.installationId,
            );
            const page = new Page(
                input,
                [
                    "setting",
                    input.packageId,
                    ...(input.installationId === undefined ? [] : [input.installationId]),
                ],
                schema.string(),
            );
            const descriptions = declarations.map(describeSetting).sort((left, right) => {
                const first = key(left.package.id, left.name);
                const second = key(right.package.id, right.name);

                return first < second ? -1 : first > second ? 1 : 0;
            });
            const selected = descriptions.filter(
                (description) =>
                    !page.after || key(description.package.id, description.name) > page.after,
            );

            return page.result(selected, (description) =>
                key(description.package.id, description.name),
            );
        }),
        resolve: implementation.setting.resolve.handler(({ input, context }) =>
            resolveSettings(input, store, options, context),
        ),
        watch: implementation.setting.watch.handler(async function* ({ input, context }) {
            // combine the verified caller lifetime with setting policy expiry and shutdown
            const lifetime = new AbortController();
            const cancellation = AbortSignal.any([
                lifetime.signal,
                context.signal,
                ...(options.signal ? [options.signal] : []),
            ]);
            const authentication = context.requireCaller().authentication;
            const expiresAt = Math.min(
                authentication.expiresAt,
                authentication.verifiedAt + CALLER_LIFETIME_MS,
            );
            let timer: ReturnType<typeof setTimeout> | undefined;
            let previous: string | undefined;
            try {
                for await (const _notification of store.watch(input.settings, cancellation)) {
                    context.requireCaller();
                    const value = await resolveSettings(input, store, options, context);

                    // discard a result if its request expired while reading storage
                    if (cancellation.aborted) {
                        return;
                    }
                    context.requireCaller();

                    // renew only while both identity and the complete policy selection remain valid
                    clearTimeout(timer);
                    const deadline = value.reduce(
                        (deadline, resolution) =>
                            Math.min(deadline, resolution.validUntil ?? expiresAt),
                        expiresAt,
                    );
                    timer = setTimeout(() => lifetime.abort(), Math.max(0, deadline - Date.now()));
                    const serialized = JSON.stringify(value);
                    if (serialized !== previous) {
                        previous = serialized;
                        yield value;
                    }
                }
            } finally {
                clearTimeout(timer);
                lifetime.abort();
            }
        }),
    };
}

/** Resolve only declarations selected by the actual hosting release. */
export async function declaration(
    reference: SettingReference,
    context: ServiceContext,
    options: SettingServerOptions,
    packageId: PackageId,
    target?: SettingSelection,
): Promise<Setting> {
    // find the declaration in the consumer release
    const installationId =
        target && "location" in target ? target.location?.installationId : undefined;
    const declarations = await options.declarations(context, packageId, installationId);
    const setting = declarations.find(
        (candidate) =>
            candidate.reference.packageId === reference.packageId &&
            candidate.reference.name === reference.name,
    );
    if (!setting) {
        throw new ServiceError("NOT_FOUND", {
            message: "setting is not declared by the selected consumer",
        });
    }

    return setting;
}

/** Resolve a batch using the verified request context without opening a subscription. */
export async function resolveSettings(
    input: schema.Infer<typeof SettingQuery>,
    store: SettingStore,
    options: SettingServerOptions,
    context: ServiceContext,
) {
    // authorize the caller to read the target
    context.requireCaller();
    const { target, packageId } = input;
    await options.authorize(context, target, "read", packageId);

    // load the consumer release once and deduplicate requested identities in request order
    const installationId = "location" in target ? target.location?.installationId : undefined;
    const declarations = await options.declarations(context, packageId, installationId);
    const available = new Map(
        declarations.map((setting) => [
            key(setting.reference.packageId, setting.reference.name),
            setting,
        ]),
    );
    const selected = new Map<string, Setting>();
    for (const reference of input.settings) {
        const identity = key(reference.packageId, reference.name);
        const setting = available.get(identity);
        if (!setting) {
            throw new ServiceError("NOT_FOUND", {
                message: "setting is not declared by the selected consumer",
            });
        }
        selected.set(identity, setting);
    }

    // read all selected assignments and policy under one consistent transaction
    const references = [...selected.values()].map((setting) => setting.reference);
    const targets = assignmentTargets(target);

    return store.database
        .transaction(
            async (transaction) => {
                // read assignments and policies at one instant
                const records = await readAssignments(transaction, references, targets);
                const policy = await options.policies(context, references, target, transaction);
                const now = Date.now();

                // group stored values once before resolving each declaration
                const assignments = Map.groupBy(records, (record) =>
                    key(record.setting.packageId, record.setting.name),
                );
                const policies = Map.groupBy(policy.policies, (record) =>
                    key(record.setting.packageId, record.setting.name),
                );

                return [...selected].map(([identity, setting]) => {
                    const result = resolveSetting(
                        setting,
                        {
                            target,
                            assignments: assignments.get(identity) ?? [],
                            policies: policies.get(identity) ?? [],
                            validUntil: policy.validUntil,
                        },
                        now,
                    );

                    return { ...result, value: schema.json().parse(result.value) };
                });
            },
            { isolationLevel: "repeatable read" },
        )
        .catch(reportSettingError);
}

/** Enumerate exact indexed targets that may contribute to this runtime selection. */
function assignmentTargets(target: SettingSelection): SettingTarget[] {
    const targets: SettingTarget[] = [];

    // enumerate the fixed declaration scopes, retaining combinations independently
    if (target.kind === "user" && target.user !== null) {
        const packages = target.packageId ? [undefined, target.packageId] : [undefined];
        const locations = target.location
            ? [
                  undefined,
                  { spaceId: target.location.spaceId },
                  ...(target.location.installationId ? [target.location] : []),
              ]
            : [undefined];
        const devices =
            "deviceId" in target && target.deviceId ? [undefined, target.deviceId] : [undefined];
        for (const consumer of packages) {
            for (const location of locations) {
                for (const deviceId of devices) {
                    targets.push({
                        kind: "user",
                        user: target.user,
                        ...(consumer ? { packageId: consumer } : {}),
                        ...(location ? { location } : {}),
                        ...(deviceId ? { deviceId } : {}),
                    });
                }
            }
        }
    } else if (target.kind === "space") {
        targets.push({ kind: "space", location: { spaceId: target.location.spaceId } });
        if (target.location.installationId) {
            targets.push(target);
        }
    } else if (target.kind === "host") {
        targets.push(target);
    }

    return targets;
}

/** Order declaration names within stable package identities. */
function key(packageId: string, name: string): string {
    return `${packageId}/${name}`;
}
