# ⚡ SonicAura AI: D_lby Atm_s & B_ng & Ol_fs_n-Grade Audio Enhancer

A lightweight, ultra-low latency, psychoacoustic AI sound enhancement engine written in pure **Rust**.

Transforms laptop speakers, external soundbars, studio monitors, IEMs, and headphones into an immersive, crisp, and high-fidelity acoustic experience inspired by **D_lby Atm_s**, **B_ng & Ol_fs_n BeoPlay**, and **Appl_ Spat__l Audio**.

---

## 🌟 Key Features

### 1. 🧠 AI Psychoacoustic & Adaptive Spectral Engine
- **Spectral Feature Extraction**: Real-time extraction of Spectral Centroid (brightness), Spectral Flux (transient detection), and RMS/Peak dynamic envelope.
- **AI Voice / Dialogue Clarity Lift**: Detects vocal formant distributions and dynamically lifts speech presence (+2 to +4 dB) while ducking muddy masking frequencies.
- **Adaptive Bass & Air Modulation**: Intelligently scales sub-bass punch and high-frequency sparkle according to audio genre and loudness.

### 2. 🔊 Psychoacoustic Missing-Fundamental Sub-Bass (MaxxBass / B&O Acoustic Lens)
- Tiny laptop speakers physically cannot reproduce deep 40–80 Hz vibrations without severe buzzing or cone distortion.
- SonicAura uses the psychoacoustic **Missing Fundamental Phenomenon**: generates euphonic 2nd and 3rd harmonics in the resonant 120–360 Hz range so the human brain perceives deep, room-shaking sub-bass on small laptop drivers.
- Includes **Driver Protection HPF** to eliminate destructive sub-sonic DC excursion.

### 3. ✨ Crystalline Harmonic Exciter & High-Frequency Air
- High-order polynomial saturation generates silky odd/even harmonics above 9 kHz.
- Adds mastering-grade "Air & Sparkle" to cymbals, vocal breath, and acoustic instruments without harshness or digital sibilance.

### 4. 🌐 D_lby Atm_s 3D Binaural Spatializer & Soundstage Widener
- **Mid/Side Stereophony**: Frequency-dependent expansion (keeps bass tightly centered in mono while expanding midrange and treble up to 180%).
- **Binaural HRTF Crossfeed**: Interaural Time Difference (ITD ~280µs) and head-shadow filtering eliminate "in-the-head" headphone fatigue.
- **3D Room Ambience**: Multi-tap prime-delay decorrelator creates cinema-like virtual depth and width.
- **Laptop Speaker Lens**: Cross-talk cancellation expands the soundstage far beyond the physical laptop chassis.

### 5. 🎚️ Complete 10-Band ISO Precision Equalizer & Presets
- High-precision Direct Form II Transposed Bi-quad IIR filters with click-free parameter interpolation.
- **Factory Presets**:
  - `D_lby Atm_s Cinema 3D`: Deep cinematic sub-bass, crystal dialogue presence, immersive 360 soundfield.
  - `B_ng & Ol_fs_n Signature`: BeoPlay acoustic warmth, silky smooth highs, open soundstage.
  - `Apple Spatial Air & Punch`: Modern punchy low-end, crisp vocal clarity, pristine sparkle.
  - `Laptop Speaker Resonator Fix`: Eliminates boxy chassis resonance (300–600 Hz), psychoacoustic sub-bass, stereo widener.
  - `DTS:X Spatial Gaming 3D`: Snappy footstep transients, pinpoint directional cues, explosion rumble.
  - `Audiophile Reference Headphones`: Harman target curve, transparent Meier binaural crossfeed.
  - `Vocal / Podcast / Dialogue Clarity AI`: Speech intelligibility boost and background noise suppression.
  - `Club / EDM / Hip-Hop Ultra Punch`: Maximum sub-harmonic drive and dynamic loudness contour.
  - `Acoustic / Classical Concert Hall`: Concert hall acoustic depth with transparent highs.
  - `Flat Reference / Bypass`: Direct passthrough for A/B testing.

### 6. 🛡️ Multiband Dynamic Compressor & True-Peak Limiter
- 3-band dynamic compression (Low, Mid, High) with automatic makeup gain.
- **Fletcher-Munson Equal Loudness Contour**: Keeps audio full-bodied and punchy even at quiet late-night volume levels.
- **Lookahead True-Peak Brickwall Limiter & Soft Clipper**: Guaranteed zero digital clipping (ceiling: -0.1 dBFS).

---

## 📊 Performance & Benchmark

