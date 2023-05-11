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

export type AccessToken = Node & {
  __typename?: "AccessToken";
  createdAt: Scalars["DateTime"];
  expiresAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  owner: UserOrganization;
  revokedAt?: Maybe<Scalars["DateTime"]>;
  scopes: Array<AccessTokenScope>;
  status: AccessTokenStatus;
  token?: Maybe<Scalars["String"]>;
  tokenKey: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type AccessTokenConnection = {
  __typename?: "AccessTokenConnection";
  /** Contains the nodes in this connection */
  edges: Array<AccessTokenEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

export type AccessTokenCreateInput = {
  expiresAt?: InputMaybe<Scalars["DateTime"]>;
  name?: InputMaybe<Scalars["String"]>;
  ownerId: Scalars["GlobalID"];
  scopes: Array<AccessTokenScope>;
};

export type AccessTokenCreatePayload = {
  __typename?: "AccessTokenCreatePayload";
  accessToken: AccessToken;
  token: Scalars["String"];
};

export type AccessTokenCreatePayloadOperationInfo = AccessTokenCreatePayload | OperationInfo;

/** An edge in a connection. */
export type AccessTokenEdge = {
  __typename?: "AccessTokenEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: AccessToken;
};

export type AccessTokenFilter = {
  includeInactive?: InputMaybe<Scalars["Boolean"]>;
};

export type AccessTokenOperationInfo = AccessToken | OperationInfo;

export enum AccessTokenScope {
  Run = "RUN",
}

export enum AccessTokenStatus {
  Active = "ACTIVE",
  Expired = "EXPIRED",
  Revoked = "REVOKED",
}

export type BuildCandidate = Node & {
  __typename?: "BuildCandidate";
  build: Statement;
  createdAt: Scalars["DateTime"];
  file: File;
  id: Scalars["GlobalID"];
  project: Project;
  projectVersion: ProjectVersion;
  status: BuildCandidateStatus;
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type BuildCandidateConnection = {
  __typename?: "BuildCandidateConnection";
  /** Contains the nodes in this connection */
  edges: Array<BuildCandidateEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type BuildCandidateEdge = {
  __typename?: "BuildCandidateEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: BuildCandidate;
};

export enum BuildCandidateStatus {
  Building = "Building",
  Cancelled = "Cancelled",
  CompletedAbandoned = "CompletedAbandoned",
  CompletedWon = "CompletedWon",
  Evaluating = "Evaluating",
  Planned = "Planned",
}

export type BuildInput = {
  buildableId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  scope: BuildScope;
};

export enum BuildScope {
  All = "ALL",
  Reactive = "REACTIVE",
  Selected = "SELECTED",
}

export type BuildSettings = Node & {
  __typename?: "BuildSettings";
  id: Scalars["GlobalID"];
  reactive: Scalars["Boolean"];
  statement?: Maybe<Statement>;
};

export type BuildState = {
  __typename?: "BuildState";
  projectVersionId: Scalars["GlobalID"];
  success: Scalars["Boolean"];
};

export type BuildStateOperationInfo = BuildState | OperationInfo;

export type Client = Node & {
  __typename?: "Client";
  active: Scalars["Boolean"];
  browserName?: Maybe<Scalars["String"]>;
  closedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  deviceName?: Maybe<Scalars["String"]>;
  file?: Maybe<File>;
  id: Scalars["GlobalID"];
  lastSeenAt?: Maybe<Scalars["DateTime"]>;
  lock?: Maybe<Lock>;
  path?: Maybe<Scalars["String"]>;
  present: Scalars["Boolean"];
  project?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  record?: Maybe<DatasetRecord>;
  statement?: Maybe<Statement>;
  type: ClientType;
  typeNode?: Maybe<SimpleTypeNode>;
  updatedAt: Scalars["DateTime"];
  user: User;
};

/** A connection to a list of items. */
export type ClientConnection = {
  __typename?: "ClientConnection";
  /** Contains the nodes in this connection */
  edges: Array<ClientEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type ClientEdge = {
  __typename?: "ClientEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Client;
};

export type ClientOperationInfo = Client | OperationInfo;

/** The type of device/client. */
export enum ClientType {
  DesktopBrowser = "DesktopBrowser",
  MobileBrowser = "MobileBrowser",
}

export type ClientUpsertInput = {
  browserName?: InputMaybe<Scalars["String"]>;
  deviceName?: InputMaybe<Scalars["String"]>;
  fileId?: InputMaybe<Scalars["GlobalID"]>;
  id: Scalars["GlobalID"];
  path?: InputMaybe<Scalars["String"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  recordId?: InputMaybe<Scalars["GlobalID"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]>;
  type: ClientType;
  typeNodeId?: InputMaybe<Scalars["GlobalID"]>;
};

export type CommitInput = {
  autoDeploy?: Scalars["Boolean"];
  description?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  projectVersionId: Scalars["GlobalID"];
  tag?: InputMaybe<Scalars["String"]>;
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
  deletedAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  revision: Scalars["Int"];
  statement: Statement;
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type DatasetRecordConnection = {
  __typename?: "DatasetRecordConnection";
  /** Contains the nodes in this connection */
  edges: Array<DatasetRecordEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type DatasetRecordEdge = {
  __typename?: "DatasetRecordEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: DatasetRecord;
};

export type DatasetRecordFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type DatasetRecordOperationInfo = DatasetRecord | OperationInfo;

export type DeleteObjectInput = {
  id: Scalars["GlobalID"];
};

export type DeployInput = {
  id: Scalars["GlobalID"];
  status: DeploymentStatus;
};

export type Deployment = Node & {
  __typename?: "Deployment";
  createdAt: Scalars["DateTime"];
  deployAllStatements: Scalars["Boolean"];
  deployedStatements: Array<Statement>;
  id: Scalars["GlobalID"];
  owner: OrganizationUser;
  project: Project;
  projectVersion: ProjectVersion;
  status: DeploymentStatus;
  type: DeploymentType;
  updatedAt: Scalars["DateTime"];
};

export type DeploymentAddStatementInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["ID"];
};

/** A connection to a list of items. */
export type DeploymentConnection = {
  __typename?: "DeploymentConnection";
  /** Contains the nodes in this connection */
  edges: Array<DeploymentEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type DeploymentEdge = {
  __typename?: "DeploymentEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Deployment;
};

export type DeploymentFilter = {
  isOwned?: InputMaybe<Scalars["Boolean"]>;
};

export type DeploymentOperationInfo = Deployment | OperationInfo;

export type DeploymentRemoveStatementInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["ID"];
};

export type DeploymentSetDeployAllStatementsInput = {
  deployAllStatements: Scalars["Boolean"];
  id: Scalars["GlobalID"];
};

export enum DeploymentStatus {
  Active = "ACTIVE",
  Archived = "ARCHIVED",
  Destroyed = "DESTROYED",
  Inactive = "INACTIVE",
  Sleeping = "SLEEPING",
}

export enum DeploymentType {
  Adhoc = "ADHOC",
  Manual = "MANUAL",
}

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

export type EvaluateSettings = Node & {
  __typename?: "EvaluateSettings";
  id: Scalars["GlobalID"];
  reactive: Scalars["Boolean"];
  statement?: Maybe<Statement>;
  weights: Scalars["JSON"];
};

export enum EvaluationKind {
  Evaluation = "EVALUATION",
  Lint = "LINT",
}

export type EvaluationResult = Node & {
  __typename?: "EvaluationResult";
  aggregatedMetrics: Scalars["JSON"];
  build?: Maybe<Statement>;
  createdAt: Scalars["DateTime"];
  file: File;
  id: Scalars["GlobalID"];
  kind: EvaluationKind;
  project: Project;
  projectVersion: ProjectVersion;
  record?: Maybe<DatasetRecord>;
  scope: EvaluationScope;
  selfMetrics?: Maybe<Scalars["JSON"]>;
  statement?: Maybe<Statement>;
  typeNode?: Maybe<SimpleTypeNode>;
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type EvaluationResultConnection = {
  __typename?: "EvaluationResultConnection";
  /** Contains the nodes in this connection */
  edges: Array<EvaluationResultEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type EvaluationResultEdge = {
  __typename?: "EvaluationResultEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: EvaluationResult;
};

export enum EvaluationScope {
  Build = "BUILD",
  Instruction = "INSTRUCTION",
  Module = "MODULE",
}

export type Execution = Node & {
  __typename?: "Execution";
  accessToken?: Maybe<AccessToken>;
  build?: Maybe<Statement>;
  cachedDuration?: Maybe<Scalars["Float"]>;
  cachedGeneratedAt?: Maybe<Scalars["DateTime"]>;
  code?: Maybe<Statement>;
  createdAt: Scalars["DateTime"];
  deployment?: Maybe<Deployment>;
  descendants: Array<Execution>;
  duration?: Maybe<Scalars["Float"]>;
  error?: Maybe<Scalars["JSON"]>;
  id: Scalars["GlobalID"];
  inputs?: Maybe<Scalars["JSON"]>;
  model?: Maybe<Statement>;
  outputs?: Maybe<Scalars["JSON"]>;
  parent?: Maybe<Execution>;
  project: Project;
  projectVersion: ProjectVersion;
  root?: Maybe<Execution>;
  /** Time of transition to Running status. */
  startedAt?: Maybe<Scalars["DateTime"]>;
  status: ExecutionStatus;
  task?: Maybe<Statement>;
  /** Time of transition to a terminal status. */
  terminatedAt?: Maybe<Scalars["DateTime"]>;
  triggerType: ExecutionTriggerType;
  updatedAt: Scalars["DateTime"];
  user?: Maybe<User>;
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

export enum ExecutionTracingLevel {
  AllFrames = "ALL_FRAMES",
  AllFramesWithData = "ALL_FRAMES_WITH_DATA",
  RootFrame = "ROOT_FRAME",
  RootFrameWithData = "ROOT_FRAME_WITH_DATA",
}

export enum ExecutionTriggerType {
  Job = "JOB",
  Manual = "MANUAL",
  RestApi = "REST_API",
  UiInteractive = "UI_INTERACTIVE",
}

export type File = Node & {
  __typename?: "File";
  createdAt: Scalars["DateTime"];
  deletedAt?: Maybe<Scalars["DateTime"]>;
  directory: Scalars["Boolean"];
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

/** A connection to a list of items. */
export type FileConnection = {
  __typename?: "FileConnection";
  /** Contains the nodes in this connection */
  edges: Array<FileEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

export type FileCreateInput = {
  directory?: Scalars["Boolean"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  name: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  path: Scalars["String"];
  projectVersionId: Scalars["GlobalID"];
};

/** An edge in a connection. */
export type FileEdge = {
  __typename?: "FileEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: File;
};

export type FileFilter = {
  isGenerated?: InputMaybe<Scalars["Boolean"]>;
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

export type GeneratedMapping = Node & {
  __typename?: "GeneratedMapping";
  id: Scalars["GlobalID"];
  sourceId?: Maybe<Scalars["GlobalID"]>;
  sourceRevision?: Maybe<Scalars["Int"]>;
  statement: Statement;
  targetId?: Maybe<Scalars["GlobalID"]>;
  targetRevision?: Maybe<Scalars["Int"]>;
};

export type GeneratedMappingFilter = {
  typeIn?: InputMaybe<Array<GeneratedMappingType>>;
};

export enum GeneratedMappingType {
  Record = "RECORD",
  Statement = "STATEMENT",
  TypeNode = "TYPE_NODE",
  Xblock = "XBLOCK",
}

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
  dependencies: Array<InterpModule>;
  errors: Array<InterpError>;
  files: Array<InterpFile>;
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  staleSymbols: Array<InterpSymbol>;
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
    deletedAt?: Maybe<Scalars["DateTime"]>;
    description?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    isArray: Scalars["Boolean"];
    isNullable: Scalars["Boolean"];
    isOutput: Scalars["Boolean"];
    key: Scalars["String"];
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    reference?: Maybe<Statement>;
    revision: Scalars["Int"];
    statement: Statement;
    tag: TypeTag;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

export type InterpSymbol = SimplyTyped & {
  __typename?: "InterpSymbol";
  availableBuilds?: Maybe<Array<Scalars["GlobalID"]>>;
  file: InterpFile;
  fqn: Scalars["String"];
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

export type Job = Node & {
  __typename?: "Job";
  buildCandidate?: Maybe<BuildCandidate>;
  createdAt: Scalars["DateTime"];
  deployment: Deployment;
  id: Scalars["GlobalID"];
  parent?: Maybe<Job>;
  project: Project;
  projectVersion: ProjectVersion;
  startedAt?: Maybe<Scalars["DateTime"]>;
  status: JobStatus;
  terminatedAt?: Maybe<Scalars["DateTime"]>;
  type: JobType;
  updatedAt: Scalars["DateTime"];
};

/** A connection to a list of items. */
export type JobConnection = {
  __typename?: "JobConnection";
  /** Contains the nodes in this connection */
  edges: Array<JobEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type JobEdge = {
  __typename?: "JobEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Job;
};

export enum JobStatus {
  Cancelled = "Cancelled",
  Cancelling = "Cancelling",
  Completed = "Completed",
  Failed = "Failed",
  Queued = "Queued",
  Running = "Running",
}

export enum JobType {
  Build = "BUILD",
  Evaluate = "EVALUATE",
  Generate = "GENERATE",
  Interp = "INTERP",
  Lint = "LINT",
}

export type Lock = Node & {
  __typename?: "Lock";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  path: Scalars["String"];
  projectVersion: ProjectVersion;
  record?: Maybe<DatasetRecord>;
  statement?: Maybe<Statement>;
  typeNode?: Maybe<SimpleTypeNode>;
  updatedAt: Scalars["DateTime"];
};

export type ModuleChange = {
  __typename?: "ModuleChange";
  clientId?: Maybe<Scalars["GlobalID"]>;
  id: Scalars["UUID"];
  mutations: Array<ModuleMutation>;
};

export type ModuleMutation = {
  __typename?: "ModuleMutation";
  fileId?: Maybe<Scalars["GlobalID"]>;
  input?: Maybe<Scalars["JSON"]>;
  projectVersionId: Scalars["GlobalID"];
  revision?: Maybe<Scalars["Int"]>;
  statementId?: Maybe<Scalars["GlobalID"]>;
  type: ModuleMutationType;
};

/** Fine-grained atomic mutations for multiplayer modules. */
export enum ModuleMutationType {
  CommentStatement = "COMMENT_STATEMENT",
  Commit = "COMMIT",
  CreateFile = "CREATE_FILE",
  CreateRecord = "CREATE_RECORD",
  CreateStatement = "CREATE_STATEMENT",
  CreateStatementBlank = "CREATE_STATEMENT_BLANK",
  CreateTypeNode = "CREATE_TYPE_NODE",
  CreateXblock = "CREATE_XBLOCK",
  DeleteFile = "DELETE_FILE",
  DeleteRecord = "DELETE_RECORD",
  DeleteStatement = "DELETE_STATEMENT",
  DeleteTypeNode = "DELETE_TYPE_NODE",
  DeleteXblock = "DELETE_XBLOCK",
  MorphStatement = "MORPH_STATEMENT",
  MoveFile = "MOVE_FILE",
  MoveRecord = "MOVE_RECORD",
  MoveStatement = "MOVE_STATEMENT",
  MoveTypeNode = "MOVE_TYPE_NODE",
  PasteFile = "PASTE_FILE",
  PasteStatement = "PASTE_STATEMENT",
  RenameFile = "RENAME_FILE",
  RenameStatement = "RENAME_STATEMENT",
  RenameTypeNode = "RENAME_TYPE_NODE",
  RestoreFile = "RESTORE_FILE",
  RestoreRecord = "RESTORE_RECORD",
  RestoreStatement = "RESTORE_STATEMENT",
  RestoreTypeNode = "RESTORE_TYPE_NODE",
  SoftDeleteFile = "SOFT_DELETE_FILE",
  SoftDeleteRecord = "SOFT_DELETE_RECORD",
  SoftDeleteStatement = "SOFT_DELETE_STATEMENT",
  SoftDeleteTypeNode = "SOFT_DELETE_TYPE_NODE",
  TruncateRecords = "TRUNCATE_RECORDS",
  UpdateFile = "UPDATE_FILE",
  UpdateGeneratedMappings = "UPDATE_GENERATED_MAPPINGS",
  UpdateRecord = "UPDATE_RECORD",
  UpdateRecordPath = "UPDATE_RECORD_PATH",
  UpdateStatement = "UPDATE_STATEMENT",
  UpdateStatementCode = "UPDATE_STATEMENT_CODE",
  UpdateStatementDescription = "UPDATE_STATEMENT_DESCRIPTION",
  UpdateStatementLanguage = "UPDATE_STATEMENT_LANGUAGE",
  UpdateStatementModifier = "UPDATE_STATEMENT_MODIFIER",
  UpdateStatementReference = "UPDATE_STATEMENT_REFERENCE",
  UpdateStatementText = "UPDATE_STATEMENT_TEXT",
  UpdateTypeNode = "UPDATE_TYPE_NODE",
  UpdateTypeNodeDescription = "UPDATE_TYPE_NODE_DESCRIPTION",
  UpdateTypeNodeType = "UPDATE_TYPE_NODE_TYPE",
}

export type Mutation = {
  __typename?: "Mutation";
  acceptOrganizationInvite: UserOperationInfo;
  addDeployedStatement: DeploymentOperationInfo;
  batchCommentStatement: StatementBatchOperationInfo;
  batchMoveStatement: StatementBatchOperationInfo;
  batchPasteStatement: StatementBatchOperationInfo;
  batchRestoreRecord: RecordBatchOperationInfo;
  batchRestoreStatement: StatementBatchOperationInfo;
  batchSoftDeleteRecord: RecordBatchOperationInfo;
  batchSoftDeleteStatement: StatementBatchOperationInfo;
  build: BuildStateOperationInfo;
  cancelOrganizationInvite: OrganizationOperationInfo;
  closeClient: ClientOperationInfo;
  commentStatement: StatementOperationInfo;
  commit: CommitPayloadOperationInfo;
  completeSignup: UserOperationInfo;
  createAccessToken: AccessTokenCreatePayloadOperationInfo;
  createFile: FileOperationInfo;
  createOrganization: OrganizationOperationInfo;
  createOrganizationInvites: OrganizationOperationInfo;
  createProject: ProjectOperationInfo;
  createRecord: DatasetRecordOperationInfo;
  createStatement: StatementOperationInfo;
  createStatementBlank: StatementOperationInfo;
  createTypeNode: SimpleTypeNodeOperationInfo;
  deleteFile: FileOperationInfo;
  deleteObject: RemoteObjectOperationInfo;
  deleteRecord: DatasetRecordOperationInfo;
  deleteStatement: StatementOperationInfo;
  deleteTypeNode: SimpleTypeNodeOperationInfo;
  logout?: Maybe<OperationInfo>;
  markNotification: NotificationOperationInfo;
  morphStatement: StatementOperationInfo;
  moveFile: FileOperationInfo;
  moveRecord: DatasetRecordOperationInfo;
  moveStatement: StatementOperationInfo;
  moveTypeNode: SimpleTypeNodeOperationInfo;
  notifyUploadedObject: RemoteObjectOperationInfo;
  removeDeployedStatement: DeploymentOperationInfo;
  removeOrganizationMembership: OrganizationOperationInfo;
  renameFile: FileOperationInfo;
  renameStatement: StatementOperationInfo;
  requestUploadObject: RemoteObjectOperationInfo;
  restore: CommitPayloadOperationInfo;
  restoreFile: FileOperationInfo;
  restoreRecord: DatasetRecordOperationInfo;
  restoreStatement: StatementOperationInfo;
  restoreStatementTypeNode: SimpleTypeNodeOperationInfo;
  revokeAccessToken: AccessTokenOperationInfo;
  run: RunStateOperationInfo;
  secretRootLogin: UserOperationInfo;
  setDeployAllStatements: DeploymentOperationInfo;
  softDeleteFile: FileOperationInfo;
  softDeleteRecord: DatasetRecordOperationInfo;
  softDeleteStatement: StatementOperationInfo;
  softDeleteTypeNode: SimpleTypeNodeOperationInfo;
  truncateRecords: StatementOperationInfo;
  updateDeployment: DeploymentOperationInfo;
  updateFile: FileOperationInfo;
  updateOrganization: OrganizationOperationInfo;
  updateOrganizationMembership: OrganizationMembershipOperationInfo;
  updatePresence: ClientOperationInfo;
  updateProjectName: ProjectOperationInfo;
  updateProjectVersion: ProjectVersionOperationInfo;
  updateProjectVisibility: ProjectOperationInfo;
  updateRecord: DatasetRecordOperationInfo;
  updateRecordPath: DatasetRecordOperationInfo;
  updateStatement: StatementOperationInfo;
  updateStatementCode: StatementOperationInfo;
  updateStatementDescription: StatementOperationInfo;
  updateStatementLanguage: StatementOperationInfo;
  updateStatementModifier: StatementOperationInfo;
  updateStatementReference: StatementOperationInfo;
  updateStatementText: StatementOperationInfo;
  updateTypeNode: SimpleTypeNodeOperationInfo;
  updateTypeNodeDescription: SimpleTypeNodeOperationInfo;
  updateTypeNodeName: SimpleTypeNodeOperationInfo;
  updateTypeNodeType: SimpleTypeNodeOperationInfo;
  updateUser: UserOperationInfo;
  upsertClient: ClientOperationInfo;
};

export type MutationAcceptOrganizationInviteArgs = {
  id: Scalars["GlobalID"];
};

export type MutationAddDeployedStatementArgs = {
  input: DeploymentAddStatementInput;
};

export type MutationBatchCommentStatementArgs = {
  input: StatementBatchCommentedInput;
};

export type MutationBatchMoveStatementArgs = {
  input: StatementBatchMoveInput;
};

export type MutationBatchPasteStatementArgs = {
  input: StatementBatchPasteInput;
};

export type MutationBatchRestoreRecordArgs = {
  input: RecordBatchRestoreInput;
};

export type MutationBatchRestoreStatementArgs = {
  input: StatementBatchRestoreInput;
};

export type MutationBatchSoftDeleteRecordArgs = {
  input: RecordBatchSoftDeleteInput;
};

export type MutationBatchSoftDeleteStatementArgs = {
  input: StatementBatchSoftDeleteInput;
};

export type MutationBuildArgs = {
  input: BuildInput;
};

export type MutationCancelOrganizationInviteArgs = {
  id: Scalars["GlobalID"];
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

export type MutationCreateAccessTokenArgs = {
  input: AccessTokenCreateInput;
};

export type MutationCreateFileArgs = {
  input: FileCreateInput;
};

export type MutationCreateOrganizationArgs = {
  input: OrganizationCreateInput;
};

export type MutationCreateOrganizationInvitesArgs = {
  input: OrganizationInviteInput;
};

export type MutationCreateProjectArgs = {
  input: ProjectCreateInput;
};

export type MutationCreateRecordArgs = {
  input: RecordCreateInput;
};

export type MutationCreateStatementArgs = {
  input: StatementCreateInput;
};

export type MutationCreateStatementBlankArgs = {
  input: StatementCreateBlankInput;
};

export type MutationCreateTypeNodeArgs = {
  input: TypeNodeCreateInput;
};

export type MutationDeleteFileArgs = {
  input: NodeInput;
};

export type MutationDeleteObjectArgs = {
  input: DeleteObjectInput;
};

export type MutationDeleteRecordArgs = {
  input: RecordDeleteInput;
};

export type MutationDeleteStatementArgs = {
  input: StatementDeleteInput;
};

export type MutationDeleteTypeNodeArgs = {
  input: TypeNodeDeleteInput;
};

export type MutationMarkNotificationArgs = {
  input: NotificationMarkInput;
};

export type MutationMorphStatementArgs = {
  input: StatementMorphInput;
};

export type MutationMoveFileArgs = {
  input: FileMoveInput;
};

export type MutationMoveRecordArgs = {
  input: RecordMoveInput;
};

export type MutationMoveStatementArgs = {
  input: StatementMoveInput;
};

export type MutationMoveTypeNodeArgs = {
  input: TypeNodeMoveInput;
};

export type MutationNotifyUploadedObjectArgs = {
  input: NotifyUploadedObjectInput;
};

export type MutationRemoveDeployedStatementArgs = {
  input: DeploymentRemoveStatementInput;
};

export type MutationRemoveOrganizationMembershipArgs = {
  input: OrganizationRemoveMembershipInput;
};

export type MutationRenameFileArgs = {
  input: FileRenameInput;
};

export type MutationRenameStatementArgs = {
  input: StatementRenameInput;
};

export type MutationRequestUploadObjectArgs = {
  input: RequestUploadObjectInput;
};

export type MutationRestoreArgs = {
  input: RestoreInput;
};

export type MutationRestoreFileArgs = {
  input: NodeInput;
};

export type MutationRestoreRecordArgs = {
  input: RecordRestoreInput;
};

export type MutationRestoreStatementArgs = {
  input: StatementRestoreInput;
};

export type MutationRestoreStatementTypeNodeArgs = {
  input: TypeNodeRestoreInput;
};

export type MutationRevokeAccessTokenArgs = {
  id: Scalars["GlobalID"];
};

export type MutationRunArgs = {
  input: RunInput;
};

export type MutationSecretRootLoginArgs = {
  username: Scalars["String"];
};

export type MutationSetDeployAllStatementsArgs = {
  input: DeploymentSetDeployAllStatementsInput;
};

export type MutationSoftDeleteFileArgs = {
  input: NodeInput;
};

export type MutationSoftDeleteRecordArgs = {
  input: RecordDeleteInput;
};

export type MutationSoftDeleteStatementArgs = {
  input: StatementSoftDeleteInput;
};

export type MutationSoftDeleteTypeNodeArgs = {
  input: TypeNodeDeleteInput;
};

export type MutationTruncateRecordsArgs = {
  input: RecordTruncateInput;
};

export type MutationUpdateDeploymentArgs = {
  input: DeployInput;
};

export type MutationUpdateFileArgs = {
  input: FileCreateInput;
};

export type MutationUpdateOrganizationArgs = {
  input: OrganizationUpdateInput;
};

export type MutationUpdateOrganizationMembershipArgs = {
  input: OrganizationUpdateMembershipInput;
};

export type MutationUpdateProjectNameArgs = {
  input: ProjectUpdateNameInput;
};

export type MutationUpdateProjectVersionArgs = {
  input: UpdateProjectVersion;
};

export type MutationUpdateProjectVisibilityArgs = {
  input: ProjectUpdateVisibilityInput;
};

export type MutationUpdateRecordArgs = {
  input: RecordUpdateInput;
};

export type MutationUpdateRecordPathArgs = {
  input: RecordUpdatePathInput;
};

export type MutationUpdateStatementArgs = {
  input: StatementCreateInput;
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

export type MutationUpdateStatementReferenceArgs = {
  input: StatementSetReferenceInput;
};

export type MutationUpdateStatementTextArgs = {
  input: StatementUpdateCodeInput;
};

export type MutationUpdateTypeNodeArgs = {
  input: TypeNodeUpdateInput;
};

export type MutationUpdateTypeNodeDescriptionArgs = {
  input: TypeNodeUpdateDescriptionInput;
};

export type MutationUpdateTypeNodeNameArgs = {
  input: TypeNodeRenameInput;
};

export type MutationUpdateTypeNodeTypeArgs = {
  input: TypeNodeUpdateTypeInput;
};

export type MutationUpdateUserArgs = {
  input: UserUpdateInput;
};

export type MutationUpsertClientArgs = {
  input: ClientUpsertInput;
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

export type Notification = Node & {
  __typename?: "Notification";
  archivedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  expiresAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  invite: OrganizationInvite;
  readAt?: Maybe<Scalars["DateTime"]>;
  status: NotificationStatus;
  type: NotificationType;
  user: User;
};

/** A connection to a list of items. */
export type NotificationConnection = {
  __typename?: "NotificationConnection";
  /** Contains the nodes in this connection */
  edges: Array<NotificationEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type NotificationEdge = {
  __typename?: "NotificationEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Notification;
};

export type NotificationFilter = {
  createdAt_Gte?: InputMaybe<Scalars["DateTime"]>;
  notArchived?: InputMaybe<Scalars["Boolean"]>;
  status?: InputMaybe<NotificationStatus>;
};

export type NotificationMarkInput = {
  id: Scalars["GlobalID"];
  status: NotificationStatus;
};

export type NotificationOperationInfo = Notification | OperationInfo;

export enum NotificationStatus {
  Active = "ACTIVE",
  Archived = "ARCHIVED",
  Expired = "EXPIRED",
  Read = "READ",
}

export enum NotificationType {
  OrganizationInvite = "ORGANIZATION_INVITE",
}

export type NotifyUploadedObjectInput = {
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

export type Organization = Node &
  Owner & {
    __typename?: "Organization";
    accessTokens: AccessTokenConnection;
    canViewFull: Scalars["Boolean"];
    canWrite: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    invites: OrganizationInviteConnection;
    members: UserConnection;
    memberships: OrganizationMembershipConnection;
    name: Scalars["String"];
    projects: ProjectConnection;
    slug: Scalars["String"];
    updatedAt: Scalars["DateTime"];
  };

export type OrganizationAccessTokensArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<AccessTokenFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type OrganizationInvitesArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type OrganizationMembersArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type OrganizationMembershipsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type OrganizationProjectsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

/** A connection to a list of items. */
export type OrganizationConnection = {
  __typename?: "OrganizationConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

export type OrganizationCreateInput = {
  name: Scalars["String"];
  slug: Scalars["String"];
};

/** An edge in a connection. */
export type OrganizationEdge = {
  __typename?: "OrganizationEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Organization;
};

export type OrganizationInvite = Node & {
  __typename?: "OrganizationInvite";
  createdAt: Scalars["DateTime"];
  email: Scalars["String"];
  emailSentAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  level: OrganizationMembershipLevel;
  organization: Organization;
  updatedAt: Scalars["DateTime"];
  user?: Maybe<User>;
};

/** A connection to a list of items. */
export type OrganizationInviteConnection = {
  __typename?: "OrganizationInviteConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationInviteEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type OrganizationInviteEdge = {
  __typename?: "OrganizationInviteEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: OrganizationInvite;
};

export type OrganizationInviteInput = {
  emails: Array<Scalars["String"]>;
  id: Scalars["GlobalID"];
  level: OrganizationMembershipLevel;
  message?: InputMaybe<Scalars["String"]>;
};

export type OrganizationMembership = Node & {
  __typename?: "OrganizationMembership";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  level: OrganizationMembershipLevel;
  organization: Organization;
  updatedAt: Scalars["DateTime"];
  user: User;
};

/** A connection to a list of items. */
export type OrganizationMembershipConnection = {
  __typename?: "OrganizationMembershipConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationMembershipEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type OrganizationMembershipEdge = {
  __typename?: "OrganizationMembershipEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: OrganizationMembership;
};

export enum OrganizationMembershipLevel {
  Administrator = "Administrator",
  Author = "Author",
  Guest = "Guest",
  Member = "Member",
  Owner = "Owner",
}

export type OrganizationMembershipOperationInfo = OperationInfo | OrganizationMembership;

export type OrganizationOperationInfo = OperationInfo | Organization;

export type OrganizationRemoveMembershipInput = {
  id: Scalars["GlobalID"];
  userId: Scalars["GlobalID"];
};

export type OrganizationUpdateInput = {
  description: Scalars["String"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type OrganizationUpdateMembershipInput = {
  id: Scalars["GlobalID"];
  level: OrganizationMembershipLevel;
  userId: Scalars["GlobalID"];
};

export type OrganizationUser = Organization | User;

export type Owner = {
  accessTokens: AccessTokenConnection;
  canViewFull: Scalars["Boolean"];
  canWrite: Scalars["Boolean"];
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  projects: ProjectConnection;
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
  canWrite: Scalars["Boolean"];
  createdAt: Scalars["DateTime"];
  deployments: DeploymentConnection;
  description?: Maybe<Scalars["String"]>;
  head: ProjectVersion;
  id: Scalars["GlobalID"];
  migrationMappings: ProjectMigrationInfo;
  name: Scalars["String"];
  owner: UserOrganization;
  path: Scalars["String"];
  slug: Scalars["String"];
  type: ProjectType;
  updatedAt: Scalars["DateTime"];
  versions: ProjectVersionConnection;
  visibility: ProjectVisibility;
};

export type ProjectDeploymentsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<DeploymentFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type ProjectMigrationMappingsArgs = {
  sourceVersionId: Scalars["GlobalID"];
  targetVersionId: Scalars["GlobalID"];
};

export type ProjectVersionsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<ProjectVersionFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
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

export type ProjectCreateInput = {
  name: Scalars["String"];
  ownerId: Scalars["GlobalID"];
  slug: Scalars["String"];
  type?: ProjectType;
  visibility: ProjectVisibility;
};

/** An edge in a connection. */
export type ProjectEdge = {
  __typename?: "ProjectEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Project;
};

export type ProjectMigrationInfo = {
  __typename?: "ProjectMigrationInfo";
  isReverse: Scalars["Boolean"];
  refMappings: Array<RefMapping>;
  sourceVersion: ProjectVersion;
  targetVersion: ProjectVersion;
};

export type ProjectOperationInfo = OperationInfo | Project;

export enum ProjectType {
  Executable = "EXECUTABLE",
  Library = "LIBRARY",
}

export type ProjectUpdateNameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type ProjectUpdateVisibilityInput = {
  id: Scalars["GlobalID"];
  visibility: ProjectVisibility;
};

export type ProjectVersion = Node & {
  __typename?: "ProjectVersion";
  childRefs: RefMappingConnection;
  children: Array<ProjectVersion>;
  committed: Scalars["Boolean"];
  committedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  deployments: DeploymentConnection;
  description?: Maybe<Scalars["String"]>;
  files: FileConnection;
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  parentRefs: RefMappingConnection;
  parents: Array<ProjectVersion>;
  project: Project;
  tag?: Maybe<Scalars["String"]>;
};

export type ProjectVersionChildRefsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<RefMappingFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type ProjectVersionDeploymentsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<DeploymentFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type ProjectVersionFilesArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<FileFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type ProjectVersionParentRefsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<RefMappingFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

/** A connection to a list of items. */
export type ProjectVersionConnection = {
  __typename?: "ProjectVersionConnection";
  /** Contains the nodes in this connection */
  edges: Array<ProjectVersionEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type ProjectVersionEdge = {
  __typename?: "ProjectVersionEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: ProjectVersion;
};

export type ProjectVersionFilter = {
  fromId: Scalars["GlobalID"];
  toId: Scalars["GlobalID"];
};

export type ProjectVersionOperationInfo = OperationInfo | ProjectVersion;

export enum ProjectVisibility {
  Private = "PRIVATE",
  Public = "PUBLIC",
  SourcePrivate = "SOURCE_PRIVATE",
}

export type PyFrame = {
  __typename?: "PyFrame";
  filename: Scalars["String"];
  line: Scalars["String"];
  lineno: Scalars["Int"];
  locals?: Maybe<Scalars["JSON"]>;
  name: Scalars["String"];
};

export type Query = {
  __typename?: "Query";
  buildCandidates: BuildCandidateConnection;
  clients: ClientConnection;
  evaluations: EvaluationResultConnection;
  executions: ExecutionConnection;
  featuredProjects: ProjectConnection;
  file?: Maybe<File>;
  jobs: JobConnection;
  me?: Maybe<User>;
  organization?: Maybe<Organization>;
  organizationBySlug?: Maybe<Organization>;
  organizations: OrganizationConnection;
  ownerBySlug?: Maybe<UserOrganization>;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  projectVersionBySlug?: Maybe<ProjectVersion>;
  projectVersionByTag?: Maybe<ProjectVersion>;
  remoteObject?: Maybe<RemoteObject>;
  statement?: Maybe<Statement>;
  systemInfo: SystemInfo;
  user?: Maybe<User>;
  userBySlug?: Maybe<User>;
  users: UserConnection;
};

export type QueryBuildCandidatesArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  statusIn?: InputMaybe<Array<BuildCandidateStatus>>;
};

export type QueryClientsArgs = {
  active?: InputMaybe<Scalars["Boolean"]>;
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  inSameOrganizations?: Scalars["Boolean"];
  last?: InputMaybe<Scalars["Int"]>;
  organizationId?: InputMaybe<Scalars["GlobalID"]>;
  present?: InputMaybe<Scalars["Boolean"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  userId?: InputMaybe<Scalars["GlobalID"]>;
};

export type QueryEvaluationsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  buildIdIn?: InputMaybe<Array<Scalars["GlobalID"]>>;
  first?: InputMaybe<Scalars["Int"]>;
  includeAncestorVersions?: Scalars["Boolean"];
  kindIn?: InputMaybe<Array<EvaluationKind>>;
  last?: InputMaybe<Scalars["Int"]>;
  latestCandidateOnly?: Scalars["Boolean"];
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  scopeIn?: InputMaybe<Array<EvaluationScope>>;
  systemIdIn?: InputMaybe<Array<Scalars["GlobalID"]>>;
};

export type QueryExecutionsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  buildIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  codeIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  first?: InputMaybe<Scalars["Int"]>;
  includeAncestorVersions?: Scalars["Boolean"];
  last?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  rootId?: InputMaybe<Scalars["GlobalID"]>;
  rootIdNull?: Scalars["Boolean"];
  taskIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
};

export type QueryFeaturedProjectsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type QueryFileArgs = {
  id: Scalars["GlobalID"];
};

export type QueryJobsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  statusIn?: InputMaybe<Array<JobStatus>>;
  typeIn?: InputMaybe<Array<JobType>>;
};

export type QueryOrganizationArgs = {
  id: Scalars["GlobalID"];
};

export type QueryOrganizationBySlugArgs = {
  organization: Scalars["String"];
};

export type QueryOrganizationsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
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

export type QueryProjectVersionBySlugArgs = {
  owner: Scalars["String"];
  project: Scalars["String"];
  tag: Scalars["String"];
};

export type QueryProjectVersionByTagArgs = {
  projectId: Scalars["GlobalID"];
  tag: Scalars["String"];
};

export type QueryRemoteObjectArgs = {
  id: Scalars["GlobalID"];
};

export type QueryStatementArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUserArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUserBySlugArgs = {
  slug: Scalars["String"];
};

export type QueryUsersArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<UserFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type RecordBatch = {
  __typename?: "RecordBatch";
  records: Array<DatasetRecord>;
};

export type RecordBatchOperationInfo = OperationInfo | RecordBatch;

export type RecordBatchRestoreInput = {
  ids: Array<Scalars["GlobalID"]>;
};

export type RecordBatchSoftDeleteInput = {
  ids: Array<Scalars["GlobalID"]>;
};

export type RecordCreateInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  statementId: Scalars["GlobalID"];
};

export type RecordDeleteInput = {
  id: Scalars["GlobalID"];
};

export type RecordMoveInput = {
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
};

export type RecordRestoreInput = {
  id: Scalars["GlobalID"];
};

export type RecordTruncateInput = {
  id: Scalars["GlobalID"];
};

export type RecordUpdateInput = {
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
};

export type RecordUpdatePathInput = {
  data?: InputMaybe<Scalars["JSON"]>;
  id: Scalars["GlobalID"];
  path: Scalars["String"];
};

export type RefMapping = Node & {
  __typename?: "RefMapping";
  id: Scalars["GlobalID"];
  kind: RefMappingKind;
  sourceId: Scalars["GlobalID"];
  sourceRevision: Scalars["Int"];
  sourceVersion: ProjectVersion;
  sourceVersionId: Scalars["GlobalID"];
  targetId: Scalars["GlobalID"];
  targetRevision: Scalars["Int"];
  targetVersion: ProjectVersion;
  targetVersionId: Scalars["GlobalID"];
  type: RefType;
};

/** A connection to a list of items. */
export type RefMappingConnection = {
  __typename?: "RefMappingConnection";
  /** Contains the nodes in this connection */
  edges: Array<RefMappingEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type RefMappingEdge = {
  __typename?: "RefMappingEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: RefMapping;
};

export type RefMappingFilter = {
  kind?: InputMaybe<RefMappingKind>;
  type?: InputMaybe<RefType>;
};

export enum RefMappingKind {
  Commit = "COMMIT",
  Paste = "PASTE",
}

export enum RefType {
  File = "FILE",
  Record = "RECORD",
  Statement = "STATEMENT",
  TypeNode = "TYPE_NODE",
  Xblock = "XBLOCK",
}

export type RemoteObject = Node & {
  __typename?: "RemoteObject";
  contentLength: Scalars["Int"];
  contentType: Scalars["String"];
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  presignedGet?: Maybe<Scalars["String"]>;
  presignedPost?: Maybe<Scalars["String"]>;
  sha512: Scalars["String"];
  status: RemoteObjectStatus;
};

export type RemoteObjectOperationInfo = OperationInfo | RemoteObject;

export enum RemoteObjectStatus {
  Available = "AVAILABLE",
  Prepared = "PREPARED",
  Uploading = "UPLOADING",
}

export type RequestUploadObjectInput = {
  contentLength: Scalars["Int"];
  contentType: Scalars["String"];
  name?: InputMaybe<Scalars["String"]>;
  projectId: Scalars["GlobalID"];
  sha512: Scalars["String"];
};

export type RestoreInput = {
  projectVersionId: Scalars["GlobalID"];
};

/** Wire-able representation of an exception. */
export type RunError = {
  __typename?: "RunError";
  message: Scalars["String"];
  symbol?: Maybe<Scalars["String"]>;
  traceback?: Maybe<Array<PyFrame>>;
  type: Scalars["String"];
};

export enum RunErrorType {
  InternalError = "INTERNAL_ERROR",
  InvalidRunconfig = "INVALID_RUNCONFIG",
  NotReady = "NOT_READY",
  RuntimeError = "RUNTIME_ERROR",
  Timeout = "TIMEOUT",
}

export type RunInput = {
  arguments?: InputMaybe<Scalars["JSON"]>;
  block?: Scalars["Boolean"];
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]>;
  trace?: ExecutionTracingLevel;
};

export type RunState = {
  __typename?: "RunState";
  buildId?: Maybe<Scalars["GlobalID"]>;
  error?: Maybe<RunErrorType>;
  errorDetails?: Maybe<RunError>;
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
  key: Scalars["String"];
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
    deletedAt?: Maybe<Scalars["DateTime"]>;
    description?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    isArray: Scalars["Boolean"];
    isNullable: Scalars["Boolean"];
    isOutput: Scalars["Boolean"];
    key: Scalars["String"];
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    reference?: Maybe<Statement>;
    revision: Scalars["Int"];
    statement: Statement;
    tag: TypeTag;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

export type SimpleTypeNodeFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type SimpleTypeNodeOperationInfo = OperationInfo | SimpleTypeNode;

/** Anything typed using SimpleType nodes. */
export type SimplyTyped = {
  rootTypeTag?: Maybe<TypeTag>;
  typeNodes?: Maybe<Array<SimpleType>>;
};

export type Statement = Node &
  SimplyTyped & {
    __typename?: "Statement";
    buildCandidates: Array<BuildCandidate>;
    buildSettings?: Maybe<BuildSettings>;
    children: Array<Statement>;
    code?: Maybe<Scalars["String"]>;
    commented: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    deletedAt?: Maybe<Scalars["DateTime"]>;
    descendants: Array<Statement>;
    description?: Maybe<Scalars["String"]>;
    evaluateSettings?: Maybe<EvaluateSettings>;
    evaluationResults: Array<EvaluationResult>;
    file: File;
    generated: Scalars["Boolean"];
    generatedMappings: Array<GeneratedMapping>;
    id: Scalars["GlobalID"];
    lang?: Maybe<Scalars["String"]>;
    modifier?: Maybe<StatementModifier>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent?: Maybe<Statement>;
    projectVersion: ProjectVersion;
    records: DatasetRecordConnection;
    reference?: Maybe<Statement>;
    referenceProjectVersion?: Maybe<ProjectVersion>;
    revision: Scalars["Int"];
    rootTypeTag?: Maybe<TypeTag>;
    symbolType?: Maybe<SymbolType>;
    type: StatementType;
    typeNodes: Array<SimpleTypeNode>;
    updatedAt: Scalars["DateTime"];
    xblocks: Array<XBlock>;
  };

export type StatementGeneratedMappingsArgs = {
  filters?: InputMaybe<GeneratedMappingFilter>;
};

export type StatementRecordsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<DatasetRecordFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type StatementTypeNodesArgs = {
  filters?: InputMaybe<SimpleTypeNodeFilter>;
};

export type StatementXblocksArgs = {
  filters?: InputMaybe<XBlockFilter>;
};

export type StatementBatch = {
  __typename?: "StatementBatch";
  statements: Array<Statement>;
};

export type StatementBatchCommentedInput = {
  commented: Scalars["Boolean"];
  ids: Array<Scalars["GlobalID"]>;
};

export type StatementBatchMoveInput = {
  fileId: Scalars["GlobalID"];
  ids: Array<Scalars["GlobalID"]>;
  orderKeys: Array<Scalars["String"]>;
  parentIds: Array<InputMaybe<Scalars["GlobalID"]>>;
};

export type StatementBatchOperationInfo = OperationInfo | StatementBatch;

export type StatementBatchPasteInput = {
  sourceIds: Array<Scalars["GlobalID"]>;
  targetFileId: Scalars["GlobalID"];
  targetIds: Array<Scalars["GlobalID"]>;
  targetOrderKeys: Array<Scalars["String"]>;
  targetParentIds: Array<InputMaybe<Scalars["GlobalID"]>>;
};

export type StatementBatchRestoreInput = {
  ids: Array<Scalars["GlobalID"]>;
};

export type StatementBatchSoftDeleteInput = {
  ids: Array<Scalars["GlobalID"]>;
};

export type StatementCommentedInput = {
  commented: Scalars["Boolean"];
  id: Scalars["GlobalID"];
};

/** Creates a blank statement */
export type StatementCreateBlankInput = {
  fileId: Scalars["GlobalID"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

/** Creates a full statement */
export type StatementCreateInput = {
  code?: InputMaybe<Scalars["String"]>;
  commented?: InputMaybe<Scalars["Boolean"]>;
  description?: InputMaybe<Scalars["String"]>;
  fileId: Scalars["GlobalID"];
  generated?: InputMaybe<Scalars["Boolean"]>;
  id?: InputMaybe<Scalars["GlobalID"]>;
  lang?: InputMaybe<Scalars["String"]>;
  modifier?: InputMaybe<StatementModifier>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  symbolType?: InputMaybe<SymbolType>;
  type: StatementType;
};

export type StatementDeleteInput = {
  id: Scalars["GlobalID"];
};

export type StatementFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

/** A modifier to a Bench statement. */
export enum StatementModifier {
  Check = "CHECK",
  Include = "INCLUDE",
  Like = "LIKE",
  Local = "LOCAL",
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
  referenceName?: InputMaybe<Scalars["String"]>;
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
  buildCandidateChanged: BuildCandidate;
  clientsChanged: Client;
  evaluationsChanged: EvaluationResult;
  executionsChanged: Execution;
  interpChanged: InterpModule;
  jobsChanged: Job;
  moduleChanged: ModuleChange;
};

export type SubscriptionBuildCandidateChangedArgs = {
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionClientsChangedArgs = {
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
};

export type SubscriptionEvaluationsChangedArgs = {
  buildIdIn?: InputMaybe<Array<Scalars["GlobalID"]>>;
  includeAncestorVersions?: Scalars["Boolean"];
  kindIn?: InputMaybe<Array<EvaluationKind>>;
  latestCandidateOnly?: Scalars["Boolean"];
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  scopeIn?: InputMaybe<Array<EvaluationScope>>;
  systemIdIn?: InputMaybe<Array<Scalars["GlobalID"]>>;
};

export type SubscriptionExecutionsChangedArgs = {
  buildIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  codeIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  includeAncestorVersions?: Scalars["Boolean"];
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  rootId?: InputMaybe<Scalars["GlobalID"]>;
  rootIdNull?: Scalars["Boolean"];
  taskIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
};

export type SubscriptionInterpChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionJobsChangedArgs = {
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  typeIn?: InputMaybe<Array<JobType>>;
};

export type SubscriptionModuleChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

/** The type of symbol content. */
export enum SymbolType {
  Block = "BLOCK",
  Build = "BUILD",
  Capability = "CAPABILITY",
  Code = "CODE",
  Data = "DATA",
  Evaluate = "EVALUATE",
  Expectation = "EXPECTATION",
  Model = "MODEL",
  Program = "PROGRAM",
  Requirement = "REQUIREMENT",
  Runconfig = "RUNCONFIG",
  Task = "TASK",
  Type = "TYPE",
}

export type SystemInfo = {
  __typename?: "SystemInfo";
  gitCommit: Scalars["String"];
  version: Scalars["String"];
};

export type TypeNodeCreateInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  isArray?: Scalars["Boolean"];
  isNullable?: Scalars["Boolean"];
  isOutput?: Scalars["Boolean"];
  key: Scalars["String"];
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type TypeNodeDeleteInput = {
  id: Scalars["GlobalID"];
};

export type TypeNodeMoveInput = {
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
};

export type TypeNodeRenameInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
};

export type TypeNodeRestoreInput = {
  id: Scalars["GlobalID"];
};

export type TypeNodeUpdateDescriptionInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
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

export type TypeNodeUpdateTypeInput = {
  id: Scalars["GlobalID"];
  isArray?: Scalars["Boolean"];
  isNullable?: Scalars["Boolean"];
  isOutput?: Scalars["Boolean"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  tag: TypeTag;
  value?: InputMaybe<Scalars["JSON"]>;
};

/** The type of type node. */
export enum TypeTag {
  Any = "ANY",
  Audio = "AUDIO",
  Boolean = "BOOLEAN",
  Embedding = "EMBEDDING",
  Enum = "ENUM",
  File = "FILE",
  Function = "FUNCTION",
  Image = "IMAGE",
  Json = "JSON",
  Literal = "LITERAL",
  Null = "NULL",
  Number = "NUMBER",
  String = "STRING",
  Struct = "STRUCT",
  TypeReference = "TYPE_REFERENCE",
  Union = "UNION",
  Video = "VIDEO",
}

export type UpdateProjectVersion = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  tag?: InputMaybe<Scalars["String"]>;
};

export type User = Node &
  Owner & {
    __typename?: "User";
    accessTokens: AccessTokenConnection;
    bot: Scalars["Boolean"];
    canViewFull: Scalars["Boolean"];
    canWrite: Scalars["Boolean"];
    completedSignup: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    email: Scalars["String"];
    id: Scalars["GlobalID"];
    name: Scalars["String"];
    notifications: NotificationConnection;
    organizationMemberships: OrganizationMembershipConnection;
    organizations: OrganizationConnection;
    projects: ProjectConnection;
    slug: Scalars["String"];
    updatedAt: Scalars["DateTime"];
    /** Required. 150 characters or fewer. Letters, digits and @/./+/-/_ only. */
    username: Scalars["String"];
  };

export type UserAccessTokensArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<AccessTokenFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type UserNotificationsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<NotificationFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type UserOrganizationMembershipsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type UserOrganizationsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type UserProjectsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type UserCompleteSignupInput = {
  fullName: Scalars["String"];
  id: Scalars["GlobalID"];
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

export type UserFilter = {
  emailEquals?: InputMaybe<Scalars["String"]>;
  slugPrefix?: InputMaybe<Scalars["String"]>;
};

export type UserOperationInfo = OperationInfo | User;

export type UserOrganization = Organization | User;

export type UserUpdateInput = {
  description: Scalars["String"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type XBlock = Node & {
  __typename?: "XBlock";
  createdAt: Scalars["DateTime"];
  description?: Maybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  kind: XKind;
  orderKey: Scalars["String"];
  revision: Scalars["Int"];
  source: XSource;
  statement: Statement;
  value?: Maybe<Scalars["JSON"]>;
};

export type XBlockFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export enum XKind {
  Input = "Input",
  Output = "Output",
  Settings = "Settings",
  Static = "Static",
}

export enum XSource {
  Developer = "Developer",
  Model = "Model",
  System = "System",
  User = "User",
}

export type DeploymentsQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type DeploymentsQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    committed: boolean;
    tag?: string | null;
    deployments: {
      __typename?: "DeploymentConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "DeploymentEdge";
        node: {
          __typename?: "Deployment";
          id: any;
          createdAt: any;
          updatedAt: any;
          type: DeploymentType;
          status: DeploymentStatus;
          deployAllStatements: boolean;
        };
      }>;
    };
  } | null;
};

export type EmptyEditorSuggestedFilesQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  last: Scalars["Int"];
}>;

export type EmptyEditorSuggestedFilesQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    files: {
      __typename?: "FileConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "FileEdge";
        node: { __typename?: "File"; id: any; name: string; path: string; deletedAt?: any | null; directory: boolean };
      }>;
    };
  } | null;
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
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type ExistingProjectVersionTagQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  tag: Scalars["String"];
}>;

export type ExistingProjectVersionTagQuery = {
  __typename?: "Query";
  projectVersionByTag?: { __typename?: "ProjectVersion"; id: any; tag?: string | null } | null;
};

export type RunInfoQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
}>;

export type RunInfoQuery = {
  __typename?: "Query";
  project?: { __typename?: "Project"; id: any; path: string; name: string; slug: string } | null;
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    name?: string | null;
    tag?: string | null;
    committed: boolean;
    createdAt: any;
    committedAt?: any | null;
  } | null;
};

export type MatchingUsersQueryVariables = Exact<{
  slug?: InputMaybe<Scalars["String"]>;
  email?: InputMaybe<Scalars["String"]>;
}>;

export type MatchingUsersQuery = {
  __typename?: "Query";
  users: {
    __typename?: "UserConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "UserEdge";
      node: { __typename?: "User"; id: any; slug: string; username: string; email: string };
    }>;
  };
};

export type RecordsQueryVariables = Exact<{
  statementId: Scalars["GlobalID"];
  after?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
}>;

export type RecordsQuery = {
  __typename?: "Query";
  statement?: {
    __typename?: "Statement";
    id: any;
    records: {
      __typename?: "DatasetRecordConnection";
      totalCount?: number | null;
      pageInfo: {
        __typename?: "PageInfo";
        hasNextPage: boolean;
        hasPreviousPage: boolean;
        startCursor?: string | null;
        endCursor?: string | null;
      };
      edges: Array<{
        __typename?: "DatasetRecordEdge";
        cursor: string;
        node: {
          __typename?: "DatasetRecord";
          id: any;
          revision: number;
          createdAt: any;
          updatedAt: any;
          deletedAt?: any | null;
          orderKey: string;
          data: any;
        };
      }>;
    };
  } | null;
};

export type NotificationsQueryVariables = Exact<{
  status?: InputMaybe<NotificationStatus>;
  notArchived?: InputMaybe<Scalars["Boolean"]>;
  first?: InputMaybe<Scalars["Int"]>;
}>;

export type NotificationsQuery = {
  __typename?: "Query";
  me?: {
    __typename?: "User";
    id: any;
    notifications: {
      __typename?: "NotificationConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "NotificationEdge";
        node: {
          __typename?: "Notification";
          id: any;
          type: NotificationType;
          createdAt: any;
          readAt?: any | null;
          archivedAt?: any | null;
          expiresAt?: any | null;
          status: NotificationStatus;
          invite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationMembershipLevel;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
        };
      }>;
    };
  } | null;
};

export type ProjectVersionsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
}>;

export type ProjectVersionsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    id: any;
    head: { __typename?: "ProjectVersion" } & {
      " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
    };
    versions: {
      __typename?: "ProjectVersionConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "ProjectVersionEdge";
        node: { __typename?: "ProjectVersion" } & {
          " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
        };
      }>;
    };
  } | null;
};

export type AutobuildFileContentByIdQueryVariables = Exact<{
  fileId: Scalars["GlobalID"];
}>;

export type AutobuildFileContentByIdQuery = {
  __typename?: "Query";
  file?: {
    __typename?: "File";
    id: any;
    projectVersion: { __typename?: "ProjectVersion"; id: any };
    statements: Array<{
      __typename?: "Statement";
      id: any;
      name?: string | null;
      type: StatementType;
      symbolType?: SymbolType | null;
      orderKey: string;
      description?: string | null;
      commented: boolean;
      parent?: { __typename?: "Statement"; id: any } | null;
      buildSettings?: { __typename?: "BuildSettings"; id: any; reactive: boolean } | null;
      evaluateSettings?: { __typename?: "EvaluateSettings"; id: any; weights: any } | null;
    }>;
  } | null;
};

export type ProfileAccessTokensQueryVariables = Exact<{
  slug: Scalars["String"];
  includeInactive: Scalars["Boolean"];
}>;

export type ProfileAccessTokensQuery = {
  __typename?: "Query";
  ownerBySlug?:
    | {
        __typename?: "Organization";
        id: any;
        accessTokens: {
          __typename?: "AccessTokenConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "AccessTokenEdge";
            node: {
              __typename?: "AccessToken";
              id: any;
              name?: string | null;
              tokenKey: string;
              createdAt: any;
              updatedAt: any;
              expiresAt?: any | null;
              revokedAt?: any | null;
              status: AccessTokenStatus;
              scopes: Array<AccessTokenScope>;
            };
          }>;
        };
      }
    | {
        __typename?: "User";
        id: any;
        accessTokens: {
          __typename?: "AccessTokenConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "AccessTokenEdge";
            node: {
              __typename?: "AccessToken";
              id: any;
              name?: string | null;
              tokenKey: string;
              createdAt: any;
              updatedAt: any;
              expiresAt?: any | null;
              revokedAt?: any | null;
              status: AccessTokenStatus;
              scopes: Array<AccessTokenScope>;
            };
          }>;
        };
      }
    | null;
};

export type CreateAccessTokenMutationVariables = Exact<{
  ownerId: Scalars["GlobalID"];
  scopes: Array<AccessTokenScope> | AccessTokenScope;
  expiresAt?: InputMaybe<Scalars["DateTime"]>;
  name?: InputMaybe<Scalars["String"]>;
}>;

export type CreateAccessTokenMutation = {
  __typename?: "Mutation";
  createAccessToken:
    | {
        __typename?: "AccessTokenCreatePayload";
        token: string;
        accessToken: {
          __typename?: "AccessToken";
          id: any;
          name?: string | null;
          tokenKey: string;
          createdAt: any;
          updatedAt: any;
          expiresAt?: any | null;
          revokedAt?: any | null;
          status: AccessTokenStatus;
          scopes: Array<AccessTokenScope>;
        };
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RevokeAccessTokenMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RevokeAccessTokenMutation = {
  __typename?: "Mutation";
  revokeAccessToken:
    | { __typename?: "AccessToken"; id: any; revokedAt?: any | null; status: AccessTokenStatus }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type OrganizationMembersQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type OrganizationMembersQuery = {
  __typename?: "Query";
  organizationBySlug?: {
    __typename?: "Organization";
    id: any;
    canWrite: boolean;
    memberships: {
      __typename?: "OrganizationMembershipConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "OrganizationMembershipEdge";
        node: {
          __typename?: "OrganizationMembership";
          id: any;
          createdAt: any;
          level: OrganizationMembershipLevel;
          user: { __typename?: "User"; id: any; slug: string; email: string; name: string; username: string };
        };
      }>;
    };
    invites: {
      __typename?: "OrganizationInviteConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "OrganizationInviteEdge";
        node: {
          __typename?: "OrganizationInvite";
          id: any;
          createdAt: any;
          level: OrganizationMembershipLevel;
          email: string;
          emailSentAt?: any | null;
          user?: { __typename?: "User"; id: any; slug: string; email: string; name: string; username: string } | null;
        };
      }>;
    };
  } | null;
};

export type ProfileSettingsQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type ProfileSettingsQuery = {
  __typename?: "Query";
  ownerBySlug?:
    | {
        __typename?: "Organization";
        id: any;
        slug: string;
        name: string;
        description?: string | null;
        canWrite: boolean;
      }
    | {
        __typename?: "User";
        id: any;
        slug: string;
        name: string;
        username: string;
        description?: string | null;
        canWrite: boolean;
      }
    | null;
};

export type UpdateOrganizationMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  description: Scalars["String"];
}>;

export type UpdateOrganizationMutation = {
  __typename?: "Mutation";
  updateOrganization:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Organization"; id: any; name: string; description?: string | null };
};

export type UpdateUserMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  description: Scalars["String"];
}>;

export type UpdateUserMutation = {
  __typename?: "Mutation";
  updateUser:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "User"; id: any; name: string; description?: string | null };
};

export type CheckOwnerBySlugQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type CheckOwnerBySlugQuery = {
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
    tag?: string | null;
    description?: string | null;
    createdAt: any;
    committed: boolean;
    committedAt?: any | null;
    files: {
      __typename?: "FileConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "FileEdge";
        node: { __typename?: "File"; id: any } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } };
      }>;
    };
  } | null;
};

export type ExistingProjectBySlugQueryVariables = Exact<{
  owner: Scalars["String"];
  project: Scalars["String"];
}>;

export type ExistingProjectBySlugQuery = {
  __typename?: "Query";
  projectBySlug?: { __typename?: "Project"; id: any; slug: string } | null;
};

export type HomeBenchesQueryVariables = Exact<{ [key: string]: never }>;

export type HomeBenchesQuery = {
  __typename?: "Query";
  me?: {
    __typename?: "User";
    id: any;
    slug: string;
    projects: {
      __typename?: "ProjectConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "ProjectEdge";
        node: {
          __typename?: "Project";
          id: any;
          name: string;
          slug: string;
          path: string;
          createdAt: any;
          type: ProjectType;
          visibility: ProjectVisibility;
          description?: string | null;
        };
      }>;
    };
    organizations: {
      __typename?: "OrganizationConnection";
      edges: Array<{
        __typename?: "OrganizationEdge";
        node: {
          __typename?: "Organization";
          projects: {
            __typename?: "ProjectConnection";
            totalCount?: number | null;
            edges: Array<{
              __typename?: "ProjectEdge";
              node: {
                __typename?: "Project";
                id: any;
                name: string;
                slug: string;
                path: string;
                createdAt: any;
                type: ProjectType;
                visibility: ProjectVisibility;
                description?: string | null;
              };
            }>;
          };
        };
      }>;
    };
  } | null;
};

export type FeaturedBenchesQueryVariables = Exact<{ [key: string]: never }>;

export type FeaturedBenchesQuery = {
  __typename?: "Query";
  featuredProjects: {
    __typename?: "ProjectConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "ProjectEdge";
      node: {
        __typename?: "Project";
        id: any;
        name: string;
        slug: string;
        path: string;
        createdAt: any;
        type: ProjectType;
        visibility: ProjectVisibility;
        description?: string | null;
      };
    }>;
  };
};

export type ProfileHomeQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type ProfileHomeQuery = {
  __typename?: "Query";
  ownerBySlug?:
    | {
        __typename?: "Organization";
        id: any;
        slug: string;
        name: string;
        description?: string | null;
        createdAt: any;
        canViewFull: boolean;
        canWrite: boolean;
        projects: {
          __typename?: "ProjectConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "ProjectEdge";
            node: {
              __typename?: "Project";
              id: any;
              name: string;
              slug: string;
              path: string;
              createdAt: any;
              type: ProjectType;
              visibility: ProjectVisibility;
              head: { __typename?: "ProjectVersion"; name?: string | null; createdAt: any };
            };
          }>;
        };
      }
    | {
        __typename?: "User";
        id: any;
        slug: string;
        name: string;
        username: string;
        bot: boolean;
        description?: string | null;
        createdAt: any;
        canViewFull: boolean;
        canWrite: boolean;
        projects: {
          __typename?: "ProjectConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "ProjectEdge";
            node: {
              __typename?: "Project";
              id: any;
              name: string;
              slug: string;
              path: string;
              createdAt: any;
              type: ProjectType;
              visibility: ProjectVisibility;
              head: { __typename?: "ProjectVersion"; name?: string | null; createdAt: any };
            };
          }>;
        };
      }
    | null;
};

