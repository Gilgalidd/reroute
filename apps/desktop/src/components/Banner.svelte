<script lang="ts">
  let { tone = "info", children }: { tone?: "info" | "error" | "success"; children: import("svelte").Snippet } = $props();
</script>

<div class="banner {tone}" role={tone === "error" ? "alert" : "status"}>
  {@render children()}
</div>

<style>
  .banner {
    border-radius: 8px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    user-select: text;
  }
  .error {
    border-color: var(--danger);
    color: var(--danger);
  }
  .success {
    border-color: var(--success);
    color: var(--success);
  }
</style>
