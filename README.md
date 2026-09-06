# ⚡ SonicAura AI: Universal Audio Enhancer & Tinnitus Relief Suite in Rust
### Dolby Atmos & Bang & Olufsen-Grade Sound • Clinical Tinnitus Relief • Adapts to ANY Earphones • Dark & Light Themes

A high-performance, ultra-low latency, psychoacoustic AI sound enhancement and acoustic therapy engine written in pure **Rust**.

Transforms laptop speakers, external soundbars, cheap earbuds, Apple AirPods, high-end IEMs, and studio headphones into an immersive, crystal-clear acoustic experience inspired by **Dolby Atmos**, **Bang & Olufsen BeoPlay**, and **Apple Spatial Audio** — while providing clinical-grade **Tailor-Made Notched Music Training (TMNST)** and **Narrowband Acoustic Masking** for tinnitus relief.

---

## 🩺 1. Interactive Tinnitus Relief & Acoustic Therapy Studio

SonicAura AI includes a clinical acoustic therapy engine built directly into the live audio pipeline right before the True-Peak brickwall limiter:

| Therapy Feature | Acoustic Mechanism & Clinical Protocol | Controls & Capabilities |
|---|---|---|
| **TMNST (Tailor-Made Notched Music Training)** | Filters out your exact tinnitus ringing frequency from any music or system sound (YouTube, Spotify, games). Stimulates **lateral inhibition** across auditory cortex neurons, retraining maladaptive brain hyperactivity over time (PNAS protocol). | Selectable notch filter bandwidth ($Q=2.0$ Wide, $Q=4.0$ Standard, $Q=8.0$ Sharp Notch). Real-time center frequency adjustment (500 Hz – 18,000 Hz). |
| **Narrowband Pink Noise Masker** | Synthesizes an organic 3-pole pink noise pillow passed through a tuned stereo bandpass filter centered at your tinnitus pitch. Blends and softens the ringing sensation without ear fatigue. | Attenuation range from gentle background pillow (`-70.0 dB`) to therapeutic masking (`-12.0 dB`). |
| **Combined Acoustic Therapy** | Simultaneously notches system audio while injecting shaped narrowband ambient noise for comprehensive acoustic relief. | Active notched music + soothing shaped ambient noise pillow. |
| **Pitch-Matching Sine Calibrator** | Pure sine wave generator calibrated at a safe, soft level (`-30 dBFS`) with smooth 15ms de-clicking crossfades. Allows pinpointing your exact ringing frequency before engaging therapy. | Hotkey **`[T]`** or **`[Space]`** to toggle on/off. Sweep pitch in 100 Hz increments with **`[←]`** / **`[→]`**. |
| **Selective Ear Targeting** | Supports unilateral and asymmetrical tinnitus. Directs therapy to the affected ear while leaving the other untouched. | `Both Ears`, `Left Ear Only`, `Right Ear Only`. |
| **True-Peak Limiter Guard** | All therapeutic signals pass through the 64-sample lookahead brickwall limiter. | Guarantees the output never clips or exceeds safe listening thresholds. |

---

## 🎨 2. Universal Dark & Light Theme Engine

SonicAura features an adaptive dual-theme design system engineered for high-contrast visibility and legibility across all terminal configurations:

- **🌙 Dark Theme (Default)**: Cyberpunk-inspired palette featuring neon cyan (`#00FFFF`), luminous light cyan (`#55FFFF`) for razor-sharp label legibility, emerald green, and vivid amber accents on dark terminal backgrounds.
- **☀️ Light Theme**: Clean studio aesthetic with a soft canvas (`#F5F6FA`), dark slate typography (`#3C4655`), deep blue borders, and emerald/amber indicators.
- **Instant Hotkey Toggle**: Press **`[L]`** anywhere in the TUI to toggle between Dark and Light mode in real-time.
- **Persistent Choice**: Theme preference is automatically saved in `~/.config/sonic_aura/config.toml`.

