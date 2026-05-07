# 🚀 Reset Trial

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Reset Trial** is a sleek, modern, and lightweight GUI application built in Rust. It is designed to help users quickly find and manage leftover files and directories associated with trial software, ensuring a clean system.

## ✨ Features

- **⚡ Asynchronous Search**: High-performance file system scanning that keeps the UI fluid and responsive.
- **🎨 Glassmorphic UI**: A contemporary dark theme with semi-transparent panels, smooth transitions, and premium typography.
- **🔍 Real-time Filtering**: Instantly filter search results as you type to find exactly what you're looking for.
- **📂 Explorer Integration**: Open files or directories directly in your system's native file explorer (Explorer, Finder, or xdg-open).
- **📜 Search History**: Keep track of your previous searches for quick access and repeated cleanups.
- **⚠️ Safe Deletion**: Integrated confirmation dialogs to prevent accidental data loss.

## 🛠️ Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **GUI Framework**: [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)
- **File System**: [walkdir](https://github.com/BurntSushi/walkdir)
- **Path Utilities**: [dirs](https://github.com/dirs-dev/dirs-rs)
- **Serialization**: [serde](https://serde.rs/) / [serde_json](https://github.com/serde-rs/json)

## 🚀 Getting Started

### Prerequisites

Ensure you have the Rust toolchain installed. If not, you can get it from [rustup.rs](https://rustup.rs/).

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/ayonsaha2011/reset-trial.git
   cd reset-trial
   ```

2. **Build the application:**
   ```bash
   cargo build --release
   ```

3. **Run the application:**
   ```bash
   cargo run --release
   ```

## 📖 Usage

1. **Search**: Type the name of the software (e.g., "Adobe", "IntelliJ") in the search bar and press Enter.
2. **Review**: Browse through the discovered files and directories.
3. **Filter**: Use the filter bar to narrow down the results.
4. **Action**:
   - Click the 📂 icon to view the item in your file explorer.
   - Click the 🗑 icon to delete an individual item.
   - Use the **DELETE ALL** button for a complete cleanup.

## 📜 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details (if applicable).

---
*Developed with ❤️ by [Ayon Saha](mailto:ayonsaha2011@gmail.com)*
