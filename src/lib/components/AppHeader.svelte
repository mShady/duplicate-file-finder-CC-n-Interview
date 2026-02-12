<script lang="ts">
  import type { DetectionResult } from '$lib/types';

  interface Props {
    currentView: 'home' | 'scanning' | 'results';
    detectionResult: DetectionResult | null;
    onNewScan: () => void;
    onViewResults: () => void;
    onToggleHistory: () => void;
  }

  let { currentView, detectionResult, onNewScan, onViewResults, onToggleHistory }: Props = $props();
</script>

<header class="app-header">
  <h1>DupliFind</h1>
  <nav>
    {#if currentView === 'results' || detectionResult}
      <button class="nav-button" onclick={onNewScan}> New Scan </button>
      {#if detectionResult}
        <button class="nav-button" class:active={currentView === 'results'} onclick={onViewResults}>
          Results ({detectionResult.groups.length})
        </button>
      {/if}
    {/if}
    <button class="nav-button" onclick={onToggleHistory}> History </button>
  </nav>
</header>

<style>
  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .app-header h1 {
    font-size: 1.25rem;
    margin: 0;
  }

  .app-header nav {
    display: flex;
    gap: 0.5rem;
  }

  .nav-button {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 0.875rem;
  }

  .nav-button:hover {
    background: var(--background);
  }

  .nav-button.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  @media (max-width: 768px) {
    .app-header h1 {
      font-size: 1rem;
    }

    .nav-button {
      padding: 0.4rem 0.75rem;
      font-size: 0.8rem;
    }
  }

  @media (max-width: 480px) {
    .app-header {
      flex-direction: column;
      gap: 0.5rem;
      align-items: flex-start;
    }

    .app-header nav {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
