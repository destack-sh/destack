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
};

export type AddCompilationInput = {
  backends: Array<Scalars["String"]>;
  name: Scalars["String"];
  taskSymbolId: Scalars["GlobalID"];
};

export type AddCompilationPayload = {
  __typename?: "AddCompilationPayload";
  compilation: Compilation;
};

export type Code = Node &
  SymbolContent & {
    __typename?: "Code";
    builtinId?: Maybe<Scalars["String"]>;
    code?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    inputSchema: SchemaElement;
    outputSchema: SchemaElement;
    symbol: Symbol;
    task?: Maybe<Task>;
  };

export type CodeTaskArgs = {
  pk?: InputMaybe<Scalars["ID"]>;
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

export type Compilation = Node & {
  __typename?: "Compilation";
  backends: Array<Model>;
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  mappings: Array<SourceMapping>;
  name: Scalars["String"];
  targetCode?: Maybe<Code>;
  targetTask?: Maybe<Task>;
  task: Task;
  updatedAt: Scalars["DateTime"];
};

export type CompileInput = {
  compilationId: Scalars["GlobalID"];
};

export type CompilePayload = {
  __typename?: "CompilePayload";
  compilation: Compilation;
};

export type CreateFilePayload = File | OperationInfo;

export type CreateSymbolPayload = OperationInfo | Symbol;

export type Dataset = Node &
  SymbolContent & {
    __typename?: "Dataset";
    id: Scalars["GlobalID"];
    length: Scalars["Int"];
    records: Array<DatasetRecord>;
    schema: SchemaElement;
    symbol: Symbol;
  };

export type DatasetRecord = Node & {
  __typename?: "DatasetRecord";
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  index: Scalars["Int"];
};

export type DatasetView = Node &
  SymbolContent & {
    __typename?: "DatasetView";
    dataset: Dataset;
    id: Scalars["GlobalID"];
    symbol: Symbol;
  };

export type DeleteFilePayload = File | OperationInfo;

export type Execution = Node & {
  __typename?: "Execution";
  children: Array<Execution>;
  code: Code;
  createdAt: Scalars["DateTime"];
  durationMillis?: Maybe<Scalars["Float"]>;
  error?: Maybe<Scalars["JSON"]>;
  id: Scalars["GlobalID"];
  inputs?: Maybe<Scalars["JSON"]>;
  model?: Maybe<Model>;
  outputs?: Maybe<Scalars["JSON"]>;
  parent?: Maybe<Execution>;
  /** Time of transition to RUNNING status. */
  startedAt?: Maybe<Scalars["DateTime"]>;
  status: Scalars["String"];
  /** Time of transition to a terminal status. */
  terminatedAt?: Maybe<Scalars["DateTime"]>;
  updatedAt: Scalars["DateTime"];
};

export type Expectation = Node &
  SymbolContent & {
    __typename?: "Expectation";
    description: Scalars["String"];
    id: Scalars["GlobalID"];
    statements: Array<Symbol>;
    symbol: Symbol;
  };

export type File = Node & {
  __typename?: "File";
  createdAt: Scalars["DateTime"];
  files: Array<File>;
  id: Scalars["GlobalID"];
  isFolder: Scalars["Boolean"];
  name: Scalars["String"];
  parent?: Maybe<File>;
  path: Scalars["String"];
  projectVersion: ProjectVersion;
  symbols: Array<Symbol>;
  updatedAt: Scalars["DateTime"];
};

export type FileCreateInput = {
  isFolder?: InputMaybe<Scalars["Boolean"]>;
  name: Scalars["String"];
  parent?: InputMaybe<NodeInput>;
  projectVersion: NodeInput;
};

export type FileDeleteInput = {
  id: Scalars["GlobalID"];
};

export type FileRenameInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
};

export type Model = Node &
  SymbolContent & {
    __typename?: "Model";
    baseline?: Maybe<Model>;
    id: Scalars["GlobalID"];
    provider: Scalars["String"];
    symbol: Symbol;
  };

export type Mutation = {
  __typename?: "Mutation";
  addCompilationTarget: AddCompilationPayload;
  commit: CommitPayload;
  compile: CompilePayload;
  createFile: CreateFilePayload;
  createSymbol: CreateSymbolPayload;
  deleteFile: DeleteFilePayload;
  renameFile: RenameFilePayload;
  renameSymbol: RenameSymbolPayload;
  run: RunCodePayload;
};

