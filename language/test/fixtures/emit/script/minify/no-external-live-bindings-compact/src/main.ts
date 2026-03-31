export { external1 } from "external1";
export * from "external2";
export { external3 as thirdExternal } from "external3";

export const summary = {
    first: "external1",
    second: "external2",
    third: "external3",
};
