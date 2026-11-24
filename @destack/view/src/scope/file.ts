import { type ObjectType, PackageMount } from "@destack/object";
import { ObjectFileClient } from "@destack/object/client";
import { ScopePath, ViewContext } from "../declare/context.ts";
import { useEnvironment, useView } from "../page/context.ts";
import { useOrigin } from "../page/view.ts";
import type { ScopeOptions } from "./text.ts";

/** Open the files of a type's objects in the view's scope that keeps them, as the page. */
export function useFiles(type: ObjectType, options: ScopeOptions = {}): ObjectFileClient {
    const scope = ViewContext.scope(useView(), options.scope ?? "space");
    const mount = `${ScopePath.of(scope)}${PackageMount.path(type.package.id)}`;

    return new ObjectFileClient({ ...useOrigin()(mount), environment: useEnvironment() });
}
