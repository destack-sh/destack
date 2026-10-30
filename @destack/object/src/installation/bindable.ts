import type { Select, Snapshot } from "@destack/db";
import type { ObjectReference } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

/** A kind of object a binding targets, and the version a deployment captures of it. */
export interface Bindable<Object extends ObjectType = ObjectType> {
    /** The bindable object type. */
    readonly object: Object;
    /** The object types captures of this kind let installations use. */
    readonly used: readonly ObjectType[];
    /** Read the version a deployment captures of a target, null for a secret without a version. */
    version(target: Select<Object["table"]>, pin: number | null): number | null;
    /** Read the objects a capture of a target in the deployment's space lets it use. */
    readonly uses: (
        captured: {
            readonly scope: string;
            readonly target: string;
            readonly version: number | null;
        },
        snapshot: Pick<Snapshot, "read">,
    ) => Promise<readonly ObjectReference[]>;
    /** The declaration kind whose needs this kind binds, such as resource or secret. */
    readonly declaration: string;
}
