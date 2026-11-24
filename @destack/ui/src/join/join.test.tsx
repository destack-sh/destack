import type { Axis } from "../position/index.ts";
import * as style from "@destack/style";
import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { DivideContext, JoinContext, useJoin } from "./index.ts";

/** A place a control joins in. */
type Place = "outside" | "divided" | Axis;

/** Read the styles that join a control in the nearest group. */
function joinedStyles(): Record<Place, style.Styles> {
    const read: Partial<Record<Place, style.Styles>> = {};
    const Probe = (properties: { readonly name: Place }) => {
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
                <DivideContext value={true}>
                    <Probe name="divided" />
                </DivideContext>
            </JoinContext>
        </>
    ));

    return {
        outside: read.outside,
        divided: read.divided,
        horizontal: read.horizontal,
        vertical: read.vertical,
    };
}

/** Read the class names of styles. */
function classOf(styles: style.Styles): string {
    return style.attrs(styles).class ?? "";
}

test("join a control to its neighbours only inside a group, by the group's orientation", () => {
    // read the joining styles of each place twice
    const first = joinedStyles();
    const second = joinedStyles();

    // a control outside a group has its own corners, and each orientation joins its own way
    expect({
        outside: first.outside,
        isHorizontalJoined: classOf(first.horizontal) !== "",
        isVerticalJoined: classOf(first.vertical) !== "",
        isOrientationDistinct: classOf(first.horizontal) !== classOf(first.vertical),
        isDividedDistinct: classOf(first.divided) !== classOf(first.vertical),
        isStable: [
            first.horizontal === second.horizontal,
            first.vertical === second.vertical,
            first.divided === second.divided,
        ],
    }).toEqual({
        outside: null,
        isHorizontalJoined: true,
        isVerticalJoined: true,
        isOrientationDistinct: true,
        isDividedDistinct: true,
        isStable: [true, true, true],
    });
});
