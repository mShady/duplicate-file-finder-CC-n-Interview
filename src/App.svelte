<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ScanButton from './lib/components/ScanButton.svelte';

  let name = $state('');
  let greeting = $state('');

  async function greet() {
    greeting = await invoke('greet', { name });
  }
</script>

<main>
  <h1>DupliFind</h1>
  <p class="subtitle">Find and remove duplicate files</p>

  <div class="scan-section">
    <ScanButton />
  </div>

  <div class="test-section">
    <h2>Backend Connection Test</h2>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        greet();
      }}
    >
      <input type="text" bind:value={name} placeholder="Enter your name" />
      <button type="submit">Test</button>
    </form>
    {#if greeting}
      <p class="greeting">{greeting}</p>
    {/if}
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    min-height: 100vh;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .scan-section {
    width: 100%;
    max-width: 500px;
    margin-bottom: 2rem;
  }

  .test-section {
    background: var(--surface);
    padding: 1.5rem;
    border-radius: 8px;
    width: 100%;
    max-width: 400px;
  }

  .test-section h2 {
    font-size: 1rem;
    margin-bottom: 1rem;
    color: var(--text-secondary);
  }

  form {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--primary);
    color: white;
    cursor: pointer;
  }

  button:hover {
    opacity: 0.9;
  }

  .greeting {
    margin-top: 1rem;
    padding: 0.75rem;
    background: var(--success-bg);
    border-radius: 4px;
    color: var(--success);
  }
</style>
