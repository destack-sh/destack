/* eslint-disable */
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends " $fragmentName" | "__typename" ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string };
  String: { input: string; output: string };
  Boolean: { input: boolean; output: boolean };
  Int: { input: number; output: number };
  Float: { input: number; output: number };
  /** Date with time (isoformat) */
  DateTime: { input: any; output: any };
  /** The `ID` scalar type represents a unique identifier, often used to refetch an object or as key for a cache. The ID type appears in a JSON response as a String; however, it is not intended to be human-readable. When expected as an input type, any string (such as `"4"`) or integer (such as `4`) input value will be accepted as an ID. */
  GlobalID: { input: any; output: any };
  /** The `JSON` scalar type represents JSON values as specified by [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf). */
  JSON: { input: any; output: any };
  UUID: { input: any; output: any };
};

export type AccessToken = Node & {
  __typename?: "AccessToken";
  createdAt: Scalars["DateTime"]["output"];
  expiresAt?: Maybe<Scalars["DateTime"]["output"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  name?: Maybe<Scalars["String"]["output"]>;
  owner: UserOrganization;
  revokedAt?: Maybe<Scalars["DateTime"]["output"]>;
  scopes: Array<AccessTokenScope>;
  status: AccessTokenStatus;
  token?: Maybe<Scalars["String"]["output"]>;
  tokenKey: Scalars["String"]["output"];
  updatedAt: Scalars["DateTime"]["output"];
};

/** A connection to a list of items. */
export type AccessTokenConnection = {
  __typename?: "AccessTokenConnection";
  /** Contains the nodes in this connection */
  edges: Array<AccessTokenEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

export type AccessTokenCreateInput = {
  expiresAt?: InputMaybe<Scalars["DateTime"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  ownerId: Scalars["GlobalID"]["input"];
  scopes: Array<AccessTokenScope>;
};

export type AccessTokenCreatePayload = {
  __typename?: "AccessTokenCreatePayload";
  accessToken: AccessToken;
  token: Scalars["String"]["output"];
};

export type AccessTokenCreatePayloadOperationInfo = AccessTokenCreatePayload | OperationInfo;

/** An edge in a connection. */
export type AccessTokenEdge = {
  __typename?: "AccessTokenEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: AccessToken;
};

export type AccessTokenFilter = {
  AND?: InputMaybe<AccessTokenFilter>;
  OR?: InputMaybe<AccessTokenFilter>;
  includeInactive?: InputMaybe<Scalars["Boolean"]["input"]>;
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

export type Blob = Node & {
  __typename?: "Blob";
  contentLength: Scalars["Int"]["output"];
  contentType: Scalars["String"]["output"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  name?: Maybe<Scalars["String"]["output"]>;
  presignedGet?: Maybe<Scalars["String"]["output"]>;
  presignedPost?: Maybe<Scalars["String"]["output"]>;
  sha512: Scalars["String"]["output"];
  status: BlobStatus;
};

export type BlobOperationInfo = Blob | OperationInfo;

export enum BlobStatus {
  Available = "AVAILABLE",
  Prepared = "PREPARED",
  Uploading = "UPLOADING",
}

export type Change = {
  clientId?: Maybe<Scalars["GlobalID"]["output"]>;
  id: Scalars["UUID"]["output"];
};

export type Client = Node & {
  __typename?: "Client";
  active: Scalars["Boolean"]["output"];
  browserName?: Maybe<Scalars["String"]["output"]>;
  closedAt?: Maybe<Scalars["DateTime"]["output"]>;
  createdAt: Scalars["DateTime"]["output"];
  deviceName?: Maybe<Scalars["String"]["output"]>;
  fileId?: Maybe<Scalars["UUID"]["output"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  lastSeenAt?: Maybe<Scalars["DateTime"]["output"]>;
  path?: Maybe<Scalars["String"]["output"]>;
  present: Scalars["Boolean"]["output"];
  project?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  statementId?: Maybe<Scalars["UUID"]["output"]>;
  type: ClientType;
  updatedAt: Scalars["DateTime"]["output"];
  user: User;
};

/** A connection to a list of items. */
export type ClientConnection = {
  __typename?: "ClientConnection";
  /** Contains the nodes in this connection */
  edges: Array<ClientEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type ClientEdge = {
  __typename?: "ClientEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Client;
};

export type ClientOperationInfo = Client | OperationInfo;

export enum ClientType {
  DesktopBrowser = "DesktopBrowser",
  MobileBrowser = "MobileBrowser",
}

export type ClientUpsertInput = {
  browserName?: InputMaybe<Scalars["String"]["input"]>;
  deviceName?: InputMaybe<Scalars["String"]["input"]>;
  fieldId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  fileId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  path?: InputMaybe<Scalars["String"]["input"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  recordId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  type: ClientType;
};

export type DeleteObjectInput = {
  id: Scalars["GlobalID"]["input"];
};

export type Edit = {
  __typename?: "Edit";
  data?: Maybe<ResolvedFieldIssue>;
  fileId?: Maybe<Scalars["GlobalID"]["output"]>;
  input?: Maybe<Scalars["JSON"]["output"]>;
  projectVersionId: Scalars["GlobalID"]["output"];
  properties?: Maybe<Array<Scalars["String"]["output"]>>;
  revision?: Maybe<Scalars["Int"]["output"]>;
  statementId?: Maybe<Scalars["GlobalID"]["output"]>;
  type: EditType;
};

export enum EditType {
  BumpFile = "BUMP_FILE",
  BumpStatement = "BUMP_STATEMENT",
  CreateField = "CREATE_FIELD",
  CreateFile = "CREATE_FILE",
  CreateIssue = "CREATE_ISSUE",
  CreateRecord = "CREATE_RECORD",
  CreateResolvedField = "CREATE_RESOLVED_FIELD",
  CreateStatement = "CREATE_STATEMENT",
  CreateTagging = "CREATE_TAGGING",
  CreateTrigger = "CREATE_TRIGGER",
  DeleteField = "DELETE_FIELD",
  DeleteFile = "DELETE_FILE",
  DeleteIssue = "DELETE_ISSUE",
  DeleteRecord = "DELETE_RECORD",
  DeleteResolvedField = "DELETE_RESOLVED_FIELD",
  DeleteStatement = "DELETE_STATEMENT",
  DeleteTagging = "DELETE_TAGGING",
  DeleteTrigger = "DELETE_TRIGGER",
  MorphStatement = "MORPH_STATEMENT",
  MoveField = "MOVE_FIELD",
  MoveFile = "MOVE_FILE",
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
  RestoreTrigger = "RESTORE_TRIGGER",
  SoftDeleteField = "SOFT_DELETE_FIELD",
  SoftDeleteFile = "SOFT_DELETE_FILE",
  SoftDeleteRecord = "SOFT_DELETE_RECORD",
  SoftDeleteStatement = "SOFT_DELETE_STATEMENT",
  SoftDeleteTagging = "SOFT_DELETE_TAGGING",
  SoftDeleteTrigger = "SOFT_DELETE_TRIGGER",
  TruncateIssues = "TRUNCATE_ISSUES",
  TruncateRecords = "TRUNCATE_RECORDS",
  TruncateResolvedFields = "TRUNCATE_RESOLVED_FIELDS",
  UpdateField = "UPDATE_FIELD",
  UpdateFieldMetadata = "UPDATE_FIELD_METADATA",
  UpdateFieldText = "UPDATE_FIELD_TEXT",
  UpdateFieldType = "UPDATE_FIELD_TYPE",
  UpdateFile = "UPDATE_FILE",
  UpdateRecord = "UPDATE_RECORD",
  UpdateStatement = "UPDATE_STATEMENT",
  UpdateStatementFlags = "UPDATE_STATEMENT_FLAGS",
  UpdateStatementHeadingLevel = "UPDATE_STATEMENT_HEADING_LEVEL",
  UpdateStatementReference = "UPDATE_STATEMENT_REFERENCE",
  UpdateStatementText = "UPDATE_STATEMENT_TEXT",
  UpdateSymbolCode = "UPDATE_SYMBOL_CODE",
  UpdateSymbolLanguage = "UPDATE_SYMBOL_LANGUAGE",
  UpdateSymbolModifier = "UPDATE_SYMBOL_MODIFIER",
  UpdateSymbolValue = "UPDATE_SYMBOL_VALUE",
  UpdateSymbolText = "UPDATE_SYMBOL_text",
  UpdateTagging = "UPDATE_TAGGING",
  UpdateTaggingMetadata = "UPDATE_TAGGING_METADATA",
  UpdateTrigger = "UPDATE_TRIGGER",
}

export type Environment = {
  __typename?: "Environment";
  language: Scalars["String"]["output"];
  packages: Array<Package>;
  platform: Scalars["String"]["output"];
  version: Scalars["String"]["output"];
};

export type EnvironmentOperationInfo = Environment | OperationInfo;

export type Field = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Field";
    ck: Scalars["UUID"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    flags: Scalars["Int"]["output"];
    hint?: Maybe<TypeHint>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    key: Scalars["String"]["output"];
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]["output"]>;
    orderKey: Scalars["String"]["output"];
    parent: Statement;
    referenceCk?: Maybe<Scalars["UUID"]["output"]>;
    revision: Scalars["Int"]["output"];
    statement: Statement;
    tag: TypeTag;
    text?: Maybe<Scalars["String"]["output"]>;
    updatedAt: Scalars["DateTime"]["output"];
    value?: Maybe<Scalars["JSON"]["output"]>;
  };

export type FieldCreateInput = {
  ck: Scalars["UUID"]["input"];
  flags?: Scalars["Int"]["input"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"]["input"];
  key: Scalars["String"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  orderKey: Scalars["String"]["input"];
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  statementId: Scalars["GlobalID"]["input"];
  tag: TypeTag;
  text?: InputMaybe<Scalars["String"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type FieldDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export type FieldFilter = {
  AND?: InputMaybe<FieldFilter>;
  OR?: InputMaybe<FieldFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type FieldMoveInput = {
  id: Scalars["GlobalID"]["input"];
  orderKey: Scalars["String"]["input"];
};

export type FieldOperationInfo = Field | OperationInfo;

export type FieldRenameInput = {
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
};

export type FieldRestoreInput = {
  id: Scalars["GlobalID"]["input"];
};

export type FieldUpdateInput = {
  flags?: Scalars["Int"]["input"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  tag: TypeTag;
  text?: InputMaybe<Scalars["String"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type FieldUpdateTextInput = {
  id: Scalars["GlobalID"]["input"];
  text?: InputMaybe<Scalars["String"]["input"]>;
};

export type FieldUpdateTypeInput = {
  flags?: Scalars["Int"]["input"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"]["input"];
  referenceCk?: InputMaybe<Scalars["GlobalID"]["input"]>;
  tag: TypeTag;
};

export type File = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "File";
    ck: Scalars["UUID"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    issues: Array<Issue>;
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    name: Scalars["String"]["output"];
    parent: ModuleNode;
    projectVersion: ProjectVersion;
    revision: Scalars["Int"]["output"];
    statements: Array<Statement>;
    updatedAt: Scalars["DateTime"]["output"];
  };

export type FileStatementsArgs = {
  filters?: InputMaybe<StatementFilter>;
};

export type FileCreateInput = {
  ck: Scalars["UUID"]["input"];
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId: Scalars["GlobalID"]["input"];
};

export type FileFilter = {
  AND?: InputMaybe<FileFilter>;
  OR?: InputMaybe<FileFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type FileMoveInput = {
  id: Scalars["GlobalID"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type FileOperationInfo = File | OperationInfo;

export type FilePasteInput = {
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sourceId: Scalars["GlobalID"]["input"];
  targetCk: Scalars["UUID"]["input"];
  targetId: Scalars["GlobalID"]["input"];
  targetVersionId: Scalars["GlobalID"]["input"];
};

export type FileRenameInput = {
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
};

export type FileUpdateInput = {
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type HasCrud = {
  createdAt: Scalars["DateTime"]["output"];
  createdBy?: Maybe<User>;
  deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
  id: Scalars["GlobalID"]["output"];
  lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
  lastEditedBy?: Maybe<User>;
  updatedAt: Scalars["DateTime"]["output"];
};

export type HasTriggeredBy = {
  trigger?: Maybe<Trigger>;
  triggerAccessToken?: Maybe<AccessToken>;
  triggerType?: Maybe<TriggerType>;
  triggerUser?: Maybe<User>;
};

export type Issue = ModuleNode &
  Node & {
    __typename?: "Issue";
    ck: Scalars["UUID"]["output"];
    id: Scalars["GlobalID"]["output"];
    kind: IssueKind;
    message?: Maybe<Scalars["String"]["output"]>;
    parent?: Maybe<ModuleNode>;
    type: IssueType;
  };

export enum IssueKind {
  Error = "Error",
  Notice = "Notice",
  Warning = "Warning",
}

export enum IssueType {
  AmbiguousDefinition = "AMBIGUOUS_DEFINITION",
  CircularAncestry = "CIRCULAR_ANCESTRY",
  CircularUnion = "CIRCULAR_UNION",
  CodeNotCacheable = "CODE_NOT_CACHEABLE",
  CodeNotExportable = "CODE_NOT_EXPORTABLE",
  CodeReferenceNotExported = "CODE_REFERENCE_NOT_EXPORTED",
  Internal = "INTERNAL",
  MismatchedUnion = "MISMATCHED_UNION",
  MissingReference = "MISSING_REFERENCE",
  TaskIsStatic = "TASK_IS_STATIC",
  TaskMissingIo = "TASK_MISSING_IO",
  UnknownImportSource = "UNKNOWN_IMPORT_SOURCE",
}

export type KillRunInput = {
  projectVersionId: Scalars["GlobalID"]["input"];
  restartIfUnresponsive: Scalars["Boolean"]["input"];
  runId: Scalars["GlobalID"]["input"];
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type KillRunPayload = {
  __typename?: "KillRunPayload";
  run?: Maybe<Run>;
};

export type KillRunPayloadOperationInfo = KillRunPayload | OperationInfo;

export type LogChange = {
  __typename?: "LogChange";
  logs: Array<LogEntry>;
};

export type LogEntry = {
  __typename?: "LogEntry";
  createdAt: Scalars["DateTime"]["output"];
  id: Scalars["GlobalID"]["output"];
  level?: Maybe<Scalars["String"]["output"]>;
  logger?: Maybe<Scalars["String"]["output"]>;
  message?: Maybe<Scalars["String"]["output"]>;
  projectVersionId: Scalars["GlobalID"]["output"];
  runId?: Maybe<Scalars["GlobalID"]["output"]>;
  sessionId?: Maybe<Scalars["GlobalID"]["output"]>;
  statementCk?: Maybe<Scalars["UUID"]["output"]>;
  statementId?: Maybe<Scalars["GlobalID"]["output"]>;
  stream: Scalars["String"]["output"];
  value?: Maybe<Scalars["JSON"]["output"]>;
};

/** A connection to a list of items. */
export type LogEntryConnection = {
  __typename?: "LogEntryConnection";
  /** Contains the nodes in this connection */
  edges: Array<LogEntryEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type LogEntryEdge = {
  __typename?: "LogEntryEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: LogEntry;
};

export enum ModuleAccessLevel {
  Admin = "Admin",
  Edit = "Edit",
  Manage = "Manage",
  Read = "Read",
  Use = "Use",
  Zero = "Zero",
}

export type ModuleChange = Change & {
  __typename?: "ModuleChange";
  clientId?: Maybe<Scalars["GlobalID"]["output"]>;
  edits: Array<Edit>;
  id: Scalars["UUID"]["output"];
};

export type ModuleNode = {
  ck: Scalars["UUID"]["output"];
  id: Scalars["GlobalID"]["output"];
  parent?: Maybe<ModuleNode>;
};

export type Mutation = {
  __typename?: "Mutation";
  acceptOrganizationInvite: UserOperationInfo;
  acceptProjectInvite: UserOperationInfo;
  batchMoveStatement: StatementBatchOperationInfo;
  batchPasteStatement: StatementBatchOperationInfo;
  batchRestoreRecord: RecordBatchOperationInfo;
  batchRestoreStatement: StatementBatchOperationInfo;
  batchSoftDeleteRecord: RecordBatchOperationInfo;
  batchSoftDeleteStatement: StatementBatchOperationInfo;
  cancelOrganizationInvite: OrganizationOperationInfo;
  cancelProjectInvite: ProjectOperationInfo;
  closeClient?: Maybe<ClientOperationInfo>;
  completeSignup: UserOperationInfo;
  createAccessToken: AccessTokenCreatePayloadOperationInfo;
  createField: FieldOperationInfo;
  createFile: FileOperationInfo;
  createOrganization: OrganizationOperationInfo;
  createOrganizationInvites: OrganizationOperationInfo;
  createProject: ProjectOperationInfo;
  createProjectInvites: ProjectOperationInfo;
  createRecord: RecordOperationInfo;
  createSecret: SecretOperationInfo;
  createStatement: StatementOperationInfo;
  createTagging: TaggingOperationInfo;
  createTrigger: TriggerOperationInfo;
  deleteField: FieldOperationInfo;
  deleteFile: FileOperationInfo;
  deleteObject: BlobOperationInfo;
  deleteRecord: RecordOperationInfo;
  deleteSecret?: Maybe<OperationInfo>;
  deleteStatement: StatementOperationInfo;
  deleteTagging: TaggingOperationInfo;
  deleteTrigger: TriggerOperationInfo;
  killRun: KillRunPayloadOperationInfo;
  logout?: Maybe<OperationInfo>;
  markNotification: NotificationOperationInfo;
  morphStatement: StatementOperationInfo;
  moveField: FieldOperationInfo;
  moveFile: FileOperationInfo;
  moveStatement: StatementOperationInfo;
  notifyUploadedObject: BlobOperationInfo;
  pasteFile: FileOperationInfo;
  removeOrganizationMembership: OrganizationOperationInfo;
  removeProjectMembership: ProjectOperationInfo;
  renameFile: FileOperationInfo;
  renameStatement: StatementOperationInfo;
  requestUploadObject: BlobOperationInfo;
  restartWorkerSet: RestartWorkerSetPayloadOperationInfo;
  restore: SnapshotPayloadOperationInfo;
  restoreField: FieldOperationInfo;
  restoreFile: FileOperationInfo;
  restoreRecord: RecordOperationInfo;
  restoreStatement: StatementOperationInfo;
  restoreTagging: TaggingOperationInfo;
  restoreTrigger: TriggerOperationInfo;
  revokeAccessToken: AccessTokenOperationInfo;
  run: RunStateOperationInfo;
  secretRootLogin: UserOperationInfo;
  snapshot: SnapshotPayloadOperationInfo;
  softDeleteField: FieldOperationInfo;
  softDeleteFile: FileOperationInfo;
  softDeleteRecord: RecordOperationInfo;
  softDeleteStatement: StatementOperationInfo;
  softDeleteTagging: TaggingOperationInfo;
  softDeleteTrigger: TriggerOperationInfo;
  updateField: FieldOperationInfo;
  updateFieldName: FieldOperationInfo;
  updateFieldText: FieldOperationInfo;
  updateFieldType: FieldOperationInfo;
  updateFile: FileOperationInfo;
  updateOrganization: OrganizationOperationInfo;
  updateOrganizationMembership: OrganizationMembershipOperationInfo;
  updatePresence: ClientOperationInfo;
  updateProjectName: ProjectOperationInfo;
  updateProjectSharing: ProjectOperationInfo;
  updateProjectVersion: ProjectVersionOperationInfo;
  updateProjectVisibility: ProjectOperationInfo;
  updateRecord: RecordOperationInfo;
  updateSecret: SecretOperationInfo;
  updateStatement: StatementOperationInfo;
  updateStatementHeadingLevel: StatementOperationInfo;
  updateStatementReference: StatementOperationInfo;
  updateStatementText: StatementOperationInfo;
  updateSymbolCode: StatementOperationInfo;
  updateSymbolValue: StatementOperationInfo;
  updateTagging: TaggingOperationInfo;
  updateTrigger: TriggerOperationInfo;
  updateUser: UserOperationInfo;
  upsertClient: ClientOperationInfo;
  wakeRuntime: WakeRuntimePayloadOperationInfo;
  wakeWorkerSet: WakeWorkerSetPayloadOperationInfo;
};

export type MutationAcceptOrganizationInviteArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type MutationAcceptProjectInviteArgs = {
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
};

export type MutationCancelProjectInviteArgs = {
  id: Scalars["GlobalID"]["input"];
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

export type MutationCreateProjectInvitesArgs = {
  input: ProjectInviteInput;
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

export type MutationCreateTriggerArgs = {
  input: TriggerCreateInput;
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

export type MutationDeleteTriggerArgs = {
  input: TriggerDeleteInput;
};

export type MutationKillRunArgs = {
  input: KillRunInput;
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

export type MutationRemoveProjectMembershipArgs = {
  input: ProjectRemoveMembershipInput;
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

export type MutationRestartWorkerSetArgs = {
  input: RestartWorkerSetInput;
};

export type MutationRestoreArgs = {
  input: RestoreInput;
};

export type MutationRestoreFieldArgs = {
  input: FieldRestoreInput;
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

export type MutationRestoreTaggingArgs = {
  input: TaggingRestoreInput;
};

export type MutationRestoreTriggerArgs = {
  input: TriggerRestoreInput;
};

export type MutationRevokeAccessTokenArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type MutationRunArgs = {
  input: RunInput;
};

export type MutationSecretRootLoginArgs = {
  username: Scalars["String"]["input"];
};

export type MutationSnapshotArgs = {
  input: SnapshotInput;
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

export type MutationSoftDeleteTriggerArgs = {
  input: TriggerDeleteInput;
};

export type MutationUpdateFieldArgs = {
  input: FieldUpdateInput;
};

export type MutationUpdateFieldNameArgs = {
  input: FieldRenameInput;
};

export type MutationUpdateFieldTextArgs = {
  input: FieldUpdateTextInput;
};

export type MutationUpdateFieldTypeArgs = {
  input: FieldUpdateTypeInput;
};

export type MutationUpdateFileArgs = {
  input: FileUpdateInput;
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

export type MutationUpdateProjectSharingArgs = {
  input: ProjectUpdateSharingInput;
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

export type MutationUpdateStatementHeadingLevelArgs = {
  input: StatementUpdateHeadingLevelInput;
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

export type MutationUpdateSymbolValueArgs = {
  input: SymbolUpdateValueInput;
};

export type MutationUpdateTaggingArgs = {
  input: TaggingUpdateInput;
};

export type MutationUpdateTriggerArgs = {
  input: TriggerUpdateInput;
};

export type MutationUpdateUserArgs = {
  input: UserUpdateInput;
};

export type MutationUpsertClientArgs = {
  input: ClientUpsertInput;
};

export type MutationWakeRuntimeArgs = {
  input: WakeRuntimeInput;
};

export type MutationWakeWorkerSetArgs = {
  input: WakeWorkerSetInput;
};

/** An object with a Globally Unique ID */
export type Node = {
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
};

/** Input of an object that implements the `Node` interface. */
export type NodeInput = {
  id: Scalars["GlobalID"]["input"];
};

export type Notification = Node & {
  __typename?: "Notification";
  archivedAt?: Maybe<Scalars["DateTime"]["output"]>;
  createdAt: Scalars["DateTime"]["output"];
  expiresAt?: Maybe<Scalars["DateTime"]["output"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  organizationInvite: OrganizationInvite;
  projectInvite: ProjectInvite;
  readAt?: Maybe<Scalars["DateTime"]["output"]>;
  run: Run;
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
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type NotificationEdge = {
  __typename?: "NotificationEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Notification;
};

export type NotificationFilter = {
  AND?: InputMaybe<NotificationFilter>;
  OR?: InputMaybe<NotificationFilter>;
  createdAt_Gte?: InputMaybe<Scalars["DateTime"]["input"]>;
  notArchived?: InputMaybe<Scalars["Boolean"]["input"]>;
  status?: InputMaybe<NotificationStatus>;
};

export type NotificationMarkInput = {
  id: Scalars["GlobalID"]["input"];
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
  ProjectInvite = "PROJECT_INVITE",
  RunFailed = "RUN_FAILED",
  RunSuspended = "RUN_SUSPENDED",
}

export type NotifyUploadedObjectInput = {
  id: Scalars["GlobalID"]["input"];
};

export type OperationInfo = {
  __typename?: "OperationInfo";
  /** List of messages returned by the operation. */
  messages: Array<OperationMessage>;
};

export type OperationMessage = {
  __typename?: "OperationMessage";
  /** The field that caused the error, or `null` if it isn't associated with any particular field. */
  field?: Maybe<Scalars["String"]["output"]>;
  /** The kind of this message. */
  kind: OperationMessageKind;
  /** The error message. */
  message: Scalars["String"]["output"];
};

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
    canViewDetail: Scalars["Boolean"]["output"];
    canWrite: Scalars["Boolean"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    description?: Maybe<Scalars["String"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    invites: OrganizationInviteConnection;
    memberships: OrganizationMembershipConnection;
    name: Scalars["String"]["output"];
    projects: ProjectConnection;
    slug: Scalars["String"]["output"];
    updatedAt: Scalars["DateTime"]["output"];
  };

export type OrganizationAccessTokensArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  filters?: InputMaybe<AccessTokenFilter>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type OrganizationInvitesArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type OrganizationMembershipsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type OrganizationProjectsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

/** A connection to a list of items. */
export type OrganizationConnection = {
  __typename?: "OrganizationConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

export type OrganizationCreateInput = {
  name: Scalars["String"]["input"];
  slug: Scalars["String"]["input"];
};

/** An edge in a connection. */
export type OrganizationEdge = {
  __typename?: "OrganizationEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Organization;
};

export type OrganizationInvite = Node & {
  __typename?: "OrganizationInvite";
  createdAt: Scalars["DateTime"]["output"];
  email: Scalars["String"]["output"];
  emailSentAt?: Maybe<Scalars["DateTime"]["output"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  level: OrganizationRole;
  organization: Organization;
  updatedAt: Scalars["DateTime"]["output"];
  user?: Maybe<User>;
};

/** A connection to a list of items. */
export type OrganizationInviteConnection = {
  __typename?: "OrganizationInviteConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationInviteEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type OrganizationInviteEdge = {
  __typename?: "OrganizationInviteEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: OrganizationInvite;
};

export type OrganizationInviteInput = {
  emails: Array<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  level: OrganizationRole;
  message?: InputMaybe<Scalars["String"]["input"]>;
};

export type OrganizationMembership = Node & {
  __typename?: "OrganizationMembership";
  createdAt: Scalars["DateTime"]["output"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  level: OrganizationRole;
  organization: Organization;
  updatedAt: Scalars["DateTime"]["output"];
  user: User;
};

/** A connection to a list of items. */
export type OrganizationMembershipConnection = {
  __typename?: "OrganizationMembershipConnection";
  /** Contains the nodes in this connection */
  edges: Array<OrganizationMembershipEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type OrganizationMembershipEdge = {
  __typename?: "OrganizationMembershipEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: OrganizationMembership;
};

export type OrganizationMembershipOperationInfo = OperationInfo | OrganizationMembership;

export type OrganizationOperationInfo = OperationInfo | Organization;

export type OrganizationRemoveMembershipInput = {
  id: Scalars["GlobalID"]["input"];
  userId: Scalars["GlobalID"]["input"];
};

export enum OrganizationRole {
  Guest = "Guest",
  Manager = "Manager",
  Member = "Member",
  Owner = "Owner",
}

export type OrganizationUpdateInput = {
  description: Scalars["String"]["input"];
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
};

export type OrganizationUpdateMembershipInput = {
  id: Scalars["GlobalID"]["input"];
  level: OrganizationRole;
  userId: Scalars["GlobalID"]["input"];
};

export type Owner = {
  accessTokens: AccessTokenConnection;
  canViewDetail: Scalars["Boolean"]["output"];
  canWrite: Scalars["Boolean"]["output"];
  createdAt: Scalars["DateTime"]["output"];
  id: Scalars["GlobalID"]["output"];
  name: Scalars["String"]["output"];
  projects: ProjectConnection;
  slug: Scalars["String"]["output"];
  updatedAt: Scalars["DateTime"]["output"];
};

export type Package = {
  __typename?: "Package";
  name: Scalars["String"]["output"];
  version: Scalars["String"]["output"];
};

/** Information to aid in pagination. */
export type PageInfo = {
  __typename?: "PageInfo";
  /** When paginating forwards, the cursor to continue. */
  endCursor?: Maybe<Scalars["String"]["output"]>;
  /** When paginating forwards, are there more items? */
  hasNextPage: Scalars["Boolean"]["output"];
  /** When paginating backwards, are there more items? */
  hasPreviousPage: Scalars["Boolean"]["output"];
  /** When paginating backwards, the cursor to continue. */
  startCursor?: Maybe<Scalars["String"]["output"]>;
};

export type Project = Node & {
  __typename?: "Project";
  accessLevel: ModuleAccessLevel;
  createdAt: Scalars["DateTime"]["output"];
  description?: Maybe<Scalars["String"]["output"]>;
  head: ProjectVersion;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  name: Scalars["String"]["output"];
  owner: UserOrganization;
  path: Scalars["String"]["output"];
  sharingEnabled: Scalars["Boolean"]["output"];
  sharingLevel: ModuleAccessLevel;
  sharingToken?: Maybe<Scalars["UUID"]["output"]>;
  slug: Scalars["String"]["output"];
  updatedAt: Scalars["DateTime"]["output"];
  usage: ProjectUsage;
  versions: ProjectVersionConnection;
  visibility: ProjectVisibility;
  workerSet: WorkerSet;
  workerSets: Array<WorkerSet>;
};

export type ProjectVersionsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  filters?: InputMaybe<ProjectVersionFilter>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type ProjectChange = Change & {
  __typename?: "ProjectChange";
  clientId?: Maybe<Scalars["GlobalID"]["output"]>;
  id: Scalars["UUID"]["output"];
};

/** A connection to a list of items. */
export type ProjectConnection = {
  __typename?: "ProjectConnection";
  /** Contains the nodes in this connection */
  edges: Array<ProjectEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

export type ProjectCreateInput = {
  name: Scalars["String"]["input"];
  ownerId: Scalars["GlobalID"]["input"];
  slug: Scalars["String"]["input"];
  visibility: ProjectVisibility;
};

/** An edge in a connection. */
export type ProjectEdge = {
  __typename?: "ProjectEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Project;
};

export type ProjectInvite = Node & {
  __typename?: "ProjectInvite";
  createdAt: Scalars["DateTime"]["output"];
  email: Scalars["String"]["output"];
  emailSentAt?: Maybe<Scalars["DateTime"]["output"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  level: ModuleAccessLevel;
  project: Project;
  updatedAt: Scalars["DateTime"]["output"];
  user?: Maybe<User>;
};

export type ProjectInviteInput = {
  emails: Array<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  level: ModuleAccessLevel;
  message?: InputMaybe<Scalars["String"]["input"]>;
};

export type ProjectOperationInfo = OperationInfo | Project;

export type ProjectRemoveMembershipInput = {
  id: Scalars["GlobalID"]["input"];
  userId: Scalars["GlobalID"]["input"];
};

export type ProjectUpdateNameInput = {
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
};

export type ProjectUpdateSharingInput = {
  id: Scalars["GlobalID"]["input"];
  sharingEnabled: Scalars["Boolean"]["input"];
  sharingLevel: ModuleAccessLevel;
  sharingToken: Scalars["UUID"]["input"];
};

export type ProjectUpdateVisibilityInput = {
  id: Scalars["GlobalID"]["input"];
  visibility: ProjectVisibility;
};

export type ProjectUsage = {
  __typename?: "ProjectUsage";
  cacheBytesTotal: Scalars["Int"]["output"];
  objectsBytesTotal: Scalars["Int"]["output"];
  recordsActive: Scalars["Int"]["output"];
};

export type ProjectVersion = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "ProjectVersion";
    children: Array<ProjectVersion>;
    ck: Scalars["UUID"]["output"];
    committed: Scalars["Boolean"]["output"];
    committedAt?: Maybe<Scalars["DateTime"]["output"]>;
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    description?: Maybe<Scalars["String"]["output"]>;
    files: Array<File>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]["output"]>;
    parent?: Maybe<ModuleNode>;
    parents: Array<ProjectVersion>;
    project: Project;
    tag?: Maybe<Scalars["String"]["output"]>;
    updatedAt: Scalars["DateTime"]["output"];
  };

export type ProjectVersionFilesArgs = {
  filters?: InputMaybe<FileFilter>;
};

/** A connection to a list of items. */
export type ProjectVersionConnection = {
  __typename?: "ProjectVersionConnection";
  /** Contains the nodes in this connection */
  edges: Array<ProjectVersionEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type ProjectVersionEdge = {
  __typename?: "ProjectVersionEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: ProjectVersion;
};

export type ProjectVersionFilter = {
  AND?: InputMaybe<ProjectVersionFilter>;
  OR?: InputMaybe<ProjectVersionFilter>;
  fromId: Scalars["GlobalID"]["input"];
  toId: Scalars["GlobalID"]["input"];
};

export type ProjectVersionOperationInfo = OperationInfo | ProjectVersion;

export enum ProjectVisibility {
  Private = "PRIVATE",
  Public = "PUBLIC",
  SourcePrivate = "SOURCE_PRIVATE",
}

export type Query = {
  __typename?: "Query";
  blob?: Maybe<Blob>;
  clients: ClientConnection;
  currentRuns: SessionStateOperationInfo;
  environment: EnvironmentOperationInfo;
  featuredProjects: ProjectConnection;
  file?: Maybe<File>;
  me?: Maybe<User>;
  module?: Maybe<ProjectVersion>;
  organization?: Maybe<Organization>;
  ownerBySlug?: Maybe<UserOrganization>;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  projectVersionBySlug?: Maybe<ProjectVersion>;
  projectVersionByTag?: Maybe<ProjectVersion>;
  run?: Maybe<Run>;
  searchLogs: LogEntryConnection;
  searchRecords: RecordConnection;
  searchRuns: RunConnection;
  secret?: Maybe<Secret>;
  session?: Maybe<Session>;
  statement?: Maybe<Statement>;
  systemInfo: SystemInfo;
  user?: Maybe<User>;
  users: UserConnection;
};

export type QueryBlobArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryClientsArgs = {
  active?: InputMaybe<Scalars["Boolean"]["input"]>;
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  inSameOrganizations?: Scalars["Boolean"]["input"];
  last?: InputMaybe<Scalars["Int"]["input"]>;
  organizationId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  present?: InputMaybe<Scalars["Boolean"]["input"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  userId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type QueryCurrentRunsArgs = {
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
};

export type QueryEnvironmentArgs = {
  projectId: Scalars["GlobalID"]["input"];
};

export type QueryFeaturedProjectsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type QueryFileArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryModuleArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryOrganizationArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryOwnerBySlugArgs = {
  slug: Scalars["String"]["input"];
};

export type QueryProjectArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryProjectBySlugArgs = {
  owner: Scalars["String"]["input"];
  project: Scalars["String"]["input"];
};

export type QueryProjectVersionArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryProjectVersionBySlugArgs = {
  owner: Scalars["String"]["input"];
  project: Scalars["String"]["input"];
  tag: Scalars["String"]["input"];
};

export type QueryProjectVersionByTagArgs = {
  projectId: Scalars["GlobalID"]["input"];
  tag: Scalars["String"]["input"];
};

export type QueryRunArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QuerySearchLogsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  query?: InputMaybe<SearchQuery>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sort?: InputMaybe<Array<SearchSort>>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]>>;
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]>>;
};

export type QuerySearchRecordsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort>>;
  statementId: Scalars["GlobalID"]["input"];
};

export type QuerySearchRunsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
  query?: InputMaybe<SearchQuery>;
  rootOnly?: InputMaybe<Scalars["Boolean"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sort?: InputMaybe<Array<SearchSort>>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]>>;
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]>>;
};

export type QuerySecretArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QuerySessionArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryStatementArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryUserArgs = {
  id: Scalars["GlobalID"]["input"];
};

export type QueryUsersArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  filters?: InputMaybe<UserFilter>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
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

export type Record = HasCrud &
  Node & {
    __typename?: "Record";
    ck: Scalars["UUID"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    revision: Scalars["Int"]["output"];
    updatedAt: Scalars["DateTime"]["output"];
    value: Scalars["JSON"]["output"];
  };

export type RecordBatch = {
  __typename?: "RecordBatch";
  records: Array<Record>;
};

export type RecordBatchOperationInfo = OperationInfo | RecordBatch;

export type RecordBatchRestoreInput = {
  ids: Array<Scalars["GlobalID"]["input"]>;
  statementId: Scalars["GlobalID"]["input"];
};

export type RecordBatchSoftDeleteInput = {
  ids: Array<Scalars["GlobalID"]["input"]>;
  statementId: Scalars["GlobalID"]["input"];
};

/** A connection to a list of items. */
export type RecordConnection = {
  __typename?: "RecordConnection";
  /** Contains the nodes in this connection */
  edges: Array<RecordEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

export type RecordCreateInput = {
  ck: Scalars["UUID"]["input"];
  id: Scalars["GlobalID"]["input"];
  statementCk: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  statementKey: Scalars["String"]["input"];
  value: Scalars["JSON"]["input"];
};

export type RecordDeleteInput = {
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
};

/** An edge in a connection. */
export type RecordEdge = {
  __typename?: "RecordEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Record;
};

export type RecordOperationInfo = OperationInfo | Record;

export type RecordRestoreInput = {
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
};

export type RecordUpdateInput = {
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  value: Scalars["JSON"]["input"];
};

export type RequestUploadObjectInput = {
  contentLength: Scalars["Int"]["input"];
  contentType: Scalars["String"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  projectId: Scalars["GlobalID"]["input"];
  sha512: Scalars["String"]["input"];
};

export type ResolvedField = ModuleNode &
  Node & {
    __typename?: "ResolvedField";
    ck: Scalars["UUID"]["output"];
    fieldCk: Scalars["UUID"]["output"];
    id: Scalars["GlobalID"]["output"];
    orderKey: Scalars["String"]["output"];
    parent?: Maybe<ModuleNode>;
    statement?: Maybe<Statement>;
  };

export type ResolvedFieldIssue = Issue | ResolvedField;

export type RestartWorkerSetInput = {
  projectId: Scalars["GlobalID"]["input"];
};

export type RestartWorkerSetPayload = {
  __typename?: "RestartWorkerSetPayload";
  success: Scalars["Boolean"]["output"];
  workerSet?: Maybe<WorkerSet>;
};

export type RestartWorkerSetPayloadOperationInfo = OperationInfo | RestartWorkerSetPayload;

export type RestoreInput = {
  projectVersionId: Scalars["GlobalID"]["input"];
};

export type Run = HasTriggeredBy &
  Node & {
    __typename?: "Run";
    children: Array<Run>;
    createdAt: Scalars["DateTime"]["output"];
    descendants: Array<Run>;
    duration?: Maybe<Scalars["Float"]["output"]>;
    error?: Maybe<Scalars["JSON"]["output"]>;
    errorNice?: Maybe<RunError>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    inputs?: Maybe<Scalars["JSON"]["output"]>;
    outputs?: Maybe<Scalars["JSON"]["output"]>;
    parent?: Maybe<Run>;
    projectVersion: ProjectVersion;
    root?: Maybe<Run>;
    session?: Maybe<Session>;
    startedAt?: Maybe<Scalars["DateTime"]["output"]>;
    statement?: Maybe<Statement>;
    statementCk?: Maybe<Scalars["UUID"]["output"]>;
    status: RunStatus;
    terminatedAt?: Maybe<Scalars["DateTime"]["output"]>;
    trigger?: Maybe<Trigger>;
    triggerAccessToken?: Maybe<AccessToken>;
    triggerType?: Maybe<TriggerType>;
    triggerUser?: Maybe<User>;
    updatedAt: Scalars["DateTime"]["output"];
    value?: Maybe<Scalars["JSON"]["output"]>;
  };

export type RunCodeFrame = {
  __typename?: "RunCodeFrame";
  filename: Scalars["String"]["output"];
  line: Scalars["String"]["output"];
  lineno: Scalars["Int"]["output"];
  locals?: Maybe<Scalars["JSON"]["output"]>;
  name: Scalars["String"]["output"];
};

/** A connection to a list of items. */
export type RunConnection = {
  __typename?: "RunConnection";
  /** Contains the nodes in this connection */
  edges: Array<RunEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type RunEdge = {
  __typename?: "RunEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Run;
};

export type RunError = {
  __typename?: "RunError";
  kind: Scalars["String"]["output"];
  message: Scalars["String"]["output"];
  statementId?: Maybe<Scalars["GlobalID"]["output"]>;
  traceback?: Maybe<Array<RunCodeFrame>>;
  type: Scalars["String"]["output"];
};

export type RunInput = {
  accessLevel: Scalars["Int"]["input"];
  block?: Scalars["Float"]["input"];
  code?: InputMaybe<Scalars["String"]["input"]>;
  globalValue?: InputMaybe<Scalars["JSON"]["input"]>;
  inputs?: InputMaybe<Scalars["JSON"]["input"]>;
  keyed?: Scalars["Boolean"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
  rootValue?: InputMaybe<Scalars["JSON"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  tags?: InputMaybe<Array<Scalars["String"]["input"]>>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]["input"]>;
};

export type RunState = {
  __typename?: "RunState";
  error?: Maybe<StartRunErrorType>;
  logs?: Maybe<Array<LogEntry>>;
  projectVersionId: Scalars["GlobalID"]["output"];
  run?: Maybe<Run>;
  statementId?: Maybe<Scalars["GlobalID"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type RunStateOperationInfo = OperationInfo | RunState;

export enum RunStatus {
  Aborted = "Aborted",
  Aborting = "Aborting",
  Cancelled = "Cancelled",
  Completed = "Completed",
  Failed = "Failed",
  Queued = "Queued",
  Running = "Running",
  Scheduled = "Scheduled",
  Suspended = "Suspended",
}

export type RunsChange = {
  __typename?: "RunsChange";
  runs: Array<Run>;
};

export enum ScheduleType {
  Cron = "CRON",
  Interval = "INTERVAL",
}

export type SearchQuery = {
  key?: InputMaybe<Scalars["String"]["input"]>;
  op: QueryOp;
  queries?: InputMaybe<Array<SearchQuery>>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type SearchSort = {
  key: Scalars["String"]["input"];
  mode?: InputMaybe<SortMode>;
  order?: SortOrder;
};

export type Secret = Node & {
  __typename?: "Secret";
  createdAt: Scalars["DateTime"]["output"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  name?: Maybe<Scalars["String"]["output"]>;
  project: Project;
  sha512: Scalars["String"]["output"];
  updatedAt: Scalars["DateTime"]["output"];
  valueRevealed: Scalars["JSON"]["output"];
};

export type SecretCreateInput = {
  name?: InputMaybe<Scalars["String"]["input"]>;
  projectId: Scalars["GlobalID"]["input"];
  value: Scalars["JSON"]["input"];
};

export type SecretDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export type SecretOperationInfo = OperationInfo | Secret;

export type SecretUpdateInput = {
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  value: Scalars["JSON"]["input"];
};

export type Session = HasTriggeredBy &
  Node & {
    __typename?: "Session";
    closedAt?: Maybe<Scalars["DateTime"]["output"]>;
    createdAt: Scalars["DateTime"]["output"];
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    openedAt?: Maybe<Scalars["DateTime"]["output"]>;
    project: Project;
    runs: Array<Run>;
    trigger?: Maybe<Trigger>;
    triggerAccessToken?: Maybe<AccessToken>;
    triggerType?: Maybe<TriggerType>;
    triggerUser?: Maybe<User>;
    updatedAt: Scalars["DateTime"]["output"];
  };

export type SessionChange = {
  __typename?: "SessionChange";
  runs: Array<Run>;
  session?: Maybe<Session>;
};

export type SessionChangeRunsChangeWorkerChange = RunsChange | SessionChange | WorkerChange;

export type SessionState = {
  __typename?: "SessionState";
  runs: Array<Run>;
  workerSet?: Maybe<WorkerSet>;
};

export type SessionStateOperationInfo = OperationInfo | SessionState;

export type SnapshotInput = {
  description?: InputMaybe<Scalars["String"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  projectVersionId: Scalars["GlobalID"]["input"];
  tag?: InputMaybe<Scalars["String"]["input"]>;
};

export type SnapshotPayload = {
  __typename?: "SnapshotPayload";
  project: Project;
  snapshot: ProjectVersion;
};

export type SnapshotPayloadOperationInfo = OperationInfo | SnapshotPayload;

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

export enum StartRunErrorType {
  AlreadyScheduled = "ALREADY_SCHEDULED",
  InternalError = "INTERNAL_ERROR",
  InvalidRun = "INVALID_RUN",
  RuntimeError = "RUNTIME_ERROR",
  Timeout = "TIMEOUT",
  Unavailable = "UNAVAILABLE",
}

export type Statement = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Statement";
    ck: Scalars["UUID"]["output"];
    code?: Maybe<Scalars["String"]["output"]>;
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    descendants: Array<Statement>;
    fields: Array<Field>;
    file: File;
    headingLevel?: Maybe<Scalars["Int"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    issues?: Maybe<Array<Issue>>;
    key?: Maybe<Scalars["String"]["output"]>;
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]["output"]>;
    orderKey: Scalars["String"]["output"];
    parent: ModuleNode;
    projectVersion: ProjectVersion;
    referenceCk?: Maybe<Scalars["UUID"]["output"]>;
    resolvedFields?: Maybe<Array<ResolvedField>>;
    revision: Scalars["Int"]["output"];
    tags: Array<Tagging>;
    text?: Maybe<Scalars["String"]["output"]>;
    triggers: Array<Trigger>;
    type: StatementType;
    updatedAt: Scalars["DateTime"]["output"];
    value?: Maybe<Scalars["JSON"]["output"]>;
    versioned: Scalars["Boolean"]["output"];
  };

export type StatementFieldsArgs = {
  filters?: InputMaybe<FieldFilter>;
};

export type StatementTagsArgs = {
  filters?: InputMaybe<TaggingFilter>;
};

export type StatementTriggersArgs = {
  filters?: InputMaybe<TriggerFilter>;
};

export type StatementBatch = {
  __typename?: "StatementBatch";
  statements: Array<Statement>;
};

export type StatementBatchMoveInput = {
  fileId: Scalars["GlobalID"]["input"];
  ids: Array<Scalars["GlobalID"]["input"]>;
  orderKeys: Array<Scalars["String"]["input"]>;
  parentIds: Array<InputMaybe<Scalars["GlobalID"]["input"]>>;
};

export type StatementBatchOperationInfo = OperationInfo | StatementBatch;

export type StatementBatchPasteInput = {
  sourceIds: Array<Scalars["GlobalID"]["input"]>;
  targetCks: Array<Scalars["UUID"]["input"]>;
  targetFileId: Scalars["GlobalID"]["input"];
  targetIds: Array<Scalars["GlobalID"]["input"]>;
  targetOrderKeys: Array<Scalars["String"]["input"]>;
  targetParentIds: Array<InputMaybe<Scalars["GlobalID"]["input"]>>;
};

export type StatementBatchRestoreInput = {
  ids: Array<Scalars["GlobalID"]["input"]>;
};

export type StatementBatchSoftDeleteInput = {
  ids: Array<Scalars["GlobalID"]["input"]>;
};

export type StatementCreateInput = {
  ck: Scalars["UUID"]["input"];
  code?: InputMaybe<Scalars["String"]["input"]>;
  fileId: Scalars["GlobalID"]["input"];
  id: Scalars["GlobalID"]["input"];
  key?: InputMaybe<Scalars["String"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  orderKey: Scalars["String"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  type: StatementType;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
  versioned: Scalars["Boolean"]["input"];
};

export type StatementDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export type StatementFilter = {
  AND?: InputMaybe<StatementFilter>;
  OR?: InputMaybe<StatementFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type StatementMorphInput = {
  headingLevel?: InputMaybe<Scalars["Int"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  key?: InputMaybe<Scalars["String"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  type: StatementType;
  versioned: Scalars["Boolean"]["input"];
};

export type StatementMoveInput = {
  fileId: Scalars["GlobalID"]["input"];
  id: Scalars["GlobalID"]["input"];
  orderKey?: InputMaybe<Scalars["String"]["input"]>;
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type StatementOperationInfo = OperationInfo | Statement;

export type StatementRenameInput = {
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
};

export type StatementRestoreInput = {
  id: Scalars["GlobalID"]["input"];
};

export type StatementSoftDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export enum StatementType {
  Blank = "BLANK",
  Choice = "CHOICE",
  Class = "CLASS",
  Code = "CODE",
  Database = "DATABASE",
  Flow = "FLOW",
  Model = "MODEL",
  Reference = "REFERENCE",
  Tag = "TAG",
  Task = "TASK",
  Text = "TEXT",
  Variable = "VARIABLE",
}

export type StatementUpdateHeadingLevelInput = {
  headingLevel?: InputMaybe<Scalars["Int"]["input"]>;
  id: Scalars["GlobalID"]["input"];
};

export type StatementUpdateInput = {
  code?: InputMaybe<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  key?: InputMaybe<Scalars["String"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  orderKey?: InputMaybe<Scalars["String"]["input"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  type?: InputMaybe<StatementType>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type StatementUpdateReferenceInput = {
  id: Scalars["GlobalID"]["input"];
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
};

export type StatementUpdateTextInput = {
  id: Scalars["GlobalID"]["input"];
  text?: InputMaybe<Scalars["String"]["input"]>;
};

export type Subscription = {
  __typename?: "Subscription";
  clientsChanged: Client;
  logsChanged: LogChange;
  moduleChanged: ModuleChange;
  projectChanged: ProjectChange;
  sessionsChanged: SessionChangeRunsChangeWorkerChange;
};

export type SubscriptionClientsChangedArgs = {
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type SubscriptionLogsChangedArgs = {
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]>>;
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]>>;
};

export type SubscriptionModuleChangedArgs = {
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
};

export type SubscriptionProjectChangedArgs = {
  projectId: Scalars["GlobalID"]["input"];
};

export type SubscriptionSessionsChangedArgs = {
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
};

export type SymbolUpdateCodeInput = {
  code?: InputMaybe<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
};

export type SymbolUpdateValueInput = {
  id: Scalars["GlobalID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type SystemInfo = {
  __typename?: "SystemInfo";
  gitCommit: Scalars["String"]["output"];
  version: Scalars["String"]["output"];
};

export type Tagging = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Tagging";
    ck: Scalars["UUID"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    key: Scalars["String"]["output"];
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    parent: Statement;
    referenceCk?: Maybe<Scalars["UUID"]["output"]>;
    revision: Scalars["Int"]["output"];
    statement: Statement;
    updatedAt: Scalars["DateTime"]["output"];
    value?: Maybe<Scalars["JSON"]["output"]>;
  };

export type TaggingCreateInput = {
  ck: Scalars["UUID"]["input"];
  id: Scalars["GlobalID"]["input"];
  key: Scalars["String"]["input"];
  referenceCk: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type TaggingDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export type TaggingFilter = {
  AND?: InputMaybe<TaggingFilter>;
  OR?: InputMaybe<TaggingFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type TaggingOperationInfo = OperationInfo | Tagging;

export type TaggingRestoreInput = {
  id: Scalars["GlobalID"]["input"];
};

export type TaggingUpdateInput = {
  id: Scalars["GlobalID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
};

export type Trigger = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Trigger";
    active: Scalars["Boolean"]["output"];
    ck: Scalars["UUID"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    createdBy?: Maybe<User>;
    cron?: Maybe<Scalars["String"]["output"]>;
    deletedAt?: Maybe<Scalars["DateTime"]["output"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    interval?: Maybe<Scalars["Int"]["output"]>;
    lastEditedAt?: Maybe<Scalars["DateTime"]["output"]>;
    lastEditedBy?: Maybe<User>;
    mapping?: Maybe<Scalars["JSON"]["output"]>;
    parent: Statement;
    revision: Scalars["Int"]["output"];
    scheduleType: ScheduleType;
    scopeCk?: Maybe<Scalars["UUID"]["output"]>;
    statementCk?: Maybe<Scalars["UUID"]["output"]>;
    timezone?: Maybe<Scalars["String"]["output"]>;
    type: TriggerType;
    updatedAt: Scalars["DateTime"]["output"];
  };

export type TriggerCreateInput = {
  active: Scalars["Boolean"]["input"];
  ck: Scalars["UUID"]["input"];
  cron?: InputMaybe<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  interval?: InputMaybe<Scalars["Int"]["input"]>;
  mapping?: InputMaybe<Scalars["JSON"]["input"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
  statementCk?: InputMaybe<Scalars["UUID"]["input"]>;
  statementId: Scalars["GlobalID"]["input"];
  timezone?: InputMaybe<Scalars["String"]["input"]>;
  type: TriggerType;
};

export type TriggerDeleteInput = {
  id: Scalars["GlobalID"]["input"];
};

export type TriggerFilter = {
  AND?: InputMaybe<TriggerFilter>;
  OR?: InputMaybe<TriggerFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type TriggerOperationInfo = OperationInfo | Trigger;

export type TriggerRestoreInput = {
  id: Scalars["GlobalID"]["input"];
};

export enum TriggerType {
  Api = "API",
  Edit = "EDIT",
  Invoke = "INVOKE",
  Message = "MESSAGE",
  Run = "RUN",
  Time = "TIME",
  User = "USER",
}

export type TriggerUpdateInput = {
  active: Scalars["Boolean"]["input"];
  cron?: InputMaybe<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  interval?: InputMaybe<Scalars["Int"]["input"]>;
  mapping?: InputMaybe<Scalars["JSON"]["input"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
  statementCk?: InputMaybe<Scalars["UUID"]["input"]>;
  timezone?: InputMaybe<Scalars["String"]["input"]>;
  type: TriggerType;
};

export enum TypeHint {
  Audio = "AUDIO",
  Checkbox = "CHECKBOX",
  Code = "CODE",
  Date = "DATE",
  Datetime = "DATETIME",
  Duration = "DURATION",
  Email = "EMAIL",
  Embedding = "EMBEDDING",
  Field = "FIELD",
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
  Run = "RUN",
  Secret = "SECRET",
  Slider = "SLIDER",
  Statement = "STATEMENT",
  Thumbs = "THUMBS",
  Time = "TIME",
  Toggle = "TOGGLE",
  Url = "URL",
  Uuid = "UUID",
  Video = "VIDEO",
}

export enum TypeTag {
  Any = "ANY",
  Blob = "BLOB",
  Boolean = "BOOLEAN",
  Enum = "ENUM",
  Function = "FUNCTION",
  Json = "JSON",
  Literal = "LITERAL",
  Node = "NODE",
  Number = "NUMBER",
  String = "STRING",
  Struct = "STRUCT",
  TypeReference = "TYPE_REFERENCE",
  Vector = "VECTOR",
}

export type UpdateProjectVersion = {
  description?: InputMaybe<Scalars["String"]["input"]>;
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  tag?: InputMaybe<Scalars["String"]["input"]>;
};

export type User = Node &
  Owner & {
    __typename?: "User";
    accessTokens: AccessTokenConnection;
    bot: Scalars["Boolean"]["output"];
    canViewDetail: Scalars["Boolean"]["output"];
    canWrite: Scalars["Boolean"]["output"];
    createdAt: Scalars["DateTime"]["output"];
    description?: Maybe<Scalars["String"]["output"]>;
    email: Scalars["String"]["output"];
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"]["output"];
    name: Scalars["String"]["output"];
    notifications: NotificationConnection;
    organizationMemberships: OrganizationMembershipConnection;
    organizations: OrganizationConnection;
    projects: ProjectConnection;
    slug: Scalars["String"]["output"];
    status: UserStatus;
    updatedAt: Scalars["DateTime"]["output"];
    username: Scalars["String"]["output"];
  };

export type UserAccessTokensArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  filters?: InputMaybe<AccessTokenFilter>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UserNotificationsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  filters?: InputMaybe<NotificationFilter>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UserOrganizationMembershipsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UserOrganizationsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UserProjectsArgs = {
  after?: InputMaybe<Scalars["String"]["input"]>;
  before?: InputMaybe<Scalars["String"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
  last?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UserCompleteSignupInput = {
  fullName: Scalars["String"]["input"];
  id: Scalars["GlobalID"]["input"];
  username: Scalars["String"]["input"];
};

/** A connection to a list of items. */
export type UserConnection = {
  __typename?: "UserConnection";
  /** Contains the nodes in this connection */
  edges: Array<UserEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  /** Total quantity of existing nodes. */
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** An edge in a connection. */
export type UserEdge = {
  __typename?: "UserEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: User;
};

export type UserFilter = {
  AND?: InputMaybe<UserFilter>;
  OR?: InputMaybe<UserFilter>;
  emailEquals?: InputMaybe<Scalars["String"]["input"]>;
  slugPrefix?: InputMaybe<Scalars["String"]["input"]>;
};

export type UserOperationInfo = OperationInfo | User;

export type UserOrganization = Organization | User;

export enum UserStatus {
  Active = "ACTIVE",
  Deactivated = "DEACTIVATED",
  InitiatedSignup = "INITIATED_SIGNUP",
  Suspended = "SUSPENDED",
  Waitlisted = "WAITLISTED",
}

export type UserUpdateInput = {
  description: Scalars["String"]["input"];
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
};

export type WakeRuntimeInput = {
  projectVersionId: Scalars["GlobalID"]["input"];
};

export type WakeRuntimePayload = {
  __typename?: "WakeRuntimePayload";
  success: Scalars["Boolean"]["output"];
};

export type WakeRuntimePayloadOperationInfo = OperationInfo | WakeRuntimePayload;

export type WakeWorkerSetInput = {
  projectId: Scalars["GlobalID"]["input"];
};

export type WakeWorkerSetPayload = {
  __typename?: "WakeWorkerSetPayload";
  success: Scalars["Boolean"]["output"];
  workerSet?: Maybe<WorkerSet>;
};

export type WakeWorkerSetPayloadOperationInfo = OperationInfo | WakeWorkerSetPayload;

export type WorkerChange = {
  __typename?: "WorkerChange";
  workerSets: Array<WorkerSet>;
};

export enum WorkerProfile {
  Large = "LARGE",
  Medium = "MEDIUM",
  Small = "SMALL",
  Tiny = "TINY",
  XlargeCpu = "XLARGE_CPU",
  XlargeMem = "XLARGE_MEM",
}

export enum WorkerRegion {
  EuCentral = "EU_CENTRAL",
  UsCentral = "US_CENTRAL",
}

export type WorkerSet = Node & {
  __typename?: "WorkerSet";
  availableReplicas: Scalars["Int"]["output"];
  createdAt: Scalars["DateTime"]["output"];
  desiredReplicas: Scalars["Int"]["output"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"]["output"];
  lastActiveAt?: Maybe<Scalars["DateTime"]["output"]>;
  lastBumpedAt?: Maybe<Scalars["DateTime"]["output"]>;
  profile: WorkerProfile;
  project: Project;
  readyReplicas: Scalars["Int"]["output"];
  region: WorkerRegion;
  sleeping: Scalars["Boolean"]["output"];
  status: WorkerSetStatus;
  targetReplicas: Scalars["Int"]["output"];
  updatedAt: Scalars["DateTime"]["output"];
};

export enum WorkerSetStatus {
  Healthy = "HEALTHY",
  Pending = "PENDING",
  Sleeping = "SLEEPING",
  Unavailable = "UNAVAILABLE",
  Unhealthy = "UNHEALTHY",
  Unknown = "UNKNOWN",
  Updating = "UPDATING",
}

export type MatchingUsersQueryVariables = Exact<{
  slug?: InputMaybe<Scalars["String"]["input"]>;
  email?: InputMaybe<Scalars["String"]["input"]>;
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

export type NotificationsQueryVariables = Exact<{
  status?: InputMaybe<NotificationStatus>;
  notArchived?: InputMaybe<Scalars["Boolean"]["input"]>;
  first?: InputMaybe<Scalars["Int"]["input"]>;
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
          organizationInvite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationRole;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
          projectInvite: {
            __typename?: "ProjectInvite";
            id: any;
            level: ModuleAccessLevel;
            project: { __typename?: "Project"; id: any; slug: string; name: string };
          };
        };
      }>;
    };
  } | null;
};

export type ExistingProjectVersionTagQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  tag: Scalars["String"]["input"];
}>;

export type ExistingProjectVersionTagQuery = {
  __typename?: "Query";
  projectVersionByTag?: { __typename?: "ProjectVersion"; id: any; tag?: string | null } | null;
};

export type BlankPanelSuggestedFilesQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
}>;

export type BlankPanelSuggestedFilesQuery = {
  __typename?: "Query";
  module?: {
    __typename?: "ProjectVersion";
    files: Array<{ __typename?: "File"; id: any; ck: any; name: string; deletedAt?: any | null }>;
  } | null;
};

export type FileContentByIdQueryVariables = Exact<{
  fileId: Scalars["GlobalID"]["input"];
}>;

export type FileContentByIdQuery = {
  __typename?: "Query";
  file?:
    | ({
        __typename?: "File";
        id: any;
        ck: any;
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
        issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type StatementContentByIdQueryVariables = Exact<{
  statementId: Scalars["GlobalID"]["input"];
}>;

export type StatementContentByIdQuery = {
  __typename?: "Query";
  statement?:
    | ({
        __typename?: "Statement";
        id: any;
        deletedAt?: any | null;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        parent:
          | { __typename?: "Field"; id: any }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue"; id: any }
          | { __typename?: "ProjectVersion"; id: any }
          | { __typename?: "ResolvedField"; id: any }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging"; id: any }
          | { __typename?: "Trigger"; id: any };
      } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } })
    | null;
};

export type ProfileAccessTokensQueryVariables = Exact<{
  slug: Scalars["String"]["input"];
  includeInactive: Scalars["Boolean"]["input"];
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
  ownerId: Scalars["GlobalID"]["input"];
  scopes: Array<AccessTokenScope> | AccessTokenScope;
  expiresAt?: InputMaybe<Scalars["DateTime"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
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
  slug: Scalars["String"]["input"];
}>;

export type OrganizationMembersQuery = {
  __typename?: "Query";
  ownerBySlug?:
    | {
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
              level: OrganizationRole;
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
              level: OrganizationRole;
              email: string;
              emailSentAt?: any | null;
              user?: {
                __typename?: "User";
                id: any;
                slug: string;
                email: string;
                name: string;
                username: string;
              } | null;
            };
          }>;
        };
      }
    | { __typename?: "User" }
    | null;
};

export type ProfileSettingsQueryVariables = Exact<{
  slug: Scalars["String"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  description: Scalars["String"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  description: Scalars["String"]["input"];
}>;

export type UpdateUserMutation = {
  __typename?: "Mutation";
  updateUser:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "User"; id: any; name: string; description?: string | null };
};

export type EnvironmentQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
}>;

export type EnvironmentQuery = {
  __typename?: "Query";
  environment:
    | {
        __typename?: "Environment";
        language: string;
        version: string;
        platform: string;
        packages: Array<{ __typename?: "Package"; name: string; version: string }>;
      }
    | { __typename?: "OperationInfo" };
  project?: {
    __typename?: "Project";
    id: any;
    usage: { __typename?: "ProjectUsage"; recordsActive: number; objectsBytesTotal: number; cacheBytesTotal: number };
  } | null;
};

export type ProjectVersionsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
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
  slug: Scalars["String"]["input"];
}>;

export type CheckOwnerBySlugQuery = {
  __typename?: "Query";
  ownerBySlug?: { __typename?: "Organization"; id: any } | { __typename?: "User"; id: any } | null;
};

export type ProjectBySlugQueryVariables = Exact<{
  owner: Scalars["String"]["input"];
  project: Scalars["String"]["input"];
}>;

export type ProjectBySlugQuery = {
  __typename?: "Query";
  projectBySlug?:
    | ({ __typename?: "Project" } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } })
    | null;
};

export type ProjectVersionHeaderQueryVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type ProjectVersionHeaderQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    ck: any;
    name?: string | null;
    tag?: string | null;
    description?: string | null;
    createdAt: any;
    committed: boolean;
    committedAt?: any | null;
    parents: Array<{ __typename?: "ProjectVersion"; id: any }>;
    children: Array<{ __typename?: "ProjectVersion"; id: any }>;
  } | null;
};

export type ExistingProjectBySlugQueryVariables = Exact<{
  owner: Scalars["String"]["input"];
  project: Scalars["String"]["input"];
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
  slug: Scalars["String"]["input"];
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
        canViewDetail: boolean;
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
        canViewDetail: boolean;
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
  slug: Scalars["String"]["input"];
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
        canViewDetail: boolean;
        canWrite: boolean;
        memberships: { __typename?: "OrganizationMembershipConnection"; totalCount?: number | null };
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
        canViewDetail: boolean;
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
    status: UserStatus;
    organizationMemberships: {
      __typename?: "OrganizationMembershipConnection";
      totalCount?: number | null;
      edges: Array<{
        __typename?: "OrganizationMembershipEdge";
        node: {
          __typename?: "OrganizationMembership";
          id: any;
          createdAt: any;
          level: OrganizationRole;
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
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  userId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  inSameOrganizations: Scalars["Boolean"]["input"];
  first?: InputMaybe<Scalars["Int"]["input"]>;
  active?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type ConnectedClientsQuery = {
  __typename?: "Query";
  clients: {
    __typename?: "ClientConnection";
    totalCount?: number | null;
    pageInfo: {
      __typename?: "PageInfo";
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor?: string | null;
      endCursor?: string | null;
    };
    edges: Array<{
      __typename?: "ClientEdge";
      node: { __typename?: "Client" } & { " $fragmentRefs"?: { ClientContentTypeFragment: ClientContentTypeFragment } };
    }>;
  };
};

export type ClientsChangedSubscriptionVariables = Exact<{
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
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

export type SearchRecordsQueryVariables = Exact<{
  statementId: Scalars["GlobalID"]["input"];
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type SearchRecordsQuery = {
  __typename?: "Query";
  searchRecords: {
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
        value: any;
      };
    }>;
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

type HasCrudContent_Field_Fragment = {
  __typename?: "Field";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_Field_Fragment" };

type HasCrudContent_File_Fragment = {
  __typename?: "File";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_File_Fragment" };

type HasCrudContent_ProjectVersion_Fragment = {
  __typename?: "ProjectVersion";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_ProjectVersion_Fragment" };

type HasCrudContent_Record_Fragment = {
  __typename?: "Record";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_Record_Fragment" };

type HasCrudContent_Statement_Fragment = {
  __typename?: "Statement";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_Statement_Fragment" };

type HasCrudContent_Tagging_Fragment = {
  __typename?: "Tagging";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_Tagging_Fragment" };

type HasCrudContent_Trigger_Fragment = {
  __typename?: "Trigger";
  id: any;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "HasCrudContent_Trigger_Fragment" };

export type HasCrudContentFragment =
  | HasCrudContent_Field_Fragment
  | HasCrudContent_File_Fragment
  | HasCrudContent_ProjectVersion_Fragment
  | HasCrudContent_Record_Fragment
  | HasCrudContent_Statement_Fragment
  | HasCrudContent_Tagging_Fragment
  | HasCrudContent_Trigger_Fragment;

export type ProjectVersionHeaderFragment = {
  __typename?: "ProjectVersion";
  id: any;
  ck: any;
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
  children: Array<{ __typename?: "ProjectVersion"; id: any }>;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type ProjectHeaderFragment = {
  __typename?: "Project";
  id: any;
  createdAt: any;
  updatedAt: any;
  name: string;
  slug: string;
  visibility: ProjectVisibility;
  accessLevel: ModuleAccessLevel;
  sharingEnabled: boolean;
  sharingToken?: any | null;
  sharingLevel: ModuleAccessLevel;
  head: { __typename?: "ProjectVersion" } & {
    " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
  };
  owner:
    | { __typename?: "Organization"; id: any; slug: string; name: string }
    | { __typename?: "User"; id: any; slug: string; username: string; name: string };
} & { " $fragmentName"?: "ProjectHeaderFragment" };

export type FileHeaderFragment = {
  __typename: "File";
  id: any;
  ck: any;
  revision: number;
  name: string;
  deletedAt?: any | null;
  createdAt: any;
  updatedAt: any;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any };
  projectVersion: { __typename?: "ProjectVersion"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "FileHeaderFragment" };

export type StatementHeaderFragment = {
  __typename: "Statement";
  id: any;
  ck: any;
  type: StatementType;
  revision: number;
  name?: string | null;
  headingLevel?: number | null;
  text?: string | null;
  orderKey: string;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any };
} & { " $fragmentName"?: "StatementHeaderFragment" };

export type FieldContentFragment = {
  __typename?: "Field";
  id: any;
  ck: any;
  revision: number;
  name?: string | null;
  key: string;
  tag: TypeTag;
  hint?: TypeHint | null;
  flags: number;
  text?: string | null;
  orderKey: string;
  referenceCk?: any | null;
  value?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent: { __typename?: "Statement"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "FieldContentFragment" };

export type TaggingContentFragment = {
  __typename?: "Tagging";
  id: any;
  ck: any;
  revision: number;
  key: string;
  referenceCk?: any | null;
  value?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent: { __typename?: "Statement"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "TaggingContentFragment" };

export type TriggerContentFragment = {
  __typename?: "Trigger";
  id: any;
  ck: any;
  revision: number;
  type: TriggerType;
  active: boolean;
  mapping?: any | null;
  timezone?: string | null;
  scheduleType: ScheduleType;
  interval?: number | null;
  cron?: string | null;
  statementCk?: any | null;
  scopeCk?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent: { __typename?: "Statement"; id: any };
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "TriggerContentFragment" };

export type StatementContentFragment = {
  __typename?: "Statement";
  id: any;
  ck: any;
  type: StatementType;
  revision: number;
  name?: string | null;
  orderKey: string;
  key?: string | null;
  text?: string | null;
  headingLevel?: number | null;
  code?: string | null;
  value?: any | null;
  referenceCk?: any | null;
  versioned: boolean;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any };
  tags: Array<{ __typename?: "Tagging" } & { " $fragmentRefs"?: { TaggingContentFragment: TaggingContentFragment } }>;
  fields: Array<{ __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } }>;
  triggers: Array<
    { __typename?: "Trigger" } & { " $fragmentRefs"?: { TriggerContentFragment: TriggerContentFragment } }
  >;
  issues?: Array<
    { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
  > | null;
  resolvedFields?: Array<
    { __typename?: "ResolvedField" } & {
      " $fragmentRefs"?: { ResolvedFieldContentFragment: ResolvedFieldContentFragment };
    }
  > | null;
  createdBy?: { __typename?: "User"; id: any } | null;
  lastEditedBy?: { __typename?: "User"; id: any } | null;
} & { " $fragmentName"?: "StatementContentFragment" };

export type IssueContentFragment = {
  __typename?: "Issue";
  id: any;
  ck: any;
  kind: IssueKind;
  type: IssueType;
  message?: string | null;
  parent?:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any }
    | null;
} & { " $fragmentName"?: "IssueContentFragment" };

export type ResolvedFieldContentFragment = {
  __typename: "ResolvedField";
  id: any;
  ck: any;
  orderKey: string;
  fieldCk: any;
  statement?: { __typename?: "Statement"; id: any } | null;
} & { " $fragmentName"?: "ResolvedFieldContentFragment" };

export type InterpFileFragment = {
  __typename?: "File";
  id: any;
  ck: any;
  revision: number;
  name: string;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any };
  issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
} & { " $fragmentName"?: "InterpFileFragment" };

export type InterpStatementFragment = {
  __typename?: "Statement";
  id: any;
  ck: any;
  type: StatementType;
  name?: string | null;
  text?: string | null;
  headingLevel?: number | null;
  revision: number;
  orderKey: string;
  key?: string | null;
  referenceCk?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  file: { __typename?: "File"; id: any };
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
    | { __typename?: "ResolvedField"; id: any }
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any };
  tags: Array<{ __typename?: "Tagging" } & { " $fragmentRefs"?: { TaggingContentFragment: TaggingContentFragment } }>;
  fields: Array<{ __typename?: "Field" } & { " $fragmentRefs"?: { FieldContentFragment: FieldContentFragment } }>;
  issues?: Array<
    { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
  > | null;
  resolvedFields?: Array<
    { __typename?: "ResolvedField" } & {
      " $fragmentRefs"?: { ResolvedFieldContentFragment: ResolvedFieldContentFragment };
    }
  > | null;
} & { " $fragmentName"?: "InterpStatementFragment" };

export type ModuleContentByIdQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
}>;

export type ModuleContentByIdQuery = {
  __typename?: "Query";
  module?: {
    __typename?: "ProjectVersion";
    id: any;
    committed: boolean;
    project: { __typename?: "Project"; path: string; name: string };
    files: Array<
      {
        __typename?: "File";
        statements: Array<
          { __typename?: "Statement" } & { " $fragmentRefs"?: { InterpStatementFragment: InterpStatementFragment } }
        >;
      } & { " $fragmentRefs"?: { InterpFileFragment: InterpFileFragment } }
    >;
  } | null;
};

export type NewNotificationsQueryVariables = Exact<{
  after?: InputMaybe<Scalars["String"]["input"]>;
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
          organizationInvite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationRole;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
          projectInvite: {
            __typename?: "ProjectInvite";
            id: any;
            level: ModuleAccessLevel;
            project: { __typename?: "Project"; id: any; slug: string; name: string };
          };
        };
      }>;
    };
  } | null;
};

export type MarkNotificationMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
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

export type BlobQueryVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type BlobQuery = {
  __typename?: "Query";
  blob?: { __typename?: "Blob"; id: any; presignedGet?: string | null } | null;
};

export type UpsertClientMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  type: ClientType;
  deviceName?: InputMaybe<Scalars["String"]["input"]>;
  browserName?: InputMaybe<Scalars["String"]["input"]>;
  projectId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  fileId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  statementId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  fieldId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  recordId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  path?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
}>;

export type CreateFileMutation = {
  __typename?: "Mutation";
  createFile:
    | {
        __typename?: "File";
        id: any;
        ck: any;
        revision: number;
        name: string;
        deletedAt?: any | null;
        createdAt: any;
        updatedAt: any;
        lastEditedAt?: any | null;
        projectVersion: { __typename?: "ProjectVersion"; id: any };
        parent:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue" }
          | { __typename?: "ProjectVersion"; id: any }
          | { __typename?: "ResolvedField" }
          | { __typename?: "Statement" }
          | { __typename?: "Tagging" }
          | { __typename?: "Trigger" };
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
        statements: Array<
          {
            __typename?: "Statement";
            issues?: Array<
              { __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }
            > | null;
          } & { " $fragmentRefs"?: { StatementContentFragment: StatementContentFragment } }
        >;
        issues: Array<{ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } }>;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteFileMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
}>;

export type RenameFileMutation = {
  __typename?: "Mutation";
  renameFile:
    | { __typename?: "File"; id: any; name: string; revision: number }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type UpdateFileMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
}>;

export type UpdateFileMutation = {
  __typename?: "Mutation";
  updateFile:
    | {
        __typename?: "File";
        id: any;
        name: string;
        revision: number;
        parent:
          | { __typename?: "Field"; id: any }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue"; id: any }
          | { __typename?: "ProjectVersion"; id: any }
          | { __typename?: "ResolvedField"; id: any }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging"; id: any }
          | { __typename?: "Trigger"; id: any };
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type PasteFileMutationVariables = Exact<{
  sourceId: Scalars["GlobalID"]["input"];
  targetId: Scalars["GlobalID"]["input"];
  targetCk: Scalars["UUID"]["input"];
  targetVersionId: Scalars["GlobalID"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
}>;

export type PasteFileMutation = {
  __typename?: "Mutation";
  pasteFile:
    | ({
        __typename?: "File";
        id: any;
        ck: any;
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
  projectId: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  contentType: Scalars["String"]["input"];
  contentLength: Scalars["Int"]["input"];
  sha512: Scalars["String"]["input"];
}>;

export type RequestUploadObjectMutation = {
  __typename?: "Mutation";
  requestUploadObject:
    | {
        __typename?: "Blob";
        id: any;
        status: BlobStatus;
        name?: string | null;
        contentType: string;
        contentLength: number;
        sha512: string;
        presignedPost?: string | null;
        presignedGet?: string | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type NotifyUploadedObjectMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type NotifyUploadedObjectMutation = {
  __typename?: "Mutation";
  notifyUploadedObject:
    | {
        __typename?: "Blob";
        id: any;
        status: BlobStatus;
        name?: string | null;
        contentType: string;
        contentLength: number;
        sha512: string;
        presignedGet?: string | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CreateOrganizationMutationVariables = Exact<{
  name: Scalars["String"]["input"];
  slug: Scalars["String"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  emails: Array<Scalars["String"]["input"]> | Scalars["String"]["input"];
  level: OrganizationRole;
  message?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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

export type UpdateProjectSharingMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  sharingEnabled: Scalars["Boolean"]["input"];
  sharingToken: Scalars["UUID"]["input"];
  sharingLevel: ModuleAccessLevel;
}>;

export type UpdateProjectSharingMutation = {
  __typename?: "Mutation";
  updateProjectSharing:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Project";
        id: any;
        sharingEnabled: boolean;
        sharingToken?: any | null;
        sharingLevel: ModuleAccessLevel;
      };
};

export type UpdateProjectNameMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
}>;

export type UpdateProjectNameMutation = {
  __typename?: "Mutation";
  updateProjectName:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Project"; id: any; name: string };
};

export type CreateSecretMutationVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  value: Scalars["JSON"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  value: Scalars["JSON"]["input"];
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
  id: Scalars["GlobalID"]["input"];
}>;

export type DeleteSecretMutation = {
  __typename?: "Mutation";
  deleteSecret?:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | null;
};

export type WakeRuntimeMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
}>;

export type WakeRuntimeMutation = {
  __typename?: "Mutation";
  wakeRuntime:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "WakeRuntimePayload" };
};

export type WakeWorkerSetMutationVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
}>;

export type WakeWorkerSetMutation = {
  __typename?: "Mutation";
  wakeWorkerSet:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "WakeWorkerSetPayload"; success: boolean };
};

export type RestartWorkerSetMutationVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
}>;

export type RestartWorkerSetMutation = {
  __typename?: "Mutation";
  restartWorkerSet:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "RestartWorkerSetPayload"; success: boolean };
};

export type StartRunMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
  statementId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
  code?: InputMaybe<Scalars["String"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  inputs?: InputMaybe<Scalars["JSON"]["input"]>;
  keyed?: InputMaybe<Scalars["Boolean"]["input"]>;
  block?: InputMaybe<Scalars["Float"]["input"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  rootValue?: InputMaybe<Scalars["JSON"]["input"]>;
  globalValue?: InputMaybe<Scalars["JSON"]["input"]>;
  accessLevel: Scalars["Int"]["input"];
  tags?: InputMaybe<Array<Scalars["String"]["input"]> | Scalars["String"]["input"]>;
}>;

export type StartRunMutation = {
  __typename?: "Mutation";
  run:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "RunState";
        projectVersionId: any;
        statementId?: any | null;
        success: boolean;
        error?: StartRunErrorType | null;
        run?: ({ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }) | null;
        logs?: Array<
          { __typename?: "LogEntry" } & { " $fragmentRefs"?: { LogEntryContentFragment: LogEntryContentFragment } }
        > | null;
      };
};

export type KillMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
  runId: Scalars["GlobalID"]["input"];
  restartIfUnresponsive: Scalars["Boolean"]["input"];
}>;

export type KillMutation = {
  __typename?: "Mutation";
  killRun:
    | {
        __typename?: "KillRunPayload";
        run?: {
          __typename?: "Run";
          id: any;
          status: RunStatus;
          startedAt?: any | null;
          terminatedAt?: any | null;
          createdAt: any;
          updatedAt: any;
          duration?: number | null;
        } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type CreateStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  fileId: Scalars["GlobalID"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  orderKey: Scalars["String"]["input"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]["input"]>;
  key?: InputMaybe<Scalars["String"]["input"]>;
  code?: InputMaybe<Scalars["String"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
  versioned: Scalars["Boolean"]["input"];
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
        ck: any;
        type: StatementType;
        revision: number;
        name?: string | null;
        orderKey: string;
        key?: string | null;
        code?: string | null;
        text?: string | null;
        headingLevel?: number | null;
        value?: any | null;
        referenceCk?: any | null;
        versioned: boolean;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        file: { __typename?: "File"; id: any };
        parent:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue" }
          | { __typename?: "ProjectVersion" }
          | { __typename?: "ResolvedField" }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging" }
          | { __typename?: "Trigger" };
        tags: Array<{ __typename?: "Tagging"; id: any }>;
        fields: Array<{ __typename?: "Field"; id: any }>;
        triggers: Array<{ __typename?: "Trigger"; id: any }>;
        resolvedFields?: Array<{
          __typename?: "ResolvedField";
          fieldCk: any;
          statement?: { __typename?: "Statement"; id: any } | null;
        }> | null;
        issues?: Array<{ __typename?: "Issue"; id: any }> | null;
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      };
};

export type UpdateStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  orderKey: Scalars["String"]["input"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]["input"]>;
  code?: InputMaybe<Scalars["String"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
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
        code?: string | null;
        text?: string | null;
        value?: any | null;
      };
};

export type MorphStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]["input"]>;
  key?: InputMaybe<Scalars["String"]["input"]>;
  headingLevel?: InputMaybe<Scalars["Int"]["input"]>;
  versioned: Scalars["Boolean"]["input"];
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
        key?: string | null;
        headingLevel?: number | null;
        versioned: boolean;
      };
};

export type MoveStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  fileId: Scalars["GlobalID"]["input"];
  parentId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  orderKey: Scalars["String"]["input"];
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
        parent:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue" }
          | { __typename?: "ProjectVersion" }
          | { __typename?: "ResolvedField" }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging" }
          | { __typename?: "Trigger" };
      };
};

export type BatchMoveStatementMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
  fileId: Scalars["GlobalID"]["input"];
  parentIds: Array<InputMaybe<Scalars["GlobalID"]["input"]>> | InputMaybe<Scalars["GlobalID"]["input"]>;
  orderKeys: Array<Scalars["String"]["input"]> | Scalars["String"]["input"];
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
          parent:
            | { __typename?: "Field" }
            | { __typename?: "File"; id: any }
            | { __typename?: "Issue" }
            | { __typename?: "ProjectVersion" }
            | { __typename?: "ResolvedField" }
            | { __typename?: "Statement"; id: any }
            | { __typename?: "Tagging" }
            | { __typename?: "Trigger" };
        }>;
      };
};

export type RenameStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  ids: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  ids: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
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
  sourceIds: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
  targetIds: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
  targetCks: Array<Scalars["UUID"]["input"]> | Scalars["UUID"]["input"];
  targetFileId: Scalars["GlobalID"]["input"];
  targetParentIds: Array<InputMaybe<Scalars["GlobalID"]["input"]>> | InputMaybe<Scalars["GlobalID"]["input"]>;
  targetOrderKeys: Array<Scalars["String"]["input"]> | Scalars["String"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
}>;

export type UpdateStatementReferenceMutation = {
  __typename?: "Mutation";
  updateStatementReference:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Statement"; id: any; revision: number; referenceCk?: any | null };
};

export type UpdateSymbolCodeMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  code?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
  text?: InputMaybe<Scalars["String"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
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
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  statementCk: Scalars["UUID"]["input"];
  statementKey: Scalars["String"]["input"];
  value: Scalars["JSON"]["input"];
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
        ck: any;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        revision: number;
        value: any;
      };
};

export type UpdateRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  value: Scalars["JSON"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
}>;

export type SoftDeleteRecordMutation = {
  __typename?: "Mutation";
  softDeleteRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; deletedAt?: any | null; revision: number };
};

export type RestoreRecordMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
}>;

export type RestoreRecordMutation = {
  __typename?: "Mutation";
  restoreRecord:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Record"; id: any; deletedAt?: any | null; revision: number };
};

export type BatchSoftDeleteRecordMutationVariables = Exact<{
  ids: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
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
  ids: Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  tag: TypeTag;
  hint?: InputMaybe<TypeHint>;
  key: Scalars["String"]["input"];
  orderKey: Scalars["String"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  flags: Scalars["Int"]["input"];
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
}>;

export type CreateFieldMutation = {
  __typename?: "Mutation";
  createField:
    | {
        __typename?: "Field";
        id: any;
        ck: any;
        key: string;
        orderKey: string;
        revision: number;
        name?: string | null;
        tag: TypeTag;
        hint?: TypeHint | null;
        text?: string | null;
        referenceCk?: any | null;
        flags: number;
        value?: any | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        statement: { __typename?: "Statement"; id: any };
        parent: { __typename?: "Statement"; id: any };
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type DeleteFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
}>;

export type RestoreFieldMutation = {
  __typename?: "Mutation";
  restoreField:
    | { __typename?: "Field"; id: any; deletedAt?: any | null }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type UpdateFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  tag: TypeTag;
  hint?: InputMaybe<TypeHint>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  text?: InputMaybe<Scalars["String"]["input"]>;
  flags: Scalars["Int"]["input"];
  referenceCk?: InputMaybe<Scalars["UUID"]["input"]>;
  value?: InputMaybe<Scalars["JSON"]["input"]>;
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
        text?: string | null;
        flags: number;
        referenceCk?: any | null;
        value?: any | null;
      }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type MoveFieldMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  orderKey: Scalars["String"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  key: Scalars["String"]["input"];
  referenceCk: Scalars["UUID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
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
        ck: any;
        revision: number;
        key: string;
        referenceCk?: any | null;
        value?: any | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        parent: { __typename?: "Statement"; id: any };
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      };
};

export type DeleteTaggingMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
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
  id: Scalars["GlobalID"]["input"];
  value?: InputMaybe<Scalars["JSON"]["input"]>;
}>;

export type UpdateTaggingMutation = {
  __typename?: "Mutation";
  updateTagging:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Tagging"; id: any; updatedAt: any; revision: number; value?: any | null };
};

export type CreateTriggerMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  ck: Scalars["UUID"]["input"];
  statementId: Scalars["GlobalID"]["input"];
  type: TriggerType;
  active: Scalars["Boolean"]["input"];
  mapping?: InputMaybe<Scalars["JSON"]["input"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  timezone?: InputMaybe<Scalars["String"]["input"]>;
  interval?: InputMaybe<Scalars["Int"]["input"]>;
  cron?: InputMaybe<Scalars["String"]["input"]>;
  statementCk?: InputMaybe<Scalars["UUID"]["input"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
}>;

export type CreateTriggerMutation = {
  __typename?: "Mutation";
  createTrigger:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Trigger";
        id: any;
        ck: any;
        revision: number;
        type: TriggerType;
        active: boolean;
        mapping?: any | null;
        scheduleType: ScheduleType;
        timezone?: string | null;
        interval?: number | null;
        cron?: string | null;
        statementCk?: any | null;
        scopeCk?: any | null;
        createdAt: any;
        updatedAt: any;
        deletedAt?: any | null;
        lastEditedAt?: any | null;
        parent: { __typename?: "Statement"; id: any };
        createdBy?: { __typename?: "User"; id: any } | null;
        lastEditedBy?: { __typename?: "User"; id: any } | null;
      };
};

export type SoftDeleteTriggerMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type SoftDeleteTriggerMutation = {
  __typename?: "Mutation";
  softDeleteTrigger:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Trigger"; id: any; deletedAt?: any | null };
};

export type RestoreTriggerMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type RestoreTriggerMutation = {
  __typename?: "Mutation";
  restoreTrigger:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Trigger"; id: any; deletedAt?: any | null };
};

export type UpdateTriggerMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  type: TriggerType;
  active: Scalars["Boolean"]["input"];
  mapping?: InputMaybe<Scalars["JSON"]["input"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  timezone?: InputMaybe<Scalars["String"]["input"]>;
  interval?: InputMaybe<Scalars["Int"]["input"]>;
  cron?: InputMaybe<Scalars["String"]["input"]>;
  statementCk?: InputMaybe<Scalars["UUID"]["input"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]["input"]>;
}>;

export type UpdateTriggerMutation = {
  __typename?: "Mutation";
  updateTrigger:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "Trigger";
        id: any;
        updatedAt: any;
        type: TriggerType;
        revision: number;
        active: boolean;
        mapping?: any | null;
        scheduleType: ScheduleType;
        timezone?: string | null;
        interval?: number | null;
        cron?: string | null;
        statementCk?: any | null;
        scopeCk?: any | null;
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
        status: UserStatus;
      };
};

export type AcceptOrganizationInviteMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
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
        status: UserStatus;
        organizationMemberships: {
          __typename?: "OrganizationMembershipConnection";
          totalCount?: number | null;
          edges: Array<{
            __typename?: "OrganizationMembershipEdge";
            node: {
              __typename?: "OrganizationMembership";
              id: any;
              level: OrganizationRole;
              organization: { __typename?: "Organization"; id: any; name: string; slug: string };
            };
          }>;
        };
      };
};

export type UpdateVersionMutationVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
  name: Scalars["String"]["input"];
  tag?: InputMaybe<Scalars["String"]["input"]>;
  description?: InputMaybe<Scalars["String"]["input"]>;
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

export type SnapshotMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  tag?: InputMaybe<Scalars["String"]["input"]>;
  description?: InputMaybe<Scalars["String"]["input"]>;
}>;

export type SnapshotMutation = {
  __typename?: "Mutation";
  snapshot:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "SnapshotPayload";
        project: {
          __typename?: "Project";
          head: { __typename?: "ProjectVersion" } & {
            " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
          };
        } & { " $fragmentRefs"?: { ProjectHeaderFragment: ProjectHeaderFragment } };
        snapshot: { __typename?: "ProjectVersion" } & {
          " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
        };
      };
};

export type RevealSecretQueryVariables = Exact<{
  secretId: Scalars["GlobalID"]["input"];
}>;

export type RevealSecretQuery = {
  __typename?: "Query";
  secret?: { __typename?: "Secret"; id: any; sha512: string; valueRevealed: any } | null;
};

export type WorkerSetContentFragment = {
  __typename?: "WorkerSet";
  id: any;
  region: WorkerRegion;
  profile: WorkerProfile;
  sleeping: boolean;
  status: WorkerSetStatus;
  desiredReplicas: number;
  targetReplicas: number;
  availableReplicas: number;
  readyReplicas: number;
  lastActiveAt?: any | null;
  project: { __typename?: "Project"; id: any };
} & { " $fragmentName"?: "WorkerSetContentFragment" };

export type RunHeaderFragment = {
  __typename?: "Run";
  id: any;
  createdAt: any;
  updatedAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  duration?: number | null;
  status: RunStatus;
  statementCk?: any | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any; tag?: string | null; name?: string | null };
  session?: { __typename?: "Session"; id: any } | null;
  root?: { __typename?: "Run"; id: any } | null;
  parent?: { __typename?: "Run"; id: any } | null;
  statement?: { __typename?: "Statement"; id: any; name?: string | null } | null;
} & { " $fragmentName"?: "RunHeaderFragment" };

export type RunContentFragment = {
  __typename?: "Run";
  id: any;
  createdAt: any;
  updatedAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  duration?: number | null;
  status: RunStatus;
  inputs?: any | null;
  outputs?: any | null;
  value?: any | null;
  statementCk?: any | null;
  triggerType?: TriggerType | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any; tag?: string | null; name?: string | null };
  session?: { __typename?: "Session"; id: any } | null;
  root?: { __typename?: "Run"; id: any } | null;
  parent?: { __typename?: "Run"; id: any } | null;
  errorNice?: {
    __typename?: "RunError";
    kind: string;
    type: string;
    message: string;
    traceback?: Array<{
      __typename?: "RunCodeFrame";
      line: string;
      filename: string;
      lineno: number;
      name: string;
      locals?: any | null;
    }> | null;
  } | null;
  statement?: { __typename?: "Statement"; id: any } | null;
  trigger?: { __typename?: "Trigger"; id: any; type: TriggerType } | null;
  triggerUser?: { __typename?: "User"; id: any; username: string; name: string } | null;
  triggerAccessToken?: { __typename?: "AccessToken"; id: any; name?: string | null } | null;
} & { " $fragmentName"?: "RunContentFragment" };

export type LogEntryContentFragment = {
  __typename?: "LogEntry";
  id: any;
  createdAt: any;
  projectVersionId: any;
  sessionId?: any | null;
  statementId?: any | null;
  statementCk?: any | null;
  runId?: any | null;
  stream: string;
  level?: string | null;
  logger?: string | null;
  message?: string | null;
  value?: any | null;
} & { " $fragmentName"?: "LogEntryContentFragment" };

export type CurrentRunsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
}>;

export type CurrentRunsQuery = {
  __typename?: "Query";
  currentRuns:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | {
        __typename?: "SessionState";
        runs: Array<{ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }>;
        workerSet?:
          | ({ __typename?: "WorkerSet" } & {
              " $fragmentRefs"?: { WorkerSetContentFragment: WorkerSetContentFragment };
            })
          | null;
      };
};

export type SessionsChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
}>;

export type SessionsChangedSubscription = {
  __typename?: "Subscription";
  sessionsChanged:
    | {
        __typename?: "RunsChange";
        runs: Array<{ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }>;
      }
    | {
        __typename?: "SessionChange";
        runs: Array<{ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }>;
      }
    | {
        __typename?: "WorkerChange";
        workerSets: Array<
          { __typename?: "WorkerSet" } & { " $fragmentRefs"?: { WorkerSetContentFragment: WorkerSetContentFragment } }
        >;
      };
};

export type RefetchProjectWorkerSetsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
}>;

export type RefetchProjectWorkerSetsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    id: any;
    workerSets: Array<
      { __typename?: "WorkerSet" } & { " $fragmentRefs"?: { WorkerSetContentFragment: WorkerSetContentFragment } }
    >;
  } | null;
};

export type SearchRunsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"]>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]> | Scalars["UUID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  rootOnly: Scalars["Boolean"]["input"];
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type SearchRunsQuery = {
  __typename?: "Query";
  searchRuns: {
    __typename?: "RunConnection";
    totalCount?: number | null;
    pageInfo: {
      __typename?: "PageInfo";
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor?: string | null;
      endCursor?: string | null;
    };
    edges: Array<{
      __typename?: "RunEdge";
      cursor: string;
      node: { __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } };
    }>;
  };
};

export type RunByIdQueryVariables = Exact<{
  id: Scalars["GlobalID"]["input"];
}>;

export type RunByIdQuery = {
  __typename?: "Query";
  run?:
    | ({
        __typename?: "Run";
        descendants: Array<{ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }>;
      } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } })
    | null;
};

export type SearchLogsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"]>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]> | Scalars["UUID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]["input"]>;
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  count?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type SearchLogsQuery = {
  __typename?: "Query";
  searchLogs: {
    __typename?: "LogEntryConnection";
    totalCount?: number | null;
    pageInfo: {
      __typename?: "PageInfo";
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor?: string | null;
      endCursor?: string | null;
    };
    edges: Array<{
      __typename?: "LogEntryEdge";
      cursor: string;
      node: { __typename?: "LogEntry" } & { " $fragmentRefs"?: { LogEntryContentFragment: LogEntryContentFragment } };
    }>;
  };
};

export type LogsChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
  statementIds?: InputMaybe<Array<Scalars["GlobalID"]["input"]> | Scalars["GlobalID"]["input"]>;
  statementCks?: InputMaybe<Array<Scalars["UUID"]["input"]> | Scalars["UUID"]["input"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]["input"]>;
  runId?: InputMaybe<Scalars["GlobalID"]["input"]>;
}>;

export type LogsChangedSubscription = {
  __typename?: "Subscription";
  logsChanged: {
    __typename?: "LogChange";
    logs: Array<
      { __typename?: "LogEntry" } & { " $fragmentRefs"?: { LogEntryContentFragment: LogEntryContentFragment } }
    >;
  };
};

export type ModuleChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
  projectVersionId: Scalars["GlobalID"]["input"];
}>;

export type ModuleChangedSubscription = {
  __typename?: "Subscription";
  moduleChanged: {
    __typename?: "ModuleChange";
    id: any;
    clientId?: any | null;
    edits: Array<{
      __typename?: "Edit";
      type: EditType;
      fileId?: any | null;
      statementId?: any | null;
      revision?: number | null;
      input?: any | null;
      data?:
        | ({ __typename?: "Issue" } & { " $fragmentRefs"?: { IssueContentFragment: IssueContentFragment } })
        | { __typename?: "ResolvedField"; id: any; ck: any; fieldCk: any }
        | null;
    }>;
  };
};

export type ProjectChangedSubscriptionVariables = Exact<{
  projectId: Scalars["GlobalID"]["input"];
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
export const HasCrudContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "HasCrudContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "HasCrud" } },
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
} as unknown as DocumentNode<HasCrudContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "head" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "accessLevel" } },
          { kind: "Field", name: { kind: "Name", value: "sharingEnabled" } },
          { kind: "Field", name: { kind: "Name", value: "sharingToken" } },
          { kind: "Field", name: { kind: "Name", value: "sharingLevel" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
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
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
export const TriggerContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
} as unknown as DocumentNode<TriggerContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
} as unknown as DocumentNode<IssueContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ResolvedFieldContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<StatementContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
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
            name: { kind: "Name", value: "issues" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
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
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
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
            name: { kind: "Name", value: "issues" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
          { kind: "Field", name: { kind: "Name", value: "lastEditedAt" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InterpStatementFragment, unknown>;
export const WorkerSetContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "WorkerSetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WorkerSet" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "region" } },
          { kind: "Field", name: { kind: "Name", value: "profile" } },
          { kind: "Field", name: { kind: "Name", value: "sleeping" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "desiredReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "targetReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "availableReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "readyReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "lastActiveAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<WorkerSetContentFragment, unknown>;
export const RunHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<RunHeaderFragment, unknown>;
export const RunContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
} as unknown as DocumentNode<RunContentFragment, unknown>;
export const LogEntryContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "LogEntryContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "LogEntry" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
          { kind: "Field", name: { kind: "Name", value: "sessionId" } },
          { kind: "Field", name: { kind: "Name", value: "statementId" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "runId" } },
          { kind: "Field", name: { kind: "Name", value: "stream" } },
          { kind: "Field", name: { kind: "Name", value: "level" } },
          { kind: "Field", name: { kind: "Name", value: "logger" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LogEntryContentFragment, unknown>;
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
                                    name: { kind: "Name", value: "organizationInvite" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "projectInvite" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "project" },
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
export const BlankPanelSuggestedFilesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "blankPanelSuggestedFiles" },
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
            name: { kind: "Name", value: "module" },
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
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
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
} as unknown as DocumentNode<BlankPanelSuggestedFilesQuery, BlankPanelSuggestedFilesQueryVariables>;
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
                { kind: "Field", name: { kind: "Name", value: "ck" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FileHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
                  name: { kind: "Name", value: "parent" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
} as unknown as DocumentNode<UpdateUserMutation, UpdateUserMutationVariables>;
export const EnvironmentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "environment" },
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
            name: { kind: "Name", value: "environment" },
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
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Environment" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "language" } },
                      { kind: "Field", name: { kind: "Name", value: "version" } },
                      { kind: "Field", name: { kind: "Name", value: "platform" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "packages" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                            { kind: "Field", name: { kind: "Name", value: "version" } },
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
                  name: { kind: "Name", value: "usage" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "recordsActive" } },
                      { kind: "Field", name: { kind: "Name", value: "objectsBytesTotal" } },
                      { kind: "Field", name: { kind: "Name", value: "cacheBytesTotal" } },
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
} as unknown as DocumentNode<EnvironmentQuery, EnvironmentQueryVariables>;
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "head" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "accessLevel" } },
          { kind: "Field", name: { kind: "Name", value: "sharingEnabled" } },
          { kind: "Field", name: { kind: "Name", value: "sharingToken" } },
          { kind: "Field", name: { kind: "Name", value: "sharingLevel" } },
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
} as unknown as DocumentNode<ProjectBySlugQuery, ProjectBySlugQueryVariables>;
export const ProjectVersionHeaderDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "projectVersionHeader" },
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
                { kind: "Field", name: { kind: "Name", value: "ck" } },
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "children" },
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
} as unknown as DocumentNode<ProjectVersionHeaderQuery, ProjectVersionHeaderQueryVariables>;
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
                      { kind: "Field", name: { kind: "Name", value: "canViewDetail" } },
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
                      { kind: "Field", name: { kind: "Name", value: "canViewDetail" } },
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
                      { kind: "Field", name: { kind: "Name", value: "canViewDetail" } },
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
                      { kind: "Field", name: { kind: "Name", value: "canViewDetail" } },
                      { kind: "Field", name: { kind: "Name", value: "canWrite" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "memberships" },
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
                { kind: "Field", name: { kind: "Name", value: "status" } },
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
} as unknown as DocumentNode<ClientsChangedSubscription, ClientsChangedSubscriptionVariables>;
export const SearchRecordsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "searchRecords" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "query" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "SearchQuery" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sort" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "SearchSort" } } },
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
            name: { kind: "Name", value: "searchRecords" },
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
} as unknown as DocumentNode<SearchRecordsQuery, SearchRecordsQueryVariables>;
export const ModuleContentByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "moduleContentById" },
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
            name: { kind: "Name", value: "module" },
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
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "InterpStatement" } }],
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
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpFile" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
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
            name: { kind: "Name", value: "issues" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InterpStatement" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
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
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
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
            name: { kind: "Name", value: "issues" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "IssueContent" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
} as unknown as DocumentNode<ModuleContentByIdQuery, ModuleContentByIdQueryVariables>;
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
                                    name: { kind: "Name", value: "organizationInvite" },
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
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "projectInvite" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        { kind: "Field", name: { kind: "Name", value: "id" } },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "project" },
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
} as unknown as DocumentNode<MarkNotificationMutation, MarkNotificationMutationVariables>;
export const BlobDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "blob" },
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
            name: { kind: "Name", value: "blob" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Blob" } },
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
} as unknown as DocumentNode<BlobQuery, BlobQueryVariables>;
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
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
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
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "projectVersion" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
} as unknown as DocumentNode<RenameFileMutation, RenameFileMutationVariables>;
export const UpdateFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateFile" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "parentId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateFile" },
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
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
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
} as unknown as DocumentNode<UpdateFileMutation, UpdateFileMutationVariables>;
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
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "targetCk" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
                      name: { kind: "Name", value: "targetCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetCk" } },
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
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FileHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "File" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Blob" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Blob" } },
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
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "OrganizationRole" } } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "head" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "accessLevel" } },
          { kind: "Field", name: { kind: "Name", value: "sharingEnabled" } },
          { kind: "Field", name: { kind: "Name", value: "sharingToken" } },
          { kind: "Field", name: { kind: "Name", value: "sharingLevel" } },
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
} as unknown as DocumentNode<UpdateProjectVisibilityMutation, UpdateProjectVisibilityMutationVariables>;
export const UpdateProjectSharingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateProjectSharing" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sharingEnabled" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sharingToken" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sharingLevel" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "ModuleAccessLevel" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateProjectSharing" },
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
                      name: { kind: "Name", value: "sharingEnabled" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sharingEnabled" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sharingToken" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sharingToken" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sharingLevel" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sharingLevel" } },
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
                      { kind: "Field", name: { kind: "Name", value: "sharingEnabled" } },
                      { kind: "Field", name: { kind: "Name", value: "sharingToken" } },
                      { kind: "Field", name: { kind: "Name", value: "sharingLevel" } },
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
} as unknown as DocumentNode<UpdateProjectSharingMutation, UpdateProjectSharingMutationVariables>;
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
} as unknown as DocumentNode<UpdateProjectNameMutation, UpdateProjectNameMutationVariables>;
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
} as unknown as DocumentNode<DeleteSecretMutation, DeleteSecretMutationVariables>;
export const WakeRuntimeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "wakeRuntime" },
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
            name: { kind: "Name", value: "wakeRuntime" },
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
} as unknown as DocumentNode<WakeRuntimeMutation, WakeRuntimeMutationVariables>;
export const WakeWorkerSetDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "wakeWorkerSet" },
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
            name: { kind: "Name", value: "wakeWorkerSet" },
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
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WakeWorkerSetPayload" } },
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
} as unknown as DocumentNode<WakeWorkerSetMutation, WakeWorkerSetMutationVariables>;
export const RestartWorkerSetDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restartWorkerSet" },
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
            name: { kind: "Name", value: "restartWorkerSet" },
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
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RestartWorkerSetPayload" } },
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
} as unknown as DocumentNode<RestartWorkerSetMutation, RestartWorkerSetMutationVariables>;
export const StartRunDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "startRun" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "code" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "inputs" } },
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
          type: { kind: "NamedType", name: { kind: "Name", value: "Float" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "timeoutSeconds" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootValue" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "globalValue" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "accessLevel" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "tags" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
          },
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
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scopeCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "code" },
                      value: { kind: "Variable", name: { kind: "Name", value: "code" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "runId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "runId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "sessionId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "inputs" },
                      value: { kind: "Variable", name: { kind: "Name", value: "inputs" } },
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
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "rootValue" },
                      value: { kind: "Variable", name: { kind: "Name", value: "rootValue" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "globalValue" },
                      value: { kind: "Variable", name: { kind: "Name", value: "globalValue" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "accessLevel" },
                      value: { kind: "Variable", name: { kind: "Name", value: "accessLevel" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "tags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "statementId" } },
                      { kind: "Field", name: { kind: "Name", value: "success" } },
                      { kind: "Field", name: { kind: "Name", value: "error" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "run" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "logs" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "LogEntryContent" } }],
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "LogEntryContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "LogEntry" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
          { kind: "Field", name: { kind: "Name", value: "sessionId" } },
          { kind: "Field", name: { kind: "Name", value: "statementId" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "runId" } },
          { kind: "Field", name: { kind: "Name", value: "stream" } },
          { kind: "Field", name: { kind: "Name", value: "level" } },
          { kind: "Field", name: { kind: "Name", value: "logger" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
        ],
      },
    },
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
} as unknown as DocumentNode<StartRunMutation, StartRunMutationVariables>;
export const KillDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "kill" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "restartIfUnresponsive" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "killRun" },
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
                      name: { kind: "Name", value: "runId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "runId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "restartIfUnresponsive" },
                      value: { kind: "Variable", name: { kind: "Name", value: "restartIfUnresponsive" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "KillRunPayload" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "run" },
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
} as unknown as DocumentNode<KillMutation, KillMutationVariables>;
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
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "versioned" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
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
                      name: { kind: "Name", value: "value" },
                      value: { kind: "Variable", name: { kind: "Name", value: "value" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "versioned" },
                      value: { kind: "Variable", name: { kind: "Name", value: "versioned" } },
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
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
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
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
                      { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
                        name: { kind: "Name", value: "triggers" },
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
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "statement" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                              },
                            },
                            { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "issues" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
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
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "orderKey" } },
                      { kind: "Field", name: { kind: "Name", value: "code" } },
                      { kind: "Field", name: { kind: "Name", value: "text" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "key" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "headingLevel" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "versioned" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
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
                      name: { kind: "Name", value: "key" },
                      value: { kind: "Variable", name: { kind: "Name", value: "key" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "headingLevel" },
                      value: { kind: "Variable", name: { kind: "Name", value: "headingLevel" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "versioned" },
                      value: { kind: "Variable", name: { kind: "Name", value: "versioned" } },
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
                      { kind: "Field", name: { kind: "Name", value: "key" } },
                      { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
                      { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "targetCks" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
                      name: { kind: "Name", value: "targetCks" },
                      value: { kind: "Variable", name: { kind: "Name", value: "targetCks" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaggingContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "FieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "hint" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "value" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TriggerContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "active" } },
          { kind: "Field", name: { kind: "Name", value: "mapping" } },
          { kind: "Field", name: { kind: "Name", value: "timezone" } },
          { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
          { kind: "Field", name: { kind: "Name", value: "interval" } },
          { kind: "Field", name: { kind: "Name", value: "cron" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ResolvedFieldContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "__typename" } },
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "StatementContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Statement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "revision" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "orderKey" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parent" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "key" } },
          { kind: "Field", name: { kind: "Name", value: "text" } },
          { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
          { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
          { kind: "Field", name: { kind: "Name", value: "versioned" } },
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
            name: { kind: "Name", value: "triggers" },
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
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TriggerContent" } }],
            },
          },
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
            name: { kind: "Name", value: "resolvedFields" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ResolvedFieldContent" } }],
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
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
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
                      name: { kind: "Name", value: "referenceCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
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
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
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
} as unknown as DocumentNode<UpdateStatementReferenceMutation, UpdateStatementReferenceMutationVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementKey" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "String" } } },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementKey" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementKey" } },
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
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "deletedAt" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "text" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
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
                      name: { kind: "Name", value: "text" },
                      value: { kind: "Variable", name: { kind: "Name", value: "text" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Field" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
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
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
            name: { kind: "Name", value: "restoreField" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "text" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Int" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
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
                      name: { kind: "Name", value: "text" },
                      value: { kind: "Variable", name: { kind: "Name", value: "text" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "referenceCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
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
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
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
                      name: { kind: "Name", value: "referenceCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "referenceCk" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
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
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "value" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Tagging" } },
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
} as unknown as DocumentNode<UpdateTaggingMutation, UpdateTaggingMutationVariables>;
export const CreateTriggerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "createTrigger" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "ck" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "type" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "TriggerType" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "active" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "mapping" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scheduleType" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "ScheduleType" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "timezone" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "interval" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "cron" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "createTrigger" },
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
                      name: { kind: "Name", value: "ck" },
                      value: { kind: "Variable", name: { kind: "Name", value: "ck" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementId" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementId" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "type" },
                      value: { kind: "Variable", name: { kind: "Name", value: "type" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "active" },
                      value: { kind: "Variable", name: { kind: "Name", value: "active" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "mapping" },
                      value: { kind: "Variable", name: { kind: "Name", value: "mapping" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scheduleType" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scheduleType" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "timezone" },
                      value: { kind: "Variable", name: { kind: "Name", value: "timezone" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "interval" },
                      value: { kind: "Variable", name: { kind: "Name", value: "interval" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "cron" },
                      value: { kind: "Variable", name: { kind: "Name", value: "cron" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scopeCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "parent" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "active" } },
                      { kind: "Field", name: { kind: "Name", value: "mapping" } },
                      { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
                      { kind: "Field", name: { kind: "Name", value: "timezone" } },
                      { kind: "Field", name: { kind: "Name", value: "interval" } },
                      { kind: "Field", name: { kind: "Name", value: "cron" } },
                      { kind: "Field", name: { kind: "Name", value: "statementCk" } },
                      { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
} as unknown as DocumentNode<CreateTriggerMutation, CreateTriggerMutationVariables>;
export const SoftDeleteTriggerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "softDeleteTrigger" },
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
            name: { kind: "Name", value: "softDeleteTrigger" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
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
} as unknown as DocumentNode<SoftDeleteTriggerMutation, SoftDeleteTriggerMutationVariables>;
export const RestoreTriggerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "restoreTrigger" },
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
            name: { kind: "Name", value: "restoreTrigger" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
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
} as unknown as DocumentNode<RestoreTriggerMutation, RestoreTriggerMutationVariables>;
export const UpdateTriggerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "updateTrigger" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "id" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "type" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "TriggerType" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "active" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "mapping" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "JSON" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scheduleType" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "ScheduleType" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "timezone" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "interval" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "cron" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "updateTrigger" },
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
                      name: { kind: "Name", value: "active" },
                      value: { kind: "Variable", name: { kind: "Name", value: "active" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "mapping" },
                      value: { kind: "Variable", name: { kind: "Name", value: "mapping" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scheduleType" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scheduleType" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "timezone" },
                      value: { kind: "Variable", name: { kind: "Name", value: "timezone" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "interval" },
                      value: { kind: "Variable", name: { kind: "Name", value: "interval" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "cron" },
                      value: { kind: "Variable", name: { kind: "Name", value: "cron" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "statementCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "statementCk" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "scopeCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "scopeCk" } },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Trigger" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "revision" } },
                      { kind: "Field", name: { kind: "Name", value: "active" } },
                      { kind: "Field", name: { kind: "Name", value: "mapping" } },
                      { kind: "Field", name: { kind: "Name", value: "scheduleType" } },
                      { kind: "Field", name: { kind: "Name", value: "timezone" } },
                      { kind: "Field", name: { kind: "Name", value: "interval" } },
                      { kind: "Field", name: { kind: "Name", value: "cron" } },
                      { kind: "Field", name: { kind: "Name", value: "statementCk" } },
                      { kind: "Field", name: { kind: "Name", value: "scopeCk" } },
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
} as unknown as DocumentNode<UpdateTriggerMutation, UpdateTriggerMutationVariables>;
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
                      { kind: "Field", name: { kind: "Name", value: "status" } },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
} as unknown as DocumentNode<UpdateVersionMutation, UpdateVersionMutationVariables>;
export const SnapshotDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "snapshot" },
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
            name: { kind: "Name", value: "snapshot" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SnapshotPayload" } },
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
                        name: { kind: "Name", value: "snapshot" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
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
          {
            kind: "Field",
            name: { kind: "Name", value: "children" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Project" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "slug" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "head" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "visibility" } },
          { kind: "Field", name: { kind: "Name", value: "accessLevel" } },
          { kind: "Field", name: { kind: "Name", value: "sharingEnabled" } },
          { kind: "Field", name: { kind: "Name", value: "sharingToken" } },
          { kind: "Field", name: { kind: "Name", value: "sharingLevel" } },
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
} as unknown as DocumentNode<SnapshotMutation, SnapshotMutationVariables>;
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
export const CurrentRunsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "currentRuns" },
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
            name: { kind: "Name", value: "currentRuns" },
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
              selections: [
                { kind: "FragmentSpread", name: { kind: "Name", value: "OperationInfoContent" } },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SessionState" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "runs" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "workerSet" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "WorkerSetContent" } }],
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "WorkerSetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WorkerSet" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "region" } },
          { kind: "Field", name: { kind: "Name", value: "profile" } },
          { kind: "Field", name: { kind: "Name", value: "sleeping" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "desiredReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "targetReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "availableReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "readyReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "lastActiveAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CurrentRunsQuery, CurrentRunsQueryVariables>;
export const SessionsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "sessionsChanged" },
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
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "sessionsChanged" },
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
              selections: [
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SessionChange" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "runs" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "RunsChange" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "runs" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "InlineFragment",
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WorkerChange" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "workerSets" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "WorkerSetContent" } }],
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
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "WorkerSetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WorkerSet" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "region" } },
          { kind: "Field", name: { kind: "Name", value: "profile" } },
          { kind: "Field", name: { kind: "Name", value: "sleeping" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "desiredReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "targetReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "availableReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "readyReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "lastActiveAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SessionsChangedSubscription, SessionsChangedSubscriptionVariables>;
export const RefetchProjectWorkerSetsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "refetchProjectWorkerSets" },
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
                  name: { kind: "Name", value: "workerSets" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "WorkerSetContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "WorkerSetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "WorkerSet" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "project" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "region" } },
          { kind: "Field", name: { kind: "Name", value: "profile" } },
          { kind: "Field", name: { kind: "Name", value: "sleeping" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "desiredReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "targetReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "availableReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "readyReplicas" } },
          { kind: "Field", name: { kind: "Name", value: "lastActiveAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<RefetchProjectWorkerSetsQuery, RefetchProjectWorkerSetsQueryVariables>;
export const SearchRunsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "searchRuns" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "rootOnly" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "query" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "SearchQuery" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sort" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "SearchSort" } } },
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
            name: { kind: "Name", value: "searchRuns" },
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
                name: { kind: "Name", value: "statementIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "statementCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runId" },
                value: { kind: "Variable", name: { kind: "Name", value: "runId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "rootOnly" },
                value: { kind: "Variable", name: { kind: "Name", value: "rootOnly" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "cursor" } },
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
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
} as unknown as DocumentNode<SearchRunsQuery, SearchRunsQueryVariables>;
export const RunByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "runById" },
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
            name: { kind: "Name", value: "run" },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "descendants" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "RunContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "RunContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Run" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "duration" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
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
            name: { kind: "Name", value: "session" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
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
                { kind: "Field", name: { kind: "Name", value: "kind" } },
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
          { kind: "Field", name: { kind: "Name", value: "value" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statement" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "triggerType" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "trigger" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerUser" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "username" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "triggerAccessToken" },
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
} as unknown as DocumentNode<RunByIdQuery, RunByIdQueryVariables>;
export const SearchLogsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "searchLogs" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "query" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "SearchQuery" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sort" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "SearchSort" } } },
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
            name: { kind: "Name", value: "searchLogs" },
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
                name: { kind: "Name", value: "statementIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "statementCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runId" },
                value: { kind: "Variable", name: { kind: "Name", value: "runId" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "LogEntryContent" } }],
                        },
                      },
                      { kind: "Field", name: { kind: "Name", value: "cursor" } },
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
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "LogEntryContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "LogEntry" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
          { kind: "Field", name: { kind: "Name", value: "sessionId" } },
          { kind: "Field", name: { kind: "Name", value: "statementId" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "runId" } },
          { kind: "Field", name: { kind: "Name", value: "stream" } },
          { kind: "Field", name: { kind: "Name", value: "level" } },
          { kind: "Field", name: { kind: "Name", value: "logger" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SearchLogsQuery, SearchLogsQueryVariables>;
export const LogsChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "logsChanged" },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "UUID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "logsChanged" },
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
                name: { kind: "Name", value: "statementIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "statementCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "statementCks" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: { kind: "Variable", name: { kind: "Name", value: "sessionId" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runId" },
                value: { kind: "Variable", name: { kind: "Name", value: "runId" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "logs" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "LogEntryContent" } }],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "LogEntryContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "LogEntry" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "projectVersionId" } },
          { kind: "Field", name: { kind: "Name", value: "sessionId" } },
          { kind: "Field", name: { kind: "Name", value: "statementId" } },
          { kind: "Field", name: { kind: "Name", value: "statementCk" } },
          { kind: "Field", name: { kind: "Name", value: "runId" } },
          { kind: "Field", name: { kind: "Name", value: "stream" } },
          { kind: "Field", name: { kind: "Name", value: "level" } },
          { kind: "Field", name: { kind: "Name", value: "logger" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          { kind: "Field", name: { kind: "Name", value: "value" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LogsChangedSubscription, LogsChangedSubscriptionVariables>;
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
            name: { kind: "Name", value: "moduleChanged" },
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
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "clientId" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "edits" },
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
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ResolvedField" } },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "Field", name: { kind: "Name", value: "ck" } },
                                  { kind: "Field", name: { kind: "Name", value: "fieldCk" } },
                                ],
                              },
                            },
                            {
                              kind: "InlineFragment",
                              typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
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
              ],
            },
          },
        ],
      },
    },
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "IssueContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Issue" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "ck" } },
          { kind: "Field", name: { kind: "Name", value: "kind" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
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
