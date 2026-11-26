import { type ActionInput, Choice } from "@destack/object";
import type { JsonObject, JsonValue } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, size, space } from "@destack/theme/tokens.stylex";
import { createUniqueId, For, type JSX, omit, Show, useLocale } from "@destack/view";
import { LazyIcon } from "@destack/ui/icon";
import { defineComponent } from "@destack/view/declare";
import { renderPart } from "../part/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";
import { ValueField } from "../value-field/index.ts";

/** The styles of an inspector and its rows. */
const styles = style.create({
    inspector: {
        display: "grid",
        gridTemplateColumns: "minmax(4.5rem, 1fr) minmax(0, 2.5fr)",
        alignItems: "center",
        columnGap: space[3],
        rowGap: space[2],
    },
    label: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        minWidth: 0,
        minHeight: size[2],
        color: color.mutedForeground,
    },
    name: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        unicodeBidi: "plaintext",
    },
});

/** The properties of an inspector, the native element's attributes included. */
export interface InspectorProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** The StyleX styles applied after the inspector's styles. */
    readonly xstyle?: style.Styles;
    /** The inputs it edits, one row each, in order. */
    readonly inputs: readonly ActionInput[];
    /** The value of each input by its name, absent for one with none. */
    readonly values: JsonObject;
    /** Handle a person changing an input's value, undefined to give none. */
    readonly onValueChange: (name: string, value: JsonValue | undefined) => void;
}

/** Edit some values side by side, each input in a row of its name and its field. */
export const Inspector = defineComponent(
    "inspector",
    function Inspector(properties: InspectorProperties): JSX.Element {
        // write each choice of an input in the person's language
        const locale = useLocale();
        const rest = omit(properties, "inputs", "values", "onValueChange");

        return renderPart("div", "inspector", rest, [text.sm, styles.inspector], {
            role: "group",
            get children() {
                return (
                    <For each={properties.inputs}>
                        {(input) => {
                            // name each field by its label in the person's language, else by its name
                            const name = () =>
                                input.label === undefined ? input.name : locale.render(input.label);
                            const description = () =>
                                input.description === undefined
                                    ? undefined
                                    : locale.render(input.description);
                            const id = createUniqueId();
                            const label = (
                                attributes: JSX.HTMLAttributes<HTMLSpanElement> = {},
                            ) => (
                                <span {...attributes} {...style.attrs(styles.label)}>
                                    <Show when={input.icon}>
                                        {(icon) => <LazyIcon name={icon()} />}
                                    </Show>
                                    <span {...style.attrs(styles.name)}>{name()}</span>
                                </span>
                            );

                            return (
                                <>
                                    {/* Label, telling what the field does when pointed at */}
                                    <Show when={description()} fallback={label()}>
                                        {(said) => (
                                            <>
                                                <Tooltip>
                                                    <TooltipTrigger render={label} />
                                                    <TooltipContent>{said()}</TooltipContent>
                                                </Tooltip>
                                                <span id={id} hidden>
                                                    {said()}
                                                </span>
                                            </>
                                        )}
                                    </Show>

                                    {/* Field */}
                                    <ValueField
                                        aria-label={name()}
                                        aria-describedby={
                                            description() === undefined ? undefined : id
                                        }
                                        type={input.value}
                                        choices={
                                            input.value.kind === "choice"
                                                ? input.value.choices.map((each) =>
                                                      Choice.of(each, locale),
                                                  )
                                                : undefined
                                        }
                                        value={properties.values[input.name]}
                                        onValueChange={(value) =>
                                            properties.onValueChange(input.name, value)
                                        }
                                        required={input.presence === "required"}
                                    />
                                </>
                            );
                        }}
                    </For>
                );
            },
        });
    },
    {},
);
