import { DeclarationReference } from "@destack/package/declare";
import { type Trigger, TriggerDescription } from "../trigger/trigger.ts";

/** Describe a workflow's trigger by what fires it, checking a schedule's timing. */
export function describeTrigger<Input>(trigger: Trigger<Input>): TriggerDescription {
    // describe a schedule as declared
    const { name } = trigger;
    if (trigger.kind === "schedule") {
        return TriggerDescription.parse({ name, kind: "schedule", schedule: trigger.schedule });
    }
    // describe the changed object type by reference
    else if (trigger.kind === "change") {
        const { object, ...change } = trigger.change;

        return TriggerDescription.parse({
            name,
            kind: "change",
            change: { ...change, object: DeclarationReference.of(object) },
        });
    }
    // describe a webhook by its signature's name
    else {
        const { signature, path } = trigger.webhook;

        return TriggerDescription.parse({
            name,
            kind: "webhook",
            webhook: { verification: signature.name, path },
        });
    }
}
