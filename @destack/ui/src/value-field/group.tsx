import * as style from "@destack/style";
import type { JSX } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { FieldContext, useFieldControl } from "../field/control.ts";

/** The properties of the group of a value's parts. */
export interface ValueGroupProperties {
    /** What the value is called, where no field's label says so. */
    readonly "aria-label"?: string | undefined;
    /** The id of the element describing the value, none without one. */
    readonly "aria-describedby"?: string | undefined;
    /** The group's look, such as a row or a grid. */
    readonly xstyle?: style.Styles;
    /** The parts, each named on its own. */
    readonly children: JSX.Element;
}

/** Render the group of a value's parts, labelled by its field, each part named on its own. */
export function ValueGroup(properties: ValueGroupProperties): JSX.Element {
    // label the group by the nearest field, as a field labels a group of controls
    const field = useFieldControl();
    field?.group();

    return (
        <div
            role="group"
            {...field?.groupAttributes()}
            aria-label={properties["aria-label"]}
            aria-describedby={
                properties["aria-describedby"] ?? field?.groupAttributes()["aria-describedby"]
            }
            {...style.attrs(properties.xstyle)}
        >
            {/* Parts, outside the field so none takes its id */}
            <FieldContext value={null}>{properties.children}</FieldContext>
        </div>
    );
}

/** Render the button that opens a value's picker, the control its field labels and describes. */
export function ValueTrigger(properties: ButtonProperties): JSX.Element {
    // take the field's id and description, beside the button's own
    const field = useFieldControl();

    return (
        <Button
            variant="outline"
            {...properties}
            id={field?.id ?? properties.id}
            aria-describedby={
                properties["aria-describedby"] ?? field?.attributes()["aria-describedby"]
            }
        />
    );
}
