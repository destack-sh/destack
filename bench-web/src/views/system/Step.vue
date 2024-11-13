<script lang="ts" setup>
import { createField, NAME_TYPE } from "@/language/field";
import {
  FLOW_GRID_STEP,
  FLOW_PORT_SIZE,
  getStepSides,
  STEP_CONTEXT_ACTIONS,
  STEP_HEADER_HEIGHT,
  useFlowContext
} from "@/language/flow";
import { cloneNode, unpackSubnodeProperty } from "@/language/node";
import { makeEdit } from "@/language/transaction";
import {
  FieldData,
  NodeType,
  PortSide,
  StepType,
  Variant,
  ViewData
} from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { IS_CHROMIUM } from "@/system/client";
import { runtime } from "@/system/runtime";
import { canvas } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { menuActionsLike, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { COLOR_BY_RUN_STATUS } from "@/ui/style";
import type { TooltipInfo } from "@/ui/tooltip";
import { focusInElement } from "@/ui/view";
import { viewEmits, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import Text from "@/views/content/Text.vue";
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
    isEnabled: () => step.value?.type != StepType.TEXT,
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
    class="group/step rounded border transition-colors duration-150"
    :class="[
      step.type == StepType.TEXT ? 'bg-gray-100' : 'bg-white',
      isInspected || isHighlighted ? 'border-primary-700' : 'border-gray-200',
    ]"
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
        class="relative cursor-crosshair rounded-2xl border outline-none transition-colors duration-150"
        :class="[
          flowCtx.isDraggingPort || isInspected || isHighlighted ? '' : 'opacity-0 group-hover/step:opacity-100',
          flowCtx.isDraggingPortAt(step, side) ? 'bg-primary-400' : 'bg-white hover:bg-primary-400',
          isInspected ||
          isHighlighted ||
          (flowCtx.draggable?.kind == 'step-port' &&
            flowCtx.draggable?.step?.ck == step.ck &&
            flowCtx.draggable.side == side)
            ? 'border-primary-700'
            : 'border-gray-200 hover:border-primary-700',
        ]"
        :style="{
          width: FLOW_PORT_SIZE + 'px',
          height: FLOW_PORT_SIZE + 'px',
        }"
        @mousedown="(e) => flowCtx.startDragging(e, { kind: 'step-port', step: step!, side })"
        @mouseup="(e) => flowCtx.endDragging(e, { kind: 'step-port', step: step!, side })"
      />
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
      <div class="ml-1 flex flex-row items-center rounded px-1 py-1">
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
          class="w-5 flex-shrink-0 rounded py-0.5 text-gray-700 hover:cursor-pointer"
        />
      </div>
      <!-- Name -->
      <NativeInput
        id="name"
        ref="nameRef"
        class="ml-2 truncate font-medium text-gray-700 transition-colors duration-150"
        is-input
        placeholder="Name..."
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
          class="rounded text-gray-400 hover:text-gray-700"
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
      <!-- Floating meta -->
      <div class="absolute right-0 top-0 px-1.5 py-1.5">
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
          class="rounded text-gray-400 hover:text-gray-700"
        >
          <i class="fas fa-ellipsis-v w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- NOTE :UX: show last step output/error here? -->
  </div>
  <div v-else ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- should never be rendered by containing flow -->
    <span class="text-danger-600">???</span>
  </div>
</template>