---

## 🎧 3. Universal Earphone Hardware Calibration Profiles

SonicAura AI auto-calibrates to overcome the physical driver limitations of any earphone in the market:

| Earphone Profile | Target Hardware | Acoustic Calibration & DSP Fixes |
|---|---|---|
| **Budget Earbuds Fix** | Cheap $5–$20 earbuds, airline earphones | Injects psychoacoustic missing fundamental sub-bass (+1.5x), notches out 3.2kHz harsh plastic resonance, eliminates 250Hz boxy mud, and synthesizes missing >10kHz air sheen. |
| **Apple AirPods / TWS** | AirPods 1/2/3/Pro/Max, Galaxy Buds, Sony TWS | Harman In-Ear Target Curve calibration, wide 3D Atmos soundstage, open-fit low-end compensation, and crystalline vocal presence. |
| **Bass-Heavy Buds** | Beats, Skullcandy, Sony Extra Bass | De-bloats muddy 150–300Hz mid-bass hum, lifts buried vocal formants (+3.5dB @ 2.8kHz), and sharpens drum/percussion transient snap. |
| **Audiophile IEMs** | Moondrop, Chi-Fi planar, Sennheiser IE, Shure | Harman 2019 Reference curve tuning with Meier HRTF binaural crossfeed (pulls audio out of the head onto virtual mastering studio monitors) and 6.5kHz sibilance de-harshing. |
| **Studio Open-Back** | Sennheiser HD600/650/800, DT990/1990, Hifiman | Sub-bass extension shelf (+5dB @ 40Hz) to restore open-back bass roll-off, smooths 8.5kHz treble peaks, and expands depth. |
| **Studio Closed-Back** | Audio-Technica ATH-M50x, Sony MDR-7506, DT770 | Eliminates 220Hz enclosed earcup resonance reflections, widens narrow soundstage, and delivers mastering clarity. |
| **Universal Neutral** | Reference flat studio monitors | Transparent uncolored baseline. |

---

## 🌍 4. Adaptive Environmental Noise & Acoustic Context Engine

Dynamic psychoacoustic anti-masking adapts automatically to your real-world acoustic surroundings:

| Environment Mode | Acoustic Context | DSP Anti-Masking Adaptation |
|---|---|---|
| **🏙️ Busy City & Traffic** | Engine rumble, buses, street noise (40–150Hz) | Dynamic sub-bass anti-masking lift (+5dB), AI vocal intelligibility booster (+3.5dB @ 2kHz) so podcasts/dialogue cut cleanly through traffic, and punchy transient leveling. |
| **✈️ Commute & Transit** | Airplanes, subways, train cabin drone | Low-frequency cabin drone immunity, dialogue-priority multi-band dynamic compression for fatigue-free travel listening. |
| **☕ Cafe & Office** | Speech chatter (500Hz–2kHz), clattering dishes | Vocal Formant Focus + 3D soundstage widening to psychoacoustically separate music/vocals from ambient crowd noise. |
| **🍃 Quiet Remote / Nature** | Silent room, mountain cabin, library (<20dB noise) | Audiophile pure dynamic range mode: ultra-gentle transparent compression, maximum 3D holographic soundstage depth, and pristine micro-detail resolution. |
| **🌙 Late-Night Whisper** | Ultra-low volume listening | Aggressive Fletcher-Munson equal-loudness curve ensures deep sub-bass and crisp vocal breath even when listening at 10%–15% volume. |
| **🏠 Balanced Studio** | Standard indoor room | Neutral balanced acoustic compensation. |

---

## 🔬 5. Commercial-Grade DSP Architecture

