import { defineDatabase } from "@destack/db";
import { tablesOf } from "@destack/object";
import objectPackage from "@destack/object/package";
import settingPackage from "@destack/setting/package";
import accountPackage from "../package.ts";

/** The universe's database: the account package and the core packages it serves beside. */
export const accountDatabase = defineDatabase({
    name: "account",
    ...tablesOf({
        packages: [accountPackage, objectPackage, settingPackage],
        follows: [],
        tables: [],
    }),
});
