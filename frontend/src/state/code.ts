import { graphql } from "@/gql";

export const CodeContentType = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
    builtinId
    code
  }
`);
