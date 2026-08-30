## [1.0.14] - 2026-08-30

### 🚀 Features

- *(release)* Add mise task to release

### 🐛 Bug Fixes

- *(player)* Restore buffering for VOD playback (#41)
- *(home)* Allow typing 'i' in search input when focused (#39)
- *(settings)* Allow theme list to scroll beyond visible viewport (#38)
## [1.0.13] - 2026-08-03

### 🚀 Features

- *(login)* Add dual QR code rendering in login UI
## [1.0.12] - 2026-07-14

### 🚀 Features

- 添加 MPV Bilibili SponsorBlock 选项以跳过广告
- Add Bangumi functionality with timeline and ranking
- Improve browsing and resilient playback
- Persist CDN ranking metrics
- Rank regional CDN catalog
- *(player)* Optimize danmaku rendering and low-latency playback

### 🐛 Bug Fixes

- Harden playback and playlist lifecycle

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 1.0.12 and update CHANGELOG
## [1.0.11] - 2026-04-20

### 🚀 Features

- 添加缓存可见行数以优化滚动和预加载逻辑

### 🐛 Bug Fixes

- 优化行加载逻辑以改善滚动体验
- Key binding conflicts in dynamic page
## [1.0.10] - 2026-04-07

### 🚀 Features

- Support guest mode without forced login

### 🐛 Bug Fixes

- *(history)* 支持直播历史回车进入直播详情

### 🚜 Refactor

- Decouple network worker from UI loop
- 重构工程分层并拆分 App 模块

### 📚 Documentation

- Update CHANGELOG.md for version 1.0.9

### ⚙️ Miscellaneous Tasks

- *(release)* Bump to 1.0.10
## [1.0.9] - 2026-01-16

### 🚀 Features

- Add live streaming functionality with recommendations and detail pages
- *(ui)* Enhance QR code display with image support in login page

### ⚙️ Miscellaneous Tasks

- Update CHANGELOG.md for version 1.0.8
- Bump version to 1.0.9
## [1.0.8] - 2026-01-13

### 🚀 Features

- *(ui)* Display dynamic keybindings in help text
- Add support for multi-part video episodes and selection

### ⚙️ Miscellaneous Tasks

- Bump version to 1.0.8
## [1.0.7] - 2026-01-12

### 🚀 Features

- *(nav)* Support nav back on search & home
- *(app)* Add home page caching and refresh action
- Add custom keybindings support and persistence
- *(ui)* Improve QR code scannability and centering

### 📚 Documentation

- Update CHANGELOG.md for version 1.0.7

### ⚙️ Miscellaneous Tasks

- Bump version to 1.0.7
## [1.0.6] - 2026-01-09

### 🚜 Refactor

- *(wbi)* Simplify URL encoding logic and improve readability

### ⚙️ Miscellaneous Tasks

- Bump version to 1.0.6
- Update CHANGELOG for version 1.0.6 and add refactor details
## [1.0.5] - 2026-01-09

### 🚀 Features

- Add Homebrew dependencies and enhance mouse support in README
- *(deps)* Update Rust dependencies to latest versions
- Add installation instructions for yay and paru in README
- *(search)* Implement hot search feature with API integration and UI updates

### 🚜 Refactor

- *(ui)* Standardize keyboard navigation across pages

### 📚 Documentation

- *(readme)* Add Homebrew installation instructions

### ⚙️ Miscellaneous Tasks

- Bump version to 1.0.5 and add CHANGELOG
- *(dist)* Move homebrew dependencies to dist configuration
## [1.0.4] - 2026-01-09

### 🚀 Features

- Add Nix flake for NixOS development
- *(flake)* Add alejandra formatter
- *(comment)* Add comment interaction functionality
- *(ui)* Add mouse scrolling and click support for dynamic page

### 🐛 Bug Fixes

- Remove global OPENSSL_STATIC for native builds

### 🚜 Refactor

- *(ui)* Remove progress bar overlay from history cards

### 📚 Documentation

- Add NixOS installation instructions

### ⚙️ Miscellaneous Tasks

- *(release)* Add homebrew formula publishing to release workflow
- Bump version to 1.0.4
- Add homepage field in Cargo.toml
## [1.0.3] - 2026-01-07

### 🚀 Features

- Implement Bilibili video playback heartbeat reporting to track watch progress
- Implement watch history page with grid layout and cover image loading

### 🚜 Refactor

- Extract page navigation logic, improve cookie parsing, use UI constants, and enhance API client lock handling and POST request structure.

### 📚 Documentation

- Update README

### ⚙️ Miscellaneous Tasks

- *(ci)* Remove AUR publishing workflow
- Bump version to 1.0.3
## [1.0.2] - 2026-01-06

### ⚙️ Miscellaneous Tasks

- *(AUR)* Update workflow to trigger on release workflow
- Bump version to 1.0.2
## [1.0.1] - 2026-01-06

### 🚀 Features

- Add AUR publishing workflow and update README with version badge
- Add new theme color variables and apply them to various UI components for improved styling.

### 📚 Documentation

- Update project documentation

### ⚡ Performance

- Dynamically adjust video card prefetching to improve smooth scrolling

### ⚙️ Miscellaneous Tasks

- Bump version to 1.0.1
## [1.0.0] - 2026-01-05

### 🚀 Features

- *(login)* Add QR code login page and API client
- Add dynamic feed and search functionality with sidebar navigation
- Implement video card grid with async cover loading and video detail page.
- *(dynamic)* Enhance dynamic feed with tabbed navigation and UP master selection
- Add theme support, settings page, configurable keybindings, and load more comments functionality
- Add dynamic detail page for image/text dynamics

### 💼 Other

- Version 1.0.0
- Version 1.0.0

### 🚜 Refactor

- Migrate player to `tokio::process` with async cookie cleanup and simplify `ApiClient` access by removing `tokio::sync::Mutex`.

### ⚙️ Miscellaneous Tasks

- Add pre-commit configuration
