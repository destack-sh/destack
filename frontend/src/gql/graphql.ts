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
  /** Represents NULL values */
  Void: any;
};

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

export type CompileInput = {
  compilationId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
};

export type CompileState = {
  __typename?: "CompileState";
  compilationId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  success: Scalars["Boolean"];
};

export type CompileStateOperationInfo = CompileState | OperationInfo;

export type DatasetRecord = {
  __typename?: "DatasetRecord";
  data: Scalars["JSON"];
  orderKey: Scalars["String"];
};

export enum ErrorType {
  AmbiguousDefinition = "AMBIGUOUS_DEFINITION",
  AmbiguousRequirement = "AMBIGUOUS_REQUIREMENT",
  CompilationMissingModel = "COMPILATION_MISSING_MODEL",
  CompilationMissingTask = "COMPILATION_MISSING_TASK",
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

export type File = Node & {
  __typename?: "File";
  createdAt: Scalars["DateTime"];
  deletedAt?: Maybe<Scalars["DateTime"]>;
  files: Array<File>;
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
  id?: InputMaybe<Scalars["GlobalID"]>;
  isDirectory?: Scalars["Boolean"];
  name: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
};

export type FileFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type FileMoveInput = {
  id: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

export type FileOperationInfo = File | OperationInfo;

export type FileRenameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
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

export type InterpSymbol = {
  __typename?: "InterpSymbol";
  file: InterpFile;
  id: Scalars["GlobalID"];
  modifier?: Maybe<StatementModifier>;
  name?: Maybe<Scalars["String"]>;
  symbolType: SymbolType;
  type: StatementType;
  typeNode?: Maybe<TypeNode>;
};

export type ModuleRuntime = {
  __typename?: "ModuleRuntime";
  dependencies: Array<InterpModule>;
  errors: Array<InterpError>;
  module: InterpModule;
};

export type Mutation = {
  __typename?: "Mutation";
  commentStatement: StatementOperationInfo;
  commit: CommitPayload;
  compile: CompileStateOperationInfo;
  createFile: FileOperationInfo;
  createStatement: StatementOperationInfo;
  createStatementRecord: StatementOperationInfo;
  createStatementTypeNode: StatementOperationInfo;
  deleteStatementTypeNode: StatementOperationInfo;
  morphStatement: StatementOperationInfo;
  moveFile: FileOperationInfo;
  moveStatement: StatementOperationInfo;
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
  updateStatementRecords: StatementOperationInfo;
  updateStatementReference: StatementOperationInfo;
  updateStatementText: StatementOperationInfo;
  updateStatementTypeNode: StatementOperationInfo;
};

export type MutationCommentStatementArgs = {
  input: StatementCommentedInput;
};

export type MutationCommitArgs = {
  input: CommitInput;
};

export type MutationCompileArgs = {
  input: CompileInput;
};

export type MutationCreateFileArgs = {
  input: FileCreateInput;
};

export type MutationCreateStatementArgs = {
  input: StatementCreateInput;
};

export type MutationCreateStatementRecordArgs = {
  input: StatementCreateRecordInput;
};

export type MutationCreateStatementTypeNodeArgs = {
  input: StatementTypeNodeDataCreateInput;
};

export type MutationDeleteStatementTypeNodeArgs = {
  input: StatementTypeNodeDataDeleteInput;
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
  input: StatementUpdateRecordInput;
};

export type MutationUpdateStatementRecordsArgs = {
  input: StatementUpdateRecordsInput;
};

export type MutationUpdateStatementReferenceArgs = {
  input: StatementSetReferenceInput;
};

export type MutationUpdateStatementTextArgs = {
  input: StatementTextInput;
};

export type MutationUpdateStatementTypeNodeArgs = {
  input: StatementTypeNodeDataCreateInput;
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
  organization: Organization;
  path: Scalars["String"];
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  versions: Array<ProjectVersion>;
};

export type ProjectVersionsArgs = {
  filters?: InputMaybe<ProjectVersionFilter>;
};

/** A connection to a list of items. */
export type ProjectConnection = {
  __typename?: "ProjectConnection";
  /** Contains the nodes in this connection */
  edges: Array<ProjectEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type ProjectEdge = {
  __typename?: "ProjectEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Project;
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
  file?: Maybe<File>;
  organization?: Maybe<Organization>;
  organizationBySlug?: Maybe<Organization>;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  projects: ProjectConnection;
  user?: Maybe<User>;
  users: UserConnection;
};

export type QueryFileArgs = {
  id: Scalars["GlobalID"];
};

export type QueryOrganizationArgs = {
  id: Scalars["GlobalID"];
};

export type QueryOrganizationBySlugArgs = {
  organization: Scalars["String"];
};

export type QueryProjectArgs = {
  id: Scalars["GlobalID"];
};

export type QueryProjectBySlugArgs = {
  organization: Scalars["String"];
  project: Scalars["String"];
};

export type QueryProjectVersionArgs = {
  id: Scalars["GlobalID"];
};

export type QueryProjectsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type QueryUserArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUsersArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type RefMapping = {
  __typename?: "RefMapping";
  source: Scalars["GlobalID"];
  target: Scalars["GlobalID"];
};

export type RunInput = {
  arguments: Scalars["JSON"];
  projectVersionId: Scalars["GlobalID"];
  runconfigId: Scalars["GlobalID"];
};

export type RunState = {
  __typename?: "RunState";
  output: Scalars["JSON"];
  projectVersionId: Scalars["GlobalID"];
  runconfigId: Scalars["GlobalID"];
  success: Scalars["Boolean"];
};

export type RunStateOperationInfo = OperationInfo | RunState;

export type SourceMapping = {
  __typename?: "SourceMapping";
  sourceId: Scalars["UUID"];
  sourcePath: Scalars["JSON"];
  sourceRevision: Scalars["Int"];
  targetId: Scalars["UUID"];
  targetPath: Scalars["JSON"];
  targetRevision: Scalars["Int"];
};

export type Statement = Node & {
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
  symbolType?: Maybe<SymbolType>;
  text?: Maybe<Scalars["String"]>;
  type: StatementType;
  typeNodes?: Maybe<Array<TypeNodeData>>;
  updatedAt: Scalars["DateTime"];
  value?: Maybe<Scalars["JSON"]>;
};

export type StatementCommentedInput = {
  commented: Scalars["Boolean"];
  id: Scalars["GlobalID"];
};

/** Create a blank statement */
export type StatementCreateInput = {
  fileId: Scalars["GlobalID"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

export type StatementCreateRecordInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  recordId: Scalars["UUID"];
};

export type StatementFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

/** A modifier to a Bench statement. */
export enum StatementModifier {
  Check = "CHECK",
  Extend = "EXTEND",
  Like = "LIKE",
  Unlike = "UNLIKE",
  Var = "VAR",
  With = "WITH",
}

export type StatementMorphInput = {
  id: Scalars["GlobalID"];
  language?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  symbolType?: InputMaybe<SymbolType>;
  type: StatementType;
  typeNodes?: InputMaybe<Array<StatementTypeNodeDataCreateInput>>;
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

export type StatementTextInput = {
  id: Scalars["GlobalID"];
  text: Scalars["String"];
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

/** Upsert a statement type node data */
export type StatementTypeNodeDataCreateInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  nodeId: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["UUID"]>;
  reference?: InputMaybe<Scalars["String"]>;
  tag: TypeTag;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type StatementTypeNodeDataDeleteInput = {
  id: Scalars["GlobalID"];
  nodeId: Scalars["GlobalID"];
};

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

export type StatementUpdateRecordInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  recordId: Scalars["UUID"];
};

export type StatementUpdateRecordsInput = {
  id: Scalars["GlobalID"];
  records: Array<Scalars["JSON"]>;
};

export type Subscription = {
  __typename?: "Subscription";
  modelExecutionChanged?: Maybe<Scalars["Void"]>;
  moduleRuntimeChanged: ModuleRuntime;
};

export type SubscriptionModelExecutionChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionModuleRuntimeChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

/** The type of symbol content. */
export enum SymbolType {
  Capability = "CAPABILITY",
  Code = "CODE",
  Compilation = "COMPILATION",
  Dataset = "DATASET",
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

export type TypeNode = {
  __typename?: "TypeNode";
  children?: Maybe<Array<TypeNode>>;
  description?: Maybe<Scalars["String"]>;
  name?: Maybe<Scalars["String"]>;
  reference?: Maybe<Scalars["String"]>;
  tag: TypeTag;
};

export type TypeNodeData = {
  __typename?: "TypeNodeData";
  description?: Maybe<Scalars["String"]>;
  id: Scalars["UUID"];
  name?: Maybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  parentId?: Maybe<Scalars["UUID"]>;
  reference?: Maybe<Scalars["String"]>;
  tag: TypeTag;
  value?: Maybe<Scalars["JSON"]>;
};

/** The type of type node. */
export enum TypeTag {
  Any = "ANY",
  Array = "ARRAY",
  Boolean = "BOOLEAN",
  Enum = "ENUM",
  Function = "FUNCTION",
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
}

export type User = Node & {
  __typename?: "User";
  createdAt: Scalars["DateTime"];
  email: Scalars["String"];
  firstName: Scalars["String"];
  id: Scalars["GlobalID"];
  lastName: Scalars["String"];
  organizations: Array<Organization>;
  updatedAt: Scalars["DateTime"];
  /** Required. 150 characters or fewer. Letters, digits and @/./+/-/_ only. */
  username: Scalars["String"];
};

/** A connection to a list of items. */
export type UserConnection = {
  __typename?: "UserConnection";
  /** Contains the nodes in this connection */
  edges: Array<UserEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type UserEdge = {
  __typename?: "UserEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: User;
};

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

export type ProjectBySlugQueryVariables = Exact<{
  organization: Scalars["String"];
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
  projectVersion?:
    | ({ __typename?: "ProjectVersion"; id: any } & {
        " $fragmentRefs"?: { ProjectVersionContentFragment: ProjectVersionContentFragment };
      })
    | null;
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

export type DatasetContentFragment = {
  __typename?: "Statement";
  records: Array<{ __typename?: "DatasetRecord"; data: any }>;
} & { " $fragmentName"?: "DatasetContentFragment" };

export type OperationInfoContentFragment = {
  __typename?: "OperationInfo";
  messages: Array<{
    __typename?: "OperationMessage";
    kind: OperationMessageKind;
    message: string;
    field?: string | null;
  }>;
} & { " $fragmentName"?: "OperationInfoContentFragment" };

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
  organization: { __typename?: "Organization"; slug: string };
} & { " $fragmentName"?: "ProjectHeaderFragment" };

export type ProjectVersionContentFragment = {
  __typename?: "ProjectVersion";
  id: any;
  name?: string | null;
  description?: string | null;
  createdAt: any;
  committed: boolean;
  committedAt?: any | null;
  files: Array<{ __typename?: "File"; id: any } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } }>;
} & { " $fragmentName"?: "ProjectVersionContentFragment" };

export type FileHeaderFragment = {
  __typename?: "File";
  id: any;
  revision: number;
  name: string;
  path: string;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
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

export type TypeNodeDataFragment = {
  __typename?: "TypeNodeData";
  id: any;
  name?: string | null;
  tag: TypeTag;
  description?: string | null;
  value?: any | null;
  parentId?: any | null;
  orderKey: string;
} & { " $fragmentName"?: "TypeNodeDataFragment" };

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
  importPath?: string | null;
  text?: string | null;
  lang?: string | null;
  code?: string | null;
  description?: string | null;
  value?: any | null;
  parent?: { __typename?: "Statement"; id: any } | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  referenceProjectVersion?: { __typename?: "ProjectVersion"; id: any } | null;
  typeNodes?: Array<
    { __typename?: "TypeNodeData" } & { " $fragmentRefs"?: { TypeNodeDataFragment: TypeNodeDataFragment } }
  > | null;
  records: Array<{ __typename?: "DatasetRecord"; data: any }>;
} & { " $fragmentName"?: "StatementContentFragment" };

export type CreateFileMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
}>;

export type CreateFileMutation = {
  __typename?: "Mutation";
  createFile:
    | ({
        __typename?: "File";
        id: any;
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementHeaderFragment: StatementHeaderFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RenameFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
}>;

export type RenameFileMutation = {
  __typename?: "Mutation";
  renameFile:
    | ({ __typename?: "File"; id: any } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
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
    | ({
        __typename?: "File";
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementHeaderFragment: StatementHeaderFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
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
    | ({
        __typename?: "File";
        id: any;
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementHeaderFragment: StatementHeaderFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CompileMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  compilationId: Scalars["GlobalID"];
}>;

export type CompileMutation = {
  __typename?: "Mutation";
  compile: { __typename?: "CompileState"; success: boolean } | { __typename?: "OperationInfo" };
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
    | ({ __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } });
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
        type: StatementType;
        symbolType?: SymbolType | null;
        name?: string | null;
        lang?: string | null;
        typeNodes?: Array<
          { __typename?: "TypeNodeData" } & { " $fragmentRefs"?: { TypeNodeDataFragment: TypeNodeDataFragment } }
        > | null;
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

export type UpdateStatementRecordsMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  records: Array<Scalars["JSON"]> | Scalars["JSON"];
}>;

export type UpdateStatementRecordsMutation = {
  __typename?: "Mutation";
  updateStatementRecords:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        revision: number;
        records: Array<{ __typename?: "DatasetRecord"; data: any }>;
      };
};

export type CommitMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
  description?: InputMaybe<Scalars["String"]>;
}>;

export type CommitMutation = {
  __typename?: "Mutation";
  commit: {
    __typename?: "CommitPayload";
    project: { __typename?: "Project" } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } };
    committedVersion: { __typename?: "ProjectVersion" } & {
      " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
    };
    newWorkingVersion: { __typename?: "ProjectVersion" } & {
      " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
    };
  };
};

export type TypeNodeContentInnerFragment = {
  __typename?: "TypeNode";
  name?: string | null;
  tag: TypeTag;
  description?: string | null;
  reference?: string | null;
} & { " $fragmentName"?: "TypeNodeContentInnerFragment" };

export type TypeNodeContentFragment = ({
  __typename?: "TypeNode";
  children?: Array<
    {
      __typename?: "TypeNode";
      children?: Array<
        {
          __typename?: "TypeNode";
          children?: Array<
            { __typename?: "TypeNode" } & {
              " $fragmentRefs"?: { TypeNodeContentInnerFragment: TypeNodeContentInnerFragment };
            }
          > | null;
        } & { " $fragmentRefs"?: { TypeNodeContentInnerFragment: TypeNodeContentInnerFragment } }
      > | null;
    } & { " $fragmentRefs"?: { TypeNodeContentInnerFragment: TypeNodeContentInnerFragment } }
  > | null;
} & { " $fragmentRefs"?: { TypeNodeContentInnerFragment: TypeNodeContentInnerFragment } }) & {
  " $fragmentName"?: "TypeNodeContentFragment";
};

export type InterpSymbolContentFragment = {
  __typename?: "InterpSymbol";
  id: any;
  name?: string | null;
  type: StatementType;
  modifier?: StatementModifier | null;
  symbolType: SymbolType;
  typeNode?:
    | ({ __typename?: "TypeNode" } & { " $fragmentRefs"?: { TypeNodeContentFragment: TypeNodeContentFragment } })
    | null;
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

export const DatasetContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "DatasetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "records" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "data" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<DatasetContentFragment, unknown>;
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
            name: { kind: "Name", value: "organization" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "slug" } }],
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
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
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
export const ProjectVersionContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionContent" },
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
} as unknown as DocumentNode<ProjectVersionContentFragment, unknown>;
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
export const TypeNodeDataFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TypeNodeData" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "TypeNodeData" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "parentId" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TypeNodeDataFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "importPath" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNodes" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeData" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "records" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "data" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementContentFragment, unknown>;
export const TypeNodeContentInnerFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TypeNodeContentInner" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "TypeNode" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "reference" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TypeNodeContentInnerFragment, unknown>;
export const TypeNodeContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TypeNodeContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "TypeNode" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeContentInner" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeContentInner" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "children" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeContentInner" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "children" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeContentInner" } },
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
} as unknown as DocumentNode<TypeNodeContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "modifier" } },
          { kind: "Field", name: { kind: "Name", value: "symbolType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNode" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeContent" } }],
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
    ...TypeNodeDataFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "organization" } },
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
                name: { kind: "Name", value: "organization" },
                value: { kind: "Variable", name: { kind: "Name", value: "organization" } },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionContent" } },
              ],
            },
          },
        ],
      },
    },
    ...ProjectVersionContentFragmentDoc.definitions,
    ...FileHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectVersionContentQuery, ProjectVersionContentQueryVariables>;
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
    ...FileHeaderFragmentDoc.definitions,
    ...StatementHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CreateFileMutation, CreateFileMutationVariables>;
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
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RenameFileMutation, RenameFileMutationVariables>;
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
                      { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
    ...FileHeaderFragmentDoc.definitions,
    ...StatementHeaderFragmentDoc.definitions,
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
                      { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
    ...FileHeaderFragmentDoc.definitions,
    ...StatementHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RestoreFileMutation, RestoreFileMutationVariables>;
export const CompileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "compile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "compilationId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "compile" },
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
                      name: { kind: "Name", value: "compilationId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "compilationId" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "CompileState" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "success" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CompileMutation, CompileMutationVariables>;
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
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } }],
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
    ...TypeNodeDataFragmentDoc.definitions,
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
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "symbolType" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "typeNodes" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TypeNodeData" } }],
                        },
                      },
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
    ...TypeNodeDataFragmentDoc.definitions,
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
export const UpdateStatementRecordsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementRecords" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "records" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatementRecords" },
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
                      name: { kind: "Name", value: "records" },
                      value: { kind: "Variable", name: { kind: "Name", value: "records" } },
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
                        name: { kind: "Name", value: "records" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "data" } }],
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
} as unknown as DocumentNode<UpdateStatementRecordsMutation, UpdateStatementRecordsMutationVariables>;
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
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "newWorkingVersion" },
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
    ...ProjectHeaderFragmentDoc.definitions,
    ...ProjectVersionHeaderFragmentDoc.definitions,
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
    ...TypeNodeContentFragmentDoc.definitions,
    ...TypeNodeContentInnerFragmentDoc.definitions,
    ...InterpErrorContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ModuleRuntimeChangedSubscription, ModuleRuntimeChangedSubscriptionVariables>;
