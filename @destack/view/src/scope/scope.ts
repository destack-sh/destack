import { schema } from "@destack/schema";
import type { Setting } from "@destack/setting";
import { SettingValue } from "@destack/setting/object";
import { type Accessor, createSignal, onCleanup } from "solid-js";
import { useView } from "../page/context.ts";
import { useClient } from "../page/view.ts";
import { ViewError } from "../error/error.ts";

/** Follow a setting's value for the page's person, their device's value first. */
export function useSetting<Value extends schema.Schema>(
    declared: Setting<Value>,
): Accessor<schema.Infer<Value> | undefined> {
    // refuse a setting of another scope than the person's
    if (declared.definition.scope !== "user") {
        throw ViewError.of(
            "SETTING_SCOPE_INVALID",
            `setting '${declared.definition.name}' is invalid: useSetting only reads the person's settings, not '${declared.definition.scope}' settings`,
        );
    }

    // select the page's space and installation in the person's scope
    const context = useView();
    const node = useClient(context.user.id);
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
    const steps = SettingValue.follow(node, [declared.reference], at, stopping.signal);
    void (async () => {
        for await (const { chain, values, device } of steps) {
            const resolved = declared.resolve(selection, values, chain, device).value;
            setValue(() => resolved);
        }
    })().catch((error: unknown) => {
        if (!stopping.signal.aborted) {
            globalThis.reportError(error);
        }
    });

    return value;
}
