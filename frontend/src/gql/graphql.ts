/* eslint-disable */
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: string;
  String: string;
  Boolean: boolean;
  Int: number;
  Float: number;
  /** Date with time (isoformat) */
  DateTime: any;
  /** The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `"4"`) or integer (such as `4`) input value will be accepted as an ID. */
  GlobalID: any;
  /** The `JSON` scalar type represents JSON values as specified by [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf). */
  JSON: any;
  UUID: any;
};

export type BuildInput = {
  buildableId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
};

export type BuildState = {
  __typename?: "BuildState";
  buildIds: Array<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  success: Scalars["Boolean"];
};

export type BuildStateOperationInfo = BuildState | OperationInfo;

export type CommitInput = {
  description?: InputMaybe<Scalars["String"]>;
  name: Scalars["String"];
  projectVersionId: Scalars["GlobalID"];
};

export type CommitPayload = {
  __typename?: "CommitPayload";
  committedVersion: ProjectVersion;
  newWorkingVersion: ProjectVersion;
  project: Project;
};

export type CommitPayloadOperationInfo = CommitPayload | OperationInfo;

export type DatasetRecord = Node & {
  __typename?: "DatasetRecord";
  createdAt: Scalars["DateTime"];
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

export enum ErrorType {
  AmbiguousDefinition = "AMBIGUOUS_DEFINITION",
  AmbiguousRequirement = "AMBIGUOUS_REQUIREMENT",
  BuildMissingModel = "BUILD_MISSING_MODEL",
  BuildMissingTask = "BUILD_MISSING_TASK",
  CircularAncestry = "CIRCULAR_ANCESTRY",
  ExpectedArguments = "EXPECTED_ARGUMENTS",
  ExpectedBlank = "EXPECTED_BLANK",
  ExpectedParameters = "EXPECTED_PARAMETERS",
  ExpectedParent = "EXPECTED_PARENT",
  ExpectedProperChildren = "EXPECTED_PROPER_CHILDREN",
  ExternalLookupFailed = "EXTERNAL_LOOKUP_FAILED",
  Internal = "INTERNAL",
  InvalidStatement = "INVALID_STATEMENT",
  InvalidTokenValue = "INVALID_TOKEN_VALUE",
  MissingExtra = "MISSING_EXTRA",
  MissingReference = "MISSING_REFERENCE",
  MissingToken = "MISSING_TOKEN",
  ReferenceTypeMismatch = "REFERENCE_TYPE_MISMATCH",
  UndefinedExternalReference = "UNDEFINED_EXTERNAL_REFERENCE",
  UndefinedLocalReference = "UNDEFINED_LOCAL_REFERENCE",
  UnexpectedChildren = "UNEXPECTED_CHILDREN",
  UnexpectedExtra = "UNEXPECTED_EXTRA",
  UnexpectedIndent = "UNEXPECTED_INDENT",
  UnexpectedParameters = "UNEXPECTED_PARAMETERS",
  UnexpectedParent = "UNEXPECTED_PARENT",
  UnexpectedStatement = "UNEXPECTED_STATEMENT",
  UnexpectedTokenType = "UNEXPECTED_TOKEN_TYPE",
  UnexpectedTokenValue = "UNEXPECTED_TOKEN_VALUE",
  UnknownImportSource = "UNKNOWN_IMPORT_SOURCE",
  UnknownToken = "UNKNOWN_TOKEN",
}

export type Execution = Node & {
  __typename?: "Execution";
  code: Statement;
  createdAt: Scalars["DateTime"];
  descendants: Array<Execution>;
  durationMillis?: Maybe<Scalars["Float"]>;
  error?: Maybe<Scalars["JSON"]>;
  id: Scalars["GlobalID"];
  inputs?: Maybe<Scalars["JSON"]>;
  model?: Maybe<Statement>;
  outputs?: Maybe<Scalars["JSON"]>;
  parent?: Maybe<Execution>;
  root?: Maybe<Execution>;
  /** Time of transition to RUNNING status. */
  startedAt?: Maybe<Scalars["DateTime"]>;
  status: ExecutionStatus;
  /** Time of transition to a terminal status. */
  terminatedAt?: Maybe<Scalars["DateTime"]>;
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type ExecutionConnection = {
  __typename?: "ExecutionConnection";
  /** Contains the nodes in this connection */
  edges: Array<ExecutionEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type ExecutionEdge = {
  __typename?: "ExecutionEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Execution;
};

export enum ExecutionStatus {
  Aborted = "Aborted",
  Aborting = "Aborting",
  Completed = "Completed",
  Created = "Created",
  Failed = "Failed",
  Queued = "Queued",
  Running = "Running",
  Scheduled = "Scheduled",
}

export type File = Node & {
  __typename?: "File";
  createdAt: Scalars["DateTime"];
  deletedAt?: Maybe<Scalars["DateTime"]>;
  files: Array<File>;
  generated: Scalars["Boolean"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  parent?: Maybe<File>;
  path: Scalars["String"];
  projectVersion: ProjectVersion;
  revision: Scalars["Int"];
  statements: Array<Statement>;
  updatedAt: Scalars["DateTime"];
};

export type FileStatementsArgs = {
  filters?: InputMaybe<StatementFilter>;
};

export type FileCreateInput = {
  directory?: Scalars["Boolean"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  name: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  path: Scalars["String"];
  projectVersionId: Scalars["GlobalID"];
};

export type FileFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type FileMoveInput = {
  id: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  path: Scalars["String"];
};

export type FileOperationInfo = File | OperationInfo;

export type FileRenameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  path: Scalars["String"];
};

export type InterpError = {
  __typename?: "InterpError";
  message: Scalars["String"];
  symbol?: Maybe<InterpSymbol>;
  type: ErrorType;
};

export type InterpFile = {
  __typename?: "InterpFile";
  id: Scalars["GlobalID"];
  module: InterpModule;
  path: Scalars["String"];
  symbols: Array<InterpSymbol>;
};

export type InterpModule = {
  __typename?: "InterpModule";
  files: Array<InterpFile>;
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

/**
 * Proxy type to SimpleType to avoid overwriting source SimpleType references
 * (no extra fields yet but needed since (SimpleType, id) global id would be the same
 *  for the simple types output by the runtime and by the source types put in).
 */
export type InterpSimpleType = Node &
  SimpleType & {
    __typename?: "InterpSimpleType";
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    isArray: Scalars["Boolean"];
    isNullable: Scalars["Boolean"];
    isOutput: Scalars["Boolean"];
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    reference?: Maybe<Statement>;
    statement: NodeType;
    tag: TypeTag;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

export type InterpSymbol = SimplyTyped & {
  __typename?: "InterpSymbol";
  file: InterpFile;
  generated: Scalars["Boolean"];
  id: Scalars["GlobalID"];
  modifier?: Maybe<StatementModifier>;
  name?: Maybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  parentId?: Maybe<Scalars["GlobalID"]>;
  rootTypeTag?: Maybe<TypeTag>;
  symbolType?: Maybe<SymbolType>;
  type: StatementType;
  typeNodes?: Maybe<Array<InterpSimpleType>>;
};

export type ModuleRuntime = {
  __typename?: "ModuleRuntime";
  dependencies: Array<InterpModule>;
  errors: Array<InterpError>;
  module: InterpModule;
  updatedAt: Scalars["DateTime"];
};

export type Mutation = {
  __typename?: "Mutation";
  build: BuildStateOperationInfo;
  commentStatement: StatementOperationInfo;
  commit: CommitPayloadOperationInfo;
  completeSignup: UserOperationInfo;
  createFile: FileOperationInfo;
  createStatement: StatementOperationInfo;
  createStatementRecord: StatementOperationInfo;
  createStatementTypeNode: StatementOperationInfo;
  deleteStatementRecord: StatementOperationInfo;
  deleteStatementTypeNode: StatementOperationInfo;
  logout?: Maybe<OperationInfo>;
  morphStatement: StatementOperationInfo;
  moveFile: FileOperationInfo;
  moveStatement: StatementOperationInfo;
  moveStatementRecord: StatementOperationInfo;
  moveStatementTypeNode: StatementOperationInfo;
  renameFile: FileOperationInfo;
  renameStatement: StatementOperationInfo;
  restoreFile: FileOperationInfo;
  restoreStatement: StatementOperationInfo;
  run: RunStateOperationInfo;
  softDeleteFile: FileOperationInfo;
  softDeleteStatement: StatementOperationInfo;
  updateStatementCode: StatementOperationInfo;
  updateStatementDescription: StatementOperationInfo;
  updateStatementLanguage: StatementOperationInfo;
  updateStatementModifier: StatementOperationInfo;
  updateStatementRecord: StatementOperationInfo;
  updateStatementReference: StatementOperationInfo;
  updateStatementText: StatementOperationInfo;
  updateStatementTypeNode: StatementOperationInfo;
};

export type MutationBuildArgs = {
  input: BuildInput;
};

export type MutationCommentStatementArgs = {
  input: StatementCommentedInput;
};

export type MutationCommitArgs = {
  input: CommitInput;
};

export type MutationCompleteSignupArgs = {
  input: UserCompleteSignupInput;
};

export type MutationCreateFileArgs = {
  input: FileCreateInput;
};

export type MutationCreateStatementArgs = {
  input: StatementCreateInput;
};

export type MutationCreateStatementRecordArgs = {
  input: RecordCreateInput;
};

export type MutationCreateStatementTypeNodeArgs = {
  input: TypeNodeCreateInput;
};

export type MutationDeleteStatementRecordArgs = {
  input: RecordDeleteInput;
};

export type MutationDeleteStatementTypeNodeArgs = {
  input: TypeNodeDeleteInput;
};

export type MutationMorphStatementArgs = {
  input: StatementMorphInput;
};

export type MutationMoveFileArgs = {
  input: FileMoveInput;
};

export type MutationMoveStatementArgs = {
  input: StatementMoveInput;
};

export type MutationMoveStatementRecordArgs = {
  input: RecordMoveInput;
};

export type MutationMoveStatementTypeNodeArgs = {
  input: TypeNodeMoveInput;
};

export type MutationRenameFileArgs = {
  input: FileRenameInput;
};

export type MutationRenameStatementArgs = {
  input: StatementRenameInput;
};

export type MutationRestoreFileArgs = {
  input: NodeInput;
};

export type MutationRestoreStatementArgs = {
  input: StatementRestoreInput;
};

export type MutationRunArgs = {
  input: RunInput;
};

export type MutationSoftDeleteFileArgs = {
  input: NodeInput;
};

export type MutationSoftDeleteStatementArgs = {
  input: StatementSoftDeleteInput;
};

export type MutationUpdateStatementCodeArgs = {
  input: StatementUpdateCodeInput;
};

export type MutationUpdateStatementDescriptionArgs = {
  input: StatementUpdateDescriptionInput;
};

export type MutationUpdateStatementLanguageArgs = {
  input: StatementUpdateLanguageInput;
};

export type MutationUpdateStatementModifierArgs = {
  input: StatementSetModifierInput;
};

export type MutationUpdateStatementRecordArgs = {
  input: RecordUpdateInput;
};

export type MutationUpdateStatementReferenceArgs = {
  input: StatementSetReferenceInput;
};

export type MutationUpdateStatementTextArgs = {
  input: StatementUpdateCodeInput;
};

export type MutationUpdateStatementTypeNodeArgs = {
  input: TypeNodeUpdateInput;
};

/** An object with a Globally Unique ID */
export type Node = {
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
};

/** Input of an object that implements the `Node` interface. */
export type NodeInput = {
  id: Scalars["GlobalID"];
};

/** Generic type for objects that implements the `Node` interface. */
export type NodeType = Node & {
  __typename?: "NodeType";
  id: Scalars["GlobalID"];
};

/** Multiple messages returned by an operation. */
export type OperationInfo = {
  __typename?: "OperationInfo";
  /** List of messages returned by the operation. */
  messages: Array<OperationMessage>;
};

/** An error that happened while executing an operation. */
export type OperationMessage = {
  __typename?: "OperationMessage";
  /** The field that caused the error, or `null` if it isn't associated with any particular field. */
  field?: Maybe<Scalars["String"]>;
  /** The kind of this message. */
  kind: OperationMessageKind;
  /** The error message. */
  message: Scalars["String"];
};

/** The kind of the returned message. */
export enum OperationMessageKind {
  Error = "ERROR",
  Info = "INFO",
  Permission = "PERMISSION",
  Validation = "VALIDATION",
  Warning = "WARNING",
}

export type Organization = Node & {
  __typename?: "Organization";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  members: Array<User>;
  name: Scalars["String"];
  projects: Array<Project>;
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

/** Information to aid in pagination. */
export type PageInfo = {
  __typename?: "PageInfo";
  /** When paginating forwards, the cursor to continue. */
  endCursor?: Maybe<Scalars["String"]>;
  /** When paginating forwards, are there more items? */
  hasNextPage: Scalars["Boolean"];
  /** When paginating backwards, are there more items? */
  hasPreviousPage: Scalars["Boolean"];
  /** When paginating backwards, the cursor to continue. */
  startCursor?: Maybe<Scalars["String"]>;
};

export type Project = Node & {
  __typename?: "Project";
  createdAt: Scalars["DateTime"];
  head: ProjectVersion;
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  owner: UserOrganization;
  path: Scalars["String"];
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  versions: Array<ProjectVersion>;
};

export type ProjectVersionsArgs = {
  filters?: InputMaybe<ProjectVersionFilter>;
};

export type ProjectVersion = Node & {
  __typename?: "ProjectVersion";
  children: Array<ProjectVersion>;
  committed: Scalars["Boolean"];
  committedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  dependencies: Array<ProjectVersion>;
  description?: Maybe<Scalars["String"]>;
  files: Array<File>;
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  parents: Array<ProjectVersion>;
  parentsRefs: Array<RefMapping>;
  project: Project;
  statements: Array<Statement>;
};

export type ProjectVersionFilesArgs = {
  filters?: InputMaybe<FileFilter>;
};

export type ProjectVersionStatementsArgs = {
  filters?: InputMaybe<StatementFilter>;
};

export type ProjectVersionFilter = {
  afterId: Scalars["GlobalID"];
};

export type Query = {
  __typename?: "Query";
  executions: ExecutionConnection;
  file?: Maybe<File>;
  me?: Maybe<User>;
  organization?: Maybe<Organization>;
  ownerBySlug?: Maybe<UserOrganization>;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  user?: Maybe<User>;
};

export type QueryExecutionsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  codeId?: InputMaybe<Scalars["GlobalID"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  rootId?: InputMaybe<Scalars["GlobalID"]>;
};

export type QueryFileArgs = {
  id: Scalars["GlobalID"];
};

export type QueryOrganizationArgs = {
  id: Scalars["GlobalID"];
};

export type QueryOwnerBySlugArgs = {
  slug: Scalars["String"];
};

export type QueryProjectArgs = {
  id: Scalars["GlobalID"];
};

export type QueryProjectBySlugArgs = {
  owner: Scalars["String"];
  project: Scalars["String"];
};

export type QueryProjectVersionArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUserArgs = {
  id: Scalars["GlobalID"];
};

export type RecordCreateInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  statementId: Scalars["GlobalID"];
};

export type RecordDeleteInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

export type RecordMoveInput = {
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  statementId: Scalars["GlobalID"];
};

export type RecordUpdateInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

export type RefMapping = {
  __typename?: "RefMapping";
  source: Scalars["GlobalID"];
  target: Scalars["GlobalID"];
};

export type RunInput = {
  arguments: Scalars["JSON"];
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
};

export type RunState = {
  __typename?: "RunState";
  buildId?: Maybe<Scalars["GlobalID"]>;
  output?: Maybe<Scalars["JSON"]>;
  projectVersionId: Scalars["GlobalID"];
  runnableId?: Maybe<Scalars["GlobalID"]>;
  success: Scalars["Boolean"];
};

export type RunStateOperationInfo = OperationInfo | RunState;

export type SimpleType = {
  description?: Maybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  isArray: Scalars["Boolean"];
  isNullable: Scalars["Boolean"];
  isOutput: Scalars["Boolean"];
  name?: Maybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  reference?: Maybe<Statement>;
  tag: TypeTag;
  value?: Maybe<Scalars["JSON"]>;
};

export type SimpleTypeNode = Node &
  SimpleType & {
    __typename?: "SimpleTypeNode";
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    isArray: Scalars["Boolean"];
    isNullable: Scalars["Boolean"];
    isOutput: Scalars["Boolean"];
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    reference?: Maybe<Statement>;
    statement: NodeType;
    tag: TypeTag;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

/** Anything typed using SimpleType nodes. */
export type SimplyTyped = {
  rootTypeTag?: Maybe<TypeTag>;
  typeNodes?: Maybe<Array<SimpleType>>;
};

export type SourceMapping = {
  __typename?: "SourceMapping";
  sourceId: Scalars["UUID"];
  sourcePath?: Maybe<Scalars["JSON"]>;
  sourceRevision: Scalars["Int"];
  targetId: Scalars["UUID"];
  targetPath?: Maybe<Scalars["JSON"]>;
  targetRevision: Scalars["Int"];
};

export type Statement = Node &
  SimplyTyped & {
    __typename?: "Statement";
    children: Array<Statement>;
    code?: Maybe<Scalars["String"]>;
    commented: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    deletedAt?: Maybe<Scalars["DateTime"]>;
    descendants: Array<Statement>;
    description?: Maybe<Scalars["String"]>;
    file: File;
    generated: Scalars["Boolean"];
    id: Scalars["GlobalID"];
    importPath?: Maybe<Scalars["String"]>;
    lang?: Maybe<Scalars["String"]>;
    mappings: Array<SourceMapping>;
    modifier?: Maybe<StatementModifier>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent?: Maybe<Statement>;
    projectVersion: ProjectVersion;
    records: Array<DatasetRecord>;
    reference?: Maybe<Statement>;
    referenceProjectVersion?: Maybe<ProjectVersion>;
    referencedBy: Array<Statement>;
    revision: Scalars["Int"];
    rootTypeTag?: Maybe<TypeTag>;
    symbolType?: Maybe<SymbolType>;
    type: StatementType;
    typeNodes: Array<SimpleTypeNode>;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

export type StatementCommentedInput = {
  commented: Scalars["Boolean"];
  id: Scalars["GlobalID"];
};

/** Creates a blank statement */
export type StatementCreateInput = {
  fileId: Scalars["GlobalID"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

export type StatementFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

/** A modifier to a Bench statement. */
export enum StatementModifier {
  Check = "CHECK",
  Include = "INCLUDE",
  Like = "LIKE",
  Magic = "MAGIC",
  Unlike = "UNLIKE",
  Var = "VAR",
  With = "WITH",
}

export type StatementMorphInput = {
  id: Scalars["GlobalID"];
  lang?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  symbolType?: InputMaybe<SymbolType>;
  type: StatementType;
  typeNodes?: InputMaybe<Array<TypeNodeCreateInput>>;
};

export type StatementMoveInput = {
  fileId: Scalars["GlobalID"];
  id: Scalars["GlobalID"];
  orderKey?: InputMaybe<Scalars["String"]>;
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

export type StatementOperationInfo = OperationInfo | Statement;

export type StatementRenameInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
};

export type StatementRestoreInput = {
  id: Scalars["GlobalID"];
};

export type StatementSetModifierInput = {
  id: Scalars["GlobalID"];
  modifier?: InputMaybe<StatementModifier>;
};

export type StatementSetReferenceInput = {
  id: Scalars["GlobalID"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
};

export type StatementSoftDeleteInput = {
  id: Scalars["GlobalID"];
};

/** The type of Bench statement. */
export enum StatementType {
  Blank = "BLANK",
  Comment = "COMMENT",
  Definition = "DEFINITION",
  Import = "IMPORT",
  Redefinition = "REDEFINITION",
  Reference = "REFERENCE",
}

export type StatementUpdateCodeInput = {
  code?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
};

export type StatementUpdateDescriptionInput = {
  description: Scalars["String"];
  id: Scalars["GlobalID"];
};

export type StatementUpdateLanguageInput = {
  id: Scalars["GlobalID"];
  language: Scalars["String"];
};

export type Subscription = {
  __typename?: "Subscription";
  moduleExecutionChanged: Execution;
  moduleRuntimeChanged: ModuleRuntime;
};

export type SubscriptionModuleExecutionChangedArgs = {
  codeId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionModuleRuntimeChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

/** The type of symbol content. */
export enum SymbolType {
  Build = "BUILD",
  Capability = "CAPABILITY",
  Code = "CODE",
  Data = "DATA",
  Expectation = "EXPECTATION",
  Model = "MODEL",
  Requirement = "REQUIREMENT",
  Runconfig = "RUNCONFIG",
  Task = "TASK",
  Type = "TYPE",
  Value = "VALUE",
}

export type Type = {
  __typename?: "Type";
  btl: Scalars["String"];
  description?: Maybe<Scalars["String"]>;
};

/** Upsert a statement type node data */
export type TypeNodeCreateInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  isArray?: Scalars["Boolean"];
  isNullable?: Scalars["Boolean"];
  isOutput?: Scalars["Boolean"];
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type TypeNodeDeleteInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

export type TypeNodeMoveInput = {
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  statementId: Scalars["GlobalID"];
};

export type TypeNodeUpdateInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  isArray?: Scalars["Boolean"];
  isNullable?: Scalars["Boolean"];
  isOutput?: Scalars["Boolean"];
  name?: InputMaybe<Scalars["String"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  tag: TypeTag;
  value?: InputMaybe<Scalars["JSON"]>;
};

/** The type of type node. */
export enum TypeTag {
  Any = "ANY",
  Array = "ARRAY",
  Audio = "AUDIO",
  Boolean = "BOOLEAN",
  Embedding = "EMBEDDING",
  Enum = "ENUM",
  Function = "FUNCTION",
  Image = "IMAGE",
  Intersection = "INTERSECTION",
  Literal = "LITERAL",
  Map = "MAP",
  Null = "NULL",
  Number = "NUMBER",
  String = "STRING",
  Struct = "STRUCT",
  Tuple = "TUPLE",
  TypeReference = "TYPE_REFERENCE",
  Union = "UNION",
  Video = "VIDEO",
}

export type User = Node & {
  __typename?: "User";
  completedSignup: Scalars["Boolean"];
  createdAt: Scalars["DateTime"];
  email: Scalars["String"];
  firstName: Scalars["String"];
  id: Scalars["GlobalID"];
  organizations: Array<Organization>;
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  /** Required. 150 characters or fewer. Letters, digits and @/./+/-/_ only. */
  username: Scalars["String"];
};

export type UserCompleteSignupInput = {
  fullName: Scalars["String"];
  id: Scalars["GlobalID"];
  username: Scalars["String"];
};

export type UserOperationInfo = OperationInfo | User;

export type UserOrganization = Organization | User;

export type FileContentByIdQueryVariables = Exact<{
  fileId: Scalars["GlobalID"];
}>;

export type FileContentByIdQuery = {
  __typename?: "Query";
  file?:
    | ({
        __typename?: "File";
        id: any;
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type ProjectVersionsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
}>;

export type ProjectVersionsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    id: any;
    versions: Array<
      { __typename?: "ProjectVersion" } & {
        " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
      }
    >;
  } | null;
};

export type OwnerBySlugQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type OwnerBySlugQuery = {
  __typename?: "Query";
  ownerBySlug?: { __typename?: "Organization"; id: any } | { __typename?: "User"; id: any } | null;
};

export type ProjectBySlugQueryVariables = Exact<{
  owner: Scalars["String"];
  project: Scalars["String"];
}>;

export type ProjectBySlugQuery = {
  __typename?: "Query";
  projectBySlug?:
    | ({ __typename?: "Project" } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } })
    | null;
};

export type ProjectVersionContentQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type ProjectVersionContentQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    name?: string | null;
    description?: string | null;
    createdAt: any;
    committed: boolean;
    committedAt?: any | null;
    files: Array<{ __typename?: "File"; id: any } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } }>;
  } | null;
};

export type MeQueryVariables = Exact<{ [key: string]: never }>;

export type MeQuery = {
  __typename?: "Query";
  me?: ({ __typename?: "User" } & { " $fragmentRefs"?: { UserContentFragment: UserContentFragment } }) | null;
};

export type ProjectMigrationRefsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  afterId: Scalars["GlobalID"];
}>;

export type ProjectMigrationRefsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    versions: Array<{
      __typename?: "ProjectVersion";
      id: any;
      name?: string | null;
      createdAt: any;
      parentsRefs: Array<{ __typename?: "RefMapping"; source: any; target: any }>;
    }>;
  } | null;
};

export type ExecutionContentFragment = {
  __typename?: "Execution";
  id: any;
  createdAt: any;
  updatedAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  status: ExecutionStatus;
  inputs?: any | null;
  outputs?: any | null;
  error?: any | null;
  root?: { __typename?: "Execution"; id: any } | null;
  parent?: { __typename?: "Execution"; id: any } | null;
  code: { __typename?: "Statement"; id: any };
  model?: { __typename?: "Statement"; id: any } | null;
} & { " $fragmentName"?: "ExecutionContentFragment" };

export type ExecutionsQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  codeId?: InputMaybe<Scalars["GlobalID"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
}>;

export type ExecutionsQuery = {
  __typename?: "Query";
  executions: {
    __typename?: "ExecutionConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "ExecutionEdge";
      cursor: string;
      node: {
        __typename?: "Execution";
        descendants: Array<
          { __typename?: "Execution" } & { " $fragmentRefs"?: { ExecutionContentFragment: ExecutionContentFragment } }
        >;
      } & { " $fragmentRefs"?: { ExecutionContentFragment: ExecutionContentFragment } };
    }>;
    pageInfo: {
      __typename?: "PageInfo";
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor?: string | null;
      endCursor?: string | null;
    };
  };
};

export type ModuleExecutionChangedSubscriptionVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type ModuleExecutionChangedSubscription = {
  __typename?: "Subscription";
  moduleExecutionChanged: { __typename?: "Execution" } & {
    " $fragmentRefs"?: { ExecutionContentFragment: ExecutionContentFragment };
  };
};

export type PageInfoFragment = {
  __typename?: "PageInfo";
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor?: string | null;
  endCursor?: string | null;
} & { " $fragmentName"?: "PageInfoFragment" };

export type OperationInfoContentFragment = {
  __typename?: "OperationInfo";
  messages: Array<{
    __typename?: "OperationMessage";
    kind: OperationMessageKind;
    message: string;
    field?: string | null;
  }>;
} & { " $fragmentName"?: "OperationInfoContentFragment" };

export type UserContentFragment = {
  __typename?: "User";
  id: any;
  username: string;
  email: string;
  firstName: string;
  createdAt: any;
  updatedAt: any;
  completedSignup: boolean;
  organizations: Array<{
    __typename?: "Organization";
    id: any;
    name: string;
    slug: string;
    createdAt: any;
    updatedAt: any;
  }>;
} & { " $fragmentName"?: "UserContentFragment" };

export type ProjectVersionHeaderFragment = {
  __typename?: "ProjectVersion";
  id: any;
  name?: string | null;
  description?: string | null;
  createdAt: any;
  committed: boolean;
  committedAt?: any | null;
  parents: Array<{ __typename?: "ProjectVersion"; id: any }>;
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type ProjectHeaderFragment = {
  __typename?: "Project";
  id: any;
  name: string;
  slug: string;
  createdAt: any;
  updatedAt: any;
  head: { __typename?: "ProjectVersion" } & {
    " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
  };
  owner:
    | { __typename?: "Organization"; id: any; slug: string; name: string }
    | { __typename?: "User"; id: any; username: string; firstName: string };
} & { " $fragmentName"?: "ProjectHeaderFragment" };

export type FileHeaderFragment = {
  __typename?: "File";
  id: any;
  revision: number;
  name: string;
  path: string;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  generated: boolean;
  parent?: { __typename?: "File"; id: any } | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any };
} & { " $fragmentName"?: "FileHeaderFragment" };

export type StatementHeaderFragment = {
  __typename?: "Statement";
  id: any;
  type: StatementType;
  revision: number;
  symbolType?: SymbolType | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  modifier?: StatementModifier | null;
  name?: string | null;
  generated: boolean;
  commented: boolean;
  orderKey: string;
  parent?: { __typename?: "Statement"; id: any } | null;
  reference?: { __typename?: "Statement"; id: any } | null;
} & { " $fragmentName"?: "StatementHeaderFragment" };

export type TypeContentFragment = { __typename?: "Type"; description?: string | null } & {
  " $fragmentName"?: "TypeContentFragment";
};

export type SimpleTypeNodeContentFragment = {
  __typename?: "SimpleTypeNode";
  id: any;
  name?: string | null;
  tag: TypeTag;
  description?: string | null;
  value?: any | null;
  orderKey: string;
  isOutput: boolean;
  isArray: boolean;
  isNullable: boolean;
  reference?: { __typename?: "Statement"; id: any } | null;
} & { " $fragmentName"?: "SimpleTypeNodeContentFragment" };

export type StatementContentFragment = {
  __typename?: "Statement";
  id: any;
  type: StatementType;
  revision: number;
  symbolType?: SymbolType | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  name?: string | null;
  commented: boolean;
  generated: boolean;
  modifier?: StatementModifier | null;
  orderKey: string;
  lang?: string | null;
  code?: string | null;
  description?: string | null;
  value?: any | null;
  rootTypeTag?: TypeTag | null;
  parent?: { __typename?: "Statement"; id: any } | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  referenceProjectVersion?: { __typename?: "ProjectVersion"; id: any } | null;
  typeNodes: Array<
    { __typename?: "SimpleTypeNode" } & {
      " $fragmentRefs"?: { SimpleTypeNodeContentFragment: SimpleTypeNodeContentFragment };
    }
  >;
  records: Array<{ __typename?: "DatasetRecord"; id: any; orderKey: string; data: any }>;
} & { " $fragmentName"?: "StatementContentFragment" };

export type CreateFileMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
  directory?: InputMaybe<Scalars["Boolean"]>;
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  path: Scalars["String"];
}>;

export type CreateFileMutation = {
  __typename?: "Mutation";
  createFile:
    | ({
        __typename?: "File";
        id: any;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteFileMutation = {
  __typename?: "Mutation";
  softDeleteFile:
    | { __typename?: "File"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RestoreFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreFileMutation = {
  __typename?: "Mutation";
  restoreFile:
    | { __typename?: "File"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RenameFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  path: Scalars["String"];
}>;

export type RenameFileMutation = {
  __typename?: "Mutation";
  renameFile:
    | { __typename?: "File"; id: any; name: string; path: string; revision: number }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type BuildMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  buildableId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type BuildMutation = {
  __typename?: "Mutation";
  build:
    | { __typename?: "BuildState"; projectVersionId: any; success: boolean; buildIds: Array<any> }
    | { __typename?: "OperationInfo" };
};

export type RunMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  arguments: Scalars["JSON"];
}>;

export type RunMutation = {
  __typename?: "Mutation";
  run:
    | { __typename?: "OperationInfo" }
    | {
        __typename?: "RunState";
        projectVersionId: any;
        runnableId?: any | null;
        buildId?: any | null;
        output?: any | null;
        success: boolean;
      };
};

export type CreateStatementMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  fileId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
}>;

export type CreateStatementMutation = {
  __typename?: "Mutation";
  createStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | ({
        __typename?: "Statement";
        id: any;
        type: StatementType;
        symbolType?: SymbolType | null;
        revision: number;
        orderKey: string;
        file: { __typename?: "File"; id: any };
        parent?: { __typename?: "Statement"; id: any } | null;
      } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } });
};

export type MorphStatementMutationVariables = Exact<{
  input: StatementMorphInput;
}>;

export type MorphStatementMutation = {
  __typename?: "Mutation";
  morphStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        type: StatementType;
        symbolType?: SymbolType | null;
        name?: string | null;
        rootTypeTag?: TypeTag | null;
        lang?: string | null;
      };
};

export type UpdateStatementModifierMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  modifier?: InputMaybe<StatementModifier>;
}>;

export type UpdateStatementModifierMutation = {
  __typename?: "Mutation";
  updateStatementModifier:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; modifier?: StatementModifier | null; revision: number };
};

export type MoveStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  fileId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
}>;

