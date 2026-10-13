import type { Table } from "@destack/db";
import type { Method } from "../method/method.ts";
import { method } from "../method/record.ts";
import { traitPermissions, type Trait } from "./trait.ts";

/** The get and list methods a type derives, those it declares itself left out. */
export type ReadableMethodMap<Methods, Defined extends Table> = ("get" extends keyof Methods
    ? {}
    : {
          readonly get: Method<{ kind: "get"; permission: "read"; mutates: false; table: Defined }>;
      }) &
    ("list" extends keyof Methods
        ? {}
        : {
              readonly list: Method<{
                  kind: "list";
                  permission: "read";
                  mutates: false;
                  table: Defined;
              }>;
          });

/** Every object type, readable one by one and in lists by its read permission. */
export const readable: Trait<{}> = {
    options: () => ({}),
    columns: () => ({}),
    constraints: () => [],
    policy: (_options, _object, permissions) => traitPermissions(permissions, ["read"]),
    methods: (_options, declared) => ({
        ...(Object.hasOwn(declared, "get") ? {} : { get: method.get("read") }),
        ...(Object.hasOwn(declared, "list") ? {} : { list: method.list("read") }),
    }),
};
