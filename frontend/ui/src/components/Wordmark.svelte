<script lang="ts">
  interface Props {
    /** The word itself. Products pass their own name. */
    text: string;
    /** Letter spacing makes an all-caps wordmark read as a mark, not as a heading. */
    tracking?: "normal" | "wide" | "widest";
  }

  let { text, tracking = "widest" }: Props = $props();

  const trackingClass = $derived(
    { normal: "tracking-normal", wide: "tracking-wide", widest: "tracking-[0.28em]" }[tracking],
  );
</script>

<!--
  Decorative: the product name is already in the window title and the header, so a
  screen reader would only repeat it.
-->
<p
  aria-hidden="true"
  class="select-none text-center text-5xl font-black text-silver sm:text-7xl {trackingClass}"
>
  {text}
</p>