export type MoveStatementMutation = {
  __typename?: "Mutation";
  moveStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        orderKey: string;
        revision: number;
        file: { __typename?: "File"; id: any };
        parent?: { __typename?: "Statement"; id: any } | null;
      };
};

export type RenameStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
}>;

export type RenameStatementMutation = {
  __typename?: "Mutation";
  renameStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; name?: string | null; revision: number };
};

export type DeleteStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteStatementMutation = {
  __typename?: "Mutation";
  softDeleteStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        deletedAt?: any | null;
        descendants: Array<{ __typename?: "Statement"; id: any; deletedAt?: any | null }>;
      };
};

export type RestoreStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreStatementMutation = {
  __typename?: "Mutation";
  restoreStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        deletedAt?: any | null;
        descendants: Array<{ __typename?: "Statement"; id: any; deletedAt?: any | null }>;
      };
};

export type CommentStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  commented: Scalars["Boolean"];
}>;

export type CommentStatementMutation = {
  __typename?: "Mutation";
  commentStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        commented: boolean;
        revision: number;
        descendants: Array<{ __typename?: "Statement"; id: any; commented: boolean }>;
      };
};

export type SetReferenceMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type SetReferenceMutation = {
  __typename?: "Mutation";
  updateStatementReference:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        reference?:
          | ({ __typename?: "Statement" } & { " $fragmentRefs"?: { StatementHeaderFragment: StatementHeaderFragment } })
          | null;
      };
};

