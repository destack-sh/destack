import type {} from "@destack/stack/declare";
import type { StackInputOf } from "@destack/object";
import type { secret } from "../object/secret.ts";

/** Add the secrets collection to stacks. */
declare module "@destack/stack/declare" {
    interface StackDefinition {
        /** The secrets the stack declares, without their values. */
        readonly secrets?: Readonly<Record<string, StackInputOf<typeof secret>>>;
    }
}
