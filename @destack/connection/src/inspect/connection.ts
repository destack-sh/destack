import type { ResourceDeclaration } from "@destack/resource";
import {
    type ConnectionDescription,
    ConnectionKind,
    type ConnectionSpec,
} from "../declare/connection.ts";

/** Describe a declared connection for the package manifest. */
export function describeConnection(
    declaration: Pick<ResourceDeclaration<unknown, typeof ConnectionSpec>, "name" | "spec">,
): ConnectionDescription {
    return ConnectionKind.description.parse({
        name: declaration.name,
        spec: declaration.spec,
    });
}
