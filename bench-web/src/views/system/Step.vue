<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { INCOMING_STEP_TYPES, OUTGOING_STEP_TYPES, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, FIELD_CONTEXT_ACTIONS, makeTypeInfo, NAME_TYPE, type TypeIdentity } from "@/language/field";
import {
  FLOW_GRID_STEP,
  FLOW_PORT_SIZE,
  getStepSides,
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
  ColorType,
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
import { IS_CHROMIUM } from "@/system/client";
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
import { COLOR_BY_RUN_STATUS, getColorHex, getNodeColorHex, getRunColorHex } from "@/ui/style";
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
const sides = computed(() => (step.value != null ? getStepSides(step.value) : []));

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
    class="group/step rounded border border-gray-200 bg-white transition-colors duration-150"
    @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step', step: step! })"
  >
    <!-- Ports -->
    <div
      v-for="side in sides"
      class="absolute left-1/2 -translate-x-1/2"
      :class="[
        side == PortSide.INCOMING ? 'top-0 -translate-y-1/2' : 'bottom-0 translate-y-1/2',
        IS_CHROMIUM ? '' : '-mt-1 mb-1', // NOTE :Cleanup: why do we need this Step port offset for non-chromium?
      ]"
    >
      <button
        class="relative cursor-crosshair rounded-sm border border-gray-200 bg-white outline-none transition-colors duration-150 hover:bg-gray-100"
        :class="
          flowCtx.getPipesAtPort(step, side).length > 0 || flowCtx.isDraggingPort
            ? ''
            : 'opacity-0 group-hover/step:opacity-100'
        "
        :style="{
          width: FLOW_PORT_SIZE + 'px',
          height: FLOW_PORT_SIZE + 'px',
        }"
        @mousedown="(e) => flowCtx.startDragging(e, { kind: 'step-port', step: step!, side })"
        @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step-port', step: step!, side })"
      >
        <div
          v-if="flowCtx.getPipesAtPort(step, side).length > 0"
          class="flex-row-wrap absolute flex flex-col"
          :style="{
            width: FLOW_PORT_SIZE - 4 + 'px',
            height: FLOW_PORT_SIZE - 4 + 'px',
            top: 1 + 'px',
            left: 1 + 'px',
          }"
        >
          <div
            v-for="pipe in flowCtx.getPipesAtPort(step, side)"
            :key="pipe.id"
            class="flex-1 rounded-sm bg-gray-600"
            :style="{ backgroundColor: getColorHex(pipe.color ?? ColorType.GRAY, ColorShade.S400) }"
          />
        </div>
      </button>
    </div>

    <!-- Header (:StepHeight) -->
    <div
      v-if="step.type != StepType.TEXT"
      ref="headerRef"
      class="flex w-full flex-row items-center transition-colors duration-150"
      :style="{
        height: STEP_HEADER_HEIGHT + 'px',
      }"
    >
      <!-- Icon -->
      <div
        class="ml-1 flex flex-row items-center rounded px-1 py-1"
        :style="{
          backgroundColor: getNodeColorHex(step, ColorShade.S300),
        }"
      >
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
          class="w-5 flex-shrink-0 rounded-sm py-0.5 text-gray-700 hover:cursor-pointer"
        />
      </div>
      <!-- Name -->
      <NativeInput
        id="name"
        ref="nameRef"
        class="ml-2 truncate font-medium text-gray-700 transition-colors duration-150"
        is-input
        :value-type="NAME_TYPE"
        :variant="Variant.STEALTH"
        :model-value="step.name"
        @update:model-value="(newValue) => flowCtx.tx.update(step!, { name: newValue as string }, { debounce: 'long' })"
      />
      <!-- Controls/Meta -->
      <div class="ml-auto flex flex-row pl-2 pr-1.5">
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
          class="rounded-sm text-gray-400 hover:text-gray-700"
        >
          <i class="fas fa-ellipsis-v w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div v-if="step.type == StepType.TEXT" ref="bodyRef" class="relative px-3 py-1">
      <!-- Content (:StepHeight) -->
      <Text
        id="text"
        class=""
        placeholder="Text..."
        is-input
        :variant="Variant.STEALTH"
        :model-value="unpackSubnodeProperty(NodeType.STEP, StepType.TEXT, step.subnodePacked, 'text')"
        @update:model-value="
          (newText) =>
            flowCtx.tx.update(
              step!,
              makeEdit(step!, { metatype: NodeType.STEP, type: StepType.TEXT, subnode: { text: newText } }),
              { debounce: 'long' },
            )
        "
      />
    </div>

    <!-- NOTE :UX: show last step output/error here? -->
  </div>
  <div v-else ref="containerRef" class="rounded-sm border border-gray-200 bg-white">
    <!-- should never be rendered by containing flow -->
    <span class="text-danger-600">???</span>
  </div>
</template>
