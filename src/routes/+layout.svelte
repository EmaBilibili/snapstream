<script>
  import "../app.css";
  let { children } = $props();
</script>

<div class="min-h-screen bg-[#0b0f19] text-slate-100 flex flex-col font-sans antialiased selection:bg-indigo-500 selection:text-white">
  {@render children()}
</div>