Tested at 48,000 Hz stereo audio on a single CPU core:
- **Throughput Speed**: **34.7x Real-time**
- **Processing Rate**: **1.67 Million samples/second**
- **Latency**: **600 nanoseconds** per sample frame
- **CPU Load**: **< 2.9% of a single CPU core**

---

## 🚀 Quick Start

### 1. Build from Source
```bash
cd sonic_aura
cargo build --release
```
Binary will be located at `./target/release/sonic_aura`.

---

### 2. Immediate Audition Demo (No setup required)
Audition the rich Dolby/B&O spatial audio demo using the built-in binaural musical synthesizer:
```bash
./target/release/sonic_aura --demo
```

---

### 3. Interactive TUI Dashboard
Launch the interactive Terminal User Interface:
```bash
./target/release/sonic_aura
```

#### TUI Keyboard Controls:
- `[Tab]`: Switch between Panels (*Presets*, *10-Band EQ*, *AI Boost Enhancers*).
- `[← / →]`: Select EQ Band or Adjust Enhancer Sliders.
- `[↑ / ↓]`: Increase / Decrease EQ Gain or navigate items.
- `[P]`: Cycle through Presets (D_lby Atm_s, B&O, Apple Spatial, Laptop Fix, etc.).
- `[Space]`: Toggle **A/B Bypass** (Instant comparison between enhanced vs raw sound).
- `[M]`: Switch Output Mode (*Headphones 3D Atmos* ↔ *Laptop Speakers* ↔ *Studio Nearfield*).
- `[A]`: Toggle AI Adaptive Boost.
- `[0]`: Reset selected EQ band to 0.0 dB.
- `[S]`: Save current settings to `~/.config/sonic_aura/config.toml`.
- `[Q]`: Quit.

---

### 4. System-Wide Audio Integration (PipeWire / PulseAudio)
To route all computer audio (YouTube, Spotify, Games, Netflix, VLC) through SonicAura:

1. Create the virtual loopback sink:
   ```bash
   ./target/release/sonic_aura --setup-sink
   ```
2. Open your system **Sound Settings** and set **`SonicAura_AI_Enhancer_Sink`** as your Default Output Device.
3. Start SonicAura (or run in background daemon mode):
   ```bash
   ./target/release/sonic_aura --daemon
   ```
4. To remove the virtual sink:
   ```bash
   ./target/release/sonic_aura --remove-sink
   ```

---

### 5. Offline Audio File Remastering
Process and remaster any WAV audio file offline with high-speed DSP:
```bash
./target/release/sonic_aura --process-file song.wav -o song_remastered.wav --preset "D_lby Atm_s"
```

---

## 🛠️ CLI Options

| Flag | Description |
|---|---|
| `-d, --demo` | Launch in Demo Synth mode (built-in binaural music) |
| `--synth-tone <TONE>` | Demo synth tone: `music` (default), `pink`, `sweep`, `kick` |
| `-p, --preset <NAME>` | Initial preset name (`"D_lby Atm_s"`, `"B_ng & Ol_fs_n"`, etc.) |
| `-D, --daemon` | Run headless audio engine in the background |
| `--setup-sink` | Automatically create PipeWire/PulseAudio virtual sink |
| `--remove-sink` | Remove PipeWire/PulseAudio virtual sink |
| `--list-devices` | List all available input/output audio hardware |
| `--process-file <IN>` | Batch remaster audio file offline |
| `-o, --output <OUT>` | Output file path for offline processing |
| `--benchmark` | Run high-throughput DSP performance benchmark |
| `-h, --help` | Display CLI help menu |

---

## 📐 Architecture

```
                       ┌────────────────────────┐
                       │  Input Audio / Loopback │
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │ 10-Band Parametric EQ  │
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │ Psychoacoustic Sub-Bass │ ◄── (MaxxBass Harmonics)
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │    Transient Shaper    │ ◄── (Dynamic Attack/Punch)
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │ Harmonic Exciter & Air │ ◄── (>10kHz B&O Sheen)
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │ 3D Binaural Spatializer│ ◄── (HRTF / M-S Expander)
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │  Multiband Compressor  │ ◄── (Fletcher-Munson Loudness)
                       └───────────┬────────────┘
                                   │
                                   ▼
                       ┌────────────────────────┐
                       │  True-Peak Limiter     │ ◄── (-0.1 dBFS Safe Ceiling)
                       └───────────┬────────────┘
                                   │
                        ┌──────────┴──────────┐
                        ▼                     ▼
              ┌──────────────────┐  ┌──────────────────┐
              │ Output Playback  │  │ AI Real-Time FFT │
              │ (Speakers / IEM) │  │ Spectrum & Meters│
              └──────────────────┘  └──────────────────┘
```

---

## 📄 License
MIT License
