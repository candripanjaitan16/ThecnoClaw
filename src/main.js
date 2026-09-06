const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

// ============ TERMINAL ELEMENTS ============
const output = document.getElementById("output");
const cmdInput = document.getElementById("cmd-input");
const promptEl = document.getElementById("prompt");

// ============ POPUP ELEMENTS ============
const popup = document.getElementById("popup");
const openPopupBtn = document.getElementById("open-popup-btn");
const popupClose = document.getElementById("popup-close");
const popupChat = document.getElementById("popup-chat");
const popupInput = document.getElementById("popup-input");
const popupSend = document.getElementById("popup-send");
const connStatus = document.getElementById("conn-status");
const btnToggleConn = document.getElementById("btn-toggle-conn");
const btnPickFile = document.getElementById("btn-pick-file");
const pickedFileLabel = document.getElementById("picked-file-label");

// ============ STATE ============
// path terminal: [] = root ($thecnoclaw), ["api"] = mode setup api, dst
let currentPath = [];
let apiSetupStep = null; // null | "model" | "key" | "endpoint"
let tempConfig = { provider: "groq", model: "", api_key: "", endpoint: "" };

let isConnected = false;
let pickedFilePath = null;
let chatHistory = []; // { role, content }

// ============ HELPERS: TERMINAL OUTPUT ============
function printLine(text, cls = "") {
  const div = document.createElement("div");
  div.className = "line " + cls;
  div.textContent = text;
  output.appendChild(div);
  output.scrollTop = output.scrollHeight;
}

function currentPromptString() {
  if (currentPath.length === 0) return "$thecnoclaw >";
  return "$thecnoclaw/" + currentPath.join("/") + " >";
}

function refreshPrompt() {
  promptEl.textContent = currentPromptString();
}

// ============ INIT ============
window.addEventListener("DOMContentLoaded", async () => {
  printLine("=== TheCnoClaw Terminal ===", "info");
  printLine("Ketik 'help' untuk melihat perintah yang tersedia.", "dim");
  refreshPrompt();

  try {
    const cfg = await invoke("get_ai_config");
    if (cfg && cfg.api_key) {
      printLine(`Konfigurasi tersimpan ditemukan (provider: ${cfg.provider}, model: ${cfg.model}).`, "dim");
      tempConfig = cfg;
    }
  } catch (e) {
    // tidak ada config, aman diabaikan
  }

  cmdInput.focus();
});

// ============ TERMINAL COMMAND HANDLING ============
cmdInput.addEventListener("keydown", async (e) => {
  if (e.key !== "Enter") return;
  const raw = cmdInput.value;
  const cmd = raw.trim();
  cmdInput.value = "";

  printLine(`${currentPromptString()} ${raw}`, "prompt-echo");

  await handleCommand(cmd);
  refreshPrompt();
});

async function handleCommand(cmd) {
  const lower = cmd.toLowerCase();

  // --- Mode: sedang di dalam alur setup API step-by-step ---
  if (apiSetupStep) {
    await handleApiSetupInput(cmd);
    return;
  }

  // --- Root level commands ---
  if (currentPath.length === 0) {
    if (lower === "help") {
      printLine("Perintah tersedia:", "info");
      printLine("  api        -> masuk ke pengaturan koneksi AI (model/key/endpoint)", "dim");
      printLine("  chat       -> buka popup AI console", "dim");
      printLine("  status     -> cek status koneksi & konfigurasi", "dim");
      printLine("  donasi     -> buka halaman donasi", "dim");
      printLine("  clear      -> bersihkan layar", "dim");
      printLine("", "");
      printLine("Tips: tutup window (tombol X) tidak mematikan app — app tetap", "dim");
      printLine("jalan di system tray. Klik ikon tray untuk membuka lagi.", "dim");
      printLine("Pilih target '.docx' di popup supaya jawaban AI otomatis", "dim");
      printLine("dikonversi jadi dokumen Word rapi (heading, bold, tabel).", "dim");
      return;
    }
    if (lower === "api") {
      currentPath = ["api"];
      printLine("Masuk ke mode pengaturan API.", "info");
      printLine("Provider yang didukung: groq (default/gratis), openai, anthropic, custom.", "dim");
      startApiSetup();
      return;
    }
    if (lower === "chat") {
      openPopup();
      return;
    }
    if (lower === "status") {
      await printStatus();
      return;
    }
    if (lower === "donasi") {
      openDonasi();
      return;
    }
    if (lower === "clear") {
      output.innerHTML = "";
      return;
    }
    if (lower === "") return;
    printLine(`Perintah tidak dikenal: '${cmd}'. Ketik 'help'.`, "error");
    return;
  }

  // --- Di dalam path/api tapi bukan sedang input step (jarang terjadi) ---
  if (currentPath[0] === "api") {
    if (lower === "keluar" || lower === "exit") {
      currentPath = [];
      apiSetupStep = null;
      printLine("Kembali ke root.", "dim");
      return;
    }
    startApiSetup();
    return;
  }
}

