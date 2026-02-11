# Kiro Hesap Yöneticisi

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Logo" width="80">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platform">
  <img src="https://img.shields.io/github/v/release/hj01857655/kiro-account-manager?label=Sürüm&color=green" alt="Sürüm">
  <img src="https://img.shields.io/github/downloads/hj01857655/kiro-account-manager/total?color=brightgreen" alt="İndirmeler">
  <img src="https://img.shields.io/badge/Lisans-CC%20BY--NC--SA%204.0-orange" alt="Lisans">
</p>

<p align="center">
  <b>🚀 Kiro IDE hesaplarını akıllıca yönetin, tek tıkla geçiş, kota takibi</b>
</p>

<p align="center">
  🌐 <a href="README.md">简体中文</a> |
  <a href="README.en.md">English</a> |
  Türkçe
</p>

---

## 📥 İndirme

**Son Sürüm**: [GitHub Releases](https://github.com/hj01857655/kiro-account-manager/releases/latest)

| Platform | İndirme Bağlantısı |
|----------|---------------------|
| 🪟 **Windows** | [KiroAccountManager_x64_zh-CN.msi](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_x64_zh-CN.msi) |
| 🍎 **macOS (Intel)** | [KiroAccountManager_x64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_x64.dmg) |
| 🍎 **macOS (Apple Silicon)** | [KiroAccountManager_aarch64.dmg](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_aarch64.dmg) |
| 🐧 **Linux (AppImage)** | [KiroAccountManager_amd64.AppImage](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_amd64.AppImage) |
| 🐧 **Linux (deb)** | [KiroAccountManager_amd64.deb](https://github.com/hj01857655/kiro-account-manager/releases/latest/download/KiroAccountManager_amd64.deb) |

**Sistem Gereksinimleri**:
- **Windows**: Windows 10/11 (64-bit), WebView2 gerekli (Win11'de yerleşik)
- **macOS**: macOS 10.15+ (Intel / Apple Silicon evrensel)
- **Linux**: x86_64, WebKitGTK gerekli

---

## ✨ Temel Özellikler

### 🔐 Çevrimiçi Giriş

- **Sosyal Giriş** — Google / GitHub, masaüstü OAuth akışı ile otomatik token yenileme
- **IdC Giriş** — AWS IAM Identity Center (Builder ID ve Kurumsal hesaplar), tam SSO OIDC desteği

### 📊 Hesap Yönetimi

- Kart görünümü / Tablo görünümü geçişi
- Kota ilerleme çubukları (ana kota / deneme / bonus)
- Abonelik türü rozetleri (Free / PRO / PRO+)
- Token süre dolum geri sayımı
- Durum vurgulama (aktif / süresi dolmuş / yasaklı / mevcut)
- Yasaklama algılama (423 Locked / 403 TEMPORARILY_SUSPENDED)

### 🔄 Tek Tıkla Hesap Geçişi

- Kesintisiz Kiro IDE hesap geçişi
- Otomatik makine kimliği sıfırlama (rastgele / bağlı mod)
- Gerçek zamanlı geçiş ilerlemesi
- Yasaklı hesapları otomatik atlama

### 📦 Toplu İşlemler

**İçe ve Dışa Aktarma**
- JSON formatı (dosya içe aktarma / yapıştırma)
- Kiro IDE'den içe aktarma (oturum açmış hesabı otomatik algıla)
- kiro-cli'den içe aktarma (SQLite veritabanı okuma)
- JSON'a dışa aktarma (toplu seçim destekli)

**Toplu Yönetim**
- Toplu yenileme (akıllı eşzamanlılık kontrolü)
- Toplu silme / toplu etiketleme
- Uzaktan silme (AWS'den kayıt silme, yalnızca Google/GitHub)
- Anahtar kelime arama ve filtreleme

### 🏷️ Etiketler ve Gruplar

- Özel etiketler (ad / renk)
- Toplu etiket atama
- Hesap grupları ve gruba göre filtreleme

### 🔍 Gelişmiş Filtreleme

- Abonelik türüne göre filtrele (Free / PRO / PRO+)
- Duruma göre filtrele (aktif / yasaklı)
- Kullanım oranı / eklenme tarihi / deneme süresi bitimine göre sırala
- Üç durumlu sıralama (azalan → artan → yok)

### 🔌 Kiro Yapılandırması

- **MCP Sunucuları** — Ekleme, silme, düzenleme, etkinleştirme / devre dışı bırakma
- **Steering Kuralları** — Görüntüleme, düzenleme

### ⚙️ Sistem Ayarları

- 4 tema (Açık / Koyu / Mor / Yeşil)
- AI model seçimi ve kilitleme
- Otomatik token yenileme (yapılandırılabilir aralık)
- Geçişte otomatik makine kimliği sıfırlama
- Gizlilik modu (e-posta maskeleme)
- Düşük bakiyede otomatik hesap değiştirme (yapılandırılabilir eşik)
- Özel tarayıcı / otomatik algılama, gizli mod
- HTTP proxy / sistem proxy otomatik algılama

### 🔑 Makine Kimliği Yönetimi

- Görüntüleme / Kopyalama / Sıfırlama
- Windows / macOS / Linux desteği

### 🖥️ IDE Entegrasyonu

- Kiro IDE çalışma durumunu algılama
- Tek tıkla başlatma / durdurma
- Proxy ve model ayarlarını otomatik senkronize etme

### ⚡ API Proxy Hizmeti

OpenAI uyumlu bir API'ye mi ihtiyacınız var? Bağımsız proje **[kiro-gateway](https://github.com/hj01857655/kiro-gateway)** kullanın

---

## 📸 Ekran Görüntüleri

![Ana Sayfa](screenshots/首页.webp)
![Hesap Yönetimi](screenshots/账号管理.webp)
![Masaüstü Yetkilendirme](screenshots/桌面授权.webp)
![Kural Yönetimi](screenshots/规则管理.webp)
![Ayarlar](screenshots/设置.png)
![Hakkında](screenshots/关于.png)

---

## 💬 Geri Bildirim

- 🐛 [Sorun Bildir](https://github.com/hj01857655/kiro-account-manager/issues)
- 💬 QQ Grubu: [1020204332](https://qm.qq.com/q/Vh7mUrNpa8)

---

## ❓ Sık Sorulan Sorular

**S: Hesap geçişinde "bearer token invalid" hatası?**
C: Token süresi dolmuş. Geçiş yapmadan önce "Yenile" düğmesine tıklayın.

**S: Token yenileme başarısız oldu?**
C: Ağ zaman aşımı. Tekrar yenilemeyi deneyin veya ağ değiştirin.

---

## 🔗 İlgili Projeler

- **[kiro-gateway](https://github.com/hj01857655/kiro-gateway)** — Kiro API Ağ Geçidi, OpenAI/Anthropic uyumlu arayüz

---

## 📄 Lisans

[CC BY-NC-SA 4.0](LICENSE) — **Ticari kullanım yasaktır**

## ⚠️ Sorumluluk Reddi

Bu yazılım yalnızca öğrenme ve iletişim amaçlıdır. **Ticari kullanım kesinlikle yasaktır.** Bu yazılımın kullanımından doğan tüm sonuçlardan kullanıcı sorumludur.

**⚠️ Bu proje kalıcı olarak ücretsizdir! Birisi sizden ücret alıyorsa, dolandırılıyorsunuz!**

---

<p align="center">❤️ ile yapıldı — hj01857655</p>