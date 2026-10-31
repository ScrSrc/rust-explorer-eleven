# Rust Explorer Eleven 🗂️

Клон проводника Windows 11, написанный на **Rust** с использованием **egui/eframe**. 
Лёгкий, быстрый и нативный — без Electron и тяжёлых зависимостей.

[![Latest Release](https://img.shields.io/github/v/release/ScrSrc/rust-explorer-eleven?label=Скачать%20.exe&style=for-the-badge&color=0078D4)](https://github.com/ScrSrc/rust-explorer-eleven/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
![Windows](https://img.shields.io/badge/Windows-10%2F11-0078D4?style=for-the-badge&logo=windows)

---

## 📸 Скриншоты

### Тёмная тема (основной вид)
![Тёмная тема](docs/images/main-dark.png)

### Светлая тема
![Светлая тема](docs/images/main-light.png)

### Окно настроек
Адаптивное окно параметров с прокруткой и прижатой кнопкой «Готово».
![Настройки](docs/images/settings.png)

### Контекстное меню
![Контекстное меню](docs/images/context-menu.png)

---

## ✨ Возможности

- 🗂️ **Вкладки** — как в настоящем проводнике: открытие, закрытие, переключение (`Ctrl+T`, `Ctrl+W`)
- 🎨 **Темы** — светлая, тёмная и «как в системе»; 8 цветов акцента + свой цвет (RGB-слайдеры)
- 🌍 **8 языков** — Русский, English, Українська, Қазақша, 中文, Español, Português, Türkçe
- 🔍 **Поиск** — рекурсивный поиск по текущей папке в фоне (без зависаний UI)
- 🖼️ **Нативные иконки** — через Windows Shell (`IShellItemImageFactory`), включая миниатюры для изображений
- 📋 **Быстрый доступ** — рабочий стол, загрузки, документы, изображения, музыка, видео, OneDrive
- 🌐 **Сетевые расположения** — обзор `\\server\share` через `WNetEnumResource`
- ⚙️ **Гибкие настройки** — скрытые элементы, режим редактирования панели навигации, язык, тема
- 🖱️ **Полный набор операций** — копировать, вырезать, вставить, переименовать, удалить в корзину
- ⌨️ **Горячие клавиши** — как в Windows Explorer (`Ctrl+C/X/V`, `F2`, `F5`, `Delete`, `Alt+←`, `Alt+→`)

---

## 📥 Скачать

Готовый `.exe` (Windows 10/11, x64) — в разделе **[Releases](https://github.com/ScrSrc/rust-explorer-eleven/releases/latest)**.

Просто скачайте `win11_explorer.exe` и запустите. Установка не требуется.

---

## 🛠️ Сборка из исходников

Требуется **Rust** (stable, 1.75+) и **Visual Studio Build Tools** (для линковки).

```bash
git clone https://github.com/ScrSrc/rust-explorer-eleven.git
cd rust-explorer-eleven
cargo build --release
