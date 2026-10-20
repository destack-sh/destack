import type { ObjectType } from "@destack/object";
import type { ObjectAccess } from "@destack/object/client";
import { schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { type Accessor, createSignal, onCleanup } from "solid-js";
import { useClient, useView } from "../page/view.ts";
import { ViewContext } from "../declare/context.ts";

/** Read and change object types of the view's space by key, as its space permissions grant. */
export function useSpace<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "space")).of(objects);
}

/** Read and change object types of the view's account by key, as its account permissions grant. */
export function useAccount<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "account")).of(objects);
}

/** Read and change object types of the person's home by key. */
export function useHome<const Objects extends Readonly<Record<string, ObjectType>>>(
    objects: Objects,
): ObjectAccess<Objects> {
    return useClient(ViewContext.scope(useView(), "home")).of(objects);
}

/** Follow a setting's value for the page's person, their device's value first. */
export function useSetting<Value extends schema.Schema>(
    declared: Setting<Value>,
): Accessor<schema.Infer<Value> | undefined> {
    // refuse a setting of another scope than the person's
    if (declared.definition.scope !== "user") {
        throw new TypeError(
            `useSetting follows the person's settings, not ${declared.definition.scope} ones: ${declared.definition.name}`,
        );
    }

    // select the page's space and installation in the person's scope
    const context = useView();
    const { database } = useClient(context.user.id);
    const [value, setValue] = createSignal<schema.Infer<Value>>();
    const selection = {
        scope: context.user.id,
        space: schema.identifier("space").parse(context.space),
        installation: schema.identifier("installation").parse(context.installation),
    };
    const at = {
        scope: context.user.id,
        ...(context.device === undefined
            ? {}
            : { device: schema.identifier("device").parse(context.device) }),
    };

    // resolve the setting now and after each change along the person's chain or on their device
    const stopping = new AbortController();
    onCleanup(() => stopping.abort());
    const steps = SettingValue.follow(database, [declared.reference], at, stopping.signal);
    void (async () => {
        for await (const { chain, values, device } of steps) {
            const resolved = declared.resolve(selection, values, chain, device).value;
            setValue(() => resolved);
        }
    })().catch((error: unknown) => {
        if (!stopping.signal.aborted) {
            reportError(error);
        }
    });

    return value;
}
