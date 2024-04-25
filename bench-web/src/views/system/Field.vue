<script lang="ts" setup>
import { ViewData, NodeType, Variant, ColorShade, FieldKind } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { makeViewId } from "@/views";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { IconInline, getNodeIcon } from "@/system/icon";
import type { ActionMapImplementation } from "@/system/action";
import Icon from "@/views/content/Icon.vue";
import { type OverlayMenuInfo, type OverlayMenuInfoIn } from "@/utils/menu";
import type { TooltipInfo } from "@/utils/tooltip";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<HTMLElement | null>(null);

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.FIELD>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `field.${nodePtr.value.id}` },
    computed(() => ({ roots: [nodePtr.value], isEnabled: nodePtr.value != null })),
  );
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    ref="fieldRef"
    v-if="field"
    class="flex w-fit flex-row items-center rounded border bg-gray-50 px-1.5 py-[3px]"
    :class="[
      inspectionPtr?.id == field.id
        ? 'border-primary-900'
        : [variant != Variant.STEALTH ? 'border-gray-200' : 'border-gray-200', 'hover:border-gray-300'],
    ]"
  >
    <!-- nocheckin: Field -->
    <IconInline
      v-bind="getNodeIcon(field)"
      :shade="field.kind == FieldKind.OPTION ? ColorShade.S600 : ColorShade.S700"
      class="mr-1 w-5 rounded border border-transparent p-0.5 hover:cursor-pointer hover:bg-primary-100 data-[menu=true]:border-primary-900 data-[menu=true]:bg-primary-100"
      v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
      v-menu="
        (): OverlayMenuInfoIn => ({
          component: Icon,
          placement: 'bottom-right',
          offset: '-referenceWidth',
          props: { modelValue: field!.icon },
          onApply: (newIcon) => pkgConnection.tx.update(field!, { icon: newIcon }),
        })
      "
    />
    <input
      ref="nameRef"
      class="w-fit min-w-fit max-w-fit truncate rounded border-0 bg-transparent font-medium outline-none ring-0 hover:bg-primary-100 hover:text-primary-900 focus:ring-0"
      spellcheck="false"
      :value="field.name"
      :size="Math.max(field.name?.length ?? 0, 5)"
      @input="pkgConnection.tx.update(field!, { name: ($event.target as HTMLInputElement).value })"
    />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
