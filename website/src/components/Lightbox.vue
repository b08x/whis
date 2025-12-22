<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import IconChevronLeft from './icons/IconChevronLeft.vue'
import IconChevronRight from './icons/IconChevronRight.vue'
import IconClose from './icons/IconClose.vue'

const props = defineProps<{
  images: { src: string, alt: string }[]
  initialIndex?: number
}>()

const isOpen = defineModel<boolean>('open', { default: false })
const currentIndex = ref(props.initialIndex ?? 0)

const currentImage = computed(() => props.images[currentIndex.value])

function next() {
  currentIndex.value = (currentIndex.value + 1) % props.images.length
}

function prev() {
  currentIndex.value = (currentIndex.value - 1 + props.images.length) % props.images.length
}

function close() {
  isOpen.value = false
}

function handleBackdropClick(e: MouseEvent) {
  if (e.target === e.currentTarget) {
    close()
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (!isOpen.value)
    return
  if (e.key === 'Escape')
    close()
  if (e.key === 'ArrowLeft')
    prev()
  if (e.key === 'ArrowRight')
    next()
}

watch(
  () => props.initialIndex,
  (val) => {
    if (val !== undefined)
      currentIndex.value = val
  },
)

onMounted(() => {
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen" class="lightbox" @click="handleBackdropClick">
      <button class="close-btn" aria-label="Close" @click="close">
        <IconClose />
      </button>
      <button class="nav-btn prev" aria-label="Previous" @click="prev">
        <IconChevronLeft />
      </button>
      <img
        v-if="currentImage"
        :src="currentImage.src"
        :alt="currentImage.alt"
        class="lightbox-img"
      >
      <button class="nav-btn next" aria-label="Next" @click="next">
        <IconChevronRight />
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.lightbox {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.lightbox-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 8px;
}

.close-btn {
  position: absolute;
  top: 1rem;
  right: 1rem;
  appearance: none;
  background: transparent;
  border: none;
  color: white;
  cursor: pointer;
  width: 3rem;
  height: 3rem;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: opacity 0.15s ease;
}

.close-btn:hover {
  opacity: 1;
}

.close-btn :deep(svg) {
  width: 24px;
  height: 24px;
}

.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  appearance: none;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  color: white;
  width: 3rem;
  height: 3rem;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition:
    opacity 0.15s ease,
    background 0.15s ease;
}

.nav-btn.prev {
  left: 1rem;
}

.nav-btn.next {
  right: 1rem;
}

.nav-btn:hover {
  opacity: 1;
  background: rgba(255, 255, 255, 0.2);
}

.nav-btn :deep(svg) {
  width: 24px;
  height: 24px;
}
</style>