export type CreateTypeNodeMutationVariables = Exact<{
  typeNode: TypeNodeCreateInput;
}>;

export type CreateTypeNodeMutation = {
  __typename?: "Mutation";
  createStatementTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        typeNodes: Array<
          { __typename?: "SimpleTypeNode" } & {
            " $fragmentRefs"?: { SimpleTypeNodeContentFragment: SimpleTypeNodeContentFragment };
          }
        >;
      };
};

export type DeleteTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type DeleteTypeNodeMutation = {
  __typename?: "Mutation";
  deleteStatementTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        typeNodes: Array<
          { __typename?: "SimpleTypeNode" } & {
            " $fragmentRefs"?: { SimpleTypeNodeContentFragment: SimpleTypeNodeContentFragment };
          }
        >;
      };
};

export type UpdateTypeNodeMutationVariables = Exact<{
  typeNode: TypeNodeUpdateInput;
}>;

export type UpdateTypeNodeMutation = {
  __typename?: "Mutation";
  updateStatementTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        typeNodes: Array<
          { __typename?: "SimpleTypeNode" } & {
            " $fragmentRefs"?: { SimpleTypeNodeContentFragment: SimpleTypeNodeContentFragment };
          }
        >;
      };
};

export type UpdateStatementDescriptionMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  description: Scalars["String"];
}>;