export type MutationAddCompilationTargetArgs = {
  input: AddCompilationInput;
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

export type MutationCreateSymbolArgs = {
  input: SymbolCreateInput;
};

export type MutationDeleteFileArgs = {
  input: FileDeleteInput;
};

export type MutationRenameFileArgs = {
  input: FileRenameInput;
};

export type MutationRenameSymbolArgs = {
  input: SymbolRenameInput;
};

export type MutationRunArgs = {
  input: RunCodeInput;
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

/** An edge in a connection. */
export type OrganizationEdge = {
  __typename?: "OrganizationEdge";
  /** A cursor for use in pagination */
  cursor: Scalars["String"];
  /** The item at the end of the edge */
  node: Organization;
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
  slug: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  versions: Array<ProjectVersion>;
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
  compilations: Array<Compilation>;
  createdAt: Scalars["DateTime"];
  description?: Maybe<Scalars["String"]>;
  files: Array<File>;
  id: Scalars["GlobalID"];
  libraries: Array<ProjectVersion>;
  mainProgram?: Maybe<Symbol>;
  name?: Maybe<Scalars["String"]>;
  parents: Array<ProjectVersion>;
  project: Project;
  symbols: Array<Symbol>;
};

export type Query = {
  __typename?: "Query";
  file?: Maybe<File>;
  organization?: Maybe<Organization>;
  organizationBySlug?: Maybe<Organization>;
  organizations: OrganizationConnection;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectVersion?: Maybe<ProjectVersion>;
  projects: ProjectConnection;
  symbol?: Maybe<Symbol>;
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

export type QueryOrganizationsArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
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

export type QuerySymbolArgs = {
  id: Scalars["GlobalID"];
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

export type RenameFilePayload = File | OperationInfo;

export type RenameSymbolPayload = OperationInfo | Symbol;

export type RunCodeInput = {
  arguments: Array<RunCodeValueArgumentInput>;
  codeId: Scalars["GlobalID"];
};

export type RunCodeOutput = {
  __typename?: "RunCodeOutput";
  name: Scalars["String"];
  value: Scalars["String"];
};

export type RunCodePayload = {
  __typename?: "RunCodePayload";
  code: Code;
  execution: Execution;
  outputs?: Maybe<Array<RunCodeOutput>>;
};

export type RunCodeValueArgumentInput = {
  name: Scalars["String"];
  value: Scalars["String"];
};

export type SchemaElement = {
  __typename?: "SchemaElement";
  choices?: Maybe<Array<Scalars["String"]>>;
  elements?: Maybe<Array<SchemaElement>>;
  name: Scalars["String"];
  required: Scalars["Boolean"];
  type: ValueType;
};

export type SourceMapping = Node & {
  __typename?: "SourceMapping";
  compilation: Compilation;
  id: Scalars["GlobalID"];
  source: Symbol;
  sourcePath: Scalars["JSON"];
  target: Symbol;
  targetPath: Scalars["JSON"];
};

export type Symbol = Node & {
  __typename?: "Symbol";
  arguments: Array<SymbolArgument>;
  children: Array<Symbol>;
  content: SymbolContent;
  createdAt: Scalars["DateTime"];
  file: File;
  generated: Scalars["Boolean"];
  id: Scalars["GlobalID"];
  index?: Maybe<Scalars["Int"]>;
  name: Scalars["String"];
  parameters: Array<SymbolParameter>;
  parent?: Maybe<Symbol>;
  projectVersion: ProjectVersion;
  type: SymbolType;
  typeNameDeclaration: Scalars["String"];
  typeShortname: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

export type SymbolContentArgs = {
  pk?: InputMaybe<Scalars["ID"]>;
};

export type SymbolArgument = Node & {
  __typename?: "SymbolArgument";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  reference?: Maybe<Symbol>;
  symbol: Symbol;
  type: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  value?: Maybe<Scalars["JSON"]>;
};

export type SymbolContent = {
  id: Scalars["GlobalID"];
};

export type SymbolCreateInput = {
  file: NodeInput;
  index: Scalars["Int"];
  name: Scalars["String"];
  parent?: InputMaybe<NodeInput>;
  projectVersion: NodeInput;
  type: SymbolType;
};

export type SymbolParameter = Node & {
  __typename?: "SymbolParameter";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  schema?: Maybe<SchemaElement>;
  symbol: Symbol;
  type: SymbolParameterType;
  updatedAt: Scalars["DateTime"];
};

/** An enumeration. */
export enum SymbolParameterType {
  Code = "CODE",
  Data = "DATA",
  Model = "MODEL",
  Value = "VALUE",
}

export type SymbolRenameInput = {
  id: Scalars["GlobalID"];
  name?: InputMaybe<Scalars["String"]>;
};

/** The type of symbol to define in a project. */
export enum SymbolType {
  Code = "CODE",
  Dataset = "DATASET",
  DatasetView = "DATASET_VIEW",
  Expectation = "EXPECTATION",
  Model = "MODEL",
  Task = "TASK",
}

export type Task = Node &
  SymbolContent & {
    __typename?: "Task";
    compilations: Array<Compilation>;
    expectations: Array<Expectation>;
    id: Scalars["GlobalID"];
    inputSchema: SchemaElement;
    outputSchema: SchemaElement;
    symbol: Symbol;
    templateImplementation?: Maybe<Symbol>;
  };

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

/** An enumeration. */
export enum ValueType {
  Array = "ARRAY",
  Boolean = "BOOLEAN",
  Null = "NULL",
  Number = "NUMBER",
  Object = "OBJECT",
  String = "STRING",
}

export type CodeContentFragment = {
  __typename?: "Code";
  id: any;
  builtinId?: string | null;
  code?: string | null;
  inputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  outputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  symbol: {
    __typename?: "Symbol";
    parameters: Array<{
      __typename?: "SymbolParameter";
      name: string;
      type: SymbolParameterType;
      schema?:
        | ({ __typename?: "SchemaElement" } & {
            " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
          })
        | null;
    }>;
    arguments: Array<{
      __typename?: "SymbolArgument";
      name: string;
      type: string;
      value?: any | null;
      reference?: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string } | null;
    }>;
  };
} & { " $fragmentName"?: "CodeContentFragment" };

export type DatasetContentFragment = {
  __typename?: "Dataset";
  id: any;
  length: number;
  schema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  records: Array<{ __typename?: "DatasetRecord"; data: any; index: number }>;
} & { " $fragmentName"?: "DatasetContentFragment" };

export type ExpectationContentFragment = {
  __typename?: "Expectation";
  id: any;
  description: string;
  statements: Array<{ __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string }>;
} & { " $fragmentName"?: "ExpectationContentFragment" };

export type FileContentByIdQueryVariables = Exact<{
  fileId: Scalars["GlobalID"];
}>;

export type FileContentByIdQuery = {
  __typename?: "Query";
  file?:
    | ({
        __typename?: "File";
        id: any;
        symbols: Array<
          { __typename?: "Symbol"; id: any } & { " $fragmentRefs"?: { SymbolContentFragment: SymbolContentFragment } }
        >;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type CodeContentToRunFragment = {
  __typename?: "Code";
  inputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  outputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  symbol: {
    __typename?: "Symbol";
    parameters: Array<{
      __typename?: "SymbolParameter";
      name: string;
      type: SymbolParameterType;
      schema?:
        | ({ __typename?: "SchemaElement" } & {
            " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
          })
        | null;
    }>;
    arguments: Array<{
      __typename?: "SymbolArgument";
      name: string;
      type: string;
      value?: any | null;
      reference?: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string } | null;
    }>;
  };
} & { " $fragmentName"?: "CodeContentToRunFragment" };

export type CodeToRunQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type CodeToRunQuery = {
  __typename?: "Query";
  symbol?: {
    __typename?: "Symbol";
    id: any;
    type: SymbolType;
    name: string;
    typeNameDeclaration: string;
    content:
      | ({ __typename?: "Code"; id: any } & {
          " $fragmentRefs"?: { CodeContentToRunFragment: CodeContentToRunFragment };
        })
      | { __typename?: "Dataset"; id: any }
      | { __typename?: "DatasetView"; id: any }
      | { __typename?: "Expectation"; id: any }
      | { __typename?: "Model"; id: any }
      | { __typename?: "Task"; id: any };
  } | null;
};

export type ExecutionHeaderFragment = {
  __typename?: "Execution";
  id: any;
  status: string;
  createdAt: any;
  startedAt?: any | null;
  terminatedAt?: any | null;
  durationMillis?: number | null;
  code: {
    __typename?: "Code";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  };
  model?: {
    __typename?: "Model";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  } | null;
} & { " $fragmentName"?: "ExecutionHeaderFragment" };

export type RunMutationVariables = Exact<{
  input: RunCodeInput;
}>;

export type RunMutation = {
  __typename?: "Mutation";
  run: {
    __typename?: "RunCodePayload";
    execution: {
      __typename?: "Execution";
      id: any;
      children: Array<
        {
          __typename?: "Execution";
          id: any;
          children: Array<
            { __typename?: "Execution"; id: any } & {
              " $fragmentRefs"?: { ExecutionHeaderFragment: ExecutionHeaderFragment };
            }
          >;
        } & { " $fragmentRefs"?: { ExecutionHeaderFragment: ExecutionHeaderFragment } }
      >;
    } & { " $fragmentRefs"?: { ExecutionHeaderFragment: ExecutionHeaderFragment } };
    outputs?: Array<{ __typename?: "RunCodeOutput"; name: string; value: string }> | null;
  };
};

export type TaskContentFragment = {
  __typename?: "Task";
  id: any;
  inputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  outputSchema: { __typename?: "SchemaElement" } & {
    " $fragmentRefs"?: { SchemaElementContentDeepFragment: SchemaElementContentDeepFragment };
  };
  expectations: Array<{
    __typename?: "Expectation";
    id: any;
    description: string;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
    statements: Array<{ __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string }>;
  }>;
  templateImplementation?: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string } | null;
  compilations: Array<
    { __typename?: "Compilation"; id: any } & {
      " $fragmentRefs"?: { CompilationHeaderFragment: CompilationHeaderFragment };
    }
  >;
} & { " $fragmentName"?: "TaskContentFragment" };

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

export type ProjectVersionContentFragment = {
  __typename?: "ProjectVersion";
  id: any;
  name?: string | null;
  description?: string | null;
  createdAt: any;
  committed: boolean;
  committedAt?: any | null;
  mainProgram?: { __typename?: "Symbol"; id: any; name: string; type: SymbolType } | null;
  files: Array<{ __typename?: "File"; id: any } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } }>;
  compilations: Array<
    { __typename?: "Compilation"; id: any } & {
      " $fragmentRefs"?: { CompilationHeaderFragment: CompilationHeaderFragment };
    }
  >;
} & { " $fragmentName"?: "ProjectVersionContentFragment" };

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
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type ProjectHeaderFragment = {
  __typename?: "Project";
  id: any;
  name: string;
  createdAt: any;
  updatedAt: any;
  head: { __typename?: "ProjectVersion" } & {
    " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
  };
} & { " $fragmentName"?: "ProjectHeaderFragment" };

export type FileHeaderFragment = {
  __typename?: "File";
  id: any;
  name: string;
  path: string;
  createdAt: any;
  updatedAt: any;
} & { " $fragmentName"?: "FileHeaderFragment" };

export type CompilationHeaderFragment = {
  __typename?: "Compilation";
  id: any;
  name: string;
  createdAt: any;
  updatedAt: any;
  task: {
    __typename?: "Task";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  };
  backends: Array<{
    __typename?: "Model";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  }>;
  targetTask?: {
    __typename?: "Task";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  } | null;
  targetCode?: {
    __typename?: "Code";
    id: any;
    symbol: { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
  } | null;
} & { " $fragmentName"?: "CompilationHeaderFragment" };

export type SchemaElementContentDeepFragment = {
  __typename?: "SchemaElement";
  name: string;
  type: ValueType;
  choices?: Array<string> | null;
  elements?: Array<{
    __typename?: "SchemaElement";
    name: string;
    type: ValueType;
    choices?: Array<string> | null;
  }> | null;
} & { " $fragmentName"?: "SchemaElementContentDeepFragment" };

export type SymbolContentFragment = {
  __typename?: "Symbol";
  id: any;
  name: string;
  type: SymbolType;
  typeShortname: string;
  typeNameDeclaration: string;
  createdAt: any;
  updatedAt: any;
  generated: boolean;
  content:
    | ({ __typename?: "Code" } & { " $fragmentRefs"?: { CodeContentFragment: CodeContentFragment } })
    | ({ __typename?: "Dataset" } & { " $fragmentRefs"?: { DatasetContentFragment: DatasetContentFragment } })
    | { __typename?: "DatasetView" }
    | ({ __typename?: "Expectation" } & {
        " $fragmentRefs"?: { ExpectationContentFragment: ExpectationContentFragment };
      })
    | { __typename?: "Model" }
    | ({ __typename?: "Task" } & { " $fragmentRefs"?: { TaskContentFragment: TaskContentFragment } });
} & { " $fragmentName"?: "SymbolContentFragment" };

export type AddCompilationMutationVariables = Exact<{
  input: AddCompilationInput;
}>;

export type AddCompilationMutation = {
  __typename?: "Mutation";
  addCompilationTarget: {
    __typename?: "AddCompilationPayload";
    compilation: { __typename?: "Compilation"; id: any; name: string; createdAt: any; updatedAt: any };
  };
};

export type CompileMutationVariables = Exact<{
  compilationId: Scalars["GlobalID"];
}>;

export type CompileMutation = {
  __typename?: "Mutation";
  compile: {
    __typename?: "CompilePayload";
    compilation: {
      __typename?: "Compilation";
      id: any;
      name: string;
      createdAt: any;
      updatedAt: any;
      targetTask?:
        | ({ __typename?: "Task" } & { " $fragmentRefs"?: { TaskContentFragment: TaskContentFragment } })
        | null;
      targetCode?:
        | ({ __typename?: "Code" } & { " $fragmentRefs"?: { CodeContentFragment: CodeContentFragment } })
        | null;
    };
  };
};

export type CreateFileMutationVariables = Exact<{
  projectVersionId: Scalars["GlobalID"];
  name: Scalars["String"];
}>;

export type CreateFileMutation = {
  __typename?: "Mutation";
  createFile:
    | { __typename?: "File"; id: any; name: string; path: string }
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
  deleteFile:
    | { __typename?: "File"; id: any }
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      });
};

export type RenameSymbolMutationVariables = Exact<{
  id: Scalars["GlobalID"];
  name: Scalars["String"];
}>;

export type RenameSymbolMutation = {
  __typename?: "Mutation";
  renameSymbol:
    | ({ __typename?: "OperationInfo" } & {
        " $fragmentRefs"?: { OperationInfoContentFragment: OperationInfoContentFragment };
      })
    | { __typename?: "Symbol"; id: any; name: string; typeNameDeclaration: string };
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

export const SchemaElementContentDeepFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "SchemaElementContentDeep" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "SchemaElement" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "choices" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "elements" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
                { kind: "Field", name: { kind: "Name", value: "choices" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SchemaElementContentDeepFragment, unknown>;
export const CodeContentToRunFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "CodeContentToRun" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Code" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "inputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "outputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "symbol" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "parameters" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "schema" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "arguments" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                            { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                          ],
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
} as unknown as DocumentNode<CodeContentToRunFragment, unknown>;
export const ExecutionHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ExecutionHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Execution" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "status" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "startedAt" } },
          { kind: "Field", name: { kind: "Name", value: "terminatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "durationMillis" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "code" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "model" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExecutionHeaderFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "path" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<FileHeaderFragment, unknown>;
export const CompilationHeaderFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "CompilationHeader" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Compilation" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "task" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "backends" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "targetTask" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "targetCode" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CompilationHeaderFragment, unknown>;
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
            name: { kind: "Name", value: "mainProgram" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "files" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "FileHeader" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "compilations" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "CompilationHeader" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectVersionContentFragment, unknown>;
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
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectHeaderFragment, unknown>;
export const CodeContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "CodeContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Code" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "builtinId" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "inputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "outputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "symbol" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "parameters" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "schema" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } },
                          ],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "arguments" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "value" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "reference" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "Field", name: { kind: "Name", value: "name" } },
                            { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                          ],
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
} as unknown as DocumentNode<CodeContentFragment, unknown>;
export const DatasetContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "DatasetContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Dataset" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "schema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          { kind: "Field", name: { kind: "Name", value: "length" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "records" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "data" } },
                { kind: "Field", name: { kind: "Name", value: "index" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<DatasetContentFragment, unknown>;
export const ExpectationContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ExpectationContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Expectation" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "statements" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExpectationContentFragment, unknown>;
export const TaskContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaskContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Task" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "inputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "outputSchema" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "SchemaElementContentDeep" } }],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "expectations" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "description" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "symbol" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "statements" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "templateImplementation" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
              ],
            },
          },
          {
            kind: "Field",
            name: { kind: "Name", value: "compilations" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "CompilationHeader" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TaskContentFragment, unknown>;
export const SymbolContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "SymbolContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Symbol" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "type" } },
          { kind: "Field", name: { kind: "Name", value: "typeShortname" } },
          { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "generated" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "content" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "FragmentSpread", name: { kind: "Name", value: "CodeContent" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "DatasetContent" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "ExpectationContent" } },
                { kind: "FragmentSpread", name: { kind: "Name", value: "TaskContent" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SymbolContentFragment, unknown>;
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
                  name: { kind: "Name", value: "symbols" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "SymbolContent" } },
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
    ...SymbolContentFragmentDoc.definitions,
    ...CodeContentFragmentDoc.definitions,
    ...SchemaElementContentDeepFragmentDoc.definitions,
    ...DatasetContentFragmentDoc.definitions,
    ...ExpectationContentFragmentDoc.definitions,
    ...TaskContentFragmentDoc.definitions,
    ...CompilationHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<FileContentByIdQuery, FileContentByIdQueryVariables>;
export const CodeToRunDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "codeToRun" },
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
            name: { kind: "Name", value: "symbol" },
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
                { kind: "Field", name: { kind: "Name", value: "type" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "content" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "InlineFragment",
                        typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Code" } },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "CodeContentToRun" } }],
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
    ...CodeContentToRunFragmentDoc.definitions,
    ...SchemaElementContentDeepFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CodeToRunQuery, CodeToRunQueryVariables>;
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
          variable: { kind: "Variable", name: { kind: "Name", value: "input" } },
          type: { kind: "NonNullType", type: { kind: "NamedType", name: { kind: "Name", value: "RunCodeInput" } } },
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
                value: { kind: "Variable", name: { kind: "Name", value: "input" } },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "execution" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionHeader" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "children" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionHeader" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "children" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "ExecutionHeader" } },
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
                  name: { kind: "Name", value: "outputs" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "name" } },
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
    ...ExecutionHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<RunMutation, RunMutationVariables>;
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
    ...CompilationHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<ProjectVersionContentQuery, ProjectVersionContentQueryVariables>;
