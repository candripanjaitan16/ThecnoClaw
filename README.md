# TheCnoClaw

Desktop app terminal-style buat ngobrol sama AI (default: Groq, gratis) dan generate dokumen.

## Cara jalanin (development)

```bash
npm install
cargo tauri dev
```

Kalau `cargo tauri` belum dikenali, install dulu CLI-nya:
```bash
cargo install tauri-cli --version "^1"
```

## Cara pakai

1. Buka app, ketik `api` di terminal.
2. Ikuti alur: provider (`groq`/`openai`/`anthropic`/`custom`) → model → API key → endpoint (kosongkan untuk default Groq).
3. Kalau Groq: dapatkan API key gratis di https://console.groq.com/keys — model contoh: `llama-3.3-70b-versatile`.
4. Setelah sukses, klik ikon 💬 di pojok kanan bawah atau ketik `chat`.
5. Di popup: klik **Connect**, (opsional) **Pilih File** target `.docx`/`.txt`, lalu ketik pesan seperti:
   - `Buatkan pengertian AI`
   - `buatkan dengan rapi` (AI akan menyusun jawaban lebih terstruktur, cocok untuk dokumen resmi)
6. Kalau ada file dipilih, jawaban AI otomatis disimpan ke file itu.
7. Ketik `status` untuk cek konfigurasi & koneksi. Ketik `donasi` untuk halaman donasi.

## Build jadi aplikasi installer

```bash
cargo tauri build
```

Hasil installer ada di `src-tauri/target/release/bundle/`.

## Catatan

- Konfigurasi (model/key/endpoint) disimpan di folder config OS (lewat crate `directories`), bukan di dalam project — aman dari ke-commit ke git.
- File `save_text_file` saat ini menyimpan sebagai plain text. Untuk docx dengan styling penuh (heading, bold, dst), perlu tambahan generator docx (langkah lanjutan).
