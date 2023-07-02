import { graphql } from "@/gql";
import {
  ModuleMutationType,
  type CreateFileMutation,
  type DeleteFileMutation,
  type RenameFileMutation,
  type RestoreFileMutation,
  type SoftDeleteFileMutation,
} from "@/gql/graphql";
import { useOperationsStore, type Transaction } from "@/state/operations";
import { OpRegistry } from "@/state/sync";
import { useMutation } from "@vue/apollo-composable";
import { v4 as uuidv4 } from "uuid";

export function newFileId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`File:${nodeId}`);
}

export function useFileOps() {
  const ops = useOperationsStore();
  const registry = new OpRegistry();

  // for the annoying redundancy see :BE-114

  // TODO @Broken @UX: optimistic create file & paste file does not work optimistically (causes reload)
  const { mutate: createFileMut } = registry.useMutation(
    ModuleMutationType.CreateFile,
    graphql(/* GraphQL */ `
      mutation createFile(
        $id: GlobalID
        $projectVersionId: GlobalID!
        $name: String!
        $directory: Boolean
        $parentId: GlobalID
      ) {
        createFile(
          input: {
            id: $id
            projectVersionId: $projectVersionId
            parentId: $parentId
            name: $name
            directory: $directory
          }
        ) {
          ... on File {
            # :fileContentById
            id
            projectVersion {
              id
            }
            ...FileHeader
            # :InterpFile :InterpStatement
            statements(filters: { isVisible: true }) {
              ...StatementContent
              issues(filters: { scope: STATEMENT }) {
                ...IssueContent
              }
            }
            issues(filters: { scope: FILE }) {
              ...IssueContent
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: {
        id: string;
        projectVersionId: string;
        parentId: string | null;
        name: string;
        directory: boolean;
      }) =>
        ({
          __typename: "Mutation",
          createFile: {
            __typename: "File",
            projectVersion: {
              __typename: "ProjectVersion",
              id: vars.projectVersionId,
            },
            parent:
              vars.parentId == null || atob(vars.parentId).startsWith("ProjectVersion")
                ? { __typename: "ProjectVersion", id: vars.projectVersionId }
                : { __typename: "File", id: vars.parentId },
            id: vars.id,
            name: vars.name,
            revision: -1,
            directory: vars.directory,
            statements: [],
            issues: [],
            // crud
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            deletedAt: null,
            createdBy: null,
            lastEditedAt: new Date().toISOString(),
            lastEditedBy: null,
          },
        } as CreateFileMutation),
      update(cache, { data: data }) {
        if (data?.createFile.__typename != "File") {
          return; // error;
        }
        // extend ProjectVersion.files array with (ref to) new file
        // must ensure that all relevant fields are present or weird things happen
        cache.modify({
          id: cache.identify(data.createFile?.projectVersion),
          fields: {
            files(currentFiles = { edges: [] }) {
              const newRef = cache.identify(data?.createFile);
              return {
                edges: [...currentFiles.edges.filter((e: any) => e.node.__ref != newRef), { node: { __ref: newRef } }],
              };
            },
          },
          optimistic: true,
        });
      },
    }
  );

  const { mutate: deleteFileMut } = registry.useMutation(
    ModuleMutationType.DeleteFile,
    graphql(/* GraphQL */ `
      mutation deleteFile($id: GlobalID!) {
        deleteFile(input: { id: $id }) {
          ... on File {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          deleteFile: {
            __typename: "File",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as DeleteFileMutation),
    }
  );

  const { mutate: softDeleteFileMut } = registry.useMutation(
    ModuleMutationType.SoftDeleteFile,
    graphql(/* GraphQL */ `
      mutation softDeleteFile($id: GlobalID!) {
        softDeleteFile(input: { id: $id }) {
          ... on File {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          softDeleteFile: {
            __typename: "File",
            id: vars.id,
            deletedAt: new Date().toISOString(),
          },
        } as SoftDeleteFileMutation),
    }
  );

  const { mutate: restoreFileMut } = registry.useMutation(
    ModuleMutationType.RestoreFile,
    graphql(/* GraphQL */ `
      mutation restoreFile($id: GlobalID!) {
        restoreFile(input: { id: $id }) {
          ... on File {
            id
            deletedAt
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string }) =>
        ({
          __typename: "Mutation",
          restoreFile: { __typename: "File", id: vars.id, deletedAt: null },
        } as RestoreFileMutation),
    }
  );

  async function delete_(tx: Transaction | null, id: string) {
    return await ops.perform({
      tx,
      type: "file.delete",
      do: async () => {
        return await deleteFileMut({ id: id });
      },
    });
  }

  async function softDelete(tx: Transaction | null, id: string) {
    return await ops.perform({
      tx,
      type: "file.softDelete",
      do: async () => {
        return await softDeleteFileMut({ id: id });
      },
      undo: async () => {
        return await restoreFileMut({ id: id });
      },
    });
  }

  async function restore(tx: Transaction | null, id: string) {
    return await ops.perform({
      tx,
      type: "file.restore",
      do: async () => {
        return await restoreFileMut({ id: id });
      },
      undo: async () => {
        return await softDeleteFileMut({ id: id });
      },
    });
  }

  async function create(
    tx: Transaction | null,
    id: string,
    projectVersionId: string,
    name: string,
    parentId: string | null,
    directory?: boolean
  ) {
    return await ops.perform({
      tx,
      type: "file.create",
      do: async () => {
        return await createFileMut({ id, projectVersionId, name, parentId, directory: directory ?? false });
      },
      undo: async () => {
        return await softDeleteFileMut({ id });
      },
      redo: async () => {
        return await restoreFileMut({ id });
      },
    });
  }

  const { mutate: renameFileMut } = registry.useMutation(
    ModuleMutationType.RenameFile,
    graphql(/* GraphQL */ `
      mutation renameFile($id: GlobalID!, $name: String!) {
        renameFile(input: { id: $id, name: $name }) {
          ... on File {
            id
            name
            revision
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      optimisticResponse: (vars: { id: string; name: string }) =>
        ({
          __typename: "Mutation",
          renameFile: {
            __typename: "File",
            id: vars.id,
            name: vars.name,
            revision: -1,
          },
        } as RenameFileMutation),
    }
  );

  async function rename(tx: Transaction | null, id: string, oldName: string, newName: string) {
    return await ops.perform({
      tx,
      type: "file.rename",
      do: async () => {
        return await renameFileMut({ id: id, name: newName });
      },
      undo: async () => {
        return await renameFileMut({ id: id, name: oldName });
      },
    });
  }

  const { mutate: pasteFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation pasteFile($sourceId: GlobalID!, $targetId: GlobalID, $targetVersionId: GlobalID!, $parentId: GlobalID) {
        pasteFile(
          input: { sourceId: $sourceId, targetId: $targetId, targetVersionId: $targetVersionId, parentId: $parentId }
        ) {
          ... on File {
            # :fileContentById
            id
            projectVersion {
              id
            }
            ...FileHeader
            # :InterpFile :InterpStatement
            issues {
              ...IssueContent
            }
            statements(filters: { isVisible: true }) {
              ...StatementContent
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    {
      update(cache, { data }) {
        if (data?.pasteFile.__typename != "File") {
          return; // error;
        }
        // extend ProjectVersion.files array with (ref to) new file
        cache.modify({
          id: cache.identify(data.pasteFile?.projectVersion),
          fields: {
            files(currentFiles = { edges: [] }) {
              const newRef = cache.identify(data?.pasteFile);
              return {
                edges: [...currentFiles.edges.filter((e: any) => e.node.__ref != newRef), { node: { __ref: newRef } }],
              };
            },
          },
          optimistic: true,
        });
      },
    }
  );

  async function paste(
    tx: Transaction | null,
    sourceId: string,
    targetId: string,
    targetVersionId: string,
    parentId: string | null
  ) {
    return await ops.perform({
      tx,
      type: "file.paste",
      do: async () => {
        return await pasteFileMut({
          sourceId: sourceId,
          targetId: targetId,
          targetVersionId: targetVersionId,
          parentId: parentId,
        });
      },
      undo: async () => {
        return await softDeleteFileMut({ id: sourceId });
      },
      redo: async () => {
        return await restoreFileMut({ id: sourceId });
      },
    });
  }

  return { registry, create, rename, delete: delete_, softDelete, restore, paste };
}
