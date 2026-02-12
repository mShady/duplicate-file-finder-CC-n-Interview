<script lang="ts">
  import DeletionHistoryPanel from './DeletionHistoryPanel.svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();
  let dialogRef = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (dialogRef) {
      dialogRef.focus();
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    if (e.key === 'Tab') {
      const focusable = dialogRef?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable && focusable.length > 0) {
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="dialog-overlay"
  role="dialog"
  aria-modal="true"
  aria-label="Deletion History"
  tabindex="-1"
  bind:this={dialogRef}
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="history-dialog" onclick={(e) => e.stopPropagation()}>
    <DeletionHistoryPanel {onClose} />
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .history-dialog {
    max-width: 700px;
    width: 90%;
  }
</style>