// ============ ALUR SETUP API ============
function startApiSetup() {
  tempConfig = { provider: tempConfig.provider || "groq", model: "", api_key: "", endpoint: "" };
  apiSetupStep = "provider";
  printLine("Masukkan provider (groq / openai / anthropic / custom) [default: groq]:", "info");
}

async function handleApiSetupInput(cmd) {
  const val = cmd.trim();

  if (apiSetupStep === "provider") {
    tempConfig.provider = val || "groq";
    apiSetupStep = "model";
    const suggestion = tempConfig.provider === "groq" ? " (contoh: llama-3.3-70b-versatile)" : "";
    printLine(`Masukkan nama model${suggestion}:`, "info");
    return;
  }

  if (apiSetupStep === "model") {
    if (!val) {
      printLine("Model tidak boleh kosong. Coba lagi:", "error");
      return;
    }
    tempConfig.model = val;
    apiSetupStep = "key";
    printLine("Masukkan API Key:", "info");
    return;
  }

  if (apiSetupStep === "key") {
    if (!val) {
      printLine("API Key tidak boleh kosong. Coba lagi:", "error");
      return;
    }
    tempConfig.api_key = val;
    apiSetupStep = "endpoint";
    const defaultEndpoint =
      tempConfig.provider === "groq"
        ? "https://api.groq.com/openai/v1/chat/completions"
        : "";
    printLine(
      `Masukkan endpoint (kosongkan untuk pakai default${defaultEndpoint ? ": " + defaultEndpoint : ""}):`,
      "info"
    );
    return;
  }

  if (apiSetupStep === "endpoint") {
    tempConfig.endpoint =
      val ||
      (tempConfig.provider === "groq"
        ? "https://api.groq.com/openai/v1/chat/completions"
        : "");

    try {
      const result = await invoke("save_ai_config", {
        provider: tempConfig.provider,
        model: tempConfig.model,
        apiKey: tempConfig.api_key,
        endpoint: tempConfig.endpoint,
      });
      currentPath = ["..", "..", ".."];
      printLine(`${currentPromptString()} ${result}`, "success");
      // reset path balik ke root setelah menampilkan pesan sukses ala contoh
      currentPath = [];
      apiSetupStep = null;
      openPopupBtn.classList.remove("hidden");
      printLine("Konfigurasi tersimpan. Ketik 'chat' atau klik ikon 💬 untuk membuka AI console.", "dim");
    } catch (err) {
      printLine(`Gagal menyimpan konfigurasi: ${err}`, "error");
      apiSetupStep = null;
      currentPath = [];
    }
    return;
  }
}

// ============ STATUS ============
async function printStatus() {
  try {
    const cfg = await invoke("get_ai_config");
    const connected = await invoke("get_connected");
    if (!cfg) {
      printLine("Belum ada konfigurasi API. Jalankan 'api' untuk setup.", "error");
      return;
    }
    printLine(`Provider  : ${cfg.provider}`, "info");
    printLine(`Model     : ${cfg.model}`, "info");
    printLine(`Endpoint  : ${cfg.endpoint}`, "info");
    printLine(`API Key   : ${"*".repeat(Math.max(cfg.api_key.length - 4, 0))}${cfg.api_key.slice(-4)}`, "info");
    printLine(`Status    : ${connected ? "terhubung" : "terputus"}`, connected ? "success" : "error");
  } catch (e) {
    printLine("Belum ada konfigurasi API. Jalankan 'api' untuk setup.", "error");
  }
}

// ============ DONASI ============
function openDonasi() {
  printLine("=== Donasi ===", "info");
  printLine("Kalau aplikasi ini membantu, kamu bisa donasi lewat:", "dim");
  printLine("  - Saweria / Trakteer: (isi link kamu di sini)", "dim");
  printLine("  - QRIS: (taruh gambar QRIS di /src/assets/qris.png lalu tampilkan)", "dim");
  printLine("Terima kasih banyak! 🙏", "success");
}

// ============ POPUP: OPEN/CLOSE ============
function openPopup() {
  popup.classList.remove("hidden");
  popupInput.focus();
}

function closePopup() {
  popup.classList.add("hidden");
}

openPopupBtn.addEventListener("click", openPopup);
popupClose.addEventListener("click", closePopup);

