<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let name = $state('');
  let greeting = $state('');
  let error = $state('');

  async function greet(): Promise<void> {
    error = '';
    try {
      greeting = await invoke<string>('greet', { name });
    } catch (e: unknown) {
      greeting = '';
      error = e instanceof Error ? e.message : 'Failed to connect to backend';
    }
  }

  function handleSubmit(e: SubmitEvent): void {
    e.preventDefault();
    void greet();
  }
</script>

<main>
  <h1>DupliFind</h1>
  <p class="subtitle">Find and remove duplicate files</p>

  <div class="test-section">
    <h2>Connection Test</h2>
    <form onsubmit={handleSubmit}>
      <label for="name-input" class="sr-only">Your name</label>
      <input
        id="name-input"
        type="text"
        bind:value={name}
        placeholder="Enter your name"
      />
      <button type="submit">Test Backend</button>
    </form>
    {#if greeting}
      <p class="greeting">{greeting}</p>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <div class="info">
    <p>This is a placeholder UI. The full interface will be built in subsequent phases.</p>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .test-section {
    background: var(--surface);
    padding: 2rem;
    border-radius: 8px;
    margin-bottom: 2rem;
    width: 100%;
    max-width: 400px;
  }

  .test-section h2 {
    font-size: 1.2rem;
    margin-bottom: 1rem;
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
    font-weight: 500;
  }

  button:hover {
    opacity: 0.9;
  }

  .greeting {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--success-bg);
    border-radius: 4px;
    color: var(--success);
  }

  .error {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--error-bg);
    border-radius: 4px;
    color: var(--error);
  }

  .info {
    color: var(--text-secondary);
    font-size: 0.875rem;
    text-align: center;
  }
</style>
