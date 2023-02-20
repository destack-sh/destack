import { graphql, useFragment } from "@/gql";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Ref } from "vue";

export const ExecutionContentType = graphql(/* GraphQL */ `
  fragment ExecutionContent on Execution {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    status
    root {
      id
    }
    parent {
      id
    }
    inputs
    outputs
    error
    code {
      id
    }
    model {
      id
    }
  }
`);

export function useExecutions(projectVersionId: Ref<string>, codeId: Ref<string | null>, live?: boolean) {
  const { result: executionsResult, subscribeToMore } = useQuery(
    graphql(/* GraphQL */ `
      query executions($projectVersionId: GlobalID!, $codeId: GlobalID, $first: Int, $last: Int) {
        executions(projectVersionId: $projectVersionId, codeId: $codeId, first: $first, last: $last) {
          totalCount
          edges {
            cursor
            node {
              ...ExecutionContent
              descendants {
                ...ExecutionContent
              }
            }
          }
          pageInfo {
            hasNextPage
            hasPreviousPage
            startCursor
            endCursor
          }
        }
      }
    `),
    { projectVersionId, codeId, first: 25 }
  );

  if (live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription moduleExecutionChanged($projectVersionId: GlobalID!) {
          moduleExecutionChanged(projectVersionId: $projectVersionId) {
            ...ExecutionContent
          }
        }
      `),
      variables: { projectVersionId },
      updateQuery: (prev, { subscriptionData }) => {
        const newExecution = subscriptionData.data.moduleExecutionChanged;
        const newExecutions = prev.executions.edges.map((edge: any) => edge.node);
        // insert or update the execution
        const index = newExecutions.findIndex((execution: any) => execution.id === newExecution.id);
        if (index === -1) {
          newExecutions.push(newExecution);
        } else {
          newExecutions[index] = newExecution;
        }
        return {
          executions: {
            ...prev.executions,
            totalCount: prev.executions.totalCount ?? 0 + (index === -1 ? 1 : 0),
            edges: newExecutions.map((execution: any) => ({
              cursor: execution.id,
              node: { ...execution, descendants: [] },
            })),
          },
        };
      },
    });
  }

  return {
    executions: computed(
      () =>
        executionsResult.value?.executions?.edges?.map((edge: any) => useFragment(ExecutionContentType, edge.node)) ??
        []
    ),
  };
}
