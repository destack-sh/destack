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

export type Compilation = Node & {
  __typename?: "Compilation";
  backends: Array<SymbolDefinition>;
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  outputInstruction?: Maybe<SymbolDefinition>;
  outputTask?: Maybe<SymbolDefinition>;
  task: Task;
  updatedAt: Scalars["DateTime"];
};

export type Dataset = Node &
  SymbolContent & {
    __typename?: "Dataset";
    id: Scalars["GlobalID"];
    length: Scalars["Int"];
    nameDotType: Scalars["String"];
    records: Array<DatasetRecord>;
    schema?: Maybe<Scalars["JSON"]>;
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
    nameDotType: Scalars["String"];
  };

export type Expectation = Node &
  SymbolContent & {
    __typename?: "Expectation";
    description: Scalars["String"];
    id: Scalars["GlobalID"];
    nameDotType: Scalars["String"];
    statements: Array<SymbolDefinition>;
  };

export type File = Node & {
  __typename?: "File";
  createdAt: Scalars["DateTime"];
  definitions: Array<SymbolDefinition>;
  files: Array<File>;
  id: Scalars["GlobalID"];
  isFolder: Scalars["Boolean"];
  name: Scalars["String"];
  parent?: Maybe<File>;
  projectVersion: ProjectVersion;
  updatedAt: Scalars["DateTime"];
};

export type Instruction = Node &
  SymbolContent & {
    __typename?: "Instruction";
    arguments: Array<InstructionArgument>;
    builtinId?: Maybe<Scalars["String"]>;
    code?: Maybe<Scalars["String"]>;
    id: Scalars["GlobalID"];
    nameDotType: Scalars["String"];
    parameters: Array<InstructionParameter>;
    scope: Scalars["String"];
    task?: Maybe<Task>;
  };

export type InstructionArgument = Node & {
  __typename?: "InstructionArgument";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  instruction: Instruction;
  instructionFree: Instruction;
  name: Scalars["String"];
  reference?: Maybe<SymbolDefinition>;
  type: Scalars["String"];
  updatedAt: Scalars["DateTime"];
  value?: Maybe<Scalars["JSON"]>;
};

export type InstructionParameter = Node & {
  __typename?: "InstructionParameter";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  instruction: Instruction;
  name: Scalars["String"];
  schema?: Maybe<Scalars["JSON"]>;
  type: InstructionParameterType;
  updatedAt: Scalars["DateTime"];
};

/** An enumeration. */
export enum InstructionParameterType {
  Dataset = "DATASET",
  Instruction = "INSTRUCTION",
  Json = "JSON",
  Model = "MODEL",
}

export type Model = Node &
  SymbolContent & {
    __typename?: "Model";
    baseline?: Maybe<Model>;
    id: Scalars["GlobalID"];
    nameDotType: Scalars["String"];
    provider: Scalars["String"];
  };

export type Mutation = {
  __typename?: "Mutation";
  compile?: Maybe<Scalars["Void"]>;
  runProgram?: Maybe<Scalars["Void"]>;
};

export type MutationCompileArgs = {
  projectVersionId: Scalars["UUID"];
  taskId: Scalars["UUID"];
};

export type MutationRunProgramArgs = {
  projectId: Scalars["UUID"];
};

/** An object with a Globally Unique ID */
export type Node = {
  /** The Globally Unique ID of this object */
  id: Scalars["GlobalID"];
};

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
  createdAt: Scalars["DateTime"];
  definitions: Array<SymbolDefinition>;
  description?: Maybe<Scalars["String"]>;
  files: Array<File>;
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  parents: Array<ProjectVersion>;
  program?: Maybe<Task>;
  project: Project;
};

