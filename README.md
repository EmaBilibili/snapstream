# ⚡ SnapStream

> Descargador multimedia de escritorio, hiperligero, moderno y de código abierto para **Linux** y **Windows**.

SnapStream permite pegar enlaces de sitios como YouTube, Instagram, Facebook, TikTok, X (Twitter) y extraer video en alta calidad o audio (MP3/M4A/FLAC) con un solo clic.

---

## ✨ Características

- 🐧 **Linux (Todas las distribuciones)**: Empaquetado en `.AppImage` y `.deb`.
- 🪟 **Windows**: Instalador nativo `.msi` y `.exe`.
- ⚡ **Ultra ligero y rápido**: Construido con **Tauri v2 (Rust)** y **Svelte 5** para mínimo consumo de memoria y CPU.
- 🎨 **Interfaz visual limpia**: Diseño oscuro moderno con Tailwind CSS y Lucide Icons.
- 🔄 **Auto-actualizaciones integradas**: La app busca y aplica nuevas versiones directamente con un clic.
- 📦 **Motor yt-dlp**: Máxima compatibilidad con plataformas de streaming y redes sociales.

---

## 🚀 Desarrollo Local

### Requisitos previos
- [Rust](https://rustup.rs/) (1.78+)
- [Node.js](https://nodejs.org/) (20+) o compatible
- WebKit2GTK y dependencias de Tauri (en Linux):
  ```bash
  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 base-devel curl wget openssl appmenu-gtk-module gtk3
  
  # Ubuntu / Debian
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev
  ```

### Ejecutar en modo desarrollo
```bash
git clone https://github.com/EmaBilibili/snapstream.git
cd snapstream

npm install
npm run tauri dev
```

---

## 🛠 Compilación para Producción

```bash
npm run tauri build
```
Los ejecutables e instaladores se generarán en `src-tauri/target/release/bundle/`.

---

## 📄 Licencia
Distribuido bajo la licencia [MIT](LICENSE).
