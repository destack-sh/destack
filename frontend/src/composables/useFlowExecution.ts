import { api } from "@/api";
import { useFlowsStore } from "@/stores";
import {
  getAllConnectedArtifacts,
  isTerminal,
  type Execution,
  type ExecutionArtifactConnection,
  type FlowExecutionPlan,
  type FlowNodeExecutionArgument,
  type FlowRuntimeData,
  type FlowVersion,
  type LimitPaginatedResult,
} from "@/types";
import { DateTime, Duration } from "luxon";
import { onBeforeUnmount, ref, watch, type Ref } from "vue";

export function useFlowExecution(
  flow: Ref<FlowVersion | null>,
  runtimeData: Ref<FlowRuntimeData>,
  poll: Ref<boolean>,
  pollIntervalMillis = 250
) {
  const executions: Ref<Array<Execution>> = ref([]);
  const flowsStore = useFlowsStore();

  // fetch executions whenever the flow instance changes
  watch(
    flow,
    (flow, oldFlow) => {
      if (oldFlow == null || flow == null || flow?.id != oldFlow?.id) {
        if (flow == null) {
          executions.value = [];
        } else {
          fetchExecutions(flow.id);
        }
      }
    },
    { immediate: true }
  );

  function getEagerExecutionConnections(flow: FlowVersion, plan: FlowExecutionPlan, execution: Execution) {
    const argumentsValues = [] as FlowNodeExecutionArgument[];
    Object.values(plan.arguments).forEach((nodeArguments: FlowNodeExecutionArgument[]) =>
      argumentsValues.push(...nodeArguments)
    );

    // TODO @Cleanup: faux/eager execution connections are tightly coupled to execution grid
    const eagerConnections = argumentsValues
      .filter((argument) => argument.records != null)
      .map((argument: FlowNodeExecutionArgument) => {
        // determine name of soon-to-be artifact according to well known backend schema
        const nodeName = flowsStore.getFlowNode(flow, argument.node)?.name;
        let artifact;
        if (argument.name == "*") {
          artifact = `${flow.flow}.${nodeName}.${argument.type}s`;
        } else {
          artifact = `${flow.flow}.${nodeName}-${argument.type}s.${argument.name}`;
        }
        // create pretend execution artifact connection with known data
        return {
          execution: execution.id,
          artifact: `${artifact}@0`,
          connection_type: argument.type,
          connection_name: argument.name,
          view_inline: { start: -argument.records.length, end: 0 },
          dataset_preview: {
            count: argument.records.length,
            limit: 3,
            results: argument.records,
          },
        } as ExecutionArtifactConnection;
      });
    return eagerConnections;
  }

  async function execute() {
    if (flow.value == null) {
      throw new Error("cannot execute: flow is not initialized");
    }

    console.log("execute flow with data", flow.value, runtimeData.value);
    const argumentsInputs: Record<string, FlowNodeExecutionArgument[]> = {};

    for (const nodeName in runtimeData.value.inputs) {
      const input = runtimeData.value.inputs[nodeName];
      const node = flowsStore.getFlowNodeByName(flow.value, nodeName);
      if (node != null) {
        // FlowRuntimeData.inputs supports only single record inputs to main input for now.
        argumentsInputs[node.id] = [
          {
            node: node.id,
            type: "input",
            name: "*",
            records: [input],
          },
        ];
      }
    }
    const plan: FlowExecutionPlan = {
      arguments: { ...argumentsInputs },
      options: {
        blocking: false,
        validate: "lazy",
      },
    };
    await api
      .post<Execution>(`/flows/${flow.value.flow}/versions/${flow.value.version}/execute`, plan)
      .then((response) => response.data)
      .then((execution) => {
        // automatically insert preview datasets for arguments we passed in to give quicker feedback
        if (plan.arguments == null || getAllConnectedArtifacts(execution).length > 0) {
          // bail if there were no arguments or if the backend already manifested them on the execution
          return execution;
        } else {
          execution.connected_artifacts = getEagerExecutionConnections(flow.value, plan, execution);
          return execution;
        }
      })
      .then((execution) => (executions.value = [execution, ...executions.value]));
  }

  function fetchExecutions(flow: string, limit = 10) {
    api
      .get<LimitPaginatedResult<Execution>>(`/executions`, { params: { type: "flow", flow, limit } })
      .then((result) => result.data)
      .then((result) => (executions.value = result.results));
  }

  const pollExecutionsInterval = setInterval(pollUnterminatedExecutions, pollIntervalMillis);
  onBeforeUnmount(() => clearInterval(pollExecutionsInterval));

  async function pollUnterminatedExecutions() {
    if (!poll.value) {
      return;
    }

    const pendingExecutions = executions.value.filter((execution) => !isTerminal(execution.state));
    if (pendingExecutions.length == 0) {
      return;
    }

    const updatedExecutions = await api
      .get<LimitPaginatedResult<Execution>>(`/executions`, {
        params: {
          // TODO @Performance: filter for id__in when polling pending executions
          //  Currently not doing this as it messes with django-filters array conversion somehow.
          // id__in: pendingExecutions.map((execution) => execution.id),
          updated_at__gt: DateTime.utc()
            .minus(Duration.fromMillis(pollIntervalMillis * 5))
            .toISO({ includeOffset: false }),
          type: "flow",
          flow: flow.value?.id,
          limit: pendingExecutions.length,
        },
      })
      .then((response) => response.data.results);

    // replace current executions with updated ones
    updatedExecutions.forEach((updatedExecution) => {
      const index = executions.value.findIndex((ex) => ex.id == updatedExecution.id);
      executions.value[index] = updatedExecution;
    });
  }

  return { execute, executions };
}
