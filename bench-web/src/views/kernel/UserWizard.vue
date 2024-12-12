<script lang="ts" setup>
import { makeTypeInfo } from "@/language/field";
import { useSubnode } from "@/language/node";
import { makeEdit } from "@/language/transaction";
import {
  BenchType,
  NodeType,
  Region,
  UserWizardViewStage,
  Variant,
  ViewData,
  ViewType,
  type NodeReferenceData,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { benchPtr } from "@/system/client";
import { useExistingConnection } from "@/system/connection";
import { canvas, goToBench } from "@/system/space";
import { logIn, signUp, user } from "@/system/user";
import { makeIcon } from "@/ui/icon";
import { getViewComponentChildren, isVueInstanceOf } from "@/ui/view";
import { DEFAULT_REGION_BY_AREA, GEOLOCATION } from "@/utils/geolocation";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import Picker from "@/views/content/Picker.vue";
import Button from "@/views/controls/Button.vue";
import { computed, ref, toRef, watchEffect, type Ref } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<ViewData, "title" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph } = useExistingConnection(toRef(props, "self"));

const subnode = useSubnode(NodeType.VIEW, ViewType.USER_WIZARD, toRef(props, "subnodePacked"));
const stage = computed(() => subnode.value?.stage ?? UserWizardViewStage.LOG_IN);
const name: Ref<string> = ref("");
const slug: Ref<string> = ref("");
const email: Ref<string> = ref("");
const password: Ref<string> = ref("");
const region: Ref<Region> = ref(Region.FRANKFURT);
watchEffect(() => {
  if (GEOLOCATION.value?.area != null) {
    const defaultRegion = DEFAULT_REGION_BY_AREA[GEOLOCATION.value.area];
    if (defaultRegion != null) {
      region.value = defaultRegion;
    }
  }
});
const isActive = ref(false);
const lastError: Ref<string | null> = ref(null);

function clear() {
  name.value = "";
  slug.value = "";
  email.value = "";
  password.value = "";
}

function switchStage() {
  const selfNode = spaceGraph.getOrError(self.value);
  if (stage.value == UserWizardViewStage.LOG_IN) {
    state.update(
      makeEdit(selfNode, {
        metatype: NodeType.VIEW,
        type: ViewType.USER_WIZARD,
        title: "Sign up",
        subnode: { stage: UserWizardViewStage.SIGN_UP },
      }),
    );
  } else if (stage.value == UserWizardViewStage.SIGN_UP) {
    state.update(
      makeEdit(selfNode, {
        metatype: NodeType.VIEW,
        type: ViewType.USER_WIZARD,
        title: "Log in",
        subnode: { stage: UserWizardViewStage.LOG_IN },
      }),
    );
  } else {
    throw new Error(`unexpected registration stage: ${stage.value}`);
  }
}

async function submit() {
  isActive.value = true;
  lastError.value = null;
  try {
    if (stage.value == UserWizardViewStage.SIGN_UP) {
      await signUp({ name: name.value, slug: slug.value, email: email.value }, password.value);
    } else if (stage.value == UserWizardViewStage.LOG_IN) {
      const { user } = await logIn({ slug: slug.value }, password.value);
      // if we're outside a Bench and have a Bench, go home
      if (user.mainBenchPtr != null && benchPtr.value == null) {
        await goToBench({ bench: user.mainBenchPtr as TypedNodeReferenceData<NodeType.BENCH> });
      }
    } else {
      throw new Error(`unexpected registration stage: ${stage.value}`);
    }
    clear();
  } catch (e) {
    lastError.value = (e as Error).message ?? "Unknown error";
  } finally {
    isActive.value = false;
  }
}

const instance = canvas.registerView(self, id);
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  const childViews = getViewComponentChildren(instance);
  if (anchor != "bottom") {
    return childViews.find((v) => isVueInstanceOf(v, NativeInput));
  } else {
    return childViews.reverse().find((v) => isVueInstanceOf(v, Button));
  }
}

defineExpose<ViewExposed>({ self, focus });
</script>
<template>
  <div class="mx-auto mt-[20%] h-fit min-w-80 max-w-96 rounded px-9 py-7 text-gray-900">
    <!-- Header -->
    <div>
      <h2 class="text-2xl font-semibold">{{ title }}</h2>
      <p class="mt-2 text-gray-500">
        <span v-if="user">You're logged in and good to go.</span>
        <span v-else-if="stage == UserWizardViewStage.LOG_IN">Log into an existing Bench account.</span>
        <span v-else-if="stage == UserWizardViewStage.SIGN_UP">Create a new Bench account.</span>
      </p>
    </div>
    <!-- Data -->
    <div v-if="!user" class="mt-5 flex w-full flex-col gap-y-3">
      <NativeInput
        v-if="stage == UserWizardViewStage.SIGN_UP"
        id="name"
        ref="nameRef"
        v-model="name"
        :icon="makeIcon({ faName: 'fas fa-user' })"
        name="Name"
        title="Name"
        :variant="Variant.PRIMARY"
        is-input
      />
      <NativeInput
        id="slug"
        ref="slugRef"
        v-model="slug"
        :icon="makeIcon({ faName: 'fas fa-at' })"
        name="slug"
        title="Username"
        :variant="Variant.PRIMARY"
        is-input
      />
      <NativeInput
        v-if="stage == UserWizardViewStage.SIGN_UP"
        id="email"
        v-model="email"
        :icon="makeIcon({ faName: 'fas fa-at' })"
        name="Email"
        title="Email"
        :variant="Variant.PRIMARY"
        is-input
      />
      <!-- NOTE :UX: add passowrd feedback (see https://zxcvbn-ts.github.io/zxcvbn/) -->
      <NativeInput
        id="password"
        v-model="password"
        :icon="makeIcon({ faName: 'fas fa-key' })"
        name="Password"
        title="Password"
        :variant="Variant.PRIMARY"
        is-input
        :value-type="makeTypeInfo({ isSecret: true })"
      />
      <Picker
        v-if="stage == UserWizardViewStage.SIGN_UP"
        id="region"
        v-model="region"
        :icon="makeIcon({ faName: 'fas fa-globe' })"
        name="Region"
        title="Region"
        is-input
        :value-type="makeTypeInfo({ benchType: BenchType.REGION, isList: false, isRequired: true })"
      />
    </div>
    <!-- Actions -->
    <div class="mt-7">
      <Button
        v-if="!user"
        id="submit"
        name="Submit"
        :icon="makeIcon({ faName: 'fas fa-arrow-right-from-bracket' })"
        :title="stage === UserWizardViewStage.LOG_IN ? 'Log in' : 'Sign up'"
        class="w-full"
        :is-disabled="isActive"
        :is-loading="isActive"
        @click="submit"
      />
      <Button
        v-if="!user"
        id="switch"
        name="Switch"
        :icon="makeIcon({ faName: 'fas fa-shuffle' })"
        :title="stage === UserWizardViewStage.LOG_IN ? 'Sign up' : 'Log in'"
        class="mt-2 w-full"
        :variant="Variant.COMPACT"
        @click="() => switchStage()"
      />
    </div>
    <!-- Error -->
    <div v-if="lastError" class="mt-3 flex w-full flex-col gap-y-1 border-t border-t-gray-200 pt-3">
      <div class="flex flex-row items-center gap-x-2">
        <i class="fas fa-circle-exclamation text-danger-600" />
        <span class="text-danger-600">Error</span>
      </div>
      <div class="flex flex-row items-center gap-x-2">
        <span class="text-gray-500">{{ lastError }}</span>
      </div>
    </div>
  </div>
</template>
