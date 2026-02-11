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

  let containerRef: HTMLElement | undefined;
  let isDragging = $state(false);
  let currentWidth = $state(400);

  // Keep currentWidth in sync with masterWidth prop changes
  $effect(() => {
    currentWidth = masterWidth;
  });

  // Computed max width based on container
  let maxWidth = $derived(
    containerRef ? containerRef.getBoundingClientRect().width - minDetailWidth : 1000
  );

  function startDrag(e: MouseEvent) {
    if (!containerRef) return;
    isDragging = true;
    e.preventDefault();
  }

  function onDrag(e: MouseEvent) {
    if (!isDragging || !containerRef) return;

    const containerRect = containerRef.getBoundingClientRect();
    const newWidth = e.clientX - containerRect.left;

    const max = containerRect.width - minDetailWidth;
    currentWidth = Math.max(minMasterWidth, Math.min(max, newWidth));
  }

  function stopDrag() {
    isDragging = false;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!containerRef) return;
    
    const step = 20;
    const max = containerRef.getBoundingClientRect().width - minDetailWidth;
    
    if (e.key === 'ArrowLeft') {
      currentWidth = Math.max(minMasterWidth, currentWidth - step);
      e.preventDefault();
    } else if (e.key === 'ArrowRight') {
      currentWidth = Math.min(max, currentWidth + step);
      e.preventDefault();
    } else if (e.key === 'Home') {
      currentWidth = minMasterWidth;
      e.preventDefault();
    } else if (e.key === 'End') {
      currentWidth = max;
      e.preventDefault();
    }
  }
</script>

<svelte:window onmousemove={onDrag} onmouseup={stopDrag} />

<div class="master-detail" bind:this={containerRef}>
  <div class="master-panel" style="width: {currentWidth}px">
    {@render master()}
  </div>

  <!-- 
    Using role="separator" makes this interactive per ARIA spec, but Svelte doesn't 
    recognize it, so we need to suppress the warnings. The tabindex and event handlers
    are semantically correct for an ARIA separator widget.
  -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="divider"
    class:dragging={isDragging}
    onmousedown={startDrag}
    onkeydown={onKeyDown}
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize panels"
    aria-valuenow={currentWidth}
    aria-valuemin={minMasterWidth}
    aria-valuemax={maxWidth}
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
    position: relative;
  }

  .divider:hover,
  .divider.dragging {
    background: var(--primary);
  }

  .divider:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  /* Visual indicator when focused (but not dragging) */
  .divider:focus-visible::after {
    content: '';
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 2px;
    height: 40px;
    background: var(--primary);
    border-radius: 1px;
  }

  .detail-panel {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
