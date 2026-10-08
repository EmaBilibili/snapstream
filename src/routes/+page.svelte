<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { check } from "@tauri-apps/plugin-updater";
  import {
    Download,
    Music,
    Video,
    Folder,
    Sparkles,
    CheckCircle2,
    AlertCircle,
    Loader2,
    RefreshCw,
    ExternalLink
  } from "lucide-svelte";

  interface MediaMetadata {
    title: String;
    thumbnail: string | null;
    duration: number | null;
    uploader: string | null;
    formats_summary: string[];
  }

  interface DownloadItem {
    id: string;
    url: string;
    title: string;
    thumbnail: string | null;
    mode: "video" | "audio";
    quality: string;
    percent: number;
    speed: string;
    eta: string;
    status: "queued" | "downloading" | "processing" | "finished" | "error";
    errorMsg?: string;
  }

  let urlInput = $state("");
  let selectedMode = $state<"video" | "audio">("video");
  let selectedQuality = $state("best");
  let downloadFolder = $state("~/Downloads");
  let isLoadingInfo = $state(false);
  let currentPreview = $state<MediaMetadata | null>(null);
  let errorMessage = $state("");
  let downloads = $state<DownloadItem[]>([]);
  let updateStatus = $state<string | null>(null);

  // Formateador de segundos a mm:ss o hh:mm:ss
  function formatDuration(sec: number | null): string {
    if (!sec) return "";
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  }

  async function handleInspectUrl() {
    if (!urlInput.trim()) return;
    errorMessage = "";
    isLoadingInfo = true;
    currentPreview = null;

    try {
      const data = await invoke<MediaMetadata>("get_media_info", { url: urlInput.trim() });
      currentPreview = data;
    } catch (err: any) {
      errorMessage = err?.toString() || "Error al obtener información del enlace.";
    } finally {
      isLoadingInfo = false;
    }
  }

  async function selectFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Seleccionar carpeta de descargas"
      });
      if (selected && typeof selected === "string") {
        downloadFolder = selected;
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function handleDownload() {
    if (!urlInput.trim()) return;
    const id = Date.now().toString();
    const title = currentPreview?.title || "Enlace multimedia";
    const thumbnail = currentPreview?.thumbnail || null;

    const newItem: DownloadItem = {
      id,
      url: urlInput.trim(),
      title: String(title),
      thumbnail,
      mode: selectedMode,
      quality: selectedQuality,
      percent: 0,
      speed: "Iniciando...",
      eta: "",
      status: "queued"
    };

    downloads = [newItem, ...downloads];

    try {
      await invoke("start_download", {
        id,
        url: urlInput.trim(),
        outputDir: downloadFolder,
        mode: selectedMode,
        quality: selectedQuality
      });
    } catch (err: any) {
      const item = downloads.find((d) => d.id === id);
      if (item) {
        item.status = "error";
        item.errorMsg = err?.toString();
      }
    }
  }

  async function checkForUpdates() {
    try {
      updateStatus = "Buscando actualizaciones...";
      const update = await check();
      if (update?.available) {
        updateStatus = `Nueva versión disponible: v${update.version}`;
        await update.downloadAndInstall();
        updateStatus = "Actualización instalada. Reinicia la app.";
      } else {
        updateStatus = "SnapStream está en su última versión.";
        setTimeout(() => (updateStatus = null), 4000);
      }
    } catch (e: any) {
      updateStatus = "No se pudo comprobar actualizaciones.";
      setTimeout(() => (updateStatus = null), 4000);
    }
  }

  onMount(() => {
    // Escuchar eventos de progreso emitidos desde Rust
    const unlisten = listen<any>("download-progress", (event) => {
      const payload = event.payload;
      const item = downloads.find((d) => d.id === payload.id);
      if (item) {
        item.percent = payload.percent;
        item.speed = payload.speed;
        item.eta = payload.eta;
        item.status = payload.status;
        if (payload.filename) {
          item.errorMsg = payload.filename;
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<div class="flex-1 flex flex-col max-w-4xl w-full mx-auto p-6 space-y-6">
  <!-- Top App Header -->
  <header class="flex items-center justify-between pb-4 border-b border-slate-800">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-xl bg-gradient-to-tr from-indigo-500 to-cyan-400 flex items-center justify-center shadow-lg shadow-indigo-500/20">
        <Sparkles class="w-5 h-5 text-white" />
      </div>
      <div>
        <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
          SnapStream
          <span class="text-xs px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-400 font-medium border border-indigo-500/20">v0.1.0</span>
        </h1>
        <p class="text-xs text-slate-400">Descargador multimedia universal y ligero</p>
      </div>
    </div>

    <div class="flex items-center space-x-2">
      {#if updateStatus}
        <span class="text-xs text-indigo-300 animate-pulse bg-indigo-950/60 px-3 py-1.5 rounded-lg border border-indigo-800">
          {updateStatus}
        </span>
      {/if}
      <button
        onclick={checkForUpdates}
        class="flex items-center space-x-1.5 text-xs bg-slate-800/80 hover:bg-slate-700/80 border border-slate-700 text-slate-300 px-3 py-1.5 rounded-lg transition-all"
        title="Buscar actualizaciones"
      >
        <RefreshCw class="w-3.5 h-3.5" />
        <span>Actualizar</span>
      </button>
    </div>
  </header>

  <!-- Input Link Card -->
  <div class="bg-slate-900/60 border border-slate-800/80 rounded-2xl p-5 shadow-xl backdrop-blur-md space-y-4">
    <label class="block text-xs font-semibold uppercase tracking-wider text-slate-400">
      Pega un enlace (YouTube, Instagram, Facebook, TikTok, X, etc.)
    </label>
    <div class="flex gap-2">
      <div class="relative flex-1">
        <input
          type="text"
          placeholder="https://..."
          bind:value={urlInput}
          onkeydown={(e) => e.key === "Enter" && handleInspectUrl()}
          class="w-full bg-slate-950/80 border border-slate-800 rounded-xl px-4 py-3 text-sm text-slate-100 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500 transition-all"
        />
      </div>
      <button
        onclick={handleInspectUrl}
        disabled={isLoadingInfo || !urlInput.trim()}
        class="px-5 py-3 rounded-xl bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white font-medium text-sm transition-all flex items-center gap-2 shadow-lg shadow-indigo-600/20 cursor-pointer"
      >
        {#if isLoadingInfo}
          <Loader2 class="w-4 h-4 animate-spin" />
          <span>Analizando...</span>
        {:else}
          <span>Inspeccionar</span>
        {/if}
      </button>
    </div>

    {#if errorMessage}
      <div class="flex items-center gap-2 p-3 bg-red-950/40 border border-red-800/50 rounded-xl text-red-300 text-xs">
        <AlertCircle class="w-4 h-4 shrink-0" />
        <span>{errorMessage}</span>
      </div>
    {/if}

    <!-- Preview Card -->
    {#if currentPreview}
      <div class="mt-4 p-4 rounded-xl bg-slate-800/40 border border-slate-700/50 flex gap-4 items-center">
        {#if currentPreview.thumbnail}
          <img
            src={currentPreview.thumbnail}
            alt="Thumbnail"
            class="w-28 h-20 object-cover rounded-lg bg-slate-950 border border-slate-800 shadow"
          />
        {/if}
        <div class="flex-1 min-w-0">
          <h2 class="text-sm font-semibold text-slate-100 truncate" title={String(currentPreview.title)}>
            {currentPreview.title}
          </h2>
          <div class="flex items-center gap-3 mt-1.5 text-xs text-slate-400">
            {#if currentPreview.uploader}
              <span>Canal: <strong class="text-slate-300">{currentPreview.uploader}</strong></span>
            {/if}
            {#if currentPreview.duration}
              <span>• Duración: <strong class="text-slate-300">{formatDuration(currentPreview.duration)}</strong></span>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    <!-- Options & Destination -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2">
      <!-- Mode Toggle -->
      <div class="flex bg-slate-950 p-1 rounded-xl border border-slate-800">
        <button
          onclick={() => { selectedMode = "video"; selectedQuality = "best"; }}
          class={`flex-1 flex items-center justify-center gap-2 py-2 text-xs font-medium rounded-lg transition-all ${
            selectedMode === "video" ? "bg-indigo-600 text-white shadow" : "text-slate-400 hover:text-slate-200"
          }`}
        >
          <Video class="w-3.5 h-3.5" />
          <span>Video</span>
        </button>
        <button
          onclick={() => { selectedMode = "audio"; selectedQuality = "mp3"; }}
          class={`flex-1 flex items-center justify-center gap-2 py-2 text-xs font-medium rounded-lg transition-all ${
            selectedMode === "audio" ? "bg-indigo-600 text-white shadow" : "text-slate-400 hover:text-slate-200"
          }`}
        >
          <Music class="w-3.5 h-3.5" />
          <span>Solo Audio</span>
        </button>
      </div>

      <!-- Quality Selector -->
      <div class="relative">
        <select
          bind:value={selectedQuality}
          class="w-full h-full bg-slate-950 border border-slate-800 rounded-xl px-3 py-2 text-xs text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
        >
          {#if selectedMode === "video"}
            <option value="best">Máxima Calidad (Original)</option>
            <option value="1080p">1080p Full HD</option>
            <option value="720p">720p HD</option>
          {:else}
            <option value="mp3">MP3 (Universal)</option>
            <option value="m4a">M4A (AAC)</option>
            <option value="flac">FLAC (Sin pérdida)</option>
          {/if}
        </select>
      </div>

      <!-- Destination folder button -->
      <button
        onclick={selectFolder}
        class="flex items-center justify-between bg-slate-950 border border-slate-800 hover:border-slate-700 px-3 py-2 rounded-xl text-xs text-slate-300 transition-all text-left"
      >
        <div class="flex items-center gap-2 truncate">
          <Folder class="w-3.5 h-3.5 text-indigo-400 shrink-0" />
          <span class="truncate">{downloadFolder}</span>
        </div>
        <span class="text-[10px] text-slate-500 underline ml-1">Cambiar</span>
      </button>
    </div>

    <!-- Download Trigger Action -->
    <div class="pt-2">
      <button
        onclick={handleDownload}
        disabled={!urlInput.trim()}
        class="w-full py-3.5 rounded-xl bg-gradient-to-r from-indigo-500 to-indigo-600 hover:from-indigo-400 hover:to-indigo-500 disabled:opacity-40 text-white font-semibold text-sm transition-all flex items-center justify-center gap-2 shadow-lg shadow-indigo-500/25 cursor-pointer"
      >
        <Download class="w-4 h-4" />
        <span>Iniciar Descarga</span>
      </button>
    </div>
  </div>

  <!-- Downloads List / History -->
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-400">
        Descargas ({downloads.length})
      </h3>
      {#if downloads.length > 0}
        <button
          onclick={() => (downloads = [])}
          class="text-xs text-slate-500 hover:text-slate-400 transition-colors"
        >
          Limpiar historial
        </button>
      {/if}
    </div>

    {#if downloads.length === 0}
      <div class="p-8 border border-dashed border-slate-800 rounded-2xl flex flex-col items-center justify-center text-slate-600 text-xs">
        <Download class="w-8 h-8 stroke-1 mb-2 opacity-50" />
        <span>No hay descargas en curso. Pega un enlace arriba para comenzar.</span>
      </div>
    {:else}
      <div class="space-y-2.5">
        {#each downloads as item (item.id)}
          <div class="bg-slate-900/40 border border-slate-800 rounded-xl p-4 transition-all">
            <div class="flex items-center justify-between gap-4">
              <div class="flex items-center gap-3 min-w-0">
                <div class="w-8 h-8 rounded-lg bg-slate-800 flex items-center justify-center shrink-0">
                  {#if item.mode === "audio"}
                    <Music class="w-4 h-4 text-emerald-400" />
                  {:else}
                    <Video class="w-4 h-4 text-indigo-400" />
                  {/if}
                </div>
                <div class="min-w-0">
                  <h4 class="text-xs font-medium text-slate-200 truncate">{item.title}</h4>
                  <div class="flex items-center gap-2 text-[11px] text-slate-500 mt-0.5">
                    <span class="capitalize">{item.mode} ({item.quality})</span>
                    {#if item.speed}
                      <span>• {item.speed}</span>
                    {/if}
                    {#if item.eta}
                      <span>• ETA: {item.eta}</span>
                    {/if}
                  </div>
                </div>
              </div>

              <div class="flex items-center gap-3 shrink-0">
                {#if item.status === "downloading"}
                  <span class="text-xs font-bold text-indigo-400 font-mono">{item.percent.toFixed(0)}%</span>
                {:else if item.status === "processing"}
                  <div class="flex items-center gap-1.5 text-xs text-amber-400">
                    <Loader2 class="w-3.5 h-3.5 animate-spin" />
                    <span>Extrayendo...</span>
                  </div>
                {:else if item.status === "finished"}
                  <div class="flex items-center gap-1 text-xs text-emerald-400">
                    <CheckCircle2 class="w-4 h-4" />
                    <span>Listo</span>
                  </div>
                {:else if item.status === "error"}
                  <span class="text-xs text-red-400" title={item.errorMsg}>Error</span>
                {/if}
              </div>
            </div>

            <!-- Progress Bar -->
            {#if item.status === "downloading" || item.status === "processing"}
              <div class="w-full bg-slate-950 rounded-full h-1.5 mt-3 overflow-hidden">
                <div
                  class="bg-gradient-to-r from-indigo-500 to-cyan-400 h-full rounded-full transition-all duration-300"
                  style="width: {item.percent}%"
                ></div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
