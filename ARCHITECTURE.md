# Project Architecture

## Modular Structure

Proje mantıklı modüllere bölünmüştür:

### 📁 src/main.rs (115 satır)
- Entry point ve main fonksiyon
- Args parsing
- Main loop
- Assembly başlangıç kodu

### 📁 src/syscalls.rs (466 satır)
- Linux syscall wrapper'ları
- Düşük seviye sistem çağrıları
- Assembly inline kodlar

### 📁 src/storage.rs (296 satır)
- **EnvStorage**: Environment variable yönetimi
- **HistoryStorage**: Komut geçmişi (10 entry, circular buffer)
- **AliasStorage**: Komut takma adları (16 alias)
- Global static instance'lar

### 📁 src/utils.rs (90 satır)
- **bytes_equal**: Byte dizisi karşılaştırma
- **trim_newline**: Satır sonu temizleme
- **trim_spaces**: Boşluk temizleme
- **split_first_word**: İlk kelimeyi ayırma
- **sort_entries**: Alfabetik sıralama (bubble sort)
- **bytes_less_than**: Byte dizisi karşılaştırma (sıralama için)

### 📁 src/io.rs (84 satır)
- **print**: Temel yazdırma
- **print_number**: Sayı yazdırma
- **CStr**: C string wrapper (safe)
- **StaticBuffer**: Thread-safe buffer
- **read_line**: Satır okuma

### 📁 src/parser.rs (259 satır)
- **expand_env_vars**: Environment variable expansion
- **DirentParser**: Directory entry parsing (safe iterator)
- **find_in_path**: PATH'de komut arama
- **find_completions**: Tab completion logic
- **read_line_with_completion**: Interactive input (kullanılmıyor)

### 📁 src/builtins.rs (189 satır)
Builtin komutlar:
- **builtin_cd**: Dizin değiştirme
- **builtin_ls**: Dizin listeleme (alfabetik sıralı, max 128 entry)
- **builtin_pwd**: Çalışma dizini (placeholder)
- **builtin_echo**: Metin yazdırma (expansion ile)
- **builtin_export**: Env var tanımlama
- **builtin_history**: Geçmiş listeme
- **builtin_alias**: Alias tanımlama

### 📁 src/executor.rs (107 satır)
- **execute_command**: Ana komut çalıştırıcı
  - History tracking
  - Alias expansion
  - Builtin routing
  - Fork/exec for external commands

### 📁 src/thread.rs (84 satır)
- **StaticTid**: Thread ID storage
- **ThreadStack**: Thread stack allocation
- **ticker_func**: Background ticker thread
- **start_ticker_thread**: Thread başlatma

## Data Flow

```
Input (STDIN)
    ↓
read_line() [io.rs]
    ↓
execute_command() [executor.rs]
    ↓
    ├─→ trim_newline() [utils.rs]
    ├─→ HISTORY.add() [storage.rs]
    ├─→ split_first_word() [utils.rs]
    ├─→ ALIASES.get() [storage.rs]
    ↓
    ├─→ builtin_* [builtins.rs]
    │   ├─→ expand_env_vars() [parser.rs]
    │   ├─→ ENV_STORAGE.get/set() [storage.rs]
    │   └─→ DirentParser [parser.rs]
    │
    └─→ fork/execve [syscalls.rs]
```

## Module Dependencies

```
main.rs
  ├─→ syscalls.rs
  ├─→ io.rs
  │   └─→ syscalls.rs
  ├─→ storage.rs
  │   ├─→ syscalls.rs
  │   └─→ io.rs
  ├─→ utils.rs
  ├─→ parser.rs
  │   ├─→ syscalls.rs
  │   └─→ storage.rs
  ├─→ builtins.rs
  │   ├─→ syscalls.rs
  │   ├─→ storage.rs
  │   ├─→ utils.rs
  │   ├─→ parser.rs
  │   └─→ io.rs
  ├─→ executor.rs
  │   ├─→ syscalls.rs
  │   ├─→ storage.rs
  │   ├─→ utils.rs
  │   ├─→ builtins.rs
  │   ├─→ parser.rs
  │   ├─→ io.rs
  │   └─→ thread.rs
  └─→ thread.rs
      ├─→ syscalls.rs
      └─→ io.rs
```

