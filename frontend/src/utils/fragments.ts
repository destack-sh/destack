import { graphql } from "@/gql";

export const OperationInfoContentType = graphql(/* GraphQL */ `
  fragment OperationInfoContent on OperationInfo {
    ... on OperationInfo {
      messages {
        kind
        message
        field
      }
    }
  }
`);

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    parents {
      id
    }
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

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    id
    type
    createdAt
    updatedAt
    modifier
    generated
    commented
    index
    file {
      id
      path
    }
    parent {
      id
    }
    symbol {
      id
    }
    reference {
      id
    }
  }
`);

export const SymbolHeaderType = graphql(/* GraphQL */ `
  fragment SymbolHeader on Symbol {
    id
    name
    type
    typeShortname
    typeNameDeclaration
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
      symbol {
        id
        name
        typeNameDeclaration
      }
    }
    backends {
      id
      symbol {
        id
        name
        typeNameDeclaration
      }
    }
    targetTask {
      id
      symbol {
        id
        name
        typeNameDeclaration
      }
    }
    targetCode {
      id
      symbol {
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
    required
    schemaId
    choices
    elements {
      name
      type
      required
      schemaId
      choices
      elements {
        name
        type
        required
        schemaId
        choices
      }
    }
  }
`);

export const StatementContentType = graphql(/* GraphQL */ `
  fragment StatementContent on Statement {
    id
    type
    typeShortname
    createdAt
    updatedAt
    commented
    generated
    modifier
    index
    parent {
      id
    }
    symbol {
      ...SymbolContent
    }
    reference {
      ...SymbolHeader
    }
    text
  }
`);

export const SymbolContentType = graphql(/* GraphQL */ `
  fragment SymbolContent on Symbol {
    id
    name
    type
    typeShortname
    typeNameDeclaration
    createdAt
    updatedAt
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
    content {
      ...CodeContent
      ...DatasetContent
      ...ExpectationContent
      ...TaskContent
      ...SchemaContent
    }
  }
`);
