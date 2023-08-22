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

export const HasCrudType = graphql(/* GraphQL */ `
  fragment HasCrudContent on HasCrud {
    id
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const ProjectVersionHeaderType = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    id
    name
    tag
    description
    committed
    committedAt
    parents {
      id
    }
    id
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const ProjectHeaderType = graphql(/* GraphQL */ `
  fragment ProjectHeader on Project {
    __typename
    id
    visibility
    createdAt
    updatedAt
    name
    slug
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
    __typename
    id
    revision
    name
    parent {
      id
    }
    projectVersion {
      id
    }
    deletedAt
    id
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const StatementHeaderType = graphql(/* GraphQL */ `
  fragment StatementHeader on Statement {
    __typename
    id
    type
    revision
    name
    orderKey
    parent {
      ... on File {
        id
      }
      ... on Statement {
        id
      }
    }
    createdAt
    updatedAt
    deletedAt
    lastEditedAt
  }
`);

export const FieldType = graphql(/* GraphQL */ `
  fragment FieldContent on Field {
    # :FieldContent
    __typename
    id
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
    parent {
      id
    }
    metadata
    # crud
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const TaggingType = graphql(/* GraphQL */ `
  fragment TaggingContent on Tagging {
    # :TaggingContent
    id
    revision
    key
    parent {
      id
    }
    reference {
      id
    }
    metadata
    # crud
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const TriggerContentType = graphql(/* GraphQL */ `
  fragment TriggerContent on Trigger {
    # :TriggerContent
    id
    revision
    parent {
      id
    }
    type
    active
    mapping
    timezone
    scheduleType
    interval
    cron
    runnable {
      id
    }
    scope {
      id
    }
    # crud
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const StatementContentType = graphql(/* GraphQL */ `
  fragment StatementContent on Statement {
    id
    type
    revision
    name
    orderKey
    parent {
      id
    }
    # symbol contents
    key
    text
    lang
    code
    description
    value
    rootTypeTag
    rootTypeFlags
    reference {
      id
    }
    tags(filters: { isVisible: true }) {
      ...TaggingContent
    }
    fields(filters: { isVisible: true }) {
      ...FieldContent
    }
    triggers(filters: { isVisible: true }) {
      ...TriggerContent
    }
    # interp
    # TODO @Performance: could probably just use module interp state for statement, but would be less responsive on load
    issues {
      ...IssueContent
    }
    resolvedFields {
      ...ResolvedFieldContent
    }
    # crud
    createdAt
    updatedAt
    deletedAt
    createdBy {
      id
    }
    lastEditedAt
    lastEditedBy {
      id
    }
  }
`);

export const IssueContentType = graphql(/* GraphQL */ `
  fragment IssueContent on Issue {
    # :IssueContent
    id
    kind
    type
    message
    parent {
      id
    }
  }
`);

export const ResolvedFieldContentType = graphql(/* GraphQL */ `
  fragment ResolvedFieldContent on ResolvedField {
    statement {
      id
    }
    field {
      id
    }
  }
`);

export const InterpFileType = graphql(/* GraphQL */ `
  fragment InterpFile on File {
    # :InterpFile
    id
    revision
    name
    parent {
      id
    }
    issues {
      ...IssueContent
    }
    createdAt
    updatedAt
    deletedAt
    lastEditedAt
  }
`);

export const InterpStatementType = graphql(/* GraphQL */ `
  fragment InterpStatement on Statement {
    # :InterpStatement
    id
    type
    name
    description
    revision
    file {
      id
    }
    parent {
      id
    }
    orderKey
    key
    rootTypeTag
    rootTypeFlags
    reference {
      id
    }
    tags(filters: { isVisible: true }) {
      # :TaggingContent
      id
      revision
      key
      parent {
        id
      }
      reference {
        id
      }
      metadata
      # crud
      createdAt
      updatedAt
      deletedAt
    }
    fields(filters: { isVisible: true }) {
      # :FieldContent
      id
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
      parent {
        id
      }
      metadata
      # crud
      createdAt
      updatedAt
      deletedAt
    }
    issues {
      ...IssueContent
    }
    resolvedFields {
      ...ResolvedFieldContent
    }
    createdAt
    updatedAt
    deletedAt
    lastEditedAt
  }
`);
