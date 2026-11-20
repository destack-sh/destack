import { graph } from "@destack/package";
import { schema } from "@destack/schema";
import type { Action, ActionDescription } from "../declare/index.ts";

/** Describe an action by its title, its keys and the call it makes. */
export function describeAction(action: Action): ActionDescription {
    return action.describe();
}

/** Invoke the object method an action calls, none for one the open view handles. */
export function actionSymbols(action: Action): graph.MemberSymbol[] {
    const { call } = action;

    return call === undefined
        ? []
        : schema.array(graph.MemberSymbol).parse([
              {
                  relationships: [
                      {
                          kind: "invokes",
                          symbol: {
                              packageId: call.object.package.id,
                              kind: "method",
                              name: call.method,
                              parent: { kind: "object", name: call.object.name },
                          },
                      },
                  ],
              },
          ]);
}