- **True Stereo RNNoise Suppression**: Dual-channel deep neural network speech enhancement and ambient noise gating.
- **AutoEQ Database Integration**: Live parametric EQ curve fetching directly from the AutoEQ project for thousands of headphone models.
- **FFT Overlap-Add Convolution Reverb**: High-resolution room acoustics and impulse response processing.
- **Multiband Upward Compression (OTT-Style)**: Pulls up micro-dynamics and tightens soundstage punch without harsh clipping.
- **EBU R128 / ITU-R BS.1770 LUFS Normalization**: Rolling perceived loudness calculation with smooth auto-gain riding.
- **Linear-Phase FIR Equalizer**: Zero phase-smearing mastering equalizer.
- **Virtual Audio Routing**: Real-time PipeWire loopback sink manager with non-blocking supervisor, de-clicked 15ms bypass crossfade, and 18Hz infrasonic DC rumble blocking filter.
- **Zero-Latency Audio Mesh Sync**: Precision UDP multicast broadcaster and receiver for multi-device synchronized audio.

---

## 🎛️ 6. 10-Band ISO Precision Equalizer & Presets

- Direct Form II Transposed Bi-quad IIR filters with click-free parameter interpolation.
- **Factory Presets**:
  - `Dolby Atmos Cinema 3D`: Deep cinematic sub-bass, crystal dialogue presence, immersive 360 soundfield.
  - `Bang & Olufsen Signature`: BeoPlay acoustic warmth, silky smooth highs, open soundstage.
  - `Apple Spatial Air & Punch`: Modern punchy low-end, crisp vocal clarity, pristine sparkle.
  - `Laptop Speaker Resonator Fix`: Eliminates boxy chassis resonance (300–600 Hz), psychoacoustic sub-bass, stereo widener.
  - `DTS:X Spatial Gaming 3D`: Snappy footstep transients, pinpoint directional cues, explosion rumble.
  - `Audiophile Reference Headphones`: Harman target curve, transparent Meier binaural crossfeed.
  - `Vocal / Podcast / Dialogue Clarity AI`: Speech intelligibility boost and background noise suppression.
  - `Club / EDM / Hip-Hop Ultra Punch`: Maximum sub-harmonic drive and dynamic loudness contour.
  - `Acoustic / Classical Concert Hall`: Concert hall acoustic depth with transparent highs.
  - `Flat Reference / Bypass`: Direct passthrough for A/B testing.

---

## ⚡ Performance Benchmark Results

Tested at 48,000 Hz stereo audio on a single CPU core:
- **Throughput Speed**: **~34x Real-time**
- **Processing Rate**: **1.67 Million samples/second**
- **Latency**: **600 nanoseconds** per sample frame
- **CPU Load**: **< 2.9%** on a single CPU core

---

## 🚀 Quick Start & CLI Flags

```bash
# Launch interactive TUI
sonic_aura

# Launch with Tinnitus Relief active at 6,500 Hz
sonic_aura --tinnitus-mode notch --tinnitus-freq 6500 --tinnitus-ear both

# Launch in Light Theme
sonic_aura --theme light

# Instant audition demo (binaural synthesizer)
sonic_aura --demo

# Specify earphone and environment directly from CLI:
sonic_aura --earphone budget --env city
sonic_aura --earphone iem --env remote
sonic_aura --earphone airpods --env cafe
```

### CLI Options:
| Flag | Description | Values / Defaults |
|---|---|---|
| `--theme` | Initial UI theme mode | `dark` (default), `light` |
| `--tinnitus-mode` | Tinnitus therapy mode | `off` (default), `notch`, `masking`, `combined` |
| `--tinnitus-freq` | Tinnitus center frequency in Hz | `500.0` – `18000.0` (default: `6000.0`) |
| `--tinnitus-ear` | Target ear for tinnitus therapy | `both` (default), `left`, `right` |
| `--earphone` | Earphone calibration profile | `budget`, `airpods`, `bass`, `iem`, `openback`, `closedback`, `neutral` |
| `--env` | Environmental anti-masking mode | `city`, `transit`, `cafe`, `remote`, `night`, `studio` |
| `--demo` | Run with built-in binaural synth | Self-contained audio testing |

