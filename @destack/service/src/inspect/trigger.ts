import { graph } from "@destack/package";
import { schema, type JsonValue } from "@destack/schema";
import { DeclarationReference } from "@destack/package/declare";
import { Schedule } from "../schedule/index.ts";
import { type Trigger, TriggerDescription } from "../trigger/index.ts";

/** Describe a trigger, checking a schedule's timing. */
export function describeTrigger(trigger: Trigger): TriggerDescription {
    // describe a schedule with its call, checking its timing
    const on = trigger.on;
    if ("schedule" in on) {
        Schedule.require(on.schedule.timing);

        return TriggerDescription.parse({ name: trigger.name, on });
    }
    // describe the changed object type by reference
    else if ("change" in on) {
        const { object, ...change } = on.change;

        return TriggerDescription.parse({
            name: trigger.name,
            on: { change: { ...change, object: DeclarationReference.of(object) } },
        });
    }

    // describe a webhook by its signature's name, without its secret
    const { signature, route } = on.webhook;

    return TriggerDescription.parse({
        name: trigger.name,
        on: { webhook: { verification: signature.name, route } },
    });
}

/** List the object method a scheduled trigger invokes in the trigger's package. */
export function triggerSymbols(input: Record<string, JsonValue>): graph.MemberSymbol[] {
    // invoke nothing outside a schedule
    const { on } = TriggerDescription.parse(input);
    if (!("schedule" in on)) {
        return [];
    }

    // split the object type and method the call names, as in `note.get`
    const { call } = on.schedule;
    const separator = call.method.indexOf(".");
    if (separator === -1) {
        throw new TypeError(`trigger call names no object type: ${call.method}`);
    }
    const type = call.method.slice(0, separator);
    const method = call.method.slice(separator + 1);

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: [
                {
                    kind: "invokes",
                    symbol: {
                        kind: "method",
                        name: method,
                        parent: { kind: "object", name: type },
                    },
                },
            ],
        },
    ]);
}