## Key Design Patterns

### 1. Safe Wrapper Pattern
```rust
pub struct EnvStorage {
    vars: UnsafeCell<[[u8; 256]; 32]>,
    count: UnsafeCell<usize>,
}

impl EnvStorage {
    pub fn set(&self, name: &[u8], value: &[u8]) -> bool {
        // Unsafe isolated inside
    }
}
```

### 2. Static Global Pattern
```rust
pub static ENV_STORAGE: EnvStorage = EnvStorage::new();
pub static HISTORY: HistoryStorage = HistoryStorage::new();
pub static ALIASES: AliasStorage = AliasStorage::new();
```

### 3. Closure-based Mutation
```rust
pub fn with_mut<F, R>(&self, f: F) -> R 
where F: FnOnce(&mut [u8]) -> R {
    unsafe {
        let buf = &mut *self.data.get();
        f(buf)
    }
}
```

### 4. Iterator Pattern
```rust
pub struct DirentParser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> DirentParser<'a> {
    pub fn next(&mut self) -> Option<DirentEntry<'a>> {
        // Safe iteration over unsafe buffer
    }
}
```

## Benefits of Modularization

### Before (1 large file)
- ✗ 1124 satır main.rs
- ✗ Zor navigasyon
- ✗ Karışık dependencies
- ✗ Düşük maintainability

### After (9 modular files)
- ✓ En büyük dosya 466 satır
- ✓ Net sorumluluklar
- ✓ Bağımsız geliştirme
- ✓ Daha iyi organizasyon

## File Size Comparison

| Module | Lines | Purpose |
|--------|-------|---------|
| syscalls.rs | 466 | System calls |
| storage.rs | 296 | Data storage |
| parser.rs | 259 | Parsing logic |
| builtins.rs | 189 | Commands |
| main.rs | 115 | Entry point |
| executor.rs | 107 | Execution |
| utils.rs | 90 | Utilities + Sorting |
| io.rs | 84 | I/O helpers |
| thread.rs | 84 | Threading |
| **Total** | **1690** | **+84 lines** |

## Next Steps for Each Module

### syscalls.rs
- [ ] Add more syscalls (pipe, dup2, signal)
- [ ] Better error handling

### storage.rs
- [ ] Persistence (save/load)
- [ ] Larger limits
- [ ] Better memory management

### parser.rs
- [ ] Pipe parsing (|)
- [ ] Redirect parsing (>, <, >>)
- [ ] Quote handling
- [ ] Globbing support

### builtins.rs
- [ ] More commands (cat, grep, etc.)
- [ ] Better pwd implementation
- [ ] Help command

### executor.rs
- [ ] Pipeline execution
- [ ] Background jobs (&)
- [ ] Better error handling

### thread.rs
- [ ] Job control
- [ ] Signal handling
- [ ] Multiple background tasks

---

**Son Güncelleme**: 2024-11-22  
**Versiyon**: 0.4.0 (HTTP Server + pwd + Tab + Sorted ls)  
**Yenilikler**:
- ✅ **HTTP Server modülü** (server.rs - 336 satır)
  - Socket syscalls: socket, bind, listen, accept, setsockopt
  - HTTP/1.1 response generation
  - HTML directory listing
  - Configurable port (default: 8000)
- ✅ pwd komutu çalışıyor (getcwd syscall - buffer pointer handling düzeltildi)
- ✅ getcwd doğru implementasyonu (pointer → length dönüşümü)
- ✅ readlink syscall eklendi (bonus)
- ✅ Tab completion raw mode altyapısı (io.rs)
- ✅ Alfabetik sıralama algoritması (bubble sort)
- ✅ ls komutu sıralı çıktı
- ✅ Max 128 dosya desteği