export type SettingsQueryVariables = Exact<{
  slug: Scalars["String"];
}>;

export type SettingsQuery = {
  __typename?: "Query";
  ownerBySlug?:
    | {
        __typename?: "Organization";
        id: any;
        slug: string;
        name: string;
        createdAt: any;
        updatedAt: any;
        canViewFull: boolean;
        canWrite: boolean;
        members: { __typename?: "UserConnection"; totalCount?: number | null };
        accessTokens: { __typename?: "AccessTokenConnection"; totalCount?: number | null };
      }
    | {
        __typename?: "User";
        id: any;
        slug: string;
        name: string;
        username: string;
        bot: boolean;
        createdAt: any;
        updatedAt: any;
        canViewFull: boolean;
        canWrite: boolean;
        accessTokens: { __typename?: "AccessTokenConnection"; totalCount?: number | null };
      }
    | null;
};

export type MeQueryVariables = Exact<{ [key: string]: never }>;

export type MeQuery = {
  __typename?: "Query";
  me?: {
    __typename?: "User";
    id: any;
    username: string;
    slug: string;
    email: string;
    name: string;
    createdAt: any;
    updatedAt: any;
    completedSignup: boolean;
    organizationMemberships: {
      __typename?: "OrganizationMembershipConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "OrganizationMembershipEdge";
        node: {
          __typename?: "OrganizationMembership";
          id: any;
          createdAt: any;
          level: OrganizationMembershipLevel;
          organization: { __typename?: "Organization"; id: any; name: string; slug: string };
        };
      }>;
    };
  } | null;
};

