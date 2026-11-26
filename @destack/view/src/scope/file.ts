import { type ObjectType, PackageMount } from "@destack/object";
import { ObjectFileClient } from "@destack/object/client";
import { ScopePath } from "../declare/context.ts";
import { useEnvironment, useView } from "../page/context.ts";
import { useOrigin } from "../page/view.ts";
import { type ScopeOptions, scopeOf } from "./query.ts";

/** Open the files of a type's objects in the view's scope that keeps them, as the page. */
export function useFiles(type: ObjectType, options: ScopeOptions = {}): ObjectFileClient {
    const scope = scopeOf(useView(), type, options);
    const mount = `${ScopePath.of(scope)}${PackageMount.path(type.package.id)}`;

    return new ObjectFileClient({ ...useOrigin()(mount), environment: useEnvironment() });
}