export type ProjectVersionProgramArgs = {
  pk?: InputMaybe<Scalars["ID"]>;
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

export type QueryUserArgs = {
  id: Scalars["GlobalID"];
};

export type QueryUsersArgs = {
  after?: InputMaybe<Scalars["String"]>;
  before?: InputMaybe<Scalars["String"]>;
  first?: InputMaybe<Scalars["Int"]>;
  last?: InputMaybe<Scalars["Int"]>;
};

export type SymbolContent = {
  id: Scalars["GlobalID"];
  nameDotType: Scalars["String"];
};

export type SymbolDefinition = Node & {
  __typename?: "SymbolDefinition";
  children: Array<SymbolDefinition>;
  committed: Scalars["Boolean"];
  committedIn?: Maybe<ProjectVersion>;
  content: SymbolContent;
  createdAt: Scalars["DateTime"];
  file: File;
  id: Scalars["GlobalID"];
  index?: Maybe<Scalars["Int"]>;
  name: Scalars["String"];
  nameDotType: Scalars["String"];
  parent?: Maybe<SymbolDefinition>;
  projectVersion: ProjectVersion;
  type: SymbolType;
  updatedAt: Scalars["DateTime"];
};

export type SymbolDefinitionContentArgs = {
  pk?: InputMaybe<Scalars["ID"]>;
};

/** The type of symbol to define in a project. */
export enum SymbolType {
  Dataset = "DATASET",
  DatasetView = "DATASET_VIEW",
  Expectation = "EXPECTATION",
  Instruction = "INSTRUCTION",
  Model = "MODEL",
  Task = "TASK",
}

export type Task = Node &
  SymbolContent & {
    __typename?: "Task";
    compilations: Array<Compilation>;
    expectations: Array<Expectation>;
    id: Scalars["GlobalID"];
    nameDotType: Scalars["String"];
    schema: Scalars["JSON"];
    templateImplementation?: Maybe<SymbolDefinition>;
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

export type DatasetContentFragment = {
  __typename?: "Dataset";
  id: any;
  records: Array<{ __typename?: "DatasetRecord"; data: any; index: number }>;
} & { " $fragmentName"?: "DatasetContentFragment" };

export type ExpectationContentFragment = {
  __typename?: "Expectation";
  id: any;
  description: string;
  statements: Array<{ __typename?: "SymbolDefinition"; id: any; nameDotType: string }>;
} & { " $fragmentName"?: "ExpectationContentFragment" };

export type FileHeaderFragment = { __typename?: "File"; id: any; name: string; createdAt: any; updatedAt: any } & {
  " $fragmentName"?: "FileHeaderFragment";
};

export type GetFileByIdQueryVariables = Exact<{
  fileId: Scalars["GlobalID"];
}>;

export type GetFileByIdQuery = {
  __typename?: "Query";
  file?:
    | ({
        __typename?: "File";
        id: any;
        definitions: Array<{
          __typename?: "SymbolDefinition";
          id: any;
          name: string;
          type: SymbolType;
          nameDotType: string;
          createdAt: any;
          updatedAt: any;
          content:
            | ({ __typename?: "Dataset" } & { " $fragmentRefs"?: { DatasetContentFragment: DatasetContentFragment } })
            | { __typename?: "DatasetView" }
            | ({ __typename?: "Expectation" } & {
                " $fragmentRefs"?: { ExpectationContentFragment: ExpectationContentFragment };
              })
            | ({ __typename?: "Instruction" } & {
                " $fragmentRefs"?: { InstructionContentFragment: InstructionContentFragment };
              })
            | { __typename?: "Model" }
            | ({ __typename?: "Task" } & { " $fragmentRefs"?: { TaskContentFragment: TaskContentFragment } });
        }>;
      } & { " $fragmentRefs"?: { FileHeaderFragment: FileHeaderFragment } })
    | null;
};

export type InstructionContentFragment = {
  __typename?: "Instruction";
  id: any;
  builtinId?: string | null;
  code?: string | null;
  parameters: Array<{
    __typename?: "InstructionParameter";
    name: string;
    type: InstructionParameterType;
    schema?: any | null;
  }>;
  arguments: Array<{
    __typename?: "InstructionArgument";
    name: string;
    type: string;
    value?: any | null;
    reference?: { __typename?: "SymbolDefinition"; id: any; nameDotType: string } | null;
  }>;
} & { " $fragmentName"?: "InstructionContentFragment" };

export type TaskContentFragment = {
  __typename?: "Task";
  id: any;
  schema: any;
  expectations: Array<{ __typename?: "Expectation"; id: any; nameDotType: string }>;
  templateImplementation?: { __typename?: "SymbolDefinition"; id: any; nameDotType: string } | null;
} & { " $fragmentName"?: "TaskContentFragment" };

export type ProjectVersionHeaderFragment = {
  __typename?: "ProjectVersion";
  name?: string | null;
  description?: string | null;
  createdAt: any;
  committed: boolean;
  committedAt?: any | null;
} & { " $fragmentName"?: "ProjectVersionHeaderFragment" };

export type GetProjectBySlugQueryVariables = Exact<{
  organization: Scalars["String"];
  project: Scalars["String"];
}>;

export type GetProjectBySlugQuery = {
  __typename?: "Query";
  projectBySlug?: { __typename?: "Project"; id: any } | null;
};

export type GetProjectVersionsQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type GetProjectVersionsQuery = {
  __typename?: "Query";
  project?: {
    __typename?: "Project";
    id: any;
    name: string;
    slug: string;
    head: { __typename?: "ProjectVersion"; id: any } & {
      " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
    };
    versions: Array<
      { __typename?: "ProjectVersion"; id: any } & {
        " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment };
      }
    >;
  } | null;
};

export type GetProjectVersionFilesQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type GetProjectVersionFilesQuery = {
  __typename?: "Query";
  projectVersion?:
    | ({
        __typename?: "ProjectVersion";
        id: any;
        files: Array<{ __typename?: "File"; id: any; name: string; createdAt: any; updatedAt: any }>;
      } & { " $fragmentRefs"?: { ProjectVersionHeaderFragment: ProjectVersionHeaderFragment } })
    | null;
};

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
                { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ExpectationContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<FileHeaderFragment, unknown>;
export const InstructionContentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InstructionContent" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Instruction" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "id" } },
          { kind: "Field", name: { kind: "Name", value: "builtinId" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "parameters" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "type" } },
                { kind: "Field", name: { kind: "Name", value: "schema" } },
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
                      { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InstructionContentFragment, unknown>;
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
          { kind: "Field", name: { kind: "Name", value: "schema" } },
          {
            kind: "Field",
            name: { kind: "Name", value: "expectations" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
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
                { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TaskContentFragment, unknown>;
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
export const GetFileByIdDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "getFileById" },
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
                  name: { kind: "Name", value: "definitions" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
                      { kind: "Field", name: { kind: "Name", value: "createdAt" } },
                      { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "content" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "FragmentSpread", name: { kind: "Name", value: "InstructionContent" } },
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
            },
          },
        ],
      },
    },
    ...FileHeaderFragmentDoc.definitions,
    ...InstructionContentFragmentDoc.definitions,
    ...DatasetContentFragmentDoc.definitions,
    ...ExpectationContentFragmentDoc.definitions,
    ...TaskContentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<GetFileByIdQuery, GetFileByIdQueryVariables>;
export const GetProjectBySlugDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "getProjectBySlug" },
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
              selections: [{ kind: "Field", name: { kind: "Name", value: "id" } }],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<GetProjectBySlugQuery, GetProjectBySlugQueryVariables>;
export const GetProjectVersionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "getProjectVersions" },
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
            name: { kind: "Name", value: "project" },
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
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "slug" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "head" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "versions" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
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
    ...ProjectVersionHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<GetProjectVersionsQuery, GetProjectVersionsQueryVariables>;
export const GetProjectVersionFilesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "getProjectVersionFiles" },
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
                { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionHeader" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "files" },
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
    ...ProjectVersionHeaderFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<GetProjectVersionFilesQuery, GetProjectVersionFilesQueryVariables>;
