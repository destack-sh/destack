import {
    type ConnectionDeclaration,
    type ConnectionDescription,
    ConnectionKind,
} from "../declare/connection.ts";

/** Describe a declared connection for the package manifest. */
export function describeConnection(declaration: ConnectionDeclaration): ConnectionDescription {
    return ConnectionKind.description.parse({
        name: declaration.name,
        kind: declaration.kind,
        spec: declaration.spec,
    });
}
