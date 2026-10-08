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
    Image as ImageIcon,
    Folder,
    Sparkles,
    CheckCircle2,
    AlertCircle,
    Loader2,
    RefreshCw,
    Clipboard,
    HardDrive,
    SlidersHorizontal,
    Radio
  } from "lucide-svelte";

  interface MediaMetadata {
    title: string;
    thumbnail: string | null;
    duration: number | null;
    uploader: string | null;
    is_direct_image: boolean;
    formats_summary: string[];
  }

  interface DownloadItem {
    id: string;
    url: string;
    title: string;
    thumbnail: string | null;
    mode: "video" | "audio" | "image";
    quality: string;
    percent: number;
    speed: string;
    eta: string;
    status: "queued" | "downloading" | "processing" | "finished" | "error";
    errorMsg?: string;
  }

  let urlInput = $state("");
  let selectedMode = $state<"video" | "audio" | "image">("video");
  let selectedQuality = $state("best");
  let downloadFolder = $state("~/Downloads");
  let isLoadingInfo = $state(false);
  let currentPreview = $state<MediaMetadata | null>(null);
  let errorMessage = $state("");
  let downloads = $state<DownloadItem[]>([]);
  let updateStatus = $state<string | null>(null);
  let isCheckingUpdate = $state(false);

  function formatDuration(sec: number | null): string {
    if (!sec) return "";
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  }

  async function pasteClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text && text.trim().startsWith("http")) {
        urlInput = text.trim();
        handleInspectUrl();
      }
    } catch (_) {}
  }

  async function handleInspectUrl() {
    if (!urlInput.trim()) return;
    errorMessage = "";
    isLoadingInfo = true;
    currentPreview = null;

    try {
      const data = await invoke<MediaMetadata>("get_media_info", { url: urlInput.trim() });
      currentPreview = data;
      if (data.is_direct_image) {
        selectedMode = "image";
      }
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
        title: "Seleccionar carpeta destino"
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
    const title = currentPreview?.title || "Recurso Multimedia";
    const thumbnail = currentPreview?.thumbnail || null;

    const newItem: DownloadItem = {
      id,
      url: urlInput.trim(),
      title: String(title),
      thumbnail,
      mode: selectedMode,
      quality: selectedQuality,
      percent: 0,
      speed: "Preparando enlace...",
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
        quality: selectedQuality,
        thumbnailUrl: thumbnail
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
    if (isCheckingUpdate) return;
    try {
      isCheckingUpdate = true;
      updateStatus = "Verificando...";
      const update = await check();
      if (update?.available) {
        updateStatus = `Nueva v${update.version}`;
        await update.downloadAndInstall();
        updateStatus = "Instalado. Reinicia la app.";
      } else {
        updateStatus = "Actualizado";
        setTimeout(() => (updateStatus = null), 3500);
      }
    } catch (e: any) {
      updateStatus = "Error de red";
      setTimeout(() => (updateStatus = null), 3500);
    } finally {
      isCheckingUpdate = false;
    }
  }

  onMount(() => {
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

<!-- Contenedor Maestro: Estilo "Mechanical Studio" con texturas técnicas y acentos cyber-amber -->
<div class="flex-1 flex flex-col w-full h-full min-h-screen bg-[#07090e] text-[#d6deeb] font-mono selection:bg-[#f59e0b] selection:text-black">
  
  <!-- Subtle Grid Accent overlay -->
  <div class="fixed inset-0 pointer-events-none opacity-[0.03] bg-[radial-gradient(#f59e0b_1px,transparent_1px)] [background-size:16px_16px]"></div>

  <!-- Main Frame Container -->
  <div class="relative z-10 flex-1 flex flex-col max-w-4xl w-full mx-auto p-5 sm:p-7 space-y-5">
    
    <!-- Top Hardware Bar -->
    <header class="flex items-center justify-between pb-4 border-b border-[#1b2234]">
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded bg-[#f59e0b] flex items-center justify-center text-black shadow-md shadow-[#f59e0b]/20 font-black text-sm">
          SS
        </div>
        <div>
          <div class="flex items-center gap-2">
            <span class="text-sm font-bold tracking-wider text-white uppercase font-sans">SnapStream</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded bg-[#161c2b] border border-[#232c42] text-[#f59e0b] font-mono font-semibold">v0.1.0</span>
          </div>
          <span class="text-[11px] text-[#5e6c87] tracking-tight">MULTI-PLATFORM ENGINE // YT - IG - FB - TT</span>
        </div>
      </div>

      <div class="flex items-center gap-2">
        {#if updateStatus}
          <div class="flex items-center gap-1.5 text-[11px] px-2.5 py-1 rounded bg-[#131926] border border-[#f59e0b]/40 text-[#f59e0b]">
            <Radio class="w-3 h-3 animate-ping" />
            <span>{updateStatus}</span>
          </div>
        {/if}
        <button
          onclick={checkForUpdates}
          disabled={isCheckingUpdate}
          class="flex items-center gap-1.5 text-[11px] bg-[#101522] hover:bg-[#161d2f] border border-[#1e263a] hover:border-[#f59e0b]/50 text-[#8b9bb4] hover:text-[#f59e0b] px-2.5 py-1.5 rounded transition-colors cursor-pointer"
          title="Verificar actualizaciones"
        >
          <RefreshCw class={`w-3 h-3 ${isCheckingUpdate ? 'animate-spin' : ''}`} />
          <span>Sync</span>
        </button>
      </div>
    </header>

    <!-- Console Deck: Input Area -->
    <div class="bg-[#0b0e17] border border-[#1b2234] rounded-lg p-4 shadow-2xl relative overflow-hidden space-y-4">
      
      <!-- Top Bar of Console -->
      <div class="flex items-center justify-between text-[11px] text-[#5e6c87] border-b border-[#141a29] pb-2.5">
        <span class="flex items-center gap-1.5 uppercase tracking-wider font-semibold">
          <SlidersHorizontal class="w-3.5 h-3.5 text-[#f59e0b]" />
          Captura de Fuente
        </span>
        <button
          onclick={pasteClipboard}
          class="flex items-center gap-1 text-[11px] text-[#8b9bb4] hover:text-[#f59e0b] transition-colors cursor-pointer"
        >
          <Clipboard class="w-3 h-3" />
          <span>Pegar Portapapeles</span>
        </button>
      </div>

      <!-- URL Input Line -->
      <div class="flex flex-col sm:flex-row gap-2">
        <div class="relative flex-1">
          <input
            type="text"
            placeholder="Pegar enlace de video, reel, post o imagen..."
            bind:value={urlInput}
            onkeydown={(e) => e.key === "Enter" && handleInspectUrl()}
            class="w-full bg-[#07090e] border border-[#1e263a] focus:border-[#f59e0b] rounded px-3.5 py-2.5 text-xs text-white placeholder-[#414d66] font-mono focus:outline-none transition-colors"
          />
        </div>
        <button
          onclick={handleInspectUrl}
          disabled={isLoadingInfo || !urlInput.trim()}
          class="px-4 py-2.5 rounded bg-[#161d2f] hover:bg-[#1e273f] active:bg-[#f59e0b] active:text-black border border-[#2a354f] hover:border-[#f59e0b]/60 disabled:opacity-30 text-white font-mono text-xs transition-colors flex items-center justify-center gap-2 cursor-pointer shrink-0"
        >
          {#if isLoadingInfo}
            <Loader2 class="w-3.5 h-3.5 animate-spin text-[#f59e0b]" />
            <span>Leyendo...</span>
          {:else}
            <span>Inspeccionar</span>
          {/if}
        </button>
      </div>

      {#if errorMessage}
        <div class="p-2.5 bg-[#250d11] border border-[#6b1e28] rounded text-[#f87171] text-xs flex items-center gap-2">
          <AlertCircle class="w-3.5 h-3.5 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      <!-- Media Inspector Details Card -->
      {#if currentPreview}
        <div class="p-3 bg-[#07090e] border border-[#1b2234] rounded flex flex-col sm:flex-row gap-3 items-start sm:items-center">
          {#if currentPreview.thumbnail}
            <img
              src={currentPreview.thumbnail}
              alt="Thumbnail"
              class="w-full sm:w-28 h-20 object-cover rounded bg-[#030407] border border-[#1b2234]"
            />
          {/if}
          <div class="flex-1 min-w-0 space-y-1">
            <h2 class="text-xs font-semibold text-white truncate font-sans" title={currentPreview.title}>
              {currentPreview.title}
            </h2>
            <div class="flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-[#5e6c87]">
              {#if currentPreview.uploader}
                <span>Canal: <strong class="text-[#8b9bb4]">{currentPreview.uploader}</strong></span>
              {/if}
              {#if currentPreview.duration}
                <span>Duración: <strong class="text-[#8b9bb4]">{formatDuration(currentPreview.duration)}</strong></span>
              {/if}
              {#if currentPreview.is_direct_image}
                <span class="text-[#f59e0b]">Modo: Imagen directa detectada</span>
              {/if}
            </div>
          </div>
        </div>
      {/if}

      <!-- Operational Controls Grid: Mode, Format, Path -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-1">
        <!-- Selector de Modo -->
        <div class="flex bg-[#07090e] p-0.5 rounded border border-[#1e263a]">
          <button
            onclick={() => { selectedMode = "video"; selectedQuality = "best"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "video" ? "bg-[#f59e0b] text-black font-bold" : "text-[#5e6c87] hover:text-[#d6deeb]"
            }`}
          >
            <Video class="w-3 h-3" />
            <span>Video</span>
          </button>
          <button
            onclick={() => { selectedMode = "audio"; selectedQuality = "mp3"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "audio" ? "bg-[#f59e0b] text-black font-bold" : "text-[#5e6c87] hover:text-[#d6deeb]"
            }`}
          >
            <Music class="w-3 h-3" />
            <span>Audio</span>
          </button>
          <button
            onclick={() => { selectedMode = "image"; selectedQuality = "original"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "image" ? "bg-[#f59e0b] text-black font-bold" : "text-[#5e6c87] hover:text-[#d6deeb]"
            }`}
          >
            <ImageIcon class="w-3 h-3" />
            <span>Foto</span>
          </button>
        </div>

        <!-- Selector de Calidad -->
        <div class="relative">
          <select
            bind:value={selectedQuality}
            class="w-full h-full bg-[#07090e] border border-[#1e263a] focus:border-[#f59e0b] rounded px-2.5 py-1.5 text-xs text-[#d6deeb] font-mono focus:outline-none"
          >
            {#if selectedMode === "video"}
              <option value="best">Máxima Calidad (Original)</option>
              <option value="1080p">1080p FHD</option>
              <option value="720p">720p HD</option>
            {:else if selectedMode === "audio"}
              <option value="mp3">MP3 320kbps</option>
              <option value="m4a">M4A (AAC)</option>
              <option value="flac">FLAC Lossless</option>
            {:else}
              <option value="original">Imagen Alta Resolución</option>
            {/if}
          </select>
        </div>

        <!-- Carpeta Destino -->
        <button
          onclick={selectFolder}
          class="flex items-center justify-between bg-[#07090e] border border-[#1e263a] hover:border-[#f59e0b]/50 px-2.5 py-1.5 rounded text-xs text-[#8b9bb4] transition-colors truncate cursor-pointer text-left"
        >
          <div class="flex items-center gap-1.5 truncate">
            <HardDrive class="w-3 h-3 text-[#f59e0b] shrink-0" />
            <span class="truncate">{downloadFolder}</span>
          </div>
          <span class="text-[10px] text-[#5e6c87] shrink-0 ml-1">Elegir</span>
        </button>
      </div>

      <!-- Action Button -->
      <div class="pt-1">
        <button
          onclick={handleDownload}
          disabled={!urlInput.trim()}
          class="w-full py-2.5 rounded bg-[#f59e0b] hover:bg-[#fbbf24] active:bg-[#d97706] disabled:opacity-20 text-black font-bold text-xs uppercase tracking-wider transition-colors flex items-center justify-center gap-2 cursor-pointer shadow-lg shadow-[#f59e0b]/10"
        >
          <Download class="w-3.5 h-3.5 stroke-[2.5]" />
          <span>Procesar y Descargar</span>
        </button>
      </div>
    </div>

    <!-- Active Tasks Console (Downloads List) -->
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs text-[#5e6c87] border-b border-[#141a29] pb-1.5">
        <span class="uppercase tracking-wider font-semibold">
          Cola de Descargas [{downloads.length}]
        </span>
        {#if downloads.length > 0}
          <button
            onclick={() => (downloads = [])}
            class="text-[11px] text-[#5e6c87] hover:text-[#d6deeb] transition-colors cursor-pointer"
          >
            Limpiar registro
          </button>
        {/if}
      </div>

      {#if downloads.length === 0}
        <div class="p-8 border border-dashed border-[#171e2e] rounded-lg flex flex-col items-center justify-center text-[#414d66] text-xs space-y-1">
          <Download class="w-6 h-6 stroke-1 opacity-40" />
          <span>Sin tareas activas. Pega un enlace arriba para procesar.</span>
        </div>
      {:else}
        <div class="space-y-2">
          {#each downloads as item (item.id)}
            <div class="bg-[#0b0e17] border border-[#1b2234] rounded p-3 transition-all space-y-2">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class="w-7 h-7 rounded bg-[#07090e] border border-[#1e263a] flex items-center justify-center shrink-0">
                    {#if item.mode === "audio"}
                      <Music class="w-3.5 h-3.5 text-[#34d399]" />
                    {:else if item.mode === "image"}
                      <ImageIcon class="w-3.5 h-3.5 text-[#f59e0b]" />
                    {:else}
                      <Video class="w-3.5 h-3.5 text-[#60a5fa]" />
                    {/if}
                  </div>
                  <div class="min-w-0">
                    <p class="text-xs font-medium text-white truncate font-sans">{item.title}</p>
                    <div class="flex items-center gap-2 text-[10px] text-[#5e6c87]">
                      <span class="uppercase text-[#8b9bb4]">{item.mode} // {item.quality}</span>
                      {#if item.speed}
                        <span>• {item.speed}</span>
                      {/if}
                      {#if item.eta}
                        <span>• ETA: {item.eta}</span>
                      {/if}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-2 shrink-0 text-xs">
                  {#if item.status === "downloading"}
                    <span class="font-bold text-[#f59e0b] font-mono">{item.percent.toFixed(0)}%</span>
                  {:else if item.status === "processing"}
                    <div class="flex items-center gap-1 text-[#f59e0b]">
                      <Loader2 class="w-3 h-3 animate-spin" />
                      <span class="text-[11px]">Remuxing...</span>
                    </div>
                  {:else if item.status === "finished"}
                    <div class="flex items-center gap-1 text-[#34d399]">
                      <CheckCircle2 class="w-3.5 h-3.5" />
                      <span class="text-[11px]">Guardado</span>
                    </div>
                  {:else if item.status === "error"}
                    <span class="text-[11px] text-[#f87171]" title={item.errorMsg}>Fallo</span>
                  {/if}
                </div>
              </div>

              {#if item.status === "downloading" || item.status === "processing"}
                <div class="w-full bg-[#07090e] border border-[#171e2e] rounded-full h-1 overflow-hidden">
                  <div
                    class="bg-[#f59e0b] h-full transition-all duration-300"
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
</div>
