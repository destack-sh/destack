import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { FieldContext, FieldControl, useFieldControl } from "../field/control.ts";
import { JoinContext, useJoin } from "../join/index.ts";
import { TopLayer } from "./index.ts";

test("render an overlay's content apart from the field and the joining group around its trigger", () => {
    // read the field and the joining styles beside the trigger and inside the top layer
    const read: Record<string, readonly [boolean, unknown]> = {};
    const Probe = (properties: { readonly name: string }) => {
        read[properties.name] = [useFieldControl() !== null, useJoin()()];

        return undefined;
    };
    draw(() => (
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
