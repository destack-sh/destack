import * as style from "@destack/style";
import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { JoinContext, type JoinOrientation, useJoin } from "./index.ts";

/** Read the styles that join a control in the nearest group, outside any group and in each orientation. */
function joinedStyles(): Record<"outside" | JoinOrientation, style.Styles> {
    const read: Partial<Record<"outside" | JoinOrientation, style.Styles>> = {};
    const Probe = (properties: { readonly name: "outside" | JoinOrientation }) => {
        read[properties.name] = useJoin()();

        return undefined;
    };
    render(() => (
        <>
            <Probe name="outside" />
            <JoinContext value={() => "horizontal"}>
                <Probe name="horizontal" />
            </JoinContext>
            <JoinContext value={() => "vertical"}>
                <Probe name="vertical" />
            </JoinContext>
        </>
    ));

    return { outside: read.outside, horizontal: read.horizontal, vertical: read.vertical };
}

/** Read the class names of styles. */
function classOf(styles: style.Styles): string {
    return style.attrs(styles).class ?? "";
}

test("join a control to its neighbours only inside a group, by the group's orientation", () => {
    // read the joining styles of each place twice
    const first = joinedStyles();
    const second = joinedStyles();

    // a control outside a group keeps its corners, and each orientation joins its own way
    expect({
        outside: first.outside,
        isHorizontalJoined: classOf(first.horizontal) !== "",
        isVerticalJoined: classOf(first.vertical) !== "",
        isOrientationDistinct: classOf(first.horizontal) !== classOf(first.vertical),
        isStable: [first.horizontal === second.horizontal, first.vertical === second.vertical],
    }).toEqual({
        outside: null,
        isHorizontalJoined: true,
        isVerticalJoined: true,
        isOrientationDistinct: true,
        isStable: [true, true],
    });
});
