import { graphql } from "@/gql";

export const CodeContentType = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
    id
    builtinId
    code
  }
`);
