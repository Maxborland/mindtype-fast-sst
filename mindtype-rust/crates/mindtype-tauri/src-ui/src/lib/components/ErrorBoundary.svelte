<script lang="ts">
  import { captureError, getCrashReportsDir } from '../utils/crashReporter';
  import Button from './Button.svelte';

  interface Props {
    children: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  let error = $state<Error | null>(null);
  let crashReportsDir = $state<string>('');

  // Track error for display
  function handleError(e: Error) {
    error = e;
    captureError(e, 'ErrorBoundary caught error');

    // Get crash reports dir
    getCrashReportsDir().then((dir) => {
      crashReportsDir = dir;
    });
  }

  // Reset error state
  function resetError() {
    error = null;
  }

  // Reload the app
  function reloadApp() {
    window.location.reload();
  }

  // Svelte 5 doesn't have built-in error boundaries yet,
  // so we use a try-catch wrapper in an $effect
  $effect(() => {
    // Set up error listener for this component tree
    const errorHandler = (event: ErrorEvent) => {
      event.preventDefault();
      handleError(event.error || new Error(event.message));
    };

    window.addEventListener('error', errorHandler);

    return () => {
      window.removeEventListener('error', errorHandler);
    };
  });
</script>

{#if error}
  <div class="error-boundary">
    <div class="error-content">
      <div class="error-icon">!</div>
      <h2>Something went wrong</h2>
      <p class="error-message">{error.message}</p>

      {#if error.stack}
        <details class="error-details">
          <summary>Technical Details</summary>
          <pre>{error.stack}</pre>
        </details>
      {/if}

      {#if crashReportsDir}
        <p class="crash-info">
          A crash report has been saved to:<br />
          <code>{crashReportsDir}</code>
        </p>
      {/if}

      <div class="error-actions">
        <Button onclick={resetError}>Try Again</Button>
        <Button onclick={reloadApp}>Reload App</Button>
      </div>

      <p class="support-info">
        If this problem persists, please contact support at
        <a href="mailto:help@mindtype.space">help@mindtype.space</a>
      </p>
    </div>
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .error-boundary {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    padding: 20px;
    background: #c0c0c0;
    font-family: 'Chicago', 'Geneva', sans-serif;
  }

  .error-content {
    max-width: 500px;
    padding: 20px;
    background: white;
    border: 2px solid black;
    box-shadow: 2px 2px 0 black;
  }

  .error-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto 16px;
    font-size: 32px;
    font-weight: bold;
    line-height: 48px;
    text-align: center;
    background: #ffcc00;
    border: 2px solid black;
    border-radius: 50%;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 16px;
    text-align: center;
  }

  .error-message {
    margin: 0 0 16px;
    padding: 8px;
    font-size: 12px;
    text-align: center;
    background: #f0f0f0;
    border: 1px solid #999;
  }

  .error-details {
    margin: 0 0 16px;
  }

  .error-details summary {
    cursor: pointer;
    font-size: 12px;
    color: #666;
  }

  .error-details pre {
    max-height: 150px;
    margin: 8px 0 0;
    padding: 8px;
    font-size: 10px;
    overflow: auto;
    background: #f8f8f8;
    border: 1px solid #ccc;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .crash-info {
    margin: 0 0 16px;
    font-size: 11px;
    color: #666;
    text-align: center;
  }

  .crash-info code {
    display: block;
    margin-top: 4px;
    font-size: 10px;
    word-break: break-all;
  }

  .error-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
    margin-bottom: 16px;
  }

  .support-info {
    margin: 0;
    font-size: 11px;
    color: #666;
    text-align: center;
  }

  .support-info a {
    color: #0066cc;
  }
</style>
