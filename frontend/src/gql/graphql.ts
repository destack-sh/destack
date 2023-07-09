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

export type CancelRunInput = {
  executionId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
};

export type CancelRunPayload = {
  __typename?: "CancelRunPayload";
  execution?: Maybe<Execution>;
  success: Scalars["Boolean"];
};

export type CancelRunPayloadOperationInfo = CancelRunPayload | OperationInfo;

export type Change = {
  clientId?: Maybe<Scalars["GlobalID"]>;
  id: Scalars["UUID"];
};

export type Client = Node & {
  __typename?: "Client";
  active: Scalars["Boolean"];
  browserName?: Maybe<Scalars["String"]>;
  closedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  deviceName?: Maybe<Scalars["String"]>;
  fileId?: Maybe<Scalars["UUID"]>;
  id: Scalars["GlobalID"];
  lastSeenAt?: Maybe<Scalars["DateTime"]>;
  path?: Maybe<Scalars["String"]>;
  present: Scalars["Boolean"];
  project?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  statementId?: Maybe<Scalars["UUID"]>;
  type: ClientType;
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
  fieldId?: InputMaybe<Scalars["GlobalID"]>;
  fileId?: InputMaybe<Scalars["GlobalID"]>;
  id: Scalars["GlobalID"];
  path?: InputMaybe<Scalars["String"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  recordId?: InputMaybe<Scalars["GlobalID"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]>;
  type: ClientType;
};

export type CommitInput = {
  description?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  projectVersionId: Scalars["GlobalID"];
  tag?: InputMaybe<Scalars["String"]>;
};

export type CommitPayload = {
  __typename?: "CommitPayload";
  committedVersion: ProjectVersion;
  project: Project;
};

export type CommitPayloadOperationInfo = CommitPayload | OperationInfo;

export type CrudModel = {
  createdAt: Scalars["DateTime"];
  createdBy?: Maybe<User>;
  deletedAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  lastEditedAt?: Maybe<Scalars["DateTime"]>;
  lastEditedBy?: Maybe<User>;
  updatedAt: Scalars["DateTime"];
};

export type Dataset = Node & {
  __typename?: "Dataset";
  backend: Scalars["String"];
  backendId: Scalars["String"];
  id: Scalars["GlobalID"];
  versioned: Scalars["Boolean"];
};

export type DatasetQuery = {
  key?: InputMaybe<Scalars["String"]>;
  op: QueryOp;
  queries?: InputMaybe<Array<DatasetQuery>>;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type DatasetSort = {
  key: Scalars["String"];
  mode?: InputMaybe<SortMode>;
  order?: SortOrder;
};

export type DeleteObjectInput = {
  id: Scalars["GlobalID"];
};

export type Execution = Node & {
  __typename?: "Execution";
  accessToken?: Maybe<AccessToken>;
  cachedDuration?: Maybe<Scalars["Float"]>;
  cachedGeneratedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  descendants: Array<Execution>;
  duration?: Maybe<Scalars["Float"]>;
  error?: Maybe<Scalars["JSON"]>;
  errorNice?: Maybe<RunError>;
  id: Scalars["GlobalID"];
  inputs?: Maybe<Scalars["JSON"]>;
  metadata?: Maybe<Scalars["JSON"]>;
  outputs?: Maybe<Scalars["JSON"]>;
  parent?: Maybe<Execution>;
  project: Project;
  projectVersion: ProjectVersion;
  root?: Maybe<Execution>;
  runnable?: Maybe<Statement>;
  startedAt?: Maybe<Scalars["DateTime"]>;
  status: ExecutionStatus;
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

export enum ExecutionTriggerType {
  Api = "API",
  Reactive = "REACTIVE",
  Scheduled = "SCHEDULED",
  Ui = "UI",
}

export type Field = CrudModel &
  ModuleNode &
  Node & {
    __typename?: "Field";
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    description?: Maybe<Scalars["String"]>;
    flags: Scalars["Int"];
    hint?: Maybe<TypeHint>;
    id: Scalars["GlobalID"];
    key: Scalars["String"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    metadata?: Maybe<Scalars["JSON"]>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent: Statement;
    reference?: Maybe<Statement>;
    revision: Scalars["Int"];
    statement: Statement;
    tag: TypeTag;
    updatedAt: Scalars["DateTime"];
  };

export type FieldCreateInput = {
  description?: InputMaybe<Scalars["String"]>;
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  key: Scalars["String"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
};

export type FieldDeleteInput = {
  id: Scalars["GlobalID"];
};

export type FieldFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type FieldMoveInput = {
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
};

export type FieldOperationInfo = Field | OperationInfo;

export type FieldRenameInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
};

export type FieldRestoreInput = {
  id: Scalars["GlobalID"];
};

export type FieldUpdateDescriptionInput = {
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
};

export type FieldUpdateInput = {
  description?: InputMaybe<Scalars["String"]>;
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  name?: InputMaybe<Scalars["String"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  tag: TypeTag;
};

export type FieldUpdateTypeInput = {
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  tag: TypeTag;
};

export type File = CrudModel &
  ModuleNode &
  Node & {
    __typename?: "File";
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    directory: Scalars["Boolean"];
    files: Array<File>;
    id: Scalars["GlobalID"];
    issues: Array<Issue>;
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    name: Scalars["String"];
    parent: ModuleNode;
    projectVersion: ProjectVersion;
    revision: Scalars["Int"];
    statements: Array<Statement>;
    updatedAt: Scalars["DateTime"];
  };

export type FileIssuesArgs = {
  filters?: InputMaybe<IssueFilter>;
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
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type FileMoveInput = {
  id: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
};

export type FileOperationInfo = File | OperationInfo;

export type FilePasteInput = {
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  sourceId: Scalars["GlobalID"];
  targetId?: InputMaybe<Scalars["GlobalID"]>;
  targetVersionId: Scalars["GlobalID"];
};

export type FileRenameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export enum InterpScope {
  File = "FILE",
  Module = "MODULE",
  Statement = "STATEMENT",
}

export type Issue = Node & {
  __typename?: "Issue";
  file?: Maybe<File>;
  id: Scalars["GlobalID"];
  kind: IssueKind;
  message?: Maybe<Scalars["String"]>;
  scope: InterpScope;
  statement?: Maybe<Statement>;
  type: IssueType;
};

export type IssueFilter = {
  scope: InterpScope;
};

export enum IssueKind {
  Error = "ERROR",
  Suggestion = "SUGGESTION",
  Warning = "WARNING",
}

export type IssueResolvedField = Issue | ResolvedField;

export enum IssueType {
  AmbiguousDefinition = "AMBIGUOUS_DEFINITION",
  CircularAncestry = "CIRCULAR_ANCESTRY",
  CircularUnion = "CIRCULAR_UNION",
  Internal = "INTERNAL",
  MismatchedUnion = "MISMATCHED_UNION",
  MissingReference = "MISSING_REFERENCE",
  UnknownImportSource = "UNKNOWN_IMPORT_SOURCE",
}

export type LangserverWakeInput = {
  projectVersionId: Scalars["GlobalID"];
};

export type LangserverWakePayload = {
  __typename?: "LangserverWakePayload";
  success: Scalars["Boolean"];
};

export type LangserverWakePayloadOperationInfo = LangserverWakePayload | OperationInfo;

export type ModuleChange = Change & {
  __typename?: "ModuleChange";
  clientId?: Maybe<Scalars["GlobalID"]>;
  id: Scalars["UUID"];
  mutations: Array<ModuleMutation>;
};

export type ModuleMutation = {
  __typename?: "ModuleMutation";
  data?: Maybe<IssueResolvedField>;
  fileId?: Maybe<Scalars["GlobalID"]>;
  input?: Maybe<Scalars["JSON"]>;
  projectVersionId: Scalars["GlobalID"];
  properties?: Maybe<Array<Scalars["String"]>>;
  revision?: Maybe<Scalars["Int"]>;
  statementId?: Maybe<Scalars["GlobalID"]>;
  type: ModuleMutationType;
};

/** Fine-grained atomic mutations for multiplayer modules. */
export enum ModuleMutationType {
  BumpFile = "BUMP_FILE",
  BumpStatement = "BUMP_STATEMENT",
  CreateField = "CREATE_FIELD",
  CreateFile = "CREATE_FILE",
  CreateIssue = "CREATE_ISSUE",
  CreateRecord = "CREATE_RECORD",
  CreateResolvedField = "CREATE_RESOLVED_FIELD",
  CreateStatement = "CREATE_STATEMENT",
  CreateTagging = "CREATE_TAGGING",
  DeleteField = "DELETE_FIELD",
  DeleteFile = "DELETE_FILE",
  DeleteIssue = "DELETE_ISSUE",
  DeleteRecord = "DELETE_RECORD",
  DeleteStatement = "DELETE_STATEMENT",
  DeleteTagging = "DELETE_TAGGING",
  MorphStatement = "MORPH_STATEMENT",
  MoveField = "MOVE_FIELD",
  MoveFile = "MOVE_FILE",
  MoveRecord = "MOVE_RECORD",
  MoveStatement = "MOVE_STATEMENT",
  MoveTagging = "MOVE_TAGGING",
  PasteFile = "PASTE_FILE",
  PasteStatement = "PASTE_STATEMENT",
  RenameField = "RENAME_FIELD",
  RenameFile = "RENAME_FILE",
  RenameStatement = "RENAME_STATEMENT",
  RestoreField = "RESTORE_FIELD",
  RestoreFile = "RESTORE_FILE",
  RestoreRecord = "RESTORE_RECORD",
  RestoreStatement = "RESTORE_STATEMENT",
  RestoreTagging = "RESTORE_TAGGING",
  SoftDeleteField = "SOFT_DELETE_FIELD",
  SoftDeleteFile = "SOFT_DELETE_FILE",
  SoftDeleteRecord = "SOFT_DELETE_RECORD",
  SoftDeleteStatement = "SOFT_DELETE_STATEMENT",
  SoftDeleteTagging = "SOFT_DELETE_TAGGING",
  TruncateFields = "TRUNCATE_FIELDS",
  TruncateFiles = "TRUNCATE_FILES",
  TruncateIssues = "TRUNCATE_ISSUES",
  TruncateRecords = "TRUNCATE_RECORDS",
  TruncateResolvedFields = "TRUNCATE_RESOLVED_FIELDS",
  TruncateStatements = "TRUNCATE_STATEMENTS",
  TruncateTaggings = "TRUNCATE_TAGGINGS",
  UpdateField = "UPDATE_FIELD",
  UpdateFieldDescription = "UPDATE_FIELD_DESCRIPTION",
  UpdateFieldMetadata = "UPDATE_FIELD_METADATA",
  UpdateFieldType = "UPDATE_FIELD_TYPE",
  UpdateFile = "UPDATE_FILE",
  UpdateRecord = "UPDATE_RECORD",
  UpdateStatement = "UPDATE_STATEMENT",
  UpdateStatementReference = "UPDATE_STATEMENT_REFERENCE",
  UpdateStatementText = "UPDATE_STATEMENT_TEXT",
  UpdateSymbolCode = "UPDATE_SYMBOL_CODE",
  UpdateSymbolDescription = "UPDATE_SYMBOL_DESCRIPTION",
  UpdateSymbolLanguage = "UPDATE_SYMBOL_LANGUAGE",
  UpdateSymbolModifier = "UPDATE_SYMBOL_MODIFIER",
  UpdateSymbolValue = "UPDATE_SYMBOL_VALUE",
  UpdateTagging = "UPDATE_TAGGING",
  UpdateTaggingMetadata = "UPDATE_TAGGING_METADATA",
}

export type ModuleNode = {
  id: Scalars["GlobalID"];
  parent?: Maybe<ModuleNode>;
};

export type Mutation = {
  __typename?: "Mutation";
  acceptOrganizationInvite: UserOperationInfo;
  batchMoveStatement: StatementBatchOperationInfo;
  batchPasteStatement: StatementBatchOperationInfo;
  batchRestoreRecord: RecordBatchOperationInfo;
  batchRestoreStatement: StatementBatchOperationInfo;
  batchSoftDeleteRecord: RecordBatchOperationInfo;
  batchSoftDeleteStatement: StatementBatchOperationInfo;
  cancelOrganizationInvite: OrganizationOperationInfo;
  cancelRun: CancelRunPayloadOperationInfo;
  closeClient?: Maybe<ClientOperationInfo>;
  commit: CommitPayloadOperationInfo;
  completeSignup: UserOperationInfo;
  createAccessToken: AccessTokenCreatePayloadOperationInfo;
  createField: FieldOperationInfo;
  createFile: FileOperationInfo;
  createOrganization: OrganizationOperationInfo;
  createOrganizationInvites: OrganizationOperationInfo;
  createProject: ProjectOperationInfo;
  createRecord: RecordOperationInfo;
  createSecret: SecretOperationInfo;
  createStatement: StatementOperationInfo;
  createTagging: TaggingOperationInfo;
  deleteField: FieldOperationInfo;
  deleteFile: FileOperationInfo;
  deleteObject: RemoteObjectOperationInfo;
  deleteRecord: RecordOperationInfo;
  deleteSecret?: Maybe<OperationInfo>;
  deleteStatement: StatementOperationInfo;
  deleteTagging: TaggingOperationInfo;
  langserverWake: LangserverWakePayloadOperationInfo;
  logout?: Maybe<OperationInfo>;
  markNotification: NotificationOperationInfo;
  morphStatement: StatementOperationInfo;
  moveField: FieldOperationInfo;
  moveFile: FileOperationInfo;
  moveRecord: RecordOperationInfo;
  moveStatement: StatementOperationInfo;
  notifyUploadedObject: RemoteObjectOperationInfo;
  pasteFile: FileOperationInfo;
  removeOrganizationMembership: OrganizationOperationInfo;
  renameFile: FileOperationInfo;
  renameStatement: StatementOperationInfo;
  requestUploadObject: RemoteObjectOperationInfo;
  restore: CommitPayloadOperationInfo;
  restoreFile: FileOperationInfo;
  restoreRecord: RecordOperationInfo;
  restoreStatement: StatementOperationInfo;
  restoreStatementField: FieldOperationInfo;
  restoreTagging: TaggingOperationInfo;
  revokeAccessToken: AccessTokenOperationInfo;
  run: RunStateOperationInfo;
  secretRootLogin: UserOperationInfo;
  softDeleteField: FieldOperationInfo;
  softDeleteFile: FileOperationInfo;
  softDeleteRecord: RecordOperationInfo;
  softDeleteStatement: StatementOperationInfo;
  softDeleteTagging: TaggingOperationInfo;
  updateField: FieldOperationInfo;
  updateFieldDescription: FieldOperationInfo;
  updateFieldName: FieldOperationInfo;
  updateFieldType: FieldOperationInfo;
  updateFile: FileOperationInfo;
  updateOrganization: OrganizationOperationInfo;
  updateOrganizationMembership: OrganizationMembershipOperationInfo;
  updatePresence: ClientOperationInfo;
  updateProjectName: ProjectOperationInfo;
  updateProjectVersion: ProjectVersionOperationInfo;
  updateProjectVisibility: ProjectOperationInfo;
  updateRecord: RecordOperationInfo;
  updateSecret: SecretOperationInfo;
  updateStatement: StatementOperationInfo;
  updateStatementReference: StatementOperationInfo;
  updateStatementText: StatementOperationInfo;
  updateSymbolCode: StatementOperationInfo;
  updateSymbolDescription: StatementOperationInfo;
  updateSymbolValue: StatementOperationInfo;
  updateTagging: TaggingOperationInfo;
  updateUser: UserOperationInfo;
  upsertClient: ClientOperationInfo;
};

export type MutationAcceptOrganizationInviteArgs = {
  id: Scalars["GlobalID"];
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

export type MutationCancelOrganizationInviteArgs = {
  id: Scalars["GlobalID"];
};

export type MutationCancelRunArgs = {
  input: CancelRunInput;
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

export type MutationCreateFieldArgs = {
  input: FieldCreateInput;
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

export type MutationCreateSecretArgs = {
  input: SecretCreateInput;
};

export type MutationCreateStatementArgs = {
  input: StatementCreateInput;
};

export type MutationCreateTaggingArgs = {
  input: TaggingCreateInput;
};

export type MutationDeleteFieldArgs = {
  input: FieldDeleteInput;
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

export type MutationDeleteSecretArgs = {
  input: SecretDeleteInput;
};

export type MutationDeleteStatementArgs = {
  input: StatementDeleteInput;
};

export type MutationDeleteTaggingArgs = {
  input: TaggingDeleteInput;
};

export type MutationLangserverWakeArgs = {
  input: LangserverWakeInput;
};

export type MutationMarkNotificationArgs = {
  input: NotificationMarkInput;
};

export type MutationMorphStatementArgs = {
  input: StatementMorphInput;
};

export type MutationMoveFieldArgs = {
  input: FieldMoveInput;
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

export type MutationNotifyUploadedObjectArgs = {
  input: NotifyUploadedObjectInput;
};

export type MutationPasteFileArgs = {
  input: FilePasteInput;
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

export type MutationRestoreStatementFieldArgs = {
  input: FieldRestoreInput;
};

export type MutationRestoreTaggingArgs = {
  input: TaggingRestoreInput;
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

export type MutationSoftDeleteFieldArgs = {
  input: FieldDeleteInput;
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

export type MutationSoftDeleteTaggingArgs = {
  input: TaggingDeleteInput;
};

export type MutationUpdateFieldArgs = {
  input: FieldUpdateInput;
};

export type MutationUpdateFieldDescriptionArgs = {
  input: FieldUpdateDescriptionInput;
};

export type MutationUpdateFieldNameArgs = {
  input: FieldRenameInput;
};

export type MutationUpdateFieldTypeArgs = {
  input: FieldUpdateTypeInput;
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

export type MutationUpdateSecretArgs = {
  input: SecretUpdateInput;
};

export type MutationUpdateStatementArgs = {
  input: StatementUpdateInput;
};

export type MutationUpdateStatementReferenceArgs = {
  input: StatementUpdateReferenceInput;
};

export type MutationUpdateStatementTextArgs = {
  input: StatementUpdateTextInput;
};

export type MutationUpdateSymbolCodeArgs = {
  input: SymbolUpdateCodeInput;
};

export type MutationUpdateSymbolDescriptionArgs = {
  input: SymbolUpdateDescriptionInput;
};

export type MutationUpdateSymbolValueArgs = {
  input: SymbolUpdateValueInput;
};

export type MutationUpdateTaggingArgs = {
  input: TaggingUpdateInput;
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
  description?: Maybe<Scalars["String"]>;
  head: ProjectVersion;
  id: Scalars["GlobalID"];
  migrationMappings: ProjectMigrationInfo;
  name: Scalars["String"];
  owner: UserOrganization;
  path: Scalars["String"];
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  versions: ProjectVersionConnection;
  visibility: ProjectVisibility;
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

export type ProjectChange = Change & {
  __typename?: "ProjectChange";
  clientId?: Maybe<Scalars["GlobalID"]>;
  id: Scalars["UUID"];
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

export type ProjectUpdateNameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type ProjectUpdateVisibilityInput = {
  id: Scalars["GlobalID"];
  visibility: ProjectVisibility;
};

export type ProjectVersion = CrudModel &
  ModuleNode &
  Node & {
    __typename?: "ProjectVersion";
    childRefs: RefMappingConnection;
    children: Array<ProjectVersion>;
    committed: Scalars["Boolean"];
    committedAt?: Maybe<Scalars["DateTime"]>;
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    description?: Maybe<Scalars["String"]>;
    files: FileConnection;
    id: Scalars["GlobalID"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]>;
    parent?: Maybe<ModuleNode>;
    parentRefs: RefMappingConnection;
    parents: Array<ProjectVersion>;
    project: Project;
    tag?: Maybe<Scalars["String"]>;
    updatedAt: Scalars["DateTime"];
  };

export type ProjectVersionChildRefsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<RefMappingFilter>;
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
  clients: ClientConnection;
  executions: ExecutionConnection;
  featuredProjects: ProjectConnection;
  file?: Maybe<File>;
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
  searchDataset: RecordConnection;
  secret?: Maybe<Secret>;
  statement?: Maybe<Statement>;
  systemInfo: SystemInfo;
  user?: Maybe<User>;
  userBySlug?: Maybe<User>;
  users: UserConnection;
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

export type QueryExecutionsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  includeAncestorVersions?: Scalars["Boolean"];
  last?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  rootIdNull?: Scalars["Boolean"];
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
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

export type QuerySearchDatasetArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  query?: InputMaybe<DatasetQuery>;
  sort?: InputMaybe<Array<DatasetSort>>;
  statementId: Scalars["GlobalID"];
};

export type QuerySecretArgs = {
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

export enum QueryOp {
  And = "AND",
  Disjoint = "DISJOINT",
  DoesNotExist = "DOES_NOT_EXIST",
  Equals = "EQUALS",
  Exists = "EXISTS",
  GreaterThan = "GREATER_THAN",
  GreaterThanOrEquals = "GREATER_THAN_OR_EQUALS",
  Intersects = "INTERSECTS",
  LessThan = "LESS_THAN",
  LessThanOrEquals = "LESS_THAN_OR_EQUALS",
  Matches = "MATCHES",
  Near = "NEAR",
  Not = "NOT",
  NotEquals = "NOT_EQUALS",
  Or = "OR",
  StartsWith = "STARTS_WITH",
  Within = "WITHIN",
}

export type Record = CrudModel & {
  __typename?: "Record";
  createdAt: Scalars["DateTime"];
  createdBy?: Maybe<User>;
  datasetId: Scalars["String"];
  deletedAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  lastEditedAt?: Maybe<Scalars["DateTime"]>;
  lastEditedBy?: Maybe<User>;
  orderKey?: Maybe<Scalars["String"]>;
  revision: Scalars["Int"];
  updatedAt: Scalars["DateTime"];
  value: Scalars["JSON"];
};

export type RecordBatch = {
  __typename?: "RecordBatch";
  records: Array<Record>;
};

export type RecordBatchOperationInfo = OperationInfo | RecordBatch;

export type RecordBatchRestoreInput = {
  ids: Array<Scalars["GlobalID"]>;
  statementId: Scalars["GlobalID"];
};

export type RecordBatchSoftDeleteInput = {
  ids: Array<Scalars["GlobalID"]>;
  statementId: Scalars["GlobalID"];
};

/** A connection to a list of items. */
export type RecordConnection = {
  __typename?: "RecordConnection";
  /** Contains the nodes in this connection */
  edges: Array<RecordEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes */
  totalCount?: Maybe<Scalars["Int"]>;
};

export type RecordCreateInput = {
  id: Scalars["GlobalID"];
  orderKey?: InputMaybe<Scalars["String"]>;
  statementId: Scalars["GlobalID"];
  value: Scalars["JSON"];
};

export type RecordDeleteInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

/** An edge in a connection. */
export type RecordEdge = {
  __typename?: "RecordEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Record;
};

export type RecordMoveInput = {
  id: Scalars["GlobalID"];
  orderKey?: InputMaybe<Scalars["String"]>;
  statementId: Scalars["GlobalID"];
};

export type RecordOperationInfo = OperationInfo | Record;

export type RecordRestoreInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

export type RecordUpdateInput = {
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  value: Scalars["JSON"];
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
};

export enum RefMappingKind {
  Commit = "COMMIT",
  Paste = "PASTE",
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

export type ResolvedField = Node & {
  __typename?: "ResolvedField";
  field: Field;
  id: Scalars["GlobalID"];
  statement?: Maybe<Statement>;
};

export type RestoreInput = {
  projectVersionId: Scalars["GlobalID"];
};

/** Wire-able representation of an exception. */
export type RunError = {
  __typename?: "RunError";
  kind: Scalars["String"];
  message: Scalars["String"];
  statementId?: Maybe<Scalars["GlobalID"]>;
  traceback?: Maybe<Array<PyFrame>>;
  type: Scalars["String"];
};

export type RunInput = {
  arguments?: InputMaybe<Scalars["JSON"]>;
  block?: Scalars["Boolean"];
  executionId?: InputMaybe<Scalars["GlobalID"]>;
  keyed?: Scalars["Boolean"];
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]>;
  trace?: Scalars["Int"];
};

export type RunState = {
  __typename?: "RunState";
  execution?: Maybe<Execution>;
  executionId?: Maybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  runnableId?: Maybe<Scalars["GlobalID"]>;
  success: Scalars["Boolean"];
};

export type RunStateOperationInfo = OperationInfo | RunState;

export type Secret = Node & {
  __typename?: "Secret";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  project: Project;
  sha512: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  valueRevealed: Scalars["JSON"];
};

export type SecretCreateInput = {
  name?: InputMaybe<Scalars["String"]>;
  projectId: Scalars["GlobalID"];
  value: Scalars["JSON"];
};

export type SecretDeleteInput = {
  id: Scalars["GlobalID"];
};

export type SecretOperationInfo = OperationInfo | Secret;

export type SecretUpdateInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  value: Scalars["JSON"];
};

export enum SortMode {
  Average = "AVERAGE",
  Max = "MAX",
  Median = "MEDIAN",
  Min = "MIN",
  Sum = "SUM",
}

export enum SortOrder {
  Ascending = "ASCENDING",
  Descending = "DESCENDING",
}

export type Statement = CrudModel &
  ModuleNode &
  Node & {
    __typename?: "Statement";
    children: Array<Statement>;
    code?: Maybe<Scalars["String"]>;
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    dataset?: Maybe<Dataset>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    descendants: Array<Statement>;
    description?: Maybe<Scalars["String"]>;
    fields: Array<Field>;
    file: File;
    id: Scalars["GlobalID"];
    issues?: Maybe<Array<Issue>>;
    key?: Maybe<Scalars["String"]>;
    lang?: Maybe<Scalars["String"]>;
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent?: Maybe<ModuleNode>;
    projectVersion: ProjectVersion;
    reference?: Maybe<Statement>;
    resolvedFields?: Maybe<Array<Field>>;
    revision: Scalars["Int"];
    rootTypeFlags?: Maybe<Scalars["Int"]>;
    rootTypeTag?: Maybe<TypeTag>;
    tags: Array<Tagging>;
    text?: Maybe<Scalars["String"]>;
    type: StatementType;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
  };

export type StatementFieldsArgs = {
  filters?: InputMaybe<FieldFilter>;
};

export type StatementIssuesArgs = {
  filters?: InputMaybe<IssueFilter>;
};

export type StatementResolvedFieldsArgs = {
  filters?: InputMaybe<FieldFilter>;
};

export type StatementTagsArgs = {
  filters?: InputMaybe<TaggingFilter>;
};

export type StatementBatch = {
  __typename?: "StatementBatch";
  statements: Array<Statement>;
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

/** Creates a full statement */
export type StatementCreateInput = {
  code?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  fileId: Scalars["GlobalID"];
  id?: InputMaybe<Scalars["GlobalID"]>;
  key?: InputMaybe<Scalars["String"]>;
  lang?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  text?: InputMaybe<Scalars["String"]>;
  type: StatementType;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type StatementDeleteInput = {
  id: Scalars["GlobalID"];
};

export type StatementFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type StatementMorphInput = {
  id: Scalars["GlobalID"];
  lang?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
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

export type StatementSoftDeleteInput = {
  id: Scalars["GlobalID"];
};

/** The type of Bench statement. */
export enum StatementType {
  Blank = "BLANK",
  Block = "BLOCK",
  Code = "CODE",
  Dataset = "DATASET",
  Expectation = "EXPECTATION",
  Model = "MODEL",
  Reference = "REFERENCE",
  Tag = "TAG",
  Task = "TASK",
  Text = "TEXT",
  Type = "TYPE",
  Value = "VALUE",
}

/** Updates a statement */
export type StatementUpdateInput = {
  code?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  key?: InputMaybe<Scalars["String"]>;
  lang?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey?: InputMaybe<Scalars["String"]>;
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  text?: InputMaybe<Scalars["String"]>;
  type?: InputMaybe<StatementType>;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type StatementUpdateReferenceInput = {
  id: Scalars["GlobalID"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
};

export type StatementUpdateTextInput = {
  id: Scalars["GlobalID"];
  text?: InputMaybe<Scalars["String"]>;
};

export type Subscription = {
  __typename?: "Subscription";
  clientsChanged: Client;
  executionsChanged: Execution;
  moduleChanged: ModuleChange;
  projectChanged: ProjectChange;
};

export type SubscriptionClientsChangedArgs = {
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
};

export type SubscriptionExecutionsChangedArgs = {
  includeAncestorVersions?: Scalars["Boolean"];
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  rootId?: InputMaybe<Scalars["GlobalID"]>;
  rootIdNull?: Scalars["Boolean"];
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
};

export type SubscriptionModuleChangedArgs = {
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionProjectChangedArgs = {
  projectId: Scalars["GlobalID"];
};

export type SymbolUpdateCodeInput = {
  code?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
};

export type SymbolUpdateDescriptionInput = {
  description: Scalars["String"];
  id: Scalars["GlobalID"];
};

export type SymbolUpdateValueInput = {
  id: Scalars["GlobalID"];
  value?: InputMaybe<Scalars["JSON"]>;
};

export type SystemInfo = {
  __typename?: "SystemInfo";
  gitCommit: Scalars["String"];
  version: Scalars["String"];
};

export type Tagging = CrudModel &
  ModuleNode &
  Node & {
    __typename?: "Tagging";
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    id: Scalars["GlobalID"];
    key: Scalars["String"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    metadata?: Maybe<Scalars["JSON"]>;
    parent: Statement;
    reference: Statement;
    revision: Scalars["Int"];
    statement: Statement;
    updatedAt: Scalars["DateTime"];
  };

export type TaggingCreateInput = {
  id: Scalars["GlobalID"];
  key: Scalars["String"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  referenceId: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
};

export type TaggingDeleteInput = {
  id: Scalars["GlobalID"];
};

export type TaggingFilter = {
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type TaggingOperationInfo = OperationInfo | Tagging;

export type TaggingRestoreInput = {
  id: Scalars["GlobalID"];
};

export type TaggingUpdateInput = {
  id: Scalars["GlobalID"];
  metadata?: InputMaybe<Scalars["JSON"]>;
};

/** Extra representation/semantics of a field/type. */
export enum TypeHint {
  Audio = "AUDIO",
  Checkbox = "CHECKBOX",
  Code = "CODE",
  Date = "DATE",
  Datetime = "DATETIME",
  Duration = "DURATION",
  Email = "EMAIL",
  Embedding = "EMBEDDING",
  Float = "FLOAT",
  Html = "HTML",
  Image = "IMAGE",
  Integer = "INTEGER",
  Key = "KEY",
  Markdown = "MARKDOWN",
  Name = "NAME",
  Phone = "PHONE",
  Rating = "RATING",
  RichText = "RICH_TEXT",
  Secret = "SECRET",
  Slider = "SLIDER",
  Thumbs = "THUMBS",
  Time = "TIME",
  Toggle = "TOGGLE",
  Url = "URL",
  Uuid = "UUID",
  Video = "VIDEO",
}

/** The Bench primitive type of a field/type. */
export enum TypeTag {
  Any = "ANY",
  Boolean = "BOOLEAN",
  Enum = "ENUM",
  File = "FILE",
  Function = "FUNCTION",
  Json = "JSON",
  Literal = "LITERAL",
  Null = "NULL",
  Number = "NUMBER",
  String = "STRING",
  Struct = "STRUCT",
  TypeReference = "TYPE_REFERENCE",
  Union = "UNION",
  Vector = "VECTOR",
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

export type ExistingProjectVersionTagQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  tag: Scalars["String"];
}>;

export type ExistingProjectVersionTagQuery = {
  __typename?: "Query";
  projectVersionByTag?: { __typename?: "ProjectVersion"; id: any; tag?: string | null } | null;
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
        node: { __typename?: "File"; id: any; name: string; deletedAt?: any | null; directory: boolean };
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
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
        issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type StatementContentByIdQueryVariables = Exact<{
  statementId: Scalars["GlobalID"];
}>;

export type StatementContentByIdQuery = {
  __typename?: "Query";
  statement?:
    | ({
        __typename?: "Statement";
        id: any;
        deletedAt?: any | null;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        file: { __typename?: "File" } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } };
      } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } })
    | null;
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

export type SearchDatasetQueryVariables = Exact<{
  statementId: Scalars["GlobalID"];
  query?: InputMaybe<DatasetQuery>;
  sort?: InputMaybe<Array<DatasetSort> | DatasetSort>;
  after?: InputMaybe<Scalars["String"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
}>;

export type SearchDatasetQuery = {
  __typename?: "Query";
  searchDataset: {
    __typename?: "RecordConnection";
    totalCount?: number | null;
    pageInfo: {
      __typename?: "PageInfo";
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor?: string | null;
      endCursor?: string | null;
    };
    edges: Array<{
      __typename?: "RecordEdge";
      cursor: string;
      node: {
        __typename?: "Record";
        id: any;
        revision: number;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        orderKey?: string | null;
        value: any;
      };
    }>;
  };
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
        sourceId: any;
        sourceVersionId: any;
        targetId: any;
        targetVersionId: any;
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
  fileId?: any | null;
  statementId?: any | null;
  lastSeenAt?: any | null;
  closedAt?: any | null;
  active: boolean;
  present: boolean;
  user: { __typename?: "User"; id: any; name: string; username: string; email: string };
  project?: { __typename?: "Project"; id: any; name: string } | null;
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

type CrudModelContent_Field_Fragment = {
  __typename?: "Field";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_Field_Fragment" };

type CrudModelContent_File_Fragment = {
  __typename?: "File";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_File_Fragment" };

type CrudModelContent_ProjectVersion_Fragment = {
  __typename?: "ProjectVersion";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_ProjectVersion_Fragment" };

type CrudModelContent_Record_Fragment = {
  __typename?: "Record";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_Record_Fragment" };

type CrudModelContent_Statement_Fragment = {
  __typename?: "Statement";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_Statement_Fragment" };

type CrudModelContent_Tagging_Fragment = {
  __typename?: "Tagging";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "CrudModelContent_Tagging_Fragment" };

export type CrudModelContentFragment =
  | CrudModelContent_Field_Fragment
  | CrudModelContent_File_Fragment
  | CrudModelContent_ProjectVersion_Fragment
  | CrudModelContent_Record_Fragment
  | CrudModelContent_Statement_Fragment
  | CrudModelContent_Tagging_Fragment;

export type ProjectVersionHeaderFragment = {
  __typename?: "ProjectVersion";
  id: any;
  name?: string | null;
  tag?: string | null;
  description?: string | null;
  committed: boolean;
  committedAt?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parents: Array<{ __typename?: "ProjectVersion"; id: any }>;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type ProjectHeaderFragment = {
  __typename?: "Project";
  id: any;
  visibility: ProjectVisibility;
  createdAt: any;
  updatedAt: any;
  name: string;
  slug: string;
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
  directory: boolean;
  deletedAt?: any | null;
  createdAt: any;
  updatedAt: any;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field" }
    | { __typename?: "File"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "Statement" }
    | { __typename?: "Tagging" };
  projectVersion: { __typename?: "ProjectVersion"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "FileHeaderFragment" };

export type StatementHeaderFragment = {
  __typename?: "Statement";
  id: any;
  type: StatementType;
  revision: number;
  name?: string | null;
  orderKey: string;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent?:
    | { __typename?: "Field" }
    | { __typename?: "File"; id: any }
    | { __typename?: "ProjectVersion" }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging" }
    | null;
} & { " $fragmentName"?: "StatementHeaderFragment" };

export type FieldContentFragment = {
  __typename?: "Field";
  id: any;
  revision: number;
  name?: string | null;
  key: string;
  tag: TypeTag;
  hint?: TypeHint | null;
  flags: number;
  description?: string | null;
  orderKey: string;
  metadata?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  parent: { __typename?: "Statement"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "FieldContentFragment" };

export type TaggingContentFragment = {
  __typename?: "Tagging";
  id: any;
  revision: number;
  key: string;
  metadata?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent: { __typename?: "Statement"; id: any };
  reference: { __typename?: "Statement"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "TaggingContentFragment" };

export type StatementContentFragment = {
  __typename?: "Statement";
  id: any;
  type: StatementType;
  revision: number;
  name?: string | null;
  orderKey: string;
  key?: string | null;
  text?: string | null;
  lang?: string | null;
  code?: string | null;
  description?: string | null;
  value?: any | null;
  rootTypeTag?: TypeTag | null;
  rootTypeFlags?: number | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent?:
    | { __typename?: "Field" }
    | { __typename?: "File"; id: any }
    | { __typename?: "ProjectVersion" }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging" }
    | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  tags: Array<{ __typename?: "Tagging" } & { " $fragmentRefs"?: { TaggingContentFragment: TaggingContentFragment } }>;
  fields: Array<{ __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } }>;
  resolvedFields?: Array<
    { __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } }
  > | null;
  issues?: Array<
    { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
  > | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "StatementContentFragment" };

export type IssueContentFragment = {
  __typename?: "Issue";
  id: any;
  scope: InterpScope;
  kind: IssueKind;
  type: IssueType;
  message?: string | null;
  file?: { __typename?: "File"; id: any } | null;
  statement?: { __typename?: "Statement"; id: any } | null;
} & { " $fragmentName"?: "IssueContentFragment" };

export type ResolvedFieldContentFragment = {
  __typename?: "ResolvedField";
  id: any;
  statement?: { __typename?: "Statement"; id: any } | null;
  field: { __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } };
} & { " $fragmentName"?: "ResolvedFieldContentFragment" };

export type InterpFileFragment = {
  __typename?: "File";
  id: any;
  revision: number;
  name: string;
  directory: boolean;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field" }
    | { __typename?: "File"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "Statement" }
    | { __typename?: "Tagging" };
  issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
} & { " $fragmentName"?: "InterpFileFragment" };

export type InterpStatementFragment = {
  __typename?: "Statement";
  id: any;
  type: StatementType;
  name?: string | null;
  description?: string | null;
  revision: number;
  orderKey: string;
  key?: string | null;
  rootTypeTag?: TypeTag | null;
  rootTypeFlags?: number | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  file: { __typename?: "File"; id: any };
  parent?:
    | { __typename?: "Field" }
    | { __typename?: "File"; id: any }
    | { __typename?: "ProjectVersion" }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging" }
    | null;
  reference?: { __typename?: "Statement"; id: any } | null;
  tags: Array<{ __typename?: "Tagging" } & { " $fragmentRefs"?: { TaggingContentFragment: TaggingContentFragment } }>;
  fields: Array<{ __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } }>;
} & { " $fragmentName"?: "InterpStatementFragment" };

export type ModuleQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type ModuleQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    committed: boolean;
    project: { __typename?: "Project"; path: string; name: string };
    files: {
      __typename?: "FileConnection";
      edges: Array<{
        __typename?: "FileEdge";
        node: {
          __typename?: "File";
          statements: Array<
            {
              __typename?: "Statement";
              issues?: Array<
                { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
              > | null;
            } & { " $fragmentRefs"?: { InterpStatementFragment: InterpStatementFragment } }
          >;
        } & { " $fragmentRefs"?: { InterpFileFragment: InterpFileFragment } };
      }>;
    };
  } | null;
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
  fieldId?: InputMaybe<Scalars["GlobalID"]>;
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
  closeClient?:
    | { __typename?: "Client" }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | null;
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

export type CreateFileMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
  directory?: InputMaybe<Scalars["Boolean"]>;
  parentId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type CreateFileMutation = {
  __typename?: "Mutation";
  createFile:
    | ({
        __typename?: "File";
        id: any;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        statements: Array<
          {
            __typename?: "Statement";
            issues?: Array<
              { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
            > | null;
          } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
        issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
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
}>;

export type RenameFileMutation = {
  __typename?: "Mutation";
  renameFile:
    | { __typename?: "File"; id: any; name: string; revision: number }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type PasteFileMutationVariables = Exact<{
  sourceId: Scalars["GlobalID"];
  targetId?: InputMaybe<Scalars["GlobalID"]>;
  targetVersionId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
}>;

export type PasteFileMutation = {
  __typename?: "Mutation";
  pasteFile:
    | ({
        __typename?: "File";
        id: any;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
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

export type WakeLangserverMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type WakeLangserverMutation = {
  __typename?: "Mutation";
  langserverWake:
    | { __typename?: "LangserverWakePayload" }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RunMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  executionId?: InputMaybe<Scalars["GlobalID"]>;
  arguments?: InputMaybe<Scalars["JSON"]>;
  keyed?: InputMaybe<Scalars["Boolean"]>;
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
        success: boolean;
        execution?: {
          __typename?: "Execution";
          id: any;
          status: ExecutionStatus;
          startedAt?: any | null;
          terminatedAt?: any | null;
          createdAt: any;
          updatedAt: any;
          duration?: number | null;
          cachedGeneratedAt?: any | null;
          cachedDuration?: number | null;
          inputs?: any | null;
          outputs?: any | null;
          errorNice?: {
            __typename?: "RunError";
            type: string;
            message: string;
            traceback?: Array<{
              __typename?: "PyFrame";
              line: string;
              filename: string;
              lineno: number;
              name: string;
              locals?: any | null;
            }> | null;
          } | null;
        } | null;
      };
};

export type CancelMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  executionId: Scalars["GlobalID"];
}>;

export type CancelMutation = {
  __typename?: "Mutation";
  cancelRun:
    | {
        __typename?: "CancelRunPayload";
        success: boolean;
        execution?: {
          __typename?: "Execution";
          id: any;
          status: ExecutionStatus;
          startedAt?: any | null;
          terminatedAt?: any | null;
          createdAt: any;
          updatedAt: any;
          duration?: number | null;
          cachedGeneratedAt?: any | null;
          cachedDuration?: number | null;
        } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CreateSecretMutationVariables = Exact<{
  projectId: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  value: Scalars["JSON"];
}>;

export type CreateSecretMutation = {
  __typename?: "Mutation";
  createSecret:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Secret"; id: any; createdAt: any; updatedAt: any; sha512: string; name?: string | null };
};

export type UpdateSecretMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  value: Scalars["JSON"];
}>;

export type UpdateSecretMutation = {
  __typename?: "Mutation";
  updateSecret:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Secret"; id: any; createdAt: any; updatedAt: any; sha512: string; name?: string | null };
};

export type DeleteSecretMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteSecretMutation = {
  __typename?: "Mutation";
  deleteSecret?:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | null;
};

export type CreateStatementMutationVariables = Exact<{
  id?: InputMaybe<Scalars["GlobalID"]>;
  fileId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  key?: InputMaybe<Scalars["String"]>;
  lang?: InputMaybe<Scalars["String"]>;
  code?: InputMaybe<Scalars["String"]>;
  text?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  value?: InputMaybe<Scalars["JSON"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
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
        name?: string | null;
        orderKey: string;
        key?: string | null;
        lang?: string | null;
        code?: string | null;
        text?: string | null;
        description?: string | null;
        value?: any | null;
        rootTypeTag?: TypeTag | null;
        rootTypeFlags?: number | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        file: { __typename?: "File"; id: any };
        parent?:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "ProjectVersion" }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging" }
          | null;
        reference?: { __typename?: "Statement"; id: any } | null;
        tags: Array<{ __typename?: "Tagging"; id: any }>;
        fields: Array<{ __typename?: "Field"; id: any }>;
        resolvedFields?: Array<{ __typename?: "Field"; id: any }> | null;
        issues?: Array<{ __typename?: "Issue"; id: any }> | null;
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      };
};

export type UpdateStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  lang?: InputMaybe<Scalars["String"]>;
  code?: InputMaybe<Scalars["String"]>;
  text?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  value?: InputMaybe<Scalars["JSON"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
}>;

export type UpdateStatementMutation = {
  __typename?: "Mutation";
  updateStatement:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Statement";
        id: any;
        type: StatementType;
        revision: number;
        updatedAt: any;
        name?: string | null;
        orderKey: string;
        lang?: string | null;
        code?: string | null;
        text?: string | null;
        description?: string | null;
        value?: any | null;
        rootTypeTag?: TypeTag | null;
        rootTypeFlags?: number | null;
      };
};

export type MorphStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  rootTypeTag?: InputMaybe<TypeTag>;
  rootTypeFlags?: InputMaybe<Scalars["Int"]>;
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
        name?: string | null;
        rootTypeTag?: TypeTag | null;
        rootTypeFlags?: number | null;
        lang?: string | null;
      };
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
        parent?:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "ProjectVersion" }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging" }
          | null;
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
          parent?:
            | { __typename?: "Field" }
            | { __typename?: "File"; id: any }
            | { __typename?: "ProjectVersion" }
            | { __typename?: "Statement"; id: any }
            | { __typename?: "Tagging" }
            | null;
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

export type UpdateStatementReferenceMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  referenceId: Scalars["GlobalID"];
}>;

export type UpdateStatementReferenceMutation = {
  __typename?: "Mutation";
  updateStatementReference:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; revision: number; reference?: { __typename?: "Statement"; id: any } | null };
};

export type UpdateSymbolDescriptionMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  description: Scalars["String"];
}>;

export type UpdateSymbolDescriptionMutation = {
  __typename?: "Mutation";
  updateSymbolDescription:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; description?: string | null; revision: number };
};

export type UpdateSymbolCodeMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  code?: InputMaybe<Scalars["String"]>;
}>;

export type UpdateSymbolCodeMutation = {
  __typename?: "Mutation";
  updateSymbolCode:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; code?: string | null; revision: number };
};

export type UpdateStatementTextMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  text?: InputMaybe<Scalars["String"]>;
}>;

export type UpdateStatementTextMutation = {
  __typename?: "Mutation";
  updateStatementText:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; text?: string | null; revision: number };
};

export type UpdateSymbolValueMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  value?: InputMaybe<Scalars["JSON"]>;
}>;

export type UpdateSymbolValueMutation = {
  __typename?: "Mutation";
  updateSymbolValue:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; value?: any | null; revision: number };
};

export type CreateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  orderKey?: InputMaybe<Scalars["String"]>;
  value: Scalars["JSON"];
}>;

export type CreateRecordMutation = {
  __typename?: "Mutation";
  createRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Record";
        id: any;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        revision: number;
        orderKey?: string | null;
        value: any;
      };
};

export type UpdateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  value: Scalars["JSON"];
}>;

export type UpdateRecordMutation = {
  __typename?: "Mutation";
  updateRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; updatedAt: any; revision: number; value: any };
};

export type DeleteRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type DeleteRecordMutation = {
  __typename?: "Mutation";
  deleteRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; deletedAt?: any | null };
};

export type SoftDeleteRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type SoftDeleteRecordMutation = {
  __typename?: "Mutation";
  softDeleteRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; deletedAt?: any | null };
};

export type RestoreRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type RestoreRecordMutation = {
  __typename?: "Mutation";
  restoreRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; deletedAt?: any | null };
};

export type BatchSoftDeleteRecordMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type BatchSoftDeleteRecordMutation = {
  __typename?: "Mutation";
  batchSoftDeleteRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "RecordBatch"; records: Array<{ __typename?: "Record"; id: any; deletedAt?: any | null }> };
};

export type BatchRestoreRecordMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]> | Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
}>;

export type BatchRestoreRecordMutation = {
  __typename?: "Mutation";
  batchRestoreRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "RecordBatch"; records: Array<{ __typename?: "Record"; id: any; deletedAt?: any | null }> };
};

export type CreateFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  hint?: InputMaybe<TypeHint>;
  key: Scalars["String"];
  orderKey: Scalars["String"];
  name: Scalars["String"];
  description?: InputMaybe<Scalars["String"]>;
  flags: Scalars["Int"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  metadata?: InputMaybe<Scalars["JSON"]>;
}>;

export type CreateFieldMutation = {
  __typename?: "Mutation";
  createField:
    | {
        __typename?: "Field";
        id: any;
        key: string;
        orderKey: string;
        revision: number;
        name?: string | null;
        tag: TypeTag;
        hint?: TypeHint | null;
        description?: string | null;
        flags: number;
        metadata?: any | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        statement: { __typename?: "Statement"; id: any };
        parent: { __typename?: "Statement"; id: any };
        reference?: { __typename?: "Statement"; id: any } | null;
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteFieldMutation = {
  __typename?: "Mutation";
  deleteField:
    | { __typename?: "Field"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type SoftDeleteFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteFieldMutation = {
  __typename?: "Mutation";
  softDeleteField:
    | { __typename?: "Field"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RestoreFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreFieldMutation = {
  __typename?: "Mutation";
  restoreStatementField:
    | { __typename?: "Field"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type UpdateFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  tag: TypeTag;
  hint?: InputMaybe<TypeHint>;
  name?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
  flags: Scalars["Int"];
  referenceId?: InputMaybe<Scalars["GlobalID"]>;
  metadata?: InputMaybe<Scalars["JSON"]>;
}>;

export type UpdateFieldMutation = {
  __typename?: "Mutation";
  updateField:
    | {
        __typename?: "Field";
        id: any;
        tag: TypeTag;
        hint?: TypeHint | null;
        updatedAt: any;
        revision: number;
        name?: string | null;
        description?: string | null;
        flags: number;
        metadata?: any | null;
        reference?: { __typename?: "Statement"; id: any } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type MoveFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
}>;

export type MoveFieldMutation = {
  __typename?: "Mutation";
  moveField:
    | { __typename?: "Field"; id: any; orderKey: string }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CreateTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  statementId: Scalars["GlobalID"];
  key: Scalars["String"];
  referenceId: Scalars["GlobalID"];
  metadata?: InputMaybe<Scalars["JSON"]>;
}>;

export type CreateTaggingMutation = {
  __typename?: "Mutation";
  createTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Tagging";
        id: any;
        revision: number;
        key: string;
        metadata?: any | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        parent: { __typename?: "Statement"; id: any };
        reference: { __typename?: "Statement"; id: any };
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      };
};

export type DeleteTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type DeleteTaggingMutation = {
  __typename?: "Mutation";
  deleteTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Tagging"; id: any; deletedAt?: any | null };
};

export type SoftDeleteTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type SoftDeleteTaggingMutation = {
  __typename?: "Mutation";
  softDeleteTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Tagging"; id: any; deletedAt?: any | null };
};

export type RestoreTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type RestoreTaggingMutation = {
  __typename?: "Mutation";
  restoreTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Tagging"; id: any; deletedAt?: any | null };
};

export type UpdateTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  metadata?: InputMaybe<Scalars["JSON"]>;
}>;

export type UpdateTaggingMutation = {
  __typename?: "Mutation";
  updateTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Tagging"; id: any; updatedAt: any; revision: number; metadata?: any | null };
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
}>;

export type CommitMutation = {
  __typename?: "Mutation";
  commit:
    | {
        __typename?: "CommitPayload";
        project: {
          __typename?: "Project";
          head: { __typename?: "ProjectVersion" } & {
            " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
          };
        } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } };
        committedVersion: { __typename?: "ProjectVersion" } & {
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
        project: {
          __typename?: "Project";
          head: { __typename?: "ProjectVersion" } & {
            " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
          };
        } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } };
        committedVersion: { __typename?: "ProjectVersion" } & {
          " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
        };
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RevealSecretQueryVariables = Exact<{
  secretId: Scalars["GlobalID"];
}>;

export type RevealSecretQuery = {
  __typename?: "Query";
  secret?: { __typename?: "Secret"; id: any; sha512: string; valueRevealed: any } | null;
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
  cachedGeneratedAt?: any | null;
  status: ExecutionStatus;
  triggerType: ExecutionTriggerType;
  inputs?: any | null;
  outputs?: any | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any; tag?: string | null; name?: string | null };
  user?: { __typename?: "User"; id: any; slug: string } | null;
  accessToken?: { __typename?: "AccessToken"; id: any; name?: string | null } | null;
  root?: { __typename?: "Execution"; id: any } | null;
  parent?: { __typename?: "Execution"; id: any } | null;
  errorNice?: {
    __typename?: "RunError";
    type: string;
    message: string;
    traceback?: Array<{
      __typename?: "PyFrame";
      line: string;
      filename: string;
      lineno: number;
      name: string;
      locals?: any | null;
    }> | null;
  } | null;
  runnable?: { __typename?: "Statement"; id: any; name?: string | null } | null;
} & { " $fragmentName"?: "ExecutionContentFragment" };

export type ExecutionsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  includeAncestorVersions?: InputMaybe<Scalars["Boolean"]>;
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
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
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  rootIdNull?: InputMaybe<Scalars["Boolean"]>;
}>;

export type ExecutionsChangedSubscription = {
  __typename?: "Subscription";
  executionsChanged: { __typename?: "Execution" } & {
    " $fragmentRefs"?: { ExecutionContentFragment: ExecutionContentFragment };
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
      data?:
        | ({ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } })
        | ({ __typename?: "ResolvedField" } & {
            " $fragmentRefs"?: { ResolvedFieldContentFragment: ResolvedFieldContentFragment };
          })
        | null;
    }>;
  };
};

export type ProjectChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"];
}>;

export type ProjectChangedSubscription = {
  __typename?: "Subscription";
  projectChanged: { __typename?: "ProjectChange"; id: any; clientId?: any | null };
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
          { kind: "Field", name: { kind: "Name", value: "fileId" } },
          { kind: "Field", name: { kind: "Name", value: "statementId" } },
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
export const CrudModelContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "CrudModelContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "CrudModel" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CrudModelContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
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
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "directory" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "projectVersion" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
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
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementHeaderFragment, unknown>;
export const TaggingContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
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
          { kind: "Field", name: { kind: "Name", value: "metadata" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TaggingContentFragment, unknown>;
export const FieldContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
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
          { kind: "Field", name: { kind: "Name", value: "metadata" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<FieldContentFragment, unknown>;
export const IssueContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "scope" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<IssueContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "lang" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeFlags" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "tags" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TaggingContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "fields" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "FieldContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "FieldContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "issues" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filters" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scope" },
                      value: { kind: "EnumValue", value: "STATEMENT" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "createdBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "lastEditedBy" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementContentFragment, unknown>;
export const ResolvedFieldContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
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
            name: { kind: "Name", value: "field" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "FieldContent" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ResolvedFieldContentFragment, unknown>;
export const InterpFileFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpFile" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "directory" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "issues" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filters" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scope" },
                      value: { kind: "EnumValue", value: "FILE" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpFileFragment, unknown>;
export const InterpStatementFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpStatement" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
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
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
          { kind: "Field", name: { kind: "Name", value: "rootTypeFlags" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "reference" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "tags" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TaggingContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "fields" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "FieldContent" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpStatementFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "cachedGeneratedAt" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "errorNice" },
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
                      { kind: "Field", name: { kind: "Name", value: "locals" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "runnable" },
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "issues" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "filters" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "scope" },
                            value: { kind: "EnumValue", value: "FILE" },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
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
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<FileContentByIdQuery, FileContentByIdQueryVariables>;
export const StatementContentByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "statementContentById" },
      variableDefinitions: [
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
                  name: { kind: "Name", value: "projectVersion" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "file" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } }],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } },
              ],
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<StatementContentByIdQuery, StatementContentByIdQueryVariables>;
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
export const SearchDatasetDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "searchDataset" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "query" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "DatasetQuery" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sort" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "DatasetSort" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "after" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "limit" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "count" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "searchDataset" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "statementId" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "query" },
                value: { kind: "Variable", name: { kind: "Name", value: "query" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "sort" },
                value: { kind: "Variable", name: { kind: "Name", value: "sort" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "after" },
                value: { kind: "Variable", name: { kind: "Name", value: "after" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "limit" },
                value: { kind: "Variable", name: { kind: "Name", value: "limit" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "count" },
                value: { kind: "Variable", name: { kind: "Name", value: "count" } },
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
                            { kind: "Field", name: { kind: "Name", value: "value" } },
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
} as unknown as DocumentNode<SearchDatasetQuery, SearchDatasetQueryVariables>;
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
              ],
            },
          },
        ],
      },
    },
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
export const ModuleDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "module" },
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "project" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "path" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                    ],
                  },
                },
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
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "InterpFile" } },
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
                                        { kind: "FragmentSpread", name: { kind: "Name", value: "InterpStatement" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "issues" },
                                          arguments: [
                                            {
                                              kind: "Argument",
                                              name: { kind: "Name", value: "filters" },
                                              value: {
                                                kind: "ObjectValue",
                                                fields: [
                                                  {
                                                    kind: "ObjectField",
                                                    name: { kind: "Name", value: "scope" },
                                                    value: { kind: "EnumValue", value: "STATEMENT" },
                                                  },
                                                ],
                                              },
                                            },
                                          ],
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              { kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } },
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
    ...InterpFileFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...InterpStatementFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ModuleQuery, ModuleQueryVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "fieldId" } },
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
                      name: { kind: "Name", value: "fieldId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "fieldId" } },
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
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "StatementContent" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "issues" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "filters" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "scope" },
                                        value: { kind: "EnumValue", value: "STATEMENT" },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
                              },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "issues" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "scope" },
                                  value: { kind: "EnumValue", value: "FILE" },
                                },
                              ],
                            },
                          },
                        ],
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
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
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
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
} as unknown as DocumentNode<RenameFileMutation, RenameFileMutationVariables>;
export const PasteFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "pasteFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sourceId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "pasteFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sourceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sourceId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "targetVersionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetVersionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "parentId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
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
                        name: { kind: "Name", value: "issues" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
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
    ...IssueContentFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<PasteFileMutation, PasteFileMutationVariables>;
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
export const WakeLangserverDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "wakeLangserver" },
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
            name: { kind: "Name", value: "langserverWake" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } }],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<WakeLangserverMutation, WakeLangserverMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "executionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "arguments" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "keyed" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
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
                      name: { kind: "Name", value: "executionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "executionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "arguments" },
                      value: { kind: "Variable", name: { kind: "Name", value: "arguments" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "keyed" },
                      value: { kind: "Variable", name: { kind: "Name", value: "keyed" } },
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
                      { kind: "Field", name: { kind: "Name", value: "success" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "execution" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "status" } },
                            { kind: "Field", name: { kind: "Name", value: "startedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "duration" } },
                            { kind: "Field", name: { kind: "Name", value: "cachedGeneratedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "cachedDuration" } },
                            { kind: "Field", name: { kind: "Name", value: "inputs" } },
                            { kind: "Field", name: { kind: "Name", value: "outputs" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "errorNice" },
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
                                        { kind: "Field", name: { kind: "Name", value: "locals" } },
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
} as unknown as DocumentNode<RunMutation, RunMutationVariables>;
export const CancelDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "cancel" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "executionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "cancelRun" },
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
                      name: { kind: "Name", value: "executionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "executionId" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "CancelRunPayload" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "success" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "execution" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "status" } },
                            { kind: "Field", name: { kind: "Name", value: "startedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                            { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "duration" } },
                            { kind: "Field", name: { kind: "Name", value: "cachedGeneratedAt" } },
                            { kind: "Field", name: { kind: "Name", value: "cachedDuration" } },
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
} as unknown as DocumentNode<CancelMutation, CancelMutationVariables>;
export const CreateSecretDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createSecret" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createSecret" },
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
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Secret" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "sha512" } },
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
} as unknown as DocumentNode<CreateSecretMutation, CreateSecretMutationVariables>;
export const UpdateSecretDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateSecret" },
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
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateSecret" },
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
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Secret" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "sha512" } },
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
} as unknown as DocumentNode<UpdateSecretMutation, UpdateSecretMutationVariables>;
export const DeleteSecretDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteSecret" },
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
            name: { kind: "Name", value: "deleteSecret" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } }],
            },
          },
        ],
      },
    },
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<DeleteSecretMutation, DeleteSecretMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "key" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "text" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
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
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "lang" },
                      value: { kind: "Variable", name: { kind: "Name", value: "lang" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "key" },
                      value: { kind: "Variable", name: { kind: "Name", value: "key" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "code" },
                      value: { kind: "Variable", name: { kind: "Name", value: "code" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "text" },
                      value: { kind: "Variable", name: { kind: "Name", value: "text" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeTag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeFlags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "name" } },
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
                          selections: [
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                              },
                            },
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                              },
                            },
                          ],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "key" } },
                      { kind: "Field", name: { kind: "Name", value: "lang" } },
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeFlags" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "tags" },
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
                        name: { kind: "Name", value: "fields" },
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
                        name: { kind: "Name", value: "resolvedFields" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "issues" },
                        arguments: [
                          {
                            kind: "Argument",
                            name: { kind: "Name", value: "filters" },
                            value: {
                              kind: "ObjectValue",
                              fields: [
                                {
                                  kind: "ObjectField",
                                  name: { kind: "Name", value: "scope" },
                                  value: { kind: "EnumValue", value: "STATEMENT" },
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
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "createdBy" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "lastEditedBy" },
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
} as unknown as DocumentNode<CreateStatementMutation, CreateStatementMutationVariables>;
export const UpdateStatementDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatement" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "name" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "text" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "description" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateStatement" },
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
                      name: { kind: "Name", value: "name" },
                      value: { kind: "Variable", name: { kind: "Name", value: "name" } },
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
                      name: { kind: "Name", value: "text" },
                      value: { kind: "Variable", name: { kind: "Name", value: "text" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "description" },
                      value: { kind: "Variable", name: { kind: "Name", value: "description" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeTag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeTag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootTypeFlags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "lang" } },
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeFlags" } },
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
} as unknown as DocumentNode<UpdateStatementMutation, UpdateStatementMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
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
                      name: { kind: "Name", value: "rootTypeFlags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootTypeFlags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeTag" } },
                      { kind: "Field", name: { kind: "Name", value: "rootTypeFlags" } },
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
                          selections: [
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                              },
                            },
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
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
                                selections: [
                                  {
                                    kind: "InlineFragment",
                                    typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                                    },
                                  },
                                  {
                                    kind: "InlineFragment",
                                    typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
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
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<BatchPasteStatementMutation, BatchPasteStatementMutationVariables>;
export const UpdateStatementReferenceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateStatementReference" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
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
} as unknown as DocumentNode<UpdateStatementReferenceMutation, UpdateStatementReferenceMutationVariables>;
export const UpdateSymbolDescriptionDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateSymbolDescription" },
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
            name: { kind: "Name", value: "updateSymbolDescription" },
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
} as unknown as DocumentNode<UpdateSymbolDescriptionMutation, UpdateSymbolDescriptionMutationVariables>;
export const UpdateSymbolCodeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateSymbolCode" },
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
            name: { kind: "Name", value: "updateSymbolCode" },
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
} as unknown as DocumentNode<UpdateSymbolCodeMutation, UpdateSymbolCodeMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "text" } },
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
                      name: { kind: "Name", value: "text" },
                      value: { kind: "Variable", name: { kind: "Name", value: "text" } },
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
                      { kind: "Field", name: { kind: "Name", value: "text" } },
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
export const UpdateSymbolValueDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateSymbolValue" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateSymbolValue" },
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
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
} as unknown as DocumentNode<UpdateSymbolValueMutation, UpdateSymbolValueMutationVariables>;
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
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Record" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Record" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Record" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Record" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Record" } },
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
export const CreateFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createField" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "hint" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeHint" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createField" },
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
                      name: { kind: "Name", value: "hint" },
                      value: { kind: "Variable", name: { kind: "Name", value: "hint" } },
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
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "metadata" },
                      value: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parent" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "hint" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      { kind: "Field", name: { kind: "Name", value: "metadata" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "createdBy" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "lastEditedBy" },
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
} as unknown as DocumentNode<CreateFieldMutation, CreateFieldMutationVariables>;
export const DeleteFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteField" },
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
            name: { kind: "Name", value: "deleteField" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
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
} as unknown as DocumentNode<DeleteFieldMutation, DeleteFieldMutationVariables>;
export const SoftDeleteFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteField" },
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
            name: { kind: "Name", value: "softDeleteField" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
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
} as unknown as DocumentNode<SoftDeleteFieldMutation, SoftDeleteFieldMutationVariables>;
export const RestoreFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreField" },
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
            name: { kind: "Name", value: "restoreStatementField" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
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
} as unknown as DocumentNode<RestoreFieldMutation, RestoreFieldMutationVariables>;
export const UpdateFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateField" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "hint" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeHint" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateField" },
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
                      name: { kind: "Name", value: "hint" },
                      value: { kind: "Variable", name: { kind: "Name", value: "hint" } },
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
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "metadata" },
                      value: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "hint" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "description" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "metadata" } },
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
} as unknown as DocumentNode<UpdateFieldMutation, UpdateFieldMutationVariables>;
export const MoveFieldDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "moveField" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
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
            name: { kind: "Name", value: "moveField" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
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
} as unknown as DocumentNode<MoveFieldMutation, MoveFieldMutationVariables>;
export const CreateTaggingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createTagging" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "key" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createTagging" },
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
                      name: { kind: "Name", value: "key" },
                      value: { kind: "Variable", name: { kind: "Name", value: "key" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "metadata" },
                      value: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "key" } },
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
                      { kind: "Field", name: { kind: "Name", value: "metadata" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "createdBy" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "lastEditedBy" },
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
} as unknown as DocumentNode<CreateTaggingMutation, CreateTaggingMutationVariables>;
export const DeleteTaggingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "deleteTagging" },
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
            name: { kind: "Name", value: "deleteTagging" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
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
} as unknown as DocumentNode<DeleteTaggingMutation, DeleteTaggingMutationVariables>;
export const SoftDeleteTaggingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteTagging" },
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
            name: { kind: "Name", value: "softDeleteTagging" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
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
} as unknown as DocumentNode<SoftDeleteTaggingMutation, SoftDeleteTaggingMutationVariables>;
export const RestoreTaggingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreTagging" },
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
            name: { kind: "Name", value: "restoreTagging" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
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
} as unknown as DocumentNode<RestoreTaggingMutation, RestoreTaggingMutationVariables>;
export const UpdateTaggingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateTagging" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateTagging" },
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
                      name: { kind: "Name", value: "metadata" },
                      value: { kind: "Variable", name: { kind: "Name", value: "metadata" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "metadata" } },
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
} as unknown as DocumentNode<UpdateTaggingMutation, UpdateTaggingMutationVariables>;
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
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectHeader" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "head" },
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
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectHeader" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "head" },
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
export const RevealSecretDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "revealSecret" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "secretId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "secret" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: { kind: "Variable", name: { kind: "Name", value: "secretId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Secret" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "sha512" } },
                      { kind: "Field", name: { kind: "Name", value: "valueRevealed" } },
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
} as unknown as DocumentNode<RevealSecretQuery, RevealSecretQueryVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
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
                name: { kind: "Name", value: "runnableIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
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
                name: { kind: "Name", value: "runnableIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "data" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
                              },
                            },
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } },
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
    ...IssueContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ModuleChangedSubscription, ModuleChangedSubscriptionVariables>;
export const ProjectChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "projectChanged" },
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
            name: { kind: "Name", value: "projectChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "projectId" },
                value: { kind: "Variable", name: { kind: "Name", value: "projectId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "clientId" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectChangedSubscription, ProjectChangedSubscriptionVariables>;
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