// ============ POPUP: CONNECT / DISCONNECT ============
async function refreshConnStatus() {
  isConnected = await invoke("get_connected");
  updateConnUI();
}

function updateConnUI() {
  if (isConnected) {
    connStatus.textContent = "● terhubung";
    connStatus.className = "connected";
    btnToggleConn.textContent = "Disconnect";
  } else {
    connStatus.textContent = "● terputus";
    connStatus.className = "disconnected";
    btnToggleConn.textContent = "Connect";
  }
}

btnToggleConn.addEventListener("click", async () => {
  const cfg = await invoke("get_ai_config").catch(() => null);
  if (!cfg) {
    addChatBubble("system", "Belum ada konfigurasi API. Jalankan 'api' di terminal dulu.");
    return;
  }
  const next = !isConnected;
  isConnected = await invoke("set_connected", { connected: next });
  updateConnUI();
  addChatBubble("system", isConnected ? "Terhubung ke AI." : "Terputus dari AI.");
});

// ============ POPUP: FILE PICKER ============
btnPickFile.addEventListener("click", async () => {
  try {
    const selected = await save({
      defaultPath: "dokumen-ai.docx",
      filters: [
        { name: "Word Document", extensions: ["docx"] },
        { name: "Teks", extensions: ["txt", "md"] },
      ],
    });
    if (selected) {
      pickedFilePath = selected;
      pickedFileLabel.textContent = selected;
      addChatBubble("system", `Target file: ${selected}`);
    }
  } catch (e) {
    addChatBubble("system", `Gagal membuka dialog file: ${e}`);
  }
});

// ============ POPUP: CHAT ============
function addChatBubble(role, text) {
  const div = document.createElement("div");
  div.className = "chat-bubble " + role;
  div.textContent = text;
  popupChat.appendChild(div);
  popupChat.scrollTop = popupChat.scrollHeight;
}

async function sendPopupMessage() {
  const text = popupInput.value.trim();
  if (!text) return;
  popupInput.value = "";

  if (!isConnected) {
    addChatBubble("system", "Belum terhubung. Klik 'Connect' dulu.");
    return;
  }

  addChatBubble("user", text);
  chatHistory.push({ role: "user", content: text });

  // Deteksi perintah "buatkan dengan rapi" -> instruksikan AI format rapi untuk docx
  const wantsRapi = /rapi|formal|professional|profesional|dokumen|makalah|laporan/i.test(text);
  const baseFormatRule =
    "Selalu tulis jawabanmu menggunakan sintaks Markdown: gunakan '#'/'##' untuk judul dan sub-judul, '**teks**' untuk penting/bold, '-' untuk daftar poin, dan tabel Markdown (| kolom | kolom |) bila data cocok disajikan sebagai tabel. Jangan gunakan format lain.";
  const systemHint = wantsRapi
    ? baseFormatRule +
      " Untuk permintaan ini, susun secara lengkap dan terstruktur (pendahuluan, isi per bagian dengan sub-judul, kesimpulan) karena akan disimpan sebagai dokumen Word formal."
    : baseFormatRule + " Jawab dengan jelas dan ringkas.";

  const messagesForApi = [
    { role: "system", content: systemHint },
    ...chatHistory,
  ];

  addChatBubble("system", "Memproses...");

  try {
    const reply = await invoke("send_chat", { messages: messagesForApi });
    // hapus bubble "Memproses..." terakhir
    popupChat.removeChild(popupChat.lastChild);

    addChatBubble("ai", reply);
    chatHistory.push({ role: "assistant", content: reply });

    // Jika ada file yang dipilih, tawarkan simpan ke file tersebut
    if (pickedFilePath) {
      await maybeSaveToFile(reply);
    }
  } catch (err) {
    popupChat.removeChild(popupChat.lastChild);
    addChatBubble("system", `Error: ${err}`);
  }
}

async function maybeSaveToFile(content) {
  try {
    const isDocx = pickedFilePath.toLowerCase().endsWith(".docx");
    if (isDocx) {
      await invoke("save_docx_file", { path: pickedFilePath, markdownContent: content });
    } else {
      await invoke("save_text_file", { path: pickedFilePath, content });
    }
    addChatBubble("system", `Tersimpan ke: ${pickedFilePath}`);
  } catch (e) {
    addChatBubble("system", `Gagal menyimpan ke file: ${e}`);
  }
}

popupSend.addEventListener("click", sendPopupMessage);
popupInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") sendPopupMessage();
});

// refresh status koneksi tiap kali popup dibuka
openPopupBtn.addEventListener("click", refreshConnStatus);