export type ClientContentTypeFragment = {
  __typename?: "Client";
  id: any;
  type: ClientType;
  deviceName?: string | null;
  browserName?: string | null;
  lastSeenAt?: any | null;
  closedAt?: any | null;
  active: boolean;
  present: boolean;
  user: { __typename?: "User"; id: any; name: string; username: string; email: string };
  project?: { __typename?: "Project"; id: any; name: string } | null;
  file?: { __typename?: "File"; id: any; name: string } | null;
  statement?: { __typename?: "Statement"; id: any; name?: string | null } | null;
} & { " $fragmentName"?: "ClientContentTypeFragment" };

export type ConnectedClientsQueryVariables = Exact<{
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  userId?: InputMaybe<Scalars["GlobalID"]>;
  inSameOrganizations: Scalars["Boolean"];
  first?: InputMaybe<Scalars["Int"]>;
  active?: InputMaybe<Scalars["Boolean"]>;
}>;

export type ConnectedClientsQuery = {
  __typename?: "Query";
  clients: {
    __typename?: "ClientConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "ClientEdge";
      node: { __typename?: "Client" } & { " $fragmentRefs"?: { ClientContentTypeFragment: ClientContentTypeFragment } };
    }>;
  };
};

export type ClientsChangedSubscriptionVariables = Exact<{
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type ClientsChangedSubscription = {
  __typename?: "Subscription";
  clientsChanged: { __typename?: "Client" } & {
    " $fragmentRefs"?: { ClientContentTypeFragment: ClientContentTypeFragment };
  };
};

export type ClientStatusFragment = {
  __typename?: "Client";
  id: any;
  lastSeenAt?: any | null;
  closedAt?: any | null;
  active: boolean;
  present: boolean;
} & { " $fragmentName"?: "ClientStatusFragment" };

export type ProjectMigrationRefsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  sourceVersionId: Scalars["GlobalID"];
  targetVersionId: Scalars["GlobalID"];
}>;

