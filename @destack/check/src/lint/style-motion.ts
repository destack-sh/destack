import type { ESTree, Rule } from "@oxlint/plugins";
import { eachCreatedObject, isCreateCall, keyOf } from "./style.ts";

/** What moves in a style, with the curve it moves on. */
const CURVES: readonly (readonly [string, string, string])[] = [
    ["transitionProperty", "transitionDuration", "transitionTimingFunction"],
    ["animationName", "animationDuration", "animationTimingFunction"],
];

/** Move what moves on a curve of the theme, in place of the browser's default. */
export const styleMotion: Rule = {
    meta: {
        type: "problem",
        schema: [],
        docs: { description: "give every transition and animation a curve" },
        messages: {
            curve: "set {{curve}}, such as motion.easingStandard, beside {{name}}",
        },
    },
    create(context) {
        return {
            CallExpression(node) {
                // find each style that moves without a curve
                if (!isCreateCall(node)) {
                    return;
                }
                eachCreatedObject(node, (object) => {
                    const keys = new Set(keysOf(object));
                    for (const [name, duration, curve] of CURVES) {
                        if (keys.has(name) && keys.has(duration) && !keys.has(curve)) {
                            context.report({
                                node: object,
                                messageId: "curve",
                                data: { name, curve },
                            });
                        }
                    }
                });
            },
        };
    },
};

/** List the keys of a style. */
function keysOf(object: ESTree.ObjectExpression): string[] {
    return object.properties.flatMap((property) => {
        const key = property.type === "Property" ? keyOf(property) : undefined;

        return key === undefined ? [] : [key];
    });
}
