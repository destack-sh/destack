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
  /** The Globally Unique ID of this object */
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
  /** Total quantity of existing nodes. */
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
  AND?: InputMaybe<AccessTokenFilter>;
  OR?: InputMaybe<AccessTokenFilter>;
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
  /** The Globally Unique ID of this object */
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
  /** Total quantity of existing nodes. */
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

export type DeleteObjectInput = {
  id: Scalars["GlobalID"];
};

export type Environment = {
  __typename?: "Environment";
  language: Scalars["String"];
  packages: Array<Package>;
  platform: Scalars["String"];
  version: Scalars["String"];
};

export type EnvironmentOperationInfo = Environment | OperationInfo;

export type Field = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Field";
    ck: Scalars["UUID"];
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    flags: Scalars["Int"];
    hint?: Maybe<TypeHint>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    key: Scalars["String"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    metadata?: Maybe<Scalars["JSON"]>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent: Statement;
    referenceCk?: Maybe<Scalars["UUID"]>;
    revision: Scalars["Int"];
    statement: Statement;
    tag: TypeTag;
    text?: Maybe<Scalars["String"]>;
    updatedAt: Scalars["DateTime"];
  };

export type FieldCreateInput = {
  ck: Scalars["UUID"];
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  key: Scalars["String"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  referenceCk?: InputMaybe<Scalars["UUID"]>;
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  text?: InputMaybe<Scalars["String"]>;
};

export type FieldDeleteInput = {
  id: Scalars["GlobalID"];
};

export type FieldFilter = {
  AND?: InputMaybe<FieldFilter>;
  OR?: InputMaybe<FieldFilter>;
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

export type FieldUpdateInput = {
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  name?: InputMaybe<Scalars["String"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]>;
  tag: TypeTag;
  text?: InputMaybe<Scalars["String"]>;
};

export type FieldUpdateTextInput = {
  id: Scalars["GlobalID"];
  text?: InputMaybe<Scalars["String"]>;
};

export type FieldUpdateTypeInput = {
  flags?: Scalars["Int"];
  hint?: InputMaybe<TypeHint>;
  id: Scalars["GlobalID"];
  referenceCk?: InputMaybe<Scalars["GlobalID"]>;
  tag: TypeTag;
};

export type File = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "File";
    ck: Scalars["UUID"];
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    /** The Globally Unique ID of this object */
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

export type FileStatementsArgs = {
  filters?: InputMaybe<StatementFilter>;
};

export type FileCreateInput = {
  ck: Scalars["UUID"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId: Scalars["GlobalID"];
};

export type FileFilter = {
  AND?: InputMaybe<FileFilter>;
  OR?: InputMaybe<FileFilter>;
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
  targetCk: Scalars["UUID"];
  targetId: Scalars["GlobalID"];
  targetVersionId: Scalars["GlobalID"];
};

export type FileRenameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type HasCrud = {
  createdAt: Scalars["DateTime"];
  createdBy?: Maybe<User>;
  deletedAt?: Maybe<Scalars["DateTime"]>;
  id: Scalars["GlobalID"];
  lastEditedAt?: Maybe<Scalars["DateTime"]>;
  lastEditedBy?: Maybe<User>;
  updatedAt: Scalars["DateTime"];
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
    ck: Scalars["UUID"];
    id: Scalars["GlobalID"];
    kind: IssueKind;
    message?: Maybe<Scalars["String"]>;
    parent?: Maybe<ModuleNode>;
    type: IssueType;
  };

export enum IssueKind {
  Error = "Error",
  Notice = "Notice",
  Warning = "Warning",
}

export type IssueResolvedField = Issue | ResolvedField;

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
  TaskImpossible = "TASK_IMPOSSIBLE",
  TaskMissingIo = "TASK_MISSING_IO",
  TextHasNoEffect = "TEXT_HAS_NO_EFFECT",
  UnknownImportSource = "UNKNOWN_IMPORT_SOURCE",
}

export type KillRunInput = {
  projectVersionId: Scalars["GlobalID"];
  runId: Scalars["GlobalID"];
};

export type KillRunPayload = {
  __typename?: "KillRunPayload";
  run?: Maybe<Run>;
  success: Scalars["Boolean"];
};

export type KillRunPayloadOperationInfo = KillRunPayload | OperationInfo;

export type LogChange = {
  __typename?: "LogChange";
  logs: Array<LogEntry>;
};

export type LogEntry = {
  __typename?: "LogEntry";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  level?: Maybe<Scalars["String"]>;
  logger?: Maybe<Scalars["String"]>;
  message?: Maybe<Scalars["String"]>;
  metadata?: Maybe<Scalars["JSON"]>;
  projectVersionId: Scalars["GlobalID"];
  runId?: Maybe<Scalars["GlobalID"]>;
  runnableCk?: Maybe<Scalars["UUID"]>;
  runnableId?: Maybe<Scalars["GlobalID"]>;
  sessionId?: Maybe<Scalars["GlobalID"]>;
  stream: Scalars["String"];
};

/** A connection to a list of items. */
export type LogEntryConnection = {
  __typename?: "LogEntryConnection";
  /** Contains the nodes in this connection */
  edges: Array<LogEntryEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type LogEntryEdge = {
  __typename?: "LogEntryEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: LogEntry;
};

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
  CreateTrigger = "CREATE_TRIGGER",
  DeleteField = "DELETE_FIELD",
  DeleteFile = "DELETE_FILE",
  DeleteIssue = "DELETE_ISSUE",
  DeleteRecord = "DELETE_RECORD",
  DeleteStatement = "DELETE_STATEMENT",
  DeleteTagging = "DELETE_TAGGING",
  DeleteTrigger = "DELETE_TRIGGER",
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
  RestoreTrigger = "RESTORE_TRIGGER",
  SoftDeleteField = "SOFT_DELETE_FIELD",
  SoftDeleteFile = "SOFT_DELETE_FILE",
  SoftDeleteRecord = "SOFT_DELETE_RECORD",
  SoftDeleteStatement = "SOFT_DELETE_STATEMENT",
  SoftDeleteTagging = "SOFT_DELETE_TAGGING",
  SoftDeleteTrigger = "SOFT_DELETE_TRIGGER",
  TruncateFields = "TRUNCATE_FIELDS",
  TruncateFiles = "TRUNCATE_FILES",
  TruncateIssues = "TRUNCATE_ISSUES",
  TruncateRecords = "TRUNCATE_RECORDS",
  TruncateResolvedFields = "TRUNCATE_RESOLVED_FIELDS",
  TruncateStatements = "TRUNCATE_STATEMENTS",
  TruncateTaggings = "TRUNCATE_TAGGINGS",
  TruncateTriggers = "TRUNCATE_TRIGGERS",
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

export type ModuleNode = {
  ck: Scalars["UUID"];
  id: Scalars["GlobalID"];
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
  deleteObject: RemoteObjectOperationInfo;
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
  moveRecord: RecordOperationInfo;
  moveStatement: StatementOperationInfo;
  notifyUploadedObject: RemoteObjectOperationInfo;
  pasteFile: FileOperationInfo;
  removeOrganizationMembership: OrganizationOperationInfo;
  removeProjectMembership: ProjectOperationInfo;
  renameFile: FileOperationInfo;
  renameStatement: StatementOperationInfo;
  requestUploadObject: RemoteObjectOperationInfo;
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
  id: Scalars["GlobalID"];
};

export type MutationAcceptProjectInviteArgs = {
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

export type MutationCancelProjectInviteArgs = {
  id: Scalars["GlobalID"];
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
  id: Scalars["GlobalID"];
};

export type MutationRunArgs = {
  input: RunInput;
};

export type MutationSecretRootLoginArgs = {
  username: Scalars["String"];
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
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  organizationInvite: OrganizationInvite;
  projectInvite: ProjectInvite;
  readAt?: Maybe<Scalars["DateTime"]>;
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
  AND?: InputMaybe<NotificationFilter>;
  OR?: InputMaybe<NotificationFilter>;
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
  ProjectInvite = "PROJECT_INVITE",
  RunFailed = "RUN_FAILED",
  RunSuspended = "RUN_SUSPENDED",
}

export type NotifyUploadedObjectInput = {
  id: Scalars["GlobalID"];
};

export type OperationInfo = {
  __typename?: "OperationInfo";
  /** List of messages returned by the operation. */
  messages: Array<OperationMessage>;
};

export type OperationMessage = {
  __typename?: "OperationMessage";
  /** The field that caused the error, or `null` if it isn't associated with any particular field. */
  field?: Maybe<Scalars["String"]>;
  /** The kind of this message. */
  kind: OperationMessageKind;
  /** The error message. */
  message: Scalars["String"];
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
    canViewDetail: Scalars["Boolean"];
    canWrite: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    invites: OrganizationInviteConnection;
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
  /** Total quantity of existing nodes. */
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
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  level: OrganizationRole;
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
  /** Total quantity of existing nodes. */
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
  level: OrganizationRole;
  message?: InputMaybe<Scalars["String"]>;
};

export type OrganizationMembership = Node & {
  __typename?: "OrganizationMembership";
  createdAt: Scalars["DateTime"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  level: OrganizationRole;
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
  /** Total quantity of existing nodes. */
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

export type OrganizationMembershipOperationInfo = OperationInfo | OrganizationMembership;

export type OrganizationOperationInfo = OperationInfo | Organization;

export type OrganizationRemoveMembershipInput = {
  id: Scalars["GlobalID"];
  userId: Scalars["GlobalID"];
};

export enum OrganizationRole {
  Guest = "Guest",
  Manager = "Manager",
  Member = "Member",
  Owner = "Owner",
}

export type OrganizationUpdateInput = {
  description: Scalars["String"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type OrganizationUpdateMembershipInput = {
  id: Scalars["GlobalID"];
  level: OrganizationRole;
  userId: Scalars["GlobalID"];
};

export type Owner = {
  accessTokens: AccessTokenConnection;
  canViewDetail: Scalars["Boolean"];
  canWrite: Scalars["Boolean"];
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  projects: ProjectConnection;
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

export type Package = {
  __typename?: "Package";
  name: Scalars["String"];
  version: Scalars["String"];
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
  accessLevel: ProjectAccessLevel;
  createdAt: Scalars["DateTime"];
  description?: Maybe<Scalars["String"]>;
  head: ProjectVersion;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  owner: UserOrganization;
  path: Scalars["String"];
  sharingEnabled: Scalars["Boolean"];
  sharingLevel: ProjectAccessLevel;
  sharingToken?: Maybe<Scalars["UUID"]>;
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  usage: ProjectUsage;
  versions: ProjectVersionConnection;
  visibility: ProjectVisibility;
  workerSet: WorkerSet;
  workerSets: Array<WorkerSet>;
};

export type ProjectVersionsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  filters?: InputMaybe<ProjectVersionFilter>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export enum ProjectAccessLevel {
  Admin = "Admin",
  Edit = "Edit",
  Manage = "Manage",
  Read = "Read",
  Use = "Use",
  Zero = "Zero",
}

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
  /** Total quantity of existing nodes. */
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

export type ProjectInvite = Node & {
  __typename?: "ProjectInvite";
  createdAt: Scalars["DateTime"];
  email: Scalars["String"];
  emailSentAt?: Maybe<Scalars["DateTime"]>;
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  level: ProjectAccessLevel;
  project: Project;
  updatedAt: Scalars["DateTime"];
  user?: Maybe<User>;
};

export type ProjectInviteInput = {
  emails: Array<Scalars["String"]>;
  id: Scalars["GlobalID"];
  level: ProjectAccessLevel;
  message?: InputMaybe<Scalars["String"]>;
};

export type ProjectOperationInfo = OperationInfo | Project;

export type ProjectRemoveMembershipInput = {
  id: Scalars["GlobalID"];
  userId: Scalars["GlobalID"];
};

export type ProjectUpdateNameInput = {
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type ProjectUpdateSharingInput = {
  id: Scalars["GlobalID"];
  sharingEnabled: Scalars["Boolean"];
  sharingLevel: ProjectAccessLevel;
  sharingToken: Scalars["UUID"];
};

export type ProjectUpdateVisibilityInput = {
  id: Scalars["GlobalID"];
  visibility: ProjectVisibility;
};

export type ProjectUsage = {
  __typename?: "ProjectUsage";
  cacheBytesTotal: Scalars["Int"];
  objectsBytesTotal: Scalars["Int"];
  recordsActive: Scalars["Int"];
};

export type ProjectVersion = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "ProjectVersion";
    children: Array<ProjectVersion>;
    ck: Scalars["UUID"];
    committed: Scalars["Boolean"];
    committedAt?: Maybe<Scalars["DateTime"]>;
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    description?: Maybe<Scalars["String"]>;
    files: Array<File>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]>;
    parent?: Maybe<ModuleNode>;
    parents: Array<ProjectVersion>;
    project: Project;
    tag?: Maybe<Scalars["String"]>;
    updatedAt: Scalars["DateTime"];
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
  AND?: InputMaybe<ProjectVersionFilter>;
  OR?: InputMaybe<ProjectVersionFilter>;
  fromId: Scalars["GlobalID"];
  toId: Scalars["GlobalID"];
};

export type ProjectVersionOperationInfo = OperationInfo | ProjectVersion;

export enum ProjectVisibility {
  Private = "PRIVATE",
  Public = "PUBLIC",
  SourcePrivate = "SOURCE_PRIVATE",
}

export type Query = {
  __typename?: "Query";
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
  remoteObject?: Maybe<RemoteObject>;
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

export type QueryCurrentRunsArgs = {
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
};

export type QueryEnvironmentArgs = {
  projectId: Scalars["GlobalID"];
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

export type QueryModuleArgs = {
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

export type QueryRunArgs = {
  id: Scalars["GlobalID"];
};

export type QuerySearchLogsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  query?: InputMaybe<SearchQuery>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]>>;
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  sort?: InputMaybe<Array<SearchSort>>;
};

export type QuerySearchRecordsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort>>;
  statementId: Scalars["GlobalID"];
};

export type QuerySearchRunsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  query?: InputMaybe<SearchQuery>;
  rootOnly?: InputMaybe<Scalars["Boolean"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]>>;
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  sort?: InputMaybe<Array<SearchSort>>;
};

export type QuerySecretArgs = {
  id: Scalars["GlobalID"];
};

export type QuerySessionArgs = {
  id: Scalars["GlobalID"];
};

export type QueryStatementArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUserArgs = {
  id: Scalars["GlobalID"];
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

export type Record = HasCrud & {
  __typename?: "Record";
  ck: Scalars["UUID"];
  createdAt: Scalars["DateTime"];
  createdBy?: Maybe<User>;
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
  totalCount?: Maybe<Scalars["Int"]>;
};

export type RecordCreateInput = {
  ck: Scalars["UUID"];
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

export type RemoteObject = Node & {
  __typename?: "RemoteObject";
  contentLength: Scalars["Int"];
  contentType: Scalars["String"];
  /** The Globally Unique ID of this object */
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

export type ResolvedField = {
  __typename?: "ResolvedField";
  fieldCk: Scalars["UUID"];
  statement?: Maybe<Statement>;
};

export type RestartWorkerSetInput = {
  projectId: Scalars["GlobalID"];
};

export type RestartWorkerSetPayload = {
  __typename?: "RestartWorkerSetPayload";
  success: Scalars["Boolean"];
  workerSet?: Maybe<WorkerSet>;
};

export type RestartWorkerSetPayloadOperationInfo = OperationInfo | RestartWorkerSetPayload;

export type RestoreInput = {
  projectVersionId: Scalars["GlobalID"];
};

export type Run = HasTriggeredBy &
  Node & {
    __typename?: "Run";
    children: Array<Run>;
    createdAt: Scalars["DateTime"];
    descendants: Array<Run>;
    duration?: Maybe<Scalars["Float"]>;
    error?: Maybe<Scalars["JSON"]>;
    errorNice?: Maybe<RunError>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    inputs?: Maybe<Scalars["JSON"]>;
    metadata?: Maybe<Scalars["JSON"]>;
    outputs?: Maybe<Scalars["JSON"]>;
    parent?: Maybe<Run>;
    projectVersion: ProjectVersion;
    root?: Maybe<Run>;
    runnable?: Maybe<Statement>;
    runnableCk?: Maybe<Scalars["UUID"]>;
    session?: Maybe<Session>;
    startedAt?: Maybe<Scalars["DateTime"]>;
    status: RunStatus;
    terminatedAt?: Maybe<Scalars["DateTime"]>;
    trigger?: Maybe<Trigger>;
    triggerAccessToken?: Maybe<AccessToken>;
    triggerType?: Maybe<TriggerType>;
    triggerUser?: Maybe<User>;
    updatedAt: Scalars["DateTime"];
  };

export type RunCodeFrame = {
  __typename?: "RunCodeFrame";
  filename: Scalars["String"];
  line: Scalars["String"];
  lineno: Scalars["Int"];
  locals?: Maybe<Scalars["JSON"]>;
  name: Scalars["String"];
};

/** A connection to a list of items. */
export type RunConnection = {
  __typename?: "RunConnection";
  /** Contains the nodes in this connection */
  edges: Array<RunEdge>;
  /** Pagination data for this connection */
  pageInfo: PageInfo;
  totalCount?: Maybe<Scalars["Int"]>;
};

/** An edge in a connection. */
export type RunEdge = {
  __typename?: "RunEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Run;
};

export type RunError = {
  __typename?: "RunError";
  kind: Scalars["String"];
  message: Scalars["String"];
  statementId?: Maybe<Scalars["GlobalID"]>;
  traceback?: Maybe<Array<RunCodeFrame>>;
  type: Scalars["String"];
};

export type RunInput = {
  block?: Scalars["Float"];
  inputs?: InputMaybe<Scalars["JSON"]>;
  keyed?: Scalars["Boolean"];
  projectVersionId: Scalars["GlobalID"];
  runId?: InputMaybe<Scalars["GlobalID"]>;
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]>;
};

export type RunState = {
  __typename?: "RunState";
  error?: Maybe<StartRunErrorType>;
  logs?: Maybe<Array<LogEntry>>;
  projectVersionId: Scalars["GlobalID"];
  run?: Maybe<Run>;
  runnableId?: Maybe<Scalars["GlobalID"]>;
  success: Scalars["Boolean"];
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
  key?: InputMaybe<Scalars["String"]>;
  op: QueryOp;
  queries?: InputMaybe<Array<SearchQuery>>;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type SearchSort = {
  key: Scalars["String"];
  mode?: InputMaybe<SortMode>;
  order?: SortOrder;
};

export type Secret = Node & {
  __typename?: "Secret";
  createdAt: Scalars["DateTime"];
  /** The Globally Unique ID of this object */
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

export type Session = HasTriggeredBy &
  Node & {
    __typename?: "Session";
    closedAt?: Maybe<Scalars["DateTime"]>;
    createdAt: Scalars["DateTime"];
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    metadata?: Maybe<Scalars["JSON"]>;
    openedAt?: Maybe<Scalars["DateTime"]>;
    project: Project;
    runs: Array<Run>;
    trigger?: Maybe<Trigger>;
    triggerAccessToken?: Maybe<AccessToken>;
    triggerType?: Maybe<TriggerType>;
    triggerUser?: Maybe<User>;
    updatedAt: Scalars["DateTime"];
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
  description?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  projectVersionId: Scalars["GlobalID"];
  tag?: InputMaybe<Scalars["String"]>;
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
    ck: Scalars["UUID"];
    code?: Maybe<Scalars["String"]>;
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    descendants: Array<Statement>;
    fields: Array<Field>;
    file: File;
    flags?: Maybe<Scalars["Int"]>;
    headingLevel?: Maybe<Scalars["Int"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    issues?: Maybe<Array<Issue>>;
    key?: Maybe<Scalars["String"]>;
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    name?: Maybe<Scalars["String"]>;
    orderKey: Scalars["String"];
    parent: ModuleNode;
    projectVersion: ProjectVersion;
    referenceCk?: Maybe<Scalars["UUID"]>;
    resolvedFields?: Maybe<Array<ResolvedField>>;
    revision: Scalars["Int"];
    tag?: Maybe<TypeTag>;
    tags: Array<Tagging>;
    text?: Maybe<Scalars["String"]>;
    triggers: Array<Trigger>;
    type: StatementType;
    updatedAt: Scalars["DateTime"];
    value?: Maybe<Scalars["JSON"]>;
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
  fileId: Scalars["GlobalID"];
  ids: Array<Scalars["GlobalID"]>;
  orderKeys: Array<Scalars["String"]>;
  parentIds: Array<InputMaybe<Scalars["GlobalID"]>>;
};

export type StatementBatchOperationInfo = OperationInfo | StatementBatch;

export type StatementBatchPasteInput = {
  sourceIds: Array<Scalars["GlobalID"]>;
  targetCks: Array<Scalars["UUID"]>;
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

export type StatementCreateInput = {
  ck: Scalars["UUID"];
  code?: InputMaybe<Scalars["String"]>;
  fileId: Scalars["GlobalID"];
  flags?: InputMaybe<Scalars["Int"]>;
  id: Scalars["GlobalID"];
  key?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]>;
  tag?: InputMaybe<TypeTag>;
  text?: InputMaybe<Scalars["String"]>;
  type: StatementType;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type StatementDeleteInput = {
  id: Scalars["GlobalID"];
};

export type StatementFilter = {
  AND?: InputMaybe<StatementFilter>;
  OR?: InputMaybe<StatementFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type StatementMorphInput = {
  flags?: InputMaybe<Scalars["Int"]>;
  headingLevel?: InputMaybe<Scalars["Int"]>;
  id: Scalars["GlobalID"];
  key?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  tag?: InputMaybe<TypeTag>;
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

export enum StatementType {
  Blank = "BLANK",
  Code = "CODE",
  Dataset = "DATASET",
  Flow = "FLOW",
  Model = "MODEL",
  Reference = "REFERENCE",
  Tag = "TAG",
  Task = "TASK",
  Text = "TEXT",
  Type = "TYPE",
  Variable = "VARIABLE",
}

export type StatementUpdateHeadingLevelInput = {
  headingLevel?: InputMaybe<Scalars["Int"]>;
  id: Scalars["GlobalID"];
};

export type StatementUpdateInput = {
  code?: InputMaybe<Scalars["String"]>;
  flags?: InputMaybe<Scalars["Int"]>;
  id: Scalars["GlobalID"];
  key?: InputMaybe<Scalars["String"]>;
  name?: InputMaybe<Scalars["String"]>;
  orderKey?: InputMaybe<Scalars["String"]>;
  referenceCk?: InputMaybe<Scalars["UUID"]>;
  tag?: InputMaybe<TypeTag>;
  text?: InputMaybe<Scalars["String"]>;
  type?: InputMaybe<StatementType>;
  value?: InputMaybe<Scalars["JSON"]>;
};

export type StatementUpdateReferenceInput = {
  id: Scalars["GlobalID"];
  referenceCk?: InputMaybe<Scalars["UUID"]>;
};

export type StatementUpdateTextInput = {
  id: Scalars["GlobalID"];
  text?: InputMaybe<Scalars["String"]>;
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
  projectId?: InputMaybe<Scalars["GlobalID"]>;
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
};

export type SubscriptionLogsChangedArgs = {
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]>>;
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]>>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
};

export type SubscriptionModuleChangedArgs = {
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
};

export type SubscriptionProjectChangedArgs = {
  projectId: Scalars["GlobalID"];
};

export type SubscriptionSessionsChangedArgs = {
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
};

export type SymbolUpdateCodeInput = {
  code?: InputMaybe<Scalars["String"]>;
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

export type Tagging = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Tagging";
    ck: Scalars["UUID"];
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    key: Scalars["String"];
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    metadata?: Maybe<Scalars["JSON"]>;
    parent: Statement;
    referenceCk?: Maybe<Scalars["UUID"]>;
    revision: Scalars["Int"];
    statement: Statement;
    updatedAt: Scalars["DateTime"];
  };

export type TaggingCreateInput = {
  ck: Scalars["UUID"];
  id: Scalars["GlobalID"];
  key: Scalars["String"];
  metadata?: InputMaybe<Scalars["JSON"]>;
  referenceCk: Scalars["UUID"];
  statementId: Scalars["GlobalID"];
};

export type TaggingDeleteInput = {
  id: Scalars["GlobalID"];
};

export type TaggingFilter = {
  AND?: InputMaybe<TaggingFilter>;
  OR?: InputMaybe<TaggingFilter>;
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

export type Trigger = HasCrud &
  ModuleNode &
  Node & {
    __typename?: "Trigger";
    active: Scalars["Boolean"];
    ck: Scalars["UUID"];
    createdAt: Scalars["DateTime"];
    createdBy?: Maybe<User>;
    cron?: Maybe<Scalars["String"]>;
    deletedAt?: Maybe<Scalars["DateTime"]>;
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    interval?: Maybe<Scalars["Int"]>;
    lastEditedAt?: Maybe<Scalars["DateTime"]>;
    lastEditedBy?: Maybe<User>;
    mapping?: Maybe<Scalars["JSON"]>;
    parent: Statement;
    revision: Scalars["Int"];
    runnableCk?: Maybe<Scalars["UUID"]>;
    scheduleType: ScheduleType;
    scopeCk?: Maybe<Scalars["UUID"]>;
    timezone?: Maybe<Scalars["String"]>;
    type: TriggerType;
    updatedAt: Scalars["DateTime"];
  };

export type TriggerCreateInput = {
  active: Scalars["Boolean"];
  ck: Scalars["UUID"];
  cron?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  interval?: InputMaybe<Scalars["Int"]>;
  mapping?: InputMaybe<Scalars["JSON"]>;
  runnableCk?: InputMaybe<Scalars["UUID"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  scopeCk?: InputMaybe<Scalars["UUID"]>;
  statementId: Scalars["GlobalID"];
  timezone?: InputMaybe<Scalars["String"]>;
  type: TriggerType;
};

export type TriggerDeleteInput = {
  id: Scalars["GlobalID"];
};

export type TriggerFilter = {
  AND?: InputMaybe<TriggerFilter>;
  OR?: InputMaybe<TriggerFilter>;
  isVisible?: InputMaybe<Scalars["Boolean"]>;
};

export type TriggerOperationInfo = OperationInfo | Trigger;

export type TriggerRestoreInput = {
  id: Scalars["GlobalID"];
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
  active: Scalars["Boolean"];
  cron?: InputMaybe<Scalars["String"]>;
  id: Scalars["GlobalID"];
  interval?: InputMaybe<Scalars["Int"]>;
  mapping?: InputMaybe<Scalars["JSON"]>;
  runnableCk?: InputMaybe<Scalars["UUID"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  scopeCk?: InputMaybe<Scalars["UUID"]>;
  timezone?: InputMaybe<Scalars["String"]>;
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
    canViewDetail: Scalars["Boolean"];
    canWrite: Scalars["Boolean"];
    createdAt: Scalars["DateTime"];
    description?: Maybe<Scalars["String"]>;
    email: Scalars["String"];
    /** The Globally Unique ID of this object */
    id: Scalars["GlobalID"];
    name: Scalars["String"];
    notifications: NotificationConnection;
    organizationMemberships: OrganizationMembershipConnection;
    organizations: OrganizationConnection;
    projects: ProjectConnection;
    slug: Scalars["String"];
    status: UserStatus;
    updatedAt: Scalars["DateTime"];
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
  /** Total quantity of existing nodes. */
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
  AND?: InputMaybe<UserFilter>;
  OR?: InputMaybe<UserFilter>;
  emailEquals?: InputMaybe<Scalars["String"]>;
  slugPrefix?: InputMaybe<Scalars["String"]>;
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
  description: Scalars["String"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
};

export type WakeRuntimeInput = {
  projectVersionId: Scalars["GlobalID"];
};

export type WakeRuntimePayload = {
  __typename?: "WakeRuntimePayload";
  success: Scalars["Boolean"];
};

export type WakeRuntimePayloadOperationInfo = OperationInfo | WakeRuntimePayload;

export type WakeWorkerSetInput = {
  projectId: Scalars["GlobalID"];
};

export type WakeWorkerSetPayload = {
  __typename?: "WakeWorkerSetPayload";
  success: Scalars["Boolean"];
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
  availableReplicas: Scalars["Int"];
  createdAt: Scalars["DateTime"];
  desiredReplicas: Scalars["Int"];
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
  lastActiveAt?: Maybe<Scalars["DateTime"]>;
  lastBumpedAt?: Maybe<Scalars["DateTime"]>;
  profile: WorkerProfile;
  project: Project;
  readyReplicas: Scalars["Int"];
  region: WorkerRegion;
  sleeping: Scalars["Boolean"];
  status: WorkerSetStatus;
  targetReplicas: Scalars["Int"];
  updatedAt: Scalars["DateTime"];
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
          organizationInvite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationRole;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
          projectInvite: {
            __typename?: "ProjectInvite";
            id: any;
            level: ProjectAccessLevel;
            project: { __typename?: "Project"; id: any; slug: string; name: string };
          };
        };
      }>;
    };
  } | null;
};

export type ExistingProjectVersionTagQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  tag: Scalars["String"];
}>;

export type ExistingProjectVersionTagQuery = {
  __typename?: "Query";
  projectVersionByTag?: { __typename?: "ProjectVersion"; id: any; tag?: string | null } | null;
};

export type BlankPanelSuggestedFilesQueryVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
}>;

export type BlankPanelSuggestedFilesQuery = {
  __typename?: "Query";
  module?: {
    __typename?: "ProjectVersion";
    files: Array<{ __typename?: "File"; id: any; ck: any; name: string; deletedAt?: any | null }>;
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
        ck: any;
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
        parent:
          | { __typename?: "Field"; id: any }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue"; id: any }
          | { __typename?: "ProjectVersion"; id: any }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging"; id: any }
          | { __typename?: "Trigger"; id: any };
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

export type EnvironmentQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
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

export type ProjectVersionHeaderQueryVariables = Exact<{
  id: Scalars["GlobalID"];
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

export type SearchRecordsQueryVariables = Exact<{
  statementId: Scalars["GlobalID"];
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
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
        orderKey?: string | null;
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
  accessLevel: ProjectAccessLevel;
  sharingEnabled: boolean;
  sharingToken?: any | null;
  sharingLevel: ProjectAccessLevel;
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
  metadata?: any | null;
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
  metadata?: any | null;
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
  runnableCk?: any | null;
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
  tag?: TypeTag | null;
  flags?: number | null;
  referenceCk?: any | null;
  createdAt: any;
  updatedAt: any;
  deletedAt?: any | null;
  lastEditedAt?: any | null;
  parent:
    | { __typename?: "Field"; id: any }
    | { __typename?: "File"; id: any }
    | { __typename?: "Issue"; id: any }
    | { __typename?: "ProjectVersion"; id: any }
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
    | { __typename?: "Statement"; id: any }
    | { __typename?: "Tagging"; id: any }
    | { __typename?: "Trigger"; id: any }
    | null;
} & { " $fragmentName"?: "IssueContentFragment" };

export type ResolvedFieldContentFragment = {
  __typename?: "ResolvedField";
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
  tag?: TypeTag | null;
  flags?: number | null;
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
  projectVersionId: Scalars["GlobalID"];
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
          organizationInvite: {
            __typename?: "OrganizationInvite";
            id: any;
            level: OrganizationRole;
            organization: { __typename?: "Organization"; id: any; slug: string; name: string };
          };
          projectInvite: {
            __typename?: "ProjectInvite";
            id: any;
            level: ProjectAccessLevel;
            project: { __typename?: "Project"; id: any; slug: string; name: string };
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
  id: Scalars["GlobalID"];
  ck: Scalars["UUID"];
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
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
  targetId: Scalars["GlobalID"];
  targetCk: Scalars["UUID"];
  targetVersionId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
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
  level: OrganizationRole;
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

export type UpdateProjectSharingMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  sharingEnabled: Scalars["Boolean"];
  sharingToken: Scalars["UUID"];
  sharingLevel: ProjectAccessLevel;
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
        sharingLevel: ProjectAccessLevel;
      };
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

export type WakeRuntimeMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
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
  projectId: Scalars["GlobalID"];
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
  projectId: Scalars["GlobalID"];
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
  projectVersionId: Scalars["GlobalID"];
  runnableId?: InputMaybe<Scalars["GlobalID"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  inputs?: InputMaybe<Scalars["JSON"]>;
  keyed?: InputMaybe<Scalars["Boolean"]>;
  block?: InputMaybe<Scalars["Float"]>;
  timeoutSeconds?: InputMaybe<Scalars["Int"]>;
}>;

export type StartRunMutation = {
  __typename?: "Mutation";
  run:
    | { __typename?: "OperationInfo" }
    | {
        __typename?: "RunState";
        projectVersionId: any;
        runnableId?: any | null;
        success: boolean;
        error?: StartRunErrorType | null;
        run?: ({ __typename?: "Run" } & { " $fragmentRefs"?: { RunContentFragment: RunContentFragment } }) | null;
        logs?: Array<
          { __typename?: "LogEntry" } & { " $fragmentRefs"?: { LogEntryContentFragment: LogEntryContentFragment } }
        > | null;
      };
};

export type KillMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  runId: Scalars["GlobalID"];
}>;

export type KillMutation = {
  __typename?: "Mutation";
  killRun:
    | {
        __typename?: "KillRunPayload";
        success: boolean;
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
  id: Scalars["GlobalID"];
  ck: Scalars["UUID"];
  fileId: Scalars["GlobalID"];
  parentId?: InputMaybe<Scalars["GlobalID"]>;
  orderKey: Scalars["String"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  key?: InputMaybe<Scalars["String"]>;
  code?: InputMaybe<Scalars["String"]>;
  text?: InputMaybe<Scalars["String"]>;
  value?: InputMaybe<Scalars["JSON"]>;
  tag?: InputMaybe<TypeTag>;
  flags?: InputMaybe<Scalars["Int"]>;
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
        tag?: TypeTag | null;
        flags?: number | null;
        referenceCk?: any | null;
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
  id: Scalars["GlobalID"];
  orderKey: Scalars["String"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  code?: InputMaybe<Scalars["String"]>;
  text?: InputMaybe<Scalars["String"]>;
  value?: InputMaybe<Scalars["JSON"]>;
  tag?: InputMaybe<TypeTag>;
  flags?: InputMaybe<Scalars["Int"]>;
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
        tag?: TypeTag | null;
        flags?: number | null;
      };
};

export type MorphStatementMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  type: StatementType;
  name?: InputMaybe<Scalars["String"]>;
  tag?: InputMaybe<TypeTag>;
  flags?: InputMaybe<Scalars["Int"]>;
  key?: InputMaybe<Scalars["String"]>;
  headingLevel?: InputMaybe<Scalars["Int"]>;
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
        tag?: TypeTag | null;
        flags?: number | null;
        key?: string | null;
        headingLevel?: number | null;
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
        parent:
          | { __typename?: "Field" }
          | { __typename?: "File"; id: any }
          | { __typename?: "Issue" }
          | { __typename?: "ProjectVersion" }
          | { __typename?: "Statement"; id: any }
          | { __typename?: "Tagging" }
          | { __typename?: "Trigger" };
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
          parent:
            | { __typename?: "Field" }
            | { __typename?: "File"; id: any }
            | { __typename?: "Issue" }
            | { __typename?: "ProjectVersion" }
            | { __typename?: "Statement"; id: any }
            | { __typename?: "Tagging" }
            | { __typename?: "Trigger" };
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
  targetCks: Array<Scalars["UUID"]> | Scalars["UUID"];
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
  referenceCk?: InputMaybe<Scalars["UUID"]>;
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
  ck: Scalars["UUID"];
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
        ck: any;
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
    | { __typename?: "Record"; id: any; deletedAt?: any | null; revision: number };
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
  ck: Scalars["UUID"];
  statementId: Scalars["GlobalID"];
  tag: TypeTag;
  hint?: InputMaybe<TypeHint>;
  key: Scalars["String"];
  orderKey: Scalars["String"];
  name?: InputMaybe<Scalars["String"]>;
  text?: InputMaybe<Scalars["String"]>;
  flags: Scalars["Int"];
  referenceCk?: InputMaybe<Scalars["UUID"]>;
  metadata?: InputMaybe<Scalars["JSON"]>;
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
        metadata?: any | null;
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
  restoreField:
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
  text?: InputMaybe<Scalars["String"]>;
  flags: Scalars["Int"];
  referenceCk?: InputMaybe<Scalars["UUID"]>;
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
        text?: string | null;
        flags: number;
        referenceCk?: any | null;
        metadata?: any | null;
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
  ck: Scalars["UUID"];
  statementId: Scalars["GlobalID"];
  key: Scalars["String"];
  referenceCk: Scalars["UUID"];
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
        ck: any;
        revision: number;
        key: string;
        referenceCk?: any | null;
        metadata?: any | null;
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

export type CreateTriggerMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  ck: Scalars["UUID"];
  statementId: Scalars["GlobalID"];
  type: TriggerType;
  active: Scalars["Boolean"];
  mapping?: InputMaybe<Scalars["JSON"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  timezone?: InputMaybe<Scalars["String"]>;
  interval?: InputMaybe<Scalars["Int"]>;
  cron?: InputMaybe<Scalars["String"]>;
  runnableCk?: InputMaybe<Scalars["UUID"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]>;
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
        runnableCk?: any | null;
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
  id: Scalars["GlobalID"];
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
  id: Scalars["GlobalID"];
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
  id: Scalars["GlobalID"];
  type: TriggerType;
  active: Scalars["Boolean"];
  mapping?: InputMaybe<Scalars["JSON"]>;
  scheduleType?: InputMaybe<ScheduleType>;
  timezone?: InputMaybe<Scalars["String"]>;
  interval?: InputMaybe<Scalars["Int"]>;
  cron?: InputMaybe<Scalars["String"]>;
  runnableCk?: InputMaybe<Scalars["UUID"]>;
  scopeCk?: InputMaybe<Scalars["UUID"]>;
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
        runnableCk?: any | null;
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

export type SnapshotMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
  tag?: InputMaybe<Scalars["String"]>;
  description?: InputMaybe<Scalars["String"]>;
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
  secretId: Scalars["GlobalID"];
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
  runnableCk?: any | null;
  projectVersion: { __typename?: "ProjectVersion"; id: any; tag?: string | null; name?: string | null };
  session?: { __typename?: "Session"; id: any } | null;
  root?: { __typename?: "Run"; id: any } | null;
  parent?: { __typename?: "Run"; id: any } | null;
  runnable?: { __typename?: "Statement"; id: any; name?: string | null } | null;
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
  metadata?: any | null;
  runnableCk?: any | null;
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
  runnable?: { __typename?: "Statement"; id: any } | null;
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
  runnableId?: any | null;
  runnableCk?: any | null;
  runId?: any | null;
  stream: string;
  level?: string | null;
  logger?: string | null;
  message?: string | null;
  metadata?: any | null;
} & { " $fragmentName"?: "LogEntryContentFragment" };

export type CurrentRunsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
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
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
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

export type SearchRunsQueryVariables = Exact<{
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]> | Scalars["UUID"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  rootOnly: Scalars["Boolean"];
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
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
  id: Scalars["GlobalID"];
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
  projectId: Scalars["GlobalID"];
  projectVersionId?: InputMaybe<Scalars["GlobalID"]>;
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]> | Scalars["UUID"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
  query?: InputMaybe<SearchQuery>;
  sort?: InputMaybe<Array<SearchSort> | SearchSort>;
  after?: InputMaybe<Scalars["String"]>;
  limit?: InputMaybe<Scalars["Int"]>;
  count?: InputMaybe<Scalars["Boolean"]>;
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
  projectId: Scalars["GlobalID"];
  projectVersionId: Scalars["GlobalID"];
  runnableIds?: InputMaybe<Array<Scalars["GlobalID"]> | Scalars["GlobalID"]>;
  runnableCks?: InputMaybe<Array<Scalars["UUID"]> | Scalars["UUID"]>;
  sessionId?: InputMaybe<Scalars["GlobalID"]>;
  runId?: InputMaybe<Scalars["GlobalID"]>;
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
  projectId: Scalars["GlobalID"];
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
          { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
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
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
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
          { kind: "Field", name: { kind: "Name", value: "tag" } },
          { kind: "Field", name: { kind: "Name", value: "flags" } },
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
            name: { kind: "Name", value: "runnable" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
              ],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
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
          { kind: "Field", name: { kind: "Name", value: "metadata" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "runnable" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
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
          { kind: "Field", name: { kind: "Name", value: "runnableId" } },
          { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
          { kind: "Field", name: { kind: "Name", value: "runId" } },
          { kind: "Field", name: { kind: "Name", value: "stream" } },
          { kind: "Field", name: { kind: "Name", value: "level" } },
          { kind: "Field", name: { kind: "Name", value: "logger" } },
          { kind: "Field", name: { kind: "Name", value: "message" } },
          { kind: "Field", name: { kind: "Name", value: "metadata" } },
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
    ...FileHeaderFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...TriggerContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...TriggerContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
    ...InterpFileFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...InterpStatementFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...TriggerContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
    ...FileHeaderFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...TriggerContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
            type: { kind: "NamedType", name: { kind: "Name", value: "ProjectAccessLevel" } },
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableId" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } },
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
              ],
            },
          },
        ],
      },
    },
    ...RunContentFragmentDoc.definitions,
    ...LogEntryContentFragmentDoc.definitions,
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
                      { kind: "Field", name: { kind: "Name", value: "success" } },
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
    ...OperationInfoContentFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "tag" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "TypeTag" } },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "flags" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
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
                      name: { kind: "Name", value: "tag" },
                      value: { kind: "Variable", name: { kind: "Name", value: "tag" } },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "flags" },
                      value: { kind: "Variable", name: { kind: "Name", value: "flags" } },
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
                      { kind: "Field", name: { kind: "Name", value: "tag" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      { kind: "Field", name: { kind: "Name", value: "key" } },
                      { kind: "Field", name: { kind: "Name", value: "headingLevel" } },
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
    ...StatementContentFragmentDoc.definitions,
    ...TaggingContentFragmentDoc.definitions,
    ...FieldContentFragmentDoc.definitions,
    ...TriggerContentFragmentDoc.definitions,
    ...IssueContentFragmentDoc.definitions,
    ...ResolvedFieldContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
                      { kind: "Field", name: { kind: "Name", value: "ck" } },
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
                      { kind: "Field", name: { kind: "Name", value: "text" } },
                      { kind: "Field", name: { kind: "Name", value: "flags" } },
                      { kind: "Field", name: { kind: "Name", value: "referenceCk" } },
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableCk" } },
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
                      name: { kind: "Name", value: "runnableCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "runnableCk" } },
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
                      { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableCk" } },
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
                      name: { kind: "Name", value: "runnableCk" },
                      value: { kind: "Variable", name: { kind: "Name", value: "runnableCk" } },
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
                      { kind: "Field", name: { kind: "Name", value: "runnableCk" } },
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
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...ProjectHeaderFragmentDoc.definitions,
    ...ProjectVersionHeaderFragmentDoc.definitions,
    ...OperationInfoContentFragmentDoc.definitions,
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
    ...OperationInfoContentFragmentDoc.definitions,
    ...RunContentFragmentDoc.definitions,
    ...WorkerSetContentFragmentDoc.definitions,
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
    ...RunContentFragmentDoc.definitions,
    ...WorkerSetContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<SessionsChangedSubscription, SessionsChangedSubscriptionVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
                name: { kind: "Name", value: "runnableIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runnableCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
    ...RunContentFragmentDoc.definitions,
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
    ...RunContentFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
                name: { kind: "Name", value: "runnableIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runnableCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
    ...LogEntryContentFragmentDoc.definitions,
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
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
          type: {
            kind: "ListType",
            type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "GlobalID" } } },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
                name: { kind: "Name", value: "runnableIds" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableIds" } },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "runnableCks" },
                value: { kind: "Variable", name: { kind: "Name", value: "runnableCks" } },
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
    ...LogEntryContentFragmentDoc.definitions,
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
