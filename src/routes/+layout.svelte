<script lang="ts">
  // Fonts for the type themes, bundled with the app (SIL Open Font License).
  import "@fontsource-variable/inter/wght.css";
  import "@fontsource-variable/literata/wght.css";
  import "@fontsource-variable/source-sans-3/wght.css";
  import "@fontsource-variable/fraunces/wght.css";
  import "@fontsource-variable/ibm-plex-sans/wght.css";
  import "../app.css";
  let { children } = $props();
</script>

{@render children()}
