// nih_plug foundation for VST3 / Audio Unit exports
// Uncomment when nih_plug is fully compiled in Cargo.toml

/*
use nih_plug::prelude::*;
use std::sync::Arc;
use crate::dsp::pipeline::AudioPipeline;

struct SonicAuraPlugin {
    params: Arc<SonicAuraParams>,
    pipeline: AudioPipeline,
}

#[derive(Params)]
struct SonicAuraParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for SonicAuraPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(SonicAuraParams::default()),
            pipeline: AudioPipeline::new(48000.0),
        }
    }
}

impl Default for SonicAuraParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new("Master Gain", 0.0, FloatRange::Linear { min: -24.0, max: 24.0 }),
        }
    }
}

impl Plugin for SonicAuraPlugin {
    const NAME: &'static str = "Sonic Aura AI";
    const VENDOR: &'static str = "Dynabook";
    const URL: &'static str = "https://example.com";
    const EMAIL: &'static str = "info@example.com";
    const VERSION: &'static str = "0.1.0";
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        }
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.pipeline.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.pipeline.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {

            let mut iter = channel_samples.into_iter();
            let l_ptr = iter.next().unwrap();
            let r_ptr = iter.next().unwrap();
            
            let (out_l, out_r) = self.pipeline.process_stereo_sample(*l_ptr, *r_ptr);
            
            *l_ptr = out_l;
            *r_ptr = out_r;
        }
        ProcessStatus::Normal
    }
}

nih_export_vst3!(SonicAuraPlugin);
nih_export_clap!(SonicAuraPlugin);
*/