export type UpdateStatementDescriptionMutation = {
  __typename?: "Mutation";
  updateStatementDescription:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; description?: string | null; revision: number };
};

export type UpdateStatementCodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  code?: InputMaybe<Scalars["String"]>;
}>;

export type UpdateStatementCodeMutation = {
  __typename?: "Mutation";
  updateStatementCode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; code?: string | null; revision: number };
};

export type UpdateStatementTextMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  code?: InputMaybe<Scalars["String"]>;
}>;

export type UpdateStatementTextMutation = {
  __typename?: "Mutation";
  updateStatementText:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; code?: string | null; revision: number };
};

export type CreateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  data: Scalars["JSON"];
}>;

export type CreateRecordMutation = {
  __typename?: "Mutation";
  createStatementRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        orderKey: string;
        revision: number;
        records: Array<{ __typename?: "DatasetRecord"; id: any; orderKey: string; data: any }>;
      };
};

export type UpdateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  data: Scalars["JSON"];
}>;

export type UpdateRecordMutation = {
  __typename?: "Mutation";
  updateStatementRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        orderKey: string;
        revision: number;
        records: Array<{ __typename?: "DatasetRecord"; id: any; orderKey: string; data: any }>;
      };
};

export type DeleteRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type DeleteRecordMutation = {
  __typename?: "Mutation";
  deleteStatementRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        orderKey: string;
        revision: number;
        records: Array<{ __typename?: "DatasetRecord"; id: any; orderKey: string; data: any }>;
      };
};

