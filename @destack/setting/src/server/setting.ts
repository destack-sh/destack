import type { ObjectType } from "@destack/object";
import { schema } from "@destack/schema";
import { SettingError } from "../error/index.ts";
import { setting, SettingValue } from "../object/index.ts";
import {
    type Setting,
    type SettingDeclared,
    SettingPlacement,
    type SettingView,
} from "../setting/index.ts";

/** What a store serves its setting values with. */
export interface SettingOptions {
    /** Read the setting of a value at a placement, none while its package is uninstalled. */
    readonly declared: (
        written: SettingDeclared,
        view: SettingView,
    ) => Promise<Setting | undefined>;
    /** The types whose declarations a stack applies before its settings. */
    readonly after: readonly ObjectType[];
}

/** Implement setting values, checked in their transaction against their declarations. */
export function implementSetting(options: SettingOptions) {
    const { declared, after } = options;

    return setting.implement({
        methods: {
            create: {
                handle: async (context, next) => {
                    // check the value against its setting's declaration in the call's view
                    await requireDeclared(
                        { ...context.input, scope: context.scope },
                        declared,
                        context,
                    );

                    return next();
                },
            },
            update: {
                handle: async (context, next) => {
                    // check the changed value against its setting's declaration in the call's view
                    const value = { ...context.target, ...context.input, scope: context.scope };
                    await requireDeclared(value, declared, context);

                    return next();
                },
            },
            resolve: async (call) => {
                // read the setting the call's scope installs, refusing one of an uninstalled package
                const { packageId, name, selection, device } = call.input;
                const found = await declared(
                    {
                        ...SettingPlacement.of({ ...selection, scope: call.scope }),
                        packageId,
                        name,
                    },
                    call,
                );
                if (found === undefined) {
                    throw SettingError.of(
                        "SETTING_UNDECLARED",
                        `${packageId} declares no setting ${name} here: it is not installed`,
                    );
                }

                // resolve it from the values the selected scope's chain places and the device's
                const chain =
                    selection.scope === null ? [] : await SettingValue.chain(call, selection.scope);
                const values = await call.read(setting.table).findMany({
                    where: { AND: [found.condition(selection), { scope: { in: chain } }] },
                });

                // read the resolved value as the JSON it stores
                const resolution = found.resolve(selection, values, chain, device);

                return { ...resolution, value: schema.json().parse(resolution.value) };
            },
        },
        declaration: {
            after,
            resolve: async (_name, desired, plan) => {
                // stamp the declared value with the release of the build declaring its setting in the apply's view
                const placement = SettingPlacement.of({ ...desired, scope: plan.stack.scope });
                const found = await declared(
                    { ...placement, packageId: desired.packageId, name: desired.name },
                    plan.transaction.view(),
                );
                if (found === undefined) {
                    return plan.wait(
                        `setting ${desired.name} waits for ${desired.packageId} to install`,
                    );
                }

                return { ...desired, release: found.package.version };
            },
        },
    });
}

/** Require a value to match its setting's declaration at its placement. */
async function requireDeclared(
    value: Parameters<typeof SettingPlacement.of>[0] &
        Pick<SettingValue, "packageId" | "name" | "mode" | "value" | "release">,
    declared: SettingOptions["declared"],
    view: SettingView,
): Promise<void> {
    // read the value's setting at its placement, refusing one of an uninstalled package
    const placement = SettingPlacement.of(value);
    const found = await declared(
        { ...placement, packageId: value.packageId, name: value.name, release: value.release },
        view,
    );
    if (found === undefined) {
        throw SettingError.of(
            "SETTING_UNDECLARED",
            `${value.packageId} declares no setting ${value.name} here: it is not installed`,
        );
    }
    found.requireWrite(
        { ...placement, mode: value.mode, value: value.value, release: value.release },
        value.scope,
    );
}
