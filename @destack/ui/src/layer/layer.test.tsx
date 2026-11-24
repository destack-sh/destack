import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { FieldContext, FieldControl, useFieldControl } from "../field/control.ts";
import { JoinContext, useJoin } from "../join/index.ts";
import { OpenChange, TopLayer } from "./index.ts";

test("render an overlay's content apart from the field and the joining group around its trigger", () => {
    // read the field and the joining styles beside the trigger and inside the top layer
    const read: Record<string, readonly [boolean, unknown]> = {};
    const Probe = (properties: { readonly name: string }) => {
        read[properties.name] = [useFieldControl() !== null, useJoin()()];

        return undefined;
    };
    render(() => (
        <FieldContext
            value={
                new FieldControl(
                    () => false,
                    () => false,
                )
            }
        >
            <JoinContext value={() => "horizontal"}>
                <Probe name="trigger" />
                <TopLayer>
                    <Probe name="content" />
                </TopLayer>
            </JoinContext>
        </FieldContext>
    ));

    // the trigger belongs to the field and the group, the content to neither
    expect({
        trigger: [read["trigger"]?.[0], read["trigger"]?.[1] === null],
        content: read["content"],
    }).toEqual({ trigger: [true, false], content: [false, null] });
});

test("tell a change the keyboard asks for from one a pointer does", () => {
    const changes = [
        new OpenChange("escape-key"),
        new OpenChange("input"),
        new OpenChange("trigger-focus"),
        new OpenChange("trigger-press", new KeyboardEvent("keydown", { key: "ArrowDown" })),
        new OpenChange("trigger-press", new MouseEvent("click", { detail: 0 })),
        new OpenChange("trigger-press", new MouseEvent("click", { detail: 1 })),
        new OpenChange("outside-press"),
    ];

    expect(changes.map((change) => change.isByKeyboard)).toEqual([
        true,
        true,
        true,
        true,
        true,
        false,
        false,
    ]);
});