export type LogoutMutationVariables = Exact<{ [key: string]: never }>;

export type LogoutMutation = {
  __typename?: "Mutation";
  logout?:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | null;
};

export type CompleteSignupMutationVariables = Exact<{
  input: UserCompleteSignupInput;
}>;

export type CompleteSignupMutation = {
  __typename?: "Mutation";
  completeSignup:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | ({ __typename?: "User" } & { " $fragmentRefs"?: { UserContentFragment: UserContentFragment } });
};

export type CommitMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
  description?: InputMaybe<Scalars["String"]>;
}>;

export type CommitMutation = {
  __typename?: "Mutation";
  commit:
    | {
        __typename?: "CommitPayload";
        project: { __typename?: "Project" } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } };
        committedVersion: { __typename?: "ProjectVersion" } & {
          " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
        };
        newWorkingVersion: { __typename?: "ProjectVersion" } & {
          " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
        };
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type InterpSymbolContentFragment = {
  __typename?: "InterpSymbol";
  id: any;
  name?: string | null;
  type: StatementType;
  orderKey: string;
  parentId?: any | null;
  modifier?: StatementModifier | null;
  symbolType?: SymbolType | null;
  rootTypeTag?: TypeTag | null;
  generated: boolean;
  typeNodes?: Array<{
    __typename?: "InterpSimpleType";
    id: any;
    name?: string | null;
    tag: TypeTag;
    description?: string | null;
    value?: any | null;
    orderKey: string;
    isOutput: boolean;
    isArray: boolean;
    isNullable: boolean;
    reference?: { __typename?: "Statement"; id: any } | null;
  }> | null;
} & { " $fragmentName"?: "InterpSymbolContentFragment" };

