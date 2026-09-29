# OmaRank - Omarchy Linux İçin Donanım Kıyaslama ve Seviye Derecelendirme Motoru

[![Omarchy Verified Plugin](https://img.shields.io/badge/Omarchy-Verified_Plugin-22c55e?style=for-the-badge&logo=omarchy)](https://github.com/ozdil)
[![Buy Me A Coffee](https://img.shields.io/badge/Buy_Me_A_Coffee-Support_Development-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/ozdil)

Omarchy masaüstü ortamı için yerel donanım tespiti, 6 bileşenli ağırlıklı silikon puanlama ve OmaStat topluluk derecelendirme eklentisi.

Geliştirici: Ozan Özdil (ozdil)  
Lisans: MIT  
Eklenti Kimliği: ozdil.omarank  

---

## Öne Çıkan Özellikler

- Yerel Rust Motoru (`omarank-engine`): Linux sysfs (`/proc/cpuinfo`, `/proc/meminfo`, `/sys/class/drm/`, `/sys/class/dmi/id/`, `lspci`, `inxi`, `hyprctl`) üzerinden sıfır gecikmeli donanım tespiti.
- 0 - 100 Ağırlıklı OmaScore (6 Bileşenli Analiz):
  - İşlemci / CPU (%24): Fiziksel çekirdekler, iş parçacıkları (threads), saat frekansı.
  - Grafik / GPU (%32): Harici GPU seviyesi, sürücü tespiti (`xe`, `amdgpu`, `nvidia`).
  - Bellek / RAM (%16): Nesil (DDR3/DDR4/DDR5), MT/s veri aktarım hızı, kanal mimarisi ve kapasite.
  - Anakart (%8): Yongaseti mimarisi ve segment sınıflandırması.
  - Ekran / Monitör (%15): Yüksek yenileme hızı (Hz) ve ultra geniş ekran (ultrawide) tespiti.
  - Depolama (%5): NVMe model tanıma ve PCIe nesil sınıflandırması.
- OmaStat Küresel Sıralama ve Seviye Matrisi:
  - Tahmini deterministik dünya sıralaması (World Rank).
  - S+'tan F'ye tam donanım seviye sınıflandırması.
- Yerel Quickshell Bar Widget'ı (`Panel.qml`):
  - Omarchy bar stiliyle uyumlu dinamik Nerd Font skor ikonu.
  - İşlemci, Grafik, Bellek, Anakart, Depolama, Ekran, Dünya Sıralaması ve Toplam Skoru gösteren 4 sütunlu telemetri ızgarası.
  - Tek tıkla değer kopyalama ve arayüz içi Künye (About) modali.

---

## Gereksinimler

- cargo ve rustc (Rust derleme zinciri)
- curl (OmaStat topluluk anket gönderimi için)
- inxi ve pciutils (Gelişmiş donanım dökümü için)

---

## Kurulum ve Derleme

```bash
# Eklenti dizinine gidin
cd ~/.config/omarchy/plugins/ozdil.omarank

# Motoru derleyin
cargo build --release

# İkiliyi kurun
cp target/release/omarank-engine ./omarank-engine
cp target/release/omarank-engine ~/.local/bin/omarank-engine
```

---

## Doğrulama ve Testler

```bash
# Birim testleri çalıştırın
cargo test

# Omarchy eklenti doğrulamasını çalıştırın
omarchy plugin validate .
```
