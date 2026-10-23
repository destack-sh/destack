import type { ObjectType } from "@destack/object";
import { setting, type SettingValue } from "../object/index.ts";
import {
    type Setting,
    type SettingDeclared,
    SettingPlacement,
    type SettingView,
} from "../setting/index.ts";

/** What a store serves its setting values with. */
export interface SettingOptions {
    /** Read the setting a value at a placement is a value of, in a view. */
    readonly declared: (written: SettingDeclared, view: SettingView) => Promise<Setting>;
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
    const placement = SettingPlacement.of(value);
    const found = await declared(
        { ...placement, packageId: value.packageId, name: value.name, release: value.release },
        view,
    );
    found.requireWrite(
        { ...placement, mode: value.mode, value: value.value, release: value.release },
        value.scope,
    );
}