---

## ⌨️ TUI Keyboard Shortcuts

| Key | Action |
|---|---|
| **`[Tab]`** | Cycle panel focus: `Presets` ➔ `Equalizer` ➔ `AI Boost` ➔ `Tinnitus Studio` |
| **`[X]`** | Quick jump / toggle focus directly to **Tinnitus Relief Studio** |
| **`[L]`** | Toggle between **Dark** and **Light** themes |
| **`[T]`** | In Tinnitus panel: toggle **Pitch-Matching Sine Tone** (`-30 dBFS`). Elsewhere: toggle Audio loopback/synth |
| **`[P]`** | Cycle Presets (*Dolby Atmos*, *Bang & Olufsen*, *Apple Spatial*, *Laptop Fix*, *DTS:X*, etc.) |
| **`[E]`** | Cycle **Earphone Profiles** (*Budget*, *AirPods*, *Bass*, *IEM*, *Open-Back*, *Closed-Back*, *Neutral*) |
| **`[N]`** | Cycle **Environment Modes** (*City*, *Transit*, *Cafe*, *Remote*, *Night*, *Studio*) |
| **`[Space]`** | Instant **A/B Bypass Toggle** (or toggle Sine Test Tone when inside Tinnitus Studio) |
| **`[↑] / [↓]`** | Navigate between sliders or parameters in the focused panel |
| **`[←] / [→]`** | Adjust value of the active parameter (sweep pitch, gain, notch $Q$, or volume) |
| **`[0]`** | Reset focused parameter to default |
| **`[S]`** | Save current EQ, theme, and tinnitus calibration to `~/.config/sonic_aura/config.toml` |
| **`[Q]`** | Exit cleanly (restoring system audio sinks and routes) |

---

## 🐧 Linux Requirements & Dependencies

### 1. Runtime Requirements (To Run the Program)

SonicAura uses Linux audio routing to capture system sound, process it through the DSP pipeline, and output it to your physical speakers/headphones.

| Component | Required Tool / Package | Why It’s Needed |
|---|---|---|
| **Audio Server** | **PipeWire** (recommended) or **PulseAudio** | System sound server. PipeWire is the default on Ubuntu 22.10+, Zorin OS 18+, Fedora 34+, Arch, Debian 12+. |
| **Routing & Control** | **`pactl`** (from `pulseaudio-utils` or `pipewire-pulse`) | Used by `virtual_device.rs` to create the virtual `SonicAura_Sink`, route audio, and mirror hardware volume controls. |
| **Audio Capture** | **`pw-record`** & **`pw-link`** (from `pipewire` / `pipewire-bin`), or **`parec`** | Used for zero-latency system monitor loopback capture without capturing microphone noise. |
| **ALSA Libraries** | **`libasound2`** | Required by the Rust audio backend (`cpal`). |
| **Terminal** | Any modern terminal emulator (GNOME Terminal, Alacritty, Kitty, WezTerm, Konsole, etc.) | Terminal with ANSI / 24-bit color support for the TUI visualizer. |

### 2. Build Requirements (To Compile from Source)

If you are building from source using `cargo build --release`:
- **Rust Toolchain**: Rust **1.85+** (Rust 2024 edition).
- **C Compiler**: `gcc` / `clang` & `make` (`build-essential`).
- **ALSA Development Headers**: `libasound2-dev` (needed by `cpal`).
- **OpenSSL Development Headers**: `libssl-dev` (needed by `reqwest` for AutoEQ database downloads).
- **`pkg-config`**: To allow Cargo to detect system C libraries.

### 3. Quick One-Liner Install by Distribution

