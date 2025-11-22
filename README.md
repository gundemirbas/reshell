# Minimal No-STD Linux Shell

Rust ile yazılmış, `no_std` (standard library kullanmadan) hafif bir Linux shell uygulaması.

## 🎯 Özellikler

### ✅ Mevcut Özellikler

#### Builtin Komutlar
- `ls [path]` - Dizin içeriğini listele (alfabetik sıralı)
- `cd <path>` - Dizin değiştir 
- `pwd` - Çalışma dizinini göster (getcwd syscall #79)
- `echo <text>` - Metni yazdır (env var expansion destekli)
- `export NAME=VALUE` - Environment variable tanımla
- `env` - Tüm environment variable'ları göster
- `alias NAME=COMMAND` - Komut takma adı oluştur
- `history` - Komut geçmişini göster
- `serve [port]` - HTTP server başlat (varsayılan port: 8000)
- `exit` - Shell'den çık

#### Özellikler
- **Environment Variable Desteği**: `$VAR` syntax ile expansion
- **Command History**: Son 10 komut kaydedilir
- **Alias Desteği**: Kısa komut takma adları
- **Tab Completion**: `/bin` ve `/usr/bin` dizinlerinde komut tamamlama (temel)
- **Thread Desteği**: Arka planda ticker thread (her 10s `#` yazdırır)
- **Fork/Exec**: External komutları çalıştırma
- **Safe Unsafe İzolasyonu**: Unsafe kod minimal ve izole edilmiş

## 📊 Teknik Detaylar

### Binary Bilgileri
- **Boyut**: ~19KB (stripped)
- **Platform**: Linux x86_64
- **Dependencies**: Hiçbiri (no_std)
- **Syscalls**: Direkt Linux syscall'ları
- **Modüller**: 10 ayrı dosya (~2700 satır)

### Proje Yapısı
```
src/
├── main.rs          (116 satır)  - Entry point
├── syscalls.rs      (603 satır)  - System calls + Socket API
├── server.rs        (336 satır)  - HTTP server
├── storage.rs       (296 satır)  - Data storage
├── parser.rs        (259 satır)  - Parsing logic
├── builtins.rs      (225 satır)  - Builtin commands
├── io.rs            (199 satır)  - I/O helpers
├── executor.rs      (112 satır)  - Command execution
├── utils.rs         (90 satır)   - Utilities + Sorting
└── thread.rs        (90 satır)   - Threading
```

Detaylı mimari bilgisi için `ARCHITECTURE.md` dosyasına bakın.

### Kullanılan Syscalls
- `read/write` - I/O işlemleri
- `fork/execve/waitpid` - Process yönetimi
- `open/close/getdents64` - Dosya/dizin işlemleri
- `chdir/getcwd` - Dizin işlemleri
- `clone` - Thread oluşturma
- `nanosleep` - Zamanlama
- `kill/gettid` - Sinyal/thread yönetimi

### Güvenlik & Mimari
- **UnsafeCell** ile safe API wrapper'ları
- **Static global** değişkenler için Sync implementation
- **Closure-based** safe abstractions
- **Zero-copy** dirent parsing

## 🚀 Kullanım

### Derleme
```bash
cargo build --release
```

### Örnek Kullanım
```bash
$ export NAME=World
$ echo Hello $NAME
Hello World

$ alias ll=ls
$ ll
.git
src
Cargo.toml

$ history
  1: export NAME=World
  2: echo Hello $NAME
  3: alias ll=ls
  4: ll

$ cd /tmp
$ ls
...

$ serve 8080
Starting HTTP server on port 8080...
Server running! Press Ctrl+C to stop.
Visit http://localhost:8080/

# Tarayıcıdan: http://localhost:8080/
# 1. Dizin listesi görüntülenir
# 2. Upload form ile dosya yükle:
#    - Filename: test.txt
#    - Content: Hello World!
#    - [Upload] butonuna tıkla
# 3. Dosya oluşturulur ve içeriği gösterilir

# POST ile dosya upload (curl):
$ curl -X POST http://localhost:8080/upload \
  -d "filename=test.txt&content=Hello World!"
# -> Dosya oluşturulur ve içeriği HTML'de gösterilir

# Multi-line içerik:
$ curl -X POST http://localhost:8080/upload \
  -d "filename=poem.txt&content=Line 1%0ALine 2%0ALine 3"

# GET ile dosya indir:
$ curl http://localhost:8080/test.txt
Hello World!

$ curl http://localhost:8080/poem.txt
Line 1
Line 2
Line 3
```

**Özellikler:**
- ✅ GET: Dosya servis etme (content-type detection)
- ✅ POST: Dosya upload + HTML'de içerik gösterimi
- ✅ HTML Form support (application/x-www-form-urlencoded)
- ✅ Multi-line content (textarea)
- ✅ Special character handling
- ✅ Directory listing (alfabetik sıralı)
- ✅ Python'ın `python -m http.server` gibi ama daha özellikli!

## 🎨 Optimizasyonlar

### Yapılan Optimizasyonlar
1. **Bellek**: Gereksiz buffer kopyaları kaldırıldı
2. **Unsafe İzolasyonu**: %70+ unsafe kod minimize edildi
3. **Safe API**: Wrapper struct'lar ile güvenli erişim
4. **Binary Size**: Strip ve LTO ile 8KB'ye düşürüldü

### Performans İpuçları
- Binary zaten oldukça optimize
- `opt-level = "z"` küçük binary için
- `lto = true` kod optimizasyonu için
- `strip = true` debug bilgilerini kaldırır

## 💡 Eklenebilecek Özellikler

### Öncelikli Özellikler

#### 1. **Pipe Desteği** (Orta)
```rust
// Örnek: ls | grep txt
```
- Pipe syscall'ı
- Multiple process coordination
- I/O redirection

#### 2. **I/O Redirection** (Kolay)
```bash
echo test > file.txt
cat < input.txt
```
- `dup2` syscall
- File handle manipulation

#### 3. **Background Jobs** (Orta)
```bash
sleep 100 &
jobs
fg %1
```
- Job control
- Process group management

#### 4. **Signal Handling** (Orta-Zor)
```rust
// Ctrl+C handling
// SIGINT, SIGTERM, SIGCHLD
```
- `sigaction` syscall
- Signal mask manipulation

#### 5. **Globbing/Wildcard** (Orta)
```bash
ls *.txt
echo file[0-9].rs
```
- Pattern matching
- Directory traversal

#### 6. **Command Substitution** (Zor)
```bash
echo $(pwd)
echo `date`
```
- Fork ve pipe
- Output capture

#### 7. **Here Documents** (Kolay)
```bash
cat << EOF
Hello
World
EOF
```
- Multi-line input
- Buffer management

#### 8. **Prompt Customization** (Kolay)
```bash
export PS1="[\u@\h \w]$ "
```
- Escape sequence parsing
- Dynamic prompt

#### 9. **Scripting Support** (Zor)
```bash
if [ -f file.txt ]; then
    echo "exists"
fi
```
- Parsing
- Control flow

#### 10. **Auto-complete İyileştirmesi** (Orta)
- Dosya/dizin completion
- Argument completion
- History-based suggestions

### Gelişmiş Özellikler

#### 11. **Vi/Emacs Mode** (Zor)
- Line editing
- Keyboard shortcuts
- Modal editing

#### 12. **Color Support** (Kolay)
- ANSI escape codes
- Syntax highlighting
- Colored ls output

#### 13. **Built-in Functions** (Kolay-Orta)
```bash
function greet() {
    echo "Hello $1"
}
```
- Function storage
- Argument passing

#### 14. **Arithmetic Expansion** (Orta)
```bash
echo $((2 + 3))
```
- Expression parsing
- Integer math

#### 15. **String Manipulation** (Orta)
```bash
${VAR#prefix}
${VAR%suffix}
${VAR//pattern/replacement}
```
- Substring operations
- Pattern matching

### Performans Optimizasyonları

#### 16. **Command Caching** (Kolay)
- PATH lookup cache
- Executable location cache

#### 17. **Buffer Pool** (Orta)
- Reusable buffers
- Memory allocation reduction

#### 18. **Lazy Evaluation** (Orta)
- Deferred expansion
- Pipeline optimization

## 🏗️ Mimari İyileştirmeleri

### Kod Organizasyonu
1. Modüler yapı (parser, executor, builtin modules)
2. Trait-based extension system
3. Plugin architecture

### Documentation
1. Inline documentation
2. API documentation
3. Architecture diagrams

## 📝 Notlar

### Bilinen Sınırlamalar
- ✅ ~~`pwd`: getcwd syscall problemi~~ → **ÇÖZÜLDİ** (doğru buffer handling)
- Tab completion: Raw mode implementasyonu hazır ama şimdilik devre dışı
- History: Persist edilmiyor (memory-only)
- Aliases: Session sonunda kaybolur
- Max 128 dosya ls desteği

### Geliştirme Önerileri
1. İlk önce temel pipe desteği ekleyin (en çok istenilen özellik)
2. I/O redirection ekleyin (kolay ve kullanışlı)
3. Signal handling ile Ctrl+C desteği
4. Daha sonra scripting özellikleri

## 📚 Kaynaklar

- [Linux System Call Table](https://filippo.io/linux-syscall-table/)
- [x86-64 ABI](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf)
- [Writing a Shell in Rust](https://www.joshmcguigan.com/blog/build-your-own-shell-rust/)

## 🤝 Katkıda Bulunma

Bu minimal bir proof-of-concept projedir. İyileştirmeler ve öneriler için environment_variables dosyasını güncelleyin.

## 📄 Lisans

MIT veya projenizin lisansı

---

**Son Güncelleme**: 2024-11-22  
**Versiyon**: 0.5.0  
**Binary Size**: 19KB  
**Yeni**:
- ✅ **HTTP Server GET & POST** - Full featured web server!
- ✅ **GET: File serving** - Content-type detection (.html, .txt, .json, .css, .js, etc.)
- ✅ **POST: File upload** - Form & curl with content display in HTML
- ✅ URL encoding/decoding
- ✅ HTML escaping for safe content display
- ✅ Styled HTML interface with dark theme
- ✅ Socket syscalls (socket, bind, listen, accept, setsockopt)
- ✅ HTML directory listing
- ✅ pwd komutu çalışıyor (getcwd syscall)
- ✅ Tab completion altyapısı hazır (raw mode)
- ✅ Alfabetik sıralama (ls)