export type InterpModuleContentFragment = {
  __typename?: "InterpModule";
  id: any;
  name: string;
  files: Array<{
    __typename?: "InterpFile";
    id: any;
    path: string;
    symbols: Array<
      { __typename?: "InterpSymbol" } & {
        " $fragmentRefs"?: { InterpSymbolContentFragment: InterpSymbolContentFragment };
      }
    >;
  }>;
} & { " $fragmentName"?: "InterpModuleContentFragment" };

export type InterpErrorContentFragment = {
  __typename?: "InterpError";
  type: ErrorType;
  message: string;
  symbol?:
    | ({ __typename?: "InterpSymbol" } & {
        " $fragmentRefs"?: { InterpSymbolContentFragment: InterpSymbolContentFragment };
      })
    | null;
} & { " $fragmentName"?: "InterpErrorContentFragment" };

export type ModuleRuntimeChangedSubscriptionVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type ModuleRuntimeChangedSubscription = {
  __typename?: "Subscription";
  moduleRuntimeChanged: {
    __typename?: "ModuleRuntime";
    updatedAt: any;
    module: { __typename?: "InterpModule" } & {
      " $fragmentRefs"?: { InterpModuleContentFragment: InterpModuleContentFragment };
    };
    dependencies: Array<
      { __typename?: "InterpModule" } & {
        " $fragmentRefs"?: { InterpModuleContentFragment: InterpModuleContentFragment };
      }
    >;
    errors: Array<
      { __typename?: "InterpError" } & { " $fragmentRefs"?: { InterpErrorContentFragment: InterpErrorContentFragment } }
    >;
  };
};

