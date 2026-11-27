import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { space } from "@destack/theme/tokens.stylex";
import type { JSX } from "@destack/view";
import { FieldSizeContext } from "../field/field.tsx";
import { renderPart } from "../part/index.ts";

/** The styles of an inspector. */
const styles = style.create({
    inspector: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        rowGap: space[3],
        minWidth: 0,
    },
});

/** The properties of an inspector, the native element's attributes included. */
export interface InspectorProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The StyleX styles applied after the inspector's styles. */
    readonly xstyle?: style.Styles;
    /** Its fields and sets of fields. */
    readonly children?: JSX.Element;
}

/** Render an inspector: a dense panel of the fields that edit a thing's values. */
export function Inspector(properties: InspectorProperties): JSX.Element {
    return (
        <FieldSizeContext value="sm">
            {renderPart("div", "inspector", properties, [text.sm, styles.inspector], {
                role: "group",
            })}
        </FieldSizeContext>
    );
}
