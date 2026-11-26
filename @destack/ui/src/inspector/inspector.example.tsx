import { t } from "@destack/locale";
import { ActionInput } from "@destack/object";
import { defineExample } from "@destack/package/declare";
import type { JsonObject, JsonValue } from "@destack/schema";
import { createSignal, type JSX } from "@destack/view";
import { Button } from "../button/index.ts";
import { Inspector } from "./inspector.tsx";

/** A button's properties, each in a row of its name and its field. */
export const inspectorDefault = defineExample({
    of: Inspector,
    name: "default",
    description: t`A button's properties, each in a row of its name and its field.`,
    layout: "padded",
    render: () => <ButtonInspector />,
});

/** Render an inspector of a button's properties beside the button it changes. */
function ButtonInspector(): JSX.Element {
    // read the button's declared properties as inputs, and store what a person gives
    const inputs = Object.entries(Button.properties).flatMap(([name, definition]) =>
        definition === undefined ? [] : [ActionInput.of(name, definition)],
    );
    const [values, setValues] = createSignal<JsonObject>({
        variant: "default",
        size: "default",
        loading: false,
    });
    const change = (name: string, value: JsonValue | undefined) =>
        setValues((current) => ({ ...current, [name]: value ?? null }));

    return (
        <Inspector aria-label="Button" inputs={inputs} values={values()} onValueChange={change} />
    );
}
