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

export type Dataset = Node & {
  __typename?: "Dataset";
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  length: Scalars["Int"];
  name: Scalars["String"];
  records: Array<DatasetRecord>;
  schema?: Maybe<Scalars["JSON"]>;
};

export type DatasetRecord = Node & {
  __typename?: "DatasetRecord";
  data: Scalars["JSON"];
  id: Scalars["GlobalID"];
  index: Scalars["Int"];
};

export type Expectation = Node & {
  __typename?: "Expectation";
  description: Scalars["String"];
  examplesDatasets: Array<Dataset>;
  id: Scalars["GlobalID"];
  index: Scalars["Int"];
  instructions: Array<Instruction>;
  task: Task;
};

export type Instruction = Node & {
  __typename?: "Instruction";
  builtinId?: Maybe<Scalars["String"]>;
  children: Array<Instruction>;
  code?: Maybe<Scalars["String"]>;
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  index: Scalars["Int"];
  name: Scalars["String"];
  parent?: Maybe<Instruction>;
  scope: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

export type Model = Node & {
  __typename?: "Model";
  baseline?: Maybe<Model>;
  createdAt: Scalars["DateTime"];
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  provider: Scalars["String"];
  updatedAt: Scalars["DateTime"];
};

export type Mutation = {
  __typename?: "Mutation";
  compileTask?: Maybe<Scalars["Void"]>;
  runProgram?: Maybe<Scalars["Void"]>;
};

export type MutationCompileTaskArgs = {
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

export type ProjectFile = Node & {
  __typename?: "ProjectFile";
  content: TaskInstructionDatasetModel;
  id: Scalars["GlobalID"];
  name: Scalars["String"];
  nameDotType: Scalars["String"];
  projectVersion: ProjectVersion;
  type: Scalars["String"];
};

export type ProjectVersion = Node & {
  __typename?: "ProjectVersion";
  committedAt?: Maybe<Scalars["DateTime"]>;
  createdAt: Scalars["DateTime"];
  description?: Maybe<Scalars["String"]>;
  files: Array<ProjectFile>;
  id: Scalars["GlobalID"];
  name?: Maybe<Scalars["String"]>;
  parents: Array<ProjectVersion>;
  program?: Maybe<Task>;
  project: Project;
};

export type Query = {
  __typename?: "Query";
  organization?: Maybe<Organization>;
  organizationBySlug?: Maybe<Organization>;
  organizations: OrganizationConnection;
  project?: Maybe<Project>;
  projectBySlug?: Maybe<Project>;
  projectFile?: Maybe<ProjectFile>;
  projectVersion?: Maybe<ProjectVersion>;
  projects: ProjectConnection;
  user?: Maybe<User>;
  users: UserConnection;
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

export type QueryProjectFileArgs = {
  id: Scalars["GlobalID"];
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

export type Task = Node & {
  __typename?: "Task";
  children: Array<Task>;
  createdAt: Scalars["DateTime"];
  expectations: Array<Expectation>;
  id: Scalars["GlobalID"];
  implementations: Array<Instruction>;
  index: Scalars["Int"];
  name: Scalars["String"];
  parent?: Maybe<Task>;
  schema: Scalars["JSON"];
  templateImplementation?: Maybe<Instruction>;
  updatedAt: Scalars["DateTime"];
};

export type TaskInstructionDatasetModel = Dataset | Instruction | Model | Task;

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

export type ProjectVersionFragmentFragment = {
  __typename?: "ProjectVersion";
  name?: string | null;
  description?: string | null;
  createdAt: any;
  committedAt?: any | null;
} & { " $fragmentName"?: "ProjectVersionFragmentFragment" };

export type TaskFragmentFragment = { __typename?: "Task"; name: string; createdAt: any; updatedAt: any } & {
  " $fragmentName"?: "TaskFragmentFragment";
};

export type InstructionFragmentFragment = {
  __typename?: "Instruction";
  name: string;
  createdAt: any;
  updatedAt: any;
  builtinId?: string | null;
  code?: string | null;
  scope: string;
} & { " $fragmentName"?: "InstructionFragmentFragment" };

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
      " $fragmentRefs"?: { ProjectVersionFragmentFragment: ProjectVersionFragmentFragment };
    };
    versions: Array<
      { __typename?: "ProjectVersion"; id: any } & {
        " $fragmentRefs"?: { ProjectVersionFragmentFragment: ProjectVersionFragmentFragment };
      }
    >;
  } | null;
};

export type GetProjectVersionFilesQueryVariables = Exact<{
  id: Scalars["GlobalID"];
}>;

export type GetProjectVersionFilesQuery = {
  __typename?: "Query";
  projectVersion?: {
    __typename?: "ProjectVersion";
    id: any;
    files: Array<{ __typename?: "ProjectFile"; id: any; name: string; type: string; nameDotType: string }>;
    program?: {
      __typename?: "Task";
      id: any;
      name: string;
      schema: any;
      children: Array<
        {
          __typename?: "Task";
          id: any;
          templateImplementation?:
            | ({ __typename?: "Instruction"; id: any } & {
                " $fragmentRefs"?: { InstructionFragmentFragment: InstructionFragmentFragment };
              })
            | null;
          children: Array<
            { __typename?: "Task"; id: any } & { " $fragmentRefs"?: { TaskFragmentFragment: TaskFragmentFragment } }
          >;
        } & { " $fragmentRefs"?: { TaskFragmentFragment: TaskFragmentFragment } }
      >;
    } | null;
  } | null;
};

export const ProjectVersionFragmentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "ProjectVersionFragment" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "ProjectVersion" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "description" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "committedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ProjectVersionFragmentFragment, unknown>;
export const TaskFragmentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "TaskFragment" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Task" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TaskFragmentFragment, unknown>;
export const InstructionFragmentFragmentDoc = {
  kind: "Document",
  definitions: [
    {
      kind: "FragmentDefinition",
      name: { kind: "Name", value: "InstructionFragment" },
      typeCondition: { kind: "NamedType", name: { kind: "Name", value: "Instruction" } },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "name" } },
          { kind: "Field", name: { kind: "Name", value: "createdAt" } },
          { kind: "Field", name: { kind: "Name", value: "updatedAt" } },
          { kind: "Field", name: { kind: "Name", value: "builtinId" } },
          { kind: "Field", name: { kind: "Name", value: "code" } },
          { kind: "Field", name: { kind: "Name", value: "scope" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<InstructionFragmentFragment, unknown>;
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
                      { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionFragment" } },
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
                      { kind: "FragmentSpread", name: { kind: "Name", value: "ProjectVersionFragment" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
    ...ProjectVersionFragmentFragmentDoc.definitions,
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "files" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "type" } },
                      { kind: "Field", name: { kind: "Name", value: "nameDotType" } },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "program" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      { kind: "Field", name: { kind: "Name", value: "name" } },
                      { kind: "Field", name: { kind: "Name", value: "schema" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "children" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            { kind: "Field", name: { kind: "Name", value: "id" } },
                            { kind: "FragmentSpread", name: { kind: "Name", value: "TaskFragment" } },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "templateImplementation" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "InstructionFragment" } },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "children" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  { kind: "Field", name: { kind: "Name", value: "id" } },
                                  { kind: "FragmentSpread", name: { kind: "Name", value: "TaskFragment" } },
                                ],
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
    ...TaskFragmentFragmentDoc.definitions,
    ...InstructionFragmentFragmentDoc.definitions,
  ],
} as unknown as DocumentNode<GetProjectVersionFilesQuery, GetProjectVersionFilesQueryVariables>;
