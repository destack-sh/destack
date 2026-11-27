import type { JSX } from "@destack/view";
import { FieldActionsContext, FieldContext } from "../field/control.ts";
import { JoinContext } from "../join/index.ts";

/** Render an overlay's content apart from the field, fieldset and group its trigger sits in. */
export function Overlay(properties: { readonly children: JSX.Element }): JSX.Element {
    return (
        <FieldContext value={null}>
            <FieldActionsContext value={null}>
                <JoinContext value={() => undefined}>{properties.children}</JoinContext>
            </FieldActionsContext>
        </FieldContext>
    );
}
