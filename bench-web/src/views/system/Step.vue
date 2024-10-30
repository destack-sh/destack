<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { OUTGOING_STEP_TYPES, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, FIELD_CONTEXT_ACTIONS, makeTypeInfo, NAME_TYPE, type TypeIdentity } from "@/language/field";
import {
  FLOW_GRID_STEP,
  FLOW_PORT_SIZE,
  STEP_CONTEXT_ACTIONS,
  STEP_HEADER_HEIGHT,
  useFlowContext,
  type Port,
} from "@/language/flow";
import { cloneNode, moveNode, onNodeMorphed, unpackSubnodeProperty } from "@/language/node";
import { makeEdit } from "@/language/transaction";
import {
  BenchType,
  ColorShade,
  FieldData,
  NodeType,
  Orientation,
  PortSide,
  RunStatus,
  StepType,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { describeNode, isNode, toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { runtime } from "@/system/runtime";
import { canvas } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import {
  startDragging,
  useDropZone,
  useMultiDropZone,
  useSingleDropZone,
  type DraggedContent,
  type MultiAnchor,
} from "@/ui/drag";
import { getNodeIcon, ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
import { menuActionsLike, pushPopover, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { COLOR_BY_RUN_STATUS, getColorHex, getRunColorHex } from "@/ui/style";
import type { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import { formatDuration, getDurationFromNow, TimeUpdateInterval } from "@/utils/time";
import { viewEmits, type ViewExposed } from "@/views/common";
import Code from "@/views/content/Code.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import Text from "@/views/content/Text.vue";
import Field from "@/views/system/Field.vue";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const stepPtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.STEP>);
const flowCtx = useFlowContext();
const stepState = flowCtx.stepsStates.value[stepPtr.value.id!]; // must exist
const { step, fields, nodePtr, node, nodeFields } = stepState;
const isInspected = computed(() => canvas.isInspected(stepPtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(stepPtr.value));

const nameRef: Ref<InstanceType<typeof NativeInput> | null> = ref(null);
const containerRef: Ref<HTMLElement | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const bodySize = useElementSize(bodyRef, undefined, { box: "border-box" });
const paddingHeight = computed(() => FLOW_GRID_STEP - ((bodySize.height.value + STEP_HEADER_HEIGHT) % FLOW_GRID_STEP));

// run
const lastRuns = computed(() => runtime.focusedRunTree.getLastActiveRuns({ ck: stepPtr.value?.ck }));
const lastRun = computed(() => runtime.focusedRunTree.getLastActiveRun({ ck: stepPtr.value?.ck }));
const lastRunStatusColor = computed(() =>
  lastRun.value?.status != null ? COLOR_BY_RUN_STATUS[lastRun.value.status] : null,
);

//
// Interaction
//

const incomingZoneRef: Ref<HTMLElement | null> = ref(null);
const outgoingZoneRef: Ref<HTMLElement | null> = ref(null);

// actions (some of these actions also only work for field ports)
const getFieldFromContext = (ctx: ActionContext | undefined): { field: FieldData | null } => {
  let field = fields.value.find((f) => f.id == ctx?.triggerNode?.id);
  if (field == null) field = nodeFields.value.find((f) => f.id == ctx?.triggerNode?.id); // related fields
  if (field == null) field = flowCtx.fields.value.find((f) => f.id == ctx?.triggerNode?.id); // related fields
  return { field: field ?? null };
};
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"step"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => focusInElement(nameRef.value!));
    },
  },
  "common.create.above": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(flowCtx.tx, flowCtx.graph, { anchor: "before", target: field });
  },
  "common.create.below": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(flowCtx.tx, flowCtx.graph, { anchor: "after", target: field });
  },
  "common.edit.duplicate": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    const duplicate = cloneNode(flowCtx.tx, flowCtx.graph, field, { includeChildren: true });
  },
  "common.edit.delete": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    flowCtx.tx.delete(field);
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="step"
    ref="containerRef"
    class="group/step rounded border bg-white transition-colors duration-150"
    :class="[
      isInspected
        ? 'border-primary-900'
        : isHighlighted
          ? 'border-primary-400'
          : 'border-gray-200 hover:border-gray-300',
    ]"
    :style="{
      borderColor: lastRunStatusColor != null ? getColorHex(lastRunStatusColor, ColorShade.S600) : undefined,
    }"
    @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step', step: step! })"
  >
    <!-- Header (:StepHeight) -->
    <div
      ref="headerRef"
      class="flex w-full flex-row items-center rounded-t border-b border-gray-200 px-2 transition-colors duration-150 group-hover/step:border-gray-300"
      :style="{
        height: STEP_HEADER_HEIGHT + 'px',
        backgroundColor: lastRunStatusColor != null ? getColorHex(lastRunStatusColor, ColorShade.S50) : undefined,
      }"
    >
      <!-- Icon/Name -->
      <IconInline
        v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
        v-menu="
          (): PopoverInfoIn => ({
            component: Icon,
            placement: 'bottom-right',
            offset: '-referenceWidth',
            props: { modelValue: step!.icon, isInput: true },
            onApply: (newIcon) => flowCtx.tx.update(step!, { icon: newIcon }),
          })
        "
        v-bind="getNodeIcon(step)"
        class="w-5 flex-shrink-0 rounded py-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
      />
      <NativeInput
        id="name"
        ref="nameRef"
        class="ml-1.5 truncate font-medium text-gray-700 transition-colors duration-150"
        is-input
        :value-type="NAME_TYPE"
        :variant="Variant.STEALTH"
        :model-value="step.name"
        @update:model-value="(newValue) => flowCtx.tx.update(step!, { name: newValue as string }, { debounce: 'long' })"
      />
      <!-- Controls/Meta -->
      <div class="ml-auto flex flex-row pl-2 pr-0.5">
        <!-- Status -->
        <!-- NOTE :UX: indicate Step/Flow Run statuses better (show all Runs on hover, total runtime, ...) -->
        <Transition
          enter-active-class="transition-opacity duration-75"
          enter-from-class="opacity-0"
          enter-to-class="opacity-100"
          mode="out-in"
          leave-active-class="transition-opacity duration-75"
          leave-from-class="opacity-100"
          leave-to-class="opacity-0"
          appear
        >
          <span
            v-if="lastRun"
            class="flex-shrink-0 truncate px-1"
            :style="{ color: getRunColorHex(lastRun.status, ColorShade.S700) }"
          >
            <span v-if="lastRuns.length > 1"> {{ lastRuns.length }}x </span>
            <!-- Duration -->
            <span v-if="lastRun.startedAt" class="mr-1">
              {{
                formatDuration(
                  lastRun.duration ??
                    getDurationFromNow(lastRun.startedAt, { updateInterval: TimeUpdateInterval.MILLISECOND }),
                  { minUnit: "s" },
                )
              }}
            </span>
            <!-- Icon -->
            <IconInline
              class="w-5 text-center"
              :class="[lastRun.status == RunStatus.RUNNING ? 'animate-spin' : '']"
              :style="{ color: getRunColorHex(lastRun.status) }"
              v-bind="ICON_BY_RUN_STATUS[lastRun.status]"
            />
          </span>
        </Transition>
        <!-- Add field -->
        <button
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
          @click="
            pushPopover({
              trigger: ($event.target as HTMLElement).closest('button')!,
              reference: ($event.target as HTMLElement).closest('button')!,
              info: {
                component: ViewType.PICKER,
                placement: 'bottom-left',
                offset: 'referenceWidth',
                props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
                onApply: (typeInfo: TypeIdentity) => {
                  const stepFields = flowCtx.getStepFields(
                    step!,
                    OUTGOING_STEP_TYPES.includes(step!.type) ? PortSide.OUTGOING : PortSide.INCOMING,
                  );
                  if (stepFields == null) return;
                  createField(flowCtx.tx, flowCtx.graph, {
                    anchor: 'inside',
                    target: stepFields.fieldParent,
                    field: { ...typeInfo, type: stepFields.type },
                  });
                },
              },
            })
          "
        >
          <i class="fas fa-plus w-5 text-center" />
        </button>
        <!-- Menu -->
        <button
          v-menu="
            (): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-left',
              offset: 'referenceWidth',
              items: menuActionsLike(STEP_CONTEXT_ACTIONS, { context: { triggerNode: stepPtr } }),
            })
          "
          class="rounded text-gray-400 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
        >
          <i class="fas fa-ellipsis-v w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div
      ref="bodyRef"
      class="relative"
      :style="{
        // ensure ports are aligned with grid (offset by half a step to connect lines :FlowGrid :StepHeight)
        paddingTop: FLOW_GRID_STEP - (STEP_HEADER_HEIGHT % FLOW_GRID_STEP) - FLOW_GRID_STEP / 2 + 'px',
      }"
    >
      <!-- Content (:StepHeight) -->
      <div
        v-if="[StepType.COMMENT, StepType.ACTION].includes(step.type)"
        class="mt-1 border-t border-gray-200 pt-1 transition-colors duration-150 group-hover/step:border-gray-300"
      >
        <Text
          id="text"
          class="px-3"
          is-input
          :variant="Variant.STEALTH"
          :model-value="unpackSubnodeProperty(NodeType.STEP, StepType.COMMENT, step.subnodePacked, 'text')"
          @update:model-value="
            (newText) =>
              flowCtx.tx.update(
                step!,
                makeEdit(step!, { metatype: NodeType.STEP, type: StepType.COMMENT, subnode: { text: newText } }),
                { debounce: 'long' },
              )
          "
        />
        <Code
          v-if="step.type == StepType.ACTION"
          id="code"
          class="px-3"
          is-input
          :variant="Variant.STEALTH"
          :model-value="unpackSubnodeProperty(NodeType.STEP, StepType.ACTION, step.subnodePacked, 'code')"
          @update:model-value="
            (newCode) =>
              flowCtx.tx.update(
                step!,
                makeEdit(step!, { metatype: NodeType.STEP, type: StepType.ACTION, subnode: { code: newCode } }),
                { debounce: 'long' },
              )
          "
        />
      </div>
    </div>

    <!-- NOTE :UX: show last step output/error here? -->

    <!-- Padding (to ensure height is a multiple of the grid) -->
    <div class="w-full" :style="{ height: paddingHeight + 'px' }" />
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- should never be rendered by containing flow -->
    <span>???</span>
  </div>
</template>
