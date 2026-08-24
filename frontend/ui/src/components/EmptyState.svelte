<script lang="ts">
  interface Props {
    title: string;
    description?: string;
  }

  let { title, description }: Props = $props();
</script>

<div class="rounded-lg border border-dashed border-border-subtle px-6 py-10 text-center">
  <p class="text-sm font-medium">{title}</p>
  {#if description}
    <p class="mt-1 text-sm text-muted">{description}</p>
  {/if}
</div>