export type ProjectMigrationRefsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    migrationMappings: {
      __typename?: "ProjectMigrationInfo";
      isReverse: boolean;
      sourceVersion: {
        __typename?: "ProjectVersion";
        id: any;
        createdAt: any;
        tag?: string | null;
        name?: string | null;
      };
      targetVersion: {
        __typename?: "ProjectVersion";
        id: any;
        createdAt: any;
        tag?: string | null;
        name?: string | null;
      };
      refMappings: Array<{
        __typename?: "RefMapping";
        type: RefType;
        sourceId: any;
        sourceVersionId: any;
        targetId: any;
        targetVersionId: any;
      }>;
    };
  } | null;
};

export type EvaluationResultContentFragment = {
  __typename?: "EvaluationResult";
  id: any;
  createdAt: any;
  updatedAt: any;
  kind: EvaluationKind;
  scope: EvaluationScope;
  selfMetrics?: any | null;
  aggregatedMetrics: any;
  project: { __typename?: "Project"; id: any };
  projectVersion: { __typename?: "ProjectVersion"; id: any };
  build?: { __typename?: "Statement"; id: any } | null;
  statement?: { __typename?: "Statement"; id: any } | null;
  record?: { __typename?: "DatasetRecord"; id: any } | null;
  typeNode?: { __typename?: "SimpleTypeNode"; id: any } | null;
} & { " $fragmentName"?: "EvaluationResultContentFragment" };