export const AddCompilationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "addCompilation" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "input" } },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "AddCompilationInput" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "addCompilationTarget" },
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
                  kind: "Field",
                  name: { kind: "Name", value: "compilation" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AddCompilationMutation, AddCompilationMutationVariables>;
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
                  kind: "Field",
                  name: { kind: "Name", value: "compilation" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "targetTask" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "TaskContent" } }],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "targetCode" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [{ kind: "FragmentSpread", name: { kind: "Name", value: "CodeContent" } }],
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
    ...TaskContentFragmentDoc.definitions,
    ...SchemaElementContentDeepFragmentDoc.definitions,
    ...CompilationHeaderFragmentDoc.definitions,
    ...CodeContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<CompileMutation, CompileMutationVariables>;
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
                      name: { kind: "Name", value: "projectVersion" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "id" },
                            value: { kind: "Variable", name: { kind: "Name", value: "projectVersionId" } },
                          },
                        ],
                      },
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
                      { kind: "Field", name: { kind: "Name", value: "path" } },
                    ],
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
} as unknown as DocumentNode<DeleteFileMutation, DeleteFileMutationVariables>;
export const RenameSymbolDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "renameSymbol" },
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
            name: { kind: "Name", value: "renameSymbol" },
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
                  typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Symbol" } },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "typeNameDeclaration" } },
                    ],
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
} as unknown as DocumentNode<RenameSymbolMutation, RenameSymbolMutationVariables>;
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