export const ExecutionContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ExecutionContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Execution" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "root" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "inputs" } },
          { kind: "Field", name: { kind: "Name", value: "outputs" } },
          { kind: "Field", name: { kind: "Name", value: "error" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "code" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "model" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExecutionContentFragment, unknown>;
export const PageInfoFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "PageInfo" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "PageInfo" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "hasNextPage" } },
          { kind: "Field", name: { kind: "Name", value: "hasPreviousPage" } },
          { kind: "Field", name: { kind: "Name", value: "startCursor" } },
          { kind: "Field", name: { kind: "Name", value: "endCursor" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<PageInfoFragment, unknown>;
export const OperationInfoContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "OperationInfoContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "OperationInfo" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "InlineFragment",
            typeCondition: { kind: "NamedType", name: { kind: "Name", value: "OperationInfo" } },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "messages" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "kind" } },
                      { kind: "Field", name: { kind: "Name", value: "message" } },
                      { kind: "Field", name: { kind: "Name", value: "field" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<OperationInfoContentFragment, unknown>;
export const UserContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "UserContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "username" } },
          { kind: "Field", name: { kind: "Name", value: "email" } },
          { kind: "Field", name: { kind: "Name", value: "firstName" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "completedSignup" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "organizations" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
                { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<UserContentFragment, unknown>;
export const ProjectVersionHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "committed" } },
          { kind: "Field", name: { kind: "Name", value: "committedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parents" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectVersionHeaderFragment, unknown>;
export const ProjectHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "head" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "owner" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                    ],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "firstName" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectHeaderFragment, unknown>;
export const FileHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FileHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "path" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "generated" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersion" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<FileHeaderFragment, unknown>;
export const StatementHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "symbolType" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "modifier" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "generated" } },
          { kind: "Field", name: { kind: "Name", value: "commented" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementHeaderFragment, unknown>;
export const TypeContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TypeContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Type" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [{ kind: "Field", name: { kind: "Name", value: "description" } }],
      },
    },
  ],
} as unknown as DocumentNode<TypeContentFragment, unknown>;
export const SimpleTypeNodeContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "SimpleTypeNodeContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "isOutput" } },
          { kind: "Field", name: { kind: "Name", value: "isArray" } },
          { kind: "Field", name: { kind: "Name", value: "isNullable" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SimpleTypeNodeContentFragment, unknown>;
export const StatementContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "symbolType" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "commented" } },
          { kind: "Field", name: { kind: "Name", value: "generated" } },
          { kind: "Field", name: { kind: "Name", value: "modifier" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lang" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "referenceProjectVersion" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNodes" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SimpleTypeNodeContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "records" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                { kind: "Field", name: { kind: "Name", value: "data" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementContentFragment, unknown>;
export const InterpSymbolContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpSymbolContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "InterpSymbol" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "parentId" } },
          { kind: "Field", name: { kind: "Name", value: "modifier" } },
          { kind: "Field", name: { kind: "Name", value: "symbolType" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
          { kind: "Field", name: { kind: "Name", value: "generated" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNodes" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "tag" } },
                { kind: "Field", name: { kind: "Name", value: "description" } },
                { kind: "Field", name: { kind: "Name", value: "value" } },
                { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "reference" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "isOutput" } },
                { kind: "Field", name: { kind: "Name", value: "isArray" } },
                { kind: "Field", name: { kind: "Name", value: "isNullable" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpSymbolContentFragment, unknown>;
export const InterpModuleContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpModuleContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "InterpModule" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "files" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "path" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbols" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpSymbolContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpModuleContentFragment, unknown>;
export const InterpErrorContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpErrorContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "InterpError" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "symbol" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpSymbolContent" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpErrorContentFragment, unknown>;
export const FileContentByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "fileContentById" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "file" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "statements" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isVisible" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...SimpleTypeNodeContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<FileContentByIdQuery, FileContentByIdQueryVariables>;
export const ProjectVersionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "projectVersions" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "versions" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...ProjectVersionHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectVersionsQuery, ProjectVersionsQueryVariables>;
export const OwnerBySlugDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ownerBySlug" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "slug" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ownerBySlug" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "slug" },
                value: { kind: "Variable", name: { kind: "Name", value: "slug" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<OwnerBySlugQuery, OwnerBySlugQueryVariables>;
export const ProjectBySlugDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "projectBySlug" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "owner" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "project" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "projectBySlug" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "owner" },
                value: { kind: "Variable", name: { kind: "Name", value: "owner" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "project" },
                value: { kind: "Variable", name: { kind: "Name", value: "project" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectHeader" } }],
            },
          },
        ],
      },
    },
    ...ProjectHeaderFragmentDoc.definitions,
    ...ProjectVersionHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectBySlugQuery, ProjectBySlugQueryVariables>;
export const ProjectVersionContentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "projectVersionContent" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersion" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "id" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "description" } },
                { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                { kind: "Field", name: { kind: "Name", value: "committed" } },
                { kind: "Field", name: { kind: "Name", value: "committedAt" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "files" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isVisible" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectVersionContentQuery, ProjectVersionContentQueryVariables>;
export const MeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "me" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "me" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "UserContent" } }],
            },
          },
        ],
      },
    },
    ...UserContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<MeQuery, MeQueryVariables>;
export const ProjectMigrationRefsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "projectMigrationRefs" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "afterId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "versions" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "afterId" },
                            value: { kind: "Variable", name: { kind: "Name", value: "afterId" } },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parentsRefs" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "source" } },
                            { kind: "Field", name: { kind: "Name", value: "target" } },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectMigrationRefsQuery, ProjectMigrationRefsQueryVariables>;
export const ExecutionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "executions" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "codeId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "last" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "executions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "codeId" },
                value: { kind: "Variable", name: { kind: "Name", value: "codeId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "first" },
                value: { kind: "Variable", name: { kind: "Name", value: "first" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "last" },
                value: { kind: "Variable", name: { kind: "Name", value: "last" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "totalCount" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "cursor" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionContent" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "descendants" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionContent" } },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "hasNextPage" } },
                      { kind: "Field", name: { kind: "Name", value: "hasPreviousPage" } },
                      { kind: "Field", name: { kind: "Name", value: "startCursor" } },
                      { kind: "Field", name: { kind: "Name", value: "endCursor" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...ExecutionContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ExecutionsQuery, ExecutionsQueryVariables>;
export const ModuleExecutionChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "moduleExecutionChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "moduleExecutionChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionContent" } }],
            },
          },
        ],
      },
    },
    ...ExecutionContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ModuleExecutionChangedSubscription, ModuleExecutionChangedSubscriptionVariables>;