export type EvaluationsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  includeAncestorVersions?: InputMaybe<Scalars["Boolean"]>;
  scopeIn?: InputMaybe<Array<EvaluationScope> | EvaluationScope>;
  kindIn?: InputMaybe<Array<EvaluationKind> | EvaluationKind>;
  buildIdIn?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  systemIdIn?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  first?: InputMaybe<Scalars["Int"]>;
}>;

export type EvaluationsQuery = {
  __typename?: "Query";
  evaluations: {
    __typename?: "EvaluationResultConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "EvaluationResultEdge";
      node: { __typename?: "EvaluationResult" } & {
        " $fragmentRefs"?: { EvaluationResultContentFragment: EvaluationResultContentFragment };
      };
    }>;
  };
};

export type EvaluationsChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  includeAncestorVersions?: InputMaybe<Scalars["Boolean"]>;
  latestCandidateOnly?: InputMaybe<Scalars["Boolean"]>;
  scopeIn?: InputMaybe<Array<EvaluationScope> | EvaluationScope>;
  kindIn?: InputMaybe<Array<EvaluationKind> | EvaluationKind>;
  buildIdIn?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  systemIdIn?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
}>;

export type EvaluationsChangedSubscription = {
  __typename?: "Subscription";
  evaluationsChanged: { __typename?: "EvaluationResult" } & {
    " $fragmentRefs"?: { EvaluationResultContentFragment: EvaluationResultContentFragment };
  };
};

export type ExecutionContentFragment = {
  __typename?: "Execution";
  id: any;
  createdAt: any;
  updatedAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  duration?: number | null;
  cachedDuration?: number | null;
  status: ExecutionStatus;
  triggerType: ExecutionTriggerType;
  inputs?: any | null;
  outputs?: any | null;
  error?: any | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any; tag?: string | null; name?: string | null };
  deployment?: { __typename?: "Deployment"; id: any } | null;
  user?: { __typename?: "User"; id: any; slug: string } | null;
  accessToken?: { __typename?: "AccessToken"; id: any; name?: string | null } | null;
  root?: { __typename?: "Execution"; id: any } | null;
  parent?: { __typename?: "Execution"; id: any } | null;
  build?: { __typename?: "Statement"; id: any; name?: string | null } | null;
  task?: { __typename?: "Statement"; id: any; name?: string | null } | null;
  code?: { __typename?: "Statement"; id: any; name?: string | null } | null;
} & { " $fragmentName"?: "ExecutionContentFragment" };

export type ExecutionsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  includeAncestorVersions?: InputMaybe<Scalars["Boolean"]>;
  buildIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  taskIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  codeIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  rootIdNull?: InputMaybe<Scalars["Boolean"]>;
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

export type ExecutionsChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  includeAncestorVersions?: InputMaybe<Scalars["Boolean"]>;
  buildIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  taskIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  codeIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  rootIdNull?: InputMaybe<Scalars["Boolean"]>;
}>;

export type ExecutionsChangedSubscription = {
  __typename?: "Subscription";
  executionsChanged: { __typename?: "Execution" } & {
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

export type ProjectVersionHeaderFragment = {
  __typename?: "ProjectVersion";
  id: any;
  name?: string | null;
  tag?: string | null;
  description?: string | null;
  createdAt: any;
  committed: boolean;
  committedAt?: any | null;
  parents: Array<{ __typename?: "ProjectVersion"; id: any }>;
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type ProjectHeaderFragment = {
  __typename?: "Project";
  id: any;
  type: ProjectType;
  visibility: ProjectVisibility;
  name: string;
  slug: string;
  createdAt: any;
  updatedAt: any;
  canWrite: boolean;
  head: { __typename?: "ProjectVersion" } & {
    " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
  };
  owner:
    | { __typename?: "Organization"; id: any; slug: string; name: string }
    | { __typename?: "User"; id: any; slug: string; username: string; name: string };
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
  directory: boolean;
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

export type SimpleTypeNodeContentFragment = {
  __typename?: "SimpleTypeNode";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  revision: number;
  name?: string | null;
  key: string;
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
  rootTypeTag?: TypeTag | null;
  parent?: { __typename?: "Statement"; id: any } | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  referenceProjectVersion?: { __typename?: "ProjectVersion"; id: any } | null;
  typeNodes: Array<
    { __typename?: "SimpleTypeNode" } & {
      " $fragmentRefs"?: { SimpleTypeNodeContentFragment: SimpleTypeNodeContentFragment };
    }
  >;
} & { " $fragmentName"?: "StatementContentFragment" };

export type JobContentFragment = {
  __typename?: "Job";
  id: any;
  createdAt: any;
  updatedAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  status: JobStatus;
  type: JobType;
  projectVersion: { __typename?: "ProjectVersion"; id: any };
} & { " $fragmentName"?: "JobContentFragment" };

export type JobsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  statusIn?: InputMaybe<Array<JobStatus> | JobStatus>;
  typeIn?: InputMaybe<Array<JobType> | JobType>;
  first?: InputMaybe<Scalars["Int"]>;
}>;

export type JobsQuery = {
  __typename?: "Query";
  jobs: {
    __typename?: "JobConnection";
    totalCount?: number | null;
    edges: Array<{
      __typename?: "JobEdge";
      node: { __typename?: "Job" } & { " $fragmentRefs"?: { JobContentFragment: JobContentFragment } };
    }>;
  };
};

export type JobsChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  typeIn?: InputMaybe<Array<JobType> | JobType>;
}>;

export type JobsChangedSubscription = {
  __typename?: "Subscription";
  jobsChanged: { __typename?: "Job" } & { " $fragmentRefs"?: { JobContentFragment: JobContentFragment } };
};

export type NewNotificationsQueryVariables = Exact<{
  after?: InputMaybe<Scalars["String"]>;
  status?: InputMaybe<NotificationStatus>;
}>;

export type NewNotificationsQuery = {
  __typename?: "Query";
  me?: {
    __typename?: "User";
    id: any;
    notifications: {
      __typename?: "NotificationConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "NotificationEdge";
        node: {
          __typename?: "Notification";
          id: any;
          type: NotificationType;
          createdAt: any;
          readAt?: any | null;
          archivedAt?: any | null;
          expiresAt?: any | null;
          status: NotificationStatus;
          invite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationMembershipLevel;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
        };
      }>;
    };
  } | null;
};

export type MarkNotificationMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  status: NotificationStatus;
}>;

