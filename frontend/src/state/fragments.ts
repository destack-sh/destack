import { graphql } from "@/gql";

export const PageInfoType = graphql(/* GraphQL */ `
  fragment PageInfo on PageInfo {
    hasNextPage
    hasPreviousPage
    startCursor
    endCursor
  }
`);

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
    tag
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
    type
    visibility
    name
    slug
    createdAt
    updatedAt
    canWrite
    head {
      ...ProjectVersionHeader
    }
    owner {
      ... on Organization {
        id
        slug
        name
      }
      ... on User {
        id
        slug
        username
        name
      }
    }
  }
`);

export const FileHeaderType = graphql(/* GraphQL */ `
  fragment FileHeader on File {
    id
    revision
    name
    parent {
      ... on File {
        id
      }
      ... on ProjectVersion {
        id
      }
    }
    createdAt
    updatedAt
    deletedAt
    directory
    projectVersion {
      id
    }
  }
`);

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    id
    type
    revision
    createdAt
    updatedAt
    deletedAt
    modifier
    name
    commented
    orderKey
    parent {
      ... on File {
        id
      }
      ... on Statement {
        id
      }
    }
  }
`);

export const FieldType = graphql(/* GraphQL */ `
  fragment FieldContent on Field {
    # :FieldContent
    id
    createdAt
    updatedAt
    deletedAt
    revision
    name
    key
    tag
    hint
    flags
    description
    orderKey
    reference {
      id
    }
    metadata
  }
`);

export const StatementContentType = graphql(/* GraphQL */ `
  fragment StatementContent on Statement {
    id
    type
    revision
    createdAt
    updatedAt
    deletedAt
    name
    commented
    modifier
    orderKey
    parent {
      ... on Statement {
        id
      }
      ... on File {
        id
      }
    }
    # symbol contents
    text
    lang
    code
    description
    value
    referenceProjectVersion {
      id
    }
    rootTypeTag
    rootTypeFlags
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
    # interp
    resolvedFields {
      ...FieldContent
    }
    issues {
      ...IssueContent
    }
  }
`);

export const IssueContentType = graphql(/* GraphQL */ `
  fragment IssueContent on Issue {
    # :IssueContent
    id
    scope
    kind
    type
    message
    file {
      id
    }
    statement {
      id
    }
  }
`);

export const ResolvedFieldContentType = graphql(/* GraphQL */ `
  fragment ResolvedFieldContent on ResolvedField {
    id
    statement {
      id
    }
    field {
      ...FieldContent
    }
  }
`);

export const InterpFileType = graphql(/* GraphQL */ `
  fragment InterpFile on File {
    id
    revision
    name
    directory
    parent {
      ... on File {
        id
      }
      ... on ProjectVersion {
        id
      }
    }
    createdAt
    updatedAt
    deletedAt
  }
`);

export const InterpStatementType = graphql(/* GraphQL */ `
  fragment InterpStatement on Statement {
    id
    type
    name
    modifier
    revision
    createdAt
    updatedAt
    deletedAt
    file {
      id
    }
    parent {
      ... on Statement {
        id
      }
      ... on File {
        id
      }
    }
    orderKey
    referenceProjectVersion {
      id
    }
    rootTypeTag
    rootTypeFlags
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
  }
`);

export const InterpStatementDataType = graphql(/* GraphQL */ `
  fragment InterpStatementData on Statement {
    id
    # TODO @Cleanup: use FieldContent and IssueContent fragments (which can't be found when used here for some reason)
    resolvedFields {
      # :FieldContent
      id
      createdAt
      updatedAt
      deletedAt
      revision
      name
      key
      tag
      hint
      description
      orderKey
      reference {
        id
      }
      flags
    }
    issues {
      # :IssueContent
      id
      kind
      scope
      type
      message
      file {
        id
      }
      statement {
        id
      }
    }
  }
`);
