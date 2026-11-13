import { BORN_SPACE, TestSpace } from "@destack/machine/test";
import vaultPackage from "../package.ts";

/** The templates vault's tests copy, which the run writes once. */
export const templates = [TestSpace.template({ packages: [vaultPackage] }, BORN_SPACE)];