export type MarkNotificationMutation = {
  __typename?: "Mutation";
  markNotification:
    | { __typename?: "Notification"; id: any; status: NotificationStatus; readAt?: any | null; archivedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RemoteObjectQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RemoteObjectQuery = {
  __typename?: "Query";
  remoteObject?: { __typename?: "RemoteObject"; id: any; presignedGet?: string | null } | null;
};

export type UpsertClientMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  type: ClientType;
  deviceName?: InputMaybe<Scalars["String"]>;
  browserName?: InputMaybe<Scalars["String"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  fileId?: InputMaybe<Scalars["GlobalID"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]>;
  typeNodeId?: InputMaybe<Scalars["GlobalID"]>;
  recordId?: InputMaybe<Scalars["GlobalID"]>;
  path?: InputMaybe<Scalars["String"]>;
}>;

export type UpsertClientMutation = {
  __typename?: "Mutation";
  upsertClient:
    | {
        __typename?: "Client";
        id: any;
        type: ClientType;
        deviceName?: string | null;
        browserName?: string | null;
        projectVersion?: { __typename?: "ProjectVersion"; id: any } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CloseClientMutationVariables = Exact<{ [key: string]: never }>;

export type CloseClientMutation = {
  __typename?: "Mutation";
  closeClient:
    | { __typename?: "Client" }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type UpdatePresenceMutationVariables = Exact<{ [key: string]: never }>;

export type UpdatePresenceMutation = {
  __typename?: "Mutation";
  updatePresence:
    | { __typename?: "Client" }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type UpdateDeploymentMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  status: DeploymentStatus;
}>;

export type UpdateDeploymentMutation = {
  __typename?: "Mutation";
  updateDeployment:
    | { __typename?: "Deployment"; id: any; type: DeploymentType; status: DeploymentStatus }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

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
    | {
        __typename?: "File";
        id: any;
        revision: number;
        name: string;
        path: string;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        directory: boolean;
        generated: boolean;
        parent?: { __typename?: "File"; id: any } | null;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        statements: Array<{ __typename?: "Statement"; id: any }>;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteFileMutation = {
  __typename?: "Mutation";
  deleteFile:
    | { __typename?: "File"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type SoftDeleteFileMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteFileMutation = {
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

export type RequestUploadObjectMutationVariables = Exact<{
  projectId: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  contentType: Scalars["String"];
  contentLength: Scalars["Int"];
  sha512: Scalars["String"];
}>;

export type RequestUploadObjectMutation = {
  __typename?: "Mutation";
  requestUploadObject:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "RemoteObject";
        id: any;
        status: RemoteObjectStatus;
        name?: string | null;
        contentType: string;
        contentLength: number;
        sha512: string;
        presignedPost?: string | null;
        presignedGet?: string | null;
      };
};

export type NotifyUploadedObjectMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type NotifyUploadedObjectMutation = {
  __typename?: "Mutation";
  notifyUploadedObject:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "RemoteObject";
        id: any;
        status: RemoteObjectStatus;
        name?: string | null;
        contentType: string;
        contentLength: number;
        sha512: string;
        presignedGet?: string | null;
      };
};

export type CreateOrganizationMutationVariables = Exact<{
  name: Scalars["String"];
  slug: Scalars["String"];
}>;

export type CreateOrganizationMutation = {
  __typename?: "Mutation";
  createOrganization:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Organization"; id: any; name: string; slug: string };
};

export type CreateInvitesMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  emails: Array<Scalars["String"]> | Scalars["String"];
  level: OrganizationMembershipLevel;
  message?: InputMaybe<Scalars["String"]>;
}>;

export type CreateInvitesMutation = {
  __typename?: "Mutation";
  createOrganizationInvites:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Organization";
        id: any;
        invites: {
          __typename?: "OrganizationInviteConnection";
          totalCount?: number | null;
          edges: Array<{ __typename?: "OrganizationInviteEdge"; node: { __typename?: "OrganizationInvite"; id: any } }>;
        };
      };
};

export type CancelInviteMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type CancelInviteMutation = {
  __typename?: "Mutation";
  cancelOrganizationInvite:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Organization";
        id: any;
        invites: {
          __typename?: "OrganizationInviteConnection";
          totalCount?: number | null;
          edges: Array<{ __typename?: "OrganizationInviteEdge"; node: { __typename?: "OrganizationInvite"; id: any } }>;
        };
      };
};

export type CreateProjectMutationVariables = Exact<{
  input: ProjectCreateInput;
}>;

export type CreateProjectMutation = {
  __typename?: "Mutation";
  createProject:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | ({ __typename?: "Project" } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } });
};

export type UpdateProjectVisibilityMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  visibility: ProjectVisibility;
}>;

export type UpdateProjectVisibilityMutation = {
  __typename?: "Mutation";
  updateProjectVisibility:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Project"; id: any; visibility: ProjectVisibility };
};

export type UpdateProjectNameMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
}>;

export type UpdateProjectNameMutation = {
  __typename?: "Mutation";
  updateProjectName:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Project"; id: any; name: string };
};

export type BuildMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  scope: BuildScope;
  buildableId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type BuildMutation = {
  __typename?: "Mutation";
  build: { __typename?: "BuildState"; projectVersionId: any; success: boolean } | { __typename?: "OperationInfo" };
};

export type RunMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  buildId?: InputMaybe<Scalars["GlobalID"]>;
  arguments?: InputMaybe<Scalars["JSON"]>;
  block?: InputMaybe<Scalars["Boolean"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]>;
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
        error?: RunErrorType | null;
        errorDetails?: {
          __typename?: "RunError";
          type: string;
          message: string;
          traceback?: Array<{
            __typename?: "PyFrame";
            line: string;
            filename: string;
            lineno: number;
            name: string;
          }> | null;
        } | null;
      };
};

export type CreateStatementMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  fileId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  type: StatementType;
  modifier?: InputMaybe<StatementModifier>;
  name?: InputMaybe<Scalars["String"]>;
  symbolType?: InputMaybe<SymbolType>;
  lang?: InputMaybe<Scalars["String"]>;
  code?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  commented?: InputMaybe<Scalars["Boolean"]>;
  generated?: InputMaybe<Scalars["Boolean"]>;
}>;

export type CreateStatementMutation = {
  __typename?: "Mutation";
  createStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
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
        rootTypeTag?: TypeTag | null;
        file: { __typename?: "File"; id: any };
        parent?: { __typename?: "Statement"; id: any } | null;
        reference?: { __typename?: "Statement"; id: any } | null;
        referenceProjectVersion?: { __typename?: "ProjectVersion"; id: any } | null;
        typeNodes: Array<{ __typename?: "SimpleTypeNode"; id: any }>;
        records: {
          __typename?: "DatasetRecordConnection";
          totalCount?: number | null;
          edges: Array<{ __typename?: "DatasetRecordEdge"; node: { __typename?: "DatasetRecord"; id: any } }>;
        };
      };
};

export type MorphStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  type: StatementType;
  symbolType?: InputMaybe<SymbolType>;
  name?: InputMaybe<Scalars["String"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  lang?: InputMaybe<Scalars["String"]>;
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

export type BatchMoveStatementMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
  fileId: Scalars["GlobalID"];
  parentIds: Array<InputMaybe<Scalars["GlobalID"]>> | InputMaybe<Scalars["GlobalID"]>;
  orderKeys: Array<Scalars["String"]> | Scalars["String"];
}>;

export type BatchMoveStatementMutation = {
  __typename?: "Mutation";
  batchMoveStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "StatementBatch";
        statements: Array<{
          __typename?: "Statement";
          id: any;
          orderKey: string;
          revision: number;
          file: { __typename?: "File"; id: any };
          parent?: { __typename?: "Statement"; id: any } | null;
        }>;
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
  deleteStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; deletedAt?: any | null };
};

export type SoftDeleteStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteStatementMutation = {
  __typename?: "Mutation";
  softDeleteStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; deletedAt?: any | null };
};

export type BatchDeleteStatementsMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
}>;

export type BatchDeleteStatementsMutation = {
  __typename?: "Mutation";
  batchSoftDeleteStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "StatementBatch";
        statements: Array<{ __typename?: "Statement"; id: any; deletedAt?: any | null }>;
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

export type BatchRestoreStatementsMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
}>;

export type BatchRestoreStatementsMutation = {
  __typename?: "Mutation";
  batchRestoreStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "StatementBatch";
        statements: Array<{ __typename?: "Statement"; id: any; deletedAt?: any | null }>;
      };
};

export type BatchPasteStatementMutationVariables = Exact<{
  sourceIds: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
  targetIds: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
  targetFileId: Scalars["GlobalID"];
  targetParentIds: Array<InputMaybe<Scalars["GlobalID"]>> | InputMaybe<Scalars["GlobalID"]>;
  targetOrderKeys: Array<Scalars["String"]> | Scalars["String"];
}>;

export type BatchPasteStatementMutation = {
  __typename?: "Mutation";
  batchPasteStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "StatementBatch";
        statements: Array<
          { __typename?: "Statement"; id: any; file: { __typename?: "File"; id: any } } & {
            " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment };
          }
        >;
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
  referenceName?: InputMaybe<Scalars["String"]>;
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
        reference?: { __typename?: "Statement"; id: any; name?: string | null } | null;
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
  createRecord:
    | {
        __typename?: "DatasetRecord";
        id: any;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        revision: number;
        orderKey: string;
        data: any;
        statement: { __typename?: "Statement"; id: any };
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type _OrderKeyFragment = { __typename?: "DatasetRecord"; orderKey: string } & {
  " $fragmentName"?: "_OrderKeyFragment";
};

export type UpdateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  data: Scalars["JSON"];
}>;

export type UpdateRecordMutation = {
  __typename?: "Mutation";
  updateRecord:
    | { __typename?: "DatasetRecord"; id: any; updatedAt: any; revision: number; data: any }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteRecordMutation = {
  __typename?: "Mutation";
  deleteRecord:
    | { __typename?: "DatasetRecord"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type SoftDeleteRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteRecordMutation = {
  __typename?: "Mutation";
  softDeleteRecord:
    | { __typename?: "DatasetRecord"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RestoreRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreRecordMutation = {
  __typename?: "Mutation";
  restoreRecord:
    | { __typename?: "DatasetRecord"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type BatchSoftDeleteRecordMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
}>;

export type BatchSoftDeleteRecordMutation = {
  __typename?: "Mutation";
  batchSoftDeleteRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "RecordBatch"; records: Array<{ __typename?: "DatasetRecord"; id: any; deletedAt?: any | null }> };
};

export type BatchRestoreRecordMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
}>;

export type BatchRestoreRecordMutation = {
  __typename?: "Mutation";
  batchRestoreRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "RecordBatch"; records: Array<{ __typename?: "DatasetRecord"; id: any; deletedAt?: any | null }> };
};

export type TruncateRecordsMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type TruncateRecordsMutation = {
  __typename?: "Mutation";
  truncateRecords:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any };
};

export type CreateTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  key: Scalars["String"];
  orderKey: Scalars["String"];
  name: Scalars["String"];
  description?: InputMaybe<Scalars["String"]>;
  isOutput: Scalars["Boolean"];
  isArray: Scalars["Boolean"];
  isNullable: Scalars["Boolean"];
  value?: InputMaybe<Scalars["JSON"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type CreateTypeNodeMutation = {
  __typename?: "Mutation";
  createTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "SimpleTypeNode";
        id: any;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        key: string;
        orderKey: string;
        revision: number;
        name?: string | null;
        tag: TypeTag;
        description?: string | null;
        value?: any | null;
        isOutput: boolean;
        isArray: boolean;
        isNullable: boolean;
        statement: { __typename?: "Statement"; id: any };
        reference?: { __typename?: "Statement"; id: any } | null;
      };
};

export type DeleteTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteTypeNodeMutation = {
  __typename?: "Mutation";
  deleteTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "SimpleTypeNode"; id: any; deletedAt?: any | null };
};

export type SoftDeleteTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteTypeNodeMutation = {
  __typename?: "Mutation";
  softDeleteTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "SimpleTypeNode"; id: any; deletedAt?: any | null };
};

export type RestoreTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreTypeNodeMutation = {
  __typename?: "Mutation";
  restoreStatementTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "SimpleTypeNode"; id: any; deletedAt?: any | null };
};

export type UpdateTypeNodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  tag: TypeTag;
  name?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  isOutput: Scalars["Boolean"];
  isArray: Scalars["Boolean"];
  isNullable: Scalars["Boolean"];
  value?: InputMaybe<Scalars["JSON"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type UpdateTypeNodeMutation = {
  __typename?: "Mutation";
  updateTypeNode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "SimpleTypeNode";
        id: any;
        tag: TypeTag;
        updatedAt: any;
        revision: number;
        name?: string | null;
        description?: string | null;
        isOutput: boolean;
        isArray: boolean;
        isNullable: boolean;
        value?: any | null;
        reference?: { __typename?: "Statement"; id: any } | null;
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
    | {
        __typename?: "User";
        id: any;
        username: string;
        slug: string;
        email: string;
        name: string;
        createdAt: any;
        updatedAt: any;
        completedSignup: boolean;
      };
};

export type AcceptOrganizationInviteMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type AcceptOrganizationInviteMutation = {
  __typename?: "Mutation";
  acceptOrganizationInvite:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "User";
        id: any;
        username: string;
        slug: string;
        email: string;
        name: string;
        createdAt: any;
        updatedAt: any;
        completedSignup: boolean;
        organizationMemberships: {
          __typename?: "OrganizationMembershipConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "OrganizationMembershipEdge";
            node: {
              __typename?: "OrganizationMembership";
              id: any;
              level: OrganizationMembershipLevel;
              organization: { __typename?: "Organization"; id: any; name: string; slug: string };
            };
          }>;
        };
      };
};

export type UpdateVersionMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  tag?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
}>;

export type UpdateVersionMutation = {
  __typename?: "Mutation";
  updateProjectVersion:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | ({ __typename?: "ProjectVersion" } & {
        " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
      });
};

export type CommitMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  tag?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  autoDeploy?: InputMaybe<Scalars["Boolean"]>;
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

export type RestoreMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type RestoreMutation = {
  __typename?: "Mutation";
  restore:
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
  availableBuilds?: Array<any> | null;
  typeNodes?: Array<{
    __typename?: "InterpSimpleType";
    id: any;
    name?: string | null;
    key: string;
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
  dependencies: Array<{
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
  }>;
  errors: Array<
    { __typename?: "InterpError" } & { " $fragmentRefs"?: { InterpErrorContentFragment: InterpErrorContentFragment } }
  >;
  staleSymbols: Array<{
    __typename?: "InterpSymbol";
    id: any;
    name?: string | null;
    type: StatementType;
    symbolType?: SymbolType | null;
    modifier?: StatementModifier | null;
    parentId?: any | null;
    rootTypeTag?: TypeTag | null;
    generated: boolean;
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

export type InterpChangedSubscriptionVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type InterpChangedSubscription = {
  __typename?: "Subscription";
  interpChanged: { __typename?: "InterpModule" } & {
    " $fragmentRefs"?: { InterpModuleContentFragment: InterpModuleContentFragment };
  };
};

export type ModuleChangedSubscriptionVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type ModuleChangedSubscription = {
  __typename?: "Subscription";
  moduleChanged: {
    __typename?: "ModuleChange";
    id: any;
    clientId?: any | null;
    mutations: Array<{
      __typename?: "ModuleMutation";
      type: ModuleMutationType;
      fileId?: any | null;
      statementId?: any | null;
      revision?: number | null;
      input?: any | null;
    }>;
  };
};

export type SystemInfoQueryVariables = Exact<{ [key: string]: never }>;

export type SystemInfoQuery = {
  __typename?: "Query";
  systemInfo: { __typename?: "SystemInfo"; version: string; gitCommit: string };
};

export const ClientContentTypeFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ClientContentType" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Client" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "deviceName" } },
          { kind: "Field", name: { kind: "Name", value: "browserName" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "user" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "email" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "file" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastSeenAt" } },
          { kind: "Field", name: { kind: "Name", value: "closedAt" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "present" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ClientContentTypeFragment, unknown>;
export const ClientStatusFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ClientStatus" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Client" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "lastSeenAt" } },
          { kind: "Field", name: { kind: "Name", value: "closedAt" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "present" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ClientStatusFragment, unknown>;
export const EvaluationResultContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "EvaluationResultContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "EvaluationResult" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "scope" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
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
            name: { kind: "Name", value: "build" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "record" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNode" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "selfMetrics" } },
          { kind: "Field", name: { kind: "Name", value: "aggregatedMetrics" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<EvaluationResultContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "cachedDuration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersion" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "tag" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "deployment" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "user" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "accessToken" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
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
            name: { kind: "Name", value: "build" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "task" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "code" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
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
          { kind: "Field", name: { kind: "Name", value: "tag" } },
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
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "canWrite" } },
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
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
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
          { kind: "Field", name: { kind: "Name", value: "directory" } },
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
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
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
          { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNodes" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SimpleTypeNodeContent" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementContentFragment, unknown>;
export const JobContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "JobContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Job" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
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
} as unknown as DocumentNode<JobContentFragment, unknown>;
export const _OrderKeyFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "_orderKey" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [{ kind: "Field", name: { kind: "Name", value: "orderKey" } }],
      },
    },
  ],
} as unknown as DocumentNode<_OrderKeyFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "availableBuilds" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "typeNodes" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "key" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "dependencies" },
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
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "InterpSymbolContent" } },
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
            name: { kind: "Name", value: "errors" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpErrorContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "staleSymbols" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
                { kind: "Field", name: { kind: "Name", value: "symbolType" } },
                { kind: "Field", name: { kind: "Name", value: "modifier" } },
                { kind: "Field", name: { kind: "Name", value: "parentId" } },
                { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                { kind: "Field", name: { kind: "Name", value: "generated" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpModuleContentFragment, unknown>;
export const DeploymentsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "deployments" },
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
            name: { kind: "Name", value: "projectVersion" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "committed" } },
                { kind: "Field", name: { kind: "Name", value: "tag" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "deployments" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isOwned" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "type" } },
                                  { kind: "Field", name: { kind: "Name", value: "status" } },
                                  { kind: "Field", name: { kind: "Name", value: "deployAllStatements" } },
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
      },
    },
  ],
} as unknown as DocumentNode<DeploymentsQuery, DeploymentsQueryVariables>;
export const EmptyEditorSuggestedFilesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "emptyEditorSuggestedFiles" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "last" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
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
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
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
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isGenerated" },
                            value: { kind: "BooleanValue", value: false },
                          },
                        ],
                      },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "name" } },
                                  { kind: "Field", name: { kind: "Name", value: "path" } },
                                  { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "directory" } },
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
      },
    },
  ],
} as unknown as DocumentNode<EmptyEditorSuggestedFilesQuery, EmptyEditorSuggestedFilesQueryVariables>;
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "projectVersion" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
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
export const ExistingProjectVersionTagDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "existingProjectVersionTag" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersionByTag" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "tag" },
                value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "tag" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExistingProjectVersionTagQuery, ExistingProjectVersionTagQueryVariables>;
