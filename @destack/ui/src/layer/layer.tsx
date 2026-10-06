import type { JSX } from "@destack/view";
import { FieldContext } from "../field/control.ts";
import { JoinContext } from "../join/index.ts";

/** Render an overlay's content apart from the field and group its trigger sits in. */
export function TopLayer(properties: { readonly children: JSX.Element }): JSX.Element {
    return (
        <FieldContext value={null}>
            <JoinContext value={() => undefined}>{properties.children}</JoinContext>
        </FieldContext>
    );
}
