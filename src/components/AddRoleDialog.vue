<script setup lang="ts">
import { computed, ref, watch } from "vue";

const props = defineProps<{
  show: boolean;
  existingRoles: string[];
}>();

const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "success", role: string): void;
}>();

const value = ref("");

// 大小写不敏感去重，与 Python 版 add_role_dialog 保持一致
const existingLower = computed(
  () =>
    new Set(
      (props.existingRoles || [])
        .map((r) => (r || "").trim().toLowerCase())
        .filter(Boolean)
    )
);

const trimmed = computed(() => value.value.trim());
const duplicate = computed(
  () => trimmed.value !== "" && existingLower.value.has(trimmed.value.toLowerCase())
);
const canConfirm = computed(() => trimmed.value !== "" && !duplicate.value);

watch(
  () => props.show,
  (v) => {
    if (v) value.value = "";
  }
);

function close() {
  emit("update:show", false);
}

function confirm() {
  if (!canConfirm.value) return;
  emit("success", trimmed.value);
  emit("update:show", false);
}
</script>

<template>
  <n-modal
    :show="show"
    preset="card"
    title="新建类别"
    style="width: 420px"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <n-form label-placement="top">
      <n-form-item
        label="类别名称"
        :validation-status="duplicate ? 'error' : undefined"
        :feedback="duplicate ? '⚠ 该类别已存在' : ''"
      >
        <n-input
          v-model:value="value"
          placeholder="如：工作、生活、开发……"
          :maxlength="32"
          autofocus
          @keyup.enter="confirm"
        />
      </n-form-item>
    </n-form>
    <template #footer>
      <n-space justify="center">
        <n-button type="primary" :disabled="!canConfirm" @click="confirm">
          确认添加
        </n-button>
        <n-button @click="close">取消</n-button>
      </n-space>
    </template>
  </n-modal>
</template>
