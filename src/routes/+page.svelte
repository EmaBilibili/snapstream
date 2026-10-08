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
    Terminal,
    Check,
    AlertTriangle,
    Loader2,
    RefreshCw,
    Clipboard,
    HardDrive,
    ChevronDown,
    Activity
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
        title: "Seleccionar directorio de destino"
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
    const title = currentPreview?.title || "recurso_multimedia";
    const thumbnail = currentPreview?.thumbnail || null;

    const newItem: DownloadItem = {
      id,
      url: urlInput.trim(),
      title: String(title),
      thumbnail,
      mode: selectedMode,
      quality: selectedQuality,
      percent: 0,
      speed: "0.0 KB/s",
      eta: "--:--",
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
      updateStatus = "comprobando...";
      const update = await check();
      if (update?.available) {
        updateStatus = `disponible: v${update.version}`;
        await update.downloadAndInstall();
        updateStatus = "instalado: reinicia";
      } else {
        updateStatus = "actualizado";
        setTimeout(() => (updateStatus = null), 3000);
      }
    } catch (e: any) {
      updateStatus = "error_red";
      setTimeout(() => (updateStatus = null), 3000);
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

<!-- Terminal Shell Container: Puro Blanco, Negro y Escala de Grises Monocromática -->
<div class="flex-1 flex flex-col w-full h-full min-h-screen bg-[#090a0f] text-[#d4d4d8] font-mono selection:bg-[#e4e4e7] selection:text-black">
  
  <div class="flex-1 flex flex-col max-w-4xl w-full mx-auto p-4 sm:p-6 space-y-4">
    
    <!-- Header: Terminal Window Bar -->
    <header class="flex items-center justify-between pb-3 border-b border-[#27272a]">
      <div class="flex items-center gap-2.5">
        <div class="w-7 h-7 rounded border border-[#3f3f46] bg-[#18181b] flex items-center justify-center text-white">
          <Terminal class="w-3.5 h-3.5 stroke-[2.2]" />
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs font-bold tracking-wider text-white">snapstream</span>
          <span class="text-[10px] text-[#71717a] font-normal">// v0.1.0</span>
        </div>
      </div>

      <div class="flex items-center gap-2">
        {#if updateStatus}
          <div class="flex items-center gap-1.5 text-[11px] px-2 py-0.5 rounded bg-[#18181b] border border-[#3f3f46] text-[#e4e4e7]">
            <Activity class="w-3 h-3 animate-pulse" />
            <span>[{updateStatus}]</span>
          </div>
        {/if}
        <button
          onclick={checkForUpdates}
          disabled={isCheckingUpdate}
          class="flex items-center gap-1.5 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#a1a1aa] hover:text-white px-2.5 py-1 rounded transition-colors cursor-pointer"
          title="Verificar actualizaciones"
        >
          <RefreshCw class={`w-3 h-3 ${isCheckingUpdate ? 'animate-spin' : ''}`} />
          <span>sync</span>
        </button>
      </div>
    </header>

    <!-- Console Input Panel -->
    <div class="bg-[#10121a] border border-[#27272a] rounded p-4 space-y-3.5">
      
      <!-- Prompt Title & Paste -->
      <div class="flex items-center justify-between text-[11px] text-[#71717a]">
        <div class="flex items-center gap-1.5">
          <span class="text-white font-bold">$</span>
          <span>input_url --extract</span>
        </div>
        <button
          onclick={pasteClipboard}
          class="flex items-center gap-1 text-[11px] text-[#a1a1aa] hover:text-white border-b border-transparent hover:border-[#a1a1aa] pb-0.5 transition-all cursor-pointer"
        >
          <Clipboard class="w-3 h-3" />
          <span>pegar_clipboard</span>
        </button>
      </div>

      <!-- Input Field -->
      <div class="flex flex-col sm:flex-row gap-2">
        <div class="relative flex-1">
          <input
            type="text"
            placeholder="https://..."
            bind:value={urlInput}
            onkeydown={(e) => e.key === "Enter" && handleInspectUrl()}
            class="w-full bg-[#090a0f] border border-[#27272a] focus:border-[#71717a] focus:ring-1 focus:ring-[#71717a] rounded px-3.5 py-2.5 text-xs text-white placeholder-[#52525b] font-mono focus:outline-none transition-colors"
          />
        </div>
        <button
          onclick={handleInspectUrl}
          disabled={isLoadingInfo || !urlInput.trim()}
          class="px-4 py-2.5 rounded bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] hover:border-white text-white font-mono text-xs transition-colors flex items-center justify-center gap-2 cursor-pointer disabled:opacity-30 shrink-0"
        >
          {#if isLoadingInfo}
            <Loader2 class="w-3.5 h-3.5 animate-spin" />
            <span>analizando</span>
          {:else}
            <span>inspeccionar</span>
          {/if}
        </button>
      </div>

      {#if errorMessage}
        <div class="p-2.5 bg-[#1c1917] border border-[#44403c] rounded text-[#fca5a5] text-xs flex items-center gap-2">
          <AlertTriangle class="w-3.5 h-3.5 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}

      <!-- Media Preview -->
      {#if currentPreview}
        <div class="p-3 bg-[#090a0f] border border-[#27272a] rounded flex flex-col sm:flex-row gap-3 items-start sm:items-center">
          {#if currentPreview.thumbnail}
            <img
              src={currentPreview.thumbnail}
              alt="Thumbnail"
              class="w-full sm:w-28 h-20 object-cover rounded bg-black border border-[#27272a] grayscale hover:grayscale-0 transition-all duration-300"
            />
          {/if}
          <div class="flex-1 min-w-0 space-y-1">
            <h2 class="text-xs font-bold text-white truncate" title={currentPreview.title}>
              {currentPreview.title}
            </h2>
            <div class="flex flex-wrap gap-x-3 gap-y-0.5 text-[11px] text-[#71717a]">
              {#if currentPreview.uploader}
                <span>autor: <strong class="text-[#d4d4d8] font-normal">{currentPreview.uploader}</strong></span>
              {/if}
              {#if currentPreview.duration}
                <span>duración: <strong class="text-[#d4d4d8] font-normal">{formatDuration(currentPreview.duration)}</strong></span>
              {/if}
              {#if currentPreview.is_direct_image}
                <span class="text-white font-semibold">[imagen detectada]</span>
              {/if}
            </div>
          </div>
        </div>
      {/if}

      <!-- Command Controls: Mode / Quality / Path -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-1">
        
        <!-- Mode Switcher -->
        <div class="flex bg-[#090a0f] p-0.5 rounded border border-[#27272a]">
          <button
            onclick={() => { selectedMode = "video"; selectedQuality = "best"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "video" ? "bg-white text-black font-bold" : "text-[#71717a] hover:text-white"
            }`}
          >
            <Video class="w-3 h-3" />
            <span>video</span>
          </button>
          <button
            onclick={() => { selectedMode = "audio"; selectedQuality = "mp3"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "audio" ? "bg-white text-black font-bold" : "text-[#71717a] hover:text-white"
            }`}
          >
            <Music class="w-3 h-3" />
            <span>audio</span>
          </button>
          <button
            onclick={() => { selectedMode = "image"; selectedQuality = "original"; }}
            class={`flex-1 flex items-center justify-center gap-1.5 py-1.5 text-xs rounded transition-all cursor-pointer ${
              selectedMode === "image" ? "bg-white text-black font-bold" : "text-[#71717a] hover:text-white"
            }`}
          >
            <ImageIcon class="w-3 h-3" />
            <span>foto</span>
          </button>
        </div>

        <!-- Quality Selector (Corregido fondo oscuro y texto nítido) -->
        <div class="relative flex items-center">
          <select
            bind:value={selectedQuality}
            class="w-full h-full bg-[#090a0f]! text-white! border border-[#27272a] focus:border-[#71717a] rounded px-3 py-1.5 text-xs font-mono focus:outline-none appearance-none cursor-pointer"
          >
            {#if selectedMode === "video"}
              <option value="best" class="bg-[#10121a] text-white">Máxima Calidad (Original)</option>
              <option value="1080p" class="bg-[#10121a] text-white">1080p FHD</option>
              <option value="720p" class="bg-[#10121a] text-white">720p HD</option>
            {:else if selectedMode === "audio"}
              <option value="mp3" class="bg-[#10121a] text-white">MP3 320kbps</option>
              <option value="m4a" class="bg-[#10121a] text-white">M4A (AAC)</option>
              <option value="flac" class="bg-[#10121a] text-white">FLAC Lossless</option>
            {:else}
              <option value="original" class="bg-[#10121a] text-white">Imagen Alta Resolución</option>
            {/if}
          </select>
          <div class="pointer-events-none absolute right-2.5 flex items-center text-[#71717a]">
            <ChevronDown class="w-3.5 h-3.5" />
          </div>
        </div>

        <!-- Destination Folder Button -->
        <button
          onclick={selectFolder}
          class="flex items-center justify-between bg-[#090a0f] border border-[#27272a] hover:border-[#71717a] px-3 py-1.5 rounded text-xs text-[#a1a1aa] hover:text-white transition-colors truncate cursor-pointer text-left"
        >
          <div class="flex items-center gap-1.5 truncate">
            <HardDrive class="w-3 h-3 text-[#71717a] shrink-0" />
            <span class="truncate">{downloadFolder}</span>
          </div>
          <span class="text-[10px] text-[#71717a] shrink-0 ml-1">dir</span>
        </button>
      </div>

      <!-- Download Button -->
      <div class="pt-1">
        <button
          onclick={handleDownload}
          disabled={!urlInput.trim()}
          class="w-full py-2.5 rounded bg-white hover:bg-[#e4e4e7] active:bg-[#d4d4d8] disabled:opacity-20 text-black font-bold text-xs uppercase tracking-wider transition-colors flex items-center justify-center gap-2 cursor-pointer"
        >
          <Download class="w-3.5 h-3.5 stroke-[2.5]" />
          <span>ejecutar descarga</span>
        </button>
      </div>
    </div>

    <!-- Active Tasks Terminal Output -->
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs text-[#71717a] border-b border-[#27272a] pb-1.5">
        <span class="font-bold text-[#a1a1aa]">
          // cola_procesos [{downloads.length}]
        </span>
        {#if downloads.length > 0}
          <button
            onclick={() => (downloads = [])}
            class="text-[11px] text-[#71717a] hover:text-white transition-colors cursor-pointer"
          >
            clear
          </button>
        {/if}
      </div>

      {#if downloads.length === 0}
        <div class="p-6 border border-dashed border-[#27272a] rounded flex flex-col items-center justify-center text-[#52525b] text-xs space-y-1">
          <Terminal class="w-5 h-5 opacity-40 mb-1" />
          <span>esperando comandos... pega un enlace arriba</span>
        </div>
      {:else}
        <div class="space-y-1.5">
          {#each downloads as item (item.id)}
            <div class="bg-[#10121a] border border-[#27272a] rounded p-2.5 transition-all space-y-2">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class="w-6 h-6 rounded bg-[#18181b] border border-[#27272a] flex items-center justify-center shrink-0">
                    {#if item.mode === "audio"}
                      <Music class="w-3 h-3 text-[#d4d4d8]" />
                    {:else if item.mode === "image"}
                      <ImageIcon class="w-3 h-3 text-[#d4d4d8]" />
                    {:else}
                      <Video class="w-3 h-3 text-[#d4d4d8]" />
                    {/if}
                  </div>
                  <div class="min-w-0">
                    <p class="text-xs font-medium text-white truncate">{item.title}</p>
                    <div class="flex items-center gap-2 text-[10px] text-[#71717a]">
                      <span class="text-[#a1a1aa]">{item.mode} // {item.quality}</span>
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
                    <span class="font-bold text-white font-mono">{item.percent.toFixed(0)}%</span>
                  {:else if item.status === "processing"}
                    <div class="flex items-center gap-1 text-white">
                      <Loader2 class="w-3 h-3 animate-spin" />
                      <span class="text-[11px]">remux</span>
                    </div>
                  {:else if item.status === "finished"}
                    <div class="flex items-center gap-1 text-white">
                      <Check class="w-3.5 h-3.5 stroke-[3]" />
                      <span class="text-[11px]">completado</span>
                    </div>
                  {:else if item.status === "error"}
                    <span class="text-[11px] text-[#fca5a5]" title={item.errorMsg}>error</span>
                  {/if}
                </div>
              </div>

              {#if item.status === "downloading" || item.status === "processing"}
                <div class="w-full bg-[#090a0f] border border-[#27272a] rounded-full h-1 overflow-hidden">
                  <div
                    class="bg-white h-full transition-all duration-300"
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
