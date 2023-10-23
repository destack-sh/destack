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
    ck
    name
    tag
    description
    committed
    committedAt
    parents {
      id
    }
    children {
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
    id
    createdAt
    updatedAt
    name
    slug
    head {
      ...ProjectVersionHeader
    }
    visibility
    accessLevel
    baseLevel
    sharingEnabled
    sharingToken
    sharingLevel
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
    ck
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
    ck
    type
    revision
    name
    headingLevel
    text
    orderKey
    parent {
      id
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
    id
    ck
    revision
    name
    key
    tag
    hint
    flags
    text
    orderKey
    referenceCk
    parent {
      id
    }
    value
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
    ck
    revision
    key
    parent {
      id
    }
    referenceCk
    value
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
    ck
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
    statementCk
    scopeCk
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
    ck
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
    headingLevel
    code
    value
    referenceCk
    versioned
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
    # TODO @Performance: could probably just use module interp state for statement, but would be slower on initial load
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
    ck
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
    __typename
    id
    ck
    statement {
      id
    }
    orderKey
    fieldCk
  }
`);

export const InterpFileType = graphql(/* GraphQL */ `
  fragment InterpFile on File {
    # :InterpFile
    id
    ck
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
    ck
    type
    name
    text
    headingLevel
    revision
    file {
      id
    }
    parent {
      id
    }
    orderKey
    key
    referenceCk
    tags(filters: { isVisible: true }) {
      ...TaggingContent
    }
    fields(filters: { isVisible: true }) {
      ...FieldContent
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
