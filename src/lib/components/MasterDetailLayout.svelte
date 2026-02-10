<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    masterWidth?: number;
    minMasterWidth?: number;
    minDetailWidth?: number;
    master: Snippet;
    detail: Snippet;
  }

  let { masterWidth = 400, minMasterWidth = 300, minDetailWidth = 400, master, detail }: Props = $props();

  let containerRef: HTMLElement;
  let isDragging = $state(false);
  let currentWidth = $state(masterWidth);

  // Keep currentWidth in sync if prop changes externally
  $effect(() => {
    currentWidth = masterWidth;
  });

  function startDrag(e: MouseEvent) {
    isDragging = true;
    e.preventDefault();
  }

  function onDrag(e: MouseEvent) {
    if (!isDragging || !containerRef) return;

    const containerRect = containerRef.getBoundingClientRect();
    const newWidth = e.clientX - containerRect.left;

    const maxWidth = containerRect.width - minDetailWidth;
    currentWidth = Math.max(minMasterWidth, Math.min(maxWidth, newWidth));
  }

  function stopDrag() {
    isDragging = false;
  }

  function onKeyDown(e: KeyboardEvent) {
    const step = 20;
    if (e.key === 'ArrowLeft') {
      currentWidth = Math.max(minMasterWidth, currentWidth - step);
      e.preventDefault();
    } else if (e.key === 'ArrowRight') {
      if (containerRef) {
        const maxWidth = containerRef.getBoundingClientRect().width - minDetailWidth;
        currentWidth = Math.min(maxWidth, currentWidth + step);
      }
      e.preventDefault();
    }
  }
</script>

<svelte:window onmousemove={onDrag} onmouseup={stopDrag} />

<div class="master-detail" bind:this={containerRef}>
  <div class="master-panel" style="width: {currentWidth}px">
    {@render master()}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
  <div
    class="divider"
    class:dragging={isDragging}
    onmousedown={startDrag}
    onkeydown={onKeyDown}
    role="separator"
    aria-orientation="vertical"
    aria-valuenow={currentWidth}
    aria-valuemin={minMasterWidth}
    tabindex="0"
  ></div>

  <div class="detail-panel">
    {@render detail()}
  </div>
</div>

<style>
  .master-detail {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  .master-panel {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .divider {
    width: 4px;
    background: var(--border);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background 0.2s;
  }

  .divider:hover,
  .divider.dragging {
    background: var(--primary);
  }

  .detail-panel {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
