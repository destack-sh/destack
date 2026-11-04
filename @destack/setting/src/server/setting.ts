import type { MethodContext, ObjectType } from "@destack/object";
import type { DeclarationReference } from "@destack/package/declare";
import { schema } from "@destack/schema";
import { setting, SettingValue } from "../object/index.ts";
import { Setting, SETTING_KIND, SettingPlacement } from "../setting/index.ts";

/** What a store serves its setting values with. */
export interface SettingOptions {
    /** The types whose declarations a stack applies before its settings. */
    readonly after: readonly ObjectType[];
}

/** Implement setting values, checked in their transaction against their declarations. */
export function implementSetting(options: SettingOptions) {
    const { after } = options;

    return setting.implement({
        methods: {
            create: {
                handle: async (context, next) => {
                    // check the value against its setting's declaration in the call's view
                    await requireDeclared({ ...context.input, scope: context.scope }, context);

                    return next();
                },
            },
            update: {
                handle: async (context, next) => {
                    // check the changed value against its setting's declaration in the call's view
                    const value = { ...context.target, ...context.input, scope: context.scope };
                    await requireDeclared(value, context);

                    return next();
                },
            },
            resolve: async (call) => {
                // read the setting of the release the call's scope runs
                const { packageId, name, selection, device } = call.input;
                const found = await findSetting(call, { packageId, name });

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
                // stamp the declared value with the release its scope runs, waiting for its package to install
                const reader = await plan.release(desired.packageId);
                if (reader === undefined) {
                    return plan.wait(
                        `setting ${desired.name} waits for ${desired.packageId} to install`,
                    );
                }
                const found = Setting.find(await Setting.list(reader), desired);

                return { ...desired, release: found.package.version };
            },
        },
    });
}

/** Require a value to match its setting's declaration at its placement, in the release it names. */
async function requireDeclared(
    value: Parameters<typeof SettingPlacement.of>[0] &
        Pick<SettingValue, "packageId" | "name" | "mode" | "value" | "release">,
    context: Pick<MethodContext, "declarations">,
): Promise<void> {
    // read the value's setting, refusing one its release does not declare
    const placement = SettingPlacement.of(value);
    const found = await findSetting(context, value, value.release);
    found.requireWrite(
        { ...placement, mode: value.mode, value: value.value, release: value.release },
        value.scope,
    );
}

/** Find a setting in the release of its package the scope runs, or the given version. */
async function findSetting(
    context: Pick<MethodContext, "declarations">,
    reference: DeclarationReference,
    version?: string,
): Promise<Setting> {
    const declarations = await context.declarations(SETTING_KIND, reference.packageId, version);

    return Setting.find(
        declarations.map((entry) => Setting.of(entry.description)),
        reference,
    );
}
