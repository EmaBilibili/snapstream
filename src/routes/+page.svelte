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
    ExternalLink,
    Monitor,
    Smartphone,
    Wifi,
    Radio,
    Server,
    Copy
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

  interface ServerInfo {
    ip: string;
    port: number;
    url: string;
    is_running: boolean;
  }

  let serverInfo = $state<ServerInfo | null>(null);
  let downloadTarget = $state<"local" | "remote">("local");
  let remotePcIp = $state("");
  let remoteConnected = $state<boolean | null>(null);
  let isTestingRemote = $state(false);
  let showRemoteModal = $state(false);
  let showServerInfoModal = $state(false);
  let copiedServerIp = $state(false);
  let isSendingRemote = $state(false);

  function setDownloadTarget(target: "local" | "remote") {
    downloadTarget = target;
    try {
      localStorage.setItem("snapstream_download_target", target);
    } catch (_) {}
    if (target === "remote" && !remotePcIp.trim()) {
      showRemoteModal = true;
    }
  }

  async function testRemoteConnection(ipToTest?: string) {
    const rawIp = (ipToTest || remotePcIp).trim();
    if (!rawIp) {
      showToast("⚠️ Ingresa una dirección IP o dominio primero");
      return false;
    }
    const cleanTarget = rawIp.replace(/\/+$/, "");
    isTestingRemote = true;
    try {
      const res = await invoke<any>("ping_remote_server", { pcIp: cleanTarget });
      remoteConnected = true;
      remotePcIp = cleanTarget;
      try {
        localStorage.setItem("snapstream_remote_ip", cleanTarget);
      } catch (_) {}
      showToast(`✓ Conectado a PC (${cleanTarget}): SnapStream v${res.version || "0.1.0"}`);
      return true;
    } catch (err: any) {
      remoteConnected = false;
      showToast(`✗ Error conectando a ${cleanTarget}`);
      return false;
    } finally {
      isTestingRemote = false;
    }
  }

  async function copyServerIpToClipboard() {
    if (!serverInfo?.ip) return;
    try {
      await navigator.clipboard.writeText(serverInfo.ip);
      copiedServerIp = true;
      showToast(`✓ IP copiada al portapapeles: ${serverInfo.ip}`);
      setTimeout(() => (copiedServerIp = false), 2500);
    } catch (_) {}
  }

  async function sendRemoteDownload(
    url: string,
    title: string,
    mode: "video" | "audio" | "image",
    quality: string,
    thumbnail: string | null
  ) {
    if (!remotePcIp.trim()) {
      showRemoteModal = true;
      showToast("⚠️ Ingresa la IP o dominio de tu PC para enviar");
      return;
    }

    const cleanTarget = remotePcIp.trim().replace(/\/+$/, "");
    isSendingRemote = true;
    const id = "remote_" + Date.now().toString() + Math.random().toString(36).substring(2, 5);

    const newItem: DownloadItem = {
      id,
      url,
      title: `[🖥️ PC] ${title}`,
      thumbnail,
      mode,
      quality,
      percent: 100,
      speed: "LAN",
      eta: "OK",
      status: "finished",
      outputPath: "~/navidrome/music"
    };

    try {
      await invoke("send_remote_download", {
        pcIp: cleanTarget,
        item: {
          url,
          title,
          mode,
          quality,
          thumbnail
        }
      });

      downloads = [newItem, ...downloads];
      saveHistory();
      const shortTitle = title.length > 32 ? title.substring(0, 32) + "..." : title;
      showToast(`✓ Enviado a PC: "${shortTitle}" → Navidrome`);
    } catch (err: any) {
      newItem.status = "error";
      newItem.errorMsg = err?.toString();
      downloads = [newItem, ...downloads];
      saveHistory();
      showToast(`✗ Error enviando a PC: ${err?.toString()}`);
    } finally {
      isSendingRemote = false;
    }
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
      } else if (typeof window !== "undefined" && navigator.userAgent.toLowerCase().includes("android")) {
        downloadFolder = "/storage/emulated/0/Download";
      }
      const savedTarget = localStorage.getItem("snapstream_download_target");
      if (savedTarget === "remote" || savedTarget === "local") {
        downloadTarget = savedTarget;
      }
      const savedIp = localStorage.getItem("snapstream_remote_ip");
      if (savedIp) {
        remotePcIp = savedIp;
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
      let data: MediaMetadata;
      if (downloadTarget === "remote" && remotePcIp.trim()) {
        try {
          data = await invoke<MediaMetadata>("get_remote_media_info", {
            pcIp: remotePcIp.trim(),
            url: raw
          });
        } catch {
          data = await invoke<MediaMetadata>("get_media_info", { url: raw });
        }
      } else {
        data = await invoke<MediaMetadata>("get_media_info", { url: raw });
      }
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
    const raw = urlInput.trim();
    if (!raw) return;

    // Si aún no se inspeccionó o se pegó una URL de playlist sin inspeccionar, inspeccionar primero
    if (!currentPreview && (raw.includes("playlist") || raw.includes("list="))) {
      await handleInspectUrl();
    }

    if (downloadTarget === "remote") {
      if (currentPreview?.is_playlist && currentPreview.playlist_items?.length) {
        const items = currentPreview.playlist_items;
        showToast(`Enviando playlist (${items.length} canciones) a PC...`);
        for (const item of items) {
          await sendRemoteDownload(item.url, item.title, "audio", "mp3", currentPreview.thumbnail);
        }
        return;
      }

      const title = currentPreview?.title || "recurso_multimedia";
      const thumb = currentPreview?.thumbnail || null;
      await sendRemoteDownload(raw, title, selectedMode, selectedQuality, thumb);
      return;
    }

    // Si es una playlist, descargar todos los elementos detectados secuencialmente
    if (currentPreview?.is_playlist && currentPreview.playlist_items?.length) {
      const items = currentPreview.playlist_items;
      showToast(`Descargando playlist (${items.length} pistas)...`);
      for (const item of items) {
        triggerSingleDownload(item.url, item.title, currentPreview.thumbnail);
        await new Promise((r) => setTimeout(r, 600));
      }
      return;
    }

    const title = currentPreview?.title || "recurso_multimedia";
    const thumb = currentPreview?.thumbnail || null;
    triggerSingleDownload(raw, title, thumb);
  }

  function handleSearchDownload(item: SearchResult, mode: "audio" | "video") {
    const quality = mode === "audio" ? "mp3" : "best";
    if (downloadTarget === "remote") {
      sendRemoteDownload(item.url, item.title, mode, quality, item.thumbnail);
    } else {
      triggerSingleDownload(item.url, item.title, item.thumbnail, mode, quality);
    }
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

    invoke<ServerInfo>("get_server_info")
      .then((info) => {
        serverInfo = info;
      })
      .catch(() => {});

    if (remotePcIp) {
      invoke<any>("ping_remote_server", { pcIp: remotePcIp })
        .then(() => {
          remoteConnected = true;
        })
        .catch(() => {
          remoteConnected = false;
        });
    }

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
  <div class="flex-1 flex flex-col max-w-4xl w-full mx-auto px-3.5 sm:px-6 pt-9 sm:pt-6 pb-8 space-y-4">
    
    <!-- Header -->
    <header class="flex items-center justify-between pb-3 border-b border-[#27272a] gap-2">
      <div class="flex items-center gap-2.5 min-w-0">
        <div class="w-8 h-8 rounded-lg border border-[#3f3f46] bg-[#18181b] flex items-center justify-center text-white shrink-0 shadow-sm">
          <Magnet class="w-4 h-4 stroke-[2.2]" />
        </div>
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-xs sm:text-sm font-bold tracking-wider text-white">snapstream</span>
          <span class="text-[10px] text-[#71717a] font-normal px-1.5 py-0.5 rounded bg-[#18181b] border border-[#27272a]">v0.1.0</span>
        </div>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        {#if serverInfo?.is_running}
          <button
            onclick={() => showServerInfoModal = true}
            class="flex items-center gap-1.5 text-xs bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] hover:border-emerald-500/50 text-[#d4d4d8] hover:text-white px-2.5 py-1.5 rounded-lg transition-all cursor-pointer"
            title="Servidor LAN activo para móvil"
          >
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <Server class="w-3.5 h-3.5 text-[#a1a1aa]" />
            <span class="text-[11px] font-mono hidden md:inline">{serverInfo.ip}:48792</span>
            <span class="text-[11px] md:hidden">LAN</span>
          </button>
        {/if}
        {#if updateStatus}
          <div class="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded bg-[#18181b] border border-[#3f3f46] text-[#e4e4e7]">
            <Activity class="w-3 h-3 animate-pulse" />
            <span class="hidden sm:inline">[{updateStatus}]</span>
          </div>
        {/if}
        <button
          onclick={checkForUpdates}
          disabled={isCheckingUpdate}
          class="flex items-center gap-1.5 text-xs bg-[#18181b] hover:bg-[#27272a] active:bg-[#3f3f46] border border-[#3f3f46] text-[#a1a1aa] hover:text-white px-2.5 py-1.5 rounded-lg transition-colors cursor-pointer"
          title="Verificar actualizaciones"
        >
          <RefreshCw class={`w-3.5 h-3.5 ${isCheckingUpdate ? 'animate-spin' : ''}`} />
          <span class="text-[11px]">sync</span>
        </button>
      </div>
    </header>

    <!-- Console Input Panel -->
    <div class="bg-[#10121a] border border-[#27272a] rounded-xl p-3.5 sm:p-4 space-y-3.5 shadow-sm">
      <!-- Target Switcher: Local Device vs Remote PC -->
      <div class="flex items-center justify-between gap-1 p-1 bg-[#090a0f] border border-[#27272a] rounded-lg text-xs">
        <div class="grid grid-cols-2 gap-1 flex-1">
          <button
            onclick={() => setDownloadTarget("local")}
            class={`flex items-center justify-center gap-1.5 py-1.5 px-2 rounded-md font-medium transition-all cursor-pointer ${
              downloadTarget === "local"
                ? "bg-[#27272a] text-white font-bold border border-[#3f3f46] shadow-sm"
                : "text-[#71717a] hover:text-[#d4d4d8]"
            }`}
          >
            <Smartphone class="w-3.5 h-3.5" />
            <span class="truncate">Descargar en móvil</span>
          </button>
          <button
            onclick={() => setDownloadTarget("remote")}
            class={`flex items-center justify-center gap-1.5 py-1.5 px-2 rounded-md font-medium transition-all cursor-pointer relative ${
              downloadTarget === "remote"
                ? "bg-[#27272a] text-white font-bold border border-[#3f3f46] shadow-sm"
                : "text-[#71717a] hover:text-[#d4d4d8]"
            }`}
          >
            <Monitor class="w-3.5 h-3.5" />
            <span class="truncate">Enviar a PC (Navidrome)</span>
            {#if remoteConnected}
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0 animate-pulse"></span>
            {/if}
          </button>
        </div>
        <button
          onclick={() => showRemoteModal = true}
          class={`p-2 rounded-md border transition-colors cursor-pointer shrink-0 ${
            remoteConnected
              ? "bg-emerald-950/40 border-emerald-800 text-emerald-300"
              : "bg-[#18181b] border-[#27272a] hover:border-[#3f3f46] text-[#71717a] hover:text-white"
          }`}
          title="Configurar IP de la PC"
        >
          <Radio class="w-3.5 h-3.5" />
        </button>
      </div>

      {#if downloadTarget === "remote"}
        <div class="flex items-center justify-between px-3 py-2 bg-emerald-950/20 border border-emerald-900/40 rounded-lg text-[11px] text-[#a1a1aa]">
          <div class="flex items-center gap-2 truncate">
            <span class="w-2 h-2 rounded-full bg-emerald-400 shrink-0 animate-pulse"></span>
            <span class="truncate">
              {#if remotePcIp}
                Destino: <strong class="text-white font-mono">{remotePcIp}:48792</strong> &rarr; <span class="text-emerald-400 font-mono">~/navidrome/music</span>
              {:else}
                <span class="text-amber-400 font-medium">⚠️ Falta configurar la IP de tu PC</span>
              {/if}
            </span>
          </div>
          <button
            onclick={() => showRemoteModal = true}
            class="text-[10px] text-emerald-400 hover:text-emerald-300 underline font-mono shrink-0 ml-2 cursor-pointer"
          >
            {remotePcIp ? "cambiar IP" : "configurar"}
          </button>
        </div>
      {/if}

      <!-- Tabs / Mode Bar (Mobile-first 50/50 segmented control) -->
      <div class="grid grid-cols-2 p-1 bg-[#090a0f] border border-[#27272a] rounded-lg gap-1">
        <button
          onclick={() => activeTab = "url"}
          class={`flex items-center justify-center gap-2 py-2 px-3 text-xs rounded-md transition-all cursor-pointer font-mono font-medium ${
            activeTab === "url"
              ? "bg-[#27272a] text-white font-bold shadow-sm border border-[#3f3f46]"
              : "text-[#71717a] hover:text-[#d4d4d8] hover:bg-white/5"
          }`}
        >
          <Magnet class="w-3.5 h-3.5" />
          <span>Enlace URL</span>
        </button>
        <button
          onclick={() => activeTab = "search"}
          class={`flex items-center justify-center gap-2 py-2 px-3 text-xs rounded-md transition-all cursor-pointer font-mono font-medium ${
            activeTab === "search"
              ? "bg-[#27272a] text-white font-bold shadow-sm border border-[#3f3f46]"
              : "text-[#71717a] hover:text-[#d4d4d8] hover:bg-white/5"
          }`}
        >
          <Search class="w-3.5 h-3.5" />
          <span>Buscar Música</span>
        </button>
      </div>

      {#if activeTab === "url"}
        <!-- URL Mode Content -->
        <div class="space-y-3">
          <div class="flex items-center justify-between text-[11px] text-[#71717a]">
            <div class="flex items-center gap-1.5">
              <span class="text-white font-bold">$</span>
              <span>input_url --extract</span>
            </div>
            <span class="text-[10px] text-[#52525b] truncate max-w-[190px] sm:max-w-none">YouTube • Spotify • TikTok • Soundcloud</span>
          </div>

          <div class="flex flex-col sm:flex-row gap-2">
            <div class="relative flex-1 flex items-center">
              <input
                type="text"
                placeholder="Pega enlace de YouTube, Spotify o TikTok..."
                bind:value={urlInput}
                onkeydown={(e) => e.key === "Enter" && handleInspectUrl()}
                class="w-full bg-[#090a0f] border border-[#27272a] focus:border-[#71717a] focus:ring-1 focus:ring-[#71717a] rounded-lg pl-3.5 pr-20 py-2.5 text-xs text-white placeholder-[#52525b] font-mono focus:outline-none transition-colors"
              />
              <div class="absolute right-1.5 flex items-center gap-1">
                {#if urlInput}
                  <button
                    onclick={() => urlInput = ""}
                    class="p-1 rounded text-[#71717a] hover:text-white cursor-pointer"
                    title="Borrar texto"
                  >
                    <X class="w-3.5 h-3.5" />
                  </button>
                {/if}
                <button
                  onclick={pasteClipboard}
                  class="px-2 py-1 rounded bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#a1a1aa] hover:text-white text-[10px] flex items-center gap-1 active:scale-95 transition-all cursor-pointer"
                  title="Pegar del portapapeles"
                >
                  <Clipboard class="w-3 h-3" />
                  <span>pegar</span>
                </button>
              </div>
            </div>
            <button
              onclick={handleInspectUrl}
              disabled={isLoadingInfo || !urlInput.trim()}
              class="w-full sm:w-auto px-5 py-2.5 rounded-lg bg-[#18181b] hover:bg-[#27272a] active:bg-[#3f3f46] border border-[#3f3f46] hover:border-white text-white font-mono text-xs transition-colors flex items-center justify-center gap-2 cursor-pointer disabled:opacity-30 shrink-0"
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
            <div class="p-3 bg-[#090a0f] border border-[#27272a] rounded-lg flex flex-col sm:flex-row gap-3 items-start sm:items-center">
              {#if currentPreview.thumbnail}
                <img
                  src={currentPreview.thumbnail}
                  alt="Thumbnail"
                  class="w-full sm:w-28 h-32 sm:h-20 object-cover rounded bg-black border border-[#27272a]"
                />
              {/if}
              <div class="flex-1 min-w-0 space-y-1 w-full">
                <div class="flex items-center gap-2">
                  {#if currentPreview.is_playlist}
                    <ListMusic class="w-4 h-4 text-white shrink-0" />
                  {/if}
                  <h2 class="text-xs sm:text-sm font-bold text-white truncate" title={currentPreview.title}>
                    {currentPreview.title}
                  </h2>
                </div>
                <div class="flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-[#71717a]">
                  {#if currentPreview.is_playlist}
                    <span class="text-white font-medium bg-[#18181b] px-2 py-0.5 rounded border border-[#3f3f46]">
                      Playlist: {currentPreview.playlist_count || currentPreview.playlist_items?.length} items
                    </span>
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

          <!-- Controls Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-1">
            <!-- Mode Segmented Selector -->
            <div class="grid grid-cols-3 p-1 bg-[#090a0f] rounded-lg border border-[#27272a] gap-1">
              <button
                onclick={() => { selectedMode = "video"; selectedQuality = "best"; }}
                class={`flex items-center justify-center gap-1.5 py-2 text-xs rounded transition-all cursor-pointer font-bold ${
                  selectedMode === "video" ? "bg-white text-black shadow-sm" : "text-[#71717a] hover:text-white"
                }`}
              >
                <Video class="w-3.5 h-3.5" />
                <span>video</span>
              </button>
              <button
                onclick={() => { selectedMode = "audio"; selectedQuality = "mp3"; }}
                class={`flex items-center justify-center gap-1.5 py-2 text-xs rounded transition-all cursor-pointer font-bold ${
                  selectedMode === "audio" ? "bg-white text-black shadow-sm" : "text-[#71717a] hover:text-white"
                }`}
              >
                <Music class="w-3.5 h-3.5" />
                <span>audio</span>
              </button>
              <button
                onclick={() => { selectedMode = "image"; selectedQuality = "original"; }}
                class={`flex items-center justify-center gap-1.5 py-2 text-xs rounded transition-all cursor-pointer font-bold ${
                  selectedMode === "image" ? "bg-white text-black shadow-sm" : "text-[#71717a] hover:text-white"
                }`}
              >
                <ImageIcon class="w-3.5 h-3.5" />
                <span>foto</span>
              </button>
            </div>

            <!-- Quality Selector -->
            <div class="relative flex items-center">
              <select
                bind:value={selectedQuality}
                class="w-full bg-[#090a0f]! text-white! border border-[#27272a] focus:border-[#71717a] rounded-lg px-3.5 py-2.5 text-xs font-mono focus:outline-none appearance-none cursor-pointer"
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
              <div class="pointer-events-none absolute right-3 flex items-center text-[#71717a]">
                <ChevronDown class="w-4 h-4" />
              </div>
            </div>

            <!-- Folder Selector -->
            <button
              onclick={selectFolder}
              class="flex items-center justify-between bg-[#090a0f] border border-[#27272a] hover:border-[#71717a] px-3.5 py-2 rounded-lg text-xs text-[#a1a1aa] hover:text-white transition-colors truncate cursor-pointer text-left"
              title="Cambiar carpeta de destino"
            >
              <div class="flex items-center gap-2 truncate">
                <HardDrive class="w-3.5 h-3.5 text-[#71717a] shrink-0" />
                <span class="truncate">{downloadFolder}</span>
              </div>
              <span class="text-[10px] text-[#71717a] shrink-0 ml-1.5 px-1 py-0.5 rounded bg-[#18181b]">dir</span>
            </button>
          </div>

          <!-- Primary Download Action -->
          <div class="pt-1">
            <button
              onclick={handleDownload}
              disabled={!urlInput.trim() || isSendingRemote}
              class="w-full py-3 rounded-lg bg-white hover:bg-[#e4e4e7] active:scale-[0.98] disabled:opacity-20 text-black font-bold text-xs uppercase tracking-wider transition-all flex items-center justify-center gap-2 cursor-pointer shadow-lg shadow-white/5"
            >
              {#if isSendingRemote}
                <Loader2 class="w-4 h-4 animate-spin text-black" />
                <span>enviando a pc...</span>
              {:else if downloadTarget === "remote"}
                <Monitor class="w-4 h-4 stroke-[2.5]" />
                <span>{currentPreview?.is_playlist ? `enviar playlist a pc (navidrome)` : 'enviar orden a pc (navidrome)'}</span>
              {:else}
                <Download class="w-4 h-4 stroke-[2.5]" />
                <span>{currentPreview?.is_playlist ? `descargar playlist (${currentPreview.playlist_items?.length || 'todas'})` : 'ejecutar descarga'}</span>
              {/if}
            </button>
          </div>
        </div>
      {:else}
        <!-- Search Mode Content -->
        <div class="space-y-3">
          <div class="flex items-center justify-between text-[11px] text-[#71717a]">
            <div class="flex items-center gap-1.5">
              <span class="text-white font-bold">$</span>
              <span>ytsearch --music --tags --navidrome</span>
            </div>
            <button
              onclick={selectFolder}
              class="flex items-center gap-1.5 text-[11px] text-[#a1a1aa] hover:text-white transition-colors cursor-pointer truncate max-w-[150px] sm:max-w-[220px]"
              title="Directorio de destino"
            >
              <Folder class="w-3 h-3 text-[#71717a] shrink-0" />
              <span class="truncate">{downloadFolder}</span>
            </button>
          </div>

          <div class="flex flex-col sm:flex-row gap-2">
            <div class="relative flex-1 flex items-center">
              <input
                type="text"
                placeholder="Escribe canción, artista o álbum..."
                bind:value={searchQuery}
                onkeydown={handleSearchInputKeydown}
                class="w-full bg-[#090a0f] border border-[#27272a] focus:border-[#71717a] focus:ring-1 focus:ring-[#71717a] rounded-lg pl-3.5 pr-10 py-2.5 text-xs text-white placeholder-[#52525b] font-mono focus:outline-none transition-colors"
              />
              {#if searchQuery}
                <button
                  onclick={() => searchQuery = ""}
                  class="absolute right-2.5 p-1 rounded text-[#71717a] hover:text-white cursor-pointer"
                  title="Borrar"
                >
                  <X class="w-3.5 h-3.5" />
                </button>
              {/if}
            </div>
            <button
              onclick={handleSearch}
              disabled={isSearching || !searchQuery.trim()}
              class="w-full sm:w-auto px-6 py-2.5 rounded-lg bg-white hover:bg-[#e4e4e7] active:bg-[#d4d4d8] text-black font-mono font-bold text-xs transition-colors flex items-center justify-center gap-2 cursor-pointer disabled:opacity-30 shrink-0 shadow-sm"
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

          <!-- Quick examples (Horizontal swipeable strip on mobile) -->
          {#if searchResults.length === 0 && !isSearching && !errorMessage}
            <div class="flex items-center gap-2 overflow-x-auto no-scrollbar py-1">
              <span class="text-[10px] text-[#71717a] shrink-0 font-medium">ejemplos:</span>
              {#each [
                { label: "Loser Tame Impala", q: "Loser Tame Impala" },
                { label: "Instant Crush", q: "Daft Punk Instant Crush" },
                { label: "Crimen Cerati", q: "Crimen Gustavo Cerati" },
                { label: "Starboy", q: "The Weeknd Starboy" }
              ] as item}
                <button
                  onclick={() => { searchQuery = item.q; handleSearch(); }}
                  class="px-3 py-1 rounded-full bg-[#18181b] hover:bg-[#27272a] border border-[#27272a] hover:border-[#52525b] text-[11px] text-[#a1a1aa] hover:text-white whitespace-nowrap active:scale-95 transition-all cursor-pointer shrink-0"
                >
                  {item.label}
                </button>
              {/each}
            </div>
          {/if}

          <!-- Search Results List -->
          {#if searchResults.length > 0}
            <div class="space-y-2 pt-1">
              <div class="flex items-center justify-between text-xs text-[#71717a] border-b border-[#27272a] pb-1.5">
                <span class="font-bold text-[#a1a1aa] truncate mr-2">
                  // resultados [{searchResults.length}] — 1-click con tags ID3
                </span>
                <button
                  onclick={() => searchResults = []}
                  class="flex items-center gap-1 text-[11px] text-[#71717a] hover:text-white transition-colors cursor-pointer shrink-0"
                >
                  <X class="w-3 h-3" />
                  <span>cerrar</span>
                </button>
              </div>

              <div class="space-y-2 max-h-[440px] overflow-y-auto pr-1">
                {#each searchResults as item (item.id)}
                  {@const dlAudio = getDownloadState(item.url, "audio")}
                  {@const dlVideo = getDownloadState(item.url, "video")}
                  {@const activeDl = (dlAudio && (dlAudio.status === "downloading" || dlAudio.status === "processing" || dlAudio.status === "queued")) ? dlAudio : ((dlVideo && (dlVideo.status === "downloading" || dlVideo.status === "processing" || dlVideo.status === "queued")) ? dlVideo : null)}
                  <div class={`p-3 bg-[#090a0f] hover:bg-[#141620] border rounded-lg flex flex-col gap-2.5 transition-all ${
                    activeDl ? "border-[#52525b] shadow-sm shadow-white/5" : "border-[#27272a] hover:border-[#3f3f46]"
                  }`}>
                    <div class="flex flex-col sm:flex-row gap-2.5 sm:items-center justify-between w-full">
                      <div class="flex items-center gap-3 min-w-0 flex-1">
                        {#if item.thumbnail}
                          <img
                            src={item.thumbnail}
                            alt={item.title}
                            class="w-14 h-14 sm:w-16 sm:h-12 object-cover rounded-md bg-black border border-[#27272a] shrink-0"
                            loading="lazy"
                          />
                        {:else}
                          <div class="w-14 h-14 sm:w-16 sm:h-12 rounded-md bg-[#18181b] border border-[#27272a] flex items-center justify-center shrink-0">
                            <Music class="w-4 h-4 text-[#71717a]" />
                          </div>
                        {/if}
                        <div class="min-w-0 flex-1 space-y-0.5">
                          <p class="text-xs font-bold text-white line-clamp-2 sm:truncate" title={item.title}>
                            {item.title}
                          </p>
                          <div class="flex items-center gap-2 text-[11px] text-[#71717a]">
                            {#if item.uploader}
                              <span class="truncate max-w-[140px] sm:max-w-[220px] text-[#a1a1aa] font-medium">{item.uploader}</span>
                            {/if}
                            {#if item.duration}
                              <span class="shrink-0">• {formatDuration(item.duration)}</span>
                            {/if}
                          </div>
                        </div>
                      </div>

                      <div class="flex items-center gap-1.5 shrink-0 self-end sm:self-center pt-1 sm:pt-0">
                        <!-- MP3 Button -->
                        {#if dlAudio}
                          {#if dlAudio.status === "downloading"}
                            <div class="flex items-center gap-1.5 text-[11px] bg-white text-black font-bold px-3 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-black" />
                              <span>{dlAudio.percent.toFixed(0)}%</span>
                            </div>
                          {:else if dlAudio.status === "processing"}
                            <div class="flex items-center gap-1.5 text-[11px] bg-white text-black font-bold px-3 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-black" />
                              <span>tags ID3</span>
                            </div>
                          {:else if dlAudio.status === "finished"}
                            <button
                              onclick={() => openInFolder(dlAudio)}
                              class="flex items-center gap-1 text-[11px] bg-white hover:bg-[#e4e4e7] text-black font-bold px-3 py-1.5 rounded-md transition-all cursor-pointer"
                              title="MP3 descargado. Clic para abrir en carpeta."
                            >
                              <Check class="w-3 h-3 stroke-[3]" />
                              <span>¡Listo!</span>
                            </button>
                          {:else if dlAudio.status === "queued"}
                            <div class="flex items-center gap-1.5 text-[11px] bg-[#27272a] text-[#d4d4d8] px-3 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-white" />
                              <span>en cola</span>
                            </div>
                          {:else if dlAudio.status === "error"}
                            <button
                              onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "audio", "mp3")}
                              class="flex items-center gap-1 text-[11px] bg-red-950 border border-red-800 text-red-200 px-2.5 py-1 rounded-md cursor-pointer"
                              title={dlAudio.errorMsg || "Error al descargar"}
                            >
                              <AlertTriangle class="w-3 h-3" />
                              <span>reintentar</span>
                            </button>
                          {/if}
                        {:else}
                          <button
                            onclick={() => handleSearchDownload(item, "audio")}
                            disabled={isSendingRemote}
                            class="flex items-center gap-1.5 text-xs bg-white hover:bg-[#e4e4e7] active:scale-95 text-black font-bold px-3 py-1.5 rounded-md transition-all cursor-pointer shadow-sm disabled:opacity-40"
                            title={downloadTarget === "remote" ? "Enviar MP3 a tu PC (Navidrome)" : "Descargar MP3 con ID3 tags y carátula para Navidrome"}
                          >
                            {#if downloadTarget === "remote"}
                              <Monitor class="w-3.5 h-3.5 stroke-[2.5]" />
                              <span>MP3</span>
                            {:else}
                              <Music class="w-3.5 h-3.5 stroke-[2.5]" />
                              <span>MP3</span>
                            {/if}
                          </button>
                        {/if}

                        <!-- Video Button -->
                        {#if dlVideo}
                          {#if dlVideo.status === "downloading"}
                            <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-white px-2.5 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-white" />
                              <span>{dlVideo.percent.toFixed(0)}%</span>
                            </div>
                          {:else if dlVideo.status === "processing"}
                            <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-white px-2.5 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-white" />
                              <span>remux</span>
                            </div>
                          {:else if dlVideo.status === "finished"}
                            <button
                              onclick={() => openInFolder(dlVideo)}
                              class="flex items-center gap-1 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-white px-2.5 py-1.5 rounded-md cursor-pointer"
                              title="Video descargado. Clic para abrir"
                            >
                              <Check class="w-3 h-3 stroke-[3]" />
                              <span>Listo</span>
                            </button>
                          {:else if dlVideo.status === "queued"}
                            <div class="flex items-center gap-1 text-[11px] bg-[#18181b] border border-[#3f3f46] text-[#a1a1aa] px-2.5 py-1.5 rounded-md font-mono">
                              <Loader2 class="w-3 h-3 animate-spin text-white" />
                              <span>cola</span>
                            </div>
                          {:else if dlVideo.status === "error"}
                            <button
                              onclick={() => triggerSingleDownload(item.url, item.title, item.thumbnail, "video", "best")}
                              class="flex items-center gap-1 text-[11px] bg-red-950 border border-red-800 text-red-200 px-2 py-1 rounded-md cursor-pointer"
                            >
                              <AlertTriangle class="w-3 h-3" />
                              <span>reintentar</span>
                            </button>
                          {/if}
                        {:else}
                          <button
                            onclick={() => handleSearchDownload(item, "video")}
                            disabled={isSendingRemote}
                            class="flex items-center gap-1.5 text-xs bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#d4d4d8] hover:text-white px-3 py-1.5 rounded-md transition-all cursor-pointer active:scale-95 disabled:opacity-40"
                            title={downloadTarget === "remote" ? "Enviar Video a tu PC" : "Descargar Video MP4"}
                          >
                            {#if downloadTarget === "remote"}
                              <Monitor class="w-3.5 h-3.5" />
                              <span>Video</span>
                            {:else}
                              <Video class="w-3.5 h-3.5" />
                              <span>Video</span>
                            {/if}
                          </button>
                        {/if}

                        <button
                          onclick={() => {
                            activeTab = "url";
                            urlInput = item.url;
                            handleInspectUrl();
                          }}
                          class="text-xs bg-[#18181b] hover:bg-[#27272a] border border-[#27272a] hover:border-[#3f3f46] text-[#71717a] hover:text-white p-2 rounded-md transition-colors cursor-pointer"
                          title="Inspeccionar enlace"
                        >
                          <ExternalLink class="w-3.5 h-3.5" />
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
                        <div class="w-full bg-[#18181b] border border-[#27272a] rounded-full h-1.5 overflow-hidden">
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
        <div class="p-3 bg-[#1c1917] border border-[#44403c] rounded-lg text-[#fca5a5] text-xs flex items-center gap-2">
          <AlertTriangle class="w-4 h-4 shrink-0 text-red-400" />
          <span>{errorMessage}</span>
        </div>
      {/if}
    </div>

    <!-- Active Tasks & Persistent History -->
    <div class="space-y-2.5">
      <div class="flex items-center justify-between text-xs text-[#71717a] border-b border-[#27272a] pb-1.5">
        <span class="font-bold text-[#a1a1aa]">
          // cola_procesos [{downloads.length}]
        </span>
        {#if downloads.length > 0}
          <button
            onclick={clearHistory}
            class="flex items-center gap-1 text-[11px] text-[#71717a] hover:text-white transition-colors cursor-pointer"
          >
            <Trash2 class="w-3.5 h-3.5" />
            <span>limpiar</span>
          </button>
        {/if}
      </div>

      {#if downloads.length === 0}
        <div class="p-8 border border-dashed border-[#27272a] rounded-xl flex flex-col items-center justify-center text-[#52525b] text-xs space-y-1.5 bg-[#090a0f]/50">
          <Magnet class="w-6 h-6 opacity-30 mb-0.5" />
          <span>esperando enlaces... pega uno arriba o busca música</span>
        </div>
      {:else}
        <div class="space-y-2">
          {#each downloads as item (item.id)}
            <div class="bg-[#10121a] border border-[#27272a] rounded-xl p-3 transition-all space-y-2.5 shadow-sm">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class="w-7 h-7 rounded-lg bg-[#18181b] border border-[#27272a] flex items-center justify-center shrink-0">
                    {#if item.mode === "audio"}
                      <Music class="w-3.5 h-3.5 text-[#d4d4d8]" />
                    {:else if item.mode === "image"}
                      <ImageIcon class="w-3.5 h-3.5 text-[#d4d4d8]" />
                    {:else}
                      <Video class="w-3.5 h-3.5 text-[#d4d4d8]" />
                    {/if}
                  </div>
                  <div class="min-w-0">
                    <p class="text-xs font-semibold text-white truncate">{item.title}</p>
                    <div class="flex items-center gap-2 text-[10px] text-[#71717a]">
                      <span class="text-[#a1a1aa] uppercase font-bold">{item.mode}</span>
                      <span>// {item.quality}</span>
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
                      <Loader2 class="w-3.5 h-3.5 animate-spin" />
                      <span class="text-[11px]">remux</span>
                    </div>
                  {:else if item.status === "finished"}
                    <div class="flex items-center gap-1.5">
                      {#if item.outputPath === "~/navidrome/music"}
                        <span class="text-[10px] text-emerald-400 font-mono bg-emerald-950/40 border border-emerald-800/60 px-2 py-0.5 rounded">
                          Navidrome PC
                        </span>
                      {:else}
                        <button
                          onclick={() => openInFolder(item)}
                          class="flex items-center gap-1 text-[11px] bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-[#d4d4d8] hover:text-white px-2.5 py-1 rounded-md transition-colors cursor-pointer"
                          title="Abrir archivo o carpeta"
                        >
                          <FolderOpen class="w-3.5 h-3.5" />
                          <span>abrir</span>
                        </button>
                      {/if}
                      <div class="flex items-center gap-0.5 text-white">
                        <Check class="w-4 h-4 stroke-[3]" />
                      </div>
                    </div>
                  {:else if item.status === "error"}
                    <span class="text-[11px] text-[#fca5a5]" title={item.errorMsg}>error</span>
                  {/if}
                </div>
              </div>

              {#if item.status === "downloading" || item.status === "processing"}
                <div class="w-full bg-[#090a0f] border border-[#27272a] rounded-full h-1.5 overflow-hidden">
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

    <!-- Modal: Información del Servidor PC -->
    {#if showServerInfoModal}
      <div class="fixed inset-0 z-50 bg-black/80 backdrop-blur-xs flex items-center justify-center p-4">
        <div class="bg-[#10121a] border border-[#3f3f46] rounded-xl max-w-md w-full p-5 space-y-4 shadow-2xl relative">
          <button
            onclick={() => showServerInfoModal = false}
            class="absolute top-4 right-4 text-[#71717a] hover:text-white cursor-pointer"
          >
            <X class="w-4 h-4" />
          </button>

          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-lg bg-emerald-950/50 border border-emerald-800 flex items-center justify-center text-emerald-400">
              <Server class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">Servidor LAN SnapStream</h3>
              <p class="text-[11px] text-emerald-400 flex items-center gap-1.5">
                <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
                Activo y esperando órdenes del móvil
              </p>
            </div>
          </div>

          <div class="p-3 bg-[#090a0f] border border-[#27272a] rounded-lg space-y-2 text-xs">
            <div class="flex items-center justify-between">
              <span class="text-[#71717a]">IP Local:</span>
              <div class="flex items-center gap-2">
                <code class="text-white font-bold bg-[#18181b] px-2 py-0.5 rounded border border-[#3f3f46]">
                  {serverInfo?.ip || '127.0.0.1'}
                </code>
                <button
                  onclick={copyServerIpToClipboard}
                  class="flex items-center gap-1 text-[11px] bg-white hover:bg-[#e4e4e7] text-black font-bold px-2 py-0.5 rounded transition-colors cursor-pointer"
                >
                  {#if copiedServerIp}
                    <Check class="w-3 h-3 stroke-[3]" />
                    <span>¡Copiada!</span>
                  {:else}
                    <Copy class="w-3 h-3" />
                    <span>Copiar</span>
                  {/if}
                </button>
              </div>
            </div>

            <div class="flex items-center justify-between">
              <span class="text-[#71717a]">Puerto LAN:</span>
              <code class="text-[#a1a1aa] font-mono">48792</code>
            </div>

            <div class="flex items-center justify-between">
              <span class="text-[#71717a]">Destino automático:</span>
              <span class="text-emerald-400 font-mono truncate max-w-[210px]" title="~/navidrome/music">~/navidrome/music</span>
            </div>
          </div>

          <div class="text-[11px] text-[#71717a] space-y-1.5 leading-relaxed bg-[#18181b]/50 p-3 rounded-lg border border-[#27272a]">
            <p class="text-[#a1a1aa] font-semibold">¿Cómo conectar tu móvil?</p>
            <ol class="list-decimal list-inside space-y-1">
              <li>Conecta tu celular a la misma red Wi-Fi.</li>
              <li>Abre SnapStream en el móvil y pulsa <strong class="text-white">Enviar a PC</strong>.</li>
              <li>Escribe la IP <code class="text-white font-bold">{serverInfo?.ip}</code> y guarda.</li>
              <li>¡Las canciones se descargarán directamente en tu PC sin ocupar espacio en el móvil!</li>
            </ol>
          </div>

          <div class="flex justify-end pt-1">
            <button
              onclick={() => showServerInfoModal = false}
              class="px-4 py-2 bg-white hover:bg-[#e4e4e7] text-black font-bold text-xs rounded-lg transition-colors cursor-pointer"
            >
              Entendido
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- Modal: Conectar con PC Remota -->
    {#if showRemoteModal}
      <div class="fixed inset-0 z-50 bg-black/80 backdrop-blur-xs flex items-center justify-center p-4">
        <div class="bg-[#10121a] border border-[#3f3f46] rounded-xl max-w-md w-full p-5 space-y-4 shadow-2xl relative">
          <button
            onclick={() => showRemoteModal = false}
            class="absolute top-4 right-4 text-[#71717a] hover:text-white cursor-pointer"
          >
            <X class="w-4 h-4" />
          </button>

          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-lg bg-[#18181b] border border-[#3f3f46] flex items-center justify-center text-white">
              <Monitor class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-bold text-white">Enviar a PC Remota (Navidrome)</h3>
              <p class="text-[11px] text-[#71717a]">Descarga en tu PC sin ocupar espacio en el móvil</p>
            </div>
          </div>

          <p class="text-xs text-[#a1a1aa] leading-relaxed">
            Ingresa la IP de tu computadora (deben estar en la misma red Wi-Fi). Las canciones se guardarán automáticamente en <code class="text-white">~/navidrome/music</code>.
          </p>

          <div class="space-y-1.5">
            <span class="text-[11px] text-[#71717a] font-medium block">IP de la PC o dominio remoto:</span>
            <div class="flex gap-2">
              <input
                type="text"
                placeholder="192.168.1.69, Tailscale IP o https://tudominio.com"
                bind:value={remotePcIp}
                onkeydown={(e) => e.key === "Enter" && testRemoteConnection()}
                class="flex-1 bg-[#090a0f] border border-[#27272a] focus:border-[#71717a] rounded-lg px-3 py-2 text-xs text-white placeholder-[#52525b] font-mono focus:outline-none"
              />
              <button
                onclick={() => testRemoteConnection()}
                disabled={isTestingRemote || !remotePcIp.trim()}
                class="px-3.5 py-2 bg-[#18181b] hover:bg-[#27272a] border border-[#3f3f46] text-white text-xs font-mono font-medium rounded-lg transition-colors cursor-pointer disabled:opacity-40 flex items-center gap-1.5 shrink-0"
              >
                {#if isTestingRemote}
                  <Loader2 class="w-3.5 h-3.5 animate-spin" />
                  <span>probando...</span>
                {:else}
                  <Wifi class="w-3.5 h-3.5" />
                  <span>probar</span>
                {/if}
              </button>
            </div>
          </div>

          <!-- Test connection status banner -->
          {#if remoteConnected === true}
            <div class="p-3 bg-emerald-950/40 border border-emerald-800 rounded-lg text-emerald-300 text-xs flex items-center gap-2">
              <Check class="w-4 h-4 shrink-0 text-emerald-400 stroke-[3]" />
              <span>¡Conexión establecida con la PC con éxito!</span>
            </div>
          {:else if remoteConnected === false}
            <div class="p-3 bg-red-950/40 border border-red-800 rounded-lg text-red-300 text-xs flex items-center gap-2">
              <AlertTriangle class="w-4 h-4 shrink-0 text-red-400" />
              <span>No se pudo conectar a la PC. Verifica que SnapStream esté abierto en tu computadora y la IP sea correcta.</span>
            </div>
          {/if}

          <div class="flex items-center justify-end gap-2 pt-2 border-t border-[#27272a]">
            <button
              onclick={() => showRemoteModal = false}
              class="px-3.5 py-2 text-xs text-[#a1a1aa] hover:text-white transition-colors cursor-pointer"
            >
              Cancelar
            </button>
            <button
              onclick={() => {
                if (remotePcIp.trim()) {
                  setDownloadTarget("remote");
                  localStorage.setItem("snapstream_remote_ip", remotePcIp.trim());
                  showToast(`Destino configurado a PC: ${remotePcIp.trim()}`);
                }
                showRemoteModal = false;
              }}
              disabled={!remotePcIp.trim()}
              class="px-4 py-2 bg-white hover:bg-[#e4e4e7] disabled:opacity-40 text-black font-bold text-xs rounded-lg transition-colors cursor-pointer"
            >
              Guardar y Activar
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if toastMessage}
      <div class="fixed bottom-6 left-4 right-4 sm:left-auto sm:right-6 sm:w-auto mx-auto max-w-sm z-50 flex items-center gap-2.5 bg-[#18181b] border border-[#3f3f46] text-white px-4 py-3 rounded-xl shadow-2xl text-xs font-mono">
        <Activity class="w-4 h-4 text-white animate-pulse shrink-0" />
        <span class="max-w-[340px] truncate">{toastMessage}</span>
      </div>
    {/if}

  </div>
</div>
