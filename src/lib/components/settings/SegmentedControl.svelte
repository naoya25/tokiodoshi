<script lang="ts" generics="T extends string">
  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    options: Option[];
    value: T;
    onChange: (v: T) => void;
    ariaLabel: string;
  }

  const { options, value, onChange, ariaLabel }: Props = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={ariaLabel}>
  {#each options as opt (opt.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === opt.value}
      class:active={value === opt.value}
      onclick={() => onChange(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    border: 1px solid color-mix(in srgb, var(--sumi) 15%, transparent);
    border-radius: 4px;
    overflow: hidden;
  }
  button {
    background: transparent;
    border: none;
    padding: 6px 12px;
    font: inherit;
    font-size: 11px;
    letter-spacing: 0.15em;
    color: inherit;
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s, background 0.15s;
  }
  button:hover {
    opacity: 0.9;
  }
  button.active {
    background: color-mix(in srgb, var(--sumi) 12%, transparent);
    opacity: 1;
  }
  button:focus-visible {
    outline: 1.5px solid var(--sumi);
    outline-offset: -1px;
  }
  button + button {
    border-left: 1px solid color-mix(in srgb, var(--sumi) 10%, transparent);
  }
</style>