export const RunInfoDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "runInfo" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
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
                { kind: "Field", name: { kind: "Name", value: "path" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersion" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "tag" } },
                { kind: "Field", name: { kind: "Name", value: "committed" } },
                { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                { kind: "Field", name: { kind: "Name", value: "committedAt" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<RunInfoQuery, RunInfoQueryVariables>;
export const MatchingUsersDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "matchingUsers" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "slug" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "email" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "users" },
            arguments: [
              { kind: "Argument", name: { kind: "Name", value: "first" }, value: { kind: "IntValue", value: "10" } },
              {
                kind: "Argument",
                name: { kind: "Name", value: "filters" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "slugPrefix" },
                      value: { kind: "Variable", name: { kind: "Name", value: "slug" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "emailEquals" },
                      value: { kind: "Variable", name: { kind: "Name", value: "email" } },
                    },
                  ],
                },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "slug" } },
                            { kind: "Field", name: { kind: "Name", value: "username" } },
                            { kind: "Field", name: { kind: "Name", value: "email" } },
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
} as unknown as DocumentNode<MatchingUsersQuery, MatchingUsersQueryVariables>;
export const RecordsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "records" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "after" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "records" },
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
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "after" },
                      value: { kind: "Variable", name: { kind: "Name", value: "after" } },
                    },
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "first" },
                      value: { kind: "Variable", name: { kind: "Name", value: "first" } },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "totalCount" } },
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
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "revision" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                                  { kind: "Field", name: { kind: "Name", value: "data" } },
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
      },
    },
  ],
} as unknown as DocumentNode<RecordsQuery, RecordsQueryVariables>;
export const NotificationsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "notifications" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "status" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "NotificationStatus" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "notArchived" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "me" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "notifications" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "status" },
                            value: { kind: "Variable", name: { kind: "Name", value: "status" } },
                          },
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "notArchived" },
                            value: { kind: "Variable", name: { kind: "Name", value: "notArchived" } },
                          },
                        ],
                      },
                    },
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "first" },
                      value: { kind: "Variable", name: { kind: "Name", value: "first" } },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "type" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "readAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "archivedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "expiresAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "status" } },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "invite" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "organization" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "id" } },
                                              { kind: "Field", name: { kind: "Name", value: "slug" } },
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                            ],
                                          },
                                        },
                                        { kind: "Field", name: { kind: "Name", value: "level" } },
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
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<NotificationsQuery, NotificationsQueryVariables>;
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
                  name: { kind: "Name", value: "head" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "versions" },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
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
                    ],
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
export const AutobuildFileContentByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "autobuildFileContentById" },
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
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "symbolType" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parent" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "commented" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "buildSettings" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "reactive" } },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "evaluateSettings" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "weights" } },
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
} as unknown as DocumentNode<AutobuildFileContentByIdQuery, AutobuildFileContentByIdQueryVariables>;
export const ProfileAccessTokensDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "profileAccessTokens" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "slug" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "includeInactive" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "accessTokens" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "includeInactive" },
                                  value: { kind: "Variable", name: { kind: "Name", value: "includeInactive" } },
                                },
                              ],
                            },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "name" } },
                                        { kind: "Field", name: { kind: "Name", value: "tokenKey" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "expiresAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "revokedAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "status" } },
                                        { kind: "Field", name: { kind: "Name", value: "scopes" } },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "accessTokens" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "includeInactive" },
                                  value: { kind: "Variable", name: { kind: "Name", value: "includeInactive" } },
                                },
                              ],
                            },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "name" } },
                                        { kind: "Field", name: { kind: "Name", value: "tokenKey" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "expiresAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "revokedAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "status" } },
                                        { kind: "Field", name: { kind: "Name", value: "scopes" } },
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
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProfileAccessTokensQuery, ProfileAccessTokensQueryVariables>;
export const CreateAccessTokenDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createAccessToken" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ownerId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopes" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: {
                kind: "NonNullType",
                type: { kind: "NamedType", name: { kind: "Name", value: "AccessTokenScope" } },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "expiresAt" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "DateTime" } },
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
            name: { kind: "Name", value: "createAccessToken" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ownerId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ownerId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scopes" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scopes" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "expiresAt" },
                      value: { kind: "Variable", name: { kind: "Name", value: "expiresAt" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "AccessTokenCreatePayload" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "token" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "accessToken" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                            { kind: "Field", name: { kind: "Name", value: "tokenKey" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "expiresAt" } },
                            { kind: "Field", name: { kind: "Name", value: "revokedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "status" } },
                            { kind: "Field", name: { kind: "Name", value: "scopes" } },
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
} as unknown as DocumentNode<CreateAccessTokenMutation, CreateAccessTokenMutationVariables>;
export const RevokeAccessTokenDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "revokeAccessToken" },
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
            name: { kind: "Name", value: "revokeAccessToken" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "AccessToken" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revokedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "status" } },
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
} as unknown as DocumentNode<RevokeAccessTokenMutation, RevokeAccessTokenMutationVariables>;
export const OrganizationMembersDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "organizationMembers" },
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
            name: { kind: "Name", value: "organizationBySlug" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "organization" },
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
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "memberships" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "level" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "user" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "id" } },
                                              { kind: "Field", name: { kind: "Name", value: "slug" } },
                                              { kind: "Field", name: { kind: "Name", value: "email" } },
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                              { kind: "Field", name: { kind: "Name", value: "username" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "invites" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "level" } },
                                        { kind: "Field", name: { kind: "Name", value: "email" } },
                                        { kind: "Field", name: { kind: "Name", value: "emailSentAt" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "user" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "id" } },
                                              { kind: "Field", name: { kind: "Name", value: "slug" } },
                                              { kind: "Field", name: { kind: "Name", value: "email" } },
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                              { kind: "Field", name: { kind: "Name", value: "username" } },
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
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<OrganizationMembersQuery, OrganizationMembersQueryVariables>;
export const ProfileSettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "profileSettings" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                    ],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
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
} as unknown as DocumentNode<ProfileSettingsQuery, ProfileSettingsQueryVariables>;
export const UpdateOrganizationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateOrganization" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateOrganization" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
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
} as unknown as DocumentNode<UpdateOrganizationMutation, UpdateOrganizationMutationVariables>;
export const UpdateUserDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateUser" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateUser" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
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
} as unknown as DocumentNode<UpdateUserMutation, UpdateUserMutationVariables>;
export const CheckOwnerBySlugDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "checkOwnerBySlug" },
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
} as unknown as DocumentNode<CheckOwnerBySlugQuery, CheckOwnerBySlugQueryVariables>;
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
                { kind: "Field", name: { kind: "Name", value: "tag" } },
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
                      { kind: "Field", name: { kind: "Name", value: "totalCount" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
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
              ],
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectVersionContentQuery, ProjectVersionContentQueryVariables>;
export const ExistingProjectBySlugDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "existingProjectBySlug" },
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
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExistingProjectBySlugQuery, ExistingProjectBySlugQueryVariables>;
export const HomeBenchesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "homeBenches" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "me" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "projects" },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "name" } },
                                  { kind: "Field", name: { kind: "Name", value: "slug" } },
                                  { kind: "Field", name: { kind: "Name", value: "path" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "type" } },
                                  { kind: "Field", name: { kind: "Name", value: "visibility" } },
                                  { kind: "Field", name: { kind: "Name", value: "description" } },
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
                  name: { kind: "Name", value: "organizations" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "projects" },
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
                                              {
                                                kind: "Field",
                                                name: { kind: "Name", value: "node" },
                                                selectionSet: {
                                                  kind: "SelectionSet",
                                                  selections: [
                                                    { kind: "Field", name: { kind: "Name", value: "id" } },
                                                    { kind: "Field", name: { kind: "Name", value: "name" } },
                                                    { kind: "Field", name: { kind: "Name", value: "slug" } },
                                                    { kind: "Field", name: { kind: "Name", value: "path" } },
                                                    { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                                    { kind: "Field", name: { kind: "Name", value: "type" } },
                                                    { kind: "Field", name: { kind: "Name", value: "visibility" } },
                                                    { kind: "Field", name: { kind: "Name", value: "description" } },
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
} as unknown as DocumentNode<HomeBenchesQuery, HomeBenchesQueryVariables>;
export const FeaturedBenchesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "featuredBenches" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "featuredProjects" },
            arguments: [
              { kind: "Argument", name: { kind: "Name", value: "last" }, value: { kind: "IntValue", value: "5" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                            { kind: "Field", name: { kind: "Name", value: "slug" } },
                            { kind: "Field", name: { kind: "Name", value: "path" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "type" } },
                            { kind: "Field", name: { kind: "Name", value: "visibility" } },
                            { kind: "Field", name: { kind: "Name", value: "description" } },
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
} as unknown as DocumentNode<FeaturedBenchesQuery, FeaturedBenchesQueryVariables>;
export const ProfileHomeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "profileHome" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "bot" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "canViewFull" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "projects" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "name" } },
                                        { kind: "Field", name: { kind: "Name", value: "slug" } },
                                        { kind: "Field", name: { kind: "Name", value: "path" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "type" } },
                                        { kind: "Field", name: { kind: "Name", value: "visibility" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "head" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                              { kind: "Field", name: { kind: "Name", value: "createdAt" } },
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
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "canViewFull" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "projects" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "name" } },
                                        { kind: "Field", name: { kind: "Name", value: "slug" } },
                                        { kind: "Field", name: { kind: "Name", value: "path" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        { kind: "Field", name: { kind: "Name", value: "type" } },
                                        { kind: "Field", name: { kind: "Name", value: "visibility" } },
                                        { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "head" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                              { kind: "Field", name: { kind: "Name", value: "createdAt" } },
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
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProfileHomeQuery, ProfileHomeQueryVariables>;
export const SettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "settings" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "bot" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "canViewFull" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "accessTokens" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "includeInactive" },
                                  value: { kind: "BooleanValue", value: false },
                                },
                              ],
                            },
                          },
                        ],
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "totalCount" } }],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "canViewFull" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "members" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "totalCount" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "accessTokens" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "includeInactive" },
                                  value: { kind: "BooleanValue", value: false },
                                },
                              ],
                            },
                          },
                        ],
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "totalCount" } }],
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
} as unknown as DocumentNode<SettingsQuery, SettingsQueryVariables>;
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
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
                { kind: "Field", name: { kind: "Name", value: "email" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                { kind: "Field", name: { kind: "Name", value: "completedSignup" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "organizationMemberships" },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "level" } },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "organization" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "name" } },
                                        { kind: "Field", name: { kind: "Name", value: "slug" } },
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
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<MeQuery, MeQueryVariables>;
export const ConnectedClientsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "connectedClients" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "userId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "inSameOrganizations" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "active" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "clients" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "userId" },
                value: { kind: "Variable", name: { kind: "Name", value: "userId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "inSameOrganizations" },
                value: { kind: "Variable", name: { kind: "Name", value: "inSameOrganizations" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "first" },
                value: { kind: "Variable", name: { kind: "Name", value: "first" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "active" },
                value: { kind: "Variable", name: { kind: "Name", value: "active" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ClientContentType" } }],
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
    ...ClientContentTypeFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ConnectedClientsQuery, ConnectedClientsQueryVariables>;
export const ClientsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "clientsChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "clientsChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ClientContentType" } }],
            },
          },
        ],
      },
    },
    ...ClientContentTypeFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ClientsChangedSubscription, ClientsChangedSubscriptionVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "sourceVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetVersionId" } },
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
                  name: { kind: "Name", value: "migrationMappings" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "sourceVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sourceVersionId" } },
                    },
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "targetVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetVersionId" } },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "isReverse" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "sourceVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "tag" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "targetVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "tag" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "refMappings" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "type" } },
                            { kind: "Field", name: { kind: "Name", value: "sourceId" } },
                            { kind: "Field", name: { kind: "Name", value: "sourceVersionId" } },
                            { kind: "Field", name: { kind: "Name", value: "targetId" } },
                            { kind: "Field", name: { kind: "Name", value: "targetVersionId" } },
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
export const EvaluationsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "evaluations" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopeIn" } },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: { kind: "NamedType", name: { kind: "Name", value: "EvaluationScope" } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "kindIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "EvaluationKind" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildIdIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "systemIdIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "evaluations" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "includeAncestorVersions" },
                value: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "scopeIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "scopeIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "kindIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "kindIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "buildIdIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "buildIdIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "systemIdIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "systemIdIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "first" },
                value: { kind: "Variable", name: { kind: "Name", value: "first" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "EvaluationResultContent" } },
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
    ...EvaluationResultContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<EvaluationsQuery, EvaluationsQueryVariables>;
export const EvaluationsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "evaluationsChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "latestCandidateOnly" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopeIn" } },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: { kind: "NamedType", name: { kind: "Name", value: "EvaluationScope" } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "kindIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "EvaluationKind" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildIdIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "systemIdIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "evaluationsChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "includeAncestorVersions" },
                value: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "latestCandidateOnly" },
                value: { kind: "Variable", name: { kind: "Name", value: "latestCandidateOnly" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "scopeIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "scopeIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "kindIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "kindIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "buildIdIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "buildIdIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "systemIdIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "systemIdIn" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "EvaluationResultContent" } }],
            },
          },
        ],
      },
    },
    ...EvaluationResultContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<EvaluationsChangedSubscription, EvaluationsChangedSubscriptionVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "taskIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "codeIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootIdNull" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
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
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "includeAncestorVersions" },
                value: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "buildIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "buildIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "taskIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "taskIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "codeIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "codeIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "rootIdNull" },
                value: { kind: "Variable", name: { kind: "Name", value: "rootIdNull" } },
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
export const ExecutionsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "executionsChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "buildIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "taskIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "codeIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootIdNull" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "executionsChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "includeAncestorVersions" },
                value: { kind: "Variable", name: { kind: "Name", value: "includeAncestorVersions" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "buildIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "buildIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "taskIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "taskIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "codeIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "codeIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "rootIdNull" },
                value: { kind: "Variable", name: { kind: "Name", value: "rootIdNull" } },
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
} as unknown as DocumentNode<ExecutionsChangedSubscription, ExecutionsChangedSubscriptionVariables>;
export const JobsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "jobs" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statusIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JobStatus" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "typeIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JobType" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "first" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "jobs" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "statusIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "statusIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "typeIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "typeIn" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "first" },
                value: { kind: "Variable", name: { kind: "Name", value: "first" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "JobContent" } }],
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
    ...JobContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<JobsQuery, JobsQueryVariables>;
export const JobsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "jobsChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "typeIn" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JobType" } } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "jobsChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "typeIn" },
                value: { kind: "Variable", name: { kind: "Name", value: "typeIn" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "JobContent" } }],
            },
          },
        ],
      },
    },
    ...JobContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<JobsChangedSubscription, JobsChangedSubscriptionVariables>;