export const CreateFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "directory" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "path" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "parentId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "directory" },
                      value: { kind: "Variable", name: { kind: "Name", value: "directory" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "path" },
                      value: { kind: "Variable", name: { kind: "Name", value: "path" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "projectVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "isVisible" },
                                  value: { kind: "BooleanValue", value: true },
                                },
                              ],
                            },
                          },
                        ],
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } }],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...SimpleTypeNodeContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CreateFileMutation, CreateFileMutationVariables>;
export const DeleteFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "softDeleteFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<DeleteFileMutation, DeleteFileMutationVariables>;
export const RestoreFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "restoreFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RestoreFileMutation, RestoreFileMutationVariables>;
export const RenameFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "renameFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "path" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "renameFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "path" },
                      value: { kind: "Variable", name: { kind: "Name", value: "path" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "path" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RenameFileMutation, RenameFileMutationVariables>;
export const BuildDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "build" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildableId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "build" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "buildableId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "buildableId" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "BuildState" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
                      { kind: "Field", name: { kind: "Name", value: "success" } },
                      { kind: "Field", name: { kind: "Name", value: "buildIds" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<BuildMutation, BuildMutationVariables>;
export const RunDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "run" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "arguments" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "run" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "runnableId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "runnableId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "buildId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "buildId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "arguments" },
                      value: { kind: "Variable", name: { kind: "Name", value: "arguments" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RunState" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
                      { kind: "Field", name: { kind: "Name", value: "runnableId" } },
                      { kind: "Field", name: { kind: "Name", value: "buildId" } },
                      { kind: "Field", name: { kind: "Name", value: "output" } },
                      { kind: "Field", name: { kind: "Name", value: "success" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<RunMutation, RunMutationVariables>;
export const CreateStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "fileId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "parentId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "orderKey" },
                      value: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "symbolType" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "file" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parent" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...StatementContentFragmentDoc.definitions,
    ...SimpleTypeNodeContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CreateStatementMutation, CreateStatementMutationVariables>;
export const MorphStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "morphStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "input" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "StatementMorphInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "morphStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: { kind: "Variable", name: { kind: "Name", value: "input" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "symbolType" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                      { kind: "Field", name: { kind: "Name", value: "lang" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<MorphStatementMutation, MorphStatementMutationVariables>;
export const UpdateStatementModifierDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementModifier" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "modifier" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "StatementModifier" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementModifier" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "modifier" },
                      value: { kind: "Variable", name: { kind: "Name", value: "modifier" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "modifier" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateStatementModifierMutation, UpdateStatementModifierMutationVariables>;
export const MoveStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "moveStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "moveStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "fileId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "parentId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "orderKey" },
                      value: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "file" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parent" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<MoveStatementMutation, MoveStatementMutationVariables>;
export const RenameStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "renameStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "renameStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RenameStatementMutation, RenameStatementMutationVariables>;
export const DeleteStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "softDeleteStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "descendants" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<DeleteStatementMutation, DeleteStatementMutationVariables>;
export const RestoreStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "restoreStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "descendants" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RestoreStatementMutation, RestoreStatementMutationVariables>;
export const CommentStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "commentStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "commented" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "commentStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "commented" },
                      value: { kind: "Variable", name: { kind: "Name", value: "commented" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "commented" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "descendants" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "commented" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CommentStatementMutation, CommentStatementMutationVariables>;
export const SetReferenceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "setReference" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementReference" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "StatementHeader" } }],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...StatementHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<SetReferenceMutation, SetReferenceMutationVariables>;
export const CreateTypeNodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createTypeNode" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "typeNode" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "TypeNodeCreateInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createStatementTypeNode" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: { kind: "Variable", name: { kind: "Name", value: "typeNode" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "typeNodes" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "SimpleTypeNodeContent" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...SimpleTypeNodeContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CreateTypeNodeMutation, CreateTypeNodeMutationVariables>;
export const DeleteTypeNodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteTypeNode" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "deleteStatementTypeNode" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "typeNodes" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "SimpleTypeNodeContent" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...SimpleTypeNodeContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<DeleteTypeNodeMutation, DeleteTypeNodeMutationVariables>;
export const UpdateTypeNodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateTypeNode" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "typeNode" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "TypeNodeUpdateInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementTypeNode" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: { kind: "Variable", name: { kind: "Name", value: "typeNode" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "typeNodes" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "SimpleTypeNodeContent" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...SimpleTypeNodeContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateTypeNodeMutation, UpdateTypeNodeMutationVariables>;
export const UpdateStatementDescriptionDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementDescription" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementDescription" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateStatementDescriptionMutation, UpdateStatementDescriptionMutationVariables>;
export const UpdateStatementCodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementCode" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "code" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementCode" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "code" },
                      value: { kind: "Variable", name: { kind: "Name", value: "code" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateStatementCodeMutation, UpdateStatementCodeMutationVariables>;
export const UpdateStatementTextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementText" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "code" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementText" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "code" },
                      value: { kind: "Variable", name: { kind: "Name", value: "code" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateStatementTextMutation, UpdateStatementTextMutationVariables>;
export const CreateRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createRecord" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "data" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createStatementRecord" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "orderKey" },
                      value: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "data" },
                      value: { kind: "Variable", name: { kind: "Name", value: "data" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                            { kind: "Field", name: { kind: "Name", value: "data" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CreateRecordMutation, CreateRecordMutationVariables>;
export const UpdateRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateRecord" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "data" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementRecord" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "data" },
                      value: { kind: "Variable", name: { kind: "Name", value: "data" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                            { kind: "Field", name: { kind: "Name", value: "data" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateRecordMutation, UpdateRecordMutationVariables>;
export const DeleteRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteRecord" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "deleteStatementRecord" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "id" },
                      value: { kind: "Variable", name: { kind: "Name", value: "id" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                            { kind: "Field", name: { kind: "Name", value: "data" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<DeleteRecordMutation, DeleteRecordMutationVariables>;
export const LogoutDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "logout" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "logout" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } }],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<LogoutMutation, LogoutMutationVariables>;
export const CompleteSignupDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "completeSignup" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "input" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "UserCompleteSignupInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "completeSignup" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: { kind: "Variable", name: { kind: "Name", value: "input" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "UserContent" } }],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...UserContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CompleteSignupMutation, CompleteSignupMutationVariables>;
export const CommitDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "commit" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "commit" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "CommitPayload" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "project" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectHeader" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "committedVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "newWorkingVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...ProjectHeaderFragmentDoc.definitions,
    ...ProjectVersionHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CommitMutation, CommitMutationVariables>;
export const ModuleRuntimeChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "moduleRuntimeChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "moduleRuntimeChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "module" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpModuleContent" } }],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "dependencies" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpModuleContent" } }],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "errors" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpErrorContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...InterpModuleContentFragmentDoc.definitions,
    ...InterpSymbolContentFragmentDoc.definitions,
    ...InterpErrorContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ModuleRuntimeChangedSubscription, ModuleRuntimeChangedSubscriptionVariables>;
