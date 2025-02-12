# 🎥 Multi-Platform Screen Casting

🚀 **A powerful Rust-based screen casting application for seamless screen sharing across Windows, macOS, and Linux.**

## 🌟 Features

✅ **Cross-Platform Compatibility** – Works on Windows, macOS, and Linux effortlessly.  
✅ **Broadcast & Receive Modes** – Stream your screen or connect to an ongoing stream.  
✅ **Custom Capture Area** – Select and share only the screen portion you need.  
✅ **Intuitive User Interface** – A smooth, user-friendly experience for both casters and receivers.  
✅ **Hotkey Support** – Quickly start, pause, or stop streaming with customizable keyboard shortcuts.  
✅ **Annotation Tools** – Draw, highlight, and add text while streaming.  
✅ **Recording Option** – Save the received stream as a video file for later use.  
✅ **Multi-Monitor Support** – Choose which display to cast from in multi-screen setups.  

---

## 🛠️ Installation

### Prerequisites

- Install [Rust](https://www.rust-lang.org/)
- Ensure `cargo` is available

### Clone the Repository

```sh
git clone https://github.com/rosfi12/Screen_Casting-Rust.git
cd screen-casting
```

### Build & Run

```sh
cargo build --release
cargo run
```

---

## 📂 Project Structure

📌 **`src/main.rs`** – Entry point of the application, responsible for handling user input, managing modes, and launching the core functionality.  
📌 **`src/enums.rs`** – Defines the key enums used throughout the app, such as stream modes, error types, and configuration states.  
📌 **`src/display_manager.rs`** – The central hub for managing the screen capture process, interacting with system APIs, and optimizing performance.  

### 📁 `src/display_manager/`

🔹 **`converter.rs`** – Converts screen data into a streamable format, ensuring optimal compression and quality.  
🔹 **`monitor.rs`** – Detects and manages multiple screens, allowing users to select which display to stream.  
🔹 **`network.rs`** – Handles peer-to-peer connections, ensuring stable and low-latency streaming.  
🔹 **`transformer.rs`** – Applies necessary transformations like resizing, encoding, and color adjustments before transmission.  
🔹 **`video_handler.rs`** – Processes raw screen data, prepares it for streaming, and enables video recording functionality.  

---

## 🎮 How to Use

1️⃣ **Select Mode** – Choose between broadcasting your screen or receiving a stream.  
2️⃣ **Customize Settings** – Adjust capture area, resolution, and streaming preferences.  
3️⃣ **Start Streaming** – Begin sharing your screen in real time with low latency.  
4️⃣ **Interact Live** – Use hotkeys to pause/resume or annotate directly on the stream.  

---

## 🔧 Configuration
Modify the `config.toml` file to adjust settings like resolution, bitrate, and network preferences.

---

## 🚀 Future Enhancements

- 🔹 WebRTC integration for higher-quality streaming.
- 🔹 Mobile support to allow streaming on Android/iOS devices.
- 🔹 Secure encryption for protected streaming sessions.

---

## 🤝 Contributing
Pull requests are welcome! Open an issue for feature requests or bug reports.

---

## 📜 License
MIT License – Free to use and modify.

---

🎬 **Start streaming like a pro!**

