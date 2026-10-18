import { AccessName } from "@destack/sync";
import { DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";
import { InstallationName } from "@destack/runtime";

/** The binding target type of a resource the stack declares, whatever its kind. */
export const RESOURCE_TARGET = "resource";

/** The binding target type of a secret of the space. */
export const SECRET_TARGET = "secret";

/** The declaration kind of secrets, which workloads bind beside resources. */
export const SECRET_DECLARATION = "secret";

/** A bindable object a binding targets. */
export const StackBindingTarget = schema.union([
    schema.object({
        /** The bindable object type, such as resource or secret. */
        type: AccessName,
        /** The object's key in this configuration. */
        name: DeclarationName,
    }),
    schema.object({
        /** The bindable object type, such as resource or secret. */
        type: AccessName,
        /** The existing object in the destination space. */
        id: schema.anyIdentifier(),
    }),
    schema.object({
        /** An installation of another space. */
        type: schema.literal("installation"),
        /** The installation's name in that space. */
        name: InstallationName,
        /** The other space, which grants the binding. */
        spaceId: schema.identifier("space"),
    }),
]);
/** A bindable object a binding targets. */
export type StackBindingTarget = schema.Infer<typeof StackBindingTarget>;

/** A binding of one package declaration to a bindable object. */
export const StackBinding = schema.object({
    /** The bound object. */
    target: StackBindingTarget,
    /** An exact version or generation to run with, else the target's current one. */
    version: schema.number().int().positive().exactOptional(),
    /** The state the declaration requires of the target, such as a database's tables. */
    state: schema.jsonObject(),
});
/** A binding of one declaration to an object the space has. */
export type StackBinding = schema.Infer<typeof StackBinding>;
