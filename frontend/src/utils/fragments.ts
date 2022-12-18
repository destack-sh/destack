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
    type
    name
    path
    createdAt
    updatedAt
    deletedAt
  }
`);

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    id
    type
    createdAt
    updatedAt
    deletedAt
    modifier
    name
    compiled
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
    type
    typeShortname
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
    deletedAt
    name
    commented
    compiled
    modifier
    index
    parent {
      id
    }
    symbol {
      ...SymbolContent
    }
    reference {
      ...StatementHeader
    }
    sourceSymbol {
      ...SymbolHeader
    }
    parameters {
      name
      type
      schema {
        ...SchemaElementContentDeep
      }
    }
    arguments {
      name
      value
      reference {
        ...StatementHeader
      }
    }
    text
  }
`);

export const SymbolContentType = graphql(/* GraphQL */ `
  fragment SymbolContent on Symbol {
    id
    type
    typeShortname
    createdAt
    updatedAt
    content {
      ...CodeContent
      ...DatasetContent
      ...ExpectationContent
      ...TaskContent
      ...SchemaContent
    }
  }
`);
