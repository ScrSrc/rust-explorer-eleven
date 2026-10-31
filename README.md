![Главная](Home.png)

![Скриншот 3](3.png)

![Скриншот 2](2.png)

![Скриншот 4](4.png)

---

## Установка

### Способ 1: Скачать готовый установщик

Перейдите в раздел [Releases](https://github.com/ScrSrc/rust-explorer-eleven/releases/latest) и скачайте файл `win11_explorer.exe`. Запустите его от имени администратора.

### Способ 2: Сборка из исходников

Требуется:
- [Rust](https://rustup.rs/) (stable)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ Build Tools)
- [WiX Toolset v3.14](https://github.com/wixtoolset/wix3/releases) (для сборки установщика)

```bash
# Клонирование репозитория
git clone https://github.com/ScrSrc/rust-explorer-eleven.git
cd rust-explorer-eleven

# Сборка исполняемого файла
cargo build --release

# Сборка установщика (опционально)
cargo install cargo-wix
cargo wix init
cargo wix
