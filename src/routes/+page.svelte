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
    Magnet,
    Check,
    AlertTriangle,
    Loader2,
    RefreshCw,
    Clipboard,
    HardDrive,
    ChevronDown,
    Activity,
    FolderOpen,
    ListMusic,
    Trash2,
    Search,
    X,
    ExternalLink
  } from "lucide-svelte";

  interface PlaylistItem {
    url: string;
    title: string;
  }

  interface MediaMetadata {
    title: string;
    thumbnail: string | null;
    duration: number | null;
    uploader: string | null;
    is_direct_image: boolean;
    is_playlist: boolean;
    playlist_count: number | null;
    playlist_items: PlaylistItem[] | null;
    formats_summary: string[];
  }

  interface SearchResult {
    id: string;
    title: string;
    uploader: string | null;
    duration: number | null;
    thumbnail: string | null;
    url: string;
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
    outputPath?: string;
  }

  let activeTab = $state<"url" | "search">("url");
  let urlInput = $state("");
  let searchQuery = $state("");
  let searchResults = $state<SearchResult[]>([]);
  let isSearching = $state(false);
  let selectedMode = $state<"video" | "audio" | "image">("video");
  let selectedQuality = $state("best");
  let downloadFolder = $state("~/Downloads");
  let isLoadingInfo = $state(false);
  let currentPreview = $state<MediaMetadata | null>(null);
  let errorMessage = $state("");
  let downloads = $state<DownloadItem[]>([]);
  let updateStatus = $state<string | null>(null);
  let isCheckingUpdate = $state(false);
  let toastMessage = $state<string | null>(null);
  let toastTimeout: any = null;

  function showToast(msg: string) {
    toastMessage = msg;
    if (toastTimeout) clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => {
      toastMessage = null;
    }, 3500);
  }

  function getDownloadState(url: string, mode: "audio" | "video") {
    return downloads.find((d) => d.url === url && d.mode === mode);
  }

  // Cargar historial persistente de LocalStorage
  function loadHistory() {
    try {
      const stored = localStorage.getItem("snapstream_history");
      if (stored) {
        downloads = JSON.parse(stored);
      }
      const savedDir = localStorage.getItem("snapstream_dir");
      if (savedDir) {
        downloadFolder = savedDir;
      }
    } catch (_) {}
  }

  function saveHistory() {
    try {
      localStorage.setItem("snapstream_history", JSON.stringify(downloads.slice(0, 30)));
    } catch (_) {}
  }

  function formatDuration(sec: number | null): string {
    if (!sec) return "";
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  }

  function isHttpUrl(str: string): boolean {
    return /^https?:\/\//i.test(str.trim());
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
    const raw = urlInput.trim();
    if (!raw) return;

    // Si el usuario escribe texto (nombre de canción/artista) en vez de URL, cambiar al buscador
    if (!isHttpUrl(raw)) {
      activeTab = "search";
      searchQuery = raw;
      handleSearch();
      return;
    }

    errorMessage = "";
    isLoadingInfo = true;
    currentPreview = null;

    try {
      const data = await invoke<MediaMetadata>("get_media_info", { url: raw });
      currentPreview = data;
      if (data.is_direct_image) {
        selectedMode = "image";
      } else if (raw.includes("music.youtube.com") || raw.includes("spotify") || raw.includes("soundcloud")) {
        selectedMode = "audio";
        selectedQuality = "mp3";
      }
    } catch (err: any) {
      errorMessage = err?.toString() || "Error al obtener información del enlace.";
    } finally {
      isLoadingInfo = false;
    }
  }

  async function handleSearch() {
    const q = searchQuery.trim();
    if (!q) return;
    errorMessage = "";
    isSearching = true;
    searchResults = [];

    try {
      const results = await invoke<SearchResult[]>("search_music", { query: q });
      searchResults = results;
      if (results.length === 0) {
        errorMessage = `No se encontraron resultados para "${q}".`;
      }
    } catch (err: any) {
      errorMessage = err?.toString() || "Error al realizar la búsqueda de música.";
    } finally {
      isSearching = false;
    }
  }

  function handleSearchInputKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      const q = searchQuery.trim();
      if (isHttpUrl(q)) {
        activeTab = "url";
        urlInput = q;
        handleInspectUrl();
      } else {
        handleSearch();
      }
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
        localStorage.setItem("snapstream_dir", selected);
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function openInFolder(item: DownloadItem) {
    try {
      const target = item.outputPath || downloadFolder;
      await invoke("open_path_in_file_manager", { path: target });
    } catch (e) {
      console.error(e);
    }
  }

  async function handleDownload() {
    if (!urlInput.trim()) return;

    // Si es una playlist, descargar todos los elementos detectados
    if (currentPreview?.is_playlist && currentPreview.playlist_items?.length) {
      const items = currentPreview.playlist_items;
      for (const item of items) {
        triggerSingleDownload(item.url, item.title, currentPreview.thumbnail);
      }
      return;
    }

    const title = currentPreview?.title || "recurso_multimedia";
    const thumb = currentPreview?.thumbnail || null;
    triggerSingleDownload(urlInput.trim(), title, thumb);
  }

  async function triggerSingleDownload(
    url: string,
    title: string,
    thumbnail: string | null,
    overrideMode?: "video" | "audio" | "image",
    overrideQuality?: string
  ) {
    const mode = overrideMode || selectedMode;
    const quality = overrideQuality || (mode === "audio" ? "mp3" : selectedQuality);
    const id = Date.now().toString() + Math.random().toString(36).substring(2, 5);

    const newItem: DownloadItem = {
      id,
      url,
      title,
      thumbnail,
      mode,
      quality,
      percent: 0,
      speed: "0.0 KB/s",
      eta: "--:--",
      status: "queued"
    };

    downloads = [newItem, ...downloads];
    saveHistory();
    const shortTitle = title.length > 38 ? title.substring(0, 38) + "..." : title;
    showToast(`Iniciando descarga: ${shortTitle} [${mode.toUpperCase()}]`);

    try {
      await invoke("start_download", {
        id,
        url,
        outputDir: downloadFolder,
        mode,
        quality,
        thumbnailUrl: thumbnail
      });
    } catch (err: any) {
      const itm = downloads.find((d) => d.id === id);
      if (itm) {
        itm.status = "error";
        itm.errorMsg = err?.toString();
        saveHistory();
        showToast(`✗ Error: ${err?.toString() || "fallo al iniciar descarga"}`);
      }
    }
  }

  function clearHistory() {
    downloads = [];
    localStorage.removeItem("snapstream_history");
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
    loadHistory();

    const unlisten = listen<any>("download-progress", (event) => {
      const payload = event.payload;
      const item = downloads.find((d) => d.id === payload.id);
      if (item) {
        item.percent = payload.percent;
        item.speed = payload.speed;
        item.eta = payload.eta;
        item.status = payload.status;
        if (payload.output_path) {
          item.outputPath = payload.output_path;
        }
        if (payload.filename) {
          item.errorMsg = payload.filename;
        }
        if (payload.status === "finished") {
          saveHistory();
          const shortTitle = item.title.length > 35 ? item.title.substring(0, 35) + "..." : item.title;
          showToast(`✓ Descarga completada: ${shortTitle}`);
        } else if (payload.status === "error") {
          saveHistory();
          showToast(`✗ Error al descargar: ${item.title}`);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<div class="flex-1 flex flex-col w-full h-full min-h-screen bg-[#090a0f] text-[#d4d4d8] font-mono selection:bg-[#e4e4e7] selection:text-black">
  <div class="flex-1 flex flex-col max-w-4xl w-full mx-auto p-4 sm:p-6 space-y-4">
    
    <!-- Header -->
    <header class="flex items-center justify-between pb-3 border-b border-[#27272a]">
      <div class="flex items-center gap-2.5">
        <div class="w-7 h-7 rounded border border-[#3f3f46] bg-[#18181b] flex items-center justify-center text-white">
          <Magnet class="w-3.5 h-3.5 stroke-[2.2]" />
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
      <!-- Tabs / Mode Bar -->
      <div class="flex items-center justify-between border-b border-[#27272a] pb-2.5">
        <div class="flex items-center gap-2">
          <button
            onclick={() => activeTab = "url"}
            class={`flex items-center gap-1.5 px-3 py-1.5 text-xs rounded transition-all cursor-pointer font-mono ${
              activeTab === "url"
                ? "bg-[#18181b] border border-[#3f3f46] text-white font-bold"
                : "text-[#71717a] hover:text-[#d4d4d8]"
            }`}
          >
            <Magnet class="w-3 h-3" />
            <span>$ enlace_url</span>
          </button>
          <button
            onclick={() => activeTab = "search"}
            class={`flex items-center gap-1.5 px-3 py-1.5 text-xs rounded transition-all cursor-pointer font-mono ${
              activeTab === "search"
                ? "bg-[#18181b] border border-[#3f3f46] text-white font-bold"
                : "text-[#71717a] hover:text-[#d4d4d8]"
            }`}
          >
            <Search class="w-3 h-3" />
            <span>$ buscador_musica</span>
          </button>
        </div>

        {#if activeTab === "url"}
          <button
            onclick={pasteClipboard}
            class="flex items-center gap-1 text-[11px] text-[#a1a1aa] hover:text-white border-b border-transparent hover:border-[#a1a1aa] pb-0.5 transition-all cursor-pointer"
          >
            <Clipboard class="w-3 h-3" />
            <span>pegar_clipboard</span>
          </button>
        {:else}
          <div class="flex items-center gap-1.5 text-[11px] text-[#71717a]">
            <span>destino:</span>
            <button
              onclick={selectFolder}
              class="text-[#a1a1aa] hover:text-white truncate max-w-[140px] sm:max-w-[200px] cursor-pointer"
              title="Cambiar carpeta de destino"
            >
              {downloadFolder}
            </button>
          </div>
        {/if}
      </div>

      {#if activeTab === "url"}
        <!-- URL Mode Content -->
        <div class="space-y-3.5">
          <div class="flex items-center justify-between text-[11px] text-[#71717a]">
            <div class="flex items-center gap-1.5">
              <span class="text-white font-bold">$</span>
              <span>input_url --extract</span>
            </div>
            <span class="text-[10px] text-[#52525b]">YouTube, YT Music, Spotify, TikTok, Twitter...</span>
          </div>

          <div class="flex flex-col sm:flex-row gap-2">
            <div class="relative flex-1">
              <input
                type="text"
                placeholder="https://... o escribe canción para buscar"
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
              {:else if urlInput.trim() && !isHttpUrl(urlInput.trim())}
                <Search class="w-3.5 h-3.5" />
                <span>buscar música</span>
              {:else}
                <span>inspeccionar</span>
              {/if}
            </button>
          </div>

          <!-- Media Preview Card -->
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
                <div class="flex items-center gap-2">
                  {#if currentPreview.is_playlist}
                    <ListMusic class="w-3.5 h-3.5 text-white shrink-0" />
                  {/if}
                  <h2 class="text-xs font-bold text-white truncate" title={currentPreview.title}>
                    {currentPreview.title}
                  </h2>
                </div>
                <div class="flex flex-wrap gap-x-3 gap-y-0.5 text-[11px] text-[#71717a]">
                  {#if currentPreview.is_playlist}
                    <span class="text-white">[Playlist: {currentPreview.playlist_count || currentPreview.playlist_items?.length} items]</span>
                  {/if}
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

          <!-- Controls -->
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-1">
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
                  <option value="mp3" class="bg-[#10121a] text-white">MP3 320kbps + ID3</option>
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

          <div class="pt-1">
            <button
              onclick={handleDownload}
              disabled={!urlInput.trim()}
              class="w-full py-2.5 rounded bg-white hover:bg-[#e4e4e7] active:bg-[#d4d4d8] disabled:opacity-20 text-black font-bold text-xs uppercase tracking-wider transition-colors flex items-center justify-center gap-2 cursor-pointer"
            >
              <Download class="w-3.5 h-3.5 stroke-[2.5]" />
              <span>{currentPreview?.is_playlist ? `descargar playlist (${currentPreview.playlist_items?.length || 'todas'})` : 'ejecutar descarga'}</span>
            </button>
          </div>
        </div>
      {:else}
        <!-- Search Mode Content -->
        <div class="space-y-3.5">
          <div class="flex items-center justify-between text-[11px] text-[#71717a]">
            <div class="flex items-center gap-1.5">
              <span class="text-white font-bold">$</span>
              <span>ytsearch --music --tags --navidrome</span>
            </div>
            <button
              onclick={selectFolder}
              class="flex items-center gap-1 text-[11px] text-[#a1a1aa] hover:text-white transition-colors cursor-pointer"
              title="Directorio de destino para descargas"
            >
              <Folder class="w-3 h-3 text-[#71717a]" />
              <span class="truncate max-w-[160px]">{downloadFolder}</span>
            </button>
          </div>

          <div class="flex flex-col sm:flex-row gap-2">
            <div class="relative flex-1">
              <input
                type="text"
                placeholder="Escribe canción, artista o álbum (ej: Loser Tame Impala)..."
                bind:value={searchQuery}
                onkeydown={handleSearchInputKeydown}
                class="w-full bg-[#090a0f] border border-[#27272a] focus:border-[#71717a] focus:ring-1 focus:ring-[#71717a] rounded px-3.5 py-2.5 text-xs text-white placeholder-[#52525b] font-mono focus:outline-none transition-colors"
              />
            </div>
            <button
              onclick={handleSearch}
              disabled={isSearching || !searchQuery.trim()}
              class="px-4 py-2.5 rounded bg-white hover:bg-[#e4e4e7] active:bg-[#d4d4d8] text-black font-mono font-bold text-xs transition-colors flex items-center justify-center gap-2 cursor-pointer disabled:opacity-30 shrink-0"
            >
              {#if isSearching}
                <Loader2 class="w-3.5 h-3.5 animate-spin text-black" />
                <span>buscando</span>
              {:else}
                <Search class="w-3.5 h-3.5 stroke-[2.5]" />
                <span>buscar</span>
              {/if}
            </button>
          </div>

          <!-- Quick examples if no results -->
          {#if searchResults.length === 0 && !isSearching && !errorMessage}
            <div class="flex items-center flex-wrap gap-2 text-[11px] text-[#71717a] pt-0.5">
              <span>ejemplos:</span>
              <button
                onclick={() => { searchQuery = "Loser Tame Impala"; handleSearch(); }}
                class="text-[#a1a1aa] hover:text-white underline decoration-dotted cursor-pointer"
              >
                Loser Tame Impala
              </button>
              <span>•</span>
              <button
                onclick={() => { searchQuery = "Daft Punk Instant Crush"; handleSearch(); }}
                class="text-[#a1a1aa] hover:text-white underline decoration-dotted cursor-pointer"
              >
                Instant Crush
              </button>
              <span>•</span>
              <button
                onclick={() => { searchQuery = "Crimen Gustavo Cerati"; handleSearch(); }}
                class="text-[#a1a1aa] hover:text-white underline decoration-dotted cursor-pointer"
              >
                Crimen Cerati
              </button>
            </div>
          {/if}

          <!-- Search Results List -->
          {#if searchResults.length > 0}
            <div class="space-y-2 pt-1">
              <div class="flex items-center justify-between text-xs text-[#71717a] border-b border-[#27272a] pb-1.5">
                <span class="font-bold text-[#a1a1aa]">
                  // resultados [{searchResults.length}] — 1-click descarga con carátula y tags ID3
                </span>
                <button
                  onclick={() => searchResults = []}
                  class="flex items-center gap-1 text-[11px] text-[#71717a] hover:text-white transition-colors cursor-pointer"
                >
                  <X class="w-3 h-3" />
                  <span>cerrar</span>
                </button>
              </div>

              <div class="space-y-1.5 max-h-[400px] overflow-y-auto pr-1">
                {#each searchResults as item (item.id)}
                  {@const dlAudio = getDownloadState(item.url, "audio")}
                  {@const dlVideo = getDownloadState(item.url, "video")}
                  {@const activeDl = (dlAudio && (dlAudio.status === "downloading" || dlAudio.status === "processing" || dlAudio.status === "queued")) ? dlAudio : ((dlVideo && (dlVideo.status === "downloading" || dlVideo.status === "processing" || dlVideo.status === "queued")) ? dlVideo : null)}
                  <div class={`p-2.5 bg-[#090a0f] hover:bg-[#141620] border rounded flex flex-col gap-2 transition-all ${
                    activeDl ? "border-[#52525b] shadow-sm shadow-white/5" : "border-[#27272a] hover:border-[#3f3f46]"
                  }`}>
                    <div class="flex flex-col sm:flex-row gap-3 items-start sm:items-center justify-between w-full">
                    <div class="flex items-center gap-3 min-w-0 flex-1">
                      {#if item.thumbnail}
                        <img
                          src={item.thumbnail}
                          alt={item.title}
                          class="w-16 h-12 object-cover rounded bg-black border border-[#27272a] shrink-0"
                          loading="lazy"
                        />
                      {:else}
                        <div class="w-16 h-12 rounded bg-[#18181b] border border-[#27272a] flex items-center justify-center shrink-0">
                          <Music class="w-4 h-4 text-[#71717a]" />
                        </div>
                      {/if}
                      <div class="min-w-0 flex-1 space-y-0.5">
                        <p class="text-xs font-bold text-white truncate" title={item.title}>
                          {item.title}
                        </p>
                        <div class="flex items-center gap-2 text-[11px] text-[#71717a]">
                          {#if item.uploader}
                            <span class="truncate max-w-[160px] sm:max-w-[220px] text-[#a1a1aa] font-medium">{item.uploader}</span>
                          {/if}
                          {#if item.duration}
                            <span>• {formatDuration(item.duration)}</span>
                          {/if}
                        </div>
                      </div>
                    </div>

                    <div class="flex items-center gap-1.5 shrink-0 self-end sm:self-center">
                      <!-- MP3 Button -->
                      {#if dlAudio}
                        {#if dlAudio.status === "downloading"}
                          <div class="flex items-center gap-1.5 text-[11px] bg-white text-black font-bold px-2.5 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-black" />
                            <span>{dlAudio.percent.toFixed(0)}%</span>
                          </div>
                        {:else if dlAudio.status === "processing"}
                          <div class="flex items-center gap-1.5 text-[11px] bg-white text-black font-bold px-2.5 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-black" />
                            <span>tags ID3</span>
                          </div>
                        {:else if dlAudio.status === "finished"}
                          <button
                            onclick={() => openInFolder(dlAudio)}
                            class="flex items-center gap-1 text-[11px] bg-white hover:bg-[#e4e4e7] text-black font-bold px-2.5 py-1.5 rounded transition-all cursor-pointer"
                            title="MP3 descargado. Clic para abrir en carpeta."
                          >
                            <Check class="w-3 h-3 stroke-[3]" />
                            <span>¡Listo!</span>
                          </button>
                        {:else if dlAudio.status === "queued"}
                          <div class="flex items-center gap-1.5 text-[11px] bg-[#27272a] text-[#d4d4d8] px-2.5 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-white" />
                            <span>en cola</span>
                          </div>
                        {:else if dlAudio.status === "error"}
                          <button
                            onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "audio", "mp3")}
                            class="flex items-center gap-1 text-[11px] bg-red-950 border border-red-800 text-red-200 px-2 py-1 rounded cursor-pointer"
                            title={dlAudio.errorMsg || "Error al descargar"}
                          >
                            <AlertTriangle class="w-3 h-3" />
                            <span>reintentar</span>
                          </button>
                        {/if}
                      {:else}
                        <button
                          onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "audio", "mp3")}
                          class="flex items-center gap-1.5 text-[11px] bg-white hover:bg-[#e4e4e7] active:scale-95 text-black font-bold px-2.5 py-1.5 rounded transition-all cursor-pointer"
                          title="Descargar MP3 con ID3 tags y carátula para Navidrome"
                        >
                          <Music class="w-3 h-3 stroke-[2.5]" />
                          <span>MP3</span>
                        </button>
                      {/if}

                      <!-- Video Button -->
                      {#if dlVideo}
                        {#if dlVideo.status === "downloading"}
                          <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-white px-2 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-white" />
                            <span>{dlVideo.percent.toFixed(0)}%</span>
                          </div>
                        {:else if dlVideo.status === "processing"}
                          <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-white px-2 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-white" />
                            <span>remux</span>
                          </div>
                        {:else if dlVideo.status === "finished"}
                          <button
                            onclick={() => openInFolder(dlVideo)}
                            class="flex items-center gap-1 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-white px-2 py-1.5 rounded cursor-pointer"
                            title="Video descargado. Clic para abrir"
                          >
                            <Check class="w-3 h-3 stroke-[3]" />
                            <span>Listo</span>
                          </button>
                        {:else if dlVideo.status === "queued"}
                          <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-[#a1a1aa] px-2 py-1.5 rounded font-mono">
                            <Loader2 class="w-3 h-3 animate-spin text-white" />
                            <span>cola</span>
                          </div>
                        {:else if dlVideo.status === "error"}
                          <button
                            onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "video", "best")}
                            class="flex items-center gap-1 text-[11px] bg-red-950 border border-red-800 text-red-200 px-2 py-1 rounded cursor-pointer"
                          >
                            <AlertTriangle class="w-3 h-3" />
                            <span>reintentar</span>
                          </button>
                        {/if}
                      {:else}
                        <button
                          onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "video", "best")}
                          class="flex items-center gap-1.5 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#d4d4d8] hover:text-white px-2.5 py-1.5 rounded transition-all cursor-pointer active:scale-95"
                          title="Descargar Video MP4"
                        >
                          <Video class="w-3 h-3" />
                          <span>Video</span>
                        </button>
                      {/if}

                      <button
                        onclick={() => {
                          activeTab = "url";
                          urlInput = item.url;
                          handleInspectUrl();
                        }}
                        class="text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#27272a] hover:border-[#3f3f46] text-[#71717a] hover:text-white p-1.5 rounded transition-colors cursor-pointer"
                        title="Inspeccionar enlace"
                      >
                        <ExternalLink class="w-3 h-3" />
                      </button>
                    </div>
                  </div>

                  <!-- Mini progress bar inside card -->
                  {#if activeDl && (activeDl.status === "downloading" || activeDl.status === "processing" || activeDl.status === "queued")}
                    <div class="w-full pt-1 px-1">
                      <div class="flex items-center justify-between text-[10px] text-[#a1a1aa] mb-1 font-mono">
                        <span>
                          {activeDl.status === "processing"
                            ? (activeDl.mode === "audio" ? "incrustando carátula y tags ID3..." : "procesando...")
                            : activeDl.status === "queued"
                            ? "en cola..."
                            : `descargando: ${activeDl.percent.toFixed(0)}%`}
                        </span>
                        <span>{activeDl.speed ? `${activeDl.speed} ` : ""}{activeDl.eta ? `• ETA: ${activeDl.eta}` : ""}</span>
                      </div>
                      <div class="w-full bg-[#18181b] border border-[#27272a] rounded-full h-1 overflow-hidden">
                        <div
                          class="bg-white h-full transition-all duration-300"
                          style="width: {activeDl.status === 'processing' ? '99%' : `${activeDl.percent}%`}"
                        ></div>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      {#if errorMessage}
        <div class="p-2.5 bg-[#1c1917] border border-[#44403c] rounded text-[#fca5a5] text-xs flex items-center gap-2">
          <AlertTriangle class="w-3.5 h-3.5 shrink-0" />
          <span>{errorMessage}</span>
        </div>
      {/if}
    </div>

    <!-- Active Tasks & Persistent History -->
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs text-[#71717a] border-b border-[#27272a] pb-1.5">
        <span class="font-bold text-[#a1a1aa]">
          // cola_procesos [{downloads.length}]
        </span>
        {#if downloads.length > 0}
          <button
            onclick={clearHistory}
            class="flex items-center gap-1 text-[11px] text-[#71717a] hover:text-white transition-colors cursor-pointer"
          >
            <Trash2 class="w-3 h-3" />
            <span>limpiar</span>
          </button>
        {/if}
      </div>

      {#if downloads.length === 0}
        <div class="p-6 border border-dashed border-[#27272a] rounded flex flex-col items-center justify-center text-[#52525b] text-xs space-y-1">
          <Magnet class="w-5 h-5 opacity-40 mb-1" />
          <span>esperando enlaces... pega uno arriba</span>
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
                    <div class="flex items-center gap-1.5">
                      <button
                        onclick={() => openInFolder(item)}
                        class="flex items-center gap-1 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#d4d4d8] hover:text-white px-2 py-0.5 rounded transition-colors cursor-pointer"
                        title="Abrir archivo o carpeta"
                      >
                        <FolderOpen class="w-3 h-3" />
                        <span>abrir</span>
                      </button>
                      <div class="flex items-center gap-0.5 text-white">
                        <Check class="w-3.5 h-3.5 stroke-[3]" />
                      </div>
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

    {#if toastMessage}
      <div class="fixed bottom-5 right-5 z-50 flex items-center gap-2.5 bg-[#18181b] border border-[#3f3f46] text-white px-3.5 py-2.5 rounded shadow-2xl text-xs font-mono">
        <Activity class="w-3.5 h-3.5 text-white animate-pulse shrink-0" />
        <span class="max-w-[340px] truncate">{toastMessage}</span>
      </div>
    {/if}

  </div>
</div>