export const NewNotificationsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "newNotifications" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "after" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "status" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "NotificationStatus" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "me" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "notifications" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "after" },
                      value: { kind: "Variable", name: { kind: "Name", value: "after" } },
                    },
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "status" },
                            value: { kind: "Variable", name: { kind: "Name", value: "status" } },
                          },
                        ],
                      },
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
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "type" } },
                                  { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "readAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "archivedAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "expiresAt" } },
                                  { kind: "Field", name: { kind: "Name", value: "status" } },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "invite" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "organization" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "id" } },
                                              { kind: "Field", name: { kind: "Name", value: "slug" } },
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                            ],
                                          },
                                        },
                                        { kind: "Field", name: { kind: "Name", value: "level" } },
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
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<NewNotificationsQuery, NewNotificationsQueryVariables>;
export const MarkNotificationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "markNotification" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "status" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "NotificationStatus" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "markNotification" },
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
                      name: { kind: "Name", value: "status" },
                      value: { kind: "Variable", name: { kind: "Name", value: "status" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Notification" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "status" } },
                      { kind: "Field", name: { kind: "Name", value: "readAt" } },
                      { kind: "Field", name: { kind: "Name", value: "archivedAt" } },
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
} as unknown as DocumentNode<MarkNotificationMutation, MarkNotificationMutationVariables>;
export const RemoteObjectDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "remoteObject" },
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
            name: { kind: "Name", value: "remoteObject" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RemoteObject" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "presignedGet" } },
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
} as unknown as DocumentNode<RemoteObjectQuery, RemoteObjectQueryVariables>;
export const UpsertClientDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "upsertClient" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "type" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "ClientType" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "deviceName" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "browserName" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "typeNodeId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "recordId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "path" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "upsertClient" },
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
                      name: { kind: "Name", value: "type" },
                      value: { kind: "Variable", name: { kind: "Name", value: "type" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "deviceName" },
                      value: { kind: "Variable", name: { kind: "Name", value: "deviceName" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "browserName" },
                      value: { kind: "Variable", name: { kind: "Name", value: "browserName" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "fileId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "typeNodeId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "typeNodeId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "recordId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "recordId" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Client" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "deviceName" } },
                      { kind: "Field", name: { kind: "Name", value: "browserName" } },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpsertClientMutation, UpsertClientMutationVariables>;
export const CloseClientDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "closeClient" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "closeClient" },
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
} as unknown as DocumentNode<CloseClientMutation, CloseClientMutationVariables>;
export const UpdatePresenceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updatePresence" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updatePresence" },
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
} as unknown as DocumentNode<UpdatePresenceMutation, UpdatePresenceMutationVariables>;
export const UpdateDeploymentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateDeployment" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "status" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "DeploymentStatus" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateDeployment" },
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
                      name: { kind: "Name", value: "status" },
                      value: { kind: "Variable", name: { kind: "Name", value: "status" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Deployment" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "status" } },
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
} as unknown as DocumentNode<UpdateDeploymentMutation, UpdateDeploymentMutationVariables>;
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
                      { kind: "Field", name: { kind: "Name", value: "directory" } },
                      { kind: "Field", name: { kind: "Name", value: "generated" } },
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
            name: { kind: "Name", value: "deleteFile" },
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
export const SoftDeleteFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteFile" },
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
} as unknown as DocumentNode<SoftDeleteFileMutation, SoftDeleteFileMutationVariables>;
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
export const RequestUploadObjectDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "requestUploadObject" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "contentType" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "contentLength" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sha512" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "requestUploadObject" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "projectId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "contentType" },
                      value: { kind: "Variable", name: { kind: "Name", value: "contentType" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "contentLength" },
                      value: { kind: "Variable", name: { kind: "Name", value: "contentLength" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sha512" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sha512" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RemoteObject" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "status" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "contentType" } },
                      { kind: "Field", name: { kind: "Name", value: "contentLength" } },
                      { kind: "Field", name: { kind: "Name", value: "sha512" } },
                      { kind: "Field", name: { kind: "Name", value: "presignedPost" } },
                      { kind: "Field", name: { kind: "Name", value: "presignedGet" } },
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
} as unknown as DocumentNode<RequestUploadObjectMutation, RequestUploadObjectMutationVariables>;
export const NotifyUploadedObjectDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "notifyUploadedObject" },
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
            name: { kind: "Name", value: "notifyUploadedObject" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RemoteObject" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "status" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "contentType" } },
                      { kind: "Field", name: { kind: "Name", value: "contentLength" } },
                      { kind: "Field", name: { kind: "Name", value: "sha512" } },
                      { kind: "Field", name: { kind: "Name", value: "presignedGet" } },
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
} as unknown as DocumentNode<NotifyUploadedObjectMutation, NotifyUploadedObjectMutationVariables>;
export const CreateOrganizationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createOrganization" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
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
            name: { kind: "Name", value: "createOrganization" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "slug" },
                      value: { kind: "Variable", name: { kind: "Name", value: "slug" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
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
} as unknown as DocumentNode<CreateOrganizationMutation, CreateOrganizationMutationVariables>;
export const CreateInvitesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createInvites" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "emails" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "level" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "OrganizationMembershipLevel" } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "message" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createOrganizationInvites" },
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
                      name: { kind: "Name", value: "emails" },
                      value: { kind: "Variable", name: { kind: "Name", value: "emails" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "level" },
                      value: { kind: "Variable", name: { kind: "Name", value: "level" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "message" },
                      value: { kind: "Variable", name: { kind: "Name", value: "message" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "invites" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
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
} as unknown as DocumentNode<CreateInvitesMutation, CreateInvitesMutationVariables>;
export const CancelInviteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "cancelInvite" },
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
            name: { kind: "Name", value: "cancelOrganizationInvite" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Organization" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "invites" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
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
} as unknown as DocumentNode<CancelInviteMutation, CancelInviteMutationVariables>;
export const CreateProjectDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createProject" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "input" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "ProjectCreateInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createProject" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectHeader" } }],
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
} as unknown as DocumentNode<CreateProjectMutation, CreateProjectMutationVariables>;
export const UpdateProjectVisibilityDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateProjectVisibility" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "visibility" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "ProjectVisibility" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateProjectVisibility" },
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
                      name: { kind: "Name", value: "visibility" },
                      value: { kind: "Variable", name: { kind: "Name", value: "visibility" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "visibility" } },
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
} as unknown as DocumentNode<UpdateProjectVisibilityMutation, UpdateProjectVisibilityMutationVariables>;
export const UpdateProjectNameDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateProjectName" },
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
            name: { kind: "Name", value: "updateProjectName" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
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
} as unknown as DocumentNode<UpdateProjectNameMutation, UpdateProjectNameMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "scope" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "BuildScope" } } },
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
                      name: { kind: "Name", value: "scope" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scope" } },
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
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "block" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "timeoutSeconds" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "block" },
                      value: { kind: "Variable", name: { kind: "Name", value: "block" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "timeoutSeconds" },
                      value: { kind: "Variable", name: { kind: "Name", value: "timeoutSeconds" } },
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
                      { kind: "Field", name: { kind: "Name", value: "error" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "errorDetails" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "type" } },
                            { kind: "Field", name: { kind: "Name", value: "message" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "traceback" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "line" } },
                                  { kind: "Field", name: { kind: "Name", value: "filename" } },
                                  { kind: "Field", name: { kind: "Name", value: "lineno" } },
                                  { kind: "Field", name: { kind: "Name", value: "name" } },
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
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "type" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "StatementType" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "modifier" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "StatementModifier" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "symbolType" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "SymbolType" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "lang" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "code" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "commented" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "generated" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "type" },
                      value: { kind: "Variable", name: { kind: "Name", value: "type" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "modifier" },
                      value: { kind: "Variable", name: { kind: "Name", value: "modifier" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "symbolType" },
                      value: { kind: "Variable", name: { kind: "Name", value: "symbolType" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "lang" },
                      value: { kind: "Variable", name: { kind: "Name", value: "lang" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "code" },
                      value: { kind: "Variable", name: { kind: "Name", value: "code" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeTag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "commented" },
                      value: { kind: "Variable", name: { kind: "Name", value: "commented" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "generated" },
                      value: { kind: "Variable", name: { kind: "Name", value: "generated" } },
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
                      { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "typeNodes" },
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
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
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
                            { kind: "Field", name: { kind: "Name", value: "totalCount" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "edges" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "type" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "StatementType" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "symbolType" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "SymbolType" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "lang" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
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
                      name: { kind: "Name", value: "type" },
                      value: { kind: "Variable", name: { kind: "Name", value: "type" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "symbolType" },
                      value: { kind: "Variable", name: { kind: "Name", value: "symbolType" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeTag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "lang" },
                      value: { kind: "Variable", name: { kind: "Name", value: "lang" } },
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
export const BatchMoveStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchMoveStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ids" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "parentIds" } },
          type: {
            kind: "NonNullType",
            type: { kind: "ListType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "orderKeys" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchMoveStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ids" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ids" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "fileId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "fileId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "parentIds" },
                      value: { kind: "Variable", name: { kind: "Name", value: "parentIds" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "orderKeys" },
                      value: { kind: "Variable", name: { kind: "Name", value: "orderKeys" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "StatementBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
} as unknown as DocumentNode<BatchMoveStatementMutation, BatchMoveStatementMutationVariables>;
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
            name: { kind: "Name", value: "deleteStatement" },
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
export const SoftDeleteStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteStatement" },
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
} as unknown as DocumentNode<SoftDeleteStatementMutation, SoftDeleteStatementMutationVariables>;
export const BatchDeleteStatementsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchDeleteStatements" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ids" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchSoftDeleteStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ids" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ids" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "StatementBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
} as unknown as DocumentNode<BatchDeleteStatementsMutation, BatchDeleteStatementsMutationVariables>;
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
export const BatchRestoreStatementsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchRestoreStatements" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ids" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchRestoreStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ids" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ids" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "StatementBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
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
} as unknown as DocumentNode<BatchRestoreStatementsMutation, BatchRestoreStatementsMutationVariables>;
export const BatchPasteStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchPasteStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sourceIds" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetIds" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetFileId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetParentIds" } },
          type: {
            kind: "NonNullType",
            type: { kind: "ListType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetOrderKeys" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchPasteStatement" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sourceIds" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sourceIds" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetIds" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetIds" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetFileId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetFileId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetParentIds" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetParentIds" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetOrderKeys" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetOrderKeys" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "StatementBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statements" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "file" },
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
} as unknown as DocumentNode<BatchPasteStatementMutation, BatchPasteStatementMutationVariables>;
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
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceName" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceName" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceName" } },
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
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
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
            name: { kind: "Name", value: "createRecord" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "data" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statement" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "data" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateRecord" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "data" } },
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
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "deleteRecord" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
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
} as unknown as DocumentNode<DeleteRecordMutation, DeleteRecordMutationVariables>;
export const SoftDeleteRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteRecord" },
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
            name: { kind: "Name", value: "softDeleteRecord" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
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
} as unknown as DocumentNode<SoftDeleteRecordMutation, SoftDeleteRecordMutationVariables>;
export const RestoreRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreRecord" },
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
            name: { kind: "Name", value: "restoreRecord" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "DatasetRecord" } },
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
} as unknown as DocumentNode<RestoreRecordMutation, RestoreRecordMutationVariables>;
export const BatchSoftDeleteRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchSoftDeleteRecord" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ids" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchSoftDeleteRecord" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ids" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ids" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RecordBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
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
} as unknown as DocumentNode<BatchSoftDeleteRecordMutation, BatchSoftDeleteRecordMutationVariables>;
export const BatchRestoreRecordDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "batchRestoreRecord" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ids" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "batchRestoreRecord" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ids" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ids" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RecordBatch" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "records" },
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
} as unknown as DocumentNode<BatchRestoreRecordMutation, BatchRestoreRecordMutationVariables>;
export const TruncateRecordsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "truncateRecords" },
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
            name: { kind: "Name", value: "truncateRecords" },
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
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
} as unknown as DocumentNode<TruncateRecordsMutation, TruncateRecordsMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "key" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
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
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isOutput" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isArray" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isNullable" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
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
            name: { kind: "Name", value: "createTypeNode" },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "key" },
                      value: { kind: "Variable", name: { kind: "Name", value: "key" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "orderKey" },
                      value: { kind: "Variable", name: { kind: "Name", value: "orderKey" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isOutput" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isOutput" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isArray" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isArray" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isNullable" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isNullable" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "key" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "statement" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
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
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "deleteTypeNode" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
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
} as unknown as DocumentNode<DeleteTypeNodeMutation, DeleteTypeNodeMutationVariables>;
export const SoftDeleteTypeNodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteTypeNode" },
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
            name: { kind: "Name", value: "softDeleteTypeNode" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
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
} as unknown as DocumentNode<SoftDeleteTypeNodeMutation, SoftDeleteTypeNodeMutationVariables>;
export const RestoreTypeNodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreTypeNode" },
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
            name: { kind: "Name", value: "restoreStatementTypeNode" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
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
} as unknown as DocumentNode<RestoreTypeNodeMutation, RestoreTypeNodeMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isOutput" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isArray" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "isNullable" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
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
            name: { kind: "Name", value: "updateTypeNode" },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isOutput" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isOutput" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isArray" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isArray" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "isNullable" },
                      value: { kind: "Variable", name: { kind: "Name", value: "isNullable" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SimpleTypeNode" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "isOutput" } },
                      { kind: "Field", name: { kind: "Name", value: "isArray" } },
                      { kind: "Field", name: { kind: "Name", value: "isNullable" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateTypeNodeMutation, UpdateTypeNodeMutationVariables>;
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
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "email" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "completedSignup" } },
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
} as unknown as DocumentNode<CompleteSignupMutation, CompleteSignupMutationVariables>;
export const AcceptOrganizationInviteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "acceptOrganizationInvite" },
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
            name: { kind: "Name", value: "acceptOrganizationInvite" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "User" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "username" } },
                      { kind: "Field", name: { kind: "Name", value: "slug" } },
                      { kind: "Field", name: { kind: "Name", value: "email" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "completedSignup" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "organizationMemberships" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "node" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        { kind: "Field", name: { kind: "Name", value: "level" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "organization" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "Field", name: { kind: "Name", value: "id" } },
                                              { kind: "Field", name: { kind: "Name", value: "name" } },
                                              { kind: "Field", name: { kind: "Name", value: "slug" } },
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
} as unknown as DocumentNode<AcceptOrganizationInviteMutation, AcceptOrganizationInviteMutationVariables>;
export const UpdateVersionDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateVersion" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
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
            name: { kind: "Name", value: "updateProjectVersion" },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
                  },
                },
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
              ],
            },
          },
        ],
      },
    },
    ...ProjectVersionHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<UpdateVersionMutation, UpdateVersionMutationVariables>;
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
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "autoDeploy" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "autoDeploy" },
                      value: { kind: "Variable", name: { kind: "Name", value: "autoDeploy" } },
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
export const RestoreDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restore" },
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
            name: { kind: "Name", value: "restore" },
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
} as unknown as DocumentNode<RestoreMutation, RestoreMutationVariables>;
export const InterpChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "interpChanged" },
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
            name: { kind: "Name", value: "interpChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectVersionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpModuleContent" } }],
            },
          },
        ],
      },
    },
    ...InterpModuleContentFragmentDoc.definitions,
    ...InterpSymbolContentFragmentDoc.definitions,
    ...InterpErrorContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<InterpChangedSubscription, InterpChangedSubscriptionVariables>;
export const ModuleChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "moduleChanged" },
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
            name: { kind: "Name", value: "moduleChanged" },
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
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "clientId" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "mutations" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "fileId" } },
                      { kind: "Field", name: { kind: "Name", value: "statementId" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "input" } },
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
} as unknown as DocumentNode<ModuleChangedSubscription, ModuleChangedSubscriptionVariables>;
export const SystemInfoDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "systemInfo" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "systemInfo" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "version" } },
                { kind: "Field", name: { kind: "Name", value: "gitCommit" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SystemInfoQuery, SystemInfoQueryVariables>;