#### **Ubuntu / Debian / Zorin OS / Linux Mint / Pop!_OS**:
```bash
# Runtime dependencies:
sudo apt update && sudo apt install -y pipewire-bin pulseaudio-utils libasound2

# Build dependencies (only if building from source):
sudo apt install -y build-essential pkg-config libasound2-dev libssl-dev
```

#### **Fedora / RHEL / AlmaLinux**:
```bash
# Runtime dependencies:
sudo dnf install -y pipewire-utils pulseaudio-utils alsa-lib

# Build dependencies (only if building from source):
sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config alsa-lib-devel openssl-devel
```

#### **Arch Linux / Manjaro**:
```bash
# Runtime dependencies:
sudo pacman -S --needed pipewire pipewire-pulse libpulse alsa-lib

# Build dependencies (only if building from source):
sudo pacman -S --needed base-devel openssl
```

### 4. Verify System Readiness

Run this one-liner to verify all required audio tools are available:
```bash
which pactl pw-record pw-link parec
```

---

## 🪟 Windows 11 & WSL2 Compatibility

### Overview & Comparison

| Feature | Inside WSL2 (Windows 11) | Native Windows 11 (PowerShell) |
|---|:---:|:---:|
| **TUI Interface & Themes** | ✅ Works | ✅ Works (best in Windows Terminal) |
| **Binaural Demo Synth (`--demo`)** | ✅ Plays to Windows speakers | ✅ Plays to Windows speakers |
| **Offline WAV Processing** | ✅ Full support | ✅ Full support |
| **Tinnitus Therapy Studio** | ✅ Full support | ✅ Full support |
| **Enhance Windows Apps (YouTube/Spotify)** | ❌ VM boundary blocks Windows host audio | ✅ Full system audio access via WASAPI |
| **Audio Latency** | Minor VM overhead (~10–20ms) | Native ultra-low latency (< 1ms) |

---

### Running inside WSL2 (Linux on Windows)

Windows 11 includes **WSLg** (WSL GUI & Audio architecture), providing an automatic PulseAudio bridge (`/mnt/wslg/PulseServer`) connected to your Windows speakers.

- **✅ Supported in WSL2**:
  - Running `sonic_aura --demo` (built-in binaural synthesizer plays through Windows speakers).
  - Offline file processing (`sonic_aura -i song.wav -o enhanced.wav`).
  - Enhancing audio from **Linux applications running inside WSL** (e.g. Linux media players or Linux browsers).
- **⚠️ WSL2 Limitation**:
  - A program running inside the WSL2 virtual machine **cannot intercept or enhance audio playing on Windows 11 host applications** (such as Windows Chrome, Windows Spotify, Discord, or PC games). Windows 11 routes audio *out* of WSL, but does not pipe the host desktop mix *into* the WSL VM.

---

### 🌟 Recommended: Run Natively on Windows 11

Because SonicAura is built with cross-platform Rust crates (`cpal`, `ratatui`, `crossterm`), you can compile and run it **natively on Windows 11 without WSL** for direct WASAPI hardware access with zero VM overhead.

#### Quick Setup:

1. **Install Rust for Windows**:
   - Download and run [rustup-init.exe](https://rustup.rs/) (select default MSVC toolchain).
   - If prompted, install C++ Build Tools via the Visual Studio Installer.

2. **Clone and Build in PowerShell**:
   ```powershell
   git clone https://github.com/CharleGutierrez/sonic-aura.git
   cd sonic-aura
   cargo build --release
   ```

3. **Run in Windows Terminal**:
   ```powershell
   # Interactive TUI:
   .\target\release\sonic_aura.exe

   # With Tinnitus Relief active at 6,000 Hz:
   .\target\release\sonic_aura.exe --tinnitus-mode notch --tinnitus-freq 6000

   # Audition demo synth:
   .\target\release\sonic_aura.exe --demo
   ```

*(Tip: Run inside **Windows Terminal** for full 24-bit TrueColor support, unicode icons, and modern ANSI rendering).*

---

## 📄 License
MIT License

