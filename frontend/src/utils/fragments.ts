import { graphql } from "@/gql";

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
  }
`);

export const ProjectHeaderType = graphql(/* GraphQL */ `
  fragment ProjectHeader on Project {
    id
    name
    createdAt
    updatedAt
    head {
      ...ProjectVersionHeader
    }
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    name
    path
    createdAt
    updatedAt
  }
`);

export const CompilationHeaderType = graphql(/* GraphQL */ `
  fragment CompilationHeader on Compilation {
    id
    name
    createdAt
    updatedAt
    task {
      id
      definition {
        id
        name
        typeNameDeclaration
      }
    }
    backends {
      id
      definition {
        id
        name
        typeNameDeclaration
      }
    }
    targetTask {
      id
      definition {
        id
        name
        typeNameDeclaration
      }
    }
    targetCode {
      id
      definition {
        id
        name
        typeNameDeclaration
      }
    }
  }
`);

export const SchemaElementContentDeepType = graphql(/* GraphQL */ `
  fragment SchemaElementContentDeep on SchemaElement {
    name
    type
    choices
    elements {
      name
      type
      choices
    }
  }
`);

export const SymbolDefinitionContentType = graphql(/* GraphQL */ `
  fragment SymbolDefinitionContent on SymbolDefinition {
    id
    name
    type
    typeShortname
    typeNameDeclaration
    createdAt
    updatedAt
    generated
    content {
      ...CodeContent
      ...DatasetContent
      ...ExpectationContent
      ...TaskContent
    }
  }
`);
