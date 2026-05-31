<script lang="ts">
  interface Props {
    checked: boolean;
    onChange: (v: boolean) => void;
    ariaLabel: string;
  }

  const { checked, onChange, ariaLabel }: Props = $props();
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={ariaLabel}
  class:on={checked}
  onclick={() => onChange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  button {
    width: 36px;
    height: 20px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--sumi) 12%, transparent);
    border: none;
    position: relative;
    cursor: pointer;
    padding: 0;
    transition: background 0.18s ease;
  }
  button.on {
    background: var(--sumi);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--ji);
    transition: transform 0.18s ease;
  }
  button.on .knob {
    transform: translateX(16px);
  }
  button:focus-visible {
    outline: 1.5px solid var(--sumi);
    outline-offset: 2px;
  }
</style>
