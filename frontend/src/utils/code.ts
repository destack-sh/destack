import { graphql } from "@/gql";

export const CodeContentType = graphql(/* GraphQL */ `
  fragment CodeContent on Code {
    id
    builtinId
    code
    symbol {
      parameters {
        name
        type
        schema {
          ...SchemaElementContentDeep
        }
      }
      arguments {
        name
        type
        value
        reference {
          id
          name
          typeNameDeclaration
        }
      }
    }
  }
`);
